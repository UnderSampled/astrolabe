//! Convert a Hype save slot into a Godot ConfigFile with one named key per record.
//!
//! This is the proof of what the save means: every record gets a key, a typed
//! value and a stated meaning, and anything not understood stays visible under
//! `unknown/`. `[restore/...]` holds the values Load Game applies. See
//! docs/openspace/save-games.md.

use crate::openspace::save_game::{SaiRecord, SaveMember, SaveSlot};
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;

const MANOIR_KEYS: &str = include_str!("../openspace/save_keys_manoir.tsv");

struct Key {
    section: &'static str,
    name: &'static str,
    tier: &'static str,
    meaning: &'static str,
}

fn key_table(level: &str) -> Result<HashMap<&'static str, Key>> {
    let mut keys = HashMap::new();
    if level != "manoir" {
        return Ok(keys);
    }
    for line in MANOIR_KEYS.lines() {
        let mut parts = line.split('\t');
        let mut next = || parts.next().context("short save key row");
        let identity = next()?;
        keys.insert(
            identity,
            Key {
                section: next()?,
                name: next()?,
                tier: next()?,
                meaning: next()?,
            },
        );
    }
    Ok(keys)
}

fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// One- and two-byte values read as signed where the source uses -1 for "none".
fn value(bytes: &[u8]) -> String {
    match bytes {
        [one] => one.to_string(),
        [a, b] => i16::from_le_bytes([*a, *b]).to_string(),
        [a, b, c, d] => u32::from_le_bytes([*a, *b, *c, *d]).to_string(),
        _ => {
            let list: Vec<_> = bytes.iter().map(u8::to_string).collect();
            format!("PackedByteArray({})", list.join(", "))
        }
    }
}

#[derive(Default)]
struct Coverage {
    records: usize,
    tiers: BTreeMap<&'static str, usize>,
}

fn member_records(
    member: &SaveMember,
    prefix: &str,
    keys: &HashMap<&'static str, Key>,
    coverage: &mut Coverage,
    sections: &mut BTreeMap<String, Vec<(String, String)>>,
) {
    let records: &[SaiRecord] = member.records.as_ref().map_or(&[], |t| &t.records);
    for record in records {
        let identity = record.identity();
        let (section, name, tier, meaning) = match keys.get(identity.as_str()) {
            Some(key) => (
                key.section.to_owned(),
                key.name.to_owned(),
                key.tier,
                key.meaning,
            ),
            None => (
                format!(
                    "unknown/block_{}_{}",
                    record.header.block, record.header.module
                ),
                format!("offset_{}", record.header.offset),
                "unmapped",
                "no catalogue entry for this level yet",
            ),
        };
        coverage.records += 1;
        *coverage.tiers.entry(tier).or_default() += 1;
        sections
            .entry(format!("{prefix}/{section}"))
            .or_default()
            .push((
                format!("; {tier} {identity}: {meaning}"),
                format!("{name}={}", value(&record.value_bytes)),
            ));
    }
}

fn number(value: &Value) -> Result<String> {
    Ok(value.as_f64().context("expected a number")?.to_string())
}

