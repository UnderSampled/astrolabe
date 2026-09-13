//! Geometry records and buffer placement derived from current model values.
use super::container::{MAX_BLOCK, emit_relocations, encoded};
use super::records::*;
use crate::model::{AssetPath, geometry::Model};
use anyhow::{Result, anyhow, ensure};
use serde::Serialize;
use std::collections::BTreeMap;
/// Explicit layout inputs for references owned by other reconstructed assets.
#[derive(Debug, Clone, Copy)]
pub struct ReferenceTarget {
    pub address: u32,
    pub block: (u8, u8),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EmittedRange {
    pub name: String,
    pub offset: usize,
    pub length: usize,
}
#[derive(Debug, Clone)]
pub struct GeometryExport {
    /// Standalone controlled-layout geometry records/buffers, not a complete loadable SNA.
    pub bytes: Vec<u8>,
    pub rtb: Vec<u8>,
    pub ranges: Vec<EmittedRange>,
    pub pointer_count: usize,
    pub coverage: Vec<String>,
}
pub(super) struct Layout {
    pub(super) bytes: Vec<u8>,
    pub(super) ranges: Vec<EmittedRange>,
    pub(super) rows: Vec<RelocRow>,
    pub(super) base: u32,
    pub(super) block: BlockKey,
}
impl Layout {
    pub(super) fn linked_list(
        &mut self,
        header: usize,
        entries: &[usize],
        next: usize,
    ) -> Result<()> {
        self.bytes[header + 8..header + 12]
            .copy_from_slice(&u32::try_from(entries.len())?.to_le_bytes());
        if let (Some(&first), Some(&last)) = (entries.first(), entries.last()) {
            self.pointer(header, first)?;
            self.pointer(header + 4, last)?;
        }
        for (i, &p) in entries.iter().enumerate() {
            if let Some(&n) = entries.get(i + 1) {
                self.pointer(p + next, n)?;
            }
            if i > 0 {
                self.pointer(p + next + 4, entries[i - 1])?;
            }
            self.pointer(p + next + 8, header)?;
        }
        Ok(())
    }
    pub(super) fn allocate(&mut self, name: impl Into<String>, bytes: &[u8]) -> Result<usize> {
        let aligned = self
            .bytes
            .len()
            .checked_add(3)
            .ok_or_else(|| anyhow!("layout overflow"))?
            & !3;
        ensure!(
            aligned + bytes.len() <= MAX_BLOCK,
            "geometry layout too large"
        );
        self.bytes.resize(aligned, 0);
        self.bytes.extend_from_slice(bytes);
        self.ranges.push(EmittedRange {
            name: name.into(),
            offset: aligned,
            length: bytes.len(),
        });
        Ok(aligned)
    }
    pub(super) fn pointer(&mut self, site: usize, offset: usize) -> Result<()> {
        let address = self
            .base
            .checked_add(offset.try_into()?)
            .ok_or_else(|| anyhow!("pointer overflow"))?;
        self.external_pointer(
            site,
            ReferenceTarget {
                address,
                block: (self.block.0, self.block.1),
            },
        )
    }
    pub(super) fn external_pointer(&mut self, site: usize, target: ReferenceTarget) -> Result<()> {
        ensure!(target.address != 0, "non-null asset resolved to zero");
        self.bytes
            .get_mut(site..site + 4)
            .ok_or_else(|| anyhow!("invalid emission site"))?
            .copy_from_slice(&target.address.to_le_bytes());
        self.rows.push(RelocRow {
            value: self
                .base
                .checked_add(site.try_into()?)
                .ok_or_else(|| anyhow!("site overflow"))?,
            module: target.block.0,
            id: target.block.1,
            unknown: [0; 2],
        });
        Ok(())
    }
}
pub(super) fn float_data<const N: usize>(values: &[[f32; N]]) -> Vec<u8> {
    values
        .iter()
        .flatten()
        .flat_map(|v| v.to_bits().to_le_bytes())
        .collect()
}
pub(super) fn short_data<const N: usize>(values: &[[u16; N]]) -> Vec<u8> {
    values
        .iter()
        .flatten()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}
/// Reconstruct the verified geometry subset from useful values with caller-supplied material layout.
/// Counts, buffer placement and pointer sites are derived. Spatial acceleration and other domain
/// records are explicitly outside this fragment; it is never presented as whole-package parity.
pub fn write_geometry(
    model: &Model,
    materials: &BTreeMap<AssetPath, ReferenceTarget>,
    base: u32,
) -> Result<GeometryExport> {
    model.validate()?;
    ensure!(
        base != 0 && base.is_multiple_of(4),
        "geometry base must be nonzero/aligned"
    );
    let mut out = Layout {
        bytes: Vec::new(),
        ranges: Vec::new(),
        rows: Vec::new(),
        base,
        block: BlockKey(6, 2),
    };
    emit_geometry(&mut out, model, materials, &BTreeMap::new())?;
    out.rows.sort_by_key(|r| r.value);
    let rtb = emit_relocations(BlockKey(6, 2), &out.rows)?;
    let coverage = vec![
        "Geometry header (0x30), point/normal arrays, triangle groups (0x20), UV mapping, point subsets \
            and material links regenerated in a controlled layout.".into(),
        "Source allocator bookkeeping, spatial acceleration, unknown geometry state, materials \
            themselves, IPO/scene records and full SNA assembly are outside this fragment.".into(),
    ];
    Ok(GeometryExport {
        bytes: out.bytes,
        rtb,
        ranges: out.ranges,
        pointer_count: out.rows.len(),
        coverage,
    })
}

/// Append geometry into its parent's layout so pointers and relocations are emitted once.
pub(super) fn emit_geometry(
    out: &mut Layout,
    model: &Model,
    materials: &BTreeMap<AssetPath, ReferenceTarget>,
    sprites: &BTreeMap<AssetPath, ReferenceTarget>,
) -> Result<usize> {
    let order = model.element_order();
    let header = GeometryWire {
        point_count: model.positions.len().try_into()?,
        point_reserved: 0,
        points: 0,
        normals: 0,
        point_materials: 0,
        unknown_10: 0,
        element_count: order.len().try_into()?,
        element_reserved: 0,
        element_types: 0,
        elements: 0,
        spatial_data: 0,
        unknown_tail: [0; 3],
    };
    let geometry = out.allocate("geometry", &encoded(&header)?)?;
    let p = out.allocate("positions", &float_data(&model.positions))?;
    out.pointer(geometry + GeometryWire::POINTS, p)?;
    if !model.normals.is_empty() {
        let p = out.allocate("normals", &float_data(&model.normals))?;
        out.pointer(geometry + GeometryWire::NORMALS, p)?;
    }
    if !order.is_empty() {
        let types = order
            .iter()
            .map(|&(sprite, _)| [if sprite { 3u16 } else { 1 }])
            .collect::<Vec<_>>();
        let p = out.allocate("element_types", &short_data(&types))?;
        out.pointer(geometry + GeometryWire::ELEMENT_TYPES, p)?;
        let table = out.allocate("elements", &vec![0; order.len() * 4])?;
        out.pointer(geometry + GeometryWire::ELEMENTS, table)?;
        for (element, &(sprite, i)) in order.iter().enumerate() {
            if sprite {
                let h = super::sprite::emit_element(out, &model.sprite_elements[i], i, sprites)?;
                out.pointer(table + element * 4, h)?;
                continue;
            }
            let sm = &model.submeshes[i];
            let wire = TrianglesWire {
                material: 0,
                triangle_count: sm.triangles.len().try_into()?,
                uv_count: sm.texcoords.len().try_into()?,
                triangles: 0,
                uv_indices: 0,
                normals: 0,
                uvs: 0,
                point_count: sm.vertex_indices.len().try_into()?,
                point_reserved: 0,
                points: 0,
            };
            let h = out.allocate(format!("submeshes/{i}"), &encoded(&wire)?)?;
            out.pointer(table + element * 4, h)?;
            let target = *materials
                .get(&sm.material.path)
                .ok_or_else(|| anyhow!("material has no export layout: {}", sm.material.path))?;
            out.external_pointer(h + TrianglesWire::MATERIAL, target)?;
            for (name, field, data) in [
                (
                    "triangles",
                    TrianglesWire::TRIANGLES,
                    short_data(&sm.triangles),
                ),
                (
                    "uv_indices",
                    TrianglesWire::UV_INDICES,
                    short_data(&sm.uv_indices),
                ),
                (
                    "face_normals",
                    TrianglesWire::NORMALS,
                    float_data(&sm.face_normals),
                ),
                ("texcoords", TrianglesWire::UVS, float_data(&sm.texcoords)),
                (
                    "vertex_indices",
                    TrianglesWire::POINTS,
                    sm.vertex_indices
                        .iter()
                        .flat_map(|v| v.to_le_bytes())
                        .collect(),
                ),
            ] {
                if !data.is_empty() {
                    let p = out.allocate(format!("submeshes/{i}/{name}"), &data)?;
                    out.pointer(h + field, p)?;
                }
            }
        }
    }
    if !model.vertex_materials.is_empty() {
        let p = out.allocate(
            "vertex_materials",
            &vec![0; model.vertex_materials.len() * 4],
        )?;
        out.pointer(geometry + GeometryWire::POINT_MATERIALS, p)?;
        for (i, m) in model.vertex_materials.iter().enumerate() {
            let Some(m) = m else {
                continue;
            };
            out.external_pointer(
                p + i * 4,
                *materials
                    .get(&m.path)
                    .ok_or_else(|| anyhow!("point material has no export layout: {}", m.path))?,
            )?;
        }
    }
    Ok(geometry)
}
