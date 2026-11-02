//! Source actor references and current Normal/Reflex behavior equality queries.
use anyhow::{Context, Result, ensure};
use astrolabe::{
    godot,
    model::{
        AssetPath, AssetRef, Selection,
        ai::{
            ActorKeyword, ActorTarget, AiModel, Behavior, BehaviorGroup, BehaviorRef, BehaviorSet,
            DesignerValue, DesignerVariable, Expression, PreservedAiNode, PreservedAiOperand,
            Script, SourceObjectIdentity, SourceRole, Statement,
        },
        character::Perso,
        io::{Content, Load},
        scene::Scene,
    },
    openspace::{ai::AiScriptCodec, codec::FormatCodec},
    rete,
};
use std::{fs, path::PathBuf, sync::Arc, time::Duration};

fn node(bytes: &mut Vec<u8>, category: u8, parameter: u32, indent: u8) {
    bytes.extend_from_slice(&parameter.to_le_bytes());
    bytes.extend_from_slice(&[0, 0, indent, category]);
}

fn query_source(condition: u32, actor_category: u8, actor_parameter: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    node(&mut bytes, 0, 0, 1); // If
    node(&mut bytes, 1, condition, 2); // behavior query
    node(&mut bytes, actor_category, actor_parameter, 3);
    node(&mut bytes, 32, 0x5678, 3); // bound BehaviorRef
    node(&mut bytes, 0, 1, 1); // Then
    node(&mut bytes, 4, 206, 2); // Noop
    node(&mut bytes, 8, 0, 0); // Sentinel
    bytes
}

fn external_behavior() -> Result<BehaviorRef> {
    Ok(BehaviorRef {
        model: AssetRef::new("fix:/ai/models/28")?,
        group: BehaviorGroup::Normal,
        behavior: 7,
    })
}

fn query_model(rules: Vec<Script>) -> Result<AiModel> {
    Ok(AiModel {
        path: AssetPath::new("level:/ai/models/behavior-query-fixture")?,
        name: "Behavior Query Fixture".into(),
        normal: Some(BehaviorSet {
            behaviors: vec![Behavior {
                id: 0,
                rules: rules.into_iter().map(Some).collect(),
                schedule: None,
                unknown: [0; 3],
            }],
            default_behavior: Some(0),
            action_capacity: 1,
            unknown: [0; 3],
        }),
        reflex: None,
        variables: Vec::new(),
        designer_unknown: [0; 3],
        unknown: [0; 4],
    })
}

fn source_actor() -> Result<AssetRef<Perso>> {
    AssetRef::new("level:/persos/world/0/source-query-actor")
}

#[test]
fn source_actor_behavior_query_roundtrips_typed_reference_and_keywords() -> Result<()> {
    let actor = source_actor()?;
    let target = external_behavior()?;
    for (category, parameter, expected) in [
        (24, 0x1234, ActorTarget::Reference(actor.clone())),
        (0, 4, ActorTarget::Keyword(ActorKeyword::SelfActor)),
        (0, 5, ActorTarget::Keyword(ActorKeyword::MainActor)),
    ] {
        let mut codec = AiScriptCodec {
            behaviors: [(3, target.clone())].into(),
            behavior_handles: vec![(target.clone(), 0x5678)],
            ..Default::default()
        };
        if category == 24 {
            codec.actors.insert(2, actor.clone());
            codec.actor_handles.push((actor.clone(), 0x1234));
        }
        let bytes = query_source(56, category, parameter);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            &script.statements[0],
            Statement::If {
                condition: Expression::IsInBehavior { actor, target: found },
                ..
            } if actor == &expected && found == &target
        ));
        assert_eq!(codec.write(&script)?, bytes);
        let model = query_model(vec![script])?;
        model.validate()?;
        let reopened = rete::decode::<AiModel>(&rete::encode(&model)?)?;
        assert_eq!(reopened, model);
        let generated = godot::ai::compile(&reopened, &[])?;
        if category == 24 {
            assert!(
                generated
                    .contains("source_actor_node(\"level:/persos/world/0/source-query-actor\")")
            );
            assert!(
                generated.contains("source_actor_is_in_behavior(behavior_actor_0, &\"normal\", 7)")
            );
        }
    }
    Ok(())
}

