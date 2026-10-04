//! Scene hierarchy, shared collision dependencies, source edits and actual physics.
use anyhow::Result;
use astrolabe::{
    godot,
    model::{
        AssetPath, AssetRef, Selection,
        collision::*,
        content::Collection,
        geometry::{Material, SourceTransform},
        io::{Content, ContentRoot, Load, Save},
        scene::*,
    },
    openspace, rete,
};
use std::{fs, time::Duration};

fn fixture() -> Result<(Scene, Collision, Material)> {
    let material = Material {
        path: AssetPath::new("level:/materials/floor")?,
        visual: None,
        collision: CollisionProperties::Surface {
            kind: 0xffff,
            identifier: 0,
        },
        sound_material: 12,
    };
    let collision = Collision {
        path: AssetPath::new("level:/collision/floor")?,
        geometry: CollisionGeometry::Mesh(CollisionMesh {
            positions: vec![[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]],
            normals: vec![],
            vertex_materials: vec![],
            elements: vec![CollisionElement::Triangles {
                material: Some(AssetRef::from_path(material.path.clone())),
                indices: vec![[0, 1, 2]],
                normals: vec![],
                texcoords: vec![],
                uv_indices: vec![],
                vertex_indices: vec![],
            }],
        }),
    };
    let mut scene = Scene {
        main_actor: None,
        armor: None,
        inventory: None,
        global_actors: vec![],
        path: AssetPath::new("level:/scene")?,
        name: "SyntheticScene".into(),
        nodes: vec![SceneNode {
            properties: Default::default(),
            id: "world".into(),
            name: "world".into(),
            parent: None,
            kind: NodeKind::World,
            transform: Some(SourceTransform {
                translation: [1., 0., 0.],
                ..Default::default()
            }),
            instance: None,
            perso: None,
            collision: None,
            draw_mask: u32::MAX,
            sector: None,
            light: None,
        }],
    };
    for (id, x) in [("floor", 2.), ("other", 7.)] {
        scene.nodes.push(SceneNode {
            properties: Default::default(),
            id: format!("world/{id}"),
            name: id.into(),
            parent: Some("world".into()),
            kind: NodeKind::Object,
            transform: Some(SourceTransform {
                translation: [x, 0., 0.],
                ..Default::default()
            }),
            instance: None,
            perso: None,
            collision: Some(AssetRef::from_path(collision.path.clone())),
            draw_mask: u32::MAX,
            sector: None,
            light: None,
        });
    }
    Ok((scene, collision, material))
}

#[test]
fn scene_collision_roundtrip_edits_and_invalid_hierarchy() -> Result<()> {
    let (scene, mut collision, material) = fixture()?;
    let temp = tempfile::tempdir()?;
    let selection = Selection::hype("scene-test", "en")?;
    let mut store = rete::Store::new(temp.path(), selection.clone())?;
    store.save(&AssetRef::from_path(scene.path.clone()), &scene)?;
    store.save(&AssetRef::from_path(collision.path.clone()), &collision)?;
    store.save(&AssetRef::from_path(material.path.clone()), &material)?;
    drop(store);
    let mut store = rete::Store::new(temp.path(), selection)?;
    assert_eq!(
        *store.load(&AssetRef::<Scene>::from_path(scene.path.clone()))?,
        scene
    );
    let persisted = store.load(&AssetRef::<Collision>::from_path(collision.path.clone()))?;
    assert_eq!(*persisted, collision);
    let original = openspace::level::write_collision(&persisted, &mut store, 0x1000)?;
    let range = original
        .ranges
        .iter()
        .find(|r| r.name == "elements/0/indices")
        .unwrap();
    assert_eq!(
        &original.bytes[range.offset..range.offset + range.length],
        &[0, 0, 1, 0, 2, 0]
    );
    let properties = original
        .ranges
        .iter()
        .find(|r| r.name == "materials/0/collision")
        .unwrap();
    assert_eq!(
        &original.bytes[properties.offset..properties.offset + 4],
        &[255, 255, 0, 0]
    );
    let CollisionGeometry::Mesh(mesh) = &mut collision.geometry else {
        unreachable!()
    };
    mesh.positions.push([0., 0., 4.]);
    mesh.elements
        .push(CollisionElement::Spheres(vec![CollisionSphere {
            center: 3,
            radius: 0.75,
            material: Some(AssetRef::from_path(material.path.clone())),
        }]));
    let changed = openspace::level::write_collision(&collision, &mut store, 0x1000)?;
    assert_ne!(original.bytes, changed.bytes);
    let spheres = changed
        .ranges
        .iter()
        .find(|r| r.name == "elements/1/spheres")
        .unwrap();
    assert_eq!(
        &changed.bytes[spheres.offset..spheres.offset + 8],
        &[3, 0, 0, 0, 0, 0, 64, 63]
    );
    assert_eq!(
        changed
            .ranges
            .iter()
            .filter(|r| r.name == "materials/0")
            .count(),
        1
    );
    let mut invalid = scene.clone();
    invalid.nodes[0].parent = Some("world/floor".into());
    assert!(invalid.validate().is_err());
    invalid = scene;
    invalid.nodes[2].id = invalid.nodes[1].id.clone();
    assert!(invalid.validate().is_err());
    Ok(())
}

