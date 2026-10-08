//! Read-only research decoder for the English Hype PC save files.
//! This is Orrery evidence tooling, not an Astrolabe content adapter or editor.

use anyhow::{Context, Result, bail, ensure};
use astrolabe::{
    model::Selection,
    openspace::level::{SaveRecordLocation, join_save_allocations},
};
use flate2::read::DeflateDecoder;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Component, Path},
};

const MAX_MEMBER: usize = 64 * 1024 * 1024;

fn u16_at(bytes: &[u8], at: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        bytes
            .get(at..at + 2)
            .context("truncated 16-bit field")?
            .try_into()?,
    ))
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        bytes
            .get(at..at + 4)
            .context("truncated 32-bit field")?
            .try_into()?,
    ))
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|value| format!("{value:02x}")).collect()
}

fn zip_members(path: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    let bytes = fs::read(path)?;
    let start = bytes.len().saturating_sub(65_557);
    let end = (start..bytes.len().saturating_sub(3))
        .rev()
        .find(|&at| bytes[at..at + 4] == [0x50, 0x4b, 0x05, 0x06])
        .context("ZIP end-of-central-directory not found")?;
    ensure!(
        u16_at(&bytes, end + 4)? == 0 && u16_at(&bytes, end + 6)? == 0,
        "multi-disk ZIP is unsupported"
    );
    let count = usize::from(u16_at(&bytes, end + 10)?);
    let central_size = u32_at(&bytes, end + 12)? as usize;
    let mut at = u32_at(&bytes, end + 16)? as usize;
    ensure!(
        at.checked_add(central_size).is_some_and(|last| last <= end),
        "ZIP central directory is outside file"
    );
    let mut result = BTreeMap::new();
    for _ in 0..count {
        ensure!(
            u32_at(&bytes, at)? == 0x0201_4b50,
            "invalid ZIP directory entry"
        );
        let method = u16_at(&bytes, at + 10)?;
        let compressed_size = u32_at(&bytes, at + 20)? as usize;
        let uncompressed_size = u32_at(&bytes, at + 24)? as usize;
        ensure!(
            uncompressed_size <= MAX_MEMBER,
            "ZIP member exceeds research limit"
        );
        let name_len = usize::from(u16_at(&bytes, at + 28)?);
        let extra_len = usize::from(u16_at(&bytes, at + 30)?);
        let comment_len = usize::from(u16_at(&bytes, at + 32)?);
        let local = u32_at(&bytes, at + 42)? as usize;
        let name = std::str::from_utf8(
            bytes
                .get(at + 46..at + 46 + name_len)
                .context("truncated ZIP name")?,
        )?;
        at = at
            .checked_add(46 + name_len + extra_len + comment_len)
            .context("ZIP directory offset overflow")?;
        if !name.to_ascii_lowercase().ends_with(".sav") {
            continue;
        }
        ensure!(
            u32_at(&bytes, local)? == 0x0403_4b50,
            "invalid ZIP local header"
        );
        let local_name_len = usize::from(u16_at(&bytes, local + 26)?);
        let local_extra_len = usize::from(u16_at(&bytes, local + 28)?);
        let data_at = local
            .checked_add(30 + local_name_len + local_extra_len)
            .context("ZIP member offset overflow")?;
        let data_end = data_at
            .checked_add(compressed_size)
            .context("ZIP member end offset overflow")?;
        let compressed = bytes
            .get(data_at..data_end)
            .context("truncated ZIP member")?;
        let data = match method {
            0 => compressed.to_vec(),
            8 => {
                let mut decoder = DeflateDecoder::new(compressed);
                let mut output = Vec::new();
                decoder
                    .by_ref()
                    .take(MAX_MEMBER as u64 + 1)
                    .read_to_end(&mut output)?;
                output
            }
            _ => bail!("unsupported ZIP compression method {method}"),
        };
        ensure!(
            data.len() == uncompressed_size,
            "ZIP member size mismatch: {name}"
        );
        let member = Path::new(name);
        ensure!(
            member
                .components()
                .all(|part| matches!(part, Component::Normal(_)))
                && !name.contains('\\')
                && name
                    .split('/')
                    .all(|part| !part.is_empty() && part != "." && part != ".."),
            "unsafe ZIP save path {name}"
        );
        ensure!(
            bytes.get(local + 30..local + 30 + local_name_len) == Some(name.as_bytes()),
            "ZIP local and central names disagree: {name}"
        );
        ensure!(
            result.insert(name.to_owned(), data).is_none(),
            "duplicate ZIP save {name}"
        );
    }
    ensure!(!result.is_empty(), "no .sav members in ZIP");
    Ok(result)
}

