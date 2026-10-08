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
    pub category_scale: [f32; 2],
    pub category_color: [[f32; 4]; 2],
    /// Ordinary, equipped, current item scales and ordinary/current colors.
    pub item_scale: [f32; 3],
    pub item_color: [[f32; 4]; 2],
    /// Source `BoxesDistance` and `BoxesScrollingSpeed`, used by the vertical
    /// category-box transition. Speed is measured per millisecond.
    pub category_motion: InventoryMotion,
    /// Source `CasesDistance` and `CasesScrollingSpeed`, used by the horizontal
    /// six-slot item-row transition. Speed is measured per millisecond.
    pub item_motion: InventoryMotion,
    pub counter_offset: [f32; 2],
    /// Fix.sda inventory text origin retained from the source presentation wire.
    pub text_position: [f32; 2],
    /// Fix.sda inventory stats origin retained even though its gauge layout is not yet known.
    pub stats_position: [f32; 2],
    pub description_position: [f32; 2],
    pub detail_position: [f32; 2],
    pub detail_scale: f32,
    pub font: InventoryFont,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InventoryMotion {
    pub distance: f32,
    pub speed_per_millisecond: f32,
}
impl InventoryMotion {
    /// The source timer uses one tick per millisecond (0041b500/00502620).
    pub fn duration_seconds(self) -> f32 {
        self.distance / self.speed_per_millisecond / 1000.0
    }
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
                    .chain(&self.text_position)
                    .chain(&self.stats_position)
                    .chain(&self.description_position)
                    .chain(&self.detail_position)
                    .all(|n| n.is_finite()),
            "invalid inventory scale"
        );
        for motion in [self.category_motion, self.item_motion] {
            ensure!(
                motion.distance.is_finite()
                    && (f32::MIN_POSITIVE..=640.0).contains(&motion.distance)
                    && motion.speed_per_millisecond.is_finite()
                    && (f32::MIN_POSITIVE..=10.0).contains(&motion.speed_per_millisecond)
                    && motion.duration_seconds().is_finite()
                    && (0.0..=5.0).contains(&motion.duration_seconds()),
                "invalid inventory motion"
            );
        }
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
