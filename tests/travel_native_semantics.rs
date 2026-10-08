//! Static prerequisites for the connected manor traveling-camera API.
use anyhow::{Context, Result, ensure};
use astrolabe::{
    godot,
    model::{
        AssetRef, Selection,
        ai::AiModel,
        character::{Family, Perso},
        io::Load,
        mechanics::{Mechanics, MechanicsData},
        route::{Route, Waypoint},
    },
    rete,
};
use std::{fs, path::PathBuf, time::Duration};

#[test]
fn native_point_turn_geometry_uses_source_coordinates() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let project = temp.path().join("project");
    godot::support::initialize(&project)?;
    fs::write(
        project.join("check_travel_native_semantics.gd"),
        include_bytes!("check_travel_native_semantics.gd"),
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    for (name, arguments) in [("parser", vec!["--check-only"]), ("native", Vec::new())] {
        let log = project.join(format!("point-turn-{name}.log"));
        let mut args = vec![
            "--headless",
            "--log-file",
            log.to_str().context("log path")?,
        ];
        args.extend(arguments);
        args.extend(["--script", "res://check_travel_native_semantics.gd"]);
        let output = godot::run_godot(&engine, &project, &args, Duration::from_secs(15))?;
        let diagnostics = fs::read_to_string(log)?;
        ensure!(!diagnostics.contains("ERROR:"), "{diagnostics}");
        if name == "native" {
            ensure!(output.contains("TRAVEL_POINT_TURN_GEOMETRY_OK"), "{output}");
        }
    }
    Ok(())
}

#[test]
#[ignore = "requires local Hype data; static prerequisites, no original execution"]
fn manor_travel_dependencies_are_bound() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let mut source =
        astrolabe::openspace::content::import_scene(&game, Selection::hype("manoir", "en")?)?;
    let mut actors = Vec::new();
    for actor in [45, 5] {
        let perso = source.load(&AssetRef::<Perso>::new(format!(
            "level:/persos/world/0/{actor}"
        ))?)?;
        ensure!(rete::decode::<Perso>(&rete::encode(perso.as_ref())?)? == *perso);
        let family = source.load(&perso.family)?;
        let reopened = rete::decode::<Family>(&rete::encode(family.as_ref())?)?;
        ensure!(reopened == *family);
        let brain = source.load(&perso.brain.as_ref().context("actor brain")?.model)?;
        ensure!(rete::decode::<AiModel>(&rete::encode(brain.as_ref())?)? == *brain);
        let mut mechanics = Vec::new();
        for state in &family.states {
            if let Some(card) = &state.mechanics {
                let card = source.load(card)?;
                ensure!(rete::decode::<Mechanics>(&rete::encode(card.as_ref())?)? == *card);
                mechanics.push(serde_json::json!({"state":state.id,"card":card.as_ref()}));
            }
        }
        let mut routes = Vec::new();
        for (slot, binding) in perso.routes.iter().enumerate() {
            let Some(reference) = &binding.route else {
                continue;
            };
            let route = source.load(reference)?;
            ensure!(rete::decode::<Route>(&rete::encode(route.as_ref())?)? == *route);
            if actor == 45 {
                ensure!(binding.progress == [0, 0, 255, 255]);
                let ids = if slot == 0 {
                    vec![136, 135]
                } else {
                    vec![129, 134, 133, 132, 131, 130]
                };
                ensure!(route.points.len() == ids.len());
                for (point, id) in route.points.iter().zip(ids) {
                    ensure!(point.path.as_str() == format!("level:/waypoints/17-1/{id}"));
                }
                let first = route.links[0].as_ref().context("first travel link")?;
                ensure!(first.connection == 0 && first.dynamic_kind == 0);
                ensure!(first.static_endpoint.as_ref() == Some(&route.points[1]));
                if slot == 1 {
                    ensure!(
                        route.links[1..].iter().all(|link| link
                            .as_ref()
                            .is_some_and(
                                |link| link.connection == 3 && link.static_endpoint.is_none()
                            ))
                    );
                }
            }
            let mut points = Vec::new();
            for point in &route.points {
                let point = source.load(point)?;
                ensure!(rete::decode::<Waypoint>(&rete::encode(point.as_ref())?)? == *point);
                if actor == 45 {
                    ensure!(point.relative_actor.is_none() && point.radius == 1.0);
                }
                points.push(serde_json::to_value(point.as_ref())?);
            }
            routes.push(serde_json::json!({"slot":slot,"progress":binding.progress,"route":route.as_ref(),"points":points}));
        }
        if actor == 45 {
            ensure!(
                perso.routes.len() == 2,
                "traveling camera lost its two ways"
            );
            ensure!(perso.routes[0].route.is_some() && perso.routes[1].route.is_some());
            ensure!(perso.current_state == Some(1) && perso.initial_state == Some(1));
            let current = family
                .states
                .iter()
                .find(|state| state.id == 1)
                .context("travel state")?;
            ensure!(current.animation.is_none());
            let card = source.load(current.mechanics.as_ref().context("travel mechanics")?)?;
            ensure!(matches!(card.data, MechanicsData::Mass { mass: 1.0 }));
            ensure!(
                perso
                    .brain
                    .as_ref()
                    .context("traveling brain")?
                    .model
                    .path
                    .as_str()
                    == "level:/ai/models/365"
            );
        } else {
            for id in [1, 28] {
                let state = family
                    .states
                    .iter()
                    .find(|state| state.id == id)
                    .context("Gogoud walking state")?;
                let card = source.load(
                    state
                        .mechanics
                        .as_ref()
                        .context("Gogoud walking mechanics")?,
                )?;
                ensure!(matches!(
                    card.data,
                    MechanicsData::Character { mode: 0, .. }
                ));
            }
            let state = family
                .states
                .iter()
                .find(|state| state.id == 36)
                .context("Gogoud final speech state")?;
            let card = source.load(
                state
                    .mechanics
                    .as_ref()
                    .context("Gogoud speech mechanics")?,
            )?;
            ensure!(matches!(
                card.data,
                MechanicsData::Unresolved { kind: 19, .. }
            ));
        }
        actors.push(serde_json::json!({"perso":perso.as_ref(),"family":family.as_ref(),"brain":brain.as_ref(),"mechanics":mechanics,"routes":routes}));
    }
    let report = serde_json::json!({"scope":"source import and Rete prerequisites; motion algorithms not accepted", "actors":actors});
    if let Some(path) = std::env::var_os("ASTROLABE_TRAVEL_AUDIT_REPORT") {
        fs::write(path, serde_json::to_vec_pretty(&report)?)?;
    }
    println!("MANOR_TRAVEL_DEPENDENCIES_OK");
    Ok(())
}