pub fn members(path: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    if path.is_dir() {
        let mut result = BTreeMap::new();
        let mut pending = vec![path.to_path_buf()];
        while let Some(directory) = pending.pop() {
            for entry in fs::read_dir(directory)? {
                let entry = entry?;
                let kind = entry.file_type()?;
                if kind.is_dir() {
                    pending.push(entry.path());
                } else if kind.is_file()
                    && entry
                        .path()
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("sav"))
                {
                    let relative = entry
                        .path()
                        .strip_prefix(path)?
                        .to_string_lossy()
                        .into_owned();
                    ensure!(
                        result
                            .insert(relative.clone(), fs::read(entry.path())?)
                            .is_none(),
                        "duplicate save {relative}"
                    );
                }
            }
        }
        ensure!(!result.is_empty(), "no .sav files in {}", path.display());
        Ok(result)
    } else if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
    {
        zip_members(path)
    } else if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("sav"))
    {
        Ok(BTreeMap::from([(
            path.file_name()
                .context("missing save filename")?
                .to_string_lossy()
                .into_owned(),
            fs::read(path)?,
        )]))
    } else {
        bail!(
            "expected a save directory, .zip, or .sav file: {}",
            path.display()
        )
    }
}

pub fn decode(source: &[u8]) -> Result<Vec<u8>> {
    astrolabe::openspace::save_game::decode(source)
}

fn record_value(kind: u8, bytes: &[u8]) -> Result<Value> {
    Ok(match kind {
        1 | 2 | 4 | 8 => {
            let mut padded = [0u8; 8];
            padded[..bytes.len()].copy_from_slice(bytes);
            json!(u64::from_le_bytes(padded))
        }
        0x11 => json!({
            "block": bytes[0],
            "module": bytes[1],
            "offset": u32_at(bytes, 2)?,
        }),
        _ => json!(hex(bytes)),
    })
}

struct RecordScan {
    records: BTreeMap<String, Value>,
    undecoded_tail: Option<Value>,
}

fn scan_records(bytes: &[u8]) -> Result<RecordScan> {
    let parsed = astrolabe::openspace::save_game::parse_records(bytes)?;
    let mut result = BTreeMap::new();
    for record in parsed.records {
        let id = record.identity();
        let kind = record.header.data_type;
        let flags = record.header.save_flags;
        let entry = json!({
            "position": record.position,
            "flags": format!("0x{flags:04x}"),
            "reserved": record.header.reserved,
            "kind": format!("0x{kind:02x}"),
            "value_interpretation": if matches!(kind, 1 | 2 | 4 | 8) {"unsigned_le_display_only"} else {"raw_or_reference"},
            "value": record_value(kind, &record.value_bytes)?,
            "value_hex": record.value_hex,
        });
        ensure!(
            result.insert(id.clone(), entry).is_none(),
            "duplicate save record {id}"
        );
    }
    Ok(RecordScan {
        records: result,
        undecoded_tail: parsed.unknown_tail.map(|tail| {
            json!({
                "position": tail.position,
                "reason": tail.reason,
                "hex": tail.value_hex,
            })
        }),
    })
}

pub fn records(bytes: &[u8]) -> Result<BTreeMap<String, Value>> {
    let scan = scan_records(bytes)?;
    ensure!(scan.undecoded_tail.is_none(), "unknown save record type");
    Ok(scan.records)
}

const ENGLISH_PC_PROFILE: &str = "hype-pc-english-supplied-save";

