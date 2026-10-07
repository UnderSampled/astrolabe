//! Source-derived Hype gameplay HUD presentation.
use super::{AssetPath, ai::SourceObjectIdentity, io::Content};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

/// Editable layout data for the shared Fix gameplay HUD.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HudPresentation {
    pub path: AssetPath,
    pub canvas_size: [u16; 2],
    /// Four source panel descriptors, in original draw order.
    pub panels: Vec<HudPanel>,
    /// Six relocated Fix.gpt texture handles in read order. FUN_0040bcc0 assigns
    /// them to globals 0060f62c, 0060f630, 0060f620, 0060f624, 0060f628, 0060f63c.
    /// Draw uses include life slots 2, 4, 3; 0040c720 slots 1, 0; and 0040c950 slot 5.
    pub source_textures: [Option<SourceObjectIdentity>; 6],
    /// The two 24-entry Fix.sda tables. The six floats in each row are retained
    /// as stored; draw functions assign meaning to the fields they use.
    pub rectangle_tables: [Vec<[f32; 6]>; 2],
    /// The two uninterpreted source words preceding the documented text offsets.
    pub source_words: [u32; 2],
    /// Text offsets stored in Fix.sda; source order is preserved.
    pub text_offsets: [[f32; 2]; 2],
    /// Draw routines identify these useful placements in the second table.
    pub elements: Vec<HudElementLayout>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HudPanel {
    pub enabled: bool,
    /// Source layer-array allocation, including for panels disabled in this profile.
    pub source_layers: Option<SourceObjectIdentity>,
    /// The source graphics material handle used by FUN_0040c270.
    pub material: Option<SourceObjectIdentity>,
    /// The descriptor's otherwise-unused third word when it resolves to a source object.
    pub unknown_reference: Option<SourceObjectIdentity>,
    /// Original bytes when the third word is not a relocated address.
    pub unknown_word: Option<[u8; 4]>,
    /// Bytes after the enabled/count fields; FUN_0040c270 does not read them.
    pub source_unknown_bytes: [u8; 2],
    /// Layer count declared in the source descriptor.
    pub source_layer_count: u8,
    /// Ordered source draw layers, once their pointed-to records are decoded.
    pub layers: Vec<HudPanelLayer>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HudPanelLayer {
    /// Texture handle at layer offset +0, passed to GLI_xSetMaterialTexture.
    pub texture: Option<SourceObjectIdentity>,
    /// Raw handle bytes when the source has no relocation binding for the word.
    pub opaque_texture_word: Option<[u8; 4]>,
    /// Source destination coordinates: x0, x1, y0, y1.
    pub destination: [f32; 4],
    /// Source normalized UV coordinates: u0, u1, v0, v1.
    pub uv: [f32; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HudElementLayout {
    pub role: String,
    pub table: u8,
    pub entries: Vec<u8>,
}

impl Content for HudPresentation {
    const KIND: &'static str = "hud-presentation";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            self.canvas_size == [640, 480],
            "unsupported HUD source canvas"
        );
        ensure!(self.panels.len() == 4, "HUD requires four source panels");
        for panel in &self.panels {
            ensure!(
                panel.layers.len() <= panel.source_layer_count as usize,
                "HUD panel layer bound"
            );
            for layer in &panel.layers {
                ensure!(
                    layer.destination.iter().all(|v| v.is_finite())
                        && layer.uv.iter().all(|v| v.is_finite()),
                    "invalid HUD panel layer geometry {:?} / {:?}",
                    layer.destination,
                    layer.uv
                );
            }
        }
        for table in &self.rectangle_tables {
            ensure!(table.len() == 24, "HUD rectangle table count differs");
            for row in table {
                ensure!(
                    row.iter().all(|v| v.is_finite()),
                    "non-finite HUD layout row"
                );
            }
        }
        ensure!(
            self.text_offsets.iter().flatten().all(|v| v.is_finite()),
            "non-finite HUD text offset"
        );
        ensure!(
            self.elements.len() == 5
                && self.elements.iter().all(|element| {
                    element.table < 2
                        && !element.entries.is_empty()
                        && element.entries.iter().all(|index| *index < 24)
                }),
            "invalid HUD element layout map"
        );
        Ok(())
    }
}
