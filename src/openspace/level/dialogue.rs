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
pub(super) fn decode_cp1252(bytes: &[u8]) -> Result<String> {
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
    let presentation = super::dialogue_presentation::import(s, game, normal_font, emphasis)?;
    let presentation_ref = AssetRef::from_path(presentation.path.clone());
    result.dialogue_presentation = Some(presentation);
    // +0 is the text-name table; +4 is the circular dialogue catalogue.
    let root = resolve(4)?;
    let mut list = root;
    let mut seen = BTreeSet::new();
    let sounds = SoundSites::open(game, selection, s)?;
    let mut names = BTreeSet::new();
    let mut omitted = 0;
    while seen.insert(list) {
        ensure!(seen.len() <= 1024, "dialogue list exceeds bound");
        let item = s.required(list, 8)?;
        let name = string(s, s.required(item, 24)?)?;
        ensure!(names.insert(name.clone()), "duplicate dialogue name {name}");
        let decoded = ItemDecoder {
            source: s,
            selection,
            actors,
            scene: result,
            sounds: &sounds,
            fonts: [normal_font, emphasis],
            presentation: &presentation_ref,
        }
        .read(item, name.clone())?;
        match decoded {
            ItemImport::Supported(value) => result.dialogues.push(value),
            ItemImport::Unsupported(reason) => {
                omitted += 1;
                result.diagnostics.push(Diagnostic {
                    code: "openspace.dialogue.unsupported_item".into(),
                    asset: Some(AssetPath::new(format!("level:/dialogue/en/{name}"))?),
                    severity: Severity::Warning,
                    message: reason,
                });
            }
        }
        list = s.required(list, 0)?;
    }
    ensure!(list == root, "dialogue catalogue closes outside its root");
    result.diagnostics.push(Diagnostic {
        code: "openspace.dialogue.coverage".into(),
        asset: Some(result.scene.path.clone()),
        severity: if omitted == 0 { Severity::Info } else { Severity::Warning },
        message: format!(
            "Imported {} structurally supported dialogues out of {} items; {} unsupported items have individual diagnostics. Source reconstruction and visual parity remain pending",
            result.dialogues.len(), seen.len(), omitted
        ),
    });
    Ok(())
}

/// Unsupported semantics are distinct from invalid decoded structure. Pointer,
/// cycle and record errors in the supported traversal remain fatal. An item
/// rejected for unsupported semantics is not traversed further.
enum ItemImport {
    Supported(Dialogue),
    Unsupported(String),
}

struct ItemDecoder<'a> {
    source: &'a SourceSession,
    selection: &'a Selection,
    actors: &'a BTreeMap<Address, AssetRef<Perso>>,
    scene: &'a SceneImport,
    sounds: &'a SoundSites,
    fonts: [Address; 2],
    presentation: &'a AssetRef<DialoguePresentation>,
}

impl ItemDecoder<'_> {
    fn read(&self, item: Address, name: String) -> Result<ItemImport> {
        let s = self.source;
        let h: ItemWire = wire(s.bytes(item, 52)?)?;
        if h.answers != 1 || !matches!(h.box_style, 1 | 3) {
            return Ok(ItemImport::Unsupported(format!(
                "{name}: {} answers and box style {} require additional dialogue semantics",
                h.answers, h.box_style
            )));
        }
        if h._reserved != 0 || h._unknown != 0 || h._unknown32 != 0 {
            return Ok(ItemImport::Unsupported(format!(
                "{name}: nonzero unpromoted item fields require source interpretation"
            )));
        }
        if s.pointer(item, 36)?.is_some()
            || s.pointer(item, 40)?.is_some()
            || s.pointer(item, 44)?.is_some()
        {
            return Ok(ItemImport::Unsupported(format!(
                "{name}: item icon or channel side effects are not represented"
            )));
        }
        let object = s.required(item, 16)?;
        let actor = s.required(object, 4)?;
        let Some(owner) = self.actors.get(&actor).cloned() else {
            return Ok(ItemImport::Unsupported(format!(
                "{name}: dialogue owner is outside the imported actor graph"
            )));
        };
        let answer = s.required(s.required(item, 4)?, 0)?;
        let answer_bytes = s.bytes(answer, 80)?;
        if s.pointer(answer, 24)?.is_some()
            || u32_at(answer_bytes, 40)? != 0
            || u32_at(answer_bytes, 76)? != u32::MAX - 1
        {
            return Ok(ItemImport::Unsupported(format!(
                "{name}: answer text, branching or icon behavior is not represented"
            )));
        }
        let callback = if let Some(target) = s.pointer(answer, 28)? {
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
            let Some(brain) = self
                .scene
                .persos
                .iter()
                .find(|p| p.path == owner.path)
                .and_then(|p| p.brain.as_ref())
            else {
                return Ok(ItemImport::Unsupported(format!(
                    "{name}: completion callback owner lacks an imported Brain"
                )));
            };
            let reference = BehaviorRef {
                model: brain.model.clone(),
                group: BehaviorGroup::Normal,
                behavior: behavior.try_into()?,
            };
            reference.validate_model(
                self.scene
                    .ai_models
                    .iter()
                    .find(|m| m.path == reference.model.path)
                    .context("dialogue callback model missing")?,
            )?;
            Some(reference)
        } else {
            None
        };
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
                h.ordinal as usize == lines.len(),
                "{name}: dialogue line order differs"
            );
            if h.flags != 0 || h._effect != 0 || h._unknown != 0 {
                return Ok(ItemImport::Unsupported(format!(
                    "{name}: dialogue line flags/effect/unknown fields require additional behavior"
                )));
            }
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
                if !self.fonts.contains(&font) {
                    return Ok(ItemImport::Unsupported(format!(
                        "{name}: dialogue span uses an unpromoted font"
                    )));
                }
                spans.push(DialogueSpan {
                    text: s
                        .pointer(a, 4)?
                        .map(|a| string(s, a))
                        .transpose()?
                        .unwrap_or_default(),
                    offset: h.offset,
                    emphasis: font == self.fonts[1],
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
        if lines.is_empty() {
            return Ok(ItemImport::Unsupported(format!(
                "{name}: textless dialogue behavior is not represented"
            )));
        }
        let voice = s
            .pointer(item, 48)?
            .map(|a| {
                self.sounds
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
            locale: self.selection.locale.clone(),
            box_style: h.box_style,
            owner,
            lines,
            voice,
            completion: callback,
            presentation: Some(self.presentation.clone()),
        };
        value.validate()?;
        Ok(ItemImport::Supported(value))
    }
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
