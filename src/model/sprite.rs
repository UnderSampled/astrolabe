//! Shared camera-facing sprite definitions and their indexed placements in models.
use super::{AssetPath, AssetRef, geometry::Material, io::Content};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sprite {
    pub path: AssetPath,
    pub views: Vec<SpriteView>,
    pub unknown: u16,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpriteView {
    pub mode: u16,
    pub threshold: f32,
    pub half_size: [f32; 2],
    pub displacement: [f32; 2],
    pub material: Option<AssetRef<Material>>,
}
impl Content for Sprite {
    const KIND: &'static str = "sprite";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            !self.views.is_empty() && self.views.len() <= i16::MAX as usize,
            "invalid sprite view count"
        );
        for v in &self.views {
            ensure!(
                v.threshold.is_finite()
                    && v.half_size
                        .iter()
                        .chain(&v.displacement)
                        .all(|n| n.is_finite()),
                "nonfinite sprite definition"
            );
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpriteElement {
    pub source_index: Option<u16>,
    pub unknown: u16,
    pub sprites: Vec<IndexedSprite>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexedSprite {
    pub center: u16,
    pub unknown: u16,
    pub sprite: AssetRef<Sprite>,
    pub size: [f32; 2],
    pub constraint: [f32; 3],
    /// Source UV rectangle: origin followed by extent, not opposite corners.
    pub uv_origin_extent: [[f32; 2]; 2],
}
