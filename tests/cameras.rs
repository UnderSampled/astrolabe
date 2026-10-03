//! Native saved camera projection after editing the canonical/Rete value.
use anyhow::{Context, Result, ensure};
use astrolabe::{
    godot,
    model::{
        AssetPath, AssetRef, Selection,
        ai::{SourceObjectIdentity, SourceRole},
        camera::{Camera, CameraParameters},
        character::{Family, Perso},
        content::Collection,
        geometry::SourceTransform,
        io::{Content, ContentRoot, Load, Save},
        scene::{NodeKind, Scene, SceneNode, SceneNodeProperties},
    },
    rete,
};
use std::{fs, time::Duration};

#[test]
fn exported_camera_uses_edited_horizontal_fov_and_source_view_axis() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let project = temp.path().join("godot");
    godot::support::initialize(&project)?;
    fs::write(
        project.join("check_camera_projection.gd"),
        include_bytes!("check_camera_projection.gd"),
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let parser_log = project.join("camera-parser.log");
    godot::run_godot(
        &engine,
        &project,
        &[
            "--headless",
            "--log-file",
            parser_log.to_str().context("log path")?,
            "--check-only",
            "--script",
            "res://check_camera_projection.gd",
        ],
        Duration::from_secs(10),
    )?;
    let diagnostics = fs::read_to_string(parser_log)?;
    ensure!(
        !diagnostics.contains("ERROR:"),
        "camera preflight failed: {diagnostics}"
    );

    let family = Family {
        path: AssetPath::new("fix:/families/camera")?,
        name: "Camera".into(),
        object_type: 7,
        bounding_geometry: None,
        states: vec![],
        configurations: vec![],
        default_configuration: None,
        animation_bank: 0,
        unknown: [0; 3],
    };
    let identity = |allocation| SourceObjectIdentity {
        role: SourceRole::Fix,
        block: [5, 0],
        allocation,
    };
    let parameters = CameraParameters {
        source: identity(2),
        target_offset: [0.; 3],
        look_offset: [0.; 3],
        actor_offset: [0., 0., 1.8],
        distance: [3.5, 4.],
        yaw: 0.,
        pitch: 0.3,
        horizontal_fov: 1.5,
        mode: 0,
        unknown: vec![0; 104],
    };
    let mut actor = Perso {
        path: AssetPath::new("level:/persos/world/camera")?,
        name: "SourceCameraActor".into(),
        family: AssetRef::from_path(family.path.clone()),
        initial_state: None,
        current_state: None,
        secondary_state: None,
        configuration: None,
        initial_configuration: None,
        object_types: [7, 0, 0],
        camera: Some(Camera {
            source: identity(1),
            clip_planes: [0.25, 10000.],
            unknown: vec![0; 196],
            parameters: std::array::from_fn(|_| parameters.clone()),
        }),
        routes: vec![],
        brain: None,
        collisions: None,
    };
    let selection = Selection::hype("manoir", "en")?;
    let mut store = rete::Store::new(temp.path().join("rete"), selection.clone())?;
    let reference = AssetRef::from_path(actor.path.clone());
    store.save(&reference, &actor)?;
    actor = (*store.load(&reference)?).clone();
    actor.camera.as_mut().unwrap().parameters[1].horizontal_fov = 1.2;
    store.save(&reference, &actor)?;
    let actor = store.load(&reference)?;
    let scene = Scene {
        path: AssetPath::new("level:/scene")?,
        name: "CameraCheck".into(),
        main_actor: None,
        global_actors: vec![],
        nodes: vec![
            SceneNode {
                id: "world".into(),
                name: "world".into(),
                parent: None,
                kind: NodeKind::World,
                properties: SceneNodeProperties::default(),
                transform: None,
                instance: None,
                perso: None,
                collision: None,
                draw_mask: 0,
                sector: None,
                light: None,
            },
            SceneNode {
                id: "world/camera".into(),
                name: "camera".into(),
                parent: Some("world".into()),
                kind: NodeKind::Perso,
                properties: SceneNodeProperties::default(),
                transform: Some(SourceTransform {
                    translation: [3., 4., 2.],
                    rotation: [[0., 1., 0.], [-1., 0., 0.], [0., 0., 1.]],
                    basis: [[0., 1., 0.], [-1., 0., 0.], [0., 0., 1.]],
                }),
                instance: None,
                perso: Some(reference.clone()),
                collision: None,
                draw_mask: 0,
                sector: None,
                light: None,
            },
        ],
    };
    let mut content = Collection::default();
    content.insert(&AssetRef::from_path(family.path.clone()), family)?;
    content.insert(&reference, (*actor).clone())?;
    let roots = [ContentRoot {
        kind: Scene::KIND.into(),
        reference: scene.path.clone(),
    }];
    content.insert(&AssetRef::from_path(scene.path.clone()), scene)?;
    godot::export(&mut content, &roots, selection, &project, &engine)?;
    let log = project.join("camera-check.log");
    let evidence = godot::run_godot(
        &engine,
        &project,
        &[
            "--headless",
            "--log-file",
            log.to_str().context("log path")?,
            "--script",
            "res://check_camera_projection.gd",
        ],
        Duration::from_secs(10),
    )?;
    ensure!(
        evidence.contains("SOURCE_CAMERA_PROJECTION_OK"),
        "{evidence}"
    );
    Ok(())
}
