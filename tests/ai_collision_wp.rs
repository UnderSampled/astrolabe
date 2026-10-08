//! Source-derived containment query decoding, waypoint-memory preservation,
//! and the native 3D inclusive boundary contract.
use anyhow::{Context, Result, ensure};
use astrolabe::{
    godot,
    model::{
        AssetPath, AssetRef, Selection,
        ai::{
            AiModel, Behavior, BehaviorSet, DesignerArray, DesignerValue, DesignerVariable,
            Expression, SourceObjectIdentity, SourceRole, Statement,
        },
        io::Content,
        io::Load,
        route::Waypoint,
    },
    openspace::{
        ai::AiScriptCodec, ai_values::DesignerValueCodec, codec::FormatCodec, content::import_scene,
    },
    rete,
};
use std::{collections::BTreeSet, fs, path::PathBuf, sync::Arc, time::Duration};

fn node(bytes: &mut Vec<u8>, category: u8, parameter: u32, indent: u8) {
    bytes.extend_from_slice(&parameter.to_le_bytes());
    bytes.extend_from_slice(&[0, 0, indent, category]);
}

fn collision_wp_source() -> Vec<u8> {
    let mut bytes = Vec::new();
    node(&mut bytes, 0, 0, 1); // If
    node(&mut bytes, 1, 108, 2); // Cond_CollisionWP
    node(&mut bytes, 0, 4, 3); // Self
    node(&mut bytes, 2, 26, 3); // ArrayElement
    node(&mut bytes, 11, 157, 4); // SourceWaypointArray
    node(&mut bytes, 11, 113, 4); // unsigned index
    node(&mut bytes, 0, 1, 1); // Then
    node(&mut bytes, 4, 206, 2); // Noop
    node(&mut bytes, 8, 0, 0); // Sentinel
    bytes
}

fn direct_waypoint_assignment_source() -> Vec<u8> {
    let mut bytes = Vec::new();
    node(&mut bytes, 2, 11, 1); // Assign to actor field
    node(&mut bytes, 2, 12, 2); // Actor dot
    node(&mut bytes, 0, 5, 3); // MainActor
    node(&mut bytes, 11, 157, 3); // Designer field 157
    node(&mut bytes, 29, 0x1234, 2); // Direct Waypoint resource value
    node(&mut bytes, 8, 0, 0); // Sentinel
    bytes
}

fn waypoint_ref() -> Result<AssetRef<Waypoint>> {
    AssetRef::new("level:/waypoints/17-1/56")
}

fn game() -> PathBuf {
    std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../disc"))
}

fn source_identity(allocation: u32) -> SourceObjectIdentity {
    SourceObjectIdentity {
        role: SourceRole::Level,
        block: [17, 1],
        allocation,
    }
}

fn array_source(handles: [u32; 2]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&8u32.to_le_bytes()); // Waypoint elements
    bytes.extend_from_slice(&[2, 0xa1, 0xb2, 0xc3]); // count + source header bytes
    bytes.extend_from_slice(&handles[0].to_le_bytes());
    bytes.extend_from_slice(&[0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17]);
    bytes.extend_from_slice(&handles[1].to_le_bytes());
    bytes.extend_from_slice(&[0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27]);
    bytes
}

fn model(array: DesignerValue) -> Result<AiModel> {
    let mut variables = (0..158)
        .map(|_| DesignerVariable {
            default: DesignerValue::Boolean(false),
            save: 0,
            initialize: 0,
        })
        .collect::<Vec<_>>();
    variables[113].default = DesignerValue::UnsignedByte(0);
    variables[157].default = array;
    Ok(AiModel {
        path: AssetPath::new("level:/ai/models/collision-wp-fixture")?,
        name: "Collision Waypoint Fixture".into(),
        normal: Some(BehaviorSet {
            behaviors: vec![Behavior {
                id: 0,
                rules: vec![Some(AiScriptCodec::default().read(&collision_wp_source())?)],
                schedule: None,
                unknown: [0; 3],
            }],
            default_behavior: Some(0),
            action_capacity: 1,
            unknown: [0; 3],
        }),
        reflex: None,
        variables,
        designer_unknown: [0; 3],
        unknown: [0; 4],
    })
}

