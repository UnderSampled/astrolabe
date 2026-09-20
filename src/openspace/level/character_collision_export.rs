//! Reconstruct reusable collision profiles once and bind per-instance selections.
use super::{
    container::{emit_relocations, encoded},
    geometry::{GeometryExport, Layout, ReferenceTarget},
    records::*,
};
use crate::model::{AssetPath, PackageRole, character::*, io::Content};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct CollisionProfileTargets {
    pub geometry: Option<ReferenceTarget>,
    pub activations: Option<ReferenceTarget>,
    pub selections: BTreeMap<u32, ReferenceTarget>,
}
pub struct CollisionProfileExport {
    pub fragment: GeometryExport,
    pub targets: CollisionProfileTargets,
}
fn layout(role: PackageRole, base: u32) -> Result<Layout> {
    ensure!(
        base > 1 && base.is_multiple_of(4),
        "invalid character collision base"
    );
    let block = match role {
        PackageRole::Fix => BlockKey(5, 0),
        PackageRole::Level => BlockKey(5, 1),
        _ => anyhow::bail!("character collisions require Fix or Level context"),
    };
    Ok(Layout {
        bytes: Vec::new(),
        ranges: Vec::new(),
        rows: Vec::new(),
        base,
        block,
    })
}
fn target(out: &Layout, offset: usize) -> Result<ReferenceTarget> {
    Ok(ReferenceTarget {
        address: out
            .base
            .checked_add(offset.try_into()?)
            .context("collision profile address overflow")?,
        block: (out.block.0, out.block.1),
    })
}
fn finish(mut out: Layout) -> Result<GeometryExport> {
    out.rows.sort_by_key(|r| r.value);
    Ok(GeometryExport {
        rtb: emit_relocations(out.block, &out.rows)?, pointer_count: out.rows.len(),
        bytes: out.bytes, ranges: out.ranges,
        coverage: vec!["Shared collision profiles and per-instance active selections reconstructed; complete Perso/package assembly remains separate.".into()],
    })
}
pub fn write_character_collisions(
    value: &CharacterCollisions,
    profiles: &BTreeMap<AssetPath, CollisionProfileTargets>,
    role: PackageRole,
    base: u32,
) -> Result<GeometryExport> {
    value.validate()?;
    let mut out = layout(role, base)?;
    out.allocate(
        "collision_set",
        &encoded(&CharacterCollisionsWire {
            geometries: [0; 4],
            activations: [0; 4],
            active: [0; 4],
            unknown: value.unknown,
        })?,
    )?;
    for (i, group) in value.groups.iter().enumerate() {
        let Some(reference) = &group.profile else {
            continue;
        };
        let targets = profiles
            .get(&reference.path)
            .context("missing collision profile placement")?;
        if let Some(target) = targets.geometry {
            out.external_pointer(i * 4, target)?;
        }
        if let Some(target) = targets.activations {
            out.external_pointer(16 + i * 4, target)?;
        }
        if let Some(id) = group.active {
            out.external_pointer(
                [32, 36, 44, 40][i],
                *targets
                    .selections
                    .get(&id)
                    .context("missing active collision selection")?,
            )?;
        }
    }
    finish(out)
}
pub fn write_collision_profile(
    value: &CollisionProfile,
    geometries: &BTreeMap<AssetPath, ReferenceTarget>,
    base: u32,
) -> Result<CollisionProfileExport> {
    value.validate()?;
    if let Some(activations) = &value.activations {
        // 0042daf0 indexes the list by family-state position.
        ensure!(
            activations
                .iter()
                .enumerate()
                .all(|(index, a)| usize::from(a.state) == index),
            "OpenSpace collision activations must follow dense family-state order"
        );
    }
    let mut out = layout(value.path.role(), base)?;
    let mut targets = CollisionProfileTargets::default();
    let empty = || ListWire {
        first: 0,
        last: 0,
        count: 0,
    };
    let mut selections = BTreeMap::new();
    for selection in &value.selections {
        let header = out.allocate(
            format!("selections/{}", selection.id),
            &encoded(&CollisionSlotsWire {
                list: empty(),
                unknown: selection.unknown,
            })?,
        )?;
        let mut entries = Vec::new();
        for (i, zone) in selection.zones.iter().enumerate() {
            entries.push(out.allocate(
                format!("selections/{}/zones/{i}", selection.id),
                &encoded(&CollisionZoneWire {
                    next: 0,
                    previous: 0,
                    header: 0,
                    index: zone.index,
                    unknown: zone.unknown,
                })?,
            )?);
        }
        out.linked_list(header, &entries, 0)?;
        selections.insert(selection.id, header);
        targets
            .selections
            .insert(selection.id, target(&out, header)?);
    }
    if let Some(geometry) = &value.geometry {
        let header = out.allocate(
            "geometry",
            &encoded(&CollisionSlotsWire {
                list: empty(),
                unknown: geometry.unknown,
            })?,
        )?;
        targets.geometry = Some(target(&out, header)?);
        let mut entries = Vec::new();
        for (i, slot) in geometry.slots.iter().enumerate() {
            let entry = out.allocate(
                format!("slots/{i}"),
                &encoded(&CollisionGeometryLinkWire {
                    next: 0,
                    previous: 0,
                    header: 0,
                    geometry: 0,
                })?,
            )?;
            if let Some(reference) = slot {
                out.external_pointer(
                    entry + 12,
                    *geometries
                        .get(&reference.path)
                        .context("missing character collision geometry placement")?,
                )?;
            }
            entries.push(entry);
        }
        out.linked_list(header, &entries, 0)?;
    }
    if let Some(activations) = &value.activations {
        let header = out.allocate("activations", &encoded(&empty())?)?;
        targets.activations = Some(target(&out, header)?);
        let mut entries = Vec::new();
        for (i, activation) in activations.iter().enumerate() {
            let entry = out.allocate(
                format!("activations/{i}"),
                &encoded(&CollisionActivationWire {
                    next: 0,
                    previous: 0,
                    header: 0,
                    selection: 0,
                    state: activation.state,
                    unknown: activation.unknown,
                })?,
            )?;
            if let Some(id) = activation.selection {
                out.pointer(entry + 12, selections[&id])?;
            }
            entries.push(entry);
        }
        out.linked_list(header, &entries, 0)?;
    }
    Ok(CollisionProfileExport {
        fragment: finish(out)?,
        targets,
    })
}
