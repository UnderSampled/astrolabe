//! Hype sector links and portal topology, corroborated by SECT/SCT engine getters.
use super::{container::wire, records::*, session::SourceSession};
use crate::model::scene::*;
use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

fn node(
    s: &SourceSession,
    a: Address,
    offset: usize,
    nodes: &BTreeMap<Address, String>,
) -> Result<String> {
    let target = s.required(a, offset)?;
    nodes
        .get(&target)
        .cloned()
        .context("sector link targets a node outside the active scene")
}

pub(super) fn entries(
    s: &SourceSession,
    a: Address,
    offset: usize,
    next_offset: usize,
) -> Result<Vec<Address>> {
    let header = Address {
        offset: a.offset + offset,
        ..a
    };
    let h: ListWire = wire(s.bytes(header, 12)?)?;
    ensure!(h.count <= 65536, "excessive sector list count");
    let mut next = s.pointer(header, 0)?;
    let mut previous = None;
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for _ in 0..h.count {
        let entry = next.context("short sector list")?;
        ensure!(seen.insert(entry), "cyclic sector list");
        ensure!(
            s.pointer(entry, next_offset + 4)? == previous,
            "sector list previous link mismatch"
        );
        ensure!(
            s.pointer(entry, next_offset + 8)? == Some(header),
            "sector list owner mismatch"
        );
        out.push(entry);
        previous = Some(entry);
        next = s.pointer(entry, next_offset)?;
    }
    ensure!(
        next.is_none() && s.pointer(header, 4)? == previous,
        "sector list tail mismatch"
    );
    Ok(out)
}
pub(super) fn node_list(
    s: &SourceSession,
    a: Address,
    offset: usize,
    nodes: &BTreeMap<Address, String>,
) -> Result<Vec<String>> {
    entries(s, a, offset, 4)?
        .into_iter()
        .map(|a| node(s, a, 0, nodes))
        .collect()
}
pub(super) fn graphics(
    s: &SourceSession,
    a: Address,
    offset: usize,
    nodes: &BTreeMap<Address, String>,
) -> Result<Vec<SectorVisibility>> {
    entries(s, a, offset, 8)?
        .into_iter()
        .map(|a| {
            let h: GraphicLinkWire = wire(s.bytes(a, 20)?)?;
            Ok(SectorVisibility {
                sector: node(s, a, 4, nodes)?,
                level: h.level,
                unknown: h.unknown,
            })
        })
        .collect()
}
pub(super) fn sound(
    s: &SourceSession,
    a: Address,
    offset: usize,
    nodes: &BTreeMap<Address, String>,
) -> Result<Vec<SectorSoundPropagation>> {
    entries(s, a, offset, 8)?
        .into_iter()
        .map(|a| {
            let h: SoundLinkWire = wire(s.bytes(a, 20)?)?;
            Ok(SectorSoundPropagation {
                sector: node(s, a, 0, nodes)?,
                volume: h.volume,
            })
        })
        .collect()
}
pub(super) fn boundary(
    s: &SourceSession,
    a: Address,
    nodes: &BTreeMap<Address, String>,
) -> Result<SectorBoundary> {
    let h: GeometryWire = wire(s.bytes(a, 48)?)?;
    ensure!(
        h.point_reserved == 0
            && h.element_reserved == 0
            && h.unknown_10 == 0
            && h.unknown_tail == [0; 3],
        "unsupported sector GEO fields"
    );
    ensure!(
        h.point_materials == 0 && h.spatial_data == 0,
        "sector GEO auxiliary data requires codec"
    );
    let mut out = SectorBoundary {
        positions: s.floats(s.required(a, 4)?, h.point_count as usize)?,
        normals: s
            .pointer(a, 8)?
            .map(|a| s.floats(a, h.point_count as usize))
            .transpose()?
            .unwrap_or_default(),
        surfaces: Vec::new(),
    };
    let kinds = s.short_arrays::<1>(s.required(a, 24)?, h.element_count as usize)?;
    let table = s.required(a, 28)?;
    for (i, [kind]) in kinds.into_iter().enumerate() {
        ensure!(kind == 1, "unsupported sector boundary element {kind}");
        let a = s.required(table, i * 4)?;
        let h: TrianglesWire = wire(s.bytes(a, 32)?)?;
        ensure!(
            h.point_reserved == 0,
            "sector boundary UV/unknown fields require codec: {h:?}"
        );
        out.surfaces.push(SectorSurface {
            texcoords: if h.uv_count == 0 {
                Vec::new()
            } else {
                s.floats(s.required(a, 20)?, h.uv_count as usize)?
            },
            uv_indices: s
                .pointer(a, 12)?
                .map(|a| s.short_arrays(a, h.triangle_count as usize))
                .transpose()?
                .unwrap_or_default(),
            sector: s
                .pointer(a, 0)?
                .map(|a| {
                    nodes
                        .get(&a)
                        .cloned()
                        .context("sector face target outside scene")
                })
                .transpose()?,
            triangles: s.short_arrays(s.required(a, 8)?, h.triangle_count as usize)?,
            normals: s
                .pointer(a, 16)?
                .map(|a| s.floats(a, h.triangle_count as usize))
                .transpose()?
                .unwrap_or_default(),
            vertex_indices: if h.point_count == 0 {
                Vec::new()
            } else {
                s.short_arrays::<1>(s.required(a, 28)?, h.point_count as usize)?
                    .into_iter()
                    .map(|[v]| v)
                    .collect()
            },
        });
    }
    out.validate()?;
    Ok(out)
}

