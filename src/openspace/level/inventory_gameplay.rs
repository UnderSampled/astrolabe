//! Hype inventory admission and pickup records; source addresses end here.
use super::{
    container::{file, pointer_file, u32_at, wire},
    records::*,
    session::SourceSession,
};
use crate::model::{AssetPath, Selection, inventory_gameplay::*, io::Content};
use anyhow::{Context, Result, ensure};
use binrw::BinRead;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// 004a1920/1960/19d0 and 004a3ba0: selected byte and signed element count.
#[derive(BinRead)]
#[br(little)]
struct HandleWire {
    _item: u32,
    _links: u32,
    selected: u8,
    unknown: u8,
    quantity: i16,
    replacements: ListWire,
}
/// 004a3a60 creates 20 bytes; 004a3bd0 maintains the handle's replacement list.
/// 004a3c70/90 read the two item SuperObject endpoints; 0041c720 consumes them.
#[derive(BinRead)]
#[br(little)]
struct ReplacementWire {
    _from: u32,
    _to: u32,
    _next: u32,
    _previous: u32,
    _owner: u32,
}
/// 004a19f0, 004a3890, 004a3940, 004a3950 and 0041c720.
#[derive(BinRead)]
#[br(little)]
struct ItemWire {
    _links: [u32; 5],
    maximum: i16,
    management: u16,
    condition: u8,
    availability: u8,
    permissions: [u8; 2],
    id: i16,
    unknown: [u8; 2],
}
/// 0045c350/370/390 and AddObject's actor+0x38 forwarding in 00539bb1.
#[derive(BinRead)]
#[br(little)]
struct StockWire {
    _links: [u32; 2],
    _category: u32,
    _item: u32,
    count: i16,
    unknown: [u8; 2],
}

