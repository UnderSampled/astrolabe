//! Source-derived coverage for Proc_ChangeComport (category 4, raw 207).
//! The installed dispatch row resolves to 0x00542650; the handler changes a
//! different actor's Normal behavior pointer and resets that actor's schedule.
use anyhow::Result;
use astrolabe::{
    godot,
    model::{
        AssetPath, AssetRef,
        ai::{
            ActorKeyword, ActorTarget, AiModel, Behavior, BehaviorGroup, BehaviorRef, BehaviorSet,
            PreservedAiNode, PreservedAiOperand, Script, Statement,
        },
        io::Content,
    },
    openspace::{ai::AiScriptCodec, codec::FormatCodec},
    rete,
};

fn node(bytes: &mut Vec<u8>, category: u8, parameter: u32, indent: u8) {
    bytes.extend_from_slice(&parameter.to_le_bytes());
    bytes.extend_from_slice(&[0, 0, indent, category]);
}

fn source_proc_change_comport() -> Vec<u8> {
    let mut bytes = Vec::new();
    node(&mut bytes, 4, 207, 1); // Proc_ChangeComport (signature PQ)
    node(&mut bytes, 0, 5, 2); // Keyword_MainActor
    node(&mut bytes, 32, 0x5678, 2); // Bound Fix model 28 Normal 50
    node(&mut bytes, 8, 0, 0); // Sentinel
    bytes
}

fn target() -> Result<BehaviorRef> {
    Ok(BehaviorRef {
        model: AssetRef::new("fix:/ai/models/28")?,
        group: BehaviorGroup::Normal,
        behavior: 50,
    })
}

fn model(statement: Statement) -> Result<AiModel> {
    Ok(AiModel {
        path: AssetPath::new("level:/ai/models/334")?,
        name: "iDummy_Load_Manoir_MAD".into(),
        normal: Some(BehaviorSet {
            behaviors: vec![Behavior {
                id: 0,
                rules: Vec::new(),
                schedule: Some(Script {
                    statements: vec![statement],
                }),
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
    })
}

#[test]
fn raw207_roundtrips_and_dispatches_to_the_bound_target_brain() -> Result<()> {
    let target = target()?;
    let codec = AiScriptCodec {
        behaviors: [(2, target.clone())].into(),
        behavior_handles: vec![(target.clone(), 0x5678)],
        ..Default::default()
    };
    let source = source_proc_change_comport();
    let parsed = codec.read(&source)?;
    assert_eq!(
        parsed.statements,
        vec![Statement::ChangeActorBehavior {
            actor: ActorTarget::Keyword(ActorKeyword::MainActor),
            target: target.clone(),
        }]
    );
    assert_eq!(codec.write(&parsed)?, source);

    let model = model(parsed.statements[0].clone())?;
    model.validate()?;
    let reopened = rete::decode::<AiModel>(&rete::encode(&model)?)?;
    assert_eq!(reopened, model);
    let generated = godot::ai::compile(&reopened, &[])?;
    assert!(generated.contains("var change_actor_0: Node3D = main_actor"));
    assert!(generated.contains("if change_actor_0 == character():"));
    assert!(generated.contains("change_actor_0.get_node_or_null(\"Brain\")"));
    assert!(
        generated
            .contains("str(change_brain_0.get(\"source_model_path\")) != \"fix:/ai/models/28\"")
    );
    assert!(generated.contains("change_brain_0.call(\"change_normal\", 50, false)"));
    assert!(generated.contains("same-actor status semantics are unresolved"));
    assert!(!generated.contains("unsupported AI statement category 4 operation 207"));
    Ok(())
}

#[test]
fn unresolved_raw207_behavior_remains_preserved_and_unwritable_as_typed() -> Result<()> {
    let source = source_proc_change_comport();
    let codec = AiScriptCodec {
        preserved_nodes: vec![
            PreservedAiNode {
                category: 4,
                indent: 1,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated(207u32.to_le_bytes()),
            },
            PreservedAiNode {
                category: 0,
                indent: 2,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated(5u32.to_le_bytes()),
            },
            PreservedAiNode {
                category: 32,
                indent: 2,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated(0x5678u32.to_le_bytes()),
            },
            PreservedAiNode {
                category: 8,
                indent: 0,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated([0; 4]),
            },
        ],
        ..Default::default()
    };
    let parsed = codec.read(&source)?;
    let Statement::Unsupported(fragment) = &parsed.statements[0] else {
        anyhow::bail!("unbound Proc_ChangeComport behavior was not preserved")
    };
    assert!(fragment.reason.contains("behavior operand is unbound"));
    assert_eq!(fragment.nodes.len(), 3);
    assert!(matches!(
        fragment.nodes[2].operand,
        PreservedAiOperand::NonRelocated(bytes) if bytes == 0x5678u32.to_le_bytes()
    ));
    assert!(codec.write(&parsed).is_err());
    Ok(())
}

#[test]
fn raw207_does_not_accept_the_separate_reflex_behavior_form() -> Result<()> {
    let mut reflex_target = target()?;
    reflex_target.group = BehaviorGroup::Reflex;
    let codec = AiScriptCodec {
        behaviors: [(2, reflex_target.clone())].into(),
        behavior_handles: vec![(reflex_target, 0x5678)],
        preserved_nodes: vec![
            PreservedAiNode {
                category: 4,
                indent: 1,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated(207u32.to_le_bytes()),
            },
            PreservedAiNode {
                category: 0,
                indent: 2,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated(5u32.to_le_bytes()),
            },
            PreservedAiNode {
                category: 32,
                indent: 2,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated(0x5678u32.to_le_bytes()),
            },
            PreservedAiNode {
                category: 8,
                indent: 0,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated([0; 4]),
            },
        ],
        ..Default::default()
    };
    let parsed = codec.read(&source_proc_change_comport())?;
    let Statement::Unsupported(fragment) = &parsed.statements[0] else {
        anyhow::bail!("raw207 incorrectly accepted a Reflex behavior reference")
    };
    assert!(fragment.reason.contains("target is not a Normal behavior"));
    assert!(codec.write(&parsed).is_err());
    Ok(())
}

#[test]
fn self_target_raw207_stays_typed_but_codegen_fails_closed() -> Result<()> {
    let statement = Statement::ChangeActorBehavior {
        actor: ActorTarget::Keyword(ActorKeyword::SelfActor),
        target: target()?,
    };
    let compiled = godot::ai::compile(&model(statement)?, &[])?;
    assert!(compiled.contains("if change_actor_0 == character():"));
    assert!(compiled.contains("fail_unsupported("));
    assert!(compiled.contains("same-actor status semantics are unresolved"));
    assert!(compiled.contains("change_brain_0.call(\"change_normal\", 50, false)"));
    Ok(())
}
