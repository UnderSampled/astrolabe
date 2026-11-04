//! Source-verified Fix HUD declarations and direct-draw inputs for the Hype build.
use super::{
    AssetPath,
    ai::SourceObjectIdentity,
    dialogue::{DialogueGlyph, DialogueTexture},
    io::Content,
};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

/// Source-verified Fix HUD declarations, image identities, and direct-draw inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HudPresentation {
    pub path: AssetPath,
    pub canvas_size: [u16; 2],
    /// Source reader contract version. Missing means the legacy inspection-only schema.
    #[serde(default)]
    pub source_layout_version: u8,
    /// Four Fix.gpt panel descriptors, in source draw order.
    pub panels: Vec<HudPanel>,
    /// The six GLI texture objects loaded by FUN_0040bcc0, in read order.
    pub source_textures: [Option<SourceObjectIdentity>; 6],
    /// Source identity, Fix.ptx order, original name and archive asset for each
    /// GLI texture object. Empty in legacy Rete values.
    #[serde(default)]
    pub artwork: Vec<HudArtwork>,
    #[serde(default)]
    pub magic: Option<HudMagic>,
    /// The dedicated Fix inventory object read by FUN_0041d400.
    #[serde(default)]
    pub currency_source_object: Option<SourceObjectIdentity>,
    /// Source ItemWire.id for the dedicated currency element.
    #[serde(default)]
    pub currency_item_id: Option<i32>,
    /// FON's named InventoryFont used by the HUD stats text handle.
    #[serde(default)]
    pub currency_font: Option<HudCounterFont>,
    /// The laser pointer's texture when no Proc_SetLaserPointerTexture names
    /// one: the engine looks up "ViseurInitial" (0x421759), live the Fix
    /// texture fixtex/gen_viseur/laserpointer.tga.
    #[serde(default)]
    pub laser_pointer_texture: Option<AssetPath>,
    /// Fix.sda's default and active 24-entry screen rectangles.
    pub rectangle_tables: [Vec<[f32; 6]>; 2],
    /// The two source thresholds read before the layout tables.
    pub source_words: [u32; 2],
    /// Offsets used by the source's HUD counter and selected-item text calls.
    pub text_offsets: [[f32; 2]; 2],
    /// Layout row uses verified against the direct-draw routines.
    pub elements: Vec<HudElementLayout>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HudCounterFont {
    pub source_object: SourceObjectIdentity,
    pub name: String,
    pub atlas: DialogueTexture,
    pub glyphs: Vec<DialogueGlyph>,
    pub tracking: f32,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HudTextureRole {
    EnemyMeterFill,
    EnemyMeterBackground,
    PlayerLifeHandle,
    PlayerLifeRemainder,
    PlayerLifeFill,
    CurrencyIcon,
    /// Fix.gpt HUD +0x70 (DAT_00610b8c): overlay drawn over unselected spell slots.
    MagicSlotOverlay,
    /// Fix.gpt HUD +0x78 (DAT_00610b84): texture of the magic-points arc object.
    MagicPointsArc,
}

/// Spell slots and magic-points arc (0043a450, 0043a6a0, 00439e50).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HudMagic {
    pub slot_overlay: HudArtwork,
    pub points_arc: HudArtwork,
    /// Inventory item kinds of the three slots ("Magic" + the b98 suffix).
    pub slot_kinds: [String; 3],
    /// Fix.sda active-table rows of the three slots (b98 +0x1e..+0x20).
    pub slot_rows: [u8; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HudArtwork {
    /// Zero-based Fix.gpt HUD texture handle slot.
    pub slot: u8,
    pub role: HudTextureRole,
    /// GLI texture allocation identity; address and relocation are transient.
    pub source_object: SourceObjectIdentity,
    /// Index in the Fix.ptx pointer table.
    pub fix_ptx_index: u16,
    /// Exact GLI object name, including original separators, case and .tga suffix.
    pub source_name: String,
    /// Resolved member in Textures.cnt, retained as a useful texture dependency.
    pub texture: AssetPath,
    /// Stored width/height from the GLI object.
    pub stored_size: [u32; 2],
    /// Logical width/height from the GLI object.
    pub logical_size: [u32; 2],
    pub source_flags: u32,
    pub sample_flags: u8,
    pub addressing: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HudPanel {
    pub enabled: bool,
    /// Relocated Fix.gpt +0 layer-array identity.
    pub source_layers: Option<SourceObjectIdentity>,
    /// Relocated Fix.gpt +4 material used by the panel layer draw procedure.
    pub layer_material: Option<SourceObjectIdentity>,
    /// Relocated Fix.gpt +8 material retained for the other panel draw procedure.
    pub draw_material: Option<SourceObjectIdentity>,
    /// Reserved for legacy inspection; pointer words are represented by identities.
    pub draw_material_word: Option<[u8; 4]>,
    /// Two descriptor bytes not assigned by the current consumers.
    pub source_unknown_bytes: [u8; 2],
    /// Number of 0x24-byte source layer records.
    pub source_layer_count: u8,
    /// All decoded 0x24-byte layer records; the static enabled flag is only the runtime draw gate.
    pub layers: Vec<HudPanelLayer>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HudPanelLayer {
    /// Relocated texture-object identity.
    pub texture: Option<SourceObjectIdentity>,
    /// Reserved for legacy inspection; pointer words are represented by identities.
    pub opaque_texture_word: Option<[u8; 4]>,
    /// Exact GLI texture name of the layer, if it has a texture.
    #[serde(default)]
    pub texture_name: Option<String>,
    /// Textures.cnt member for the layer texture.
    #[serde(default)]
    pub texture_asset: Option<AssetPath>,
    /// Logical width/height of the layer's GLI texture.
    #[serde(default)]
    pub texture_logical_size: [u32; 2],
    /// GLI source flags of the layer texture (bit 1 selects the color key, as for HUD artwork).
    #[serde(default)]
    pub texture_flags: u32,
    /// Screen rectangle in 640x480 space, ordered x0, x1, y0, y1 (FUN_0040c270 -> FUN_0047abb0).
    pub destination: [f32; 4],
    /// Texture coordinates, ordered u0, u1, v0, v1.
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
        if self.source_layout_version == 0 {
            bail!("legacy HUD assets are inspection-only; Fix source layout version 1 is required")
        }
        anyhow::ensure!(
            self.source_layout_version == 1,
            "unsupported Fix HUD source layout version"
        );
        anyhow::ensure!(self.canvas_size == [640, 480], "unsupported HUD canvas");
        anyhow::ensure!(
            self.panels.len() == 4,
            "HUD requires four Fix panel descriptors"
        );
        anyhow::ensure!(
            self.panels
                .iter()
                .map(|panel| panel.source_layer_count)
                .collect::<Vec<_>>()
                == [31, 7, 14, 4]
                && self.panels.iter().all(|panel| !panel.enabled
                    && panel.layers.len() == usize::from(panel.source_layer_count)),
            "Fix HUD panel descriptors differ from the verified profile"
        );
        anyhow::ensure!(
            self.artwork.len() == 6,
            "HUD requires six verified Fix texture objects"
        );
        anyhow::ensure!(
            self.currency_source_object
                .as_ref()
                .is_some_and(|identity| {
                    identity.role == super::ai::SourceRole::Fix && identity.block == [6, 1]
                }),
            "HUD currency source object is missing or not Fix-owned"
        );
        anyhow::ensure!(
            self.currency_item_id == Some(0),
            "HUD currency ItemWire identity is missing or out of range"
        );
        let font = self
            .currency_font
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("HUD source InventoryFont is missing"))?;
        anyhow::ensure!(
            font.source_object.role == super::ai::SourceRole::Fix
                && font.source_object.block == [19, 1]
                && font.name == "InventoryFont"
                && font.glyphs.len() == 127
                && font.atlas.texture.role() == super::PackageRole::Texture
                && matches!(
                    font.atlas.texture.file().to_ascii_lowercase().as_str(),
                    "fixtex/motortxt/lettrageb.tga" | "fixtex/motortxt/lettrageb.gf"
                )
                && font.atlas.size == [256, 256]
                && font.atlas.size.iter().all(|size| *size >= 2)
                && font.tracking.is_finite()
                && font.tracking >= 0.0
                && font.color.iter().all(|value| value.is_finite()),
            "HUD source InventoryFont declaration differs (source={:?}, name={}, atlas={}, size={:?}, glyphs={})",
            font.source_object,
            font.name,
            font.atlas.texture,
            font.atlas.size,
            font.glyphs.len()
        );
        let expected_roles = [
            HudTextureRole::EnemyMeterFill,
            HudTextureRole::EnemyMeterBackground,
            HudTextureRole::PlayerLifeHandle,
            HudTextureRole::PlayerLifeRemainder,
            HudTextureRole::PlayerLifeFill,
            HudTextureRole::CurrencyIcon,
        ];
        let expected_names = [
            "fixtex\\stencil\\jauges\\ennemis\\enemyfg_nz.tga",
            "fixtex\\stencil\\jauges\\ennemis\\enemybg_nz.tga",
            "fixtex\\stencil\\jauges\\vie\\poigne_nz.tga",
            "fixtex\\stencil\\jauges\\vie\\vie0.tga",
            "fixtex\\stencil\\jauges\\vie\\vie1.tga",
            "fixtex\\stencil\\jauges\\gold\\gold_icone_nz.tga",
        ];
        let expected_sizes = [[64, 8], [128, 16], [32, 32], [64, 16], [64, 16], [128, 64]];
        let expected_flags = [2, 2, 18, 8, 8, 26];
        let mut ptx_indices = std::collections::BTreeSet::new();
        for (slot, (artwork, role)) in self.artwork.iter().zip(expected_roles).enumerate() {
            anyhow::ensure!(
                artwork.slot as usize == slot,
                "HUD artwork slot order differs"
            );
            anyhow::ensure!(
                artwork.role == role,
                "HUD artwork role differs at slot {slot}"
            );
            anyhow::ensure!(
                artwork.source_object.role == super::ai::SourceRole::Fix,
                "HUD texture is not Fix-owned"
            );
            anyhow::ensure!(
                artwork.source_object.block == [6, 1],
                "HUD GLI object is not in Fix 6:1"
            );
            anyhow::ensure!(
                ptx_indices.insert(artwork.fix_ptx_index),
                "duplicate Fix.ptx HUD texture slot"
            );
            anyhow::ensure!(
                artwork.source_name == expected_names[slot],
                "HUD source texture identity differs at slot {slot}"
            );
            anyhow::ensure!(
                artwork.stored_size.iter().all(|n| *n > 0),
                "HUD texture has zero stored size"
            );
            anyhow::ensure!(
                artwork.logical_size.iter().all(|n| *n > 0),
                "HUD texture has zero logical size"
            );
            anyhow::ensure!(
                artwork.logical_size == expected_sizes[slot],
                "HUD logical texture size differs at slot {slot}"
            );
            anyhow::ensure!(
                artwork.source_flags == expected_flags[slot]
                    && artwork.sample_flags == 0
                    && artwork.addressing == 0,
                "HUD GLI sampling declaration differs at slot {slot}"
            );
            anyhow::ensure!(
                artwork.texture.role() == super::PackageRole::Texture,
                "HUD artwork dependency has the wrong package role"
            );
            let archive_name = artwork
                .source_name
                .replace('\\', "/")
                .to_ascii_lowercase()
                .replace(".tga", ".gf");
            anyhow::ensure!(
                artwork.texture.file().to_ascii_lowercase() == archive_name,
                "HUD GLI name and Textures.cnt asset path differ at slot {slot}"
            );
        }
        anyhow::ensure!(
            self.artwork
                .iter()
                .map(|artwork| artwork.fix_ptx_index)
                .collect::<Vec<_>>()
                == [22, 23, 13, 11, 12, 30],
            "HUD Fix.ptx identity sequence differs"
        );
        anyhow::ensure!(
            self.source_textures
                .iter()
                .zip(&self.artwork)
                .all(|(old, new)| *old == Some(new.source_object.clone())),
            "HUD source texture identity list differs from artwork mapping"
        );
        anyhow::ensure!(
            self.rectangle_tables
                .iter()
                .all(|table| table.len() == 24 && table.iter().flatten().all(|v| v.is_finite())),
            "HUD rectangle table shape or values differ"
        );
        anyhow::ensure!(
            self.text_offsets.iter().flatten().all(|v| v.is_finite()),
            "invalid HUD source offsets"
        );
        anyhow::ensure!(
            self.elements.iter().all(|element| {
                element.table < 2
                    && element.entries.iter().all(|index| *index < 24)
                    && (!element.entries.is_empty() || element.role == "enemy_meter_dynamic_rows")
            }),
            "invalid verified HUD element layout map"
        );
        anyhow::ensure!(
            self.elements
                == [
                    HudElementLayout {
                        role: "player_life_base".into(),
                        table: 1,
                        entries: vec![0],
                    },
                    HudElementLayout {
                        role: "player_life_segments".into(),
                        table: 1,
                        entries: vec![1],
                    },
                    HudElementLayout {
                        role: "enemy_meter_background_fill_pairs".into(),
                        table: 1,
                        entries: vec![11, 12, 13, 14, 15, 16],
                    },
                    HudElementLayout {
                        role: "currency_icon".into(),
                        table: 1,
                        entries: vec![23],
                    },
                ],
            "HUD direct-draw row selectors differ from the verified source"
        );
        Ok(())
    }
}
