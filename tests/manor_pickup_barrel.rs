//! Finite Manor interaction evidence; source reads never launch the original game.
use anyhow::{Context, Result, ensure};
use astrolabe::{
    godot,
    model::{
        AssetPath, Selection,
        ai::{
            ActorKeyword, ActorTarget, AiModel, Behavior, BehaviorSet, Comparison, DesignerValue,
            DesignerVariable, Expression, Script, Statement,
        },
    },
    openspace::level::import_scene,
    rete,
};
use std::{fs, path::PathBuf, time::Duration};

#[test]
fn native_pickup_spatial_contacts_barrel_masks_and_grant_lifecycle() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let project = temp.path().join("project");
    godot::support::initialize(&project)?;
    fs::write(
        project.join("check_manor_pickup_barrel.gd"),
        include_bytes!("check_manor_pickup_barrel.gd"),
    )?;
    let model = grant_lifecycle_fixture()?;
    let reopened = rete::decode::<AiModel>(&rete::encode(&model)?)?;
    assert_eq!(model, reopened);
    fs::write(
        project.join("grant_lifecycle.gd"),
        godot::ai::compile(&reopened, &[])?,
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    for (name, parser) in [("parser", true), ("native", false)] {
        let log = project.join(format!("manor-{name}.log"));
        let mut args = vec![
            "--headless",
            "--log-file",
            log.to_str().context("log path")?,
        ];
        if parser {
            args.push("--check-only");
        }
        args.extend(["--script", "res://check_manor_pickup_barrel.gd"]);
        let output = godot::run_godot(&engine, &project, &args, Duration::from_secs(15))?;
        let diagnostics = fs::read_to_string(log)?;
        ensure!(!diagnostics.contains("ERROR:"), "{diagnostics}");
        if !parser {
            ensure!(
                output.contains("MANOR_PICKUP_BARREL_PRIMITIVES_OK"),
                "{output}"
            );
        }
    }
    Ok(())
}

/// Synthetic compiled grant/suspend/kill slice. The two-frame suspension tests
/// native coroutine ownership; it does not replace Manor's tutorial WaitBoolean.
fn grant_lifecycle_fixture() -> Result<AiModel> {
    Ok(AiModel {
        path: AssetPath::new("level:/ai/test-grant-lifecycle")?,
        name: "Grant lifecycle fixture".into(),
        normal: Some(BehaviorSet {
            behaviors: vec![Behavior {
                id: 0,
                rules: vec![],
                schedule: Some(Script {
                    statements: vec![
                        Statement::Assign {
                            variable: 0,
                            value: Expression::InventoryAddObject(Box::new(Expression::Actor(
                                ActorTarget::Keyword(ActorKeyword::SelfActor),
                            ))),
                        },
                        Statement::If {
                            condition: Expression::Compare {
                                operation: Comparison::Equal,
                                left: Box::new(Expression::Variable(0)),
                                right: Box::new(Expression::Integer(12)),
                            },
                            then: vec![
                                Statement::WaitFrames(Expression::Integer(2)),
                                Statement::KillSelf,
                            ],
                            otherwise: None,
                        },
                    ],
                }),
                unknown: [0; 3],
            }],
            default_behavior: Some(0),
            action_capacity: 1,
            unknown: [0; 3],
        }),
        reflex: None,
        variables: vec![DesignerVariable {
            default: DesignerValue::Integer(0),
            save: 0,
            initialize: 0,
        }],
        designer_unknown: [0; 3],
        unknown: [0; 4],
    })
}

