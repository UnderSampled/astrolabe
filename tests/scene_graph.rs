use anyhow::Result;
use astrolabe::{
    godot,
    model::{
        AssetPath, AssetRef, Selection,
        collision::*,
        content::Collection,
        geometry::SourceTransform,
        io::{Content, ContentRoot, Load, Save},
        scene::*,
    },
    openspace::level::{
        ReferenceTarget, SceneDependencies, read_transform, write_collision, write_scene_graph,
    },
    rete,
};
use std::collections::BTreeMap;
fn fixture() -> Result<Scene> {
    let nodes = [
        ("world", None, NodeKind::World, [10., 0., 0.]),
        (
            "world/object",
            Some("world"),
            NodeKind::Object,
            [0., 2., 0.],
        ),
        ("world/fixed", Some("world"), NodeKind::Perso, [3., 4., 5.]),
    ]
    .into_iter()
    .map(|(id, parent, kind, translation)| SceneNode {
        properties: SceneNodeProperties {
            transform_space: if id.ends_with("fixed") {
                TransformSpace::World
            } else {
                TransformSpace::Local
            },
            ..Default::default()
        },
        id: id.into(),
        name: id.into(),
        parent: parent.map(String::from),
        kind,
        transform: Some(SourceTransform {
            translation,
            ..Default::default()
        }),
        instance: None,
        perso: None,
        collision: None,
        draw_mask: u32::MAX,
        sector: None,
        light: None,
    })
    .collect();
    Ok(Scene {
        main_actor: None,
        armor: None,
        inventory: None,
        global_actors: vec![],
        path: AssetPath::new("level:/scene")?,
        name: "Graph".into(),
        nodes,
    })
}
fn word(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes(b[p..p + 4].try_into().unwrap())
}
#[test]
fn scene_graph_rete_edits_derived_hierarchy_and_embedded_matrix_targets() -> Result<()> {
    let mut scene = fixture()?;
    scene.nodes[1].properties.source_kind = Some(0x15);
    scene.nodes[1].properties.opacity = 127.5;
    scene.nodes[1].properties.initial_global_transform = Some(SourceTransform {
        translation: [1., 2., 3.],
        ..Default::default()
    });
    scene.nodes[1].properties.bounding_geometry = Some(AssetRef::new("level:/collision/bounds")?);
    let selection = Selection::hype("scene-graph", "en")?;
    let temp = tempfile::tempdir()?;
    let mut store = rete::Store::new(temp.path(), selection.clone())?;
    let reference = AssetRef::<Scene>::from_path(scene.path.clone());
    store.save(&reference, &scene)?;
    drop(store);
    let mut store = rete::Store::new(temp.path(), selection)?;
    let mut scene = (*store.load(&reference)?).clone();
    scene.nodes[0].transform.as_mut().unwrap().translation[0] = 20.;
    let mut dependencies = SceneDependencies {
        data: BTreeMap::from([
            (
                "world/object".into(),
                ReferenceTarget {
                    address: 0x20000,
                    block: (6, 2),
                },
            ),
            (
                "world/fixed".into(),
                ReferenceTarget {
                    address: 0x30000,
                    block: (5, 0),
                },
            ),
        ]),
        local_transforms: BTreeMap::from([(
            "world/fixed".into(),
            ReferenceTarget {
                address: 0x30018,
                block: (5, 0),
            },
        )]),
        bounds: BTreeMap::from([(
            AssetPath::new("level:/collision/bounds")?,
            ReferenceTarget {
                address: 0x40000,
                block: (6, 1),
            },
        )]),
        ..Default::default()
    };
    let output = write_scene_graph(&scene, &dependencies, 0x10000)?;
    let b = &output.fragment.bytes;
    assert_eq!(word(b, 8), 0x10040);
    assert_eq!(word(b, 12), 0x10080);
    assert_eq!(word(b, 16), 2);
    assert_eq!(word(b, 64), 0x15);
    assert_eq!(word(b, 64 + 20), 0x10080);
    assert_eq!(word(b, 128 + 24), 0x10040);
    assert_eq!(word(b, 64 + 28), 0x10000);
    assert_eq!(word(b, 128 + 28), 0x10000);
    assert_eq!(word(b, 128 + 32), 0x30018);
    assert_eq!(word(b, 128 + 40), u32::MAX);
    assert_eq!(word(b, 64 + 40), u32::MAX - 1);
    assert_eq!(word(b, 64 + 52), 0x40000);
    assert_eq!(word(b, 64 + 60), 127.5f32.to_bits());
    for (name, expected) in [
        ("world", [20., 0., 0.]),
        ("world/object", [20., 2., 0.]),
        ("world/fixed", [3., 4., 5.]),
    ] {
        let p = (output.global_transforms[name].address - 0x10000) as usize;
        assert_eq!(read_transform(&b[p..])?.affine().translation, expected);
    }
    let p = (word(b, 64 + 56) - 0x10000) as usize;
    assert_eq!(read_transform(&b[p..])?.affine().translation, [1., 2., 3.]);
    assert!(
        !output
            .fragment
            .ranges
            .iter()
            .any(|r| r.name == "nodes/world/fixed/local")
    );
    assert_ne!(
        output.global_transforms["world"].address,
        output.global_transforms["world/object"].address
    );
    dependencies.bounds.clear();
    assert!(write_scene_graph(&scene, &dependencies, 0x10000).is_err());
    scene.nodes[1].properties.source_kind = Some(4);
    assert!(scene.validate().is_err());
    scene.nodes[1].properties.source_kind = None;
    scene.nodes[0].properties.opacity = f32::NAN;
    assert!(scene.validate().is_err());
    Ok(())
}

