//! Controlled ENV/physical/VET/fog reconstruction with explicit external layout
//! only for still-unpromoted components.
use super::{
    GeometryExport, ReferenceTarget,
    container::{emit_relocations, encoded},
    environment::{EnvironmentWire, VisualEnvironmentWire, write_fog, write_physical_environment},
    geometry::Layout,
    records::BlockKey,
};
use crate::model::{AssetPath, environment::*, io::Content};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

/// Unpromoted components must have caller-resolved placements. Keys are the
/// owning environment (sound) or visual environment (water) asset.
#[derive(Default)]
pub struct EnvironmentDependencies {
    pub sound: BTreeMap<AssetPath, ReferenceTarget>,
    pub water: BTreeMap<AssetPath, ReferenceTarget>,
}
pub fn write_environments(
    environments: &[Environment],
    physicals: &[PhysicalEnvironment],
    visuals: &[VisualEnvironment],
    fogs: &[Fog],
    dependencies: &EnvironmentDependencies,
    base: u32,
) -> Result<GeometryExport> {
    ensure!(
        base != 0 && base.is_multiple_of(4),
        "environment base must be aligned/nonzero"
    );
    let mut out = Layout {
        bytes: Vec::new(),
        ranges: Vec::new(),
        rows: Vec::new(),
        base,
        block: BlockKey(6, 2),
    };
    let mut paths = std::collections::BTreeSet::new();
    let mut env_offsets = Vec::new();
    let mut visual_offsets = BTreeMap::new();
    let mut fog_offsets = BTreeMap::new();
    let mut physical_offsets = BTreeMap::new();
    for v in environments {
        v.validate()?;
        ensure!(
            paths.insert(&v.path),
            "duplicate environment asset identity"
        );
        env_offsets.push(out.allocate(
            v.path.as_str(),
            &encoded(&EnvironmentWire {
                mechanics: 0,
                sound: 0,
                visual: 0,
            })?,
        )?);
    }
    for v in visuals {
        v.validate()?;
        ensure!(
            paths.insert(&v.path),
            "duplicate environment asset identity"
        );
        visual_offsets.insert(
            v.path.clone(),
            out.allocate(
                v.path.as_str(),
                &encoded(&VisualEnvironmentWire { water: 0, fog: 0 })?,
            )?,
        );
    }
    for v in fogs {
        ensure!(
            paths.insert(&v.path),
            "duplicate environment asset identity"
        );
        fog_offsets.insert(
            v.path.clone(),
            out.allocate(v.path.as_str(), &write_fog(v)?)?,
        );
    }
    for v in physicals {
        v.validate()?;
        ensure!(
            paths.insert(&v.path),
            "duplicate physical environment asset identity"
        );
        physical_offsets.insert(
            v.path.clone(),
            out.allocate(v.path.as_str(), &write_physical_environment(v)?)?,
        );
    }
    for (v, p) in environments.iter().zip(env_offsets) {
        if let Some(reference) = &v.physical {
            out.pointer(
                p,
                *physical_offsets
                    .get(&reference.path)
                    .context("missing physical environment in export graph")?,
            )?;
        }
        ensure!(
            v.unresolved_sound == dependencies.sound.contains_key(&v.path),
            "environment sound/layout mismatch for {}",
            v.path
        );
        if v.unresolved_sound {
            out.external_pointer(p + 4, dependencies.sound[&v.path])?;
        }
        if let Some(reference) = &v.visual {
            out.pointer(
                p + 8,
                *visual_offsets
                    .get(&reference.path)
                    .context("missing visual environment in export graph")?,
            )?;
        }
    }
    for v in visuals {
        let p = visual_offsets[&v.path];
        ensure!(
            v.unresolved_water == dependencies.water.contains_key(&v.path),
            "water component/layout mismatch for {}",
            v.path
        );
        if v.unresolved_water {
            out.external_pointer(p, dependencies.water[&v.path])?;
        }
        if let Some(reference) = &v.fog {
            out.pointer(
                p + 4,
                *fog_offsets
                    .get(&reference.path)
                    .context("missing fog in export graph")?,
            )?;
        }
    }
    out.rows.sort_by_key(|r| r.value);
    Ok(GeometryExport {rtb:emit_relocations(out.block,&out.rows)?,pointer_count:out.rows.len(),bytes:out.bytes,ranges:out.ranges,
        coverage:vec!["Shared ENV, physical, VET and fog records rebuilt with derived pointers; sound and water targets still require explicit caller layouts.".into()]})
}
