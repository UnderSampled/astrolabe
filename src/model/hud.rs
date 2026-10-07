//! Legacy HUD presentation shape retained while source layout validation is pending.
use super::{AssetPath, ai::SourceObjectIdentity, io::Content};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

/// Unverified legacy layout data; retained only so old Rete assets can be inspected.
/// Every field preserves a former parser assumption and has no validated source meaning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HudPresentation {
    pub path: AssetPath,
    pub canvas_size: [u16; 2],
    /// Legacy field with unverified source meaning.
    pub panels: Vec<HudPanel>,
    /// Legacy field with unverified source meaning.
    pub source_textures: [Option<SourceObjectIdentity>; 6],
    /// Legacy field with unverified source meaning.
    pub rectangle_tables: [Vec<[f32; 6]>; 2],
    /// Legacy field with unverified source meaning.
    pub source_words: [u32; 2],
    /// Legacy field with unverified source meaning.
    pub text_offsets: [[f32; 2]; 2],
    /// Legacy field with unverified source meaning.
    pub elements: Vec<HudElementLayout>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HudPanel {
    pub enabled: bool,
    /// Legacy field with unverified source meaning.
    pub source_layers: Option<SourceObjectIdentity>,
    /// Legacy field with unverified source meaning.
    pub layer_material: Option<SourceObjectIdentity>,
    /// Legacy field with unverified source meaning.
    pub draw_material: Option<SourceObjectIdentity>,
    /// Legacy field with unverified source meaning.
    pub draw_material_word: Option<[u8; 4]>,
    /// Legacy field with unverified source meaning.
    pub source_unknown_bytes: [u8; 2],
    /// Legacy field with unverified source meaning.
    pub source_layer_count: u8,
    /// Legacy field with unverified source meaning.
    pub layers: Vec<HudPanelLayer>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HudPanelLayer {
    /// Legacy field with unverified source meaning.
    pub texture: Option<SourceObjectIdentity>,
    /// Legacy field with unverified source meaning.
    pub opaque_texture_word: Option<[u8; 4]>,
    /// Legacy field with unverified source meaning.
    pub destination: [f32; 4],
    /// Legacy field with unverified source meaning.
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
        bail!(
            "legacy HUD assets are inspection-only until the source reader and model are validated"
        )
    }
}
