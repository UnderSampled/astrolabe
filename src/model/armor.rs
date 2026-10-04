//! Shared armor definitions and their initial mutable gameplay state.
use super::{
    AssetPath, AssetRef, PackageRole,
    ai::{SourceObjectIdentity, SourceRole},
    io::Content,
    texture::Texture,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

/// Fight_ChangeArmor selects from this ordered Fix-owned catalog, globally.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmorCatalog {
    pub path: AssetPath,
    pub definitions: Vec<ArmorDefinition>,
    pub selected: Option<u8>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmorDefinition {
    pub source: SourceObjectIdentity,
    /// Intact, broken, and optional background artwork, in source order.
    pub artwork: [Option<ArmorArtwork>; 3],
    pub pieces: Vec<ArmorPiece>,
    /// Five resistance ranks, each indexed by the number of broken pieces.
    pub damage_factors: Vec<Vec<f32>>,
    pub break_thresholds: [u8; 5],
    pub minimum_damage: u8,
    pub broken_count: u8,
    pub resistance_rank: u8,
    pub current_damage_piece: Option<u8>,
    pub unknown: [u8; 3],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmorArtwork {
    pub texture: AssetRef<Texture>,
    pub size: [u32; 2],
    pub color_key: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmorPiece {
    pub source: SourceObjectIdentity,
    /// Source normalized rectangle endpoints: u0, v0, u1, v1.
    /// V increases upward in the engine; target presentation may flip it.
    pub rectangle: [f32; 4],
    pub broken: u8,
    pub strikes: u8,
    pub unknown: [u8; 2],
}
impl Content for ArmorCatalog {
    const KIND: &'static str = "armor-catalog";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            self.path.role() == PackageRole::Fix && self.definitions.len() <= 255,
            "invalid armor catalog ownership/count"
        );
        ensure!(
            self.selected
                .is_none_or(|n| (n as usize) < self.definitions.len()),
            "armor selection leaves catalog"
        );
        let mut identities = Vec::new();
        let mut pieces = Vec::new();
        for definition in &self.definitions {
            ensure!(
                definition.source.role == SourceRole::Fix,
                "armor definition must belong to Fix"
            );
            ensure!(
                !identities.contains(&definition.source),
                "duplicate armor identity"
            );
            identities.push(definition.source.clone());
            let count = definition.pieces.len();
            ensure!(
                count <= 255
                    && definition.resistance_rank < 5
                    && definition.broken_count as usize <= count,
                "invalid armor state"
            );
            ensure!(
                definition.broken_count as usize
                    == definition.pieces.iter().filter(|p| p.broken == 1).count(),
                "armor broken count differs from piece state"
            );
            ensure!(
                definition
                    .current_damage_piece
                    .is_none_or(|n| (n as usize) < count),
                "armor damage selection leaves pieces"
            );
            ensure!(
                definition.damage_factors.len() == 5
                    && definition
                        .damage_factors
                        .iter()
                        .all(|row| row.len() == count + 1 && row.iter().all(|n| n.is_finite())),
                "invalid armor damage factor matrix"
            );
            ensure!(
                definition.artwork[0].is_some() && definition.artwork[1].is_some(),
                "armor requires intact and broken artwork"
            );
            for art in definition.artwork.iter().flatten() {
                ensure!(
                    art.texture.path.role() == PackageRole::Texture
                        && art.size.iter().all(|n| *n > 0),
                    "invalid armor artwork"
                );
            }
            for piece in &definition.pieces {
                ensure!(
                    piece.source.role == SourceRole::Fix,
                    "armor piece must belong to Fix"
                );
                ensure!(
                    !pieces.contains(&piece.source)
                        && piece.rectangle.iter().all(|n| n.is_finite())
                        && piece.broken <= 1,
                    "invalid or multiply owned armor piece"
                );
                pieces.push(piece.source.clone());
            }
        }
        Ok(())
    }
}
