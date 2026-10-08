use anyhow::Result;
use astrolabe::{
    model::{
        AssetPath,
        ai::{
            ActorKeyword, ActorTarget, AiModel, Behavior, BehaviorSet, Comparison, Expression,
            Script, Statement,
        },
    },
    rete,
};

#[test]
fn camera_shift_expressions_round_trip_through_rete() -> Result<()> {
    let model = AiModel {
        path: AssetPath::new("fix:/ai/models/camera-shift-alpha")?,
        name: "Camera Shift Alpha".into(),
        normal: Some(BehaviorSet {
            behaviors: vec![Behavior {
                id: 0,
                rules: vec![Some(Script {
                    statements: vec![
                        Statement::If {
                            condition: Expression::Compare {
                                operation: Comparison::Equal,
                                left: Box::new(Expression::CameraShiftAlpha {
                                    selector: Box::new(Expression::Integer(1)),
                                    receiver: ActorTarget::Keyword(ActorKeyword::SelfActor),
                                }),
                                right: Box::new(Expression::Real(0.0)),
                            },
                            then: vec![Statement::Noop],
                            otherwise: None,
                        },
                        Statement::If {
                            condition: Expression::Compare {
                                operation: Comparison::Equal,
                                left: Box::new(Expression::CameraShiftTeta {
                                    selector: Box::new(Expression::Integer(0)),
                                    receiver: ActorTarget::Keyword(ActorKeyword::SelfActor),
                                }),
                                right: Box::new(Expression::Real(0.0)),
                            },
                            then: vec![Statement::Noop],
                            otherwise: None,
                        },
                    ],
                })],
                schedule: None,
                unknown: [0; 3],
            }],
            default_behavior: Some(0),
            action_capacity: 0,
            unknown: [0; 3],
        }),
        reflex: None,
        variables: Vec::new(),
        designer_unknown: [0; 3],
        unknown: [0; 4],
    };

    let reopened = rete::decode::<AiModel>(&rete::encode(&model)?)?;
    assert_eq!(reopened, model);
    assert_eq!(rete::SCHEMA, 55);
    Ok(())
}
