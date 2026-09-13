//! Controlled ENV/VET/fog reconstruction with explicit external component layout.
use super::{
    GeometryExport, ReferenceTarget,
    container::{emit_relocations, encoded},
    environment::{EnvironmentWire, VisualEnvironmentWire, write_fog},
    geometry::Layout,
    records::BlockKey,
};
use crate::model::{AssetPath, environment::*, io::Content};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

/// Unpromoted components must have caller-resolved placements. Keys are the
/// owning environment (mechanics/sound) or visual environment (water) asset.
#[derive(Default)]
pub struct EnvironmentDependencies {
    pub mechanics: BTreeMap<AssetPath, ReferenceTarget>,
    pub sound: BTreeMap<AssetPath, ReferenceTarget>,
    pub water: BTreeMap<AssetPath, ReferenceTarget>,
}
pub fn write_environments(
    environments: &[Environment],
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
    for (v, p) in environments.iter().zip(env_offsets) {
        for (field, present, layout) in [
            (0, v.unresolved_mechanics, &dependencies.mechanics),
            (4, v.unresolved_sound, &dependencies.sound),
        ] {
            ensure!(
                present == layout.contains_key(&v.path),
                "environment component/layout mismatch for {} at field {field}",
                v.path
            );
            if present {
                out.external_pointer(p + field, layout[&v.path])?;
            }
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
        coverage:vec!["Shared ENV, VET and fog records rebuilt with derived pointers; mechanics, sound and water targets require explicit caller layouts and their content codecs remain separate.".into()]})
}
