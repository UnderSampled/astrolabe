//! Connected source camera contracts and native component behavior.
use anyhow::{Context, Result, ensure};
use astrolabe::{
    godot,
    model::{
        AssetRef, Selection,
        ai::{ActorKeyword, ActorTarget, AiModel, Expression, Statement, walk::AstNode},
        character::Perso,
        io::Load,
        route::Waypoint,
    },
    rete,
};
use std::{fs, path::PathBuf, time::Duration};

#[test]
fn native_camera_target_motion_and_cleanup_match_source() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let project = temp.path().join("project");
    godot::support::initialize(&project)?;
    fs::write(
        project.join("check_camera_native_semantics.gd"),
        include_bytes!("check_camera_native_semantics.gd"),
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    for (name, arguments) in [("parser", vec!["--check-only"]), ("native", Vec::new())] {
        let log = project.join(format!("camera-{name}.log"));
        let mut args = vec![
            "--headless",
            "--log-file",
            log.to_str().context("log path")?,
        ];
        args.extend(arguments);
        args.extend(["--script", "res://check_camera_native_semantics.gd"]);
        let output = godot::run_godot(&engine, &project, &args, Duration::from_secs(15))?;
        let diagnostics = fs::read_to_string(log)?;
        ensure!(!diagnostics.contains("ERROR:"), "{diagnostics}");
        if name == "native" {
            ensure!(output.contains("CAMERA_NATIVE_SEMANTICS_OK"), "{output}");
        }
    }
    Ok(())
}

