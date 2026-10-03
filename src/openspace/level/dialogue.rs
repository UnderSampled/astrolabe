//! Manor localization import. Reuses SNA/LZO/relocation codecs; presentation stays in Godot.
use super::{
    container::{file, merge_relocations, pointer_file, reloc, sna, u32_at, wire},
    records::*,
    scene::SceneImport,
    session::SourceSession,
    sound::SoundSites,
};
use crate::model::{
    AssetPath, AssetRef, Selection,
    ai::{BehaviorGroup, BehaviorRef},
    character::Perso,
    dialogue::*,
    io::{Content, Diagnostic, Severity},
};
use anyhow::{Context, Result, ensure};
use binrw::BinRead;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
#[derive(BinRead)]
#[br(little)]
struct ItemWire {
    answers: u8,
    _selected: u8,
    box_style: u8,
    _reserved: u8,
    _answers: u32,
    _unknown: u32,
    _lines: u32,
    _owner: u32,
    _runtime_mode: u32,
    _name: u32,
    _runtime_line: u32,
    _unknown32: u32,
    _icon: u32,
    _show_channels: u32,
    _hide_channels: u32,
    _voice: u32,
}
#[derive(BinRead)]
#[br(little)]
struct LineWire {
    _next: u32,
    _spans: u32,
    ordinal: u32,
    flags: u8,
    _padding: [u8; 3],
    _effect: u32,
    _unknown: u32,
    end_seconds: f32,
}
#[derive(BinRead)]
#[br(little)]
struct SpanWire {
    _next: u32,
    _text: u32,
    offset: f32,
    _font: u32,
}
fn add(a: Address, n: usize) -> Address {
    Address {
        offset: a.offset + n,
        ..a
    }
}
/// The installed manor RTG header is 11 although only two groups follow.
/// Its header meaning is not established. Accept this evidenced profile only;
/// every populated language block must still have a complete relocation group.
fn language_relocations(
    bytes: &[u8],
    blocks: &BTreeMap<BlockKey, Block>,
    expected_header: u8,
) -> Result<BTreeMap<BlockKey, Vec<RelocRow>>> {
    ensure!(
        bytes.first().copied() == Some(expected_header),
        "unrecognized manor RTG header"
    );
    let mut pos = 1;
    let mut keys = BTreeSet::new();
    while pos < bytes.len() {
        let h: RelocGroup = wire(&bytes[pos..])?;
        pos += 6;
        ensure!(
            blocks.contains_key(&BlockKey(h.module, h.id)) && keys.insert(BlockKey(h.module, h.id)),
            "RTG block identity differs"
        );
        if h.count > 0 {
            let (_, used) = crate::openspace::compression::decode_envelope(
                &bytes[pos..],
                super::container::MAX_BLOCK,
            )?;
            pos += used;
        }
    }
    ensure!(
        pos == bytes.len()
            && blocks
                .iter()
                .all(|(k, b)| keys.contains(k) || b.data.is_empty()),
        "RTG omitted a populated block"
    );
    let mut normalized = bytes.to_vec();
    normalized[0] = keys.len().try_into()?;
    reloc(&normalized)
}
pub(super) fn load_language(
    s: &mut SourceSession,
    game: &Path,
    selection: &Selection,
) -> Result<()> {
    ensure!(
        selection.locale == "en",
        "manor dialogue extraction requires evidenced English locale"
    );
    let folder = game.join("LangData/English/world/levels");
    for (owner, prefix) in [
        (Owner::Fix, folder.join("Fix")),
        (
            Owner::Level,
            folder.join(&selection.level).join(&selection.level),
        ),
    ] {
        let blocks = sna(&file(&prefix.with_extension("lng"))?)?;
        let relocations = language_relocations(
            &file(&prefix.with_extension("rtg"))?,
            &blocks,
            match owner {
                Owner::Fix => 4,
                Owner::Level => 11,
            },
        )?;
        for (key, block) in blocks {
            if let Some(old) = s.blocks.get(&owner).and_then(|b| b.get(&key)) {
                ensure!(
                    old.base == block.base,
                    "language/source block bases differ for {key:?}"
                );
                if !old.data.is_empty() {
                    ensure!(
                        block.data.is_empty() || old.data == block.data,
                        "language overwrites populated source block"
                    );
                    continue;
                }
            }
            if !block.data.is_empty() {
                s.actual.insert(key, owner);
            }
            s.blocks.entry(owner).or_default().insert(key, block);
        }
        merge_relocations(&mut s.pointers, owner, relocations)?;
    }
    Ok(())
}
fn string(s: &SourceSession, a: Address) -> Result<String> {
    let bytes = &s.block(a)?.data[a.offset..];
    let end = bytes
        .iter()
        .take(4097)
        .position(|b| *b == 0)
        .context("unterminated dialogue text")?;
    decode_cp1252(&bytes[..end])
}
fn decode_cp1252(bytes: &[u8]) -> Result<String> {
    const HIGH: [u32; 32] = [
        0x20ac, 0, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, 0x02c6, 0x2030, 0x0160, 0x2039,
        0x0152, 0, 0x017d, 0, 0, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014, 0x02dc,
        0x2122, 0x0161, 0x203a, 0x0153, 0, 0x017e, 0x0178,
    ];
    bytes
        .iter()
        .map(|&b| {
            let code = if (0x80..=0x9f).contains(&b) {
                HIGH[usize::from(b - 0x80)]
            } else {
                u32::from(b)
            };
            ensure!(
                code != 0 && (b >= 32 || b == b'\t'),
                "unsupported Windows-1252 text byte {b}"
            );
            char::from_u32(code).context("invalid dialogue character")
        })
        .collect()
}
pub(super) fn import(
    s: &SourceSession,
    game: &Path,
    selection: &Selection,
    actors: &BTreeMap<Address, AssetRef<Perso>>,
    result: &mut SceneImport,
) -> Result<()> {
    let prefix = game
        .join("LangData/English/world/levels")
        .join(&selection.level)
        .join(&selection.level);
    let snapshot = file(&prefix.with_extension("dlg"))?;
    ensure!(snapshot.len() == 48, "DLG snapshot extent differs");
    let refs = pointer_file(&file(&prefix.with_extension("rtd"))?)?;
    let resolve = |offset| -> Result<Address> {
        let word = u32_at(&snapshot, offset)?;
        let key = *refs.get(&word).context("DLG root/font lacks RTD binding")?;
        s.resolve(Owner::Level, key, word)
    };
    let normal_font = resolve(8)?;
    let emphasis = resolve(12)?;
    // +0 is the text-name table; +4 is the circular dialogue catalogue.
    let root = resolve(4)?;
    let mut list = root;
    let mut seen = BTreeSet::new();
    let sounds = SoundSites::open(game, selection, s)?;
    // The normal manor opening alternates the six Gogoud and two Hype-statue
    // items. Keep this bounded set explicit until branching/channel semantics
    // are promoted; one representative gameplay item exercises framed pages.
    let supported = [
        "gogood_texte1_manoir",
        "gogood_texte1a_manoir",
        "gogood_texte1b_manoir",
        "gogood_texte1c_manoir",
        "gogood_texte2_manoir",
        "gogood_texte2a_manoir",
        "hype_texte1_manoir",
        "hype_texte2_manoir",
        "gogood_master_whatif01",
    ];
    while seen.insert(list) {
        ensure!(seen.len() <= 1024, "dialogue list exceeds bound");
        let item = s.required(list, 8)?;
        let name = string(s, s.required(item, 24)?)?;
        if supported.contains(&name.as_str()) {
            let h: ItemWire = wire(s.bytes(item, 52)?)?;
            ensure!(
                h.answers == 1 && matches!(h.box_style, 1 | 3),
                "{name}: unsupported selected dialogue structure"
            );
            ensure!(
                s.pointer(item, 36)?.is_none()
                    && s.pointer(item, 40)?.is_none()
                    && s.pointer(item, 44)?.is_none(),
                "{name}: selected dialogue needs item icon/channel side effects"
            );
            let object = s.required(item, 16)?;
            let actor = s.required(object, 4)?;
            let owner = actors
                .get(&actor)
                .context("dialogue owner absent from imported actor graph")?
                .clone();
            let answer = s.required(s.required(item, 4)?, 0)?;
            ensure!(
                s.pointer(answer, 24)?.is_none()
                    && u32_at(s.bytes(answer, 80)?, 40)? == 0
                    && u32_at(s.bytes(answer, 80)?, 76)? == u32::MAX - 1,
                "{name}: selected dialogue has branching/answer text/icon"
            );
            let callback = s
                .pointer(answer, 28)?
                .map(|target| -> Result<BehaviorRef> {
                    let brain = s.required(actor, 16)?;
                    let mind = s.required(brain, 0)?;
                    let model = s.required(mind, 0)?;
                    let set = s.required(model, 0)?;
                    let raw: SetWire = wire(s.bytes(set, 16)?)?;
                    let entries = s.required(set, 0)?;
                    ensure!(
                        target.owner == entries.owner
                            && target.key == entries.key
                            && target.offset >= entries.offset
                            && (target.offset - entries.offset).is_multiple_of(12),
                        "dialogue callback misses normal behavior table"
                    );
                    let behavior = (target.offset - entries.offset) / 12;
                    ensure!(
                        behavior < raw.count as usize,
                        "dialogue behavior index out of range"
                    );
                    let path = result
                        .persos
                        .iter()
                        .find(|p| p.path == owner.path)
                        .and_then(|p| p.brain.as_ref())
                        .context("dialogue owner lacks compiled Brain")?
                        .model
                        .clone();
                    let reference = BehaviorRef {
                        model: path,
                        group: BehaviorGroup::Normal,
                        behavior: behavior.try_into()?,
                    };
                    reference.validate_model(
                        result
                            .ai_models
                            .iter()
                            .find(|m| m.path == reference.model.path)
                            .context("dialogue callback model missing")?,
                    )?;
                    Ok(reference)
                })
                .transpose()?;
            let mut lines = Vec::new();
            let mut line = s.pointer(item, 12)?;
            let mut line_seen = BTreeSet::new();
            while let Some(a) = line {
                ensure!(
                    line_seen.insert(a) && lines.len() < 1024,
                    "cyclic/excessive dialogue lines"
                );
                let h: LineWire = wire(s.bytes(a, 28)?)?;
                ensure!(
                    h.ordinal as usize == lines.len() && h.flags == 0,
                    "{name}: dialogue line order/flags need additional behavior"
                );
                let mut spans = Vec::new();
                let mut span = s.pointer(a, 4)?;
                let mut span_seen = BTreeSet::new();
                while let Some(a) = span {
                    ensure!(
                        span_seen.insert(a) && spans.len() < 64,
                        "cyclic/excessive dialogue spans"
                    );
                    let h: SpanWire = wire(s.bytes(a, 16)?)?;
                    let font = s.required(a, 12)?;
                    ensure!(
                        font == normal_font || font == emphasis,
                        "{name}: dialogue span uses an unpromoted font"
                    );
                    spans.push(DialogueSpan {
                        text: s
                            .pointer(a, 4)?
                            .map(|a| string(s, a))
                            .transpose()?
                            .unwrap_or_default(),
                        offset: h.offset,
                        emphasis: font == emphasis,
                    });
                    span = s.pointer(a, 0)?;
                }
                lines.push(DialogueLine {
                    spans,
                    end_seconds: h.end_seconds,
                    flags: h.flags,
                });
                line = s.pointer(a, 0)?;
            }
            let voice = s
                .pointer(item, 48)?
                .map(|a| {
                    sounds
                        .events
                        .get(&add(a, 12))
                        .cloned()
                        .context("dialogue voice lacks localized SND binding")
                })
                .transpose()?
                .flatten();
            let value = Dialogue {
                path: AssetPath::new(format!("level:/dialogue/en/{name}"))?,
                name,
                locale: selection.locale.clone(),
                box_style: h.box_style,
                owner,
                lines,
                voice,
                completion: callback,
            };
            value.validate()?;
            result.dialogues.push(value);
        }
        list = s.required(list, 0)?;
    }
    ensure!(
        list == root && result.dialogues.len() == supported.len(),
        "dialogue catalogue/selected names differ"
    );
    result.diagnostics.push(Diagnostic { code:"openspace.dialogue.manor_slice".into(),asset:Some(result.scene.path.clone()),severity:Severity::Warning,message:format!("Imported {} evidenced manor dialogues out of {} items (eight opening items and one ordinary conversation); other dialogue choices, channel effects, source reconstruction and font metrics remain pending",result.dialogues.len(),seen.len()) });
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows1252_keeps_typography_and_rejects_undefined_bytes() -> Result<()> {
        assert_eq!(decode_cp1252(b"caf\xe9 \x93yes\x94\x97")?, "café “yes”—");
        assert!(decode_cp1252(&[0x81]).is_err());
        Ok(())
    }
}
