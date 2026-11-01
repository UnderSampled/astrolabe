//! Source-derived guard coverage for the unresolved Hype MTH_Modulus operation.
use anyhow::Result;
use astrolabe::{
    godot,
    model::{AssetPath, ai::*, io::Content},
    openspace::{ai::AiScriptCodec, codec::FormatCodec},
    rete,
};

fn source_nodes() -> (AiScriptCodec, Vec<u8>) {
    // Fix/17-0/28, schedule script 54. The two source call trees are
    // Cond_Equal(MTH_Modulus(dsg[132], 40), 0) and
    // Cond_Equal(MTH_Modulus(dsg[132] + 20, 40), 0), joined with the
    // designer[2]/designer[3] == 1 checks by Cond_And. The 40 operands are
    // not the outer comparison values. Installed table row 188 names handler
    // 0x00539f60 MTH_Modulus; its callee semantics remain unverified, so raw
    // ID 188 stays unsupported. Row 189 is RelativeXPositionOfPerso.
    let nodes: [(u8, u32, u8); 25] = [
        (0, 0, 1),   // If
        (1, 0, 2),   // Cond_And
        (1, 3, 3),   // Cond_Equal
        (11, 2, 4),  // designer[2]
        (12, 1, 4),  // integer 1
        (1, 3, 3),   // Cond_Equal
        (3, 188, 4), // MTH_Modulus
        (11, 132, 5),
        (12, 40, 5),
        (12, 0, 4),  // compare the first result with integer 0
        (0, 1, 1),   // Then (empty in this focused source-shaped fixture)
        (0, 0, 1),   // If
        (1, 0, 2),   // Cond_And
        (1, 3, 3),   // Cond_Equal
        (11, 3, 4),  // designer[3]
        (12, 1, 4),  // integer 1
        (1, 3, 3),   // Cond_Equal
        (3, 188, 4), // MTH_Modulus
        (2, 0, 5),   // Plus
        (11, 132, 6),
        (12, 20, 6),
        (12, 40, 5),
        (12, 0, 4), // compare the second result with integer 0
        (0, 1, 1),  // Then (empty in this focused source-shaped fixture)
        (8, 0, 0),  // sentinel
    ];

    let mut bytes = Vec::with_capacity(nodes.len() * 8);
    let mut preserved_nodes = Vec::with_capacity(nodes.len());
    for (category, parameter, indent) in nodes {
        bytes.extend_from_slice(&parameter.to_le_bytes());
        bytes.extend_from_slice(&[0, 0, indent, category]);
        preserved_nodes.push(PreservedAiNode {
            category,
            indent,
            reserved: [0; 2],
            operand: PreservedAiOperand::NonRelocated(parameter.to_le_bytes()),
        });
    }
    (
        AiScriptCodec {
            preserved_nodes,
            ..Default::default()
        },
        bytes,
    )
}

fn modulus_fragment(statement: &Statement) -> &UnsupportedFragment {
    let Statement::If {
        condition: Expression::And(_, comparison),
        then,
        otherwise: None,
    } = statement
    else {
        panic!("source condition lost its If/And structure")
    };
    assert!(then.is_empty());
    let Expression::Compare {
        operation: Comparison::Equal,
        left,
        right,
    } = comparison.as_ref()
    else {
        panic!("source MTH_Modulus call lost its equality comparison")
    };
    assert!(matches!(right.as_ref(), Expression::Integer(0)));
    let Expression::Unsupported(fragment) = left.as_ref() else {
        panic!("unknown MTH_Modulus operation became executable")
    };
    fragment
}

fn model_with_script(script: Script) -> Result<AiModel> {
    let variable = DesignerVariable {
        default: DesignerValue::Integer(0),
        save: 0,
        initialize: 0,
    };
    Ok(AiModel {
        path: AssetPath::new("fix:/ai/models/28")?,
        name: "Hype Fix model 28".into(),
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
        variables: vec![variable; 133],
        designer_unknown: [0; 3],
        unknown: [0; 4],
    })
}

#[test]
fn unresolved_modulus_calls_remain_preserved_and_guarded() -> Result<()> {
    let (codec, source_bytes) = source_nodes();
    let script = codec.read(&source_bytes)?;
    assert_eq!(script.statements.len(), 2);

    let first = modulus_fragment(&script.statements[0]);
    assert_eq!(first.start, 6);
    assert_eq!(
        first.reason,
        "unsupported AI expression category 3 operation 188 at node 6"
    );
    assert_eq!(
        first.nodes,
        vec![
            PreservedAiNode {
                category: 3,
                indent: 4,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated(188_u32.to_le_bytes()),
            },
            PreservedAiNode {
                category: 11,
                indent: 5,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated(132_u32.to_le_bytes()),
            },
            PreservedAiNode {
                category: 12,
                indent: 5,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated(40_u32.to_le_bytes()),
            },
        ]
    );

    let second = modulus_fragment(&script.statements[1]);
    assert_eq!(second.start, 17);
    assert_eq!(
        second.reason,
        "unsupported AI expression category 3 operation 188 at node 17"
    );
    assert_eq!(
        second.nodes,
        vec![
            PreservedAiNode {
                category: 3,
                indent: 4,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated(188_u32.to_le_bytes()),
            },
            PreservedAiNode {
                category: 2,
                indent: 5,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated(0_u32.to_le_bytes()),
            },
            PreservedAiNode {
                category: 11,
                indent: 6,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated(132_u32.to_le_bytes()),
            },
            PreservedAiNode {
                category: 12,
                indent: 6,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated(20_u32.to_le_bytes()),
            },
            PreservedAiNode {
                category: 12,
                indent: 5,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated(40_u32.to_le_bytes()),
            },
        ]
    );
    assert!(AiScriptCodec::default().write(&script).is_err());

    let model = model_with_script(script)?;
    model.validate()?;
    let reopened = rete::decode::<AiModel>(&rete::encode(&model)?)?;
    assert_eq!(reopened, model);

    let code = godot::ai::compile(&reopened, &[])?;
    assert!(code.contains("stand_in("));
    assert!(code.contains(
        "source AI node 6: unsupported AI expression category 3 operation 188 at node 6"
    ));
    assert!(code.contains(
        "source AI node 17: unsupported AI expression category 3 operation 188 at node 17"
    ));
    assert!(
        !code.contains('%'),
        "MTH_Modulus leaked into executable generated arithmetic"
    );
    Ok(())
}
