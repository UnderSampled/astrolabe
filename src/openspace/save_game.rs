//! Hype PC save-file decoding at the OpenSpace boundary.
//!
//! The save table is a serialized view of SAI registrations, not an asset
//! model. Keep source addresses and unknown record bytes here; target adapters
//! may map only fields whose ownership and behavior have source evidence.

use anyhow::{Context, Result, bail, ensure};
use binrw::{BinRead, Endian};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Cursor, path::Path};

const MAX_DECODED_MEMBER: usize = 64 * 1024 * 1024;
const SAVE_LENGTH_XOR: u32 = 0x5455_5657;

#[derive(Debug, Clone, Copy, BinRead)]
#[br(little)]
struct SaveEnvelopeHeader {
    checksum_xor: u8,
    decoded_length_xor: u32,
}

/// Fixed ten-byte prefix shared by the Hype save-table record kinds.
#[derive(Debug, Clone, Copy, BinRead, Serialize)]
#[br(little)]
pub struct SaiRecordHeader {
    pub block: u8,
    pub module: u8,
    pub offset: u32,
    pub save_flags: u16,
    pub reserved: u8,
    pub data_type: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaiRecord {
    pub header: SaiRecordHeader,
    pub position: usize,
    /// Original value bytes. JSON uses `value_hex` to avoid expanding bytes to
    /// decimal arrays; they remain available to target adapters in memory.
    #[serde(skip_serializing)]
    pub value_bytes: Vec<u8>,
    pub value_hex: String,
}

impl SaiRecord {
    pub fn identity(&self) -> String {
        format!(
            "{}:{}:{}",
            self.header.block, self.header.module, self.header.offset
        )
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UnknownRecordTail {
    pub position: usize,
    pub data_type: u8,
    pub reason: String,
    pub value_hex: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordTable {
    pub records: Vec<SaiRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unknown_tail: Option<UnknownRecordTail>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveMember {
    pub name: String,
    pub role: String,
    pub encoded_size: usize,
    pub sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decoded_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decoded_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub u32_values: Option<[u32; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_lines: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records: Option<RecordTable>,
}

/// One complete `SaveGameN/` directory, retaining every supported record's
/// source identity and raw value bytes.
#[derive(Debug, Clone, Serialize)]
pub struct SaveSlot {
    pub schema: u32,
    pub format: String,
    pub slot: String,
    pub current_level: String,
    pub display_name: String,
    pub game_timer_ms: u64,
    pub members: BTreeMap<String, SaveMember>,
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Decode one obfuscated Hype SAI save stream and validate its checksum and
/// declared output size.
pub fn decode(source: &[u8]) -> Result<Vec<u8>> {
    ensure!(source.len() >= 5, "truncated Hype save envelope");
    let mut cursor = Cursor::new(source);
    let header = SaveEnvelopeHeader::read_options(&mut cursor, Endian::Little, ())
        .context("reading Hype save envelope")?;
    let expected_length = (header.decoded_length_xor ^ SAVE_LENGTH_XOR) as usize;
    ensure!(
        expected_length <= MAX_DECODED_MEMBER,
        "Hype save declares an oversized decoded member"
    );

    let mut stream = Vec::with_capacity(source.len() - 5);
    for (index, byte) in source[5..].iter().enumerate() {
        let mask = 0x57u8.wrapping_add(index as u8).reverse_bits() ^ 0xb9;
        stream.push(*byte ^ mask);
    }
    let checksum = stream.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte));
    ensure!(
        checksum == (header.checksum_xor ^ 0x53),
        "Hype save checksum mismatch"
    );

    let mut windows = [[0u8; 8]; 256];
    for (index, window) in windows.iter_mut().enumerate() {
        window.fill(index as u8);
    }
    let mut output = Vec::with_capacity(expected_length);
    let mut position = 0;
    while position < stream.len() {
        ensure!(
            stream.len() - position >= 2,
            "truncated Hype save window header at {position}"
        );
        let window_index = usize::from(stream[position]);
        let change_mask = stream[position + 1];
        position += 2;
        for (bit, value) in windows[window_index].iter_mut().enumerate() {
            if change_mask & (1 << bit) != 0 {
                *value = *stream
                    .get(position)
                    .context("truncated Hype save window update")?;
                position += 1;
            }
        }

        match stream.len() - position {
            0 => output.extend_from_slice(&windows[window_index]),
            1 => {
                let count = usize::from(stream[position]);
                ensure!(count <= 8, "invalid final Hype save window size {count}");
                output.extend_from_slice(&windows[window_index][..count]);
                position += 1;
            }
            _ => output.extend_from_slice(&windows[window_index]),
        }
        ensure!(
            output.len() <= expected_length,
            "Hype save expands beyond its declared size"
        );
    }
    ensure!(
        output.len() == expected_length,
        "Hype save decoded size {} != {expected_length}",
        output.len()
    );
    Ok(output)
}

/// Parse a SAI save table. An unrecognized final type is returned verbatim as
/// `unknown_tail`; subsequent boundaries are intentionally not guessed.
pub fn parse_records(bytes: &[u8]) -> Result<RecordTable> {
    let mut records = Vec::new();
    let mut position = 0;
    while position < bytes.len() {
        let start = position;
        ensure!(
            bytes.len() - start >= 10,
            "truncated Hype save record at {start}"
        );
        let mut cursor = Cursor::new(&bytes[start..]);
        let header = SaiRecordHeader::read_options(&mut cursor, Endian::Little, ())
            .with_context(|| format!("reading Hype save record header at {start}"))?;
        position += 10;
        let length = match header.data_type {
            0x01 | 0x02 | 0x04 | 0x08 => usize::from(header.data_type),
            0x10 => {
                let length_end = position
                    .checked_add(4)
                    .context("Hype save array length offset overflow")?;
                let length_bytes: [u8; 4] = bytes
                    .get(position..length_end)
                    .context("truncated Hype save array length")?
                    .try_into()
                    .expect("slice width checked");
                position = length_end;
                usize::try_from(u32::from_le_bytes(length_bytes))?
            }
            0x11 => 6,
            data_type => {
                return Ok(RecordTable {
                    records,
                    unknown_tail: Some(UnknownRecordTail {
                        position: start,
                        data_type,
                        reason: format!(
                            "unknown Hype save record type {data_type:#04x}; length is unverified"
                        ),
                        value_hex: hex(&bytes[start..]),
                    }),
                });
            }
        };
        let end = position
            .checked_add(length)
            .context("Hype save record end offset overflow")?;
        let value_bytes = bytes
            .get(position..end)
            .with_context(|| format!("Hype save record at {start} overruns its file"))?
            .to_vec();
        position = end;
        records.push(SaiRecord {
            header,
            position: start,
            value_hex: hex(&value_bytes),
            value_bytes,
        });
    }
    Ok(RecordTable {
        records,
        unknown_tail: None,
    })
}

fn save_slot_name(path: &Path) -> Result<String> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .context("save slot path has no UTF-8 directory name")?;
    let Some(suffix) = name.strip_prefix("SaveGame") else {
        bail!("pass a SaveGameN directory, not a parent directory")
    };
    ensure!(
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit()),
        "save slot directory must be named SaveGame followed by digits"
    );
    Ok(name.to_owned())
}

/// Import one original `SaveGameN` directory. The function is read-only and
/// does not modify the user's source files.
pub fn read_slot(path: &Path) -> Result<SaveSlot> {
    ensure!(
        path.is_dir(),
        "save input must be an extracted SaveGameN directory"
    );
    let slot = save_slot_name(path)?;
    let mut paths = BTreeMap::<String, std::path::PathBuf>::new();
    for entry in
        fs::read_dir(path).with_context(|| format!("listing save slot {}", path.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if Path::new(&name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("sav"))
        {
            ensure!(
                paths.insert(name.clone(), entry.path()).is_none(),
                "duplicate save member {name}"
            );
        }
    }
    for required in ["General.sav", "General000.sav", "Level000.sav"] {
        ensure!(
            paths.contains_key(required),
            "save slot is missing {required}"
        );
    }

    let mut members = BTreeMap::new();
    let mut current_level = None;
    let mut display_name = None;
    let mut game_timer_ms = None;
    for (name, member_path) in paths {
        let encoded = fs::read(&member_path)
            .with_context(|| format!("reading save member {}", member_path.display()))?;
        let (role, decoded, words, lines, records, raw_hex) = if name == "General.sav" {
            ensure!(
                encoded.len() == 8,
                "General.sav must contain two little-endian words"
            );
            let mut cursor = Cursor::new(encoded.as_slice());
            #[derive(BinRead)]
            #[br(little)]
            struct GeneralWords {
                first: u32,
                second: u32,
            }
            let values = GeneralWords::read_options(&mut cursor, Endian::Little, ())?;
            (
                "general_flags",
                None,
                Some([values.first, values.second]),
                None,
                None,
                Some(hex(&encoded)),
            )
        } else {
            let decoded_bytes = decode(&encoded).with_context(|| format!("decoding {name}"))?;
            let digest = Some(sha256(&decoded_bytes));
            if name == "General000.sav" {
                let text = std::str::from_utf8(&decoded_bytes)
                    .context("General000.sav is not UTF-8 text")?;
                let parsed_lines = text.lines().map(str::to_owned).collect::<Vec<_>>();
                ensure!(
                    parsed_lines.len() >= 4,
                    "General000.sav has fewer than four lines"
                );
                let level = parsed_lines[0].trim().to_owned();
                ensure!(
                    !level.is_empty(),
                    "General000.sav has an empty current level"
                );
                let timer_low: u32 = parsed_lines[2]
                    .trim()
                    .parse()
                    .context("parsing General000.sav timer low word")?;
                let timer_high: u32 = parsed_lines[3]
                    .trim()
                    .parse()
                    .context("parsing General000.sav timer high word")?;
                current_level = Some(level);
                display_name = Some(parsed_lines[1].clone());
                game_timer_ms = Some(u64::from(timer_low) | (u64::from(timer_high) << 32));
                (
                    "slot_description",
                    Some((decoded_bytes, digest)),
                    None,
                    Some(parsed_lines),
                    None,
                    None,
                )
            } else {
                let table =
                    parse_records(&decoded_bytes).with_context(|| format!("parsing {name}"))?;
                (
                    if name == "Level000.sav" {
                        "player_records"
                    } else {
                        "level_records"
                    },
                    Some((decoded_bytes, digest)),
                    None,
                    None,
                    Some(table),
                    None,
                )
            }
        };
        let (decoded_size, decoded_sha256) = decoded
            .map(|(bytes, digest)| (Some(bytes.len()), digest))
            .unwrap_or((None, None));
        members.insert(
            name.clone(),
            SaveMember {
                name,
                role: role.to_owned(),
                encoded_size: encoded.len(),
                sha256: sha256(&encoded),
                decoded_size,
                decoded_sha256,
                raw_hex,
                u32_values: words,
                text_lines: lines,
                records,
            },
        );
    }

    Ok(SaveSlot {
        schema: 1,
        format: "hype-pc-save-slot".to_owned(),
        slot,
        current_level: current_level.context("save slot has no parsed level metadata")?,
        display_name: display_name.context("save slot has no parsed display name")?,
        game_timer_ms: game_timer_ms.context("save slot has no parsed game timer")?,
        members,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn compressed_fixture(decoded: &[u8]) -> Vec<u8> {
        let mut stream = Vec::new();
        for chunk in decoded.chunks(8) {
            stream.push(0);
            let mask = if chunk.len() == 8 {
                u8::MAX
            } else {
                (1u8 << chunk.len()) - 1
            };
            stream.push(mask);
            stream.extend_from_slice(chunk);
            if chunk.len() < 8 {
                stream.push(chunk.len() as u8);
            }
        }
        let checksum = stream.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte));
        let mut encoded = Vec::with_capacity(5 + stream.len());
        encoded.push(checksum ^ 0x53);
        encoded.extend_from_slice(&((decoded.len() as u32) ^ SAVE_LENGTH_XOR).to_le_bytes());
        for (index, byte) in stream.iter().enumerate() {
            let mask = 0x57u8.wrapping_add(index as u8).reverse_bits() ^ 0xb9;
            encoded.push(*byte ^ mask);
        }
        encoded
    }

    fn record(block: u8, module: u8, offset: u32, flags: u16, kind: u8, value: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(block);
        bytes.push(module);
        bytes.extend_from_slice(&offset.to_le_bytes());
        bytes.extend_from_slice(&flags.to_le_bytes());
        bytes.push(0);
        bytes.push(kind);
        if kind == 0x10 {
            bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
        }
        bytes.extend_from_slice(value);
        bytes
    }

    #[test]
    fn decodes_envelope_and_preserves_unknown_record_tail() -> Result<()> {
        let bytes = b"test SAI values";
        let encoded = compressed_fixture(bytes);
        assert_eq!(decode(&encoded)?, bytes);

        let mut table_bytes = record(0, 5, 123, 0x0204, 2, &[0x34, 0x12]);
        table_bytes.extend_from_slice(&record(0, 5, 125, 0x0204, 0x7f, &[0x55]));
        let table = parse_records(&table_bytes)?;
        assert_eq!(table.records[0].identity(), "0:5:123");
        assert_eq!(table.records[0].value_bytes, [0x34, 0x12]);
        let tail = table.unknown_tail.context("unknown tail preserved")?;
        assert_eq!(tail.position, 12);
        assert_eq!(tail.value_hex, "00057d0000000402007f55");
        Ok(())
    }

    #[test]
    fn imports_an_extracted_original_slot_without_modifying_it() -> Result<()> {
        let temp = tempdir()?;
        let slot = temp.path().join("SaveGame0");
        fs::create_dir(&slot)?;
        fs::write(slot.join("General.sav"), [0u8; 8])?;
        fs::write(
            slot.join("General000.sav"),
            compressed_fixture(b"manoir\r\nGogoud's Manoir I\r\n249589\r\n0\r\n"),
        )?;
        fs::write(
            slot.join("Level000.sav"),
            compressed_fixture(&record(0, 5, 4986212, 0x0204, 2, &[30, 0])),
        )?;
        fs::write(
            slot.join("manoir.sav"),
            compressed_fixture(&record(1, 5, 710068, 0x0a0c, 1, &[1])),
        )?;
        let imported = read_slot(&slot)?;
        assert_eq!(imported.current_level, "manoir");
        assert_eq!(imported.display_name, "Gogoud's Manoir I");
        assert_eq!(imported.game_timer_ms, 249_589);
        assert_eq!(imported.members["Level000.sav"].role, "player_records");
        assert_eq!(
            imported.members["manoir.sav"]
                .records
                .as_ref()
                .unwrap()
                .records[0]
                .identity(),
            "1:5:710068"
        );
        Ok(())
    }
}
