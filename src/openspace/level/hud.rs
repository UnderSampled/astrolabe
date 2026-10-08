//! Fix HUD declarations and direct-draw inputs for the installed Hype profile.
use super::{
    container::{file, pointer_file, u32_at, wire},
    dialogue_presentation,
    records::{Address, BlockKey, Owner, SuperObjectWire, TextureInfoWire},
    session::SourceSession,
};
use crate::model::{
    AssetPath, Selection,
    hud::{
        HudArtwork, HudCounterFont, HudElementLayout, HudPanel, HudPanelLayer, HudPresentation,
        HudTextureRole,
    },
    io::Content,
};
use anyhow::{Context, Result, ensure};
use binrw::BinRead;
use std::{collections::BTreeMap, path::Path};

const FIX_GPT_SIZE: usize = 19_344;
const FIX_SDA_SIZE: usize = 3_683;
const FIX_PTX_SIZE: usize = 3_496;
const FIX_HUD_GPT_OFFSET: usize = 0x4a38;
const FIX_HUD_SDA_OFFSET: usize = 0x9b4;
const FIX_HUD_ARRAYS_OFFSET: usize = FIX_HUD_SDA_OFFSET + 0x18;
const HUD_RECT_ROWS: usize = 24;
const HUD_RECT_BYTES: usize = HUD_RECT_ROWS * 6 * 4;

#[derive(BinRead)]
#[br(little)]
struct PanelWire {
    layers: u32,
    layer_material: u32,
    draw_material: u32,
    enabled: u8,
    layer_count: u8,
    unknown: [u8; 2],
}

#[derive(BinRead)]
#[br(little)]
struct LayerWire {
    _texture: u32,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    u0: f32,
    u1: f32,
    v0: f32,
    v1: f32,
}

#[derive(BinRead)]
#[br(little)]
struct CurrencyItemWire {
    _links: [u32; 5],
    _capacity: i16,
    _gestion_mode: u16,
    _external_condition: u8,
    _available: u8,
    _permissions: [u8; 2],
    id: i16,
    _unknown: [u8; 2],
}

#[derive(Debug)]
struct FixTextureTable {
    pointers: Vec<u32>,
}

