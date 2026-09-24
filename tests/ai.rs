//! Typed AI round trips and execution of generated code against maintained support.
use anyhow::Result;
use astrolabe::{
    godot,
    model::{
        AssetPath, AssetRef, Selection,
        ai::*,
        content::Collection,
        io::{Content, ContentRoot},
    },
    openspace::{ai::AiScriptCodec, codec::FormatCodec},
    rete,
};
use std::{fs, time::Duration};

#[test]
fn native_magick_service_follows_verified_remove_points_contract() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let project = temp.path().join("godot");
    godot::support::initialize(&project)?;
    fs::write(
        project.join("check_magick.gd"),
        include_bytes!("check_magick.gd"),
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let evidence = godot::run_godot(
        &engine,
        &project,
        &["--headless", "--script", "res://check_magick.gd"],
        Duration::from_secs(20),
    )?;
    assert!(evidence.contains("MAGICK_SERVICE_OK"), "{evidence}");
    Ok(())
}

#[test]
fn unknown_contact_stops_compiled_ai_before_negated_branch() -> Result<()> {
    let mut model = model()?;
    model.normal.as_mut().unwrap().behaviors[0]
        .schedule
        .as_mut()
        .unwrap()
        .statements = vec![Statement::If {
        condition: Expression::Not(Box::new(Expression::CollideWithNothing)),
        then: vec![Statement::Assign {
            variable: 0,
            value: Expression::Integer(1),
        }],
        otherwise: None,
    }];
    let code = godot::ai::compile(&model, &[])?;
    let query = code.find("source_collide_with_nothing()").unwrap();
    let guard = code
        .find("requires a verified source contact result")
        .unwrap();
    let branch = code.find("if bool((not bool(contact_result_").unwrap();
    assert!(query < guard && guard < branch, "{code}");
    Ok(())
}

#[test]
fn camera_target_call_keeps_source_actor_and_main_actor() -> Result<()> {
    let mut model = model()?;
    model.normal.as_mut().unwrap().behaviors[0]
        .schedule
        .as_mut()
        .unwrap()
        .statements = vec![Statement::CameraChangeTargetedPerso {
        camera: AssetRef::new("level:/persos/world/0/2")?,
        target: ActorKeyword::MainActor,
    }];
    let code = godot::ai::compile(&model, &[])?;
    assert!(
        code.contains(
            "source_camera_change_targeted_perso(\"level:/persos/world/0/2\", main_actor)"
        )
    );
    assert!(code.contains("requires camera target state"));
    Ok(())
}

