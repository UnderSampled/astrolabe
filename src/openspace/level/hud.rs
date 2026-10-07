//! Source-derived Fix gameplay HUD layout and retained GLI texture handles.
use super::{
    container::{file, pointer_file, u32_at, wire},
    records::*,
    session::SourceSession,
};
use crate::model::{AssetPath, Selection, ai::SourceObjectIdentity, hud::*, io::Content};
use anyhow::{Context, Result, ensure};
use binrw::BinRead;
use std::{collections::BTreeMap, path::Path};

const FIX_GPT_HUD_OFFSET: usize = 0x4904;
const HUD_PANEL_COUNT: usize = 4;
const HUD_PANEL_BYTES: usize = 16;
const HUD_TEXTURE_COUNT: usize = 6;
const HUD_TEXTURE_STRIDE: usize = 4;
const HUD_LAYER_BYTES: usize = 36;
const HUD_SDA_OFFSET: usize = 0x9b4;

/// FUN_0040bcc0/FUN_0040bda0 read/write four 16-byte panel descriptors and six
/// contiguous texture handles. FUN_0040c270 draws the descriptor layer arrays.
#[derive(BinRead)]
#[br(little)]
struct PanelDescriptorWire {
    layers: u32,
    material: u32,
    unknown: u32,
    enabled: u8,
    layer_count: u8,
    _flags: [u8; 2],
}

/// FUN_0040c270 consumes each layer as one relocated Fix object handle and eight
/// floats, at a 0x24-byte stride. The handle is not a serialized TextureInfo.
#[derive(BinRead)]
#[br(little)]
struct PanelLayerWire {
    texture_handle: u32,
    coordinates: [f32; 8],
}

/// Fix.sda begins this HUD scalar block at 0x9b4. The first two words are
/// retained uninterpreted; the next four floats are the two source text offsets.
#[derive(BinRead)]
#[br(little)]
struct HudSdaWire {
    source_words: [u32; 2],
    text_offsets: [[f32; 2]; 2],
    rectangle_tables: [[[f32; 6]; 24]; 2],
}

fn identity_at(
    s: &SourceSession,
    rtp: &BTreeMap<u32, BlockKey>,
    pointer: u32,
) -> Result<Option<SourceObjectIdentity>> {
    if pointer == 0 {
        return Ok(None);
    }
    let key = *rtp
        .get(&pointer)
        .with_context(|| format!("HUD pointer {pointer:#010x} lacks Fix.rtp binding"))?;
    let address = s.resolve(Owner::Fix, key, pointer)?;
    Ok(Some(s.source_identity(address)?))
}

fn unknown_reference(
    s: &SourceSession,
    rtp: &BTreeMap<u32, BlockKey>,
    word: u32,
) -> Result<(Option<SourceObjectIdentity>, Option<[u8; 4]>)> {
    if word == 0 || rtp.contains_key(&word) {
        return Ok((identity_at(s, rtp, word)?, None));
    }
    ensure!(
        !s.resembles_source_address(word),
        "HUD descriptor contains unbound source-like pointer {word:#010x}"
    );
    Ok((None, Some(word.to_le_bytes())))
}

fn gpt_identity(
    s: &SourceSession,
    rtp: &BTreeMap<u32, BlockKey>,
    pointer: u32,
) -> Result<Option<SourceObjectIdentity>> {
    identity_at(s, rtp, pointer)
}

fn panel_layer(s: &SourceSession, address: Address) -> Result<HudPanelLayer> {
    let wire: PanelLayerWire = wire(s.bytes(address, HUD_LAYER_BYTES)?)?;
    let relocated = s.relocated(address)?;
    let texture = if relocated {
        s.pointer(address, 0)?
            .map(|texture| s.source_identity(texture))
            .transpose()?
    } else {
        None
    };
    let c = wire.coordinates;
    Ok(HudPanelLayer {
        texture,
        opaque_texture_word: (!relocated && wire.texture_handle != 0)
            .then_some(wire.texture_handle.to_le_bytes()),
        // FUN_0040c270 passes the x and y endpoints from alternating words.
        destination: [c[0], c[2], c[1], c[3]],
        // FUN_0047abb0 takes u0,u1,v0,v1 in this order.
        uv: [c[4], c[6], c[5], c[7]],
    })
}