fn children(s: &SourceSession, a: Address) -> Result<Vec<Address>> {
    let h: SuperObjectWire = wire(s.bytes(a, 64)?)?;
    ensure!(h.child_count <= 256, "inventory gameplay child bound");
    let mut values = Vec::new();
    let mut seen = BTreeSet::new();
    let mut next = s.pointer(a, 8)?;
    while let Some(child) = next {
        ensure!(
            seen.insert(child) && values.len() < 256,
            "inventory gameplay child cycle"
        );
        ensure!(
            s.required(child, 28)? == a,
            "inventory gameplay child parent differs"
        );
        values.push(child);
        next = s.pointer(child, 20)?;
    }
    ensure!(
        values.len() == h.child_count as usize,
        "inventory gameplay child count differs"
    );
    Ok(values)
}
fn handle(s: &SourceSession, object: Address) -> Result<(Address, HandleWire)> {
    let handle = s.required(object, 4)?;
    let bytes = s.allocation_bytes(handle)?;
    ensure!(
        bytes.len() == 24,
        "inventory handle must be 24 bytes at {handle:?}"
    );
    let h: HandleWire = wire(bytes)?;
    ensure!(
        h.replacements.count <= 256,
        "inventory replacement count bound"
    );
    Ok((handle, h))
}
fn definition(s: &SourceSession, object: Address) -> Result<InventoryDefinition> {
    ensure!(
        u32_at(s.bytes(object, 4)?, 0)? == 18,
        "inventory definition object is not an item"
    );
    let (handle, h) = handle(s, object)?;
    let member = s.required(handle, 0)?;
    let item: ItemWire = wire(s.bytes(member, 32)?)?;
    ensure!(h.selected <= 1, "inventory selected flag is not Boolean");
    // The special-resource definition has a verified null kind. Its source
    // operations address it through the special root, without kind dispatch.
    // Nonzero fields still require an actual source relocation.
    let kind = s
        .pointer(member, 12)?
        .map(|text| -> Result<String> {
            let bytes = &s.block(text)?.data[text.offset..];
            let end = bytes
                .iter()
                .take(4097)
                .position(|b| *b == 0)
                .context("inventory kind is unterminated")?;
            ensure!(end > 0, "inventory kind is empty");
            super::dialogue::decode_cp1252(&bytes[..end])
        })
        .transpose()?;
    Ok(InventoryDefinition {
        source: s.source_identity(object)?,
        id: item.id,
        kind,
        maximum: item.maximum,
        quantity: h.quantity,
        selected: h.selected != 0,
        availability: item.availability,
        management: item.management,
        pickup_category: None,
        replacements: Vec::new(),
        unknown: [
            item.condition,
            item.permissions[0],
            item.permissions[1],
            item.unknown[0],
            item.unknown[1],
            h.unknown,
        ],
    })
}
fn replacements(
    s: &SourceSession,
    object: Address,
    identities: &BTreeMap<Address, crate::model::ai::SourceObjectIdentity>,
) -> Result<Vec<InventoryReplacement>> {
    let (owner, h) = handle(s, object)?;
    let mut next = s.pointer(owner, 12)?;
    let last = s.pointer(owner, 16)?;
    let mut previous = None;
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    while let Some(a) = next {
        ensure!(
            seen.insert(a) && out.len() < h.replacements.count as usize,
            "inventory replacement cycle/count at {a:?}"
        );
        let bytes = s.allocation_bytes(a)?;
        ensure!(
            bytes.len() == 20,
            "inventory replacement must be 20 bytes at {a:?}"
        );
        let _: ReplacementWire = wire(bytes)?;
        ensure!(
            s.pointer(a, 12)? == previous,
            "inventory replacement previous differs at {a:?}"
        );
        ensure!(
            s.required(a, 16)? == owner,
            "inventory replacement owner differs at {a:?}"
        );
        let endpoint = |offset| -> Result<Option<crate::model::ai::SourceObjectIdentity>> {
            s.pointer(a, offset)?.map(|item| identities.get(&item).cloned()
                .with_context(|| format!("inventory replacement endpoint at {a:?}+{offset} leaves catalogue: {item:?}")))
                .transpose()
        };
        out.push(InventoryReplacement {
            source: s.source_identity(a)?,
            from: endpoint(0)?,
            to: endpoint(4)?,
        });
        previous = Some(a);
        next = s.pointer(a, 8)?;
    }
    ensure!(
        out.len() == h.replacements.count as usize && previous == last,
        "inventory replacement tail/count differs at {owner:?}"
    );
    Ok(out)
}
pub(crate) fn import(game: &Path, selection: &Selection) -> Result<InventoryCatalog> {
    let s = SourceSession::open(game, selection)?;
    let folder = game.join("Gamedata/World/Levels");
    let gpt = file(&folder.join("Fix.gpt"))?;
    let refs = pointer_file(&file(&folder.join("Fix.rtp"))?)?;
    // 0041bec0: main catalogue, category tree, row, then special resource.
    let root = |slot| -> Result<Address> {
        let value = u32_at(&gpt, slot)?;
        s.resolve(
            Owner::Fix,
            *refs
                .get(&value)
                .context("inventory root lacks RTP binding")?,
            value,
        )
    };
    let objects = children(&s, root(18152).context("resolve inventory catalogue root")?)
        .context("traverse inventory catalogue root")?;
    ensure!(objects.len() == 62, "unrecognized Hype inventory catalogue");
    let mut items = objects
        .iter()
        .enumerate()
        .map(|(index, &a)| {
            let item = definition(&s, a)
                .with_context(|| format!("import inventory catalogue item {index} at {a:?}"))?;
            ensure!(
                item.kind.is_some(),
                "inventory catalogue item {index} at {a:?} has no kind"
            );
            Ok(item)
        })
        .collect::<Result<Vec<_>>>()?;
    let identities: BTreeMap<_, _> = objects
        .iter()
        .copied()
        .zip(items.iter().map(|i| i.source.clone()))
        .collect();
    let special_root = root(18164).context("resolve special inventory resource root")?;
    let special = children(&s, special_root).context("traverse special inventory resource root")?;
    ensure!(
        special.len() == 1,
        "unrecognized special inventory resource root"
    );
    let mut gold = definition(&s, special[0])
        .with_context(|| format!("import special inventory resource at {:?}", special[0]))?;
    let mut replacement_items = identities.clone();
    replacement_items.insert(special[0], gold.source.clone());
    for (index, (&object, item)) in objects.iter().zip(&mut items).enumerate() {
        item.replacements = replacements(&s, object, &replacement_items).with_context(|| {
            format!("import inventory catalogue item {index} replacements at {object:?}")
        })?;
    }
    gold.replacements = replacements(&s, special[0], &replacement_items)
        .context("import special inventory resource replacements")?;
    let mut categories = Vec::new();
    for category in children(&s, root(18156)?)? {
        ensure!(
            u32_at(s.bytes(category, 4)?, 0)? == 19,
            "inventory category is not a box"
        );
        let children = children(&s, category)?;
        ensure!(!children.is_empty(), "inventory category has no header");
        let nested = children
            .get(1)
            .map(|a| s.bytes(*a, 4).and_then(|b| u32_at(b, 0)))
            .transpose()?
            == Some(19);
        if nested {
            for group in children.into_iter().skip(1) {
                admission(&s, group, &identities, &mut categories)?;
            }
        } else {
            admission(&s, category, &identities, &mut categories)?;
        }
    }
    let value = InventoryCatalog {
        path: AssetPath::new("fix:/inventory/gameplay")?,
        items,
        gold,
        gold_category: s.source_identity(special_root)?,
        categories,
    };
    value.validate()?;
    Ok(value)
}
fn admission(
    s: &SourceSession,
    category: Address,
    ids: &BTreeMap<Address, crate::model::ai::SourceObjectIdentity>,
    out: &mut Vec<InventoryAdmission>,
) -> Result<()> {
    let mut slots = Vec::new();
    for slot in children(s, category)?.into_iter().skip(1) {
        slots.push(
            s.pointer(slot, 4)?
                .map(|item| {
                    ids.get(&item)
                        .cloned()
                        .context("inventory slot item leaves catalogue")
                })
                .transpose()?,
        );
    }
    out.push(InventoryAdmission {
        source: s.source_identity(category)?,
        slots,
    });
    Ok(())
}
pub(super) fn stock(s: &SourceSession, perso: Address) -> Result<Option<InventoryStock>> {
    let Some(a) = s.pointer(perso, 56)? else {
        return Ok(None);
    };
    let value: StockWire = wire(s.bytes(a, 20)?)?;
    Ok(Some(InventoryStock {
        category: s.pointer(a, 8)?.map(|a| s.source_identity(a)).transpose()?,
        item: s
            .pointer(a, 12)?
            .map(|a| s.source_identity(a))
            .transpose()?,
        count: value.count,
        unknown: value.unknown,
    }))
}
