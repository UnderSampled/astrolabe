//! Armor roots and declarative records corroborated by the pinned Hype engine.
use super::{
    container::{file, pointer_file, u32_at, wire},
    records::*,
    session::SourceSession,
};
use crate::model::{AssetPath, AssetRef, Selection, armor::*, io::Content};
use anyhow::{Context, Result, ensure};
use binrw::BinRead;
use std::{collections::BTreeSet, path::Path};

/// 0043ad40 creates 40-byte definitions; 0043aa70 consumes their fields.
#[derive(BinRead)]
#[br(little)]
struct DefinitionWire {
    _next: u32,
    _artwork: [u32; 3],
    _pieces: u32,
    _factors: u32,
    thresholds: [u8; 5],
    piece_count: u8,
    broken_count: u8,
    rank: u8,
    minimum_damage: u8,
    unknown: [u8; 3],
    _current_piece: u32,
}
/// 0043adff allocates 24 bytes; 0043ab40 reads rectangle/artwork selection.
#[derive(BinRead)]
#[br(little)]
struct PieceWire {
    _next: u32,
    rectangle: [f32; 4],
    broken: u8,
    strikes: u8,
    unknown: [u8; 2],
}
fn record_bytes<'a>(
    s: &'a SourceSession,
    address: Address,
    size: usize,
    kind: &str,
) -> Result<&'a [u8]> {
    let bytes = s
        .allocation_bytes(address)
        .with_context(|| format!("armor {kind} at {address:?}"))?;
    ensure!(
        bytes.len() == size,
        "armor {kind} at {address:?} has {} bytes; expected {size}",
        bytes.len()
    );
    Ok(bytes)
}
fn artwork(s: &SourceSession, address: Address) -> Result<ArmorArtwork> {
    // 00486400 returns TextureInfo; 00482e60 reads its logical width/height.
    let value: TextureInfoWire = wire(record_bytes(s, address, 188, "artwork")?)?;
    let name = std::str::from_utf8(value.name.split(|b| *b == 0).next().unwrap())?
        .replace('\\', "/")
        .to_ascii_lowercase();
    Ok(ArmorArtwork {
        texture: AssetRef::new(format!("texture:/{name}"))?,
        size: [value.logical_width, value.logical_height],
        color_key: value.flags & 2 != 0,
    })
}
pub(crate) fn import(game: &Path, selection: &Selection) -> Result<ArmorCatalog> {
    let s = SourceSession::open(game, selection)?;
    let folder = game.join("Gamedata/World/Levels");
    let gpt = file(&folder.join("Fix.gpt"))?;
    let scalar = file(&folder.join("Fix.sda"))?;
    let refs = pointer_file(&file(&folder.join("Fix.rtp"))?)?;
    // Inventory's catalogue at GPT18152 is the SEVENTH pointer read in
    // 0041c290 (0041c2f9), so that reader starts at GPT18128. Its whole
    // declaration is 72 bytes; counting those from the catalogue is 24 late.
    // 004e5150 consumes 24+6*n pointer words; its count follows inventory's
    // 384 scalar bytes and 00416420's four scalar bytes, starting at SDA20.
    let count_bytes: [u8; 2] = scalar
        .get(408..410)
        .context("armor prerequisite count missing")?
        .try_into()?;
    let preceding_count = i16::from_le_bytes(count_bytes);
    ensure!(
        (0..=1024).contains(&preceding_count),
        "armor prerequisite count bound"
    );
    let root_slot = 18128 + 72 + 448 + (96 + preceding_count as usize * 24) + 8 + 16 + 136 + 140;
    let count = *scalar.get(3660).context("armor catalog count missing")? as usize;
    let value = u32_at(&gpt, root_slot)?;
    let mut next = if value == 0 {
        None
    } else {
        Some(
            s.resolve(
                Owner::Fix,
                *refs
                    .get(&value)
                    .context("armor root lacks Fix RTP binding")?,
                value,
            )?,
        )
    };
    let mut definitions = Vec::new();
    let mut seen = BTreeSet::new();
    while let Some(address) = next {
        ensure!(
            seen.insert(address) && definitions.len() < count,
            "armor list cycle/count"
        );
        let value: DefinitionWire = record_bytes(&s, address, 40, "definition")
            .and_then(wire)
            .with_context(|| format!("armor definition from Fix.gpt slot {root_slot}"))?;
        let artwork = [4, 8, 12].map(|offset| {
            s.pointer(address, offset)
                .and_then(|p| p.map(|a| self::artwork(&s, a)).transpose())
        });
        let [a, b, c] = artwork;
        let artwork = [a?, b?, c?];
        let mut pieces = Vec::new();
        let mut addresses = Vec::new();
        let mut part = s.pointer(address, 16)?;
        let mut seen_parts = BTreeSet::new();
        while let Some(a) = part {
            ensure!(
                seen_parts.insert(a) && pieces.len() < value.piece_count as usize,
                "armor piece list cycle/count"
            );
            let p: PieceWire = wire(record_bytes(&s, a, 24, "piece")?)?;
            addresses.push(a);
            pieces.push(ArmorPiece {
                source: s.source_identity(a)?,
                rectangle: p.rectangle,
                broken: p.broken,
                strikes: p.strikes,
                unknown: p.unknown,
            });
            part = s.pointer(a, 0)?;
        }
        ensure!(
            pieces.len() == value.piece_count as usize,
            "armor piece count differs"
        );
        let selected = s.pointer(address, 36)?;
        let current_damage_piece = selected
            .map(|a| {
                addresses
                    .iter()
                    .position(|p| *p == a)
                    .context("armor selected damage piece leaves definition")
                    .map(|n| n as u8)
            })
            .transpose()?;
        let factors = s.required(address, 20)?;
        let columns = pieces.len() + 1;
        let bytes = record_bytes(&s, factors, 5 * columns * 4, "damage factors")?;
        let damage_factors = bytes
            .chunks_exact(columns * 4)
            .map(|row| {
                row.chunks_exact(4)
                    .map(|word| f32::from_le_bytes(word.try_into().unwrap()))
                    .collect()
            })
            .collect();
        definitions.push(ArmorDefinition {
            source: s.source_identity(address)?,
            artwork,
            pieces,
            damage_factors,
            break_thresholds: value.thresholds,
            minimum_damage: value.minimum_damage,
            broken_count: value.broken_count,
            resistance_rank: value.rank,
            current_damage_piece,
            unknown: value.unknown,
        });
        next = s.pointer(address, 0)?;
    }
    ensure!(definitions.len() == count, "armor catalog count differs");
    let catalog = ArmorCatalog {
        path: AssetPath::new("fix:/gameplay/armor")?,
        selected: (!definitions.is_empty()).then_some(0),
        definitions,
    };
    catalog.validate()?;
    Ok(catalog)
}