fn parse_panels(
    s: &SourceSession,
    gpt: &[u8],
    rtp: &BTreeMap<u32, BlockKey>,
    descriptor_offset: usize,
) -> Result<Vec<HudPanel>> {
    let mut panels = Vec::with_capacity(HUD_PANEL_COUNT);
    for index in 0..HUD_PANEL_COUNT {
        let start = descriptor_offset + index * HUD_PANEL_BYTES;
        let descriptor: PanelDescriptorWire = wire(
            gpt.get(start..start + HUD_PANEL_BYTES)
                .context("truncated HUD panel descriptor")?,
        )?;
        ensure!(descriptor.enabled <= 1, "invalid HUD panel enabled flag");
        let source_layers = identity_at(s, rtp, descriptor.layers)?;
        let material = identity_at(s, rtp, descriptor.material)?;
        let (unknown_reference, unknown_word) = unknown_reference(s, rtp, descriptor.unknown)?;
        let mut layers = Vec::with_capacity(descriptor.layer_count as usize);
        // FUN_0040c270 reads layer records only inside its enabled-panel guard.
        // Disabled profile arrays retain their allocation identity and declared count;
        // their unused coordinate bytes are not promoted to layout data.
        if descriptor.enabled != 0 && descriptor.layer_count != 0 {
            let key = *rtp
                .get(&descriptor.layers)
                .with_context(|| format!("HUD panel {index} layer array lacks Fix.rtp binding"))?;
            let base = s.resolve(Owner::Fix, key, descriptor.layers)?;
            for layer_index in 0..descriptor.layer_count as usize {
                layers.push(
                    panel_layer(
                        s,
                        Address {
                            offset: base.offset + layer_index * HUD_LAYER_BYTES,
                            ..base
                        },
                    )
                    .with_context(|| format!("HUD panel {index} layer {layer_index}"))?,
                );
            }
        }
        panels.push(HudPanel {
            enabled: descriptor.enabled != 0,
            source_layers,
            material,
            unknown_reference,
            unknown_word,
            source_unknown_bytes: descriptor._flags,
            source_layer_count: descriptor.layer_count,
            layers,
        });
    }
    Ok(panels)
}

fn source_texture_links(
    s: &SourceSession,
    gpt: &[u8],
    rtp: &BTreeMap<u32, BlockKey>,
    offset: usize,
) -> Result<[Option<SourceObjectIdentity>; HUD_TEXTURE_COUNT]> {
    // FUN_0040bcc0 reads six contiguous four-byte Fix.gpt pointers through
    // FUN_004e97a0, assigning them to globals 0060f62c through 0060f63c in
    // source order. FUN_004e97a0 applies Fix relocation via FUN_004eab90.
    let mut textures = Vec::with_capacity(HUD_TEXTURE_COUNT);
    for index in 0..HUD_TEXTURE_COUNT {
        let slot = offset + index * HUD_TEXTURE_STRIDE;
        textures.push(
            gpt_identity(s, rtp, u32_at(gpt, slot)?)
                .with_context(|| format!("HUD texture handle slot {index}"))?,
        );
    }
    textures
        .try_into()
        .map_err(|_| anyhow::anyhow!("HUD texture handle count differs"))
}

pub(crate) fn import(game: &Path, selection: &Selection) -> Result<HudPresentation> {
    ensure!(
        selection.locale == "en",
        "HUD currently supports English source data"
    );
    let s = SourceSession::open(game, selection)?;
    let folder = game.join("Gamedata/World/Levels");
    let gpt = file(&folder.join("Fix.gpt"))?;
    ensure!(gpt.len() == 19344, "installed Fix.gpt extent differs");
    let rtp = pointer_file(&file(&folder.join("Fix.rtp"))?)?;
    let panels = parse_panels(&s, &gpt, &rtp, FIX_GPT_HUD_OFFSET)
        .context("read four Fix.gpt HUD panel descriptors")?;
    let source_textures = source_texture_links(
        &s,
        &gpt,
        &rtp,
        FIX_GPT_HUD_OFFSET + HUD_PANEL_COUNT * HUD_PANEL_BYTES,
    )
    .context("read six contiguous Fix.gpt HUD texture handles")?;
    let sda = file(&folder.join("Fix.sda"))?;
    let bytes = sda
        .get(HUD_SDA_OFFSET..)
        .context("truncated Fix.sda HUD layout")?;
    let source: HudSdaWire = wire(bytes)?;
    let elements = vec![
        HudElementLayout {
            role: "life".into(),
            table: 1,
            entries: vec![0, 1],
        },
        HudElementLayout {
            role: "armor".into(),
            table: 1,
            entries: vec![2],
        },
        HudElementLayout {
            role: "magic".into(),
            table: 1,
            entries: vec![5, 6, 7],
        },
        HudElementLayout {
            role: "selected_object".into(),
            table: 1,
            entries: vec![10],
        },
        HudElementLayout {
            role: "weapon".into(),
            table: 1,
            entries: vec![22],
        },
    ];
    let value = HudPresentation {
        path: AssetPath::new("fix:/hud/en")?,
        canvas_size: [640, 480],
        panels,
        source_textures,
        rectangle_tables: source.rectangle_tables.map(Vec::from),
        source_words: source.source_words,
        text_offsets: source.text_offsets,
        elements,
    };
    value.validate()?;
    Ok(value)
}