impl FixTextureTable {
    fn read(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() == FIX_PTX_SIZE, "unsupported Fix.ptx extent");
        let capacity = u32_at(bytes, 0)?;
        let count_u32 = u32_at(bytes, 4)?;
        ensure!(
            capacity == 1024 && count_u32 <= capacity,
            "unsupported Fix.ptx header"
        );
        let count = usize::try_from(count_u32)?;
        let pointer_end = 8usize
            .checked_add(
                count
                    .checked_mul(4)
                    .context("Fix.ptx pointer count overflow")?,
            )
            .context("Fix.ptx pointer extent overflow")?;
        let loaded_count = usize::try_from(u32_at(bytes, pointer_end)?)?;
        ensure!(
            loaded_count == count,
            "Fix.ptx pointer/channel counts differ"
        );
        let expected = pointer_end
            .checked_add(8)
            .and_then(|n| n.checked_add(loaded_count.checked_mul(4)?))
            .context("Fix.ptx channel extent overflow")?;
        ensure!(
            expected == bytes.len(),
            "Fix.ptx channel table extent differs"
        );
        Ok(Self {
            pointers: (0..count)
                .map(|index| u32_at(bytes, 8 + index * 4))
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

pub(super) fn gpt_hud_offsets(sda: &[u8]) -> Result<(usize, usize)> {
    // 00401e00's sequential GPT reads before 0040bcc0. Counts come from the
    // simultaneously-open Fix.sda stream: FON text-handle count at +8, and 004e5150's
    // inclusive shadow index at +0x198. FUN_004e94c0 uses the SDA FILE* while
    // FUN_004e97a0/FUN_004e9800 use the separate GPT FILE*.
    let text_handle_count = usize::try_from(u32_at(sda, 8)?)?;
    let shadow_last = i16::from_le_bytes(
        sda.get(0x198..0x19a)
            .context("Fix.sda shadow count missing")?
            .try_into()?,
    );
    ensure!(
        text_handle_count == 409,
        "unsupported Fix FON text-handle count {text_handle_count}"
    );
    ensure!(
        shadow_last == 4,
        "unsupported Fix shadow extent {shadow_last}"
    );

    let mut cursor = 0usize;
    // 8 initial pointer words, 0x90 raw, FUN_00422c00's 0x3d54, 100
    // matrix pointers, then 4 + 0xc + 0x20 raw-byte reads.
    cursor += 8 * 4 + 0x90 + 0x3d54 + 100 * 4 + 4 + 0xc + 0x20;
    // fn_vReadFONGeneral: 4 fixed words, count words, 0xc raw bytes.
    cursor += 0x1c + text_handle_count * 4;
    // 0x10 raw, three pointers, FUN_00421f30 (12 pointers), five pointers.
    cursor += 0x10 + 3 * 4 + 12 * 4 + 5 * 4;
    // DLG (8), 0050d270, 0040b750, inventory (1) GPT pointer words.
    cursor += 8 * 4 + 3 * 4;
    let currency_root = cursor + 9 * 4; // FUN_0041c290's DAT_006107bc.
    cursor += 18 * 4; // 0041c290
    // 00416420: 4 + 100 + 7 + 1 GPT words. Its 60 e9a60 reads use another stream.
    cursor += 112 * 4;
    // 004e5150: one, 3*(inclusive max+1), 20, and 3*max GPT words.
    let shadow_last = shadow_last as usize;
    cursor += (1 + (shadow_last + 1) * 3 + 20 + shadow_last * 3) * 4;
    cursor += 2 * 4; // 004161b0
    cursor += 4 * 4; // 0040f7b0; seven e9a60 reads use another stream.
    cursor += 2 * 4 + 0x80; // 0040f300
    ensure!(
        cursor == FIX_HUD_GPT_OFFSET,
        "Fix.gpt cursor accounting differs: {cursor:#x}"
    );
    Ok((currency_root, cursor))
}

fn gpt_hud_cursor(sda: &[u8]) -> Result<usize> {
    Ok(gpt_hud_offsets(sda)?.1)
}

fn fix_pointer(
    session: &SourceSession,
    refs: &BTreeMap<u32, BlockKey>,
    value: u32,
) -> Result<Option<Address>> {
    if value == 0 {
        return Ok(None);
    }
    let key = *refs
        .get(&value)
        .with_context(|| format!("Fix.gpt pointer {value:#010x} lacks RTP binding"))?;
    if key == BlockKey(255, 255) {
        return Ok(None);
    }
    Ok(Some(session.resolve_local(Owner::Fix, key, value)?))
}

pub(super) fn currency_item(
    session: &SourceSession,
    refs: &BTreeMap<u32, BlockKey>,
    gpt: &[u8],
    gpt_offset: usize,
) -> Result<(Address, Address, Address, i32)> {
    let root_value = u32_at(gpt, gpt_offset)?;
    let root = fix_pointer(session, refs, root_value)?
        .context("Fix HUD currency inventory root is null")?;
    let child = session
        .pointer(root, 8)?
        .context("Fix HUD currency item is absent")?;
    let root_wire: SuperObjectWire = wire(session.bytes(root, 64)?)?;
    ensure!(
        root_wire.child_count == 1,
        "Fix HUD currency root child count differs"
    );
    ensure!(
        session.required(child, 28)? == root && session.pointer(child, 20)?.is_none(),
        "Fix HUD currency item is not the sole direct child"
    );
    ensure!(
        u32_at(session.bytes(child, 4)?, 0)? == 18,
        "Fix HUD currency child is not an inventory item"
    );
    let item_handle = session.required(child, 4)?;
    let item_definition = session.required(item_handle, 0)?;
    let item: CurrencyItemWire = wire(session.bytes(item_definition, 32)?)?;
    ensure!(item.id >= 0, "Fix HUD currency item id is negative");
    Ok((child, item_handle, item_definition, i32::from(item.id)))
}

fn f32_at(bytes: &[u8], offset: usize) -> Result<f32> {
    Ok(f32::from_bits(u32_at(bytes, offset)?))
}

fn f32_pair(bytes: &[u8], offset: usize) -> Result<[f32; 2]> {
    Ok([f32_at(bytes, offset)?, f32_at(bytes, offset + 4)?])
}

fn texture_name(info: &TextureInfoWire) -> Result<String> {
    let end = info
        .name
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(info.name.len());
    ensure!(end > 0, "Fix GLI texture name is empty");
    Ok(std::str::from_utf8(&info.name[..end])?.to_owned())
}

fn artwork_role(slot: usize) -> HudTextureRole {
    match slot {
        0 => HudTextureRole::EnemyMeterFill,
        1 => HudTextureRole::EnemyMeterBackground,
        2 => HudTextureRole::PlayerLifeHandle,
        3 => HudTextureRole::PlayerLifeRemainder,
        4 => HudTextureRole::PlayerLifeFill,
        5 => HudTextureRole::CurrencyIcon,
        _ => unreachable!("six Fix HUD texture slots"),
    }
}

fn ptx_index(
    session: &SourceSession,
    ptx: &FixTextureTable,
    rtt: &BTreeMap<u32, BlockKey>,
    target: Address,
) -> Result<u16> {
    let mut matches = Vec::new();
    for (index, &value) in ptx.pointers.iter().enumerate() {
        if value == 0 {
            continue;
        }
        let key = *rtt
            .get(&value)
            .with_context(|| format!("Fix.ptx[{index}] pointer lacks RTT binding"))?;
        if key != BlockKey(255, 255) && session.resolve_local(Owner::Fix, key, value)? == target {
            matches.push(u16::try_from(index)?);
        }
    }
    ensure!(
        matches.len() == 1,
        "Fix GLI object has {} Fix.ptx identities",
        matches.len()
    );
    Ok(matches[0])
}

fn panel(
    session: &SourceSession,
    refs: &BTreeMap<u32, BlockKey>,
    wire_value: PanelWire,
) -> Result<HudPanel> {
    ensure!(
        wire_value.enabled <= 1,
        "Fix HUD panel enabled flag is not boolean"
    );
    let layer_array = fix_pointer(session, refs, wire_value.layers)?;
    let layer_material = fix_pointer(session, refs, wire_value.layer_material)?;
    let draw_material = fix_pointer(session, refs, wire_value.draw_material)?;
    let mut layers = Vec::new();
    if let Some(array) = layer_array {
        ensure!(
            array.key == BlockKey(5, 0),
            "Fix HUD layer array is outside Fix 5:0"
        );
        let bytes = session.allocation_bytes(array)?;
        let expected = usize::from(wire_value.layer_count) * 0x24;
        ensure!(
            bytes.len() == expected,
            "Fix HUD layer array extent differs from descriptor"
        );
        if wire_value.enabled != 0 {
            for index in 0..wire_value.layer_count as usize {
                let address = Address {
                    offset: array.offset + index * 0x24,
                    ..array
                };
                let layer: LayerWire = wire(session.bytes(address, 0x24)?)?;
                ensure!(
                    [
                        layer.x0, layer.y0, layer.x1, layer.y1, layer.u0, layer.u1, layer.v0,
                        layer.v1
                    ]
                    .iter()
                    .all(|value| value.is_finite()),
                    "Fix HUD layer has non-finite geometry"
                );
                let texture = session.pointer(address, 0)?;
                layers.push(HudPanelLayer {
                    texture: texture.map(|a| session.source_identity(a)).transpose()?,
                    opaque_texture_word: None,
                    destination: [layer.x0, layer.x1, layer.y0, layer.y1],
                    uv: [layer.u0, layer.u1, layer.v0, layer.v1],
                });
            }
        }
    } else {
        ensure!(
            wire_value.layer_count == 0,
            "Fix HUD panel has layers but no array pointer"
        );
    }
    for address in [layer_material, draw_material].into_iter().flatten() {
        ensure!(
            address.key == BlockKey(6, 1),
            "Fix HUD panel material is outside Fix 6:1"
        );
        ensure!(
            !session.allocation_bytes(address)?.is_empty(),
            "Fix HUD panel material allocation is empty"
        );
    }
    let identity =
        |address: Option<Address>| address.map(|a| session.source_identity(a)).transpose();
    Ok(HudPanel {
        enabled: wire_value.enabled != 0,
        source_layers: identity(layer_array)?,
        layer_material: identity(layer_material)?,
        draw_material: identity(draw_material)?,
        draw_material_word: None,
        source_unknown_bytes: wire_value.unknown,
        source_layer_count: wire_value.layer_count,
        layers,
    })
}

pub(crate) fn import(
    game: &Path,
    selection: &Selection,
    archive: &super::super::texture::TextureArchive,
) -> Result<HudPresentation> {
    ensure!(
        selection.locale == "en",
        "Fix HUD requires evidenced English locale"
    );
    let folder = game.join("Gamedata/World/Levels");
    let gpt = file(&folder.join("Fix.gpt"))?;
    let sda = file(&folder.join("Fix.sda"))?;
    ensure!(gpt.len() == FIX_GPT_SIZE, "unsupported Fix.gpt extent");
    ensure!(sda.len() == FIX_SDA_SIZE, "unsupported Fix.sda extent");
    let gpt_offset = gpt_hud_cursor(&sda)?;
    ensure!(
        gpt_offset + 0x40 + 6 * 4 <= gpt.len(),
        "Fix HUD GPT record exceeds source extent"
    );
    let session = SourceSession::open(game, selection)?;
    let refs = pointer_file(&file(&folder.join("Fix.rtp"))?)?;
    let rtt = pointer_file(&file(&folder.join("Fix.rtt"))?)?;
    let ptx = FixTextureTable::read(&file(&folder.join("Fix.ptx"))?)?;

    let mut panels = Vec::with_capacity(4);
    for index in 0..4 {
        let descriptor: PanelWire =
            wire(&gpt[gpt_offset + index * 16..gpt_offset + (index + 1) * 16])?;
        panels.push(panel(&session, &refs, descriptor)?);
    }
    ensure!(
        panels.iter().all(|value| !value.enabled)
            && panels
                .iter()
                .map(|value| value.source_layer_count)
                .collect::<Vec<_>>()
                == [31, 7, 14, 4],
        "Fix HUD panel descriptors differ from the installed disabled-panel profile"
    );

    let slots = gpt_offset + 0x40;
    let mut source_textures = Vec::with_capacity(6);
    let mut artwork = Vec::with_capacity(6);
    for slot in 0..6 {
        let value = u32_at(&gpt, slots + slot * 4)?;
        let object = fix_pointer(&session, &refs, value)?
            .with_context(|| format!("Fix HUD texture slot {slot} is null"))?;
        ensure!(
            object.key == BlockKey(6, 1),
            "Fix HUD GLI object is outside Fix block 6:1"
        );
        let object_bytes = session.allocation_bytes(object)?;
        ensure!(
            object_bytes.len() == 0xbc,
            "Fix HUD GLI object extent differs"
        );
        let info: TextureInfoWire = wire(object_bytes)?;
        let source_name = texture_name(&info)?;
        ensure!(
            source_name.to_ascii_lowercase().ends_with(".tga"),
            "Fix HUD GLI name is not a TGA source path"
        );
        let index = ptx_index(&session, &ptx, &rtt, object)?;
        let entry = archive.find_entry(&source_name).with_context(|| {
            format!("Fix HUD GLI source name is absent from Textures.cnt: {source_name}")
        })?;
        let image = archive.image(entry)?;
        let logical_size = [info.logical_width, info.logical_height];
        ensure!(
            [image.width(), image.height()] == logical_size,
            "Fix HUD GLI logical size {:?} differs from decoded archive member {} size {}x{}",
            logical_size,
            entry.name,
            image.width(),
            image.height()
        );
        ensure!(
            info.stored_width > 0 && info.stored_height > 0,
            "Fix HUD GLI has zero stored dimensions"
        );
        let source_object = session.source_identity(object)?;
        source_textures.push(Some(source_object.clone()));
        artwork.push(HudArtwork {
            slot: u8::try_from(slot)?,
            role: artwork_role(slot),
            source_object,
            fix_ptx_index: index,
            source_name,
            texture: entry.texture.path.clone(),
            stored_size: [info.stored_width, info.stored_height],
            logical_size,
            source_flags: info.flags,
            sample_flags: info.sample_flags,
            addressing: info.addressing,
        });
    }
    ensure!(
        artwork.iter().map(|a| a.fix_ptx_index).collect::<Vec<_>>() == [22, 23, 13, 11, 12, 30],
        "Fix HUD PTX identity sequence differs from the installed profile"
    );
    let source_textures: [Option<crate::model::ai::SourceObjectIdentity>; 6] = source_textures
        .try_into()
        .map_err(|_| anyhow::anyhow!("Fix HUD slot count differs"))?;

    let (currency_root_offset, _) = gpt_hud_offsets(&sda)?;
    let (currency_source, _, _, currency_item_id) =
        currency_item(&session, &refs, &gpt, currency_root_offset)?;
    let (font_address, font) = dialogue_presentation::named_font(&session, "InventoryFont")?;
    let currency_font = HudCounterFont {
        source_object: session.source_identity(font_address)?,
        name: font.name,
        atlas: font.atlas,
        glyphs: font.glyphs,
        tracking: font.tracking,
        color: font.color,
    };

    // 0040bcc0 reads this section from the separate Fix.sda stream. Its two
    // raw words, two XY offsets and 24x6-float tables follow the sequential
    // SDA readers through 0040f300; the verified cursor for this profile is 0x9b4.
    ensure!(
        FIX_HUD_ARRAYS_OFFSET + 2 * HUD_RECT_BYTES <= sda.len(),
        "Fix HUD SDA arrays exceed source extent"
    );
    let source_words = [
        u32_at(&sda, FIX_HUD_SDA_OFFSET)?,
        u32_at(&sda, FIX_HUD_SDA_OFFSET + 4)?,
    ];
    let text_offsets = [
        f32_pair(&sda, FIX_HUD_SDA_OFFSET + 8)?,
        f32_pair(&sda, FIX_HUD_SDA_OFFSET + 16)?,
    ];
    let read_table = |offset: usize| -> Result<Vec<[f32; 6]>> {
        sda[offset..offset + HUD_RECT_BYTES]
            .as_chunks::<24>()
            .0
            .iter()
            .map(|row| {
                let value: [f32; 6] = wire(row)?;
                ensure!(
                    value.iter().all(|v| v.is_finite()),
                    "Fix HUD rectangle has non-finite values"
                );
                Ok(value)
            })
            .collect()
    };
    let rectangle_tables = [
        read_table(FIX_HUD_ARRAYS_OFFSET)?,
        read_table(FIX_HUD_ARRAYS_OFFSET + HUD_RECT_BYTES)?,
    ];
    let elements = vec![
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
    ];
    let value = HudPresentation {
        path: AssetPath::new("fix:/hud/en")?,
        canvas_size: [640, 480],
        source_layout_version: 1,
        panels,
        source_textures,
        artwork,
        currency_source_object: Some(session.source_identity(currency_source)?),
        currency_item_id: Some(currency_item_id),
        currency_font: Some(currency_font),
        rectangle_tables,
        source_words,
        text_offsets,
        elements,
    };
    value.validate()?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::texture::TextureUsage;

    #[test]
    #[ignore = "requires local Hype Fix data"]
    fn cursor_uses_source_counts_and_targets_verified_fix_layout() -> Result<()> {
        let game = std::env::var_os("ASTROLABE_GAME_DIR")
            .map(std::path::PathBuf::from)
            .context("ASTROLABE_GAME_DIR required for Fix HUD cursor regression")?;
        let sda = file(&game.join("Gamedata/World/Levels/Fix.sda"))?;
        ensure!(gpt_hud_cursor(&sda)? == 0x4a38);
        ensure!(gpt_hud_cursor(&sda)? + 0x40 == 0x4a78);
        Ok(())
    }

    #[test]
    #[ignore = "requires local Hype Fix data and texture archive"]
    fn local_hud_objects_join_fix_ptx_and_decoded_archive_images() -> Result<()> {
        let game = std::env::var_os("ASTROLABE_GAME_DIR")
            .map(std::path::PathBuf::from)
            .context("ASTROLABE_GAME_DIR required for Fix HUD source regression")?;
        let selection = Selection::hype("manoir", "en")?;
        let archive = crate::openspace::texture::decode_cnt(
            &file(&game.join("Gamedata/Textures.cnt"))?,
            TextureUsage::Material,
        )?;
        let value = import(&game, &selection, &archive)?;
        ensure!(value.source_layout_version == 1);
        ensure!(
            value
                .artwork
                .iter()
                .map(|a| a.fix_ptx_index)
                .collect::<Vec<_>>()
                == [22, 23, 13, 11, 12, 30]
        );
        ensure!(
            value
                .artwork
                .iter()
                .map(|a| a.logical_size)
                .collect::<Vec<_>>()
                == [[64, 8], [128, 16], [32, 32], [64, 16], [64, 16], [128, 64]]
        );
        ensure!(
            value
                .artwork
                .iter()
                .all(|a| a.source_object.role == crate::model::ai::SourceRole::Fix)
        );
        ensure!(value.currency_item_id == Some(0));
        ensure!(value.currency_source_object.as_ref().is_some_and(|object| {
            object.role == crate::model::ai::SourceRole::Fix && object.block == [6, 1]
        }));
        let font = value
            .currency_font
            .as_ref()
            .context("HUD currency font absent")?;
        ensure!(font.name == "InventoryFont");
        ensure!(font.source_object.block == [19, 1]);
        ensure!(font.glyphs.len() == 127);
        ensure!(font.atlas.size == [256, 256]);
        ensure!(font.atlas.texture.as_str() == "texture:/fixtex/motortxt/lettrageb.tga");
        ensure!(value.artwork.iter().all(|a| {
            a.texture
                .as_str()
                .starts_with("texture:/FixTex/STENCIL/Jauges/")
        }));
        Ok(())
    }
}
