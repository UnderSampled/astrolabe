//! Source-derived coverage for Proc_SetVisualGMTDiffuseCoef (category 4/op 237).
use anyhow::{Context, Result};
use astrolabe::{
    godot,
    model::{
        AssetPath, AssetRef,
        ai::{
            AiModel, Behavior, BehaviorSet, DesignerValue, DesignerVariable, Expression, Script,
            Statement,
        },
        geometry::Material,
        io::Content,
    },
    openspace::{ai::AiScriptCodec, codec::FormatCodec},
    rete,
};

fn node(bytes: &mut Vec<u8>, category: u8, parameter: u32, indent: u8) {
    bytes.extend_from_slice(&parameter.to_le_bytes());
    bytes.extend_from_slice(&[0, 0, indent, category]);
}

fn source_gated_diffuse_set() -> Vec<u8> {
    let mut bytes = Vec::new();
    node(&mut bytes, 0, 0, 1); // If
    node(&mut bytes, 11, 16, 2); // live branch condition
    node(&mut bytes, 0, 1, 1); // Then
    node(&mut bytes, 4, 237, 2); // Proc_SetVisualGMTDiffuseCoef
    node(&mut bytes, 36, 0x1234, 3); // bound GMT identity
    node(&mut bytes, 11, 87, 3); // packed DSG field, read at execution time
    node(&mut bytes, 8, 0, 0); // sentinel
    bytes
}

fn model(script: Script) -> Result<AiModel> {
    Ok(AiModel {
        path: AssetPath::new("fix:/ai/models/28")?,
        name: "Hype".into(),
        normal: Some(BehaviorSet {
            behaviors: vec![Behavior {
                id: 0,
                rules: Vec::new(),
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
                default: DesignerValue::Integer(0),
                save: 0,
                initialize: 0,
            };
            88
        ],
        designer_unknown: [0; 3],
        unknown: [0; 4],
    })
}

#[test]
fn raw237_preserves_source_material_identity_and_live_dsg_gate() -> Result<()> {
    let material = AssetRef::<Material>::new("fix:/materials/06-01/material-1")?;
    let codec = AiScriptCodec {
        materials: [(4, material.clone())].into(),
        material_handles: vec![(material.clone(), 0x1234)],
        ..Default::default()
    };
    let source = source_gated_diffuse_set();
    let parsed = codec.read(&source)?;
    assert_eq!(codec.write(&parsed)?, source);
    assert_eq!(
        parsed,
        Script {
            statements: vec![Statement::If {
                condition: Expression::Variable(16),
                then: vec![Statement::SetVisualGMTDiffuseCoef {
                    material: material.clone(),
                    value: Expression::Variable(87),
                }],
                otherwise: None,
            }],
        }
    );

    let model = model(parsed)?;
    model.validate()?;
    let reopened = rete::decode::<AiModel>(&rete::encode(&model)?)?;
    assert_eq!(reopened, model);

    let generated = godot::ai::compile(&reopened, &[])?;
    let condition = generated
        .find("if bool(dsg_16):")
        .with_context(|| format!("source branch guard was not compiled: {generated}"))?;
    let material_call = generated
        .find("character().call(\"source_set_visual_gmt_diffuse_coef\"")
        .context("raw237 renderer call was not emitted")?;
    assert!(material_call > condition, "{generated}");
    let target_guard = generated
        .find("character().call(\"source_visual_material_target_ready\"")
        .context("raw237 material target preflight was not emitted")?;
    let packed_value = generated
        .find("int(dsg_87)")
        .context("raw237 packed operand was not compiled")?;
    assert!(target_guard < packed_value, "{generated}");
    assert!(
        generated.contains("\"fix:/materials/06-01/material-1\", int(dsg_87)"),
        "{generated}"
    );
    assert!(
        generated.contains("has_method(\"source_visual_material_target_ready\")")
            && generated.contains("has_method(\"source_set_visual_gmt_diffuse_coef\")")
            && generated.contains("fail_unsupported("),
        "renderer capability must fail explicitly until the native visual-material service is present: {generated}"
    );
    Ok(())
}

#[test]
fn raw237_does_not_guess_an_unbound_gmt_identity() -> Result<()> {
    let source = source_gated_diffuse_set();
    let error = AiScriptCodec::default()
        .read(&source)
        .expect_err("unbound category-36 material must not become a guessed target");
    assert!(error.to_string().contains("bound category-36 material"));
    Ok(())
}

#[test]
fn raw237_rejects_non_integer_designer_storage_without_reinterpretation() -> Result<()> {
    let material = AssetRef::<Material>::new("fix:/materials/06-01/material-1")?;
    let codec = AiScriptCodec {
        materials: [(4, material.clone())].into(),
        material_handles: vec![(material, 0x1234)],
        ..Default::default()
    };
    let parsed = codec.read(&source_gated_diffuse_set())?;
    let mut model = model(parsed)?;
    model.variables[87].default = DesignerValue::Real(0.0);
    model.validate()?;
    let error = godot::ai::compile(&model, &[])
        .expect_err("non-integer packed storage must not be numerically reinterpreted");
    assert!(error.to_string().contains("32-bit integer designer field"));
    Ok(())
}
