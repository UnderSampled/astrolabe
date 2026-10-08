use super::*;
use anyhow::{Context, Result};
use serde::Serialize;
use std::{collections::BTreeSet, path::PathBuf};

fn source_bool_record(block: u8, module: u8, offset: u32, value: u8) -> Result<SaiRecord> {
    let mut bytes = Vec::with_capacity(15);
    bytes.push(block);
    bytes.push(module);
    bytes.extend_from_slice(&offset.to_le_bytes());
    bytes.extend_from_slice(&0x0204u16.to_le_bytes());
    bytes.push(0);
    bytes.push(0x10);
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.push(value);
    let table = crate::openspace::save_game::parse_records(&bytes)?;
    table
        .records
        .into_iter()
        .next()
        .context("synthetic SAI Boolean record was not parsed")
}

fn candidate(
    source_object: Option<&str>,
    source_appearance: Option<&str>,
    source_field: Option<&str>,
) -> SaveAllocationCandidate {
    SaveAllocationCandidate {
        owner: "fix",
        block_size: 0,
        allocation_index: Some(0),
        allocation_data_start: Some(0),
        allocation_end: Some(1),
        allocation_issue: None,
        data_prefix_hex: None,
        source_object: source_object.map(str::to_owned),
        source_appearance: source_appearance.map(str::to_owned),
        source_field: source_field.map(str::to_owned),
        source_meaning: None,
        source_rule: Some("test"),
        source_type: Some("designer type 0".into()),
        field_match_ambiguity: Vec::new(),
    }
}

#[derive(Serialize)]
struct GodotField<'a> {
    identity: &'a ActorBrainFieldIdentity,
    value: &'a DesignerValue,
    appearance_aliases: &'a [AssetPath],
}

fn godot_payload(projection: &ActorBrainSaveProjection) -> Result<serde_json::Value> {
    let fields = projection
        .fields
        .iter()
        .map(|(identity, value)| GodotField {
            identity,
            value: &value.value,
            appearance_aliases: &value.appearance_aliases,
        })
        .collect::<Vec<_>>();
    Ok(serde_json::to_value(fields)?)
}

#[test]
fn godot_facing_brain_projection_contains_only_semantic_identity_and_value() -> Result<()> {
    let identity =
        ActorBrainFieldIdentity::new("level:/persos/world/0/0", ActorBrainFieldId::dsg(11))?;
    let value = ActorBrainFieldValue {
        value: DesignerValue::Boolean(false),
        appearance_aliases: Vec::new(),
    };
    let projection = ActorBrainSaveProjection {
        fields: BTreeMap::from([(identity, value)]),
    };
    let payload = godot_payload(&projection)?;
    let entry = payload[0].as_object().context("field is not an object")?;
    ensure!(
        entry.len() == 3,
        "semantic payload contains unexpected fields"
    );
    let identity_json = entry["identity"]
        .as_object()
        .context("identity is not an object")?;
    ensure!(
        identity_json.len() == 3,
        "identity contains source mechanics"
    );
    ensure!(identity_json["owner"] == "level:/persos/world/0/0");
    ensure!(identity_json["component"] == "brain");
    ensure!(identity_json["field"] == "dsg_11");
    ensure!(entry["value"]["Boolean"] == false);
    let json = serde_json::to_string(&payload)?;
    for forbidden in [
        "address",
        "block",
        "module",
        "offset",
        "relocation",
        "allocation",
        "save_flags",
    ] {
        ensure!(
            !json.contains(forbidden),
            "Godot-facing payload leaked {forbidden}"
        );
    }
    Ok(())
}

#[test]
fn global_intro_gate_projection_is_address_free_and_typed() -> Result<()> {
    let identity = ActorBrainFieldIdentity::new("fix:/persos/15", ActorBrainFieldId::dsg(147))?;
    let projection = ActorBrainSaveProjection {
        fields: BTreeMap::from([(
            identity,
            ActorBrainFieldValue {
                value: DesignerValue::Boolean(true),
                appearance_aliases: Vec::new(),
            },
        )]),
    };
    let payload = godot_payload(&projection)?;
    let json = serde_json::to_string(&payload)?;
    ensure!(json.contains("fix:/persos/15"));
    ensure!(json.contains("\"field\":\"dsg_147\""));
    ensure!(json.contains("\"Boolean\":true"));
    for forbidden in ["0:17:4562", "address", "offset", "relocation", "save_flags"] {
        ensure!(
            !json.contains(forbidden),
            "semantic Global flag payload leaked {forbidden}"
        );
    }
    Ok(())
}

#[test]
fn canonical_owner_keeps_level_appearance_alias_separate() -> Result<()> {
    let identity = ActorBrainFieldIdentity::new("fix:/persos/15", ActorBrainFieldId::dsg(149))?;
    let appearance = AssetPath::new("level:/persos/world/0/1")?;
    let candidate = candidate(
        Some("fix:/persos/15"),
        Some("level:/persos/world/0/1"),
        Some("DSG[149]"),
    );
    let aliases = appearance_aliases(&candidate, &identity)?;
    ensure!(identity.owner.as_str() == "fix:/persos/15");
    ensure!(aliases == [appearance]);
    Ok(())
}

#[test]
fn ambiguous_actor_owners_are_rejected_without_selecting_a_candidate() -> Result<()> {
    let identity =
        ActorBrainFieldIdentity::new("level:/persos/world/0/174", ActorBrainFieldId::dsg(1))?;
    let join = SaveAllocationJoin {
        id: "reader-only-record".into(),
        status: "multiple_bounded_allocations",
        candidates: vec![
            candidate(Some("level:/persos/world/0/174"), None, Some("DSG[1]")),
            candidate(
                Some("fix:/persos/15"),
                Some("level:/persos/world/0/1"),
                Some("DSG[1]"),
            ),
        ],
        reference_target: None,
    };
    let error =
        select_candidate(&join, &identity).expect_err("multiple source owners must fail closed");
    ensure!(error.to_string().contains("multiple actor Brain owners"));
    Ok(())
}

