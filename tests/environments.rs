use anyhow::Result;
use astrolabe::{
    godot,
    model::{
        AssetPath, AssetRef, Selection,
        content::Collection,
        environment::*,
        io::{Content, ContentRoot, Load, Save},
        scene::*,
    },
    openspace::level::{
        EnvironmentDependencies, ReferenceTarget, read_fog, write_environments, write_fog,
        write_sectors,
    },
    rete,
};
use std::{collections::BTreeMap, time::Duration};
fn fixture() -> Result<(Environment, VisualEnvironment, Fog, Scene)> {
    let fog = Fog {
        path: AssetPath::new("level:/fogs/haze")?,
        near: 2.,
        far: 12.,
        infinite: 100.,
        color: [0.2, 0.5, 0.8],
        unknown: 0x12345678,
    };
    let visual = VisualEnvironment {
        path: AssetPath::new("level:/visual-environments/air")?,
        fog: Some(AssetRef::from_path(fog.path.clone())),
        unresolved_water: false,
    };
    let env = Environment {
        path: AssetPath::new("level:/environments/air")?,
        visual: Some(AssetRef::from_path(visual.path.clone())),
        physical: None,
        unresolved_sound: false,
    };
    let mut scene = Scene {
        main_actor: None,
        armor: None,
        hud: None,
        global_actors: vec![],
        path: AssetPath::new("level:/scene")?,
        name: "Fog".into(),
        nodes: Vec::new(),
    };
    for (id, parent) in [("world", None), ("world/room", Some("world"))] {
        scene.nodes.push(SceneNode {
            properties: Default::default(),
            id: id.into(),
            name: id.into(),
            parent: parent.map(String::from),
            kind: NodeKind::Sector,
            transform: None,
            instance: None,
            perso: None,
            collision: None,
            draw_mask: u32::MAX,
            light: None,
            sector: Some(Sector {
                name: id.into(),
                environments: vec![AssetRef::from_path(env.path.clone())],
                ..Default::default()
            }),
        });
    }
    Ok((env, visual, fog, scene))
}
fn u32_at(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes(b[p..p + 4].try_into().unwrap())
}

#[test]
fn shared_environment_rete_edits_and_relocated_source_records() -> Result<()> {
    let (env, visual, fog, scene) = fixture()?;
    let temp = tempfile::tempdir()?;
    let selection = Selection::hype("fog-test", "en")?;
    let mut store = rete::Store::new(temp.path(), selection.clone())?;
    store.save(&AssetRef::from_path(env.path.clone()), &env)?;
    store.save(&AssetRef::from_path(visual.path.clone()), &visual)?;
    store.save(&AssetRef::from_path(fog.path.clone()), &fog)?;
    store.save(&AssetRef::from_path(scene.path.clone()), &scene)?;
    drop(store);
    let mut store = rete::Store::new(temp.path(), selection)?;
    let mut edited: Fog = (*store.load(&AssetRef::<Fog>::from_path(fog.path.clone()))?).clone();
    edited.far = 20.;
    edited.color[1] = 0.75;
    store.save(&AssetRef::from_path(edited.path.clone()), &edited)?;
    let scene: std::sync::Arc<Scene> = store.load(&AssetRef::from_path(scene.path.clone()))?;
    assert_eq!(
        scene.nodes[0].sector.as_ref().unwrap().environments,
        scene.nodes[1].sector.as_ref().unwrap().environments
    );
    let bytes = write_fog(&edited)?;
    assert_eq!(read_fog(&bytes, edited.path.clone())?, edited);
    assert_eq!(u32_at(&bytes, 4), 20f32.to_bits());
    assert_eq!(u32_at(&bytes, 24), fog.unknown);
    let mut second = env.clone();
    second.path = AssetPath::new("level:/environments/second")?;
    let dependencies = EnvironmentDependencies::default();
    let graph = write_environments(
        &[env.clone(), second],
        &[],
        std::slice::from_ref(&visual),
        std::slice::from_ref(&edited),
        &dependencies,
        0x20000,
    )?;
    assert_eq!(graph.ranges.len(), 4);
    assert_eq!(graph.pointer_count, 3);
    assert_eq!(u32_at(&graph.bytes, 8), 0x20018);
    assert_eq!(u32_at(&graph.bytes, 20), 0x20018);
    assert_eq!(u32_at(&graph.bytes, 28), 0x20020);
    assert_eq!(&graph.bytes[32..], bytes);
    let targets = BTreeMap::from([(
        env.path.clone(),
        ReferenceTarget {
            address: 0x20000,
            block: (6, 2),
        },
    )]);
    let sectors = write_sectors(
        &scene,
        &BTreeMap::new(),
        &BTreeMap::new(),
        &targets,
        0x30000,
    )?;
    for range in sectors
        .ranges
        .iter()
        .filter(|r| r.name.contains("/list_4/"))
    {
        assert_eq!(u32_at(&sectors.bytes, range.offset), 0x20000);
    }
    assert!(
        write_sectors(
            &scene,
            &BTreeMap::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
            0x30000
        )
        .is_err()
    );
    let mut with_physical = env.clone();
    let mut physical = PhysicalEnvironment {
        path: AssetPath::new("level:/physical-environments/air")?,
        gravity: 17.0,
        vertical: [0.0, 0.0, 1.0],
        retained: [0; 32],
    };
    physical.retained[8] = 0xa5;
    with_physical.physical = Some(AssetRef::from_path(physical.path.clone()));
    assert!(
        write_environments(
            std::slice::from_ref(&with_physical),
            &[],
            std::slice::from_ref(&visual),
            std::slice::from_ref(&fog),
            &dependencies,
            0x20000
        )
        .is_err()
    );
    store.save(&AssetRef::from_path(physical.path.clone()), &physical)?;
    let restored: std::sync::Arc<PhysicalEnvironment> =
        store.load(&AssetRef::from_path(physical.path.clone()))?;
    assert_eq!(*restored, physical);
    physical.gravity = 19.5;
    physical.vertical = [0.0, 1.0, 0.0];
    store.save(&AssetRef::from_path(physical.path.clone()), &physical)?;
    let rebuilt = write_environments(
        &[with_physical],
        std::slice::from_ref(&physical),
        std::slice::from_ref(&visual),
        std::slice::from_ref(&fog),
        &dependencies,
        0x20000,
    )?;
    assert_eq!(rebuilt.ranges.len(), 4);
    assert_eq!(rebuilt.pointer_count, 3);
    assert_eq!(u32_at(&rebuilt.bytes, 0), 0x20030);
    assert_eq!(u32_at(&rebuilt.bytes, 48), 19.5f32.to_bits());
    assert_eq!(u32_at(&rebuilt.bytes, 56), 1.0f32.to_bits());
    assert_eq!(rebuilt.bytes[64 + 8], 0xa5);
    assert!(
        write_environments(
            std::slice::from_ref(&env),
            &[],
            &[],
            std::slice::from_ref(&fog),
            &dependencies,
            0x20000
        )
        .is_err()
    );
    assert!(
        write_environments(
            std::slice::from_ref(&env),
            &[],
            &[visual],
            &[],
            &dependencies,
            0x20000
        )
        .is_err()
    );
    edited.near = f32::NAN;
    assert!(write_fog(&edited).is_err());
    Ok(())
}

