//! Evidenced Manor/Astrolabe localization import. Reuses SNA/LZO/relocation codecs; presentation stays in Godot.
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
    audio::SoundEventLink,
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
// Selection/module source bodies identify these fields. Unknown prefix/runtime
// words remain unclassified; no pointer is traversed from those placeholders.
#[derive(BinRead)]
#[br(little)]
/// Answer record, laid out by the AnswerSection parser 0056fec0 and read by
/// 0056dd50 (draw), 0056f3a0 (scroll) and 0056e3b0 (select).
struct AnswerWire {
    /// +0/+4: inventory gobject/tobject cost (AnswerBad path when unmet).
    stock_lists: [u32; 2],
    /// +8: AnswerDelete; the selected answer leaves the item.
    one_shot: u8,
    _padding: [u8; 3],
    /// +0xc/+0x10: FontOFF (the drawn font) and FontON (parsed, never drawn).
    _font_off: u32,
    _font_on: u32,
    _unknown_word: u32,
    /// +0x18: AnswerNext, the item InitDraw restarts on.
    next: u32,
    /// +0x1c: AnswerComportment, stored as the pending callback.
    _completion: u32,
    /// +0x20: runtime selected flag.
    _selected: u32,
    _unknown_word2: u32,
    /// +0x28: AnswerText line list.
    text: u32,
    /// +0x2c/+0x30: AnswerState refs applied on scroll/select.
    states: [u32; 2],
    /// +0x34/+0x38: channel activate/deactivate lists.
    channels: [u32; 2],
    /// +0x3c: AnswerSound, played on MainActor when selected.
    _voice: u32,
    /// +0x40/+0x44: AnswerBad {code, item} mapping.
    result_mapping: [u32; 2],
    /// +0x48: gold cost.
    gold: i32,
    /// +0x4c: icon, -2 for none.
    icon: i32,
}
#[derive(BinRead)]
#[br(little)]
struct LineWire {
    _next: u32,
    _spans: u32,
    ordinal: u32,
    flags: u8,
    _padding: [u8; 3],
    _layout: u32,
    justification: f32,
    end_seconds: f32,
}
// 0056cc70 forwards line+16 to 0056c8e0: two FON alignment selectors
// followed by X/Y offsets. This is a drawing anchor, not a letter effect.
#[derive(BinRead)]
#[br(little)]
struct LineLayoutWire {
    alignment: [u32; 2],
    offset: [f32; 2],
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
/// The installed Manor/Astrolabe RTG header is 11 although only two groups follow.
/// Its header meaning is not established. Accept this evidenced profile only;
/// every populated language block must still have a complete relocation group.
fn language_relocations(
    bytes: &[u8],
    blocks: &BTreeMap<BlockKey, Block>,
    expected_header: u8,
) -> Result<BTreeMap<BlockKey, Vec<RelocRow>>> {
    ensure!(
        bytes.first().copied() == Some(expected_header),
        "unrecognized dialogue RTG header"
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
        "dialogue extraction requires evidenced English locale"
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
    let selection_font = resolve(16)?;
    let mut presentation =
        super::dialogue_presentation::import(s, game, normal_font, emphasis, selection_font)?;
    let sounds = SoundSites::open(game, selection, s)?;
    [presentation.cash_sound, presentation.scroll_sound] =
        super::dialogue_presentation::dialog_sounds(s, game, &sounds.events)?;
    let presentation_ref = AssetRef::from_path(presentation.path.clone());
    result.dialogue_presentation = Some(presentation);
    // +0 is the text-name table; +4 is the circular dialogue catalogue.
    let root = resolve(4)?;
    let mut list = root;
    let mut seen = BTreeSet::new();
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
            ItemImport::Supported(value) => result.dialogues.push(*value),
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
    Supported(Box<Dialogue>),
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
        let question = h.answers > 1 && h.box_style == 1;
        if !(h.answers == 1 || question) || !matches!(h.box_style, 1 | 3 | 4 | 5) {
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
        let answer_table = s.required(item, 4)?;
        let mut callback = None;
        let mut answers = Vec::new();
        if question {
            for index in 0..usize::from(h.answers) {
                let answer = s.required(answer_table, index * 4)?;
                let wire: AnswerWire = wire(s.bytes(answer, 80)?)?;
                // Only plain choices: no cost/AnswerBad, state or channel
                // effects, icon, chained item or substitute font.
                let font = s.pointer(answer, 12)?;
                let mut unsupported = Vec::new();
                if wire.stock_lists != [0; 2] {
                    unsupported.push("inventory cost");
                }
                if wire.states != [0; 2] {
                    unsupported.push("state");
                }
                if wire.channels != [0; 2] {
                    unsupported.push("channel");
                }
                if wire.icon != -2 {
                    unsupported.push("icon");
                }
                if font.is_some_and(|font| font != self.fonts[0]) {
                    unsupported.push("font");
                }
                if !unsupported.is_empty() {
                    return Ok(ItemImport::Unsupported(format!(
                        "{name}: answer {index} {} behavior is not represented",
                        unsupported.join("/")
                    )));
                }
                let lines = match self.lines(s.pointer(answer, 40)?, &name, h.box_style)? {
                    Ok(lines) if !lines.is_empty() => lines,
                    Ok(_) => {
                        return Ok(ItemImport::Unsupported(format!(
                            "{name}: textless answer {index} in a question is not represented"
                        )));
                    }
                    Err(reason) => return Ok(ItemImport::Unsupported(reason)),
                };
                let completion = match self.callback(answer, actor, &owner, &name)? {
                    Ok(value) => value,
                    Err(reason) => return Ok(ItemImport::Unsupported(reason)),
                };
                let item_name =
                    |address: Address| -> Result<String> { string(s, s.required(address, 24)?) };
                let next = s.pointer(answer, 24)?.map(item_name).transpose()?;
                let count = wire.result_mapping[0] as usize;
                ensure!(count <= 16, "{name}: excessive AnswerBad mapping");
                let mut shortfall = Vec::new();
                if count > 0 {
                    let table = s.required(answer, 68)?;
                    for entry in 0..count {
                        let code = s.bytes(add(table, entry * 8), 1)?[0];
                        shortfall.push((code, item_name(s.required(table, entry * 8 + 4)?)?));
                    }
                }
                answers.push(DialogueAnswer {
                    lines,
                    completion,
                    voice: self.voice(s.pointer(answer, 60)?)?,
                    one_shot: wire.one_shot != 0,
                    gold: wire.gold,
                    next,
                    shortfall,
                });
            }
        } else {
            let answer = s.required(answer_table, 0)?;
            let answer_wire: AnswerWire = wire(s.bytes(answer, 80)?)?;
            if answer_wire.next != 0 || answer_wire.text != 0 || answer_wire.icon != -2 {
                return Ok(ItemImport::Unsupported(format!(
                    "{name}: answer text, branching or icon behavior is not represented"
                )));
            }
            if h.box_style == 5
                && (answer_wire.stock_lists != [0; 2]
                    || answer_wire.states != [0; 2]
                    || answer_wire.channels != [0; 2]
                    || answer_wire._voice != 0
                    || answer_wire.result_mapping != [0; 2]
                    || answer_wire.gold != 0)
            {
                return Ok(ItemImport::Unsupported(format!(
                    "{name}: style5 answer side effects require additional dialogue semantics"
                )));
            }
            callback = match self.callback(answer, actor, &owner, &name)? {
                Ok(value) => value,
                Err(reason) => return Ok(ItemImport::Unsupported(reason)),
            };
        }
        let lines = match self.lines(s.pointer(item, 12)?, &name, h.box_style)? {
            Ok(lines) => lines,
            Err(reason) => return Ok(ItemImport::Unsupported(reason)),
        };
        if lines.is_empty() {
            return Ok(ItemImport::Unsupported(format!(
                "{name}: textless dialogue behavior is not represented"
            )));
        }
        if question {
            // 0056cb10 appends the answers to the last page's rows (nothing
            // guards a full page). A finite deadline there would let the
            // speech clock select answer 0 (0056c2d0 mode 1 -> 0056c690).
            let last = (lines.len() - 1) % 6 + 1;
            let times: Vec<f32> = lines[lines.len() - last..]
                .iter()
                .map(|l| l.end_seconds)
                .collect();
            if times.iter().any(|t| *t < 3.0e38) {
                return Ok(ItemImport::Unsupported(format!(
                    "{name}: timed question page (times {times:?}) is not represented"
                )));
            }
        }
        let voice = self.voice(s.pointer(item, 48)?)?;
        // Style 5 pages at six rows, null padding included (dialogue_item.gd):
        // pickup messages such as potion_bleue run to two pages.
        if h.box_style == 5
            && (voice.is_some()
                || lines.iter().any(|line| {
                    line.flags != 3
                        || line.layout.is_some()
                        || line.end_seconds.to_bits() != 0x7f7fc99e
                }))
        {
            return Ok(ItemImport::Unsupported(format!(
                "{name}: style5 requires unvoiced pages of flags3, unanchored, untimed lines"
            )));
        }
        if h.box_style == 4
            && (lines.len() > 2
                || voice.is_some()
                || lines.iter().any(|line| line.end_seconds < 3.0e38))
        {
            return Ok(ItemImport::Unsupported(format!(
                "{name}: style4 requires at most two untimed unflagged lines and no voice; observed lines={}, voice={}, completion={}, times={:?}",
                lines.len(),
                voice.is_some(),
                callback.is_some(),
                lines
                    .iter()
                    .map(|line| line.end_seconds)
                    .collect::<Vec<_>>()
            )));
        }
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
            answers,
        };
        value.validate()?;
        Ok(ItemImport::Supported(Box::new(value)))
    }

    fn voice(&self, sound: Option<Address>) -> Result<Option<SoundEventLink>> {
        Ok(sound
            .map(|a| {
                self.sounds
                    .events
                    .get(&add(a, 12))
                    .cloned()
                    .context("dialogue voice lacks localized SND binding")
            })
            .transpose()?
            .flatten())
    }

    /// Answer+0x1c: the owner's normal behavior stored as the pending callback.
    fn callback(
        &self,
        answer: Address,
        actor: Address,
        owner: &AssetRef<Perso>,
        name: &str,
    ) -> Result<std::result::Result<Option<BehaviorRef>, String>> {
        let s = self.source;
        let Some(target) = s.pointer(answer, 28)? else {
            return Ok(Ok(None));
        };
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
            return Ok(Err(format!(
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
        Ok(Ok(Some(reference)))
    }

    /// An ItemText or AnswerText line list.
    fn lines(
        &self,
        head: Option<Address>,
        name: &str,
        box_style: u8,
    ) -> Result<std::result::Result<Vec<DialogueLine>, String>> {
        let s = self.source;
        let mut lines = Vec::new();
        let mut line = head;
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
            if h.flags != 0 && !(box_style == 5 && h.flags == 3) {
                return Ok(Err(format!(
                    "{name}: dialogue line {} requires additional behavior (flags {:#04x})",
                    h.ordinal, h.flags
                )));
            }
            let layout = s
                .pointer(a, 16)?
                .map(|address| -> Result<DialogueLineLayout> {
                    let value: LineLayoutWire = wire(s.bytes(address, 16)?)?;
                    Ok(DialogueLineLayout {
                        alignment: value.alignment,
                        offset: value.offset,
                    })
                })
                .transpose()?;
            if let Some(layout) = &layout
                && (!matches!(box_style, 1 | 4) || layout.alignment.iter().any(|v| *v > 2))
            {
                return Ok(Err(format!(
                    "{name}: unsupported line drawing layout {layout:?}"
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
                    return Ok(Err(format!(
                        "{name}: dialogue span uses an unpromoted font"
                    )));
                }
                spans.push(DialogueSpan {
                    text: s.pointer(a, 4)?.map(|a| string(s, a)).transpose()?,
                    offset: h.offset,
                    emphasis: font == self.fonts[1],
                });
                span = s.pointer(a, 0)?;
            }
            lines.push(DialogueLine {
                layout,
                spans,
                justification: h.justification,
                end_seconds: h.end_seconds,
                flags: h.flags,
            });
            line = s.pointer(a, 0)?;
        }
        Ok(Ok(lines))
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
