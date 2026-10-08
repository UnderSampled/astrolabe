//! Source-derived Action_WaitInteger decoding and native polling semantics.
// Source anchor: Fix/17-0/28 Normal 49 / Script 241 (catalogue `4060e60`),
// callback `0054ddb0`; the reached tree compares Fix DSG[5], selector 5, and
// MainActor DSG[7].
use anyhow::{Result, anyhow, ensure};
use astrolabe::{
    godot,
    model::{
        AssetPath, AssetRef,
        ai::{
            AiModel, Behavior, BehaviorSet, DesignerValue, DesignerVariable, Expression, Script,
            Statement,
        },
        character::Perso,
        io::Content,
    },
    openspace::{ai::AiScriptCodec, codec::FormatCodec},
    rete,
};
use std::{fs, path::PathBuf, time::Duration};

fn node(bytes: &mut Vec<u8>, category: u8, parameter: u32, indent: u8) {
    bytes.extend_from_slice(&parameter.to_le_bytes());
    bytes.extend_from_slice(&[0, 0, indent, category]);
}

fn wait_integer_source(actor_handle: u32, selector: i32) -> Vec<u8> {
    let mut bytes = Vec::new();
    node(&mut bytes, 5, 40, 1); // Action_WaitInteger
    node(&mut bytes, 2, 12, 2); // Global.DSG[5]
    node(&mut bytes, 24, actor_handle, 3);
    node(&mut bytes, 11, 5, 3);
    node(&mut bytes, 12, selector as u32, 2); // relation selector
    node(&mut bytes, 2, 12, 2); // MainActor.DSG[7]
    node(&mut bytes, 0, 5, 3);
    node(&mut bytes, 11, 7, 3);
    node(&mut bytes, 8, 0, 0);
    bytes
}

fn actor_reference() -> Result<AssetRef<Perso>> {
    AssetRef::new("fix:/persos/test/global")
}

fn script_codec(actor: AssetRef<Perso>) -> AiScriptCodec {
    AiScriptCodec {
        actors: [(2, actor.clone())].into(),
        actor_handles: vec![(actor, 0x1234)],
        ..Default::default()
    }
}

fn model(schedule: Script) -> Result<AiModel> {
    Ok(AiModel {
        path: AssetPath::new("level:/ai/models/wait-integer-fixture")?,
        name: "Wait Integer Fixture".into(),
        normal: Some(BehaviorSet {
            behaviors: vec![Behavior {
                id: 0,
                rules: Vec::new(),
                schedule: Some(schedule),
                unknown: [0; 3],
            }],
            default_behavior: Some(0),
            action_capacity: 1,
            unknown: [0; 3],
        }),
        reflex: None,
        variables: vec![
            DesignerVariable {
                default: DesignerValue::Integer(5), // dynamic selector
                save: 0,
                initialize: 0,
            },
            DesignerVariable {
                default: DesignerValue::Integer(0), // following-statement marker
                save: 0,
                initialize: 0,
            },
        ],
        designer_unknown: [0; 3],
        unknown: [0; 4],
    })
}

fn wait_statement(selector: Expression) -> Result<Statement> {
    Ok(Statement::WaitInteger {
        left: Expression::ActorFieldRead {
            actor: actor_reference()?,
            variable: 5,
        },
        selector,
        right: Expression::MainActorFieldRead(7),
    })
}

fn wait_model(selector: Expression) -> Result<AiModel> {
    model(Script {
        statements: vec![
            wait_statement(selector)?,
            Statement::Assign {
                variable: 1,
                value: Expression::Integer(1),
            },
        ],
    })
}

#[test]
fn raw40_roundtrips_live_fix_and_main_actor_operands_through_rete_and_codegen() -> Result<()> {
    let actor = actor_reference()?;
    let codec = script_codec(actor.clone());
    let source = wait_integer_source(0x1234, 5);
    let script = codec.read(&source)?;
    assert!(matches!(
        script.statements.as_slice(),
        [Statement::WaitInteger {
            left: Expression::ActorFieldRead { actor: found, variable: 5 },
            selector: Expression::Integer(5),
            right: Expression::MainActorFieldRead(7),
        }] if found == &actor
    ));
    assert_eq!(codec.write(&script)?, source);

    let typed = wait_model(Expression::Variable(0))?;
    typed.validate()?;
    let reopened = rete::decode::<AiModel>(&rete::encode(&typed)?)?;
    assert_eq!(reopened, typed);
    let generated = godot::ai::compile(&reopened, &[])?;
    assert!(generated.contains("match wait_selector_0:"), "{generated}");
    assert!(generated.contains("& 0xffffffff"));
    assert!(generated.contains("wait_observed_0 >= 0x80000000"));
    assert!(generated.contains("var wait_brain_0: Node = (actor_brain_0"));
    assert!(generated.contains("main_actor_brain_"));
    Ok(())
}