fn restore_sections(state: &Value, out: &mut String) -> Result<()> {
    let hype = &state["actors"]["world/0/0/Hype"];
    let position = hype["transform"]["position"]
        .as_array()
        .context("position")?;
    let basis = hype["transform"]["basis"].as_array().context("basis")?;
    let list = |values: &[Value]| -> Result<String> {
        Ok(values
            .iter()
            .map(number)
            .collect::<Result<Vec<_>>>()?
            .join(", "))
    };
    let rows = basis
        .iter()
        .map(|row| list(row.as_array().context("basis row")?))
        .collect::<Result<Vec<_>>>()?;
    writeln!(
        out,
        "[restore/hype]\n; Godot placement of Hype from his saved matrix"
    )?;
    writeln!(
        out,
        "position=Vector3({})\nbasis=Basis({})",
        list(position)?,
        rows.join(", ")
    )?;
    for (key, field) in [
        ("magick_current_points", "current_points"),
        ("magick_maximum_points", "maximum_points"),
        ("magick_selected_spell_code", "selected_spell_code"),
    ] {
        writeln!(out, "{key}={}", hype["magick"][field])?;
    }
    writeln!(
        out,
        "\n[restore/inventory]\n; item id to quantity; -1 means not carried; item 0 is gold"
    )?;
    if !hype["inventory"]["gold"].is_null() {
        writeln!(out, "gold={}", hype["inventory"]["gold"])?;
    }
    for (item, amount) in hype["inventory"]["quantities"]
        .as_object()
        .context("inventory")?
    {
        writeln!(out, "item_{item}={amount}")?;
    }
    writeln!(out)?;
    for (actor, fields) in state["actors"].as_object().context("actors")? {
        if let Some(brain) = fields["brain"].as_object() {
            writeln!(out, "[restore/brain/{actor}]")?;
            for (field, value) in brain {
                writeln!(out, "{field}={value}")?;
            }
            writeln!(out)?;
        }
    }
    for (actor, fields) in state["world"].as_object().context("world")? {
        writeln!(out, "[restore/world/{actor}]")?;
        for (field, value) in fields.as_object().context("world actor")? {
            writeln!(out, "{field}={value}")?;
        }
        writeln!(out)?;
    }
    Ok(())
}

pub struct ConvertedSave {
    pub text: String,
    pub records: usize,
    pub tiers: BTreeMap<&'static str, usize>,
}

pub fn convert(
    save: &SaveSlot,
    profile: &str,
    state: Option<&Value>,
    not_restored: &[&str],
) -> Result<ConvertedSave> {
    let keys = key_table(&save.current_level)?;
    let mut coverage = Coverage::default();
    let mut sections = BTreeMap::new();
    for (name, member) in &save.members {
        let prefix = if name == "Level000.sav" {
            "player".to_owned()
        } else {
            format!("level_{}", name.trim_end_matches(".sav"))
        };
        member_records(member, &prefix, &keys, &mut coverage, &mut sections);
    }
    let mut out = String::new();
    writeln!(
        out,
        "; Hype save slot converted by Astrolabe. See docs/openspace/save-games.md."
    )?;
    writeln!(
        out,
        "; verified = shown in a running game, source = read from the engine,"
    )?;
    writeln!(
        out,
        "; unknown = kept but not understood, unmapped = no catalogue entry yet.\n"
    )?;
    writeln!(
        out,
        "[meta]\nformat=\"astrolabe-save\"\nschema=1\nsource=\"original\""
    )?;
    writeln!(
        out,
        "profile={}\nslot={}",
        quoted(profile),
        quoted(&save.slot)
    )?;
    writeln!(
        out,
        "level={}\ntitle={}",
        quoted(&save.current_level),
        quoted(&save.display_name)
    )?;
    writeln!(
        out,
        "game_timer_ms={}\n\n[coverage]\nrecords={}",
        save.game_timer_ms, coverage.records
    )?;
    for tier in ["verified", "source", "unknown", "unmapped"] {
        writeln!(
            out,
            "{tier}={}",
            coverage.tiers.get(tier).copied().unwrap_or(0)
        )?;
    }
    let list: Vec<_> = not_restored.iter().map(|item| quoted(item)).collect();
    writeln!(out, "not_restored=PackedStringArray({})\n", list.join(", "))?;
    if let Some(state) = state {
        restore_sections(state, &mut out)?;
    }
    for (section, rows) in sections {
        writeln!(out, "[{section}]")?;
        for (comment, row) in rows {
            writeln!(out, "{comment}\n{row}")?;
        }
        writeln!(out)?;
    }
    Ok(ConvertedSave {
        text: out,
        records: coverage.records,
        tiers: coverage.tiers,
    })
}
