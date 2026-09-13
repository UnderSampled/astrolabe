//! Controlled sector reconstruction. Scene SO and shared-light placements are caller inputs.
use super::{
    GeometryExport, ReferenceTarget,
    container::{emit_relocations, encoded},
    geometry::{Layout, float_data, short_data},
    records::*,
};
use crate::model::{AssetPath, io::Content, scene::*};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

/// Regenerate supported sector records, portals, names and ordered link lists.
/// Targets identify SO records, never sector-data records. Surface,
/// stream and sound-event lists are outside this fragment and emitted empty.
/// This API does not assemble a loadable SNA or replay source layout decisions.
pub fn write_sectors(
    scene: &Scene,
    nodes: &BTreeMap<String, ReferenceTarget>,
    lights: &BTreeMap<AssetPath, ReferenceTarget>,
    environments: &BTreeMap<AssetPath, ReferenceTarget>,
    base: u32,
) -> Result<GeometryExport> {
    scene.validate()?;
    ensure!(
        base != 0 && base.is_multiple_of(4),
        "sector base must be aligned/nonzero"
    );
    let mut out = Layout {
        bytes: Vec::new(),
        rows: Vec::new(),
        ranges: Vec::new(),
        base,
        block: BlockKey(6, 2),
    };
    for node in &scene.nodes {
        let Some(sector) = &node.sector else {
            continue;
        };
        let prefix = format!("sectors/{}", node.id);
        let header = out.allocate(
            &prefix,
            &encoded(&SectorWire {
                border: 0,
                lists: std::array::from_fn(|_| ListWire {
                    first: 0,
                    last: 0,
                    count: 0,
                }),
                virtual_sector: sector.virtual_sector,
                unknown: sector.unknown,
                activation: sector.activation,
                name: 0,
            })?,
        )?;
        let mut name: Vec<u8> = sector.name.chars().map(|c| c as u8).collect();
        name.push(0);
        let p = out.allocate(format!("{prefix}/name"), &name)?;
        out.pointer(header + 156, p)?;
        if let Some(boundary) = &sector.boundary {
            let p = emit_boundary(&mut out, &format!("{prefix}/boundary"), boundary, nodes)?;
            out.pointer(header, p)?;
        }
        for (field, ids) in [
            (28, &sector.persos),
            (100, &sector.collision),
            (112, &sector.activity),
        ] {
            let mut entries = Vec::new();
            for (i, id) in ids.iter().enumerate() {
                let p = out.allocate(
                    format!("{prefix}/list_{field}/{i}"),
                    &encoded(&NodeLinkWire {
                        target: 0,
                        next: 0,
                        previous: 0,
                        header: 0,
                    })?,
                )?;
                out.external_pointer(
                    p,
                    *nodes
                        .get(id)
                        .with_context(|| format!("missing scene SO layout for {id}"))?,
                )?;
                entries.push(p);
            }
            out.linked_list(header + field, &entries, 4)?;
        }
        for (field, links) in [(76, &sector.graphics), (88, &sector.optional_graphics)] {
            let mut entries = Vec::new();
            for (i, link) in links.iter().enumerate() {
                let p = out.allocate(
                    format!("{prefix}/list_{field}/{i}"),
                    &encoded(&GraphicLinkWire {
                        level: link.level,
                        unknown: link.unknown,
                        sector: 0,
                        next: 0,
                        previous: 0,
                        header: 0,
                    })?,
                )?;
                out.external_pointer(
                    p + 4,
                    *nodes
                        .get(&link.sector)
                        .context("missing graphics SO layout")?,
                )?;
                entries.push(p);
            }
            out.linked_list(header + field, &entries, 8)?;
        }
        let mut entries = Vec::new();
        for (i, link) in sector.sound.iter().enumerate() {
            let p = out.allocate(
                format!("{prefix}/list_124/{i}"),
                &encoded(&SoundLinkWire {
                    sector: 0,
                    volume: link.volume,
                    next: 0,
                    previous: 0,
                    header: 0,
                })?,
            )?;
            out.external_pointer(
                p,
                *nodes
                    .get(&link.sector)
                    .context("missing sound-propagation SO layout")?,
            )?;
            entries.push(p);
        }
        out.linked_list(header + 124, &entries, 8)?;
        for (field, refs, targets) in [
            (
                40,
                sector
                    .static_lights
                    .iter()
                    .map(|r| &r.path)
                    .collect::<Vec<_>>(),
                lights,
            ),
            (
                52,
                sector.dynamic_lights.iter().map(|r| &r.path).collect(),
                lights,
            ),
            (
                4,
                sector.environments.iter().map(|r| &r.path).collect(),
                environments,
            ),
        ] {
            let mut entries = Vec::new();
            for (i, reference) in refs.iter().enumerate() {
                let p = out.allocate(
                    format!("{prefix}/list_{field}/{i}"),
                    &encoded(&NodeLinkWire {
                        target: 0,
                        next: 0,
                        previous: 0,
                        header: 0,
                    })?,
                )?;
                out.external_pointer(
                    p,
                    *targets
                        .get(*reference)
                        .context("missing sector asset layout")?,
                )?;
                entries.push(p);
            }
            out.linked_list(header + field, &entries, 4)?;
        }
    }
    out.rows.sort_by_key(|r| r.value);
    Ok(GeometryExport { rtb: emit_relocations(out.block, &out.rows)?, pointer_count: out.rows.len(), bytes: out.bytes, ranges: out.ranges, coverage: vec![
        "Sector portals, names, flags, environment/perso/light membership and graphics/optional-graphics/collision/activity/sound-propagation links rebuilt with derived list topology and caller-resolved SO/light/environment targets.".into(),
        "Surface, stream and sound-event lists, scene SO records and full SNA assembly are outside this controlled sector fragment.".into(),
    ] })
}

