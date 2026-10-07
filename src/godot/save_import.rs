//! Convert source-backed Hype save values into the small native Godot save API.
//!
//! The source table and its unmapped bytes stay in the import report. Runtime
//! Godot scripts see only supported gameplay properties, never SAI addresses.

use crate::openspace::save_game::{SaiRecord, SaveSlot};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::path::Path;

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

fn source_translation(save: &SaveSlot) -> Result<[f32; 3]> {
    let record = player_record(save, "2:6:1463692")?;
    ensure!(
        record.header.data_type == 0x10 && record.value_bytes.len() == 36,
        "English PC Hype position record has an unexpected value type"
    );
    // The saved value is a 36-byte prefix of the source POS matrix. The
    // translation floats are at bytes 4, 8 and 12; retain the rest verbatim in
    // source_save until the matrix restoration contract is fully mapped.
    let mut translation = [0.0f32; 3];
    for (index, value) in translation.iter_mut().enumerate() {
        let start = 4 + index * 4;
        let bytes: [u8; 4] = record.value_bytes[start..start + 4]
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

fn saved_bool(save: &SaveSlot, identity: &str, kind: u8) -> Result<bool> {
    let record = player_record(save, identity)?;
    let value = unsigned(record, identity, kind, 1)?;
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => anyhow::bail!("English PC save record {identity} is not Boolean"),
    }
}

fn godot_state(save: &SaveSlot) -> Result<Option<Value>> {
    if save.current_level != "manoir" {
        return Ok(None);
    }

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
    let translation = source_translation(save)?;

    // Inventory main-list +10 stores signed instantiated item numbers. Map
    // only the item handles used by the current Godot manor slice; every
    // unselected/unmapped record remains in the attached source table.
    let mut quantities = serde_json::Map::new();
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

    let magick = json!({
        "current_points": current_mana,
        "maximum_points": maximum_mana,
        "selected_spell_code": selected_spell_code,
    });
    let source_translation = json!([translation[0], translation[1], translation[2]]);

    let lever = saved_bool(save, "0:17:297732", 0x10)?;
    let mut warnings = Vec::new();
    if quantities.len() != 5 {
        warnings.push(
            "One or more supported source item numbers use special negative values; their raw records remain preserved.".to_owned(),
        );
    }

    Ok(Some(json!({
        "schema": 1,
        "level": "manoir",
        "actors": {
            "world/0/0/Hype": {
                "magick": magick,
                "inventory": {"quantities": quantities},
                "source_translation": source_translation,
            }
        },
        "world": {
            "level:/persos/world/0/27": {"dsg_8": lever}
        },
        "warnings": warnings,
    })))
}

/// Read and convert one extracted original save slot into an import JSON file
/// consumed by the Godot Load Game path.
pub fn import_original_slot(input: &Path, output: &Path, profile: &str) -> Result<Value> {
    ensure!(
        profile == HYPE_PC_ENGLISH_PROFILE,
        "unsupported save import profile; select {HYPE_PC_ENGLISH_PROFILE} explicitly"
    );
    let save = crate::openspace::save_game::read_slot(input)?;
    let state = godot_state(&save)?;
    let has_godot_state = state.is_some();
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
        "restored_fields": [
            "Hype saved translation",
            "Hype current and maximum mana",
            "Hype selected magic slot",
            "supported manor inventory item numbers",
            "iDummy_PourCercles.DSG[8] manor lever state",
        ],
        "not_restored": [
            "Hype saved orientation and current AI behavior state",
            "active weapon selection",
            "bridge-gate pose or animation; the mapped gate records cover active/HP state only",
            "other source records without a supported Godot field mapping",
        ],
        "source_save": save,
    });
    if !has_godot_state {
        report["godot_state"] = Value::Null;
    }
    let bytes = serde_json::to_vec_pretty(&report)?;
    crate::files::write_atomic(output, &bytes)?;
    Ok(report)
}