#[test]
#[ignore = "requires local Manor source data"]
fn manor_barrel_templates_keep_actor_stock_and_distinct_source_ownership() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let native_project = tempfile::tempdir()?;
    godot::support::initialize(native_project.path())?;
    fs::create_dir_all(native_project.path().join("game"))?;
    for name in ["inventory_host.gd", "save_file.gd", "save_state.gd"] {
        fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("godot/game")
                .join(name),
            native_project.path().join("game").join(name),
        )?;
    }
    fs::write(
        native_project.path().join("generated-binding.gd"),
        "extends \"res://addons/astrolabe/behavior/behavior.gd\"\n@export var source_model_path := \"test:/drop\"\n@export var actor_brain_0_path: NodePath\nvar actor_brain_0: Node\nfunc source_external_actor_brains() -> PackedStringArray:\n    return PackedStringArray([\"fix:/persos/15\"])\n",
    )?;
    fs::write(
        native_project.path().join("fixture-sound.gd"),
        "extends Resource\nvar identity := \"synthetic pickup dispatch cue\"\nfunc execute(_emitter: Node3D): pass\n",
    )?;
    fs::write(
        native_project.path().join("fixture-sound.tres"),
        "[gd_resource type=\"Resource\" load_steps=2 format=3]\n[ext_resource type=\"Script\" path=\"res://fixture-sound.gd\" id=\"script\"]\n[resource]\nscript = ExtResource(\"script\")\n",
    )?;
    fs::write(
        native_project.path().join("check_manor_dispatch.gd"),
        include_bytes!("check_manor_dispatch.gd"),
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let parser_log = native_project.path().join("dispatch-parser.log");
    godot::run_godot(
        &engine,
        native_project.path(),
        &[
            "--headless",
            "--log-file",
            parser_log.to_str().context("parser log")?,
            "--check-only",
            "--script",
            "res://check_manor_dispatch.gd",
        ],
        Duration::from_secs(10),
    )?;
    let imported = import_scene(&game, &Selection::hype("manoir", "en")?)?;
    let tutorial = imported
        .ai_models
        .iter()
        .find(|model| model.path.as_str() == "fix:/ai/models/535")
        .context("pickup tutorial dependency missing")?;
    let behavior = tutorial
        .normal
        .as_ref()
        .context("tutorial Normal set missing")?
        .behaviors
        .iter()
        .find(|behavior| behavior.id == 6)
        .context("pickup tutorial Normal6 missing")?;
    for script in behavior
        .rules
        .iter()
        .flatten()
        .chain(behavior.schedule.iter())
    {
        ensure!(
            script.unsupported_fragments().is_empty(),
            "first-pickup tutorial unresolved: {:?}",
            script.unsupported_fragments()
        );
    }
    if let Some(path) = std::env::var_os("ASTROLABE_PICKUP_EVIDENCE_DIR") {
        let path = PathBuf::from(path);
        fs::create_dir_all(&path)?;
        fs::write(
            path.join("tutorial-source.json"),
            serde_json::to_vec_pretty(tutorial)?,
        )?;
    }
    let global = imported
        .persos
        .iter()
        .find(|actor| actor.path.as_str() == "fix:/persos/15")
        .context("Global dependency missing")?;
    let global_model = imported
        .ai_models
        .iter()
        .find(|model| model.path == global.brain.as_ref().unwrap().model.path)
        .context("Global AI dependency missing")?;
    println!(
        "MANOR_GLOBAL220_SAVE {:x}",
        global_model.variables[220].save
    );
    for index in 220..=228 {
        ensure!(
            global_model.variables[index].save == 0x200
                && matches!(
                    global_model.variables[index].default,
                    DesignerValue::Boolean(_)
                ),
            "first-pickup save ownership changed for Global[{index}]"
        );
        println!(
            "MANOR_FIRST_PICKUP_FIELD {index} save={:x} default={:?}",
            global_model.variables[index].save, global_model.variables[index].default
        );
    }
    let barrel = imported
        .ai_models
        .iter()
        .find(|model| model.path.as_str() == "level:/ai/models/488")
        .context("Manor barrel AI missing")?;
    assert_eq!(barrel.object_templates().len(), 9);
    assert_eq!(barrel.variables[8].save, 0x0a00);
    let catalog = imported
        .inventory_catalog
        .as_ref()
        .context("inventory catalogue missing")?;
    let mut actors = Vec::new();
    for index in 71..=74 {
        let path = format!("level:/persos/world/0/{index}");
        let actor = imported
            .persos
            .iter()
            .find(|actor| actor.path.as_str() == path)
            .context("placed barrel missing")?;
        assert_eq!(actor.brain.as_ref().unwrap().model.path, barrel.path);
        assert_eq!(
            actor.brain.as_ref().unwrap().variables[4],
            DesignerValue::UnsignedByte(0)
        );
        let mut edited = actor.clone();
        edited.brain.as_mut().unwrap().variables[8] = DesignerValue::Boolean(true);
        let reopened = rete::decode::<astrolabe::model::character::Perso>(&rete::encode(&edited)?)?;
        assert_eq!(reopened, edited);
        assert_eq!(
            actor.brain.as_ref().unwrap().variables[8],
            DesignerValue::Boolean(false)
        );
        actors.push(actor);
    }
    let expected = [
        (12, 1),
        (13, 1),
        (14, 1),
        (10, 1),
        (11, 1),
        (80, 10),
        (81, 5),
        (0, 50),
    ];
    for (index, reference) in barrel.object_templates().into_iter().enumerate() {
        let actor = imported
            .persos
            .iter()
            .find(|actor| actor.path == reference.path)
            .context("drop template missing")?;
        if let Some(stock) = actor.inventory_stock.as_ref() {
            let item = catalog
                .items
                .iter()
                .chain(std::iter::once(&catalog.gold))
                .find(|item| Some(&item.source) == stock.item.as_ref())
                .context("drop stock item lacks catalogue join")?;
            assert!(stock.category.is_some());
            assert_eq!((item.id, stock.quantity), expected[index]);
            println!(
                "MANOR_DROP {} {} item={} count={} model={}",
                actor.path,
                actor.name,
                item.id,
                stock.quantity,
                actor
                    .brain
                    .as_ref()
                    .map(|b| b.model.path.as_str())
                    .unwrap_or("none")
            );
        } else {
            assert_eq!(
                index, 8,
                "only the health/magic food template has no inventory stock"
            );
            assert_eq!(actor.path.as_str(), "fix:/persos/5-0-27330");
            assert_eq!(
                actor.brain.as_ref().unwrap().model.path.as_str(),
                "fix:/ai/models/1196"
            );
        }
        let reopened = rete::decode::<astrolabe::model::character::Perso>(&rete::encode(actor)?)?;
        assert_eq!(reopened, *actor);
        let drop_model = imported
            .ai_models
            .iter()
            .find(|model| model.path == actor.brain.as_ref().unwrap().model.path)
            .context("drop AI missing")?;
        assert_eq!(drop_model.variables.len(), 1);
        assert_eq!(
            drop_model.variables[0].save, 0,
            "drop fade/bounce field is transient"
        );
        assert_eq!(
            rete::decode::<AiModel>(&rete::encode(drop_model)?)?,
            *drop_model
        );
        actors.push(actor);
    }
    let models: Vec<_> = imported
        .ai_models
        .iter()
        .filter(|model| {
            model.path == barrel.path
                || model.path.as_str() == "fix:/ai/models/671"
                || actors.iter().any(|actor| {
                    actor
                        .brain
                        .as_ref()
                        .is_some_and(|brain| brain.model.path == model.path)
                })
        })
        .collect();
    for model in &models {
        let family = actors
            .iter()
            .copied()
            .chain(imported.persos.iter())
            .find(|actor| {
                actor
                    .brain
                    .as_ref()
                    .is_some_and(|brain| brain.model.path == model.path)
            })
            .and_then(|actor| {
                imported
                    .families
                    .iter()
                    .find(|family| family.path == actor.family.path)
            });
        for set in [&model.normal, &model.reflex].into_iter().flatten() {
            for behavior in &set.behaviors {
                for script in behavior
                    .rules
                    .iter()
                    .flatten()
                    .chain(behavior.schedule.iter())
                {
                    ensure!(
                        script.unsupported_fragments().is_empty(),
                        "{} behavior{} unresolved: {:?}",
                        model.path,
                        behavior.id,
                        script.unsupported_fragments()
                    );
                }
            }
        }
        let events = model
            .sound_events()
            .into_iter()
            .map(|event| (event.clone(), "res://fixture-sound.tres".to_string()))
            .collect::<Vec<_>>();
        let generated = godot::ai::compile_for_family(model, &events, family)?;
        println!("MANOR_COMPILED {} {} bytes", model.path, generated.len());
        let name = format!("{}.gd", model.path.as_str().replace([':', '/'], "_"));
        fs::write(native_project.path().join(&name), &generated)?;
        let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
        let log = native_project.path().join("source-parser.log");
        godot::run_godot(
            &engine,
            native_project.path(),
            &[
                "--headless",
                "--log-file",
                log.to_str().context("parser log")?,
                "--check-only",
                "--script",
                &format!("res://{name}"),
            ],
            Duration::from_secs(10),
        )?;
        if let Some(path) = std::env::var_os("ASTROLABE_PICKUP_EVIDENCE_DIR") {
            let path = PathBuf::from(path);
            fs::create_dir_all(&path)?;
            fs::write(
                path.join(format!(
                    "{}.gd",
                    model.path.as_str().replace([':', '/'], "_")
                )),
                generated,
            )?;
        }
    }
    fs::write(
        native_project.path().join("check_manor_dispatch.gd"),
        include_bytes!("check_manor_dispatch.gd"),
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let log = native_project.path().join("actual-dispatch.log");
    let output = godot::run_godot(
        &engine,
        native_project.path(),
        &[
            "--headless",
            "--log-file",
            log.to_str().context("dispatch log")?,
            "--script",
            "res://check_manor_dispatch.gd",
        ],
        Duration::from_secs(15),
    )?;
    ensure!(output.contains("MANOR_ACTUAL_DISPATCH_OK"), "{output}");
    if let Some(path) = std::env::var_os("ASTROLABE_PICKUP_EVIDENCE_DIR") {
        let path = PathBuf::from(path);
        fs::create_dir_all(&path)?;
        fs::copy(&log, path.join("actual-dispatch.log"))?;
        fs::write(
            path.join("manor-source.json"),
            serde_json::to_vec_pretty(
                &serde_json::json!({"actors":actors,"models":models,"catalog":catalog}),
            )?,
        )?;
    }
    Ok(())
}
