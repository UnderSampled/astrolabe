//! Sector links are scene-local identities; portals are inspectable geometry, not walls.
use anyhow::Result;
use astrolabe::{
    godot,
    model::{
        AssetPath, AssetRef, Selection,
        content::Collection,
        geometry::SourceTransform,
        io::{Content, ContentRoot, Load, Save},
        scene::*,
    },
    openspace::level::{ReferenceTarget, write_sectors},
    rete,
};
use std::{collections::BTreeMap, fs, time::Duration};
fn fixture() -> Result<Scene> {
    let mut scene = Scene {
        path: AssetPath::new("level:/scene")?,
        name: "Sectors".into(),
        nodes: Vec::new(),
    };
    for (id, parent) in [
        ("world", None),
        ("world/a", Some("world")),
        ("world/b", Some("world")),
    ] {
        scene.nodes.push(SceneNode {
            properties: Default::default(),
            id: id.into(),
            name: id.into(),
            parent: parent.map(String::from),
            kind: if parent.is_some() {
                NodeKind::Sector
            } else {
                NodeKind::World
            },
            transform: Some(SourceTransform {
                translation: if parent.is_none() {
                    [3., 0., 0.]
                } else {
                    [0.; 3]
                },
                ..Default::default()
            }),
            instance: None,
            perso: None,
            collision: None,
            draw_mask: u32::MAX,
            sector: parent.map(|_| Sector {
                name: id.into(),
                activation: 7,
                ..Default::default()
            }),
            light: None,
        });
    }
    let sector = scene.nodes[1].sector.as_mut().unwrap();
    sector.boundary = Some(SectorBoundary {
        positions: vec![[0., -2., -2.], [0., 2., -2.], [0., 0., 2.]],
        normals: vec![],
        surfaces: vec![SectorSurface {
            sector: Some("world/b".into()),
            triangles: vec![[0, 1, 2]],
            normals: vec![[1., 0., 0.]],
            vertex_indices: vec![2, 0, 1],
            texcoords: vec![[0., 0.]],
            uv_indices: vec![[0, 0, 0]],
        }],
    });
    sector.graphics = vec![SectorVisibility {
        sector: "world/b".into(),
        level: 3,
        unknown: 19,
    }];
    sector.collision = vec!["world/b".into()];
    sector.activity = vec!["world/a".into(), "world/b".into()];
    sector.sound = vec![SectorSoundPropagation {
        sector: "world/b".into(),
        volume: -1200,
    }];
    sector.unknown = [1, 2, 3];
    Ok(scene)
}
#[test]
fn sector_rete_edits_references_and_generated_lists() -> Result<()> {
    let scene = fixture()?;
    let temp = tempfile::tempdir()?;
    let mut store = rete::Store::new(temp.path(), Selection::hype("sectors", "en")?)?;
    let reference = AssetRef::<Scene>::from_path(scene.path.clone());
    store.save(&reference, &scene)?;
    drop(store);
    let mut store = rete::Store::new(temp.path(), Selection::hype("sectors", "en")?)?;
    assert_eq!(*store.load(&reference)?, scene);
    let targets = BTreeMap::from([
        (
            "world/a".into(),
            ReferenceTarget {
                address: 0x2000,
                block: (6, 1),
            },
        ),
        (
            "world/b".into(),
            ReferenceTarget {
                address: 0x2100,
                block: (6, 1),
            },
        ),
    ]);
    let original = write_sectors(
        &scene,
        &targets,
        &BTreeMap::new(),
        &BTreeMap::new(),
        0x10000,
    )?;
    let mut changed = scene.clone();
    let s = changed.nodes[1].sector.as_mut().unwrap();
    s.activity.reverse();
    s.boundary.as_mut().unwrap().positions[0][0] = 0.5;
    s.boundary.as_mut().unwrap().surfaces[0].sector = Some("world/a".into());
    let edited = write_sectors(
        &changed,
        &targets,
        &BTreeMap::new(),
        &BTreeMap::new(),
        0x10000,
    )?;
    assert_ne!(original.bytes, edited.bytes);
    let p = edited
        .ranges
        .iter()
        .find(|r| r.name == "sectors/world/a/boundary/0")
        .unwrap()
        .offset;
    assert_eq!(
        u32::from_le_bytes(edited.bytes[p..p + 4].try_into()?),
        0x2000
    );
    let p = edited
        .ranges
        .iter()
        .find(|r| r.name == "sectors/world/a/list_112/0")
        .unwrap()
        .offset;
    assert_eq!(
        u32::from_le_bytes(edited.bytes[p..p + 4].try_into()?),
        0x2100
    );
    changed.nodes[2].id = "world/renamed".into();
    assert!(
        changed.validate().is_err(),
        "rename must not silently break sector links"
    );
    let mut changed = scene;
    changed.nodes[1]
        .sector
        .as_mut()
        .unwrap()
        .boundary
        .as_mut()
        .unwrap()
        .surfaces[0]
        .triangles[0][0] = 99;
    assert!(changed.validate().is_err());
    Ok(())
}
#[test]
#[ignore = "requires Godot; included by tools/test.py --local"]
fn godot_sector_portal_crossing_respects_parent_transform_and_is_not_collision() -> Result<()> {
    let scene = fixture()?;
    let mut source = Collection::default();
    source.insert(&AssetRef::from_path(scene.path.clone()), scene.clone())?;
    let temp = tempfile::tempdir()?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    godot::export(
        &mut source,
        &[ContentRoot {
            kind: Scene::KIND.into(),
            reference: scene.path,
        }],
        Selection::hype("sectors", "en")?,
        temp.path(),
        &engine,
    )?;
    fs::write(
        temp.path().join("test.gd"),
        r#"extends SceneTree
func require(ok, reason):
    if not ok:
        push_error(reason)
        quit(1)
    return ok
func _initialize():
    call_deferred("check")
func check():
    var scene = load("res://generated/hype/levels/sectors/level.tscn").instantiate()
    root.add_child(scene)
    await physics_frame
    await physics_frame
    var portal = scene.get_node("world/a/SectorBoundary")
    if not require(portal.mesh.get_surface_count() == 1 and portal.mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX].size() == 6, "portal wireframe did not survive scene saving"): return
    if not require(not portal.visible, "diagnostic boundary should start hidden"): return
    var hit = portal.intersect_segment(Vector3(1,0,0), Vector3(5,0,0))
    if not require(not hit.is_empty() and hit.sector == "world/b", "portal neighbor lost"): return
    if not require(absf(hit.fraction - 0.5) < 0.0001 and hit.position.is_equal_approx(Vector3(3,0,0)), "portal placement differs"): return
    if not require(portal.intersect_segment(Vector3(1,5,0),Vector3(5,5,0)).is_empty(), "missed face reported as crossing"): return
    if not require(portal.intersect_segment(Vector3(3,0,0),Vector3(3,0,0)).is_empty(), "zero segment reported as crossing"): return
    var query = PhysicsRayQueryParameters3D.create(Vector3(1,0,0),Vector3(5,0,0))
    if not require(scene.get_world_3d().direct_space_state.intersect_ray(query).is_empty(), "sector portal became a physical wall"): return
    var data = JSON.parse_string(scene.get_node("world/a").get_meta("source_sector"))
    if not require(data.graphics[0].level == 3 and data.sound[0].volume == -1200, "routing data lost"): return
    scene.free()
    quit(0)
"#,
    )?;
    godot::run_godot(
        &engine,
        temp.path(),
        &["--headless", "--script", "res://test.gd"],
        Duration::from_secs(30),
    )?;
    Ok(())
}
