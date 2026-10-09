//! Convert source-backed Hype save values into the small native Godot save API.
//!
//! Source records and unmapped bytes stay in a diagnostic sibling artifact.
//! Runtime Godot scripts see only supported gameplay properties, never SAI addresses.

use crate::model::{Selection, ai::DesignerValue};
use crate::openspace::{
    level::{
        SavedInventoryItem,
        brain_save::{
            ActorBrainFieldId, ActorBrainFieldIdentity, OptionalActorBrainSaveProjection,
            project_actor_brain_save_fields_optional,
        },
        read_saved_inventory_item_zero,
    },
    save_game::{SaiRecord, SaveSlot},
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub const HYPE_PC_ENGLISH_PROFILE: &str = "hype-pc-english-supplied-save";

fn player_record<'a>(save: &'a SaveSlot, identity: &str) -> Result<&'a SaiRecord> {
    let table = save
        .members
        .get("Level000.sav")
        .context("save is missing its player record file")?
        .records
        .as_ref()
        .context("Level000.sav has no decoded SAI records")?;
    let mut matches = table
        .records
        .iter()
        .filter(|record| record.identity() == identity);
    let record = matches
        .next()
        .with_context(|| format!("English PC save is missing supported record {identity}"))?;
    ensure!(
        matches.next().is_none(),
        "English PC save has duplicate supported record {identity}"
    );
    ensure!(
        record.header.save_flags == 0x0204,
        "English PC save record {identity} has unexpected save flags"
    );
    Ok(record)
}

fn unsigned(record: &SaiRecord, identity: &str, kind: u8, width: usize) -> Result<u32> {
    ensure!(
        record.header.data_type == kind && record.value_bytes.len() == width,
        "English PC save record {identity} has an unexpected value type"
    );
    ensure!(width <= 4, "unsupported save integer width {width}");
    let mut bytes = [0u8; 4];
    bytes[..width].copy_from_slice(&record.value_bytes);
    Ok(u32::from_le_bytes(bytes))
}

fn signed_i16(save: &SaveSlot, identity: &str) -> Result<i16> {
    let record = player_record(save, identity)?;
    ensure!(
        record.header.data_type == 0x02 && record.value_bytes.len() == 2,
        "English PC inventory record {identity} is not a 16-bit value"
    );
    Ok(i16::from_le_bytes(
        record.value_bytes[..2]
            .try_into()
            .expect("record width checked"),
    ))
}

fn supported_inventory_state(save: &SaveSlot, item_zero: SavedInventoryItem) -> Result<Value> {
    ensure!(
        item_zero.item_id == 0
            && item_zero.quantity >= 0
            && item_zero.capacity >= 0
            && item_zero.quantity <= item_zero.capacity,
        "OpenSpace reader returned invalid item-0 inventory semantics"
    );
    let mut quantities = serde_json::Map::new();

    // The Fix composition resolves item 0 as the special gold resource, which
    // is not an ordinary catalogue member. Keep its independently owned value
    // out of the ordinary item/admission dictionary, including an explicit zero.

    // Inventory main-list +10 stores signed instantiated item numbers. Map
    // only the item handles used by the current Godot manor slice; every
    // unselected/unmapped record remains in the diagnostic sibling artifact.
    for (item, identity) in [
        ("21", "0:10:4242"),
        ("54", "0:10:1562"),
        ("61", "0:10:1738"),
        ("74", "0:10:2802"),
        ("80", "0:10:214"),
    ] {
        let value = signed_i16(save, identity)?;
        if value >= -1 {
            quantities.insert(item.to_owned(), json!(value));
        }
    }
    Ok(json!({"gold": item_zero.quantity, "quantities": quantities}))
}

fn source_diagnostics_path(output: &Path) -> PathBuf {
    let stem = output
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("original_save_import");
    output.with_file_name(format!("{stem}.source-diagnostics.json"))
}

