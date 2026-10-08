//! Connected first-visit calls through source encoding, useful AI and Rete.
use anyhow::Result;
use astrolabe::{
    godot,
    model::{AssetPath, AssetRef, ai::*},
    openspace::{ai::AiScriptCodec, codec::FormatCodec},
    rete,
};

#[test]
fn first_visit_actor_calls_and_signed_point_turn_reconstruct_source_and_rete() -> Result<()> {
    let actor = AssetRef::new("level:/persos/world/0/45")?;
    let script = Script {
        statements: vec![
            Statement::AddHitPoints {
                actor: Expression::Actor(ActorTarget::Keyword(ActorKeyword::MainActor)),
                value: Expression::ActorHitPointsMaximum(Box::new(Expression::Actor(
                    ActorTarget::Keyword(ActorKeyword::MainActor),
                ))),
            },
            Statement::Assign {
                variable: 0,
                value: Expression::ActorPosition(ActorTarget::Reference(actor.clone())),
            },
            Statement::Assign {
                variable: 1,
                value: Expression::ActorFacingAngle {
                    receiver: ActorTarget::Keyword(ActorKeyword::MainActor),
                    target: Box::new(Expression::Actor(ActorTarget::Reference(actor.clone()))),
                },
            },
            Statement::SwitchMicrophone {
                actor: Expression::Actor(ActorTarget::Reference(actor.clone())),
                evaluated_second: Some(Expression::Actor(ActorTarget::Keyword(
                    ActorKeyword::MainActor,
                ))),
            },
            Statement::If {
                condition: Expression::DialogIsOverAndChangeBehavior,
                then: vec![Statement::Assign {
                    variable: 1,
                    value: Expression::Real(0.),
                }],
                otherwise: None,
            },
            Statement::IncrementalTurnPosition {
                point: Expression::Vector(Box::new([
                    Expression::Real(-12.8),
                    Expression::Real(143.5),
                    Expression::Real(33.0),
                ])),
                frames: Expression::Integer(-3),
            },
            Statement::SetObjectActive {
                actor: ActorTarget::Keyword(ActorKeyword::SelfActor),
                active: false,
            },
        ],
    };
    let mut codec = AiScriptCodec {
        actor_handles: vec![(actor.clone(), 100)],
        ..Default::default()
    };
    let bytes = codec.write(&script)?;
    for (i, node) in bytes.as_chunks::<8>().0.iter().enumerate() {
        if node[7] == 24 {
            codec.actors.insert(i, actor.clone());
        }
    }
    assert_eq!(codec.read(&bytes)?, script);
    let model = AiModel {
        path: AssetPath::new("level:/ai/intro_calls")?,
        name: "First-visit source calls".into(),
        normal: Some(BehaviorSet {
            behaviors: vec![Behavior {
                id: 0,
                rules: vec![],
                schedule: Some(script),
                unknown: [0; 3],
            }],
            default_behavior: Some(0),
            action_capacity: 0,
            unknown: [0; 3],
        }),
        reflex: None,
        variables: vec![
            DesignerVariable {
                default: DesignerValue::Vector([0.; 3]),
                save: 0,
                initialize: 0,
            },
            DesignerVariable {
                default: DesignerValue::Real(0.),
                save: 0,
                initialize: 0,
            },
        ],
        designer_unknown: [0; 3],
        unknown: [0; 4],
    };
    let reopened = rete::decode::<AiModel>(&rete::encode(&model)?)?;
    assert_eq!(reopened, model);
    let code = godot::ai::compile(&reopened, &[])?;
    assert!(code.contains("int(maximum_life_actor_"));
    assert!(code.contains(".source_hit_points.maximum)"));
    assert!(code.contains("source_actor_node(\"level:/persos/world/0/45\")"));
    assert!(code.contains(".source_position()"));
    assert!(code.contains("source_turn_angles_to_position(turn_point_"));
    assert!(code.contains("source_motion_binding_present"));
    assert!(!code.contains("remaining_2 < 0: remaining_2 = 1"));
    assert!(code.contains("source_set_object_active_state(false)"));
    assert!(code.contains("57.295780181884766"));
    assert!(code.contains("source_dialog_is_over_and_change_behavior(main_actor)"));
    assert!(
        code.find("var microphone_actor_").unwrap()
            < code.find("var _microphone_discarded_actor_").unwrap()
    );
    Ok(())
}
