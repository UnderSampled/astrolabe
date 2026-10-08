//! Inventory geometry, bindings and localized labels end at this source boundary.
use super::{
    container::{file, pointer_file, u32_at, wire},
    dialogue, dialogue_presentation,
    records::*,
    session::SourceSession,
};
use crate::model::{AssetPath, AssetRef, Selection, inventory::*, io::Content};
use anyhow::{Context, Result, ensure};
use binrw::BinRead;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// 0041bec0 writes pointers to Fix.gpt and scalar values separately to Fix.sda.
/// The supplied Hype profile starts this scalar section after 20 earlier bytes.
#[derive(BinRead)]
#[br(little)]
struct PresentationWire {
    _sound_properties: [u32; 5],
    initial_category: u32,
    _timer_and_text_handle: [u32; 2],
    description: [f32; 2],
    detail: [f32; 2],
    detail_scale: f32,
    _cursor_and_arrow: [f32; 6],
    text_position: [f32; 2],
    stats_position: [f32; 2],
    colors: [[f32; 4]; 9],
    category_size: [f32; 2],
    category_selected_scale: f32,
    category_scale: f32,
    /// `BoxesDistance`, `BoxesScrollingSpeed`, `CasesDistance`,
    /// `CasesScrollingSpeed`. The callback maps these to the vertical box and
    /// horizontal case transitions respectively (00418380/0041b500).
    motion: [f32; 4],
    counter_offset: [f32; 2],
    _other_offset: [f32; 2],
    item_size: [f32; 2],
    item_scales: [f32; 3],
}
/// 004a3ca0, 004a3dc0..004a3eb0. Navigation points to the owning SuperObject.
#[derive(BinRead)]
#[br(little)]
struct BoxWire {
    _neighbors: [u32; 4],
    _rotation: u32,
    position: [f32; 2],
    _current_and_padding: [u8; 4],
}
/// 00419750/00419ef0 traverse this ordinary sprite list.
#[derive(BinRead)]
#[br(little)]
struct ImageWire {
    _texture: u32,
    size: [f32; 2],
    _links: [u32; 3],
}
/// INV_fn_wGetInfo and INV_fn_bItemGetAvailable corroborate +28 and +25.
#[derive(BinRead)]
#[br(little)]
struct ItemWire {
    _links: [u32; 5],
    _capacity: i16,
    gestion_mode: u16,
    _external_condition: u8,
    available: u8,
    _permissions: [u8; 2],
    id: i16,
    _unknown: [u8; 2],
}
fn string(s: &SourceSession, a: Address) -> Result<String> {
    let b = s
        .block(a)?
        .data
        .get(a.offset..)
        .context("inventory string outside block")?;
    let n = b
        .iter()
        .take(4097)
        .position(|b| *b == 0)
        .context("unbounded inventory string")?;
    dialogue::decode_cp1252(&b[..n])
}
fn children(s: &SourceSession, a: Address) -> Result<Vec<Address>> {
    let h: SuperObjectWire = wire(s.bytes(a, 64)?)?;
    ensure!(h.child_count <= 256, "inventory child bound");
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let mut next = s.pointer(a, 8)?;
    while let Some(c) = next {
        ensure!(
            seen.insert(c) && out.len() < 256,
            "inventory child cycle/bound"
        );
        ensure!(s.required(c, 28)? == a, "inventory parent mismatch");
        out.push(c);
        next = s.pointer(c, 20)?;
    }
    ensure!(
        out.len() == h.child_count as usize,
        "inventory child count differs"
    );
    Ok(out)
}
fn images(s: &SourceSession, object: Address, default: [f32; 2]) -> Result<Vec<InventoryImage>> {
    ensure!(
        u32_at(s.bytes(object, 4)?, 0)? == 22,
        "inventory image is not SetOfSprites"
    );
    let list = s.required(object, 4)?;
    let count = u32_at(s.bytes(list, 12)?, 8)?;
    ensure!((1..=16).contains(&count), "inventory sprite layer bound");
    let mut next = s.pointer(list, 0)?;
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    while let Some(a) = next {
        ensure!(seen.insert(a) && out.len() < 16, "inventory sprite cycle");
        let entry: ImageWire = wire(s.bytes(a, 24)?)?;
        let texture = s.required(a, 0)?;
        let tex: TextureInfoWire = wire(s.bytes(texture, 188)?)?;
        let name = std::str::from_utf8(tex.name.split(|b| *b == 0).next().unwrap())?
            .replace('\\', "/")
            .to_ascii_lowercase();
        let size = if entry.size == [0., 0.] {
            default
        } else {
            entry.size
        };
        out.push(InventoryImage {
            texture: AssetRef::new(format!("texture:/{name}"))?,
            size,
            color_key: tex.flags & 2 != 0,
        });
        next = s.pointer(a, 12)?;
    }
    ensure!(
        out.len() == count as usize,
        "inventory sprite count differs"
    );
    Ok(out)
}
fn fix_pointer(
    s: &SourceSession,
    bytes: &[u8],
    refs: &BTreeMap<u32, BlockKey>,
    offset: usize,
) -> Result<Address> {
    let value = u32_at(bytes, offset)?;
    s.resolve(
        Owner::Fix,
        *refs
            .get(&value)
            .context("inventory Fix.gpt pointer lacks RTP binding")?,
        value,
    )
}

