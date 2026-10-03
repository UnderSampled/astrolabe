use super::{
    container::{encoded, wire},
    geometry::{Layout, ReferenceTarget, float_data},
    records::*,
    session::SourceSession,
};
use crate::model::{AssetPath, AssetRef, geometry::Material, io::Content, sprite::*};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

pub(super) fn read_element(
    session: &SourceSession,
    address: Address,
    index: usize,
    materials: &mut Vec<Material>,
    material_ids: &mut BTreeMap<Address, AssetPath>,
    notes: &mut Vec<String>,
) -> Result<(SpriteElement, Vec<Sprite>)> {
    let h: SpriteElementWire = wire(session.bytes(address, 8)?)?;
    let table = if h.count == 0 {
        address
    } else {
        session.required(address, 4)?
    };
    let mut definitions = BTreeMap::new();
    let mut sprites = Vec::new();
    for i in 0..h.count as usize {
        let entry = Address {
            offset: table.offset + i * 44,
            ..table
        };
        let p: IndexedSpriteWire = wire(session.bytes(entry, 44)?)?;
        let definition = session.required(entry, 4)?;
        let index = session.allocation_index(definition)?;
        let role = if definition.owner == Owner::Fix {
            "fix"
        } else {
            "level"
        };
        let path = AssetPath::new(format!("{role}:/sprites/{index}"))?;
        if !definitions.contains_key(&path) {
            let value = read_definition(session, definition, materials, material_ids, notes)?;
            definitions.insert(path.clone(), value);
        }
        sprites.push(IndexedSprite {
            center: p.center,
            unknown: p.unknown,
            sprite: AssetRef::from_path(path),
            size: p.size,
            constraint: p.constraint,
            uv_origin_extent: p.uv_origin_extent,
        });
    }
    Ok((
        SpriteElement {
            source_index: Some(index.try_into()?),
            unknown: h.unknown,
            sprites,
        },
        definitions.into_values().collect(),
    ))
}

pub(super) fn read_definition(
    session: &SourceSession,
    definition: Address,
    materials: &mut Vec<Material>,
    material_ids: &mut BTreeMap<Address, AssetPath>,
    notes: &mut Vec<String>,
) -> Result<Sprite> {
    let index = session.allocation_index(definition)?;
    let role = if definition.owner == Owner::Fix {
        "fix"
    } else {
        "level"
    };
    let path = AssetPath::new(format!("{role}:/sprites/{index}"))?;
    let d: SpriteWire = wire(session.bytes(definition, 24)?)?;
    ensure!(d.count > 0, "empty sprite definition");
    let modes = session.short_arrays::<1>(session.required(definition, 4)?, d.count as usize)?;
    let thresholds = session.floats::<1>(session.required(definition, 8)?, d.count as usize)?;
    let sizes = session.floats::<2>(session.required(definition, 12)?, d.count as usize)?;
    let displacements = session.floats::<2>(session.required(definition, 16)?, d.count as usize)?;
    let material_table = session.required(definition, 20)?;
    let mut views = Vec::new();
    for v in 0..d.count as usize {
        let material = session
            .pointer(material_table, v * 4)?
            .map(|m| {
                super::import::load_material(
                    session,
                    m,
                    &path.to_string(),
                    materials,
                    material_ids,
                    notes,
                )
            })
            .transpose()?;
        views.push(SpriteView {
            mode: modes[v][0],
            threshold: thresholds[v][0],
            half_size: sizes[v],
            displacement: displacements[v],
            material,
        });
    }
    let value = Sprite {
        path: path.clone(),
        views,
        unknown: d.unknown,
    };
    value.validate()?;
    Ok(value)
}

pub(super) fn emit_definition(
    out: &mut Layout,
    sprite: &Sprite,
    materials: &BTreeMap<AssetPath, ReferenceTarget>,
) -> Result<usize> {
    sprite.validate()?;
    let h = out.allocate(
        format!("sprites/{}/definition", sprite.path.file()),
        &encoded(&SpriteWire {
            count: sprite.views.len().try_into()?,
            unknown: sprite.unknown,
            modes: 0,
            thresholds: 0,
            sizes: 0,
            displacements: 0,
            materials: 0,
        })?,
    )?;
    for (field, name, data) in [
        (
            4,
            "modes",
            sprite
                .views
                .iter()
                .flat_map(|v| v.mode.to_le_bytes())
                .collect(),
        ),
        (
            8,
            "thresholds",
            float_data(
                &sprite
                    .views
                    .iter()
                    .map(|v| [v.threshold])
                    .collect::<Vec<_>>(),
            ),
        ),
        (
            12,
            "sizes",
            float_data(&sprite.views.iter().map(|v| v.half_size).collect::<Vec<_>>()),
        ),
        (
            16,
            "displacements",
            float_data(
                &sprite
                    .views
                    .iter()
                    .map(|v| v.displacement)
                    .collect::<Vec<_>>(),
            ),
        ),
    ] {
        let p = out.allocate(format!("sprites/{}/{name}", sprite.path.file()), &data)?;
        out.pointer(h + field, p)?;
    }
    let table = out.allocate(
        format!("sprites/{}/materials", sprite.path.file()),
        &vec![0; sprite.views.len() * 4],
    )?;
    out.pointer(h + 20, table)?;
    for (i, view) in sprite.views.iter().enumerate() {
        if let Some(m) = &view.material {
            out.external_pointer(
                table + i * 4,
                *materials
                    .get(&m.path)
                    .context("sprite material missing from layout")?,
            )?;
        }
    }
    Ok(h)
}
pub(super) fn emit_element(
    out: &mut Layout,
    element: &SpriteElement,
    index: usize,
    targets: &BTreeMap<AssetPath, ReferenceTarget>,
) -> Result<usize> {
    let h = out.allocate(
        format!("sprite_elements/{index}"),
        &encoded(&SpriteElementWire {
            count: element.sprites.len().try_into()?,
            unknown: element.unknown,
            entries: 0,
        })?,
    )?;
    let mut data = Vec::new();
    for s in &element.sprites {
        data.extend(encoded(&IndexedSpriteWire {
            center: s.center,
            unknown: s.unknown,
            sprite: 0,
            size: s.size,
            constraint: s.constraint,
            uv_origin_extent: s.uv_origin_extent,
        })?);
    }
    if !data.is_empty() {
        let p = out.allocate(format!("sprite_elements/{index}/entries"), &data)?;
        out.pointer(h + 4, p)?;
        for (i, s) in element.sprites.iter().enumerate() {
            out.external_pointer(
                p + i * 44 + 4,
                *targets
                    .get(&s.sprite.path)
                    .context("sprite definition missing from layout")?,
            )?;
        }
    }
    Ok(h)
}
