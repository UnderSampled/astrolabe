//! Byte coverage of the source level and Fix blocks: which bytes the parser read,
//! from which parsing site, and which it never touched. A signal of what we do not
//! understand yet, never a gate and never used to rebuild anything.
//!
//! Off unless `ASTROLABE_BYTE_COVERAGE` names an output directory. Every
//! `SourceSession` read goes through `record`, tagged with the caller's source
//! line through `#[track_caller]`, so the parser pays one branch when it is off.
use super::records::{Address, BlockKey, Owner};
use std::collections::BTreeMap;
use std::panic::Location;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

type Reads = Vec<(usize, usize, &'static Location<'static>)>;

struct Table {
    blocks: BTreeMap<(Owner, BlockKey), (usize, Reads)>,
    whole_block: BTreeMap<(Owner, BlockKey), Vec<&'static Location<'static>>>,
}

fn table() -> Option<&'static Mutex<Table>> {
    static TABLE: OnceLock<Option<Mutex<Table>>> = OnceLock::new();
    TABLE
        .get_or_init(|| {
            std::env::var_os("ASTROLABE_BYTE_COVERAGE").map(|_| {
                Mutex::new(Table {
                    blocks: BTreeMap::new(),
                    whole_block: BTreeMap::new(),
                })
            })
        })
        .as_ref()
}

pub(super) fn record(a: Address, n: usize, block_len: usize, site: &'static Location<'static>) {
    let Some(table) = table() else { return };
    let mut table = table.lock().expect("coverage table");
    let entry = table
        .blocks
        .entry((a.owner, a.key))
        .or_insert_with(|| (block_len, Vec::new()));
    entry.1.push((a.offset, n, site));
}

/// Every block appears in the report, including blocks nothing reads.
pub(super) fn register(owner: Owner, key: BlockKey, len: usize) {
    let Some(table) = table() else { return };
    let mut table = table.lock().expect("coverage table");
    table
        .blocks
        .entry((owner, key))
        .or_insert_with(|| (len, Vec::new()));
}

/// Callers that slice a whole block directly are listed, not attributed per byte.
pub(super) fn record_block(a: Address, site: &'static Location<'static>) {
    let Some(table) = table() else { return };
    let mut table = table.lock().expect("coverage table");
    let sites = table.whole_block.entry((a.owner, a.key)).or_default();
    if !sites.contains(&site) {
        sites.push(site);
    }
}

/// A read counts as uninterpreted when its parsing line says it keeps bytes
/// opaque: the source line mentions unknown, preserved, opaque or raw bytes.
fn uninterpreted(site: &Location<'_>) -> bool {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(site.file());
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    let line = text
        .lines()
        .nth(site.line().saturating_sub(1) as usize)
        .unwrap_or("")
        .to_ascii_lowercase();
    ["unknown", "preserv", "opaque", "raw"]
        .iter()
        .any(|word| line.contains(word))
}

/// Write `<dir>/<label>.json` and `<dir>/<label>.txt`. Returns the summary text.
pub fn write_report(label: &str) -> anyhow::Result<Option<String>> {
    let (Some(table), Some(dir)) = (table(), std::env::var_os("ASTROLABE_BYTE_COVERAGE")) else {
        return Ok(None);
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir)?;
    let table = table.lock().expect("coverage table");
    let mut text = String::new();
    let mut rows = Vec::new();
    let mut interpretation: BTreeMap<String, bool> = BTreeMap::new();
    for (&(owner, key), (len, reads)) in &table.blocks {
        if *len == 0 {
            continue;
        }
        // 0 untouched, 1 read but uninterpreted, 2 interpreted. Each byte keeps
        // the strongest reading and the site that gave it.
        let mut state = vec![0u8; *len];
        let mut tag: Vec<Option<&'static Location<'static>>> = vec![None; *len];
        for &(offset, n, site) in reads {
            let key = format!("{}:{}", site.file(), site.line());
            let kind = if *interpretation
                .entry(key)
                .or_insert_with(|| uninterpreted(site))
            {
                1
            } else {
                2
            };
            for i in offset..(offset + n).min(*len) {
                if kind >= state[i] {
                    state[i] = kind;
                    tag[i] = Some(site);
                }
            }
        }
        let count = |k: u8| state.iter().filter(|&&s| s == k).count();
        let (untouched, opaque, interpreted) = (count(0), count(1), count(2));
        let mut gaps = Vec::new();
        let mut start = None;
        for i in 0..=*len {
            let gap = state.get(i) == Some(&0);
            match (gap, start) {
                (true, None) => start = Some(i),
                (false, Some(s)) => {
                    let site = |j: Option<usize>| {
                        j.and_then(|j| tag.get(j).copied().flatten())
                            .map(|l| {
                                format!(
                                    "{}:{}",
                                    l.file().trim_start_matches("src/openspace/level/"),
                                    l.line()
                                )
                            })
                            .unwrap_or_default()
                    };
                    gaps.push((i - s, s, site(s.checked_sub(1)), site(Some(i))));
                    start = None;
                }
                _ => {}
            }
        }
        gaps.sort_by_key(|gap| std::cmp::Reverse(gap.0));
        let pct = |n: usize| {
            if *len == 0 {
                0.0
            } else {
                100.0 * n as f64 / *len as f64
            }
        };
        let whole = table
            .whole_block
            .get(&(owner, key))
            .map(|v| {
                v.iter()
                    .map(|l| format!("{}:{}", l.file(), l.line()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        text.push_str(&format!(
            "{owner:?} block {}:{} {len} bytes: interpreted {:.1}%, read uninterpreted {:.1}%, untouched {:.1}%{}\n",
            key.0,
            key.1,
            pct(interpreted),
            pct(opaque),
            pct(untouched),
            if whole.is_empty() { String::new() } else { format!(" (also sliced whole by {})", whole.join(", ")) }
        ));
        for (size, offset, before, after) in gaps.iter().take(5) {
            text.push_str(&format!(
                "    gap 0x{offset:x} size {size}: after {before}, before {after}\n"
            ));
        }
        rows.push(serde_json::json!({
            "owner": format!("{owner:?}"), "block": [key.0, key.1], "bytes": len,
            "interpreted": interpreted, "read_uninterpreted": opaque, "untouched": untouched,
            "whole_block_readers": whole,
            "largest_gaps": gaps.iter().take(20).map(|(size, offset, before, after)| serde_json::json!({
                "offset": offset, "size": size, "after_read_by": before, "before_read_by": after})).collect::<Vec<_>>(),
        }));
    }
    std::fs::write(
        dir.join(format!("{label}.json")),
        serde_json::to_vec_pretty(&rows)?,
    )?;
    std::fs::write(dir.join(format!("{label}.txt")), &text)?;
    Ok(Some(text))
}