#[test]
fn raw108_roundtrips_and_rete_keeps_dynamic_waypoint_array_identity() -> Result<()> {
    let bytes = collision_wp_source();
    let script = AiScriptCodec::default().read(&bytes)?;
    assert_eq!(AiScriptCodec::default().write(&script)?, bytes);
    assert!(matches!(
        script.statements.as_slice(),
        [Statement::If {
            condition: Expression::WaypointSphereContains { actor, waypoint },
            ..
        }] if **actor == Expression::Actor(astrolabe::model::ai::ActorTarget::Keyword(
                astrolabe::model::ai::ActorKeyword::SelfActor
            ))
            && **waypoint == Expression::ArrayElement {
                array: Box::new(Expression::Variable(157)),
                index: Box::new(Expression::Variable(113)),
            }
    ));

    let reference = waypoint_ref()?;
    let direct_codec = AiScriptCodec {
        waypoints: [(4, reference.clone())].into(),
        waypoint_handles: vec![(reference.clone(), 0x1234)],
        ..Default::default()
    };
    let direct_bytes = direct_waypoint_assignment_source();
    let direct = direct_codec.read(&direct_bytes)?;
    assert_eq!(direct_codec.write(&direct)?, direct_bytes);
    assert_eq!(
        direct.statements,
        vec![Statement::MainActorFieldAssign {
            variable: 157,
            value: Expression::WaypointReference(reference.clone()),
        }]
    );

    let typed = model(DesignerValue::WaypointArray(DesignerArray::new(vec![
        Some(reference.clone()),
        None,
    ])))?;
    typed.validate()?;
    let reopened = rete::decode::<AiModel>(&rete::encode(&typed)?)?;
    assert_eq!(reopened, typed);
    assert!(reopened.waypoint_references().contains(&&reference));
    Ok(())
}

#[test]
fn designer_waypoint_array_preserves_null_unknown_bytes_and_unresolved_identities() -> Result<()> {
    let reference = waypoint_ref()?;
    let bytes = array_source([0x1234, 0]);
    let codec = DesignerValueCodec {
        kind: 23,
        waypoints: [(8, Some(reference.clone())), (20, None)].into(),
        waypoint_handles: vec![(Some(reference.clone()), 0x1234), (None, 0)],
        ..Default::default()
    };
    let decoded = codec.read(&bytes)?;
    assert_eq!(
        decoded,
        DesignerValue::WaypointArray(DesignerArray {
            values: vec![Some(reference.clone()), None],
            unknown: [0xa1, 0xb2, 0xc3],
            element_unknown: [
                (0, [0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17]),
                (1, [0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27]),
            ]
            .into(),
        })
    );
    assert_eq!(codec.write(&decoded)?, bytes);

    let identities = [source_identity(56), source_identity(57)];
    let unresolved = DesignerValueCodec {
        kind: 23,
        waypoints: [(8, Some(reference))].into(),
        source_objects: [
            (8, Some(identities[0].clone())),
            (20, Some(identities[1].clone())),
        ]
        .into(),
        ..Default::default()
    }
    .read(&array_source([0x1234, 0x5678]))?;
    assert!(matches!(
        &unresolved,
        DesignerValue::SourceWaypointArray(value)
            if value.values == identities.map(Some).to_vec()
                && value.unknown == [0xa1, 0xb2, 0xc3]
                && value.element_unknown.len() == 2
    ));
    assert!(
        DesignerValueCodec {
            kind: 23,
            ..Default::default()
        }
        .write(&unresolved)
        .is_err()
    );
    Ok(())
}

#[test]
fn compiler_bounds_checks_array_and_keeps_unresolved_operands_explicit() -> Result<()> {
    let model = model(DesignerValue::WaypointArray(DesignerArray::new(vec![None])))?;
    let generated = godot::ai::compile(&model, &[])?;
    assert!(generated.contains("waypoint_index_0 < 0"), "{generated}");
    assert!(generated.contains("waypoint_index_0 >= waypoint_array_0.size()"));
    assert!(generated.contains("has_method(\"source_world_position\")"));
    assert!(generated.contains("fail_unsupported"));
    assert!(generated.contains("source_actor_in_waypoint_sphere"));

    let output = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("output");
    fs::create_dir_all(&output)?;
    let project = tempfile::tempdir_in(output)?;
    godot::support::initialize(project.path())?;
    fs::write(project.path().join("compiled_collision_wp.gd"), generated)?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    godot::run_godot(
        &engine,
        project.path(),
        &[
            "--headless",
            "--check-only",
            "--script",
            "res://compiled_collision_wp.gd",
            "--log-file",
            "collision-wp-compile.log",
        ],
        Duration::from_secs(15),
    )?;
    Ok(())
}

