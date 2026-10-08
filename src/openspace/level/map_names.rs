//! Localized map-name state from the finite Fix loader path, not actor visibility.
use super::{
    container::{file, pointer_file, u32_at, wire},
    records::*,
    session::SourceSession,
};
use crate::model::scene::MapName;
use anyhow::{Context, Result, ensure};
use binrw::BinRead;
use std::{collections::BTreeSet, path::Path};

/// 00416800 allocates 60 bytes. 00416560 fills the five authored fields;
/// 00416310 compares +0x30 and changes +0x38 on every matching record.
#[derive(BinRead)]
#[br(little)]
struct NameWire {
    label: [u8; 40],
    position: [i32; 2],
    id: i32,
    unknown: i32,
    active: u32,
}

pub(super) fn import(s: &SourceSession, game: &Path) -> Result<Vec<MapName>> {
    let source = game.join("Gamedata/World/Levels");
    let lang = game.join("LangData/English/world/levels");
    let gpt = file(&source.join("Fix.gpt"))?;
    let sda = file(&source.join("Fix.sda"))?;
    let rtp = pointer_file(&file(&source.join("Fix.rtp"))?)?;
    let dlg = file(&lang.join("Fix.dlg"))?;
    let rtd = pointer_file(&file(&lang.join("Fix.rtd"))?)?;
    // 00401e00: FONGeneral consumes one root plus SDA+8 localized words.
    // 0041ea40 consumes one additional localized root. 004a2340 then consumes
    // one word per multilingual list node; 00416420 consumes 60 map names next.
    let font_count = u32_at(&sda, 8)? as usize;
    ensure!(
        font_count == 409,
        "unsupported Fix font count for name table"
    );
    let mut cursor = (2 + font_count) * 4;
    let word = u32_at(&gpt, 18124)?;
    let mut next = Some(s.resolve(
        Owner::Fix,
        *rtp.get(&word).context("map-name prerequisite lacks RTP")?,
        word,
    )?);
    let mut seen = BTreeSet::new();
    while let Some(node) = next {
        ensure!(
            seen.insert(node) && seen.len() <= 256,
            "map-name prerequisite list cycle/bound"
        );
        // The registered destination is typed as a pointer site; do not scan allocations.
        s.required(node, 0)?;
        next = s.pointer(node, 4)?;
        cursor += 4;
    }
    let mut names = Vec::new();
    for index in 0..60 {
        let word = u32_at(&dlg, cursor + index * 4)?;
        let address = s.resolve(
            Owner::Fix,
            *rtd.get(&word).context("map name lacks RTD")?,
            word,
        )?;
        let bytes = s.allocation_bytes(address)?;
        ensure!(
            bytes.len() == 60,
            "map name {index} record size differs: {} at {address:?}, DLG cursor {cursor}, {} inventory registrations",
            bytes.len(),
            seen.len()
        );
        let value: NameWire = wire(bytes)?;
        ensure!(value.active <= 1, "map name activation is not Boolean");
        let end = value
            .label
            .iter()
            .position(|&b| b == 0)
            .context("map name label lacks terminator")?;
        names.push(MapName {
            source: s.source_identity(address)?,
            label: super::dialogue::decode_cp1252(&value.label[..end])?,
            label_tail: value.label[end + 1..].to_vec(),
            position: value.position,
            id: value.id,
            unknown: value.unknown,
            active: value.active != 0,
        });
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires supplied English Hype files"]
    fn local_name_table_follows_fix_loader_order() -> Result<()> {
        let game = std::env::var_os("ASTROLABE_GAME_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "disc".into());
        let selection = crate::model::Selection::hype("manoir", "en")?;
        let mut session = SourceSession::open(&game, &selection)?;
        super::super::dialogue::load_language(&mut session, &game, &selection)?;
        let names = import(&session, &game)?;
        assert_eq!(names.len(), 60);
        assert!(names.iter().any(|n| n.id != 0 && !n.label.is_empty()));
        assert!(names.iter().any(|n| n.label.is_empty()));
        assert_eq!(
            names
                .iter()
                .map(|n| (n.source.block, n.source.allocation))
                .collect::<BTreeSet<_>>()
                .len(),
            60
        );
        assert!(names.iter().all(|n| !n.active));
        let encoded = serde_json::to_string(&names)?;
        assert!(!encoded.contains("address") && !encoded.contains("offset"));
        Ok(())
    }
}