#[test]
#[ignore = "requires local Godot; included by tools/test.py --local"]
fn godot_environment_fog_sharing_and_mixed_sector_guard() -> Result<()> {
    let (env, visual, fog, mut scene) = fixture()?;
    let mut source = Collection::default();
    source.insert(&AssetRef::from_path(env.path.clone()), env)?;
    source.insert(&AssetRef::from_path(visual.path.clone()), visual)?;
    source.insert(&AssetRef::from_path(fog.path.clone()), fog)?;
    let temp = tempfile::tempdir()?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    for mixed in [false, true] {
        if mixed {
            scene.nodes[1].sector.as_mut().unwrap().environments.clear();
        }
        source.insert(&AssetRef::from_path(scene.path.clone()), scene.clone())?;
        let output = temp.path().join(if mixed { "mixed" } else { "uniform" });
        let report = godot::export(
            &mut source,
            &[ContentRoot {
                reference: scene.path.clone(),
                kind: Scene::KIND.into(),
            }],
            Selection::hype("fog-test", "en")?,
            &output,
            &engine,
        )?;
        assert_eq!(
            report
                .diagnostics
                .iter()
                .any(|s| s.contains("multiple fog environments")),
            mixed
        );
        let check=r#"extends SceneTree
func _initialize():
    var scene = load("res://generated/hype/levels/fog-test/level.tscn").instantiate()
    var env = scene.get_node("Environment").environment
    var expected = EXPECTED
    if env.fog_enabled != expected:
        push_error("sector fog conflict guard failed")
        quit(1)
        return
    if expected:
        if env.fog_mode != Environment.FOG_MODE_DEPTH or env.fog_density != 1.0 or env.fog_depth_begin != 2.0 or env.fog_depth_end != 12.0 or not env.fog_light_color.is_equal_approx(Color(0.2,0.5,0.8)):
            push_error("source fog settings lost")
            quit(1)
            return
    var metadata = JSON.parse_string(scene.get_meta("source_environments"))
    if metadata[0].size() != 1 or metadata[1].size() != 1 or metadata[2].size() != 1:
        push_error("shared environment duplicated")
        quit(1)
        return
    scene.free()
    quit(0)
"#.replace("EXPECTED",if mixed {"false"} else {"true"});
        std::fs::write(output.join("check.gd"), check)?;
        godot::run_godot(
            &engine,
            &output,
            &["--headless", "--script", "res://check.gd"],
            Duration::from_secs(15),
        )?;
    }
    Ok(())
}
