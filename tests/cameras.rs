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
use std::{
    fs,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

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

#[test]
#[ignore = "requires local Hype and Godot; actual camera rule component, not intro acceptance"]
fn imported_camera_rule_aims_at_manor_helpers_while_native_eye_moves() -> Result<()> {
    use astrolabe::model::ai::AiModel;
    use sha2::{Digest, Sha256};

    let temp = tempfile::tempdir()?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let evidence_root = std::env::var_os("ASTROLABE_EVIDENCE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| "TestResults".into());
    fs::create_dir_all(&evidence_root)?;
    let id = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let evidence = evidence_root
        .canonicalize()?
        .join(format!("manor-native-camera-rule-{id}"));
    fs::create_dir(&evidence)?;

    // Parse the actual harness before the expensive source/Rete/scene conversion.
    let syntax = temp.path().join("syntax");
    fs::create_dir(&syntax)?;
    fs::write(syntax.join("project.godot"), "config_version=5\n")?;
    fs::write(
        syntax.join("check_manor_camera.gd"),
        include_bytes!("check_manor_camera.gd"),
    )?;
    let parser_log = evidence.join("parser.log");
    godot::run_godot(
        &engine,
        &syntax,
        &[
            "--headless",
            "--log-file",
            parser_log.to_str().context("parser log path")?,
            "--check-only",
            "--script",
            "res://check_manor_camera.gd",
        ],
        Duration::from_secs(10),
    )?;
    ensure!(fs::metadata(&parser_log)?.len() <= 1024 * 1024);
    ensure!(!fs::read_to_string(&parser_log)?.contains("ERROR:"));

    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("disc"));
    let selection = Selection::hype("manoir", "en")?;
    let mut imported = astrolabe::openspace::content::import_scene(&game, selection.clone())?;
    let camera_ref = AssetRef::<Perso>::new("level:/persos/world/0/2")?;
    let camera = imported.load(&camera_ref)?;
    let brain = camera.brain.as_ref().context("source camera Brain")?;
    ensure!(brain.model.path.as_str() == "fix:/ai/models/437");
    let model: std::sync::Arc<AiModel> = imported.load(&brain.model)?;
    let behavior = model
        .normal
        .as_ref()
        .context("source normal behavior set")?
        .behaviors
        .iter()
        .find(|behavior| behavior.id == 0)
        .context("source first camera behavior")?;
    let rule = behavior.rules[0]
        .as_ref()
        .context("source first camera rule")?;
    ensure!(rule.unsupported_fragments().is_empty());
    let model_hash = format!("{:x}", Sha256::digest(rete::encode(model.as_ref())?));
    for (ordinal, name) in [
        (33, "iDummy14_Intro_Manoir"),
        (18, "iDummy1_Intro_Manoir"),
        (19, "iDummy2_Intro_Manoir"),
        (20, "iDummy3_Intro_Manoir"),
    ] {
        let actor = imported.load(&AssetRef::<Perso>::new(format!(
            "level:/persos/world/0/{ordinal}"
        ))?)?;
        ensure!(actor.name == name && actor.brain.is_none());
    }
    let source_mode = camera
        .camera
        .as_ref()
        .context("source camera framing")?
        .parameters[1]
        .mode;
    ensure!(source_mode == 0, "source initial camera mode changed");
    let rete_root = temp.path().join("rete");
    astrolabe::app::save_rete(&mut imported, &rete_root)?;
    let (mut reopened, _) = rete::Store::open(&rete_root)?;
    // The source file starts in mode 0. The opening capture has mode 3. This
    // declared fixture edit exercises that component without inventing AI code
    // or pretending the unimplemented source ChangeState command ran.
    let mut configured = (*reopened.load(&camera_ref)?).clone();
    configured.camera.as_mut().unwrap().parameters[1].mode = 3;
    reopened.save(&camera_ref, &configured)?;
    drop(reopened);
    let (mut reopened, package) = rete::Store::open(&rete_root)?;
    ensure!(
        *reopened.load(&brain.model)? == *model,
        "camera AI was changed"
    );
    ensure!(*reopened.load(&camera_ref)? == configured);
    fs::write(
        evidence.join("fixture.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "scene": "manoir", "camera": camera_ref.path, "model": brain.model.path,
            "model_rete_sha256": model_hash, "source_mode": source_mode, "fixture_mode": 3,
            "compiled_rule": "normal_0_rule_0", "ai_modified": false,
            "setup": "Other Brains held; recorded opening eye placements and target bindings supplied by harness; subsequent native AnimationPlayer motion is authored test setup",
            "source_evidence": "TestResults/manor-camera-target-source-v6-sol-20261003/report.json",
            "pending": ["original shot commands/timing", "mode-0 follow and obstruction", "initial target binding", "viewport selection", "full intro"]
        }))?,
    )?;
    let project = temp.path().join("project");
    godot::export(&mut reopened, &package.roots, selection, &project, &engine)?;
    fs::write(
        project.join("check_manor_camera.gd"),
        include_bytes!("check_manor_camera.gd"),
    )?;
    let log = evidence.join("godot.log");
    let report = evidence.join("report.json");
    let output = godot::run_godot_gameplay(
        &engine,
        &project,
        &[
            "--headless",
            "--log-file",
            log.to_str().context("camera log path")?,
            "--script",
            "res://check_manor_camera.gd",
            "--",
            report.to_str().context("camera report path")?,
        ],
        Duration::from_secs(20),
    )?;
    fs::write(evidence.join("stdout.log"), &output)?;
    ensure!(output.contains("MANOR_SOURCE_CAMERA_RULE_OK"), "{output}");
    println!(
        "Actual manor camera rule component evidence: {}",
        evidence.display()
    );
    Ok(())
}