fn source_position_prefix(save: &SaveSlot) -> Result<&[u8]> {
    let record = player_record(save, "2:6:1463692")?;
    ensure!(
        record.header.data_type == 0x10 && record.value_bytes.len() == 36,
        "English PC Hype position record has an unexpected value type"
    );
    ensure!(
        record.value_bytes[0] > 0x7f,
        "English PC Hype position is not an uncompressed source transform"
    );
    Ok(&record.value_bytes)
}

fn source_translation(bytes: &[u8]) -> Result<[f32; 3]> {
    let mut translation = [0.0f32; 3];
    for (index, value) in translation.iter_mut().enumerate() {
        let start = 4 + index * 4;
        let bytes: [u8; 4] = bytes[start..start + 4]
            .try_into()
            .expect("position record width checked");
        *value = f32::from_le_bytes(bytes);
        ensure!(
            value.is_finite(),
            "English PC Hype position record has a non-finite translation"
        );
    }
    Ok(translation)
}

fn source_rotation(bytes: &[u8]) -> Option<[[f32; 3]; 3]> {
    // Hype registers only the first 0x24 bytes of its 0x58-byte POS value.
    // That prefix contains three translation floats and five of the nine
    // rotation floats. Reconstruct the missing second-row Z only when the
    // orthonormal matrix constraints determine a unique value.
    let float_at = |offset: usize| {
        f32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("checked width"))
    };
    let row0 = [float_at(16), float_at(20), float_at(24)];
    let row1_xy = [float_at(28), float_at(32)];
    if row0
        .iter()
        .chain(row1_xy.iter())
        .any(|value| !value.is_finite())
    {
        return None;
    }
    let row0_len2 = row0.iter().map(|value| value * value).sum::<f32>();
    if (row0_len2 - 1.0).abs() > 0.01 {
        return None;
    }

    let xy_dot = row0[0] * row1_xy[0] + row0[1] * row1_xy[1];
    let row1_z = if row0[2].abs() > 0.001 {
        -xy_dot / row0[2]
    } else if xy_dot.abs() <= 0.001
        && (row1_xy[0] * row1_xy[0] + row1_xy[1] * row1_xy[1] - 1.0).abs() <= 0.001
    {
        0.0
    } else {
        // With a zero Z in row zero, the remaining length permits two signs.
        return None;
    };
    let row1 = [row1_xy[0], row1_xy[1], row1_z];
    let row1_len2 = row1.iter().map(|value| value * value).sum::<f32>();
    if !row1_z.is_finite() || (row1_len2 - 1.0).abs() > 0.01 {
        return None;
    }

    // Remove saved float rounding while preserving its orientation. The last
    // axis follows from a right-handed orthonormal transform.
    let normalize = |vector: [f32; 3]| -> Option<[f32; 3]> {
        let length = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
        (length.is_finite() && length > 0.0)
            .then(|| [vector[0] / length, vector[1] / length, vector[2] / length])
    };
    let row0 = normalize(row0)?;
    let projection = row0[0] * row1[0] + row0[1] * row1[1] + row0[2] * row1[2];
    let row1 = normalize([
        row1[0] - projection * row0[0],
        row1[1] - projection * row0[1],
        row1[2] - projection * row0[2],
    ])?;
    let row2 = normalize([
        row0[1] * row1[2] - row0[2] * row1[1],
        row0[2] * row1[0] - row0[0] * row1[2],
        row0[0] * row1[1] - row0[1] * row1[0],
    ])?;
    Some([row0, row1, row2])
}