fn model() -> Result<AiModel> {
    Ok(AiModel {
        path: AssetPath::new("level:/ai/proximity")?,
        name: "Proximity".into(),
        normal: Some(BehaviorSet {
            behaviors: vec![Behavior {
                id: 0,
                rules: vec![],
                schedule: Some(Script {
                    statements: vec![
                        Statement::Wait {
                            milliseconds: Expression::Integer(100),
                        },
                        Statement::If {
                            condition: Expression::Compare {
                                operation: Comparison::Less,
                                left: Box::new(Expression::MainActorDistance),
                                right: Box::new(Expression::Real(3.0)),
                            },
                            then: vec![
                                Statement::Assign {
                                    variable: 0,
                                    value: Expression::Integer(1),
                                },
                                Statement::Assign {
                                    variable: 1,
                                    value: Expression::Integer(7),
                                },
                            ],
                            otherwise: Some(vec![
                                Statement::Assign {
                                    variable: 0,
                                    value: Expression::Integer(0),
                                },
                                Statement::Assign {
                                    variable: 1,
                                    value: Expression::Integer(9),
                                },
                            ]),
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
        variables: vec![
            DesignerVariable {
                default: DesignerValue::Boolean(false),
                save: 0,
                initialize: 0,
            },
            DesignerVariable {
                default: DesignerValue::Integer(0),
                save: 0,
                initialize: 0,
            },
        ],
        designer_unknown: [0; 3],
        unknown: [0; 4],
    })
}
#[test]
fn typed_ai_codec_preserves_branches_values_and_rejects_unknown_or_malformed_nodes() -> Result<()> {
    let model = model()?;
    let script = model.normal.as_ref().unwrap().behaviors[0]
        .schedule
        .as_ref()
        .unwrap();
    let codec = AiScriptCodec::default();
    let bytes = codec.write(script)?;
    assert_eq!(codec.read(&bytes)?, *script);
    let reopened = rete::decode::<AiModel>(&rete::encode(&model)?)?;
    assert_eq!(reopened, model);
    assert_eq!(
        codec.write(
            reopened.normal.as_ref().unwrap().behaviors[0]
                .schedule
                .as_ref()
                .unwrap()
        )?,
        bytes
    );
    let mut bad = bytes.clone();
    bad[4] = 1;
    assert!(codec.read(&bad).is_err());
    let mut bad = bytes.clone();
    bad[7] = 36;
    assert!(codec.read(&bad).is_err());
    let mut bad = bytes.clone();
    bad[14] = 3;
    assert!(codec.read(&bad).is_err());
    assert!(codec.read(&bytes[..bytes.len() - 8]).is_err());
    let mut bad = model.clone();
    bad.variables.clear();
    assert!(bad.validate().is_err());
    let mut bad = model.clone();
    bad.normal.as_mut().unwrap().default_behavior = Some(42);
    assert!(bad.validate().is_err());
    let mut rules = model.clone();
    rules.normal.as_mut().unwrap().behaviors[0]
        .rules
        .push(Some(script.clone()));
    assert!(godot::ai::compile(&rules, &[]).is_ok());
    // An out-of-range source index must fail rather than truncate to a u8.
    let mut bad = bytes.clone();
    let variable = bad
        .as_chunks_mut::<8>()
        .0
        .iter_mut()
        .find(|n| n[7] == 11)
        .unwrap();
    variable[..4].copy_from_slice(&256u32.to_le_bytes());
    assert!(codec.read(&bad).is_err());
    assert!(
        codec
            .write(&Script {
                statements: vec![Statement::Assign {
                    variable: 256,
                    value: Expression::Integer(1),
                }],
            })
            .is_err()
    );
    Ok(())
}

#[test]
fn unsupported_source_script_survives_rete_without_becoming_executable() -> Result<()> {
    let mut model = model()?;
    let behavior = &mut model.normal.as_mut().unwrap().behaviors[0];
    let supported = behavior.schedule.take().unwrap();
    behavior.rules.push(Some(supported.clone()));
    behavior.schedule = Some(Script {
        statements: vec![Statement::Unsupported(UnsupportedFragment {
            reason: "category 1 operation 99".into(),
            start: 3,
            nodes: vec![PreservedAiNode {
                category: 1,
                indent: 1,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated(99u32.to_le_bytes()),
            }],
        })],
    });
    let reopened = rete::decode::<AiModel>(&rete::encode(&model)?)?;
    assert_eq!(reopened, model);
    let schedule = reopened.normal.as_ref().unwrap().behaviors[0]
        .schedule
        .as_ref()
        .unwrap();
    assert!(AiScriptCodec::default().write(schedule).is_err());
    assert_eq!(
        AiScriptCodec::default().read(&AiScriptCodec::default().write(&supported)?)?,
        supported
    );
    let code = godot::ai::compile(&reopened, &[])?;
    assert!(code.contains("normal_0_rule_0"));
    assert!(code.contains("fail_unsupported"));
    assert!(code.contains("source AI node 3: category 1 operation 99"));
    Ok(())
}

#[test]
fn hype_sound_fade_call_remains_typed_through_source_rete_and_godot() -> Result<()> {
    let mut model = model()?;
    let script = Script {
        statements: vec![Statement::SoundFadeMusic {
            arg0: Expression::Integer(127),
            arg1: Expression::Integer(1),
        }],
    };
    let bytes = AiScriptCodec::default().write(&script)?;
    assert_eq!(AiScriptCodec::default().read(&bytes)?, script);
    model.normal.as_mut().unwrap().behaviors[0].schedule = Some(script);
    let reopened = rete::decode::<AiModel>(&rete::encode(&model)?)?;
    assert_eq!(reopened, model);
    let compiled = godot::ai::compile(&reopened, &[])?;
    assert!(compiled.contains("sound_fade_music(int(127), int(1))"));
    Ok(())
}

#[test]
fn nowhere_clears_source_waypoint_without_enabling_unknown_writes() -> Result<()> {
    let mut model = model()?;
    model.variables.push(DesignerVariable {
        default: DesignerValue::SourceWaypoint(None),
        save: 0,
        initialize: 1,
    });
    model.normal.as_mut().unwrap().behaviors[0].schedule = Some(Script {
        statements: vec![
            Statement::Assign {
                variable: 2,
                value: Expression::Nowhere,
            },
            Statement::Assign {
                variable: 2,
                value: Expression::Integer(1),
            },
        ],
    });
    let compiled = godot::ai::compile(&model, &[])?;
    assert!(compiled.contains("dsg_2 = null"));
    assert!(compiled.contains("writes unresolved source designer field 2"));
    Ok(())
}

#[test]
fn partial_compiled_script_runs_supported_branch_and_stops_at_reached_fragment() -> Result<()> {
    let mut model = model()?;
    model.variables.push(DesignerVariable {
        default: DesignerValue::Actor(None),
        save: 0,
        initialize: 0,
    });
    let fragment = UnsupportedFragment {
        reason: "procedure requires native API".into(),
        start: 7,
        nodes: vec![PreservedAiNode {
            category: 4,
            indent: 2,
            reserved: [0; 2],
            operand: PreservedAiOperand::NonRelocated(445u32.to_le_bytes()),
        }],
    };
    let schedule = &mut model.normal.as_mut().unwrap().behaviors[0].schedule;
    *schedule = Some(Script {
        statements: vec![
            Statement::Assign {
                variable: 1,
                value: Expression::Integer(1),
            },
            Statement::If {
                condition: Expression::Variable(0),
                then: vec![Statement::Unsupported(fragment.clone())],
                otherwise: Some(vec![Statement::Assign {
                    variable: 1,
                    value: Expression::Integer(2),
                }]),
            },
            Statement::Assign {
                variable: 1,
                value: Expression::Integer(3),
            },
        ],
    });
    let temp = tempfile::tempdir()?;
    let selection = Selection::hype("synthetic", "en")?;
    let reference = AssetRef::from_path(model.path.clone());
    let roots = vec![ContentRoot {
        kind: AiModel::KIND.into(),
        reference: model.path.clone(),
    }];
    let mut source = Collection::default();
    source.insert(&reference, model.clone())?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let project = temp.path().join("godot");
    godot::export(&mut source, &roots, selection, &project, &engine)?;
    fs::write(
        project.join("check_partial_ai.gd"),
        include_bytes!("check_partial_ai.gd"),
    )?;
    let log = godot::run_godot(
        &engine,
        &project,
        &["--headless", "--script", "res://check_partial_ai.gd"],
        Duration::from_secs(30),
    )
    .expect_err("reached Godot stub must report an error")
    .to_string();
    assert!(log.contains("PARTIAL_AI_REACHED_FAILURE_OK"), "{log}");
    assert!(log.contains("source AI node 7"), "{log}");

    // The same script structure also guards an unsupported expression.
    model.normal.as_mut().unwrap().behaviors[0]
        .schedule
        .as_mut()
        .unwrap()
        .statements[1] = Statement::If {
        condition: Expression::And(
            Box::new(Expression::Variable(0)),
            Box::new(Expression::Compare {
                operation: Comparison::Equal,
                left: Box::new(Expression::Variable(2)),
                right: Box::new(Expression::Unsupported(fragment)),
            }),
        ),
        then: vec![Statement::Assign {
            variable: 1,
            value: Expression::Integer(2),
        }],
        otherwise: None,
    };
    fs::write(
        project.join("generated/hype/levels/synthetic/ai/proximity.gd"),
        godot::ai::compile(&model, &[])?,
    )?;
    let log = godot::run_godot(
        &engine,
        &project,
        &["--headless", "--script", "res://check_partial_ai.gd"],
        Duration::from_secs(30),
    )
    .expect_err("reached expression stub must report an error")
    .to_string();
    assert!(log.contains("PARTIAL_AI_REACHED_FAILURE_OK"), "{log}");
    assert!(log.contains("source AI node 7"), "{log}");
    Ok(())
}

#[test]
fn generated_gdscript_runs_native_waits_branches_and_persistent_instance_variables() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let selection = Selection::hype("synthetic", "en")?;
    let model = model()?;
    let reference = AssetRef::from_path(model.path.clone());
    let roots = vec![ContentRoot {
        kind: AiModel::KIND.into(),
        reference: model.path.clone(),
    }];
    let mut source = Collection::default();
    source.insert(&reference, model.clone())?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let project = temp.path().join("godot");
    let direct = godot::export(&mut source, &roots, selection.clone(), &project, &engine)?;
    assert!(
        direct
            .files
            .contains_key("generated/hype/levels/synthetic/ai/proximity.gd")
    );
    let mut reopened = rete::Store::new(temp.path().join("rete"), selection.clone())?;
    use astrolabe::model::io::Save;
    reopened.save(&reference, &model)?;
    let reloaded = godot::export(
        &mut reopened,
        &roots,
        selection,
        &temp.path().join("reloaded"),
        &engine,
    )?;
    assert_eq!(direct.files, reloaded.files);
    fs::write(project.join("check_ai.gd"), include_bytes!("check_ai.gd"))?;
    let evidence = godot::run_godot(
        &engine,
        &project,
        &["--headless", "--script", "res://check_ai.gd"],
        Duration::from_secs(30),
    )?;
    assert!(evidence.contains("COMPILED_AI_OK"));
    println!("{evidence}");
    let mut edited = model.clone();
    let statements = &mut edited.normal.as_mut().unwrap().behaviors[0]
        .schedule
        .as_mut()
        .unwrap()
        .statements;
    let Statement::If {
        condition: Expression::Compare { right, .. },
        ..
    } = &mut statements[1]
    else {
        panic!("fixture branch")
    };
    **right = Expression::Real(1.0);
    let code = godot::ai::compile(&edited, &[])?;
    assert_ne!(code, godot::ai::compile(&model, &[])?);
    fs::write(
        project.join("generated/hype/levels/synthetic/ai/proximity.gd"),
        code,
    )?;
    let evidence = godot::run_godot(
        &engine,
        &project,
        &[
            "--headless",
            "--script",
            "res://check_ai.gd",
            "--",
            "edited",
        ],
        Duration::from_secs(30),
    )?;
    assert!(evidence.contains("COMPILED_AI_EDIT_OK"));
    println!("{evidence}");
    Ok(())
}
