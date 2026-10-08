//! Editable inventory presentation; gameplay quantities remain actor-owned state.
use super::{
    AssetPath, AssetRef,
    dialogue::{DialogueGlyph, DialogueTexture},
    io::Content,
    texture::Texture,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryPresentation {
    pub path: AssetPath,
    pub locale: String,
    pub background: AssetRef<Texture>,
    pub initial_category: String,
    pub categories: Vec<InventoryCategory>,
    pub slots: Vec<InventorySlot>,
    pub items: Vec<InventoryItem>,
    /// First catalogue sprite list for each HUD item identity, including items
    /// outside selectable categories (Proc421 / 00418f60 / 0041e020).
    #[serde(default)]
    pub pickup_artwork: Vec<PickupArtwork>,
    pub category_scale: [f32; 2],
    pub category_color: [[f32; 4]; 2],
    /// Ordinary, equipped, current item scales and ordinary/current colors.
    pub item_scale: [f32; 3],
    pub item_color: [[f32; 4]; 2],
    pub counter_offset: [f32; 2],
    pub description_position: [f32; 2],
    pub detail_position: [f32; 2],
    pub detail_scale: f32,
    pub font: InventoryFont,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PickupArtwork {
    pub id: i32,
    pub images: Vec<InventoryImage>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryFont {
    pub name: String,
    pub atlas: DialogueTexture,
    pub glyphs: Vec<DialogueGlyph>,
    pub tracking: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryImage {
    pub texture: AssetRef<Texture>,
    pub size: [f32; 2],
    pub color_key: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryCategory {
    pub key: String,
    pub parent: Option<String>,
    /// Children have already resolved angle/radius into this relative pixel position.
    pub position: [f32; 2],
    pub neighbors: [Option<String>; 4],
    pub images: Vec<InventoryImage>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventorySlot {
    pub position: [f32; 2],
    pub neighbors: [Option<u32>; 4],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryItem {
    pub id: i32,
    pub kind: String,
    pub label: String,
    pub category: String,
    pub images: Vec<InventoryImage>,
    pub draw_counter: bool,
    pub draw_when_zero: bool,
    pub available: u8,
}
impl Content for InventoryPresentation {
    const KIND: &'static str = "inventory-presentation";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            self.locale == "en" && self.background.path.role() == super::PackageRole::Texture,
            "unsupported inventory locale/background"
        );
        ensure!(
            (1..=32).contains(&self.categories.len())
                && (1..=32).contains(&self.slots.len())
                && (1..=256).contains(&self.items.len()),
            "invalid inventory presentation counts"
        );
        ensure!(
            self.pickup_artwork.len() <= 256,
            "too many pickup artwork entries"
        );
        let artwork_ids: std::collections::BTreeSet<_> =
            self.pickup_artwork.iter().map(|value| value.id).collect();
        ensure!(
            artwork_ids.len() == self.pickup_artwork.len(),
            "duplicate pickup artwork identity"
        );
        for artwork in &self.pickup_artwork {
            ensure!(
                (0..=32767).contains(&artwork.id) && (1..=16).contains(&artwork.images.len()),
                "invalid pickup artwork"
            );
            for image in &artwork.images {
                ensure!(
                    image.texture.path.member().is_none()
                        && image.size.iter().all(|v| v.is_finite() && *v > 0.),
                    "invalid pickup image"
                );
            }
        }
        let keys: std::collections::BTreeSet<_> =
            self.categories.iter().map(|c| c.key.as_str()).collect();
        ensure!(
            keys.len() == self.categories.len() && keys.contains(self.initial_category.as_str()),
            "invalid inventory category identity"
        );
        for c in &self.categories {
            ensure!(
                c.key
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    && c.position.iter().all(|n| n.is_finite())
                    && c.parent
                        .as_ref()
                        .is_none_or(|p| p != &c.key && keys.contains(p.as_str()))
                    && c.neighbors
                        .iter()
                        .flatten()
                        .all(|p| keys.contains(p.as_str())),
                "invalid inventory category layout"
            );
            ensure!(
                c.parent.as_ref().is_none_or(|p| self
                    .categories
                    .iter()
                    .find(|x| &x.key == p)
                    .is_some_and(|x| x.parent.is_none())),
                "inventory category hierarchy must have one child level"
            );
            images(&c.images)?;
        }
        for s in &self.slots {
            ensure!(
                s.position.iter().all(|n| n.is_finite())
                    && s.neighbors
                        .iter()
                        .flatten()
                        .all(|n| (*n as usize) < self.slots.len()),
                "invalid inventory slot"
            );
        }
        let mut ids = std::collections::BTreeSet::new();
        for i in &self.items {
            ensure!(
                ids.insert(i.id)
                    && (0..=32767).contains(&i.id)
                    && !i.label.is_empty()
                    && i.label.len() <= 4096
                    && keys.contains(i.category.as_str()),
                "invalid inventory item identity"
            );
            images(&i.images)?;
        }
        ensure!(
            self.category_scale
                .iter()
                .chain(self.item_scale.iter())
                .all(|n| n.is_finite() && *n > 0. && *n <= 4.)
                && self.detail_scale.is_finite()
                && self.detail_scale > 0.
                && self
                    .counter_offset
                    .iter()
                    .chain(&self.description_position)
                    .chain(&self.detail_position)
                    .all(|n| n.is_finite()),
            "invalid inventory scale"
        );
        ensure!(
            self.category_color
                .iter()
                .chain(self.item_color.iter())
                .flatten()
                .all(|n| n.is_finite() && (0.0..=1.0).contains(n)),
            "invalid inventory color"
        );
        ensure!(
            !self.font.name.is_empty()
                && (10..=1024).contains(&self.font.glyphs.len())
                && self.font.atlas.texture.role() == super::PackageRole::Texture
                && self.font.atlas.size.iter().all(|n| (2..=4096).contains(n))
                && self.font.tracking.is_finite()
                && self.font.tracking >= 0.,
            "invalid inventory font"
        );
        let mut chars = std::collections::BTreeSet::new();
        for glyph in &self.font.glyphs {
            ensure!(
                chars.insert(glyph.character)
                    && [glyph.width, glyph.half_height, glyph.vertical_offset]
                        .iter()
                        .all(|n| n.is_finite())
                    && glyph.width >= 0.
                    && glyph.half_height <= 0.,
                "invalid inventory glyph"
            );
            if let Some(uv) = glyph.uv {
                ensure!(
                    glyph.width > 0.
                        && glyph.half_height < 0.
                        && uv
                            .iter()
                            .flatten()
                            .all(|n| n.is_finite() && (0.0..=1.0).contains(n))
                        && uv[1].iter().all(|n| *n > 0.),
                    "invalid inventory glyph UV"
                );
                ensure!(
                    (0..2).all(|n| uv[0][n] + uv[1][n]
                        <= 1. + 1. / (self.font.atlas.size[n] - 1) as f32 + 0.00001),
                    "inventory glyph exceeds atlas"
                );
            }
        }
        for code in '0'..='9' {
            ensure!(
                self.font
                    .glyphs
                    .iter()
                    .any(|g| g.character == code && g.uv.is_some()),
                "missing inventory counter glyph"
            );
        }
        Ok(())
    }
}
fn images(values: &[InventoryImage]) -> Result<()> {
    ensure!(
        !values.is_empty() && values.len() <= 16,
        "invalid inventory image layers"
    );
    for v in values {
        ensure!(
            v.texture.path.role() == super::PackageRole::Texture
                && v.size
                    .iter()
                    .all(|n| n.is_finite() && *n > 0. && *n <= 640.),
            "invalid inventory image"
        );
    }
    Ok(())
}
