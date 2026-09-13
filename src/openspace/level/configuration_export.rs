//! Rebuild both mutable and reset object tables from their useful values.
use super::{
    container::{emit_relocations, encoded},
    geometry::{GeometryExport, Layout, ReferenceTarget, float_data},
    records::*,
};
use crate::model::{AssetPath, PackageRole, character::*, io::Content};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

/// Controlled fragment whose root is a detached ObjectList. Model targets must
/// identify physical objects (PO), not instantiated wrappers (IPO). Family list
/// attachment and enclosing package assembly belong to the caller.
pub fn write_configuration(
    value: &ObjectList,
    models: &BTreeMap<AssetPath, ReferenceTarget>,
    actions: &BTreeMap<AssetPath, ReferenceTarget>,
    base: u32,
) -> Result<GeometryExport> {
    value.validate()?;
    ensure!(
        base > 1 && base.is_multiple_of(4),
        "invalid configuration base"
    );
    let block = match value.path.role() {
        PackageRole::Fix => BlockKey(5, 0),
        PackageRole::Level => BlockKey(5, 1),
        _ => anyhow::bail!("invalid configuration package role"),
    };
    let mut out = Layout {
        bytes: Vec::new(),
        ranges: Vec::new(),
        rows: Vec::new(),
        base,
        block,
    };
    emit_configuration(&mut out, value, models, actions, "")?;
    out.rows.sort_by_key(|r| r.value);
    Ok(GeometryExport {
        bytes: out.bytes, rtb: emit_relocations(block, &out.rows)?, ranges: out.ranges,
        pointer_count: out.rows.len(),
        coverage: vec!["Configuration entries, optional reset snapshot, per-entry scales and shared model/action references regenerated from values. The root is detached; enclosing family links/package assembly and original scale-allocation sharing remain unsupported.".into()],
    })
}

pub(super) fn emit_configuration(
    out: &mut Layout,
    value: &ObjectList,
    models: &BTreeMap<AssetPath, ReferenceTarget>,
    actions: &BTreeMap<AssetPath, ReferenceTarget>,
    prefix: &str,
) -> Result<usize> {
    value.validate()?;
    let root = out.allocate(
        format!("{prefix}configuration"),
        &encoded(&ObjectListWire {
            next: 0,
            previous: 0,
            header: 0,
            parts: 0,
            reset_parts: 0,
            count: value.parts.len().try_into()?,
            unknown: value.unknown,
        })?,
    )?;
    emit_parts(
        out,
        root + 12,
        &format!("{prefix}parts"),
        &value.parts,
        models,
        actions,
    )?;
    if let Some(parts) = &value.reset_parts {
        emit_parts(
            out,
            root + 16,
            &format!("{prefix}reset_parts"),
            parts,
            models,
            actions,
        )?;
    }
    Ok(root)
}

fn emit_parts(
    out: &mut Layout,
    site: usize,
    name: &str,
    parts: &[ObjectPart],
    models: &BTreeMap<AssetPath, ReferenceTarget>,
    actions: &BTreeMap<AssetPath, ReferenceTarget>,
) -> Result<()> {
    if parts.is_empty() {
        return Ok(());
    }
    let table = out.allocate(name, &vec![0; parts.len() * 20])?;
    out.pointer(site, table)?;
    for (i, part) in parts.iter().enumerate() {
        let entry = table + i * 20;
        out.bytes[entry..entry + 20].copy_from_slice(&encoded(&ObjectPartWire {
            scale: 0,
            content: 0,
            unknown: part.unknown[..4].try_into()?,
            kind: match part.kind {
                ObjectPartKind::Absent => -1,
                ObjectPartKind::Model => 0,
                ObjectPartKind::Action => 5,
            },
            tail: part.unknown[4..].try_into()?,
        })?);
        if let Some(scale) = part.scale {
            let target = out.allocate(format!("{name}/{i}/scale"), &float_data(&[scale]))?;
            out.pointer(entry, target)?;
        }
        if let Some(reference) = &part.instance {
            out.external_pointer(
                entry + 4,
                *models.get(&reference.path).with_context(|| {
                    format!("configuration physical object missing: {}", reference.path)
                })?,
            )?;
        }
        if let Some(reference) = &part.action {
            out.external_pointer(
                entry + 4,
                *actions
                    .get(&reference.path)
                    .with_context(|| format!("configuration action missing: {}", reference.path))?,
            )?;
        }
    }
    Ok(())
}