#[test]
fn raw57_actor_reference_remains_explicitly_unsupported() -> Result<()> {
    let actor = source_actor()?;
    let target = BehaviorRef {
        group: BehaviorGroup::Reflex,
        behavior: 7,
        ..external_behavior()?
    };
    let codec = AiScriptCodec {
        preserved_nodes: vec![
            PreservedAiNode {
                category: 0,
                indent: 1,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated([0; 4]),
            },
            PreservedAiNode {
                category: 1,
                indent: 2,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated([57, 0, 0, 0]),
            },
            PreservedAiNode {
                category: 24,
                indent: 3,
                reserved: [0; 2],
                operand: PreservedAiOperand::Source(Some(SourceObjectIdentity {
                    role: SourceRole::Level,
                    block: [5, 1],
                    allocation: 33,
                })),
            },
            PreservedAiNode {
                category: 32,
                indent: 3,
                reserved: [0; 2],
                operand: PreservedAiOperand::Behavior(target.clone()),
            },
            PreservedAiNode {
                category: 0,
                indent: 1,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated(1u32.to_le_bytes()),
            },
            PreservedAiNode {
                category: 4,
                indent: 2,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated(206u32.to_le_bytes()),
            },
            PreservedAiNode {
                category: 8,
                indent: 0,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated([0; 4]),
            },
        ],
        actors: [(2, actor.clone())].into(),
        behaviors: [(3, target.clone())].into(),
        actor_handles: vec![(actor, 0x1234)],
        behavior_handles: vec![(target, 0x5678)],
        ..Default::default()
    };
    let script = codec.read(&query_source(57, 24, 0x1234))?;
    assert!(matches!(
        &script.statements[0],
        Statement::If {
            condition: Expression::Unsupported(fragment),
            ..
        } if fragment.reason.contains("reflex behavior query actor reference is unsupported")
    ));
    assert!(codec.write(&script).is_err());
    Ok(())
}

fn runtime_query_model() -> Result<AiModel> {
    let self_target = BehaviorRef {
        model: AssetRef::new("level:/ai/models/actor-query-runtime")?,
        group: BehaviorGroup::Normal,
        behavior: 0,
    };
    let main_target = BehaviorRef {
        model: AssetRef::new("fix:/ai/models/main-target")?,
        group: BehaviorGroup::Normal,
        behavior: 4,
    };
    let cross_model_target = BehaviorRef {
        model: AssetRef::new("fix:/ai/models/other-target")?,
        group: BehaviorGroup::Normal,
        behavior: 7,
    };
    let null_target = BehaviorRef {
        model: AssetRef::new("fix:/ai/models/null-target")?,
        group: BehaviorGroup::Normal,
        behavior: 0,
    };
    let targets = [self_target, main_target, cross_model_target, null_target];
    let actors = [
        ActorTarget::Keyword(ActorKeyword::SelfActor),
        ActorTarget::Keyword(ActorKeyword::MainActor),
        ActorTarget::Reference(AssetRef::new("level:/persos/world/cross")?),
        ActorTarget::Reference(AssetRef::new("level:/persos/world/null")?),
    ];
    let rules = targets
        .into_iter()
        .zip(actors)
        .enumerate()
        .map(|(index, (target, actor))| Script {
            statements: vec![Statement::If {
                condition: Expression::IsInBehavior { actor, target },
                then: vec![Statement::Assign {
                    variable: index as u16,
                    value: Expression::Integer(1),
                }],
                otherwise: None,
            }],
        })
        .collect();
    let mut model = query_model(rules)?;
    model.path = AssetPath::new("level:/ai/models/actor-query-runtime")?;
    model.name = "Actor Query Runtime".into();
    model.variables = (0..4)
        .map(|_| DesignerVariable {
            default: DesignerValue::Integer(0),
            save: 0,
            initialize: 0,
        })
        .collect();
    Ok(model)
}