pub fn inspect_with_profile(path: &Path, profile: Option<&str>) -> Result<Value> {
    ensure!(
        profile.is_none_or(|name| name == ENGLISH_PC_PROFILE),
        "unsupported save semantic profile; supported: {ENGLISH_PC_PROFILE}"
    );
    let mut files = BTreeMap::new();
    for (name, source) in members(path)? {
        let mut entry = json!({
            "encoded_size": source.len(),
            "sha256": sha256(&source),
        });
        let filename = Path::new(&name)
            .file_name()
            .and_then(|value| value.to_str());
        if filename == Some("General.sav") {
            if source.len() == 8 {
                entry["u32_values"] = json!([u32_at(&source, 0)?, u32_at(&source, 4)?]);
            }
        } else {
            let data = decode(&source).with_context(|| format!("decoding {name}"))?;
            entry["decoded_size"] = json!(data.len());
            entry["decoded_sha256"] = json!(sha256(&data));
            if filename.is_some_and(|value| value.starts_with("General")) {
                let lines = String::from_utf8_lossy(&data)
                    .lines()
                    .map(str::to_owned)
                    .collect::<Vec<_>>();
                entry["text_lines"] = json!(lines);
            } else {
                let mut scan = scan_records(&data).with_context(|| format!("parsing {name}"))?;
                if profile.is_some() && filename == Some("Level000.sav") {
                    for (id, field) in [
                        ("0:5:4986212", "current_points"),
                        ("0:5:4986214", "maximum_points"),
                    ] {
                        let record = scan.records.get_mut(id).with_context(|| {
                            format!("{name}: missing known mana record {id}; profile mismatch")
                        })?;
                        ensure!(
                            record["kind"] == "0x02" && record["flags"] == "0x0204",
                            "{name}: mana record {id} differs from selected profile"
                        );
                        record["semantic"] = json!({
                            "owner": "Hype magick",
                            "field": field,
                            "unit": "points",
                            "evidence": if field == "current_points" {
                                "controlled druidik and thereyougo changes, game-written records and isolated fresh-process reloads on 2026-09-27"
                            } else {
                                "controlled thereyougo change, game-written record and isolated fresh-process reload on 2026-09-27"
                            },
                            "confidence": "fresh_process_reload_demonstrated",
                        });
                    }
                    for (id, kind, code) in [
                        ("0:10:1560", "MagicIce", 54),
                        ("0:10:2800", "MagicLightning", 74),
                    ] {
                        let record = scan.records.get_mut(id).with_context(|| {
                            format!("{name}: missing known magic selected-state record {id}; profile mismatch")
                        })?;
                        ensure!(
                            record["kind"] == "0x01"
                                && record["flags"] == "0x0204"
                                && matches!(record["value_hex"].as_str(), Some("00" | "01")),
                            "{name}: magic selected-state record {id} differs from selected profile"
                        );
                        record["semantic"] = json!({
                            "owner": format!("Fix inventory {kind} {code}"),
                            "field": "selected",
                            "type": "Boolean",
                            "evidence": "inventory handle +8 selected byte corroborated by Hype's INV_fn_bIsSelected, physical thereyougo input, normal game save and isolated fresh-process reload on 2026-09-27",
                            "confidence": "fresh_process_reload_demonstrated",
                        });
                    }
                    let id = "0:17:297732";
                    let record = scan.records.get_mut(id).with_context(|| {
                        format!("{name}: missing known manor unlock record {id}; profile mismatch")
                    })?;
                    ensure!(
                        record["kind"] == "0x10"
                            && record["flags"] == "0x0204"
                            && matches!(record["value_hex"].as_str(), Some("00" | "01")),
                        "{name}: manor unlock record {id} differs from selected profile"
                    );
                    record["semantic"] = json!({
                        "owner": "level:/persos/world/0/27 (iDummy_PourCercles)",
                        "field": "DSG[8]",
                        "type": "Boolean",
                        "meaning": "secret manor lever teleport unlock",
                        "evidence": "source designer field address/save flag, guarded lever interaction, normal game save and isolated fresh-process reload on 2026-09-27",
                        "confidence": "fresh_process_reload_demonstrated",
                    });
                }
                let mut kinds = BTreeMap::<String, usize>::new();
                let mut flags = BTreeMap::<String, usize>::new();
                for record in scan.records.values() {
                    *kinds
                        .entry(record["kind"].as_str().unwrap().to_owned())
                        .or_default() += 1;
                    *flags
                        .entry(record["flags"].as_str().unwrap().to_owned())
                        .or_default() += 1;
                }
                entry["record_count"] = json!(scan.records.len());
                entry["types"] = json!(kinds);
                entry["flags"] = json!(flags);
                entry["records"] = json!(scan.records);
                if let Some(tail) = scan.undecoded_tail {
                    entry["undecoded_tail"] = tail;
                }
            }
        }
        files.insert(name, entry);
    }
    Ok(json!({
        "schema": 2,
        "format": "hype_pc_save_research",
        "source": path,
        "semantic_profile": profile,
        "semantic_profile_provenance": if profile.is_some() {"caller_selected_not_detected_from_save"} else {"none"},
        "files": files,
    }))
}

