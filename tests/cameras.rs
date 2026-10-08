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
    let mut parameters = CameraParameters {
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
    parameters.unknown[88] = 1;
    parameters.unknown[96..100].copy_from_slice(&1_u32.to_le_bytes());
    // The intro manor camera uses -1 to select the bound Perso itself.
    parameters.unknown[100] = 0xff;
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
        hit_points: None,
        camera: Some(Camera {
            source: identity(1),
            clip_planes: [0.25, 10000.],
            unknown: vec![0; 196],
            parameters: std::array::from_fn(|_| parameters.clone()),
        }),
        routes: vec![],
        brain: None,
        stock: None,
        particles: None,
        collisions: None,
    };
    let main_actor_ref = AssetRef::<Perso>::new("level:/persos/world/main")?;
    let mut main_actor = actor.clone();
    main_actor.path = main_actor_ref.path.clone();
    main_actor.name = "MainActor".into();
    main_actor.camera = None;
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
        main_actor: Some(main_actor_ref.clone()),
        armor: None,
        hud: None,
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
            SceneNode {
                id: "world/main".into(),
                name: "main".into(),
                parent: Some("world".into()),
                kind: NodeKind::Perso,
                properties: SceneNodeProperties::default(),
                transform: Some(SourceTransform {
                    translation: [0.; 3],
                    rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                    basis: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                }),
                instance: None,
                perso: Some(main_actor_ref.clone()),
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
    content.insert(&main_actor_ref, main_actor)?;
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

/// A checked, contiguous section of the original World initialization schedule.
/// The prefix has unsupported presentation/game-state calls, so this excerpt
/// is component evidence and must never be reported as the full opening.
fn opening_shot_excerpt(
    model: &astrolabe::model::ai::AiModel,
) -> Result<Vec<astrolabe::model::ai::Statement>> {
    use astrolabe::model::ai::{ActorTarget, BehaviorGroup, Comparison, Expression, Statement};
    ensure!(model.path.as_str() == "level:/ai/models/154" && model.name == "IWorld_Manoir");
    let behavior = model
        .normal
        .as_ref()
        .context("World normal set")?
        .behaviors
        .iter()
        .find(|b| b.id == 2)
        .context("World initial behavior 2")?;
    let original = &behavior
        .schedule
        .as_ref()
        .context("World initialization schedule")?
        .statements;
    let then = original.iter().find_map(|s| match s {
        Statement::If { condition: Expression::Compare { left, operation: Comparison::Equal, right }, then, .. }
            if matches!(left.as_ref(), Expression::ActorFieldRead {actor, variable:147} if actor.path.as_str()=="fix:/persos/15") && **right == Expression::Integer(0) => Some(then),
        _ => None,
    }).context("first-visit opening branch")?;
    ensure!(then.len() >= 12);
    let shots = then[then.len() - 12..].to_vec();
    ensure!(
        astrolabe::model::ai::Script {
            statements: shots.clone()
        }
        .unsupported_fragments()
        .is_empty()
    );
    for (shot, (helper, waypoint)) in [(33, 86), (18, 84), (19, 82)].into_iter().enumerate() {
        let offset = shot * 4;
        let camera_path = "level:/persos/world/0/2";
        ensure!(
            matches!(&shots[offset], Statement::CameraChangeTargetedPerso {camera,target:ActorTarget::Reference(target)} if camera.path.as_str()==camera_path && target.path.as_str()==format!("level:/persos/world/0/{helper}"))
        );
        ensure!(
            matches!(&shots[offset+1], Statement::SetActorAbsolutePosition {actor,position:Expression::WaypointPosition(point)} if actor.path.as_str()==camera_path && point.path.as_str()==format!("level:/waypoints/17-1/{waypoint}"))
        );
        ensure!(
            matches!(&shots[offset+2], Statement::CameraChangeState {camera,state:Expression::Integer(3)} if camera.path.as_str()==camera_path)
        );
    }
    ensure!(
        shots[3]
            == Statement::Wait {
                milliseconds: Expression::Integer(3600)
            }
    );
    ensure!(
        shots[7]
            == Statement::Wait {
                milliseconds: Expression::Integer(4500)
            }
    );
    ensure!(
        matches!(&shots[11], Statement::ChangeBehavior {target,pause_after:false} if target.model.path==model.path && target.group==BehaviorGroup::Normal && target.behavior==3)
    );
    Ok(shots)
}

#[test]
#[ignore = "requires local Hype; source/Rete evidence without native execution"]
fn manor_opening_camera_commands_and_waypoints_survive_rete() -> Result<()> {
    use astrolabe::model::{ai::AiModel, route::Waypoint};
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let mut imported =
        astrolabe::openspace::content::import_scene(&game, Selection::hype("manoir", "en")?)?;
    let reference = AssetRef::<AiModel>::new("level:/ai/models/154")?;
    let original = imported.load(&reference)?;
    let shots = opening_shot_excerpt(&original)?;
    let gogoud_ref = AssetRef::<AiModel>::new("level:/ai/models/210")?;
    let gogoud = imported.load(&gogoud_ref)?;
    verify_fourth_shot_source(&original, &gogoud)?;
    let points = [86, 84, 82, 67]
        .into_iter()
        .map(|n| {
            imported.load(&AssetRef::<Waypoint>::new(format!(
                "level:/waypoints/17-1/{n}"
            ))?)
        })
        .collect::<Result<Vec<_>>>()?;
    let observed: [[f64; 3]; 4] = [
        [-16.6989136, 155.743668, 33.2094803],
        [-8.26678085, 147.569885, 41.2058792],
        [-11.9290009, 158.426208, 34.5001259],
        [-17.185480118, 146.039352417, 34.619735718],
    ];
    for (point, eye) in points.iter().zip(observed) {
        ensure!(point.relative_actor.is_none());
        ensure!(
            point
                .position
                .iter()
                .zip(eye)
                .all(|(a, b)| (f64::from(*a) - b).abs() < 0.00005),
            "waypoint {} differs from observed original eye",
            point.path
        );
    }
    let temp = tempfile::tempdir()?;
    astrolabe::app::save_rete(&mut imported, temp.path())?;
    let (mut reopened, _) = rete::Store::open(temp.path())?;
    ensure!(*reopened.load(&reference)? == *original);
    ensure!(*reopened.load(&gogoud_ref)? == *gogoud);
    ensure!(opening_shot_excerpt(reopened.load(&reference)?.as_ref())? == shots);
    for point in points {
        ensure!(*reopened.load(&AssetRef::<Waypoint>::from_path(point.path.clone()))? == *point);
    }
    let camera = reopened.load(&AssetRef::<Perso>::new("level:/persos/world/0/2")?)?;
    ensure!(camera.camera.as_ref().context("source camera")?.parameters[1].mode == 0);
    Ok(())
}

#[test]
#[ignore = "requires local Hype and Godot; actual three-shot excerpt, not full intro acceptance"]
fn imported_manor_opening_shot_excerpt_selects_eyes_targets_and_waits() -> Result<()> {
    run_manor_shot_excerpt(false)
}

#[test]
#[ignore = "requires local Hype and Godot; original Gogoud AI with a declared World initialization excerpt"]
fn imported_gogoud_opening_ai_drives_fourth_camera_cut() -> Result<()> {
    run_manor_shot_excerpt(true)
}

fn verify_fourth_shot_source(
    world: &astrolabe::model::ai::AiModel,
    gogoud: &astrolabe::model::ai::AiModel,
) -> Result<()> {
    use astrolabe::model::ai::{ActorTarget, Comparison, Expression, Statement};
    let behavior = |model: &astrolabe::model::ai::AiModel, id| {
        model
            .normal
            .as_ref()
            .unwrap()
            .behaviors
            .iter()
            .find(|b| b.id == id)
            .cloned()
    };
    ensure!(
        behavior(world, 2)
            .context("World normal 2")?
            .rules
            .is_empty()
    );
    let producer = behavior(gogoud, 0).context("Gogoud route behavior")?;
    let statements = &producer
        .schedule
        .context("Gogoud route schedule")?
        .statements;
    ensure!(statements.len() >= 6);
    ensure!(matches!(statements[0], Statement::GoOverRoute(6)));
    for (index, state) in [(1, 26), (2, 24)] {
        ensure!(
            matches!(&statements[index], Statement::ExecuteAction(Expression::Action(Some(action))) if action.family.path.as_str()=="level:/families/27" && action.state==state)
        );
    }
    ensure!(
        matches!(&statements[3], Statement::ChangeAction(Expression::Action(Some(action))) if action.family.path.as_str()=="level:/families/27" && action.state==23)
    );
    ensure!(
        matches!(&statements[4], Statement::ActorFieldAssign {actor,variable:0,value:Expression::Integer(4)} if actor.path.as_str()=="level:/persos/world/0/3")
    );
    ensure!(matches!(
        &statements[5],
        Statement::Wait {
            milliseconds: Expression::Integer(1200)
        }
    ));
    let receiver = behavior(world, 3).context("World fourth-shot behavior")?;
    let rule = receiver.rules[0]
        .as_ref()
        .context("World fourth-shot rule")?;
    let Statement::If {
        condition, then, ..
    } = &rule.statements[0]
    else {
        anyhow::bail!("World fourth-shot branch missing");
    };
    ensure!(
        matches!(condition, Expression::Compare {left,operation:Comparison::Equal,right} if **left==Expression::Variable(0) && **right==Expression::Integer(4))
    );
    ensure!(then.len() == 3);
    ensure!(
        matches!(&then[0], Statement::CameraChangeTargetedPerso {camera,target:ActorTarget::Reference(target)} if camera.path.as_str()=="level:/persos/world/0/2" && target.path.as_str()=="level:/persos/world/0/20")
    );
    ensure!(
        matches!(&then[1], Statement::SetActorAbsolutePosition {actor,position:Expression::WaypointPosition(point)} if actor.path.as_str()=="level:/persos/world/0/2" && point.path.as_str()=="level:/waypoints/17-1/67")
    );
    ensure!(
        matches!(&then[2], Statement::CameraChangeState {camera,state:Expression::Integer(3)} if camera.path.as_str()=="level:/persos/world/0/2")
    );
    Ok(())
}

fn run_manor_shot_excerpt(with_gogoud: bool) -> Result<()> {
    use astrolabe::model::ai::{AiModel, DesignerValue, Script};
    use sha2::{Digest, Sha256};
    let (stem, harness, script, marker) = if with_gogoud {
        (
            "manor-gogoud-fourth-shot",
            "check_gogoud_camera.gd",
            include_bytes!("check_gogoud_camera.gd").as_slice(),
            "MANOR_GOGOUD_FOURTH_SHOT_OK",
        )
    } else {
        (
            "manor-source-shot-excerpt",
            "check_manor_shots.gd",
            include_bytes!("check_manor_shots.gd").as_slice(),
            "MANOR_SOURCE_SHOT_EXCERPT_OK",
        )
    };
    let temp = tempfile::tempdir()?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let evidence_root = std::env::var_os("ASTROLABE_EVIDENCE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| "TestResults".into());
    fs::create_dir_all(&evidence_root)?;
    let evidence = evidence_root.canonicalize()?.join(format!(
        "{stem}-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    fs::create_dir(&evidence)?;
    let syntax = temp.path().join("syntax");
    fs::create_dir(&syntax)?;
    fs::write(syntax.join("project.godot"), "config_version=5\n")?;
    fs::write(
        syntax.join("check_manor_shots.gd"),
        include_bytes!("check_manor_shots.gd"),
    )?;
    fs::write(syntax.join(harness), script)?;
    let harness_argument = format!("res://{harness}");
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
            &harness_argument,
        ],
        Duration::from_secs(10),
    )?;
    ensure!(
        fs::metadata(&parser_log)?.len() <= 1024 * 1024
            && !fs::read_to_string(&parser_log)?.contains("ERROR:")
    );
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let selection = Selection::hype("manoir", "en")?;
    let mut imported = astrolabe::openspace::content::import_scene(&game, selection.clone())?;
    let model_ref = AssetRef::<AiModel>::new("level:/ai/models/154")?;
    let original = imported.load(&model_ref)?;
    let shots = opening_shot_excerpt(&original)?;
    let original_hash = format!("{:x}", Sha256::digest(rete::encode(original.as_ref())?));
    let camera_model_ref = AssetRef::<AiModel>::new("fix:/ai/models/437")?;
    let camera_model = imported.load(&camera_model_ref)?;
    let camera_hash = format!("{:x}", Sha256::digest(rete::encode(camera_model.as_ref())?));
    let gogoud_model_ref = AssetRef::<AiModel>::new("level:/ai/models/210")?;
    let gogoud_model = imported.load(&gogoud_model_ref)?;
    let gogoud_hash = format!("{:x}", Sha256::digest(rete::encode(gogoud_model.as_ref())?));
    let gogoud_ref = AssetRef::<Perso>::new("level:/persos/world/0/5")?;
    let gogoud = imported.load(&gogoud_ref)?;
    if with_gogoud {
        verify_fourth_shot_source(&original, &gogoud_model)?;
        let world = imported.load(&AssetRef::<Perso>::new("level:/persos/world/0/3")?)?;
        ensure!(
            world.brain.as_ref().context("World Brain")?.variables[0] == DesignerValue::Integer(0)
        );
        ensure!(gogoud.brain.as_ref().context("Gogoud Brain")?.model == gogoud_model_ref);
    }
    let rete_root = temp.path().join("rete");
    astrolabe::app::save_rete(&mut imported, &rete_root)?;
    let (mut reopened, _) = rete::Store::open(&rete_root)?;
    ensure!(
        *reopened.load(&model_ref)? == *original,
        "full source model changed during Rete round trip"
    );
    let camera_ref = AssetRef::<Perso>::new("level:/persos/world/0/2")?;
    let camera = reopened.load(&camera_ref)?;
    ensure!(camera.camera.as_ref().context("source camera")?.parameters[1].mode == 0);
    let mut excerpt = (*original).clone();
    let behavior = excerpt
        .normal
        .as_mut()
        .unwrap()
        .behaviors
        .iter_mut()
        .find(|b| b.id == 2)
        .unwrap();
    behavior.schedule = Some(Script {
        statements: shots.clone(),
    });
    let fixture_hash = format!("{:x}", Sha256::digest(rete::encode(&excerpt)?));
    // Only the temporary World schedule is narrowed. All selected commands,
    // waits, references and other behaviors remain those of the source model.
    reopened.save(&model_ref, &excerpt)?;
    drop(reopened);
    let (mut reopened, package) = rete::Store::open(&rete_root)?;
    ensure!(*reopened.load(&model_ref)? == excerpt && *reopened.load(&camera_ref)? == *camera);
    ensure!(
        *reopened.load(&camera_model_ref)? == *camera_model,
        "camera AI changed in excerpt fixture"
    );
    ensure!(
        *reopened.load(&gogoud_model_ref)? == *gogoud_model
            && *reopened.load(&gogoud_ref)? == *gogoud,
        "Gogoud AI or actor changed in excerpt fixture"
    );
    let setup = if with_gogoud {
        "World normal coroutine runs the declared excerpt and unchanged behavior 3; World reflex and other Brains held. Actual unchanged Gogoud AI starts normally and drives native route/action/contact, then assigns World.dsg_0=4. Viewport activated and unchanged camera aiming rule invoked by harness; no shot/trigger assignments. Stops at fourth cut before following dialogue."
    } else {
        "Other Brains held; viewport activated by harness. World excerpt clock/coroutine driven on native physics frames; unchanged camera rule invoked for aiming. Camera eye/target/mode selected only by the imported source commands."
    };
    fs::write(
        evidence.join("fixture.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "scene":"manoir","model":model_ref.path,"original_model_rete_sha256":original_hash,
        "fixture_model_rete_sha256":fixture_hash,"ai_modified":true,
        "camera_model_rete_sha256":camera_hash,"camera_ai_modified":false,
        "gogoud_model_rete_sha256":gogoud_hash,"gogoud_ai_modified":false,"with_gogoud":with_gogoud,
            "extraction":"World behavior 2, first-visit branch, final 12 original statements (source nodes 909 through 947); earlier initialization/branch guards are excluded",
            "commands":shots,"source_mode":0,"fixture_mode_edit":false,
            "setup":setup,
            "pending":["complete initialization and first-visit guards","remaining opening shots","dialogue and presentation","full approximately 209-second intro"]
        }))?,
    )?;
    let project = temp.path().join("project");
    godot::export(&mut reopened, &package.roots, selection, &project, &engine)?;
    fs::write(
        project.join("check_manor_shots.gd"),
        include_bytes!("check_manor_shots.gd"),
    )?;
    fs::write(project.join(harness), script)?;
    // Preserve the compiled source section after the disposable project exits.
    fs::copy(
        project.join("generated/hype/levels/manoir/ai/models/154.gd"),
        evidence.join("compiled-world-excerpt.gd"),
    )?;
    if with_gogoud {
        fs::copy(
            project.join("generated/hype/levels/manoir/ai/models/210.gd"),
            evidence.join("compiled-gogoud.gd"),
        )?;
    }
    let log = evidence.join("godot.log");
    let report = evidence.join("report.json");
    let output = godot::run_godot_gameplay(
        &engine,
        &project,
        &[
            "--headless",
            "--log-file",
            log.to_str().context("Godot log path")?,
            "--script",
            &harness_argument,
            "--",
            report.to_str().context("shot report path")?,
        ],
        Duration::from_secs(25),
    )?;
    fs::write(evidence.join("stdout.log"), &output)?;
    ensure!(output.contains(marker), "{output}");
    println!(
        "Original shot excerpt (Gogoud integration: {with_gogoud}) evidence: {}",
        evidence.display()
    );
    Ok(())
}