#[test]
fn unresolved_raw40_actor_operand_is_preserved_and_compiles_fail_closed() -> Result<()> {
    let source = wait_integer_source(0x1234, 5);
    let mut preserved_nodes = Vec::new();
    for (index, node_bytes) in source.as_chunks::<8>().0.iter().enumerate() {
        let parameter = u32::from_le_bytes(node_bytes[..4].try_into()?);
        let category = node_bytes[7];
        let indent = node_bytes[6];
        let operand = if category == 24 {
            astrolabe::model::ai::PreservedAiOperand::Incomplete(
                "synthetic unbound actor reference".into(),
            )
        } else {
            astrolabe::model::ai::PreservedAiOperand::NonRelocated(node_bytes[..4].try_into()?)
        };
        let _ = (index, parameter);
        preserved_nodes.push(astrolabe::model::ai::PreservedAiNode {
            category,
            indent,
            reserved: [node_bytes[4], node_bytes[5]],
            operand,
        });
    }
    let codec = AiScriptCodec {
        preserved_nodes,
        ..Default::default()
    };
    let script = codec.read(&source)?;
    assert!(matches!(
        script.statements.as_slice(),
        [Statement::WaitInteger {
            left: Expression::Unsupported(fragment),
            selector: Expression::Integer(5),
            right: Expression::MainActorFieldRead(7),
        }] if fragment.reason.contains("unbound actor field read")
    ));
    assert!(codec.write(&script).is_err());

    let mut unresolved = wait_model(Expression::Integer(5))?;
    let Statement::WaitInteger { left, .. } = &mut unresolved.normal.as_mut().unwrap().behaviors[0]
        .schedule
        .as_mut()
        .unwrap()
        .statements[0]
    else {
        unreachable!()
    };
    *left = match &script.statements[0] {
        Statement::WaitInteger { left, .. } => left.clone(),
        _ => unreachable!(),
    };
    let generated = godot::ai::compile(&unresolved, &[])?;
    assert!(generated.contains("fail_unsupported"));
    assert!(generated.contains("unbound actor field read"));
    assert!(generated.contains("requires main actor Brain field 7"));
    Ok(())
}

#[test]
fn native_raw40_captures_arguments_polls_signed_fields_and_cancels_pending_waits() -> Result<()> {
    let output = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("output");
    fs::create_dir_all(&output)?;
    let project = tempfile::tempdir_in(output)?;
    godot::support::initialize(project.path())?;
    let generated = godot::ai::compile(&wait_model(Expression::Variable(0))?, &[])?;
    fs::write(project.path().join("compiled_wait_integer.gd"), generated)?;
    fs::write(
        project.path().join("ai_wait_integer.gd"),
        include_bytes!("../godot/tests/ai_wait_integer.gd"),
    )?;
    fs::write(
        project.path().join("ai_wait_integer_fields.gd"),
        include_bytes!("../godot/tests/ai_wait_integer_fields.gd"),
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let log = project.path().join("ai-wait-integer.log");
    let log_argument = log.to_string_lossy().into_owned();
    let run = godot::run_godot(
        &engine,
        project.path(),
        &[
            "--headless",
            "--script",
            "res://ai_wait_integer.gd",
            "--log-file",
            &log_argument,
        ],
        Duration::from_secs(25),
    );
    let log_contents = fs::read_to_string(&log).unwrap_or_default();
    let evidence = run.map_err(|error| anyhow!("{error}\nGodot log: {log_contents}"))?;
    ensure!(
        evidence.contains("AI_WAIT_INTEGER_OK") || log_contents.contains("AI_WAIT_INTEGER_OK"),
        "native WaitInteger harness did not report success\nstdout: {evidence}\nlog: {log_contents}"
    );
    ensure!(
        !evidence.contains("SCRIPT ERROR:") && !log_contents.contains("SCRIPT ERROR:"),
        "native WaitInteger harness reported script errors\nstdout: {evidence}\nlog: {log_contents}"
    );
    Ok(())
}