pub fn inspect(path: &Path) -> Result<Value> {
    inspect_with_profile(path, None)
}

/// Join decoded record addresses to bounded source allocations for one level.
/// This does not assign semantic field names or persist runtime addresses.
pub fn source_allocation_report(save: &Path, game: &Path, level: &str) -> Result<Value> {
    let inspection = inspect_with_profile(save, Some(ENGLISH_PC_PROFILE))?;
    let mut locations = BTreeMap::<String, SaveRecordLocation>::new();
    let mut conflicting_occurrences = BTreeMap::<String, Vec<String>>::new();
    for (file, content) in inspection["files"]
        .as_object()
        .context("save inspection files are missing")?
    {
        let Some(records) = content["records"].as_object() else {
            continue;
        };
        for (id, record) in records {
            let components = id
                .split(':')
                .map(str::parse::<u32>)
                .collect::<std::result::Result<Vec<_>, _>>()?;
            ensure!(components.len() == 3, "invalid save record identity {id}");
            let block = u8::try_from(components[0])?;
            let module = u8::try_from(components[1])?;
            let width = if record["kind"] == "0x11" {
                4
            } else {
                record["value_hex"]
                    .as_str()
                    .context("save record has no value bytes")?
                    .len()
                    / 2
            };
            let reference_target = if record["kind"] == "0x11" {
                let target = &record["value"];
                Some((
                    u8::try_from(
                        target["block"]
                            .as_u64()
                            .context("reference block missing")?,
                    )?,
                    u8::try_from(
                        target["module"]
                            .as_u64()
                            .context("reference module missing")?,
                    )?,
                    u32::try_from(
                        target["offset"]
                            .as_u64()
                            .context("reference offset missing")?,
                    )?,
                ))
            } else {
                None
            };
            let location = SaveRecordLocation {
                id: id.clone(),
                block,
                module,
                offset: components[2],
                live_width: width,
                kind: record["kind"].as_str().context("save kind missing")?.into(),
                flags: record["flags"]
                    .as_str()
                    .context("save flags missing")?
                    .into(),
                reference_target,
            };
            if let Some(prior) = locations.get(id) {
                if prior.live_width != width || prior.reference_target != reference_target {
                    conflicting_occurrences
                        .entry(id.clone())
                        .or_default()
                        .push(file.clone());
                }
            } else {
                locations.insert(id.clone(), location);
            }
        }
    }
    for id in conflicting_occurrences.keys() {
        locations.remove(id);
    }
    let selection = Selection::hype(level, "en")?;
    let queries = locations.into_values().collect::<Vec<_>>();
    let joined = join_save_allocations(game, &selection, &queries)?;
    let mut statuses = BTreeMap::<&str, usize>::new();
    for item in &joined.joins {
        *statuses.entry(item.status).or_default() += 1;
    }
    Ok(json!({
        "schema": 1,
        "profile": ENGLISH_PC_PROFILE,
        "source_level": level,
        "save_source": save,
        "record_identity_count": queries.len() + conflicting_occurrences.len(),
        "joined_query_count": queries.len(),
        "status_counts": statuses,
        "conflicting_occurrences": conflicting_occurrences,
        "joins": joined.joins,
        "designer_discovery": joined.designer_discovery,
        "interpretation": "bounded source allocation only; field meaning and runtime restoration require separate evidence",
    }))
}

fn semantic_record(record: Option<&Value>) -> Option<Value> {
    record.map(|value| {
        let mut result = value.clone();
        result.as_object_mut().unwrap().remove("position");
        result
    })
}

fn verified_semantic_value(record: &Value) -> Option<Value> {
    let semantic = record.get("semantic")?;
    if semantic["confidence"] != "fresh_process_reload_demonstrated" {
        return None;
    }
    if semantic["type"] == "Boolean" {
        return match record["value_hex"].as_str()? {
            "00" => Some(json!(false)),
            "01" => Some(json!(true)),
            _ => None,
        };
    }
    if semantic["unit"] == "points" && record["kind"] == "0x02" {
        return record.get("value").cloned();
    }
    None
}

