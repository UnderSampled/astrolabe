//! Priority, suspension and state entry in direct generated GDScript.
use anyhow::Result;
use astrolabe::{
    godot,
    model::{AssetPath, AssetRef, ai::*, io::Content},
    openspace::{ai::AiScriptCodec, codec::FormatCodec},
    rete,
};
use std::{collections::BTreeMap, fs, time::Duration};

fn assign(variable: u16, value: i32) -> Statement {
    Statement::Assign {
        variable,
        value: Expression::Integer(value),
    }
}
fn script(statements: Vec<Statement>) -> Script {
    Script { statements }
}
fn branch(variable: u16, then: Vec<Statement>) -> Option<Script> {
    Some(script(vec![Statement::If {
        condition: Expression::Variable(variable),
        then,
        otherwise: None,
    }]))
}
fn behavior(id: u32, rules: Vec<Option<Script>>, schedule: Vec<Statement>) -> Behavior {
    Behavior {
        id,
        rules,
        schedule: (!schedule.is_empty()).then_some(script(schedule)),
        unknown: [0; 3],
    }
}
fn set(behaviors: Vec<Behavior>) -> Option<BehaviorSet> {
    Some(BehaviorSet {
        behaviors,
        default_behavior: Some(0),
        action_capacity: 2,
        unknown: [0; 3],
    })
}
fn fixture() -> Result<AiModel> {
    let path = AssetPath::new("level:/ai/behavior-flow")?;
    let change = |group, behavior, pause_after| Statement::ChangeBehavior {
        target: BehaviorRef {
            model: AssetRef::from_path(path.clone()),
            group,
            behavior,
        },
        pause_after,
    };
    let wait = || Statement::Wait {
        milliseconds: Expression::Integer(100000),
    };
    Ok(AiModel {
        path: path.clone(),
        name: "Behavior flow fixture".into(),
        normal: set(vec![
            behavior(
                0,
                vec![
                    branch(
                        0,
                        vec![
                            assign(5, 1),
                            Statement::WaitFrames(Expression::Integer(1)),
                            assign(5, 2),
                            assign(0, 0),
                        ],
                    ),
                    branch(
                        1,
                        vec![
                            assign(5, 4),
                            Statement::WaitFrames(Expression::Integer(1)),
                            assign(5, 5),
                            change(BehaviorGroup::Normal, 5, true),
                            assign(5, 99),
                        ],
                    ),
                ],
                vec![
                    assign(6, 7),
                    Statement::Wait {
                        milliseconds: Expression::Variable(8),
                    },
                    assign(7, 8),
                ],
            ),
            behavior(
                5,
                vec![],
                vec![
                    assign(2, 5),
                    change(BehaviorGroup::Normal, 6, false),
                    assign(2, 99),
                ],
            ),
            behavior(
                6,
                vec![],
                vec![
                    change(BehaviorGroup::Normal, 6, false),
                    assign(2, 6),
                    wait(),
                ],
            ),
            behavior(
                7,
                vec![],
                vec![
                    change(BehaviorGroup::Reflex, 1, false),
                    assign(2, 7),
                    wait(),
                ],
            ),
        ]),
        reflex: set(vec![
            behavior(
                0,
                vec![branch(
                    4,
                    vec![
                        assign(4, 0),
                        Statement::WaitFrames(Expression::Integer(1)),
                        assign(5, 3),
                    ],
                )],
                vec![],
            ),
            behavior(
                1,
                vec![Some(script(vec![
                    change(BehaviorGroup::Normal, 6, false),
                    assign(5, 9),
                ]))],
                vec![],
            ),
        ]),
        variables: (0..9)
            .map(|i| DesignerVariable {
                default: if [0, 1, 4].contains(&i) {
                    DesignerValue::Boolean(false)
                } else {
                    DesignerValue::Integer(if i == 8 { 100 } else { 0 })
                },
                save: 0,
                initialize: 0,
            })
            .collect(),
        designer_unknown: [0; 3],
        unknown: [0; 4],
    })
}

#[test]
fn behavior_references_and_yield_pairs_reconstruct_and_validate_ownership() -> Result<()> {
    let model = fixture()?;
    model.validate()?;
    let target = BehaviorRef {
        model: AssetRef::from_path(model.path.clone()),
        group: BehaviorGroup::Normal,
        behavior: 6,
    };
    let script = script(vec![
        Statement::ChangeBehavior {
            target: target.clone(),
            pause_after: true,
        },
        Statement::If {
            condition: Expression::IsInBehavior(target.clone()),
            then: vec![Statement::WaitFrames(Expression::Integer(1))],
            otherwise: None,
        },
    ]);
    let codec = AiScriptCodec {
        behaviors: BTreeMap::from([(3, target.clone()), (7, target.clone())]),
        behavior_handles: vec![(target, 0x1234)],
        ..Default::default()
    };
    let bytes = codec.write(&script)?;
    assert_eq!(codec.read(&bytes)?, script);
    let mut wrong = model.clone();
    if let Statement::ChangeBehavior { target, .. } = &mut wrong.normal.as_mut().unwrap().behaviors
        [1]
    .schedule
    .as_mut()
    .unwrap()
    .statements[1]
    {
        target.model = AssetRef::new("fix:/ai/another-model")?;
    }
    assert!(wrong.validate().is_err());
    let mut wrong = model.clone();
    wrong
        .normal
        .as_mut()
        .unwrap()
        .behaviors
        .retain(|b| b.id != 6);
    assert!(wrong.validate().is_err());
    // A yield policy cannot silently attach to an unrelated or missing operation.
    assert!(AiScriptCodec::default().read(&bytes[..16]).is_err());
    let mut wrong = bytes.clone();
    wrong[8..12].copy_from_slice(&2u32.to_le_bytes());
    assert!(codec.read(&wrong).is_err());
    Ok(())
}

#[test]
fn native_rules_preempt_schedules_and_reflex_actions_own_the_tick() -> Result<()> {
    let model = fixture()?;
    let reopened = rete::decode::<AiModel>(&rete::encode(&model)?)?;
    let compiled = godot::ai::compile(&model, &[])?;
    assert_eq!(compiled, godot::ai::compile(&reopened, &[])?);
    let temp = tempfile::tempdir()?;
    godot::support::initialize(temp.path())?;
    fs::write(temp.path().join("flow.gd"), compiled)?;
    fs::write(
        temp.path().join("check.gd"),
        include_str!("check_behavior_flow.gd"),
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let evidence = godot::run_godot(
        &engine,
        temp.path(),
        &["--headless", "--script", "res://check.gd"],
        Duration::from_secs(15),
    )?;
    assert!(evidence.contains("NATIVE_BEHAVIOR_FLOW_OK"));
    println!("{evidence}");
    Ok(())
}