#[cfg(test)]
pub(super) fn check_reconstruction(scene: &Scene) -> Result<()> {
    use super::{
        ReferenceTarget,
        container::{merge_relocations, reloc, sna, write_sna_block},
    };
    let nodes: BTreeMap<_, _> = scene
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            (
                n.id.clone(),
                ReferenceTarget {
                    address: 0x1000 + i as u32 * 4,
                    block: (6, 1),
                },
            )
        })
        .collect();
    let mut light_paths = BTreeSet::new();
    for sector in scene.nodes.iter().filter_map(|n| n.sector.as_ref()) {
        light_paths.extend(
            sector
                .static_lights
                .iter()
                .chain(&sector.dynamic_lights)
                .map(|r| r.path.clone()),
        );
    }
    let lights = light_paths
        .iter()
        .enumerate()
        .map(|(i, path)| {
            (
                path.clone(),
                ReferenceTarget {
                    address: 0x1000 + (scene.nodes.len() + i) as u32 * 4,
                    block: (6, 1),
                },
            )
        })
        .collect();
    let environment_paths: BTreeSet<_> = scene
        .nodes
        .iter()
        .flat_map(|n| &n.sector)
        .flat_map(|s| &s.environments)
        .map(|r| r.path.clone())
        .collect();
    let environments: BTreeMap<_, _> = environment_paths
        .iter()
        .enumerate()
        .map(|(i, p)| {
            (
                p.clone(),
                ReferenceTarget {
                    address: 0x1000 + (scene.nodes.len() + light_paths.len() + i) as u32 * 4,
                    block: (6, 1),
                },
            )
        })
        .collect();
    let fragment = super::write_sectors(scene, &nodes, &lights, &environments, 0x10000)?;
    let mut bytes = write_sna_block(&fragment.bytes, (6, 2), 0x10000)?;
    bytes.extend(write_sna_block(
        &vec![0; (scene.nodes.len() + light_paths.len() + environment_paths.len()) * 4],
        (6, 1),
        0x1000,
    )?);
    let blocks = sna(&bytes)?;
    let mut pointers = BTreeMap::new();
    merge_relocations(&mut pointers, Owner::Level, reloc(&fragment.rtb)?)?;
    let session = SourceSession {
        fix_bindings: Default::default(),
        allocation_cache: Default::default(),
        actual: blocks.keys().map(|&k| (k, Owner::Level)).collect(),
        blocks: BTreeMap::from([(Owner::Level, blocks)]),
        pointers,
        gpt: Vec::new(),
        rtp: BTreeMap::new(),
        ptx: Vec::new(),
        rtt: BTreeMap::new(),
    };
    let nodes = nodes
        .iter()
        .map(|(id, target)| {
            (
                Address {
                    owner: Owner::Level,
                    key: BlockKey(6, 1),
                    offset: (target.address - 0x1000) as usize,
                },
                id.clone(),
            )
        })
        .collect();
    for n in &scene.nodes {
        let Some(expected) = &n.sector else {
            continue;
        };
        let range = fragment
            .ranges
            .iter()
            .find(|r| r.name == format!("sectors/{}", n.id))
            .unwrap();
        let a = Address {
            owner: Owner::Level,
            key: BlockKey(6, 2),
            offset: range.offset,
        };
        let h: SectorWire = wire(session.bytes(a, 160)?)?;
        assert_eq!(h.virtual_sector, expected.virtual_sector);
        assert_eq!(h.activation, expected.activation);
        assert_eq!(h.unknown, expected.unknown);
        assert_eq!(
            session
                .pointer(a, 0)?
                .map(|b| boundary(&session, b, &nodes))
                .transpose()?,
            expected.boundary
        );
        assert_eq!(node_list(&session, a, 28, &nodes)?, expected.persos);
        assert_eq!(node_list(&session, a, 100, &nodes)?, expected.collision);
        assert_eq!(node_list(&session, a, 112, &nodes)?, expected.activity);
        assert_eq!(graphics(&session, a, 76, &nodes)?, expected.graphics);
        assert_eq!(
            graphics(&session, a, 88, &nodes)?,
            expected.optional_graphics
        );
        assert_eq!(sound(&session, a, 124, &nodes)?, expected.sound);
        let links = entries(&session, a, 4, 4)?;
        assert_eq!(links.len(), expected.environments.len());
        for (a, r) in links.iter().zip(&expected.environments) {
            assert_eq!(
                session.required(*a, 0)?.offset,
                (environments[&r.path].address - 0x1000) as usize
            );
        }
        for (field, refs) in [
            (40, &expected.static_lights),
            (52, &expected.dynamic_lights),
        ] {
            let entries = entries(&session, a, field, 4)?;
            assert_eq!(entries.len(), refs.len());
            for (a, r) in entries.iter().zip(refs) {
                let target = session.required(*a, 0)?;
                assert_eq!(target.offset, (lights[&r.path].address - 0x1000) as usize);
            }
        }
    }
    Ok(())
}
