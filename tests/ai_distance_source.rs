//! Source-byte round trip and native lowering for Manoir's tunnel distance.
use anyhow::Result;
use astrolabe::{
    godot,
    model::{AssetPath, ai::*, io::Content},
    openspace::{ai::AiScriptCodec, codec::FormatCodec},
    rete,
};
use std::{fs, time::Duration};

fn source_nodes() -> Vec<u8> {
    // The reached Manoir model 334 condition contains category-3 operand
    // [08 00 00 00] at Level:11:01+0xd400, followed by MainActor, and compares
    // it with Real(6.0). Its preserved Rete node is indented 4; this compact
    // source-shaped tree keeps that nesting and the same operation bytes.
    // The extra IsValidActor is only a valid boolean sibling for this isolated
    // And fixture; the local Rete source has an IsInBehavior sibling instead.
    let nodes: [(u8, u32, u8); 11] = [
        (0, 0, 1), // If
        (1, 0, 2), // Cond_And
        (1, 5, 3), // Cond_Less
        (3, 8, 4), // installed raw category-3 ID 8
        (0, 5, 5), // MainActor
        (13, 6.0f32.to_bits(), 4),
        (1, 64, 3),  // valid boolean sibling
        (0, 5, 4),   // MainActor
        (0, 1, 1),   // Then
        (4, 206, 2), // Noop
        (8, 0, 0),   // sentinel
    ];
    let mut bytes = Vec::with_capacity(nodes.len() * 8);
    for (category, parameter, indent) in nodes {
        bytes.extend_from_slice(&parameter.to_le_bytes());
        bytes.extend_from_slice(&[0, 0, indent, category]);
    }
    bytes
}

fn model(script: Script) -> Result<AiModel> {
    Ok(AiModel {
        path: AssetPath::new("level:/ai/manoir-distance-fixture")?,
        name: "Manoir distance source fixture".into(),
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
fn installed_id_8_preserves_source_bytes_and_lowers_the_handler_formula() -> Result<()> {
    // The installed PE row 8 at 0x600494 is named Func_DistanceXZToPerso
    // (signature P, handler 0x537aa0). That handler passes selector 3 to
    // 0x52ea00; 0x52e910 selector 3 computes sqrt(x^2+y^2). The helper first
    // transforms the actor displacement by the inverse of the explicit
    // operand actor's matrix. Thus the lowering follows handler behavior,
    // while the PE/Raymap name-table disagreement stays explicit here.
    let source = source_nodes();
    let script = AiScriptCodec::default().read(&source)?;
    let Statement::If {
        condition: Expression::And(distance, _),
        then,
        otherwise: None,
    } = &script.statements[0]
    else {
        panic!("source If/And structure changed")
    };
    assert!(matches!(then.as_slice(), [Statement::Noop]));
    let Expression::Compare {
        operation: Comparison::Less,
        left,
        right,
    } = distance.as_ref()
    else {
        panic!("source tunnel threshold comparison changed")
    };
    assert_eq!(
        left.as_ref(),
        &Expression::ActorLocalSourceXYDistance(Box::new(Expression::Actor(ActorTarget::Keyword(
            ActorKeyword::MainActor
        ),)))
    );
    assert_eq!(right.as_ref(), &Expression::Real(6.0));
    assert_eq!(AiScriptCodec::default().write(&script)?, source);

    let model = model(script)?;
    model.validate()?;
    let reopened = rete::decode::<AiModel>(&rete::encode(&model)?)?;
    assert_eq!(reopened, model);
    assert!(
        reopened
            .expressions()
            .iter()
            .any(|expression| matches!(expression, Expression::ActorLocalSourceXYDistance(_)))
    );

    let code = godot::ai::compile(&reopened, &[])?;
    assert!(
        code.contains("NativeContactPickup.distance_xy(spatial_receiver_0, spatial_target_0)"),
        "{code}"
    );
    assert!(
        !code.contains("unsupported AI expression category 3 operation 8"),
        "{code}"
    );

    let temp = tempfile::tempdir()?;
    let project = temp.path().join("godot");
    godot::support::initialize(&project)?;
    fs::write(project.join("compiled_distance.gd"), code)?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    godot::run_godot(
        &engine,
        &project,
        &[
            "--headless",
            "--check-only",
            "--script",
            "res://compiled_distance.gd",
            "--log-file",
            "compiled-distance.log",
        ],
        Duration::from_secs(10),
    )?;
    Ok(())
}