fn emit_boundary(
    out: &mut Layout,
    prefix: &str,
    boundary: &SectorBoundary,
    nodes: &BTreeMap<String, ReferenceTarget>,
) -> Result<usize> {
    let g = out.allocate(
        prefix,
        &encoded(&GeometryWire {
            point_count: boundary.positions.len().try_into()?,
            point_reserved: 0,
            points: 0,
            normals: 0,
            point_materials: 0,
            unknown_10: 0,
            element_count: boundary.surfaces.len().try_into()?,
            element_reserved: 0,
            element_types: 0,
            elements: 0,
            spatial_data: 0,
            unknown_tail: [0; 3],
        })?,
    )?;
    let p = out.allocate(
        format!("{prefix}/positions"),
        &float_data(&boundary.positions),
    )?;
    out.pointer(g + 4, p)?;
    if !boundary.normals.is_empty() {
        let p = out.allocate(format!("{prefix}/normals"), &float_data(&boundary.normals))?;
        out.pointer(g + 8, p)?;
    }
    if boundary.surfaces.is_empty() {
        return Ok(g);
    }
    let p = out.allocate(
        format!("{prefix}/kinds"),
        &short_data(&vec![[1]; boundary.surfaces.len()]),
    )?;
    out.pointer(g + 24, p)?;
    let table = out.allocate(
        format!("{prefix}/surfaces"),
        &vec![0; boundary.surfaces.len() * 4],
    )?;
    out.pointer(g + 28, table)?;
    for (i, s) in boundary.surfaces.iter().enumerate() {
        let prefix = format!("{prefix}/{i}");
        let p = out.allocate(
            &prefix,
            &encoded(&TrianglesWire {
                material: 0,
                triangle_count: s.triangles.len().try_into()?,
                uv_count: s.texcoords.len().try_into()?,
                triangles: 0,
                uv_indices: 0,
                normals: 0,
                uvs: 0,
                point_count: s.vertex_indices.len().try_into()?,
                point_reserved: 0,
                points: 0,
            })?,
        )?;
        out.pointer(table + i * 4, p)?;
        if let Some(id) = &s.sector {
            out.external_pointer(
                p,
                *nodes
                    .get(id)
                    .context("missing boundary-neighbor SO layout")?,
            )?;
        }
        for (field, label, bytes) in [
            (8, "triangles", short_data(&s.triangles)),
            (12, "uv_indices", short_data(&s.uv_indices)),
            (16, "normals", float_data(&s.normals)),
            (20, "texcoords", float_data(&s.texcoords)),
            (
                28,
                "vertices",
                short_data(&s.vertex_indices.iter().map(|&v| [v]).collect::<Vec<_>>()),
            ),
        ] {
            if !bytes.is_empty() {
                let target = out.allocate(format!("{prefix}/{label}"), &bytes)?;
                out.pointer(p + field, target)?;
            }
        }
    }
    Ok(g)
}