#[test]
fn generated_behavior_query_reads_self_main_cross_model_and_null_slots() -> Result<()> {
    let model = runtime_query_model()?;
    model.validate()?;
    let generated = godot::ai::compile(&model, &[])?;
    let output = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("output");
    fs::create_dir_all(&output)?;
    let project = tempfile::tempdir_in(output)?;
    godot::support::initialize(project.path())?;
    fs::write(project.path().join("compiled_actor_query.gd"), generated)?;
    fs::write(
        project.path().join("test_actor_query.gd"),
        include_bytes!("../godot/tests/ai_behavior_query.gd"),
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let log = project.path().join("actor-query.log");
    let log_argument = log.to_string_lossy().into_owned();
    let evidence = godot::run_godot(
        &engine,
        project.path(),
        &[
            "--headless",
            "--script",
            "res://test_actor_query.gd",
            "--log-file",
            &log_argument,
        ],
        Duration::from_secs(20),
    )?;
    let log = fs::read_to_string(log)?;
    assert!(
        !evidence.contains("SCRIPT ERROR") && !log.contains("SCRIPT ERROR"),
        "Godot reported a script error despite the success marker:\nstdout: {evidence}\nlog: {log}"
    );
    assert!(
        evidence.contains("AI_BEHAVIOR_QUERY_OK") || log.contains("AI_BEHAVIOR_QUERY_OK"),
        "stdout: {evidence}\nlog: {log}"
    );
    Ok(())
}

fn game() -> PathBuf {
    std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../disc"))
}

#[test]
#[ignore = "requires local Hype game data; verifies the actual relocated category-24 actor pointer"]
fn manor_model_334_category24_resolves_through_level_actor_identity_map_and_rete() -> Result<()> {
    let selection = Selection::hype("manoir", "en")?;
    let mut imported = astrolabe::openspace::content::import_scene(&game(), selection.clone())?;
    let model_ref = AssetRef::<AiModel>::new("level:/ai/models/334")?;
    let model: Arc<AiModel> = imported.load(&model_ref)?;
    let schedule = model
        .normal
        .as_ref()
        .and_then(|set| set.behaviors.iter().find(|behavior| behavior.id == 0))
        .and_then(|behavior| behavior.schedule.as_ref())
        .context("model334 Normal0 schedule missing")?;
    let Statement::If {
        condition: Expression::And(_, query),
        then,
        otherwise,
    } = &schedule.statements[0]
    else {
        anyhow::bail!("model334 source And condition changed")
    };
    ensure!(otherwise.is_none(), "model334 source branch gained an Else");
    ensure!(
        then.len() == 14,
        "model334 true-branch statement count changed"
    );
    ensure!(
        matches!(
            (&then[0], &then[1]),
            (
                Statement::MainActorFieldAssign {
                    variable: 56,
                    value: Expression::Integer(1)
                },
                Statement::MainActorFieldAssign {
                    variable: 51,
                    value: Expression::Integer(0)
                }
            )
        ),
        "model334 branch's first MainActor field writes changed"
    );
    let Expression::IsInBehavior {
        actor: ActorTarget::Reference(actor_ref),
        target,
    } = query.as_ref()
    else {
        anyhow::bail!("category24 actor was not resolved into a semantic Perso reference")
    };
    // Op56 is source ordinal 6 after the confirmed ordinal-3 op8 at
    // Level:11:01+0xd400. Its category24 operand resolves through the SNA
    // relocation and rooted scene actor map, not by matching an AI model.
    ensure!(actor_ref.path.as_str() == "level:/persos/world/0/3");
    ensure!(target.model.path.as_str() == "level:/ai/models/154");
    ensure!(target.group == BehaviorGroup::Normal && target.behavior == 0);
    let actor: Arc<Perso> = imported.load(actor_ref)?;
    let actor_brain = actor
        .brain
        .as_ref()
        .context("resolved actor has no Brain")?;
    ensure!(actor_brain.model.path.as_str() == "level:/ai/models/154");
    let scene_ref = AssetRef::<Scene>::new("level:/scene")?;
    let scene: Arc<Scene> = imported.load(&scene_ref)?;
    let main_actor_ref = scene
        .main_actor
        .as_ref()
        .context("Manoir scene has no source MainActor")?;
    let main_actor: Arc<Perso> = imported.load(main_actor_ref)?;
    ensure!(
        main_actor
            .brain
            .as_ref()
            .is_some_and(|brain| brain.model.path.as_str() == "fix:/ai/models/28"),
        "Manoir MainActor Brain is not the referenced Fix model 28"
    );
    ensure!(
        scene
            .nodes
            .iter()
            .filter_map(|node| node.perso.as_ref())
            .any(|perso| perso == actor_ref),
        "resolved actor reference is not the rooted scene Perso selected by the source relocation"
    );
    println!("category24 source actor resolved to {}", actor_ref.path);

    // Keep the camera operation in its owner's scope. The installed PE row for
    // category5/raw49 is Action_Camera_GoToWP (signature PW); raw207 is the
    // source-traced cross-actor Normal behavior switch.
    let Statement::CameraGoToWaypoint { camera, waypoint } = &then[2] else {
        anyhow::bail!("model334 camera waypoint action is no longer a typed GoToWP")
    };
    ensure!(
        matches!(camera, Expression::Actor(ActorTarget::Reference(_))),
        "Action_Camera_GoToWP Perso operand lost its actor identity: {camera:?}"
    );
    ensure!(
        matches!(
            waypoint,
            Expression::WaypointReference(reference)
                if reference.path.as_str() == "level:/waypoints/17-1/55"
        ),
        "Action_Camera_GoToWP Waypoint operand lost its source identity: {waypoint:?}"
    );
    ensure!(
        matches!(
            &then[3],
            Statement::CameraChangeState { camera, state: Expression::Integer(3) }
                if camera.path.as_str() == "level:/persos/world/0/2"
        ),
        "model334 camera state target changed"
    );
    ensure!(
        matches!(
            (&then[4], &then[5]),
            (
                Statement::MainActorFieldAssign {
                    variable: 62,
                    value: Expression::Integer(0)
                },
                Statement::MainActorFieldAssign {
                    variable: 123,
                    value: Expression::Integer(3)
                }
            )
        ),
        "model334 post-camera MainActor field writes changed"
    );
    for (index, allocation) in (56..=58).enumerate() {
        let Statement::ArrayAssign {
            array: Expression::MainActorFieldRead(157),
            index: Expression::Integer(actual_index),
            value: Expression::WaypointReference(waypoint),
        } = &then[6 + index]
        else {
            anyhow::bail!("model334 MainActor Waypoint array assignment {index} changed")
        };
        ensure!(*actual_index == index as i32);
        ensure!(
            waypoint.path.as_str() == format!("level:/waypoints/17-1/{allocation}"),
            "model334 DSG157[{index}] Waypoint lost its source identity: {}",
            waypoint.path
        );
    }
    for index in 0..3 {
        ensure!(
            matches!(
                &then[9 + index],
                Statement::ArrayAssign {
                    array: Expression::MainActorFieldRead(166),
                    index: Expression::Integer(actual_index),
                    value: Expression::Integer(0),
                } if *actual_index == index as i32
            ),
            "model334 DSG166[{index}] clear changed"
        );
    }
    let Statement::ChangeActorBehavior {
        actor: ActorTarget::Keyword(ActorKeyword::MainActor),
        target: requested_behavior,
    } = &then[12]
    else {
        anyhow::bail!("Proc_ChangeComport did not resolve MainActor and its behavior")
    };
    ensure!(
        requested_behavior.model.path.as_str() == "fix:/ai/models/28"
            && requested_behavior.group == BehaviorGroup::Normal
            && requested_behavior.behavior == 50
    );
    ensure!(
        matches!(
            &then[13],
            Statement::ChangeBehavior {
                target,
                pause_after: false
            } if target.model.path.as_str() == "level:/ai/models/334"
                && target.group == BehaviorGroup::Normal
                && target.behavior == 1
        ),
        "model334 self behavior switch changed"
    );

    let output = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("output");
    fs::create_dir_all(&output)?;
    let rete_dir = tempfile::tempdir_in(output)?;
    astrolabe::app::save_rete(&mut imported, rete_dir.path())?;
    let (mut store, _) = rete::Store::open(rete_dir.path())?;
    let reopened: Arc<AiModel> = store.load(&model_ref)?;
    ensure!(*reopened == *model, "resolved actor query changed in Rete");
    Ok(())
}