fn empty_bounds() -> Result<Collision> {
    Ok(Collision {
        path: AssetPath::new("level:/collision/empty")?,
        geometry: CollisionGeometry::Mesh(CollisionMesh {
            positions: vec![[0.; 3]],
            normals: vec![],
            vertex_materials: vec![],
            elements: vec![CollisionElement::Spheres(vec![CollisionSphere {
                center: 0,
                radius: 0.,
                material: None,
            }])],
        }),
    })
}

#[test]
fn world_placement_and_empty_bounds_preserve_source_meaning() -> Result<()> {
    let mut scene = fixture()?;
    // A scaled quarter-turn parent exercises basis composition, not just offsets.
    scene.nodes[0].transform.as_mut().unwrap().rotation =
        [[0., 1., 0.], [-1., 0., 0.], [0., 0., 1.]];
    scene.nodes[0].transform.as_mut().unwrap().basis = [[0., 2., 0.], [-3., 0., 0.], [0., 0., 4.]];
    let transforms = scene.global_transforms()?;
    assert_eq!(transforms["world/object"].translation, [4., 0., 0.]);
    assert_eq!(transforms["world/object"].basis, transforms["world"].basis);
    assert_eq!(
        transforms["world/object"].rotation,
        transforms["world"].rotation
    );
    assert_eq!(
        transforms["world/fixed"],
        *scene.nodes[2].transform.as_ref().unwrap()
    );
    let mut bounds = empty_bounds()?;
    bounds.validate()?;
    assert_eq!(rete::decode::<Collision>(&rete::encode(&bounds)?)?, bounds);
    let out = write_collision(&bounds, &mut Collection::default(), 0x2000)?;
    let p = out
        .ranges
        .iter()
        .find(|r| r.name == "elements/0/spheres")
        .unwrap()
        .offset;
    assert_eq!(word(&out.bytes, p + 4), 0f32.to_bits());
    let CollisionGeometry::Mesh(mesh) = &mut bounds.geometry else {
        unreachable!()
    };
    let CollisionElement::Spheres(spheres) = &mut mesh.elements[0] else {
        unreachable!()
    };
    spheres[0].radius = -1.;
    assert!(bounds.validate().is_err());
    Ok(())
}

#[test]
#[ignore = "requires Godot; included by tools/test.py --local"]
fn godot_world_transforms_and_picking_bounds_do_not_add_physics() -> Result<()> {
    let mut scene = fixture()?;
    let empty = empty_bounds()?;
    let mut bounds = empty.clone();
    bounds.path = AssetPath::new("level:/collision/picking")?;
    let CollisionGeometry::Mesh(mesh) = &mut bounds.geometry else {
        unreachable!()
    };
    let CollisionElement::Spheres(spheres) = &mut mesh.elements[0] else {
        unreachable!()
    };
    spheres[0].radius = 3.;
    scene.nodes[1].properties.bounding_geometry = Some(AssetRef::from_path(bounds.path.clone()));
    scene.nodes[2].collision = Some(AssetRef::from_path(empty.path.clone()));
    let mut source = Collection::default();
    source.insert(&AssetRef::from_path(scene.path.clone()), scene.clone())?;
    source.insert(&AssetRef::from_path(bounds.path.clone()), bounds)?;
    source.insert(&AssetRef::from_path(empty.path.clone()), empty)?;
    let temp = tempfile::tempdir()?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    godot::export(
        &mut source,
        &[ContentRoot {
            kind: Scene::KIND.into(),
            reference: scene.path,
        }],
        Selection::hype("scene-graph", "en")?,
        temp.path(),
        &engine,
    )?;
    std::fs::write(
        temp.path().join("graph.gd"),
        r#"extends SceneTree
func require(ok: bool, reason: String) -> bool:
    if not ok:
        push_error(reason)
        quit(1)
    return ok
func _initialize():
    call_deferred("check")
func check():
    var scene = load("res://generated/hype/levels/scene-graph/level.tscn").instantiate()
    root.add_child(scene)
    await process_frame
    var local = scene.get_node("world/object")
    var fixed = scene.get_node("world/fixed")
    if not require(local.global_position.is_equal_approx(Vector3(10,0,-2)), "local hierarchy placement"): return
    if not require(fixed.top_level and fixed.global_position.is_equal_approx(Vector3(3,5,-4)), "static world placement"): return
    scene.get_node("world").position.x += 5
    if not require(local.global_position.is_equal_approx(Vector3(15,0,-2)), "local node must follow parent edit"): return
    if not require(fixed.global_position.is_equal_approx(Vector3(3,5,-4)), "static world node inherited parent edit"): return
    if not require(local.has_meta("source_picking_bounds") and not local.has_node("Collision"), "picking bounds became physics"): return
    if not require(fixed.get_node("Collision").get_child_count() == 0, "empty sphere became a physics shape"): return
    scene.free()
    quit(0)
"#,
    )?;
    godot::run_godot(
        &engine,
        temp.path(),
        &["--headless", "--script", "res://graph.gd"],
        std::time::Duration::from_secs(30),
    )?;
    Ok(())
}