fn verified_semantic_change(before: Option<&Value>, after: Option<&Value>) -> Option<Value> {
    let before = before?;
    let after = after?;
    let semantic = before.get("semantic")?;
    if Some(semantic) != after.get("semantic") {
        return None;
    }
    let old = verified_semantic_value(before)?;
    let new = verified_semantic_value(after)?;
    (old != new).then(|| {
        json!({
            "owner": semantic["owner"],
            "field": semantic["field"],
            "type": semantic.get("type"),
            "unit": semantic.get("unit"),
            "confidence": semantic["confidence"],
            "before": old,
            "after": new,
        })
    })
}

pub fn diff(before: &Path, after: &Path) -> Result<Value> {
    diff_with_slots(before, after, None, None, None)
}

fn scoped_files(report: &Value, slot: Option<&str>) -> Result<BTreeMap<String, Value>> {
    let files = report["files"]
        .as_object()
        .context("save report has no files")?;
    if let Some(slot) = slot {
        ensure!(
            slot.starts_with("SaveGame")
                && slot[8..]
                    .chars()
                    .all(|character| character.is_ascii_digit())
                && slot.len() > 8,
            "slot must be SaveGame followed by digits"
        );
    }
    let mut result = BTreeMap::new();
    for (name, value) in files {
        let key = if let Some(slot) = slot {
            ensure!(
                !name.contains('/'),
                "slot override requires a flat slot directory"
            );
            format!("{slot}/{name}")
        } else {
            name.clone()
        };
        result.insert(key, value.clone());
    }
    Ok(result)
}

