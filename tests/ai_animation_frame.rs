//! Source-derived coverage for Hype's current animation frame query.
use anyhow::Result;
use astrolabe::{
    godot,
    model::{AssetPath, ai::*, io::Content},
    openspace::{ai::AiScriptCodec, codec::FormatCodec},
    rete,
};

fn source_barrel_frame_condition() -> Vec<u8> {
    // Level/17-1/612 behavior 2 rule script 627 compares the current frame
    // with 5 after checking one of three authored barrel actions.
    let nodes = [
        (0, 0, 1),                  // If
        (1, 6, 2),                  // Cond_Greater
        (3, 184, 3),                // Func_GetCurrectAnimFrame
        (13, 5.0_f32.to_bits(), 3), // float 5
        (0, 1, 1),                  // Then
        (4, 206, 2),                // Noop: the source effect is outside this focused fixture
        (8, 0, 0),                  // sentinel
    ];
    let mut bytes = Vec::with_capacity(nodes.len() * 8);
    for (kind, parameter, indent) in nodes {
        bytes.extend_from_slice(&parameter.to_le_bytes());
        bytes.extend([0, 0, indent, kind]);
    }
    bytes
}

fn model_with_script(script: Script) -> Result<AiModel> {
    Ok(AiModel {
        path: AssetPath::new("level:/ai/animation-frame")?,
        name: "Animation frame query".into(),
        normal: Some(BehaviorSet {
            behaviors: vec![Behavior {
                id: 0,
                rules: Vec::new(),
                schedule: Some(script),
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

#[test]
fn current_animation_frame_round_trips_and_fails_closed_without_a_clip() -> Result<()> {
    let codec = AiScriptCodec::default();
    let source = source_barrel_frame_condition();
    let script = codec.read(&source)?;
    assert_eq!(
        script,
        Script {
            statements: vec![Statement::If {
                condition: Expression::Compare {
                    operation: Comparison::Greater,
                    left: Box::new(Expression::CurrentAnimationFrame),
                    right: Box::new(Expression::Real(5.0)),
                },
                then: vec![Statement::Noop],
                otherwise: None,
            }],
        }
    );
    assert_eq!(codec.write(&script)?, source);

    let model = model_with_script(script)?;
    model.validate()?;
    let reopened = rete::decode::<AiModel>(&rete::encode(&model)?)?;
    assert_eq!(reopened, model);

    let generated = godot::ai::compile(&reopened, &[])?;
    assert!(
        generated
            .contains("if character().current_clip == null or character().sample_rate <= 0.0:"),
        "{generated}"
    );
    assert!(
        generated.contains(
            "Func_GetCurrectAnimFrame requires an active native animation clip with a positive source frame rate"
        ),
        "{generated}"
    );
    assert!(
        generated.contains("var current_animation_frame_0: int = character().frame_index"),
        "{generated}"
    );
    assert!(
        generated.contains("fail_unsupported(") && generated.contains("return"),
        "{generated}"
    );
    Ok(())
}
