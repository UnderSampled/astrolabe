//! Connected source camera contracts and native component behavior.
use anyhow::{Context, Result, ensure};
use astrolabe::{
    godot,
    model::{
        AssetRef, Selection,
        ai::{ActorKeyword, ActorTarget, AiModel, Expression, Statement, walk::AstNode},
        armor::ArmorCatalog,
        character::Perso,
        io::Load,
        mechanics::{Mechanics, MechanicsData},
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
    let armor = imported.load(&AssetRef::<ArmorCatalog>::new("fix:/gameplay/armor")?)?;
    ensure!(armor.selected == Some(0) && !armor.definitions.is_empty());
    ensure!(rete::decode::<ArmorCatalog>(&rete::encode(armor.as_ref())?)? == *armor);
    let world_setup = normal_statements(&world, 2, None)?;
    ensure!(
        world_setup.iter().any(|statement| matches!(
            statement,
            Statement::SetArmorResistance(Expression::Integer(0))
        )),
        "World154 first-visit schedule lost Proc430(0)"
    );
    let main_actor = imported.load(&AssetRef::<Perso>::new("level:/persos/world/0/0")?)?;
    let family = imported.load(&main_actor.family)?;
    ensure!(main_actor.initial_state == Some(47));
    let no_card = family
        .states
        .iter()
        .find(|state| state.id == 343)
        .context("MainActor first-visit state343")?;
    ensure!(no_card.mechanics.is_none());
    let main_brain = imported.load(&main_actor.brain.as_ref().context("MainActor Brain")?.model)?;
    ensure!(rete::decode::<AiModel>(&rete::encode(main_brain.as_ref())?)? == *main_brain);
    ensure!(
        normal_statements(&main_brain, 0, None)?
            .iter()
            .any(|statement| matches!(
                statement,
                Statement::ChangeAction(Expression::Action(Some(action))) if action.state == 343
            )),
        "MainActor initialization lost its conditional first-visit no-card action"
    );
    let mut main_mechanics = Vec::new();
    for state in &family.states {
        let Some(reference) = &state.mechanics else {
            continue;
        };
        let card = imported.load(reference)?;
        ensure!(rete::decode::<Mechanics>(&rete::encode(card.as_ref())?)? == *card);
        let kind = match &card.data {
            MechanicsData::Character { mode, .. } => *mode,
            MechanicsData::Mass { .. } => 9,
            MechanicsData::Unresolved { kind, .. } => *kind,
        };
        main_mechanics.push((state.id, kind, reference.path.clone()));
    }
    for state_index in [10, 24, 20, 70] {
        ensure!(
            main_mechanics
                .iter()
                .any(|(state, kind, _)| *state == state_index && *kind == 4),
            "MainActor state {state_index} lacks the shared Walk publisher's kind-4 binding"
        );
    }
    println!(
        "MAIN_ACTOR_CAMERA_MECHANICS: {:?}",
        main_mechanics
            .iter()
            .filter(|(state, _, _)| [0, 10, 20, 24, 47, 48, 70].contains(state))
            .collect::<Vec<_>>()
    );
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
    let evidence = serde_json::json!({"scope":"static import and Rete equality; no Godot or original intro acceptance", "camera_calls": rows, "owner_motion_binding_present": camera_component.owner_motion_binding_present, "selected_armor": armor.selected, "main_actor_mechanics": main_mechanics, "main_actor_no_card_state": no_card.id, "main_actor_initialization_model": main_brain.path});
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