pub(crate) fn import(game: &Path, selection: &Selection) -> Result<InventoryPresentation> {
    ensure!(
        selection.locale == "en",
        "inventory requires evidenced English locale"
    );
    let mut s = SourceSession::open(game, selection)?;
    dialogue::load_language(&mut s, game, selection)?;
    let folder = game.join("Gamedata/World/Levels");
    let gpt = file(&folder.join("Fix.gpt"))?;
    let rtp = pointer_file(&file(&folder.join("Fix.rtp"))?)?;
    let root = |n| fix_pointer(&s, &gpt, &rtp, n);
    // Pointer-stream slots are source declarations, never portable asset identities.
    let category_root = root(18156)?;
    let category_nodes = children(&s, category_root)?;
    ensure!(
        category_nodes.len() == 5,
        "unrecognized Hype inventory category root"
    );
    let scalar_file = file(&folder.join("Fix.sda"))?;
    let globals: PresentationWire = wire(
        scalar_file
            .get(20..)
            .context("truncated inventory scalar declarations")?,
    )?;
    ensure!(
        globals.initial_category < 5,
        "inventory initial category bound"
    );
    let mut keyed = BTreeMap::new();
    for (index, &a) in category_nodes.iter().enumerate() {
        keyed.insert(a, format!("category_{index}"));
    }
    for &a in &category_nodes {
        for (index, c) in children(&s, a)?.iter().skip(1).enumerate() {
            if u32_at(s.bytes(*c, 4)?, 0)? == 19 {
                keyed.insert(*c, format!("{}_group_{index}", keyed[&a]));
            }
        }
    }
    let mut categories = Vec::new();
    for (&object, key) in &keyed {
        ensure!(
            u32_at(s.bytes(object, 4)?, 0)? == 19,
            "inventory category kind differs"
        );
        let data = s.required(object, 4)?;
        let h: BoxWire = wire(s.bytes(data, 32)?)?;
        let owner = s.required(object, 28)?;
        let parent = keyed.get(&owner).cloned();
        let position = if parent.is_some() {
            let radians = h.position[0].to_radians();
            [radians.cos() * h.position[1], radians.sin() * h.position[1]]
        } else {
            h.position
        };
        let mut neighbors = [None, None, None, None];
        for (index, offset) in [8, 12, 0, 4].into_iter().enumerate() {
            neighbors[index] = s
                .pointer(data, offset)?
                .map(|a| {
                    keyed
                        .get(&a)
                        .cloned()
                        .context("inventory category navigation leaves root")
                })
                .transpose()?;
        }
        categories.push(InventoryCategory {
            key: key.clone(),
            parent: parent.clone(),
            position,
            neighbors,
            images: images(
                &s,
                children(&s, object)?[0],
                if parent.is_some() {
                    globals.item_size
                } else {
                    globals.category_size
                },
            )?,
        });
    }
    categories.sort_by_key(|c| c.key.clone());
    let row = children(&s, root(18160)?)?;
    ensure!(row.len() == 6, "unrecognized inventory item row");
    let mut slots = Vec::new();
    for object in &row {
        ensure!(
            u32_at(s.bytes(*object, 4)?, 0)? == 19,
            "inventory slot kind differs"
        );
        let data = s.required(*object, 4)?;
        let h: BoxWire = wire(s.bytes(data, 32)?)?;
        let mut neighbors = [None; 4];
        for (index, offset) in [8, 12, 0, 4].into_iter().enumerate() {
            neighbors[index] = s
                .pointer(data, offset)?
                .map(|a| {
                    row.iter()
                        .position(|b| *b == a)
                        .context("inventory slot neighbor leaves row")
                        .and_then(|n| Ok(n.try_into()?))
                })
                .transpose()?;
        }
        slots.push(InventorySlot {
            position: h.position,
            neighbors,
        });
    }
    let item_objects = children(&s, root(18152)?)?;
    ensure!(
        item_objects.len() == 62,
        "unrecognized main inventory catalogue"
    );
    let known_items: BTreeSet<_> = item_objects.iter().copied().collect();
    let mut membership = BTreeMap::new();
    // Stock actions retain actual category/item references (+8/+12):
    // 0045bf00, 0045c350/370 and 0041c720. Repeated pickup owners must agree.
    for &block in s.blocks[&Owner::Fix].keys() {
        if block.0 != 5 {
            continue;
        }
        for &offset in s.allocation_indices(Owner::Fix, block)?.keys() {
            let a = Address {
                owner: Owner::Fix,
                key: block,
                offset,
            };
            let extent = u32_at(
                s.bytes(
                    Address {
                        offset: offset - 4,
                        ..a
                    },
                    4,
                )?,
                0,
            )? as usize
                * 4
                - 4;
            if extent != 20 {
                continue;
            }
            let (Ok(Some(category)), Ok(Some(item))) = (s.pointer(a, 8), s.pointer(a, 12)) else {
                continue;
            };
            if !keyed.contains_key(&category) || !known_items.contains(&item) {
                continue;
            }
            ensure!(
                membership
                    .insert(item, category)
                    .is_none_or(|old| old == category),
                "conflicting source item category"
            );
        }
    }
    // The multilingual registration chain stores writable table sites in Fix;
    // Fix.dlg carries values in that exact chain order (004a2310/004a2340).
    let label_table = root(18168)?;
    let mut sites = Vec::new();
    let mut next = Some(root(18124)?);
    let mut seen = BTreeSet::new();
    while let Some(a) = next {
        ensure!(
            seen.insert(a) && sites.len() < 256,
            "inventory localized site chain"
        );
        let site = s.required(a, 0)?;
        ensure!(
            site.owner == label_table.owner
                && site.key == label_table.key
                && site.offset >= label_table.offset
                && (site.offset - label_table.offset).is_multiple_of(4),
            "inventory text site outside table"
        );
        sites.push((site.offset - label_table.offset) / 4);
        next = s.pointer(a, 4)?;
    }
    let lang = game.join("LangData/English/world/levels");
    let dlg = file(&lang.join("Fix.dlg"))?;
    let rtd = pointer_file(&file(&lang.join("Fix.rtd"))?)?;
    let mut localized = Vec::new();
    // This language section starts with inventory registrations, followed by
    // a separate world-label registration chain in the same text block.
    for chunk in dlg.as_chunks::<4>().0 {
        if localized.len() == sites.len() {
            break;
        }
        let value = u32::from_le_bytes(*chunk);
        if let Some(&key) = rtd.get(&value)
            && key == BlockKey(10, 1)
        {
            localized.push(string(&s, s.resolve(Owner::Fix, key, value)?)?);
        }
    }
    ensure!(
        sites.len() == localized.len(),
        "inventory localized text registration/value count differs: {} sites, {} values",
        sites.len(),
        localized.len()
    );
    let labels: BTreeMap<_, _> = sites.into_iter().zip(localized).collect();
    let mut items = Vec::new();
    for object in item_objects {
        let Some(category) = membership.get(&object) else {
            continue;
        };
        ensure!(
            u32_at(s.bytes(object, 4)?, 0)? == 18,
            "inventory item kind differs"
        );
        let handle = s.required(object, 4)?;
        let member = s.required(handle, 0)?;
        let h: ItemWire = wire(s.bytes(member, 32)?)?;
        let kind = string(&s, s.required(member, 12)?)?;
        let label = labels
            .get(&(h.id as usize))
            .context("inventory item has no localized label")?
            .clone();
        items.push(InventoryItem {
            id: h.id.into(),
            kind,
            label,
            category: keyed[category].clone(),
            images: images(&s, children(&s, object)?[0], globals.item_size)?,
            draw_counter: h.available != 1,
            draw_when_zero: h.gestion_mode & 2 != 0,
            available: h.available,
        });
    }
    let (_, font) = dialogue_presentation::named_font(&s, "InventoryFont")?;
    let background = string(&s, root(18140)?)?
        .replace('\\', "/")
        .to_ascii_lowercase();
    let value = InventoryPresentation {
        path: AssetPath::new("fix:/inventory/en")?,
        locale: selection.locale.clone(),
        background: AssetRef::new(format!("texture:/{background}"))?,
        initial_category: keyed[&category_nodes[globals.initial_category as usize]].clone(),
        categories,
        slots,
        items,
        category_scale: [globals.category_scale, globals.category_selected_scale],
        category_color: [globals.colors[2], globals.colors[1]],
        item_scale: globals.item_scales,
        item_color: [globals.colors[7], globals.colors[5]],
        category_motion: InventoryMotion {
            distance: globals.motion[0],
            speed_per_millisecond: globals.motion[1],
        },
        item_motion: InventoryMotion {
            distance: globals.motion[2],
            speed_per_millisecond: globals.motion[3],
        },
        counter_offset: globals.counter_offset,
        text_position: globals.text_position,
        stats_position: globals.stats_position,
        description_position: globals.description,
        detail_position: globals.detail,
        detail_scale: globals.detail_scale,
        font: InventoryFont {
            name: font.name,
            atlas: font.atlas,
            glyphs: font.glyphs,
            tracking: font.tracking,
        },
    };
    value.validate()?;
    Ok(value)
}