pub fn diff_with_slots(
    before: &Path,
    after: &Path,
    before_slot: Option<&str>,
    after_slot: Option<&str>,
    profile: Option<&str>,
) -> Result<Value> {
    let previous = inspect_with_profile(before, profile)?;
    let current = inspect_with_profile(after, profile)?;
    let old_files = scoped_files(&previous, before_slot)?;
    let new_files = scoped_files(&current, after_slot)?;
    let mut files = BTreeMap::new();
    for name in old_files.keys().chain(new_files.keys()) {
        if files.contains_key(name) {
            continue;
        }
        let Some(old) = old_files.get(name) else {
            files.insert(name.clone(), json!({"status": "added"}));
            continue;
        };
        let Some(new) = new_files.get(name) else {
            files.insert(name.clone(), json!({"status": "removed"}));
            continue;
        };
        let mut entry = json!({
            "before_sha256": old["sha256"],
            "after_sha256": new["sha256"],
            "encoded_changed": old["sha256"] != new["sha256"],
        });
        if let (Some(a), Some(b)) = (old["records"].as_object(), new["records"].as_object()) {
            let mut changes = BTreeMap::new();
            let mut semantic_changes = BTreeMap::new();
            for key in a.keys().chain(b.keys()) {
                if changes.contains_key(key) {
                    continue;
                }
                let prior = a.get(key);
                let next = b.get(key);
                if let Some(change) = verified_semantic_change(prior, next) {
                    semantic_changes.insert(key.clone(), change);
                }
                if semantic_record(prior) != semantic_record(next) {
                    changes.insert(key.clone(), json!({"before": prior, "after": next}));
                }
            }
            entry["record_change_count"] = json!(changes.len());
            entry["record_changes"] = json!(changes);
            entry["verified_semantic_change_count"] = json!(semantic_changes.len());
            entry["verified_semantic_changes"] = json!(semantic_changes);
            if old.get("undecoded_tail") != new.get("undecoded_tail") {
                entry["undecoded_tail_change"] = json!({
                    "before": old.get("undecoded_tail"),
                    "after": new.get("undecoded_tail"),
                });
            }
        } else if old.get("text_lines").is_some() && new.get("text_lines").is_some() {
            entry["before_lines"] = old["text_lines"].clone();
            entry["after_lines"] = new["text_lines"].clone();
        } else if old.get("u32_values").is_some() && new.get("u32_values").is_some() {
            entry["before_u32"] = old["u32_values"].clone();
            entry["after_u32"] = new["u32_values"].clone();
        }
        files.insert(name.clone(), entry);
    }
    Ok(json!({
        "schema": 2,
        "format": "hype_pc_save_research_diff",
        "before": before,
        "after": after,
        "before_slot_override": before_slot,
        "after_slot_override": after_slot,
        "semantic_profile": profile,
        "files": files,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_keep_their_relative_paths() -> Result<()> {
        let temp = tempfile::tempdir()?;
        for slot in ["SaveGame0", "SaveGame1"] {
            fs::create_dir(temp.path().join(slot))?;
            fs::write(temp.path().join(slot).join("General.sav"), [0u8; 8])?;
        }
        let report = inspect(temp.path())?;
        ensure!(report["files"]["SaveGame0/General.sav"]["u32_values"] == json!([0, 0]));
        ensure!(report["files"]["SaveGame1/General.sav"]["u32_values"] == json!([0, 0]));
        ensure!(
            report["files"]
                .as_object()
                .is_some_and(|files| files.len() == 2)
        );
        ensure!(scoped_files(&report, Some("SaveGame0")).is_err());
        Ok(())
    }

    #[test]
    fn unknown_record_keeps_the_unparsed_tail() -> Result<()> {
        let mut bytes = vec![0, 5];
        bytes.extend_from_slice(&42u32.to_le_bytes());
        bytes.extend_from_slice(&0x0204u16.to_le_bytes());
        bytes.extend_from_slice(&[0, 2, 30, 0]);
        bytes.extend_from_slice(&[0, 5]);
        bytes.extend_from_slice(&43u32.to_le_bytes());
        bytes.extend_from_slice(&[4, 2, 0, 0xff, 0xaa, 0xbb]);
        let scan = scan_records(&bytes)?;
        ensure!(scan.records.len() == 1);
        ensure!(scan.undecoded_tail.as_ref().unwrap()["position"] == 12);
        ensure!(scan.undecoded_tail.as_ref().unwrap()["hex"] == hex(&bytes[12..]));
        ensure!(records(&bytes).is_err());
        Ok(())
    }

    #[test]
    fn duplicate_and_truncated_records_fail() -> Result<()> {
        let mut record = vec![0, 5];
        record.extend_from_slice(&42u32.to_le_bytes());
        record.extend_from_slice(&0x0204u16.to_le_bytes());
        record.extend_from_slice(&[0, 2, 30, 0]);
        let mut duplicate = record.clone();
        duplicate.extend_from_slice(&record);
        ensure!(records(&duplicate).is_err());
        ensure!(records(&record[..record.len() - 1]).is_err());
        let mut truncated_reference = record[..10].to_vec();
        truncated_reference[9] = 0x11;
        truncated_reference.extend_from_slice(&[0, 5, 1, 0, 0]);
        ensure!(records(&truncated_reference).is_err());
        Ok(())
    }

    #[test]
    #[ignore = "requires supplied English save fixture; selected by --local"]
    fn supplied_english_save_parses_without_record_loss() -> Result<()> {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reference/Savefile_EN.zip");
        let report = inspect(&fixture)?;
        let files = &report["files"];
        ensure!(files.as_object().is_some_and(|files| files.len() == 4));
        ensure!(files["SaveGame0/Level000.sav"]["decoded_size"] == 17_405);
        ensure!(files["SaveGame0/Level000.sav"]["record_count"] == 1_271);
        ensure!(files["SaveGame0/manoir.sav"]["decoded_size"] == 3_583);
        ensure!(files["SaveGame0/manoir.sav"]["record_count"] == 263);
        ensure!(files["SaveGame0/General000.sav"]["text_lines"][0] == "manoir");
        ensure!(
            files["SaveGame0/General.sav"]["u32_values"] == json!([0, 0]),
            "unexpected General.sav values"
        );
        let mapped = inspect_with_profile(&fixture, Some(ENGLISH_PC_PROFILE))?;
        ensure!(
            inspect_with_profile(&fixture, Some("wrong-build")).is_err(),
            "a real save was accepted under a mismatched source profile"
        );
        ensure!(
            mapped["files"]["SaveGame0/Level000.sav"]["records"]["0:5:4986212"]["semantic"]["field"]
                == "current_points"
        );
        ensure!(
            mapped["files"]["SaveGame0/Level000.sav"]["records"]["0:17:297732"]["semantic"]["field"]
                == "DSG[8]"
        );
        ensure!(
            mapped["files"]["SaveGame0/Level000.sav"]["records"]["0:10:1560"]["semantic"]["owner"]
                == "Fix inventory MagicIce 54"
        );
        ensure!(
            mapped["files"]["SaveGame0/Level000.sav"]["records"]["0:10:1560"]["semantic"]["field"]
                == "selected"
        );
        Ok(())
    }
}