fn godot_basis(rows: [[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let [row0, row1, row2] = rows;
    [
        [row0[0], row0[2], -row0[1]],
        [row2[0], row2[2], -row2[1]],
        [-row1[0], -row1[2], row1[1]],
    ]
}

fn saved_bool(save: &SaveSlot, identity: &str, kind: u8) -> Result<bool> {
    let record = player_record(save, identity)?;
    let value = unsigned(record, identity, kind, 1)?;
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => anyhow::bail!("English PC save record {identity} is not Boolean"),
    }
}

fn global_intro_latch_from_projection(
    projection: &OptionalActorBrainSaveProjection,
    identity: &ActorBrainFieldIdentity,
) -> Result<Option<bool>> {
    let value = projection.projection.fields.get(identity);
    let unresolved = projection
        .unresolved
        .iter()
        .filter(|candidate| *candidate == identity)
        .count();
    ensure!(
        unresolved <= 1 && (value.is_some() ^ (unresolved == 1)),
        "optional Global Brain projection is inconsistent for {}",
        identity.owner
    );
    if unresolved == 1 {
        return Ok(None);
    }
    match &value.expect("projection consistency checked").value {
        DesignerValue::Boolean(value) => Ok(Some(*value)),
        _ => anyhow::bail!("Global Brain dsg_147 projection is not Boolean"),
    }
}

fn global_intro_latch(game: &Path, save: &SaveSlot) -> Result<Option<bool>> {
    let records = save
        .members
        .get("Level000.sav")
        .context("save is missing its player record file")?
        .records
        .as_ref()
        .context("Level000.sav has no decoded SAI records")?;
    let identity = ActorBrainFieldIdentity::new("fix:/persos/15", ActorBrainFieldId::dsg(147))?;
    let projection = project_actor_brain_save_fields_optional(
        game,
        &Selection::hype("manoir", "en")?,
        &records.records,
        std::slice::from_ref(&identity),
    )?;
    global_intro_latch_from_projection(&projection, &identity)
}

fn global_brain_state(gate_sequence: bool, intro_latch: Option<bool>) -> Value {
    let mut brain = json!({"dsg_149": gate_sequence});
    if let Some(value) = intro_latch {
        brain["dsg_147"] = json!(value);
    }
    brain
}

fn godot_state(
    game: &Path,
    save: &SaveSlot,
    item_zero: Option<SavedInventoryItem>,
) -> Result<Option<(Value, Vec<String>)>> {
    if save.current_level != "manoir" {
        return Ok(None);
    }
    let item_zero = item_zero.context("manoir save has no source-composed item-0 value")?;

    // These Hype PC English-build identities have source ownership and reload
    // evidence recorded in docs/openspace/save-games.md. Keep the profile
    // explicit: the save itself does not identify an executable build.
    let current_mana_record = player_record(save, "0:5:4986212")?;
    let maximum_mana_record = player_record(save, "0:5:4986214")?;
    let current_mana = unsigned(current_mana_record, "0:5:4986212", 0x02, 2)?;
    let maximum_mana = unsigned(maximum_mana_record, "0:5:4986214", 0x02, 2)?;
    let selected_slot =
        unsigned(player_record(save, "0:5:4986216")?, "0:5:4986216", 0x02, 2)? & 0xff;
    let selected_spell_code = match selected_slot {
        0 => 61,
        1 => 54,
        2 => 74,
        _ => anyhow::bail!("English PC save has an invalid selected magic slot"),
    };
    ensure!(
        current_mana <= maximum_mana && maximum_mana <= i32::MAX as u32,
        "English PC save has invalid mana values"
    );
    let source_position = source_position_prefix(save)?;
    let translation = source_translation(source_position)?;
    let rotation = source_rotation(source_position);
    let global_gate_sequence = saved_bool(save, "0:17:4560", 0x10)?;
    let global_intro_latch = global_intro_latch(game, save)?;

    let inventory = supported_inventory_state(save, item_zero)?;

    let magick = json!({
        "current_points": current_mana,
        "maximum_points": maximum_mana,
        "selected_spell_code": selected_spell_code,
    });
    let position = json!([translation[0], translation[2], -translation[1]]);

    let lever = saved_bool(save, "0:17:297732", 0x10)?;
    let mut warnings = Vec::new();
    if global_intro_latch.is_none() {
        warnings.push(
            "No saved semantic value resolved for optional Global Brain dsg_147; the field was omitted so native restore preserves the scene-defined initial value.".to_owned(),
        );
    }
    if inventory["quantities"]
        .as_object()
        .map_or(0, |value| value.len())
        != 5
    {
        warnings.push(
            "One or more supported source item numbers use special negative values; their raw records remain preserved.".to_owned(),
        );
    }

    let mut state = json!({
        "schema": 1,
        "level": "manoir",
        "actors": {
            "world/0/0/Hype": {
                "magick": magick,
                "inventory": inventory,
                "transform": {"position": position},
            },
            "fix:/persos/15": {
                "brain": global_brain_state(global_gate_sequence, global_intro_latch),
            }
        },
        "world": {
            "level:/persos/world/0/27": {"dsg_8": lever}
        },
    });
    if let Some(rotation) = rotation {
        // Godot Basis stores its three local axes as columns. Convert each
        // source row vector into Godot coordinates and order it like the
        // measured native fixture's transform conversion.
        state["actors"]["world/0/0/Hype"]["transform"]["basis"] = json!(godot_basis(rotation));
    } else {
        warnings.push(
            "Hype orientation was omitted because the saved matrix prefix does not uniquely determine it.".to_owned(),
        );
    }
    Ok(Some((state, warnings)))
}

/// Read and convert one extracted original save slot into an import JSON file
/// consumed by the Godot Load Game path.
pub fn import_original_slot(
    input: &Path,
    game: &Path,
    output: &Path,
    profile: &str,
) -> Result<Value> {
    ensure!(
        profile == HYPE_PC_ENGLISH_PROFILE,
        "unsupported save import profile; select {HYPE_PC_ENGLISH_PROFILE} explicitly"
    );
    let save = crate::openspace::save_game::read_slot(input)?;
    let item_zero = if save.current_level == "manoir" {
        Some(read_saved_inventory_item_zero(game, &save)?)
    } else {
        None
    };
    let state = godot_state(game, &save, item_zero)?;
    let has_godot_state = state.is_some();
    let (state, warnings) = state
        .map(|(state, warnings)| (Some(state), warnings))
        .unwrap_or((None, Vec::new()));
    let mut restored_fields = vec![
        "Hype saved position and uniquely recoverable orientation",
        "Hype current and maximum mana",
        "Hype selected magic slot",
        "reserved item-0 resource balance and supported manor inventory item numbers",
        "iDummy_PourCercles.DSG[8] manor lever state",
        "fix:/persos/15 Global.DSG[149], read by manor gate AI model 272",
    ];
    if state.as_ref().is_some_and(|state| {
        state["actors"]["fix:/persos/15"]["brain"]
            .get("dsg_147")
            .is_some()
    }) {
        restored_fields.push("fix:/persos/15 Brain.dsg_147 source transition latch");
    }
    let mut report = json!({
        "schema": 1,
        "format": "astrolabe-godot-original-save-import",
        "profile": profile,
        "profile_provenance": "caller_selected_not_detected_from_save",
        "slot": save.slot,
        "current_level": save.current_level,
        "display_name": save.display_name,
        "game_timer_ms": save.game_timer_ms,
        "godot_state": state,
        "semantic_inventory_items": item_zero.map_or_else(Vec::new, |item| {
            vec![json!({
                "component": "inventory_item",
                "item_id": item.item_id,
                "quantity": item.quantity,
                "capacity": item.capacity,
            })]
        }),
        "warnings": warnings,
        "restored_fields": restored_fields,
        "not_restored": [
            "Hype current AI behavior state",
            "Hype default magic-point amount; the native magick API has no separate default-points field",
            "Hype health and armor; no Hype-owned save identity is mapped to the native hit-point resource",
            "active weapon selection; the fixture's selected-item flags do not identify one active weapon",
            "other source records without a supported Godot field mapping",
        ],
    });
    if !has_godot_state {
        report["godot_state"] = Value::Null;
    }
    let bytes = serde_json::to_vec_pretty(&report)?;
    crate::files::write_atomic(output, &bytes)?;
    let diagnostics = serde_json::to_vec_pretty(&json!({
        "schema": 1,
        "format": "astrolabe-openspace-save-source-diagnostics",
        "profile": profile,
        "source_save": save,
    }))?;
    crate::files::write_atomic(&source_diagnostics_path(output), &diagnostics)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::{
        global_brain_state, global_intro_latch_from_projection, godot_basis, source_rotation,
        supported_inventory_state,
    };
    use crate::model::ai::DesignerValue;
    use crate::openspace::level::SavedInventoryItem;
    use crate::openspace::level::brain_save::{
        ActorBrainFieldId, ActorBrainFieldIdentity, ActorBrainFieldValue, ActorBrainSaveProjection,
        OptionalActorBrainSaveProjection,
    };
    use crate::openspace::save_game::{SaveMember, SaveSlot, parse_records};
    use std::collections::BTreeMap;

    fn synthetic_save(records: &[(u32, i16)]) -> SaveSlot {
        let mut bytes = Vec::new();
        for (offset, value) in records {
            bytes.extend_from_slice(&[0, 10]);
            bytes.extend_from_slice(&offset.to_le_bytes());
            bytes.extend_from_slice(&0x0204u16.to_le_bytes());
            bytes.extend_from_slice(&[0, 0x02]);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        let table = parse_records(&bytes).expect("synthetic SAI records parse");
        let member = SaveMember {
            name: "Level000.sav".to_owned(),
            role: "player".to_owned(),
            encoded_size: bytes.len(),
            sha256: String::new(),
            decoded_size: Some(bytes.len()),
            decoded_sha256: None,
            raw_hex: None,
            u32_values: None,
            text_lines: None,
            records: Some(table),
        };
        SaveSlot {
            schema: 1,
            format: "hype-pc-save".to_owned(),
            slot: "SaveGame0".to_owned(),
            current_level: "manoir".to_owned(),
            display_name: String::new(),
            game_timer_ms: 0,
            members: BTreeMap::from([("Level000.sav".to_owned(), member)]),
        }
    }

    #[test]
    fn reconstructs_source_rotation_from_five_saved_floats() {
        let mut bytes = [0u8; 36];
        for (offset, value) in [
            (16, -0.999_995_77_f32),
            (20, 0.002_917_85),
            (24, 0.0),
            (28, -0.002_917_85),
            (32, -0.999_995_77),
        ] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        let basis = source_rotation(&bytes).expect("rotation is determined");
        assert!((basis[0][0] + 1.0).abs() < 0.000_01);
        assert!((basis[1][1] + 1.0).abs() < 0.000_01);
        assert!((basis[2][2] - 1.0).abs() < 0.000_01);
        assert!(basis[2][0].abs() < 0.000_01);
        assert!(basis[2][1].abs() < 0.000_01);
        let godot = godot_basis(basis);
        assert!((godot[0][0] + 1.0).abs() < 0.000_01);
        assert!((godot[1][1] - 1.0).abs() < 0.000_01);
        assert!((godot[2][2] + 1.0).abs() < 0.000_01);
    }

    #[test]
    fn omits_source_rotation_when_missing_component_is_ambiguous() {
        let mut bytes = [0u8; 36];
        for (offset, value) in [(16, 1.0_f32), (20, 0.0), (24, 0.0), (28, 0.0), (32, 0.8)] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        assert!(source_rotation(&bytes).is_none());
    }

    #[test]
    fn item_zero_balance_preserves_an_explicit_zero() {
        let save = synthetic_save(&[(4242, -1), (1562, -1), (1738, -1), (2802, -1), (214, 8)]);
        let item_zero = SavedInventoryItem {
            item_id: 0,
            quantity: 0,
            capacity: 200,
        };
        let inventory = supported_inventory_state(&save, item_zero).expect("supported inventory");
        assert_eq!(inventory["gold"], 0);
        assert!(inventory["quantities"].get("0").is_none());
        assert_eq!(inventory["quantities"]["80"], 8);
        assert_eq!(inventory["quantities"].as_object().unwrap().len(), 5);
    }

    #[test]
    fn item_zero_gameplay_payload_is_address_free_and_stable() {
        let save = synthetic_save(&[(4242, -1), (1562, -1), (1738, -1), (2802, -1), (214, 8)]);
        let original = SavedInventoryItem {
            item_id: 0,
            quantity: 20,
            capacity: 200,
        };
        let relocated = SavedInventoryItem {
            item_id: 0,
            quantity: 20,
            capacity: 200,
        };
        let original_state = supported_inventory_state(&save, original).unwrap();
        let relocated_state = supported_inventory_state(&save, relocated).unwrap();
        assert_eq!(original_state, relocated_state);
        assert_eq!(relocated_state["gold"], 20);
        let payload = serde_json::json!({"inventory":relocated_state});
        let serialized = serde_json::to_string(&payload).unwrap();
        for forbidden in ["offset", "block", "module", "relocation", "5790", "5828"] {
            assert!(!serialized.contains(forbidden));
        }
    }

    fn test_brain_identity() -> ActorBrainFieldIdentity {
        ActorBrainFieldIdentity::new("fix:/persos/15", ActorBrainFieldId::dsg(147)).unwrap()
    }

    fn test_projection(
        identity: &ActorBrainFieldIdentity,
        value: Option<DesignerValue>,
    ) -> OptionalActorBrainSaveProjection {
        let mut fields = BTreeMap::new();
        let mut unresolved = Vec::new();
        if let Some(value) = value {
            fields.insert(
                identity.clone(),
                ActorBrainFieldValue {
                    value,
                    appearance_aliases: Vec::new(),
                },
            );
        } else {
            unresolved.push(identity.clone());
        }
        OptionalActorBrainSaveProjection {
            projection: ActorBrainSaveProjection { fields },
            unresolved,
        }
    }

    #[test]
    fn optional_global_brain_field_preserves_boolean_and_absence_semantics() {
        let identity = test_brain_identity();
        for value in [false, true] {
            let projection = test_projection(&identity, Some(DesignerValue::Boolean(value)));
            let intro_latch = global_intro_latch_from_projection(&projection, &identity).unwrap();
            assert_eq!(intro_latch, Some(value));
            let brain = global_brain_state(true, intro_latch);
            assert_eq!(brain["dsg_149"], true);
            assert_eq!(brain["dsg_147"], value);
        }

        let absent = test_projection(&identity, None);
        assert_eq!(
            global_intro_latch_from_projection(&absent, &identity).unwrap(),
            None
        );
        let brain = global_brain_state(true, None);
        assert_eq!(brain["dsg_149"], true);
        assert!(brain.get("dsg_147").is_none());
        let payload = serde_json::to_string(&serde_json::json!({
            "actors": {"fix:/persos/15": {"brain": brain}},
        }))
        .unwrap();
        for private_source_detail in [
            "4562",
            "offset",
            "block",
            "module",
            "allocation",
            "registration",
        ] {
            assert!(!payload.contains(private_source_detail));
        }
    }

    #[test]
    fn optional_global_brain_field_adapter_rejects_wrong_semantics_and_inconsistent_absence() {
        let identity = test_brain_identity();
        let wrong_type = test_projection(&identity, Some(DesignerValue::Integer(1)));
        assert!(global_intro_latch_from_projection(&wrong_type, &identity).is_err());

        let mut inconsistent = test_projection(&identity, None);
        inconsistent.projection.fields.insert(
            identity.clone(),
            ActorBrainFieldValue {
                value: DesignerValue::Boolean(false),
                appearance_aliases: Vec::new(),
            },
        );
        assert!(global_intro_latch_from_projection(&inconsistent, &identity).is_err());
    }
}