#[test]
fn native_waypoint_sphere_uses_inclusive_full_3d_and_fresh_actor_relative_position() -> Result<()> {
    let output = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("output");
    fs::create_dir_all(&output)?;
    let project = tempfile::tempdir_in(output)?;
    godot::support::initialize(project.path())?;
    fs::write(
        project.path().join("test_collision_wp.gd"),
        include_bytes!("../godot/tests/ai_collision_wp.gd"),
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let log = project.path().join("collision-wp.log");
    let log_argument = log.to_string_lossy().into_owned();
    let evidence = godot::run_godot(
        &engine,
        project.path(),
        &[
            "--headless",
            "--script",
            "res://test_collision_wp.gd",
            "--log-file",
            &log_argument,
        ],
        Duration::from_secs(15),
    );
    let log_contents = fs::read_to_string(&log).unwrap_or_default();
    let evidence =
        evidence.map_err(|error| anyhow::anyhow!("{error:#}\nGodot log: {log_contents}"))?;
    let log = log_contents;
    assert!(
        evidence.contains("AI_COLLISION_WP_OK") || log.contains("AI_COLLISION_WP_OK"),
        "stdout: {evidence}\nlog: {log}"
    );
    Ok(())
}

#[test]
#[ignore = "requires local Hype data; verifies Manor model28 and model334 waypoint identities"]
fn manor_waypoint_array_and_direct_category29_values_resolve_to_shared_assets() -> Result<()> {
    let mut imported = import_scene(&game(), Selection::hype("manoir", "en")?)?;
    let collision_ref = AssetRef::<AiModel>::new("fix:/ai/models/28")?;
    let collision: Arc<AiModel> = imported.load(&collision_ref)?;
    let query = collision
        .expressions()
        .into_iter()
        .find_map(|expression| match expression {
            Expression::WaypointSphereContains { actor, waypoint } => Some((actor, waypoint)),
            _ => None,
        })
        .context("Hype model28 raw108 query missing")?;
    assert!(matches!(
        query.0.as_ref(),
        Expression::Actor(astrolabe::model::ai::ActorTarget::Keyword(
            astrolabe::model::ai::ActorKeyword::SelfActor
        ))
    ));
    ensure!(
        matches!(
            collision
                .variables
                .get(113)
                .map(|variable| &variable.default),
            Some(DesignerValue::UnsignedByte(_))
        ),
        "model28 waypoint index field is not source unsigned byte"
    );
    ensure!(
        matches!(
            collision
                .variables
                .get(157)
                .map(|variable| &variable.default),
            Some(DesignerValue::WaypointArray(_)) | Some(DesignerValue::SourceWaypointArray(_))
        ),
        "model28 waypoint field is not preserved as a waypoint array"
    );
    let Expression::ArrayElement { array, index } = query.1.as_ref() else {
        anyhow::bail!("Cond_CollisionWP waypoint operand lost dynamic ArrayElement")
    };
    assert_eq!(array.as_ref(), &Expression::Variable(157));
    assert_eq!(index.as_ref(), &Expression::Variable(113));
    let collision_reopened = rete::decode::<AiModel>(&rete::encode(collision.as_ref())?)?;
    assert_eq!(collision_reopened, *collision);

    let writer_ref = AssetRef::<AiModel>::new("level:/ai/models/334")?;
    let writer: Arc<AiModel> = imported.load(&writer_ref)?;
    let direct_waypoints = writer
        .expressions()
        .into_iter()
        .filter_map(|expression| match expression {
            Expression::WaypointReference(reference) => Some(reference.path.to_string()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        direct_waypoints,
        [
            "level:/waypoints/17-1/56".to_owned(),
            "level:/waypoints/17-1/57".to_owned(),
            "level:/waypoints/17-1/58".to_owned(),
        ]
        .into()
    );
    let writer_reopened = rete::decode::<AiModel>(&rete::encode(writer.as_ref())?)?;
    assert_eq!(writer_reopened, *writer);
    for identity in 56..=58 {
        let waypoint: Arc<Waypoint> =
            imported.load(&AssetRef::new(format!("level:/waypoints/17-1/{identity}"))?)?;
        assert_eq!(waypoint.source, source_identity(identity));
        waypoint.validate()?;
    }
    Ok(())
}
