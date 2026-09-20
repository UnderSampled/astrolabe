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