#[test]
#[ignore = "requires local game data and Godot; included by tools/test.py --local"]
fn source_scene_rete_routes_preserve_hierarchy_collision_and_model_sharing() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let selection = Selection::hype("astrolabe", "en")?;
    let mut imported = openspace::content::import_scene(&game, selection)?;
    let root = AssetRef::<Scene>::new("level:/scene")?;
    let scene = imported.load(&root)?;
    assert_eq!(scene.global_actors.len(), 1);
    let global = &scene.global_actors[0];
    assert_eq!(global.source.role, astrolabe::model::ai::SourceRole::Fix);
    assert_eq!(global.source.block, [5, 0]);
    assert_eq!(global.source.allocation, 15);
    assert_eq!(global.perso.path.as_str(), "fix:/persos/15");
    let global_perso: std::sync::Arc<astrolabe::model::character::Perso> =
        imported.load(&global.perso)?;
    assert_eq!(global_perso.name, "Global");
    let global_brain = global_perso.brain.as_ref().expect("global Brain imported");
    let global_model: std::sync::Arc<astrolabe::model::ai::AiModel> =
        imported.load(&global_brain.model)?;
    assert_eq!(global_model.name, "Global");
    assert_eq!(global_model.variables.len(), 238);
    assert!(matches!(
        global_model.variables[17].default,
        astrolabe::model::ai::DesignerValue::Boolean(true)
    ));
    assert_eq!(scene.nodes.len(), 150);
    assert_eq!(
        scene.nodes.iter().filter(|n| n.instance.is_some()).count(),
        22
    );
    assert_eq!(
        scene.nodes.iter().filter(|n| n.collision.is_some()).count(),
        22
    );
    let mut shared = 0;
    for node in &scene.nodes {
        if let Some(reference) = &node.collision {
            let collision = imported.load(reference)?;
            if let CollisionGeometry::Model(model) = &collision.geometry {
                shared += 1;
                let instance = imported.load(node.instance.as_ref().unwrap())?;
                assert_eq!(model, &instance.model);
            }
        }
    }
    assert_eq!(shared, 20, "source render/collision GEO identity changed");
    let temp = tempfile::tempdir()?;
    astrolabe::app::save_rete(&mut imported, &temp.path().join("rete"))?;
    let (mut store, package) = rete::Store::open(temp.path().join("rete"))?;
    assert_eq!(*store.load(&root)?, *scene);
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let direct = godot::export(
        &mut imported,
        &package.roots,
        package.selection.clone(),
        &temp.path().join("direct"),
        &engine,
    )?;
    let reloaded = godot::export(
        &mut store,
        &package.roots,
        package.selection.clone(),
        &temp.path().join("reloaded"),
        &engine,
    )?;
    assert_eq!(direct.files, reloaded.files);
    let project = temp.path().join("reloaded");
    fs::write(
        project.join("native_scene_check.gd"),
        include_bytes!("native_scene_check.gd"),
    )?;
    let check_native = |state: &str, radius: &str| -> Result<()> {
        let evidence = godot::run_godot(
            &engine,
            &project,
            &[
                "--headless",
                "--script",
                "res://native_scene_check.gd",
                "--",
                state,
                radius,
            ],
            Duration::from_secs(60),
        )?;
        assert!(evidence.contains("NATIVE_SCENE_OK"));
        println!("{evidence}");
        Ok(())
    };
    check_native("47", "9")?;
    let source = openspace::export(&mut store, &package.roots, &temp.path().join("source"))?;
    assert!(
        source
            .files
            .iter()
            .any(|p| p.contains("collision/world/1/0/0"))
    );
    // One edit to the shared model changes collision resolution and reconstructed positions.
    let collision = store.load(
        scene
            .nodes
            .iter()
            .find_map(|n| n.collision.as_ref())
            .unwrap(),
    )?;
    let CollisionGeometry::Model(model_ref) = &collision.geometry else {
        panic!("first GEO should be shared")
    };
    let mut model = (*store.load(model_ref)?).clone();
    let before = openspace::level::write_collision(&collision, &mut store, 0x1000)?;
    model.positions[0][2] += 0.5;
    store.save(model_ref, &model)?;
    assert_eq!(collision.resolve(&mut store)?.positions, model.positions);
    let after = openspace::level::write_collision(&collision, &mut store, 0x1000)?;
    assert_ne!(before.bytes, after.bytes);
    // An independently authored scene inherits a deliberate Rete state edit on regeneration.
    let hero_ref = AssetRef::<astrolabe::model::character::Perso>::new("level:/persos/world/0/0")?;
    let mut hero = (*store.load(&hero_ref)?).clone();
    hero.current_state = Some(32);
    store.save(&hero_ref, &hero)?;
    let ai_ref = AssetRef::<astrolabe::model::ai::AiModel>::new("level:/ai/models/824")?;
    let mut ai = (*store.load(&ai_ref)?).clone();
    use astrolabe::model::ai::{Expression, Statement};
    let script = ai.normal.as_mut().unwrap().behaviors[0]
        .schedule
        .as_mut()
        .unwrap();
    let Statement::If {
        condition: Expression::Compare { right, .. },
        ..
    } = &mut script.statements[1]
    else {
        panic!("source torch branch")
    };
    **right = Expression::Real(3.0);
    store.save(&ai_ref, &ai)?;
    let authored = project.join("game/characters/luminous_hype.tscn");
    let authored_bytes = fs::read(&authored)?;
    let settings = fs::read_to_string(project.join("project.godot"))?
        + "\n[authored]\nregeneration_marker=73\n";
    fs::write(project.join("project.godot"), &settings)?;
    let native_before = fs::read_to_string(
        project.join("generated/hype/levels/astrolabe/persos/Hype__891c7341e3.tscn"),
    )?;
    let edited = godot::export(
        &mut store,
        &package.roots,
        package.selection,
        &project,
        &engine,
    )?;
    assert_ne!(edited.files, reloaded.files);
    assert_eq!(fs::read(&authored)?, authored_bytes);
    assert_eq!(fs::read_to_string(project.join("project.godot"))?, settings);
    let native_after = fs::read_to_string(
        project.join("generated/hype/levels/astrolabe/persos/Hype__891c7341e3.tscn"),
    )?;
    let identities = |text: &str| {
        text.lines()
            .filter_map(|l| {
                l.split_once(" unique_id=")
                    .or_else(|| l.split_once(" uid="))
                    .map(|(_, value)| value.to_owned())
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        identities(&native_before),
        identities(&native_after),
        "regeneration changed resource/node identity"
    );
    godot::support::update(&project)?;
    fs::remove_dir_all(project.join(".godot"))?;
    godot::run_godot(
        &engine,
        &project,
        &["--headless", "--import"],
        Duration::from_secs(60),
    )?;
    check_native("32", "3")?;
    // Exercise the actual animation editor on these same saved native resources.
    let plugin = project.join("addons/native_editor_probe");
    fs::create_dir_all(&plugin)?;
    fs::write(
        plugin.join("probe.gd"),
        include_bytes!("native_editor_check.gd"),
    )?;
    fs::write(
        plugin.join("plugin.cfg"),
        "[plugin]\nname=\"Native acceptance probe\"\ndescription=\"Disposable editor check\"\nauthor=\"Astrolabe\"\nversion=\"1\"\nscript=\"probe.gd\"\n",
    )?;
    fs::write(
        project.join("project.godot"),
        settings.clone()
            + "\n[editor_plugins]\nenabled=PackedStringArray(\"res://addons/native_editor_probe/plugin.cfg\")\n",
    )?;
    let editor = godot::run_godot(
        &engine,
        &project,
        &["--headless", "--editor"],
        Duration::from_secs(60),
    )?;
    assert!(editor.contains("NATIVE_EDITOR_OK"));
    assert_eq!(editor.matches("NATIVE_SCENE_DOCK_OK").count(), 2);
    println!(
        "{}",
        editor
            .lines()
            .filter(
                |line| line.contains("NATIVE_EDITOR_OK") || line.contains("NATIVE_SCENE_DOCK_OK")
            )
            .collect::<Vec<_>>()
            .join("\n")
    );
    fs::write(project.join("project.godot"), settings)?;
    Ok(())
}

#[test]
#[ignore = "requires Godot; included by tools/test.py --local"]
fn godot_scene_physics_uses_parent_transforms_and_collision_edits() -> Result<()> {
    let (scene, collision, material) = fixture()?;
    let mut source = Collection::default();
    source.insert(&AssetRef::from_path(scene.path.clone()), scene.clone())?;
    source.insert(&AssetRef::from_path(collision.path.clone()), collision)?;
    source.insert(&AssetRef::from_path(material.path.clone()), material)?;
    let temp = tempfile::tempdir()?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    godot::export(
        &mut source,
        &[ContentRoot {
            kind: Scene::KIND.into(),
            reference: scene.path,
        }],
        Selection::hype("scene-test", "en")?,
        temp.path(),
        &engine,
    )?;
    fs::write(
        temp.path().join("physics.gd"),
        r#"extends SceneTree
func require(ok: bool, reason: String) -> bool:
    if not ok:
        push_error(reason)
        quit(1)
    return ok
func _initialize():
    call_deferred("check")
func check():
    var scene = load("res://generated/hype/levels/scene-test/level.tscn").instantiate()
    root.add_child(scene)
    await physics_frame
    await physics_frame
    var state = scene.get_world_3d().direct_space_state
    for x in [3.0, 8.0]:
        var hit = state.intersect_ray(PhysicsRayQueryParameters3D.create(Vector3(x, 5, 0), Vector3(x, -5, 0)))
        if not require(not hit.is_empty(), "missing collision at translated floor"): return
        if not require(absf(hit.position.y) < 0.001, "collision floor height differs"): return
    if not require(state.intersect_ray(PhysicsRayQueryParameters3D.create(Vector3(0,5,0), Vector3(0,-5,0))).is_empty(), "untransformed collision left at origin"): return
    if not require(scene.get_node("world/floor").global_position.is_equal_approx(Vector3(3,0,0)), "hierarchy lost"): return
    scene.get_node("world/floor/Collision/0").disabled = true
    await physics_frame
    await physics_frame
    if not require(state.intersect_ray(PhysicsRayQueryParameters3D.create(Vector3(3,5,0), Vector3(3,-5,0))).is_empty(), "disabled collision still active"): return
    scene.free()
    quit(0)
"#,
    )?;
    godot::run_godot(
        &engine,
        temp.path(),
        &["--headless", "--script", "res://physics.gd"],
        Duration::from_secs(30),
    )?;
    Ok(())
}