#[test]
#[ignore = "requires local Hype data; static connected slice, no original execution"]
fn manor_connected_camera_slice_preserves_shots_and_handoff() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let mut imported =
        astrolabe::openspace::content::import_scene(&game, Selection::hype("manoir", "en")?)?;
    let world = imported.load(&AssetRef::<AiModel>::new("level:/ai/models/154")?)?;
    let camera = imported.load(&AssetRef::<AiModel>::new("fix:/ai/models/437")?)?;
    let travel = imported.load(&AssetRef::<AiModel>::new("level:/ai/models/365")?)?;
    let gogoud = imported.load(&AssetRef::<AiModel>::new("level:/ai/models/210")?)?;
    let statue = imported.load(&AssetRef::<AiModel>::new("level:/ai/models/294")?)?;
    let mut rows = Vec::new();
    for model in [&world, &camera, &travel, &gogoud, &statue] {
        let reopened = rete::decode::<AiModel>(&rete::encode(model.as_ref())?)?;
        ensure!(reopened == **model);
        for set in [&model.normal, &model.reflex].into_iter().flatten() {
            for behavior in &set.behaviors {
                for (slot, script) in behavior
                    .rules
                    .iter()
                    .enumerate()
                    .filter_map(|(slot, rule)| rule.as_ref().map(|s| (format!("rule{slot}"), s)))
                    .chain(behavior.schedule.as_ref().map(|s| ("schedule".into(), s)))
                {
                    for (node, _) in script.walk() {
                        let AstNode::Statement(statement) = node else {
                            continue;
                        };
                        match statement {
                            Statement::CameraChangeTargetedPerso { camera, .. }
                            | Statement::CameraResetTargetedPerso { camera }
                            | Statement::CameraChangeState { camera, .. } => {
                                ensure!(camera.path.as_str() == "level:/persos/world/0/2");
                                imported.load(camera)?;
                            }
                            Statement::SetActorAbsolutePosition { actor, position }
                                if actor.path.as_str() == "level:/persos/world/0/2" =>
                            {
                                if let Expression::WaypointPosition(point) = position {
                                    let waypoint = imported.load(point)?;
                                    ensure!(waypoint.relative_actor.is_none());
                                    let reopened = rete::decode::<Waypoint>(&rete::encode(
                                        waypoint.as_ref(),
                                    )?)?;
                                    ensure!(reopened == *waypoint);
                                }
                            }
                            _ => continue,
                        }
                        rows.push(serde_json::json!({"model":model.path,"behavior":behavior.id,"slot":slot,"statement":statement}));
                    }
                }
            }
        }
    }
    let source_camera = imported.load(&AssetRef::<Perso>::new("level:/persos/world/0/2")?)?;
    let camera_component = source_camera.camera.as_ref().context("camera component")?;
    ensure!(camera_component.parameters[0].mode == 0);
    ensure!(camera_component.parameters[0].target_child_index() == -1);
    ensure!(camera_component.parameters[1].mode == 0);
    let reopened = rete::decode::<Perso>(&rete::encode(source_camera.as_ref())?)?;
    ensure!(reopened.camera == source_camera.camera);
    // Assert the connected source prerequisites, including the moving-eye
    // route and final restore. These are authored AST checks, not execution.
    let world_three = normal_statements(&world, 3, Some(1))?;
    ensure!(
        world_three.iter().any(|statement| matches!(statement,
        Statement::ChangeActorBehavior { actor: ActorTarget::Reference(actor), target }
            if actor.path.as_str() == "level:/persos/world/0/45"
                && target.model.path.as_str() == "level:/ai/models/365" && target.behavior == 2
        )),
        "World3 no longer starts the traveling camera route"
    );
    let travel_two = normal_statements(&travel, 2, Some(0))?;
    ensure!(
        travel_two.iter().any(|statement| matches!(statement,
            Statement::SetActorAbsolutePosition { actor, position: Expression::SelfPosition }
            if actor.path.as_str() == "level:/persos/world/0/2"
        )),
        "traveling camera no longer copies its native eye position"
    );
    ensure!(travel_two.iter().any(|statement| matches!(statement,
        Statement::If { condition: Expression::WaypointSphereContains { waypoint, .. }, .. }
        if matches!(waypoint.as_ref(), Expression::WaypointReference(point)
            if point.path.as_str() == "level:/waypoints/17-1/135")
    )));
    ensure!(
        travel_two.iter().any(|statement| matches!(
            statement,
            Statement::CameraChangeState {
                state: Expression::Integer(6),
                ..
            }
        )),
        "traveling route lost its required mode6 camera request"
    );
    ensure!(travel_two.iter().any(|statement| matches!(statement,
        Statement::ActorFieldAssign { actor, variable: 0, value: Expression::Integer(1) }
        if actor.path.as_str() == "level:/persos/world/0/28"
    )));
    let travel_three = normal_statements(&travel, 3, Some(0))?;
    ensure!(
        travel_three.iter().any(|statement| matches!(statement,
            Statement::ActorFieldAssign { actor, variable: 1, value: Expression::Integer(1) }
            if actor.path.as_str() == "level:/persos/world/0/5"
        )),
        "return travel route lost its Gogoud handoff field"
    );
    let final_gogoud = normal_statements(&gogoud, 12, None)?;
    let main_target = final_gogoud
        .iter()
        .position(|statement| {
            matches!(
                statement,
                Statement::CameraChangeTargetedPerso {
                    target: ActorTarget::Keyword(ActorKeyword::MainActor),
                    ..
                }
            )
        })
        .context("Gogoud12 MainActor target")?;
    let restore = final_gogoud
        .iter()
        .position(|statement| {
            matches!(
                statement,
                Statement::CameraChangeState {
                    state: Expression::Integer(-1),
                    ..
                }
            )
        })
        .context("Gogoud12 initial camera-mode restore")?;
    ensure!(main_target < restore);
    for point in travel.waypoint_references() {
        imported.load(point)?;
    }
    let evidence = serde_json::json!({"scope":"static import and Rete equality; no Godot or original intro acceptance", "camera_calls": rows, "owner_motion_binding_present": camera_component.owner_motion_binding_present});
    if let Some(path) = std::env::var_os("ASTROLABE_CAMERA_AUDIT_REPORT") {
        fs::write(path, serde_json::to_vec_pretty(&evidence)?)?;
    }
    println!(
        "CAMERA_CONNECTED_SOURCE_OK: {} camera calls, five connected models",
        rows.len()
    );
    Ok(())
}

fn normal_statements(
    model: &AiModel,
    behavior: u32,
    rule: Option<usize>,
) -> Result<Vec<&Statement>> {
    let behavior = model
        .normal
        .as_ref()
        .context("normal behavior set")?
        .behaviors
        .iter()
        .find(|candidate| candidate.id == behavior)
        .context("normal behavior")?;
    let script = match rule {
        Some(index) => behavior.rules.get(index).and_then(Option::as_ref),
        None => behavior.schedule.as_ref(),
    }
    .context("source script")?;
    Ok(script
        .walk()
        .filter_map(|(node, _)| match node {
            AstNode::Statement(statement) => Some(statement),
            _ => None,
        })
        .collect())
}