#[test]
fn descriptor_ambiguity_is_rejected_explicitly() -> Result<()> {
    let identity =
        ActorBrainFieldIdentity::new("level:/persos/world/0/174", ActorBrainFieldId::dsg(1))?;
    let mut ambiguous = candidate(None, None, None);
    ambiguous
        .field_match_ambiguity
        .push("level:/persos/world/0/174 DSG[1]".into());
    let join = SaveAllocationJoin {
        id: "reader-only-record".into(),
        status: "single_bounded_allocation",
        candidates: vec![ambiguous],
        reference_target: None,
    };
    let error = select_candidate(&join, &identity)
        .expect_err("ambiguous source descriptors must fail closed");
    ensure!(error.to_string().contains("ambiguous source ownership"));
    Ok(())
}

#[test]
fn duplicate_semantic_identity_requests_are_rejected() -> Result<()> {
    let identity =
        ActorBrainFieldIdentity::new("level:/persos/world/0/174", ActorBrainFieldId::dsg(1))?;
    let requested = [identity.clone(), identity];
    let error = project_actor_brain_save_fields(
        Path::new("unused"),
        &Selection::hype("manoir", "en")?,
        &[],
        &requested,
    )
    .expect_err("duplicate requested semantic keys must fail");
    ensure!(
        error
            .to_string()
            .contains("duplicate requested actor Brain field")
    );
    Ok(())
}

#[test]
#[ignore = "requires supplied Hype source corpus; selected by --local"]
fn source_brain_fields_join_to_named_owner_and_typed_value() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("disc"));
    let selection = Selection::hype("manoir", "en")?;
    let records = [
        source_bool_record(0, 17, 15_586, 0)?,
        source_bool_record(1, 17, 187_072, 1)?,
        source_bool_record(0, 17, 4_562, 1)?,
    ];
    let identities = [
        ActorBrainFieldIdentity::new("level:/persos/world/0/0", ActorBrainFieldId::dsg(11))?,
        ActorBrainFieldIdentity::new("level:/persos/world/0/174", ActorBrainFieldId::dsg(1))?,
        ActorBrainFieldIdentity::new("fix:/persos/15", ActorBrainFieldId::dsg(147))?,
    ];
    let projection = project_actor_brain_save_fields(&game, &selection, &records, &identities)?;
    ensure!(projection.fields.len() == 3);
    ensure!(projection.fields[&identities[0]].value == DesignerValue::Boolean(false));
    ensure!(projection.fields[&identities[1]].value == DesignerValue::Boolean(true));
    ensure!(projection.fields[&identities[2]].value == DesignerValue::Boolean(true));
    ensure!(
        projection.fields[&identities[0]]
            .appearance_aliases
            .is_empty()
    );
    ensure!(
        projection.fields[&identities[1]]
            .appearance_aliases
            .is_empty()
    );
    ensure!(
        projection.fields[&identities[2]].appearance_aliases
            == [AssetPath::new("level:/persos/world/0/1")?]
    );
    let json = serde_json::to_string(&godot_payload(&projection)?)?;
    ensure!(json.contains("level:/persos/world/0/0"));
    ensure!(json.contains("level:/persos/world/0/174"));
    ensure!(json.contains("fix:/persos/15"));
    ensure!(json.contains("level:/persos/world/0/1"));
    ensure!(json.contains("dsg_11") && json.contains("dsg_1") && json.contains("dsg_147"));
    for forbidden in [
        "0:17:15586",
        "1:17:187072",
        "0:17:4562",
        "address",
        "offset",
        "relocation",
    ] {
        ensure!(
            !json.contains(forbidden),
            "source field payload leaked {forbidden}"
        );
    }
    let identities_from_projection = projection.fields.keys().cloned().collect::<BTreeSet<_>>();
    ensure!(identities_from_projection == identities.iter().cloned().collect());

    let invalid_record = source_bool_record(0, 17, 4_562, 2)?;
    let invalid = project_actor_brain_save_fields_optional(
        &game,
        &selection,
        &[invalid_record],
        &[identities[2].clone()],
    )
    .expect_err("invalid Boolean bytes must not be treated as an absent optional field");
    ensure!(invalid.to_string().contains("invalid Boolean value"));
    Ok(())
}

#[test]
#[ignore = "requires supplied Hype source corpus; selected by --local"]
fn optional_brain_projection_reports_only_a_missing_registered_record() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("disc"));
    let selection = Selection::hype("manoir", "en")?;
    let records = [source_bool_record(0, 17, 15_586, 0)?];
    let hype = ActorBrainFieldIdentity::new("level:/persos/world/0/0", ActorBrainFieldId::dsg(11))?;
    let global = ActorBrainFieldIdentity::new("fix:/persos/15", ActorBrainFieldId::dsg(147))?;
    let requested = [hype.clone(), global.clone()];

    let optional =
        project_actor_brain_save_fields_optional(&game, &selection, &records, &requested)?;
    ensure!(optional.projection.fields.len() == 1);
    ensure!(optional.projection.fields[&hype].value == DesignerValue::Boolean(false));
    ensure!(optional.unresolved.as_slice() == std::slice::from_ref(&global));

    let strict = project_actor_brain_save_fields(&game, &selection, &records, &requested)
        .expect_err("strict projection must reject an unresolved requested field");
    ensure!(
        strict
            .to_string()
            .contains("no unique supported saved source field")
    );
    Ok(())
}
