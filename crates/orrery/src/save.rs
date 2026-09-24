//! Read-only research decoder for the English Hype PC save files.
//! This is Orrery evidence tooling, not an Astrolabe content adapter or editor.

use anyhow::{Context, Result, bail, ensure};
use flate2::read::DeflateDecoder;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, path::Path};

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
        let base = Path::new(name)
            .file_name()
            .context("missing ZIP member basename")?
            .to_string_lossy()
            .into_owned();
        ensure!(
            result.insert(base.clone(), data).is_none(),
            "duplicate ZIP save {base}"
        );
    }
    ensure!(!result.is_empty(), "no .sav members in ZIP");
    Ok(result)
}

fn members(path: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    if path.is_dir() {
        let directory = if path.join("SaveGame0").is_dir() {
            path.join("SaveGame0")
        } else {
            path.to_path_buf()
        };
        let mut result = BTreeMap::new();
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let member = entry.path();
            if member
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("sav"))
            {
                result.insert(
                    entry.file_name().to_string_lossy().into_owned(),
                    fs::read(member)?,
                );
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
    ensure!(
        source.len() >= 5,
        "save is shorter than its five-byte header"
    );
    let expected_checksum = source[0] ^ 0x53;
    let expected_size = u32_at(source, 1)? ^ 0x5455_5657;
    ensure!(
        expected_size as usize <= MAX_MEMBER,
        "decoded save exceeds research limit"
    );
    let mut stream = Vec::with_capacity(source.len() - 5);
    let mut key = 0x57u8;
    for &byte in &source[5..] {
        stream.push(byte ^ key.reverse_bits() ^ 0xb9);
        key = key.wrapping_add(1);
    }
    let checksum = stream
        .iter()
        .fold(0u8, |sum, value| sum.wrapping_add(*value));
    ensure!(
        checksum == expected_checksum,
        "compressed stream checksum {checksum:#04x} != {expected_checksum:#04x}"
    );
    let mut history = [0u8; 256 * 8];
    for (index, chunk) in history.as_chunks_mut::<8>().0.iter_mut().enumerate() {
        chunk.fill(index as u8);
    }
    let mut window = [0u8; 8];
    let mut result = Vec::with_capacity(expected_size as usize);
    let mut pos = 0;
    while pos < stream.len() {
        ensure!(stream.len() - pos >= 2, "truncated save window selector");
        let index = usize::from(stream[pos]);
        let changed = stream[pos + 1];
        pos += 2;
        for slot in 0..8 {
            if changed & (1 << slot) != 0 {
                let value = *stream
                    .get(pos)
                    .context("truncated save window replacement")?;
                history[index * 8 + slot] = value;
                pos += 1;
            }
            window[slot] = history[index * 8 + slot];
        }
        let remaining = stream.len() - pos;
        match remaining {
            0 => result.extend_from_slice(&window),
            1 => {
                let count = usize::from(stream[pos]);
                ensure!(count <= 8, "invalid final save window size {count}");
                result.extend_from_slice(&window[..count]);
                pos += 1;
            }
            _ => result.extend_from_slice(&window),
        }
        ensure!(
            result.len() <= MAX_MEMBER,
            "decoded save exceeds research limit"
        );
    }
    ensure!(
        result.len() == expected_size as usize,
        "decoded save size {} != {expected_size}",
        result.len()
    );
    Ok(result)
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

pub fn records(bytes: &[u8]) -> Result<BTreeMap<String, Value>> {
    let mut result = BTreeMap::new();
    let mut pos = 0;
    while pos < bytes.len() {
        let start = pos;
        ensure!(bytes.len() - pos >= 10, "truncated save record at {pos}");
        let block = bytes[pos];
        let module = bytes[pos + 1];
        let offset = u32_at(bytes, pos + 2)?;
        let flags = u16_at(bytes, pos + 6)?;
        let reserved = bytes[pos + 8];
        let kind = bytes[pos + 9];
        pos += 10;
        let length = match kind {
            1 | 2 | 4 | 8 => usize::from(kind),
            0x10 => {
                let length = u32_at(bytes, pos)? as usize;
                pos += 4;
                length
            }
            0x11 => 6,
            _ => bail!("unknown save record type {kind:#04x} at {start}"),
        };
        let end = pos
            .checked_add(length)
            .context("save record end offset overflow")?;
        let value = bytes
            .get(pos..end)
            .context("save record overruns decoded file")?;
        pos = end;
        let id = format!("{block}:{module}:{offset}");
        let entry = json!({
            "position": start,
            "flags": format!("0x{flags:04x}"),
            "reserved": reserved,
            "kind": format!("0x{kind:02x}"),
            "value": record_value(kind, value)?,
            "value_hex": hex(value),
        });
        ensure!(
            result.insert(id.clone(), entry).is_none(),
            "duplicate save record {id}"
        );
    }
    Ok(result)
}

pub fn inspect(path: &Path) -> Result<Value> {
    let mut files = BTreeMap::new();
    for (name, source) in members(path)? {
        let mut entry = json!({
            "encoded_size": source.len(),
            "sha256": sha256(&source),
        });
        if name == "General.sav" {
            if source.len() == 8 {
                entry["u32_values"] = json!([u32_at(&source, 0)?, u32_at(&source, 4)?]);
            }
        } else {
            let data = decode(&source).with_context(|| format!("decoding {name}"))?;
            entry["decoded_size"] = json!(data.len());
            entry["decoded_sha256"] = json!(sha256(&data));
            if name.starts_with("General") {
                let lines = String::from_utf8_lossy(&data)
                    .lines()
                    .map(str::to_owned)
                    .collect::<Vec<_>>();
                entry["text_lines"] = json!(lines);
            } else {
                let parsed = records(&data).with_context(|| format!("parsing {name}"))?;
                let mut kinds = BTreeMap::<String, usize>::new();
                let mut flags = BTreeMap::<String, usize>::new();
                for record in parsed.values() {
                    *kinds
                        .entry(record["kind"].as_str().unwrap().to_owned())
                        .or_default() += 1;
                    *flags
                        .entry(record["flags"].as_str().unwrap().to_owned())
                        .or_default() += 1;
                }
                entry["record_count"] = json!(parsed.len());
                entry["types"] = json!(kinds);
                entry["flags"] = json!(flags);
                entry["records"] = json!(parsed);
            }
        }
        files.insert(name, entry);
    }
    Ok(json!({"schema": 1, "format": "hype_pc_save_research", "source": path, "files": files}))
}

fn semantic_record(record: Option<&Value>) -> Option<Value> {
    record.map(|value| {
        let mut result = value.clone();
        result.as_object_mut().unwrap().remove("position");
        result
    })
}

pub fn diff(before: &Path, after: &Path) -> Result<Value> {
    let previous = inspect(before)?;
    let current = inspect(after)?;
    let old_files = previous["files"].as_object().unwrap();
    let new_files = current["files"].as_object().unwrap();
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
            for key in a.keys().chain(b.keys()) {
                if changes.contains_key(key) {
                    continue;
                }
                let prior = a.get(key);
                let next = b.get(key);
                if semantic_record(prior) != semantic_record(next) {
                    changes.insert(key.clone(), json!({"before": prior, "after": next}));
                }
            }
            entry["record_change_count"] = json!(changes.len());
            entry["record_changes"] = json!(changes);
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
        "schema": 1,
        "format": "hype_pc_save_research_diff",
        "before": before,
        "after": after,
        "files": files,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires supplied English save fixture; selected by --local"]
    fn supplied_english_save_parses_without_record_loss() -> Result<()> {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reference/Savefile_EN.zip");
        let report = inspect(&fixture)?;
        let files = &report["files"];
        ensure!(files.as_object().is_some_and(|files| files.len() == 4));
        ensure!(files["Level000.sav"]["decoded_size"] == 17_405);
        ensure!(files["Level000.sav"]["record_count"] == 1_271);
        ensure!(files["manoir.sav"]["decoded_size"] == 3_583);
        ensure!(files["manoir.sav"]["record_count"] == 263);
        ensure!(files["General000.sav"]["text_lines"][0] == "manoir");
        ensure!(
            files["General.sav"]["u32_values"] == json!([0, 0]),
            "unexpected General.sav values"
        );
        Ok(())
    }
}
