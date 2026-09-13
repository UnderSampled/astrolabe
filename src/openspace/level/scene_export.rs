//! Full 64-byte Hype SO records and derived hierarchy/matrix placement.
//! Data components are supplied by their owning codecs, never replayed source pointers.
use super::{
    GeometryExport, ReferenceTarget,
    container::{emit_relocations, encoded},
    geometry::Layout,
    records::*,
    write_transform,
};
use crate::model::{AssetPath, animation::TransformValue, io::Content, scene::*};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct SceneDependencies {
    /// Node ID -> sector data, IPO or Perso record, according to the node kind.
    pub data: BTreeMap<String, ReferenceTarget>,
    pub lights: BTreeMap<AssetPath, ReferenceTarget>,
    pub bounds: BTreeMap<AssetPath, ReferenceTarget>,
    /// Optional embedded/externally shared local matrices, e.g. Perso3dData+24.
    /// The caller writes the corresponding useful node transform there.
    pub local_transforms: BTreeMap<String, ReferenceTarget>,
}
pub struct SceneGraphExport {
    pub fragment: GeometryExport,
    pub nodes: BTreeMap<String, ReferenceTarget>,
    pub local_transforms: BTreeMap<String, ReferenceTarget>,
    pub global_transforms: BTreeMap<String, ReferenceTarget>,
}
pub fn write_scene_graph(
    scene: &Scene,
    dependencies: &SceneDependencies,
    base: u32,
) -> Result<SceneGraphExport> {
    scene.validate()?;
    ensure!(
        base != 0 && base.is_multiple_of(4),
        "scene base must be aligned/nonzero"
    );
    let mut out = Layout {
        bytes: Vec::new(),
        rows: Vec::new(),
        ranges: Vec::new(),
        base,
        block: BlockKey(6, 2),
    };
    let globals = scene.global_transforms()?;
    let mut offsets = BTreeMap::new();
    let mut nodes = BTreeMap::new();
    let target = |offset: usize| -> Result<ReferenceTarget> {
        Ok(ReferenceTarget {
            address: base
                .checked_add(offset.try_into()?)
                .context("scene layout overflow")?,
            block: (6, 2),
        })
    };
    let mut children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for node in &scene.nodes {
        if let Some(p) = &node.parent {
            children.entry(p).or_default().push(&node.id);
        }
        let kind = node.properties.source_kind.unwrap_or(match node.kind {
            NodeKind::World => 0,
            NodeKind::Sector => 8,
            NodeKind::Perso => 4,
            NodeKind::Object => 0xd,
            NodeKind::Unknown(n) => n,
        });
        let p = out.allocate(
            format!("nodes/{}", node.id),
            &encoded(&SuperObjectWire {
                kind,
                data: 0,
                first_child: 0,
                last_child: 0,
                child_count: 0,
                next: 0,
                previous: 0,
                parent: 0,
                local_transform: 0,
                global_transform: 0,
                last_compute_frame: if node.properties.transform_space == TransformSpace::World {
                    u32::MAX
                } else {
                    u32::MAX - 1
                },
                draw_mask: node.draw_mask,
                light: 0,
                bounding_volume: 0,
                initial_global_transform: 0,
                opacity: node.properties.opacity,
            })?,
        )?;
        offsets.insert(node.id.as_str(), p);
        nodes.insert(node.id.clone(), target(p)?);
    }
    let mut local_transforms = BTreeMap::new();
    let mut global_transforms = BTreeMap::new();
    for node in &scene.nodes {
        let p = offsets[node.id.as_str()];
        if node.kind != NodeKind::World {
            out.external_pointer(
                p + 4,
                *dependencies
                    .data
                    .get(&node.id)
                    .with_context(|| format!("missing scene data layout for {}", node.id))?,
            )?;
        } else {
            ensure!(
                !dependencies.data.contains_key(&node.id),
                "world node cannot have a data component"
            );
        }
        if let Some(parent) = &node.parent {
            out.pointer(p + 28, offsets[parent.as_str()])?;
        }
        if let Some(list) = children.get(node.id.as_str()) {
            out.bytes[p + 16..p + 20].copy_from_slice(&u32::try_from(list.len())?.to_le_bytes());
            out.pointer(p + 8, offsets[list[0]])?;
            out.pointer(p + 12, offsets[list[list.len() - 1]])?;
            for pair in list.windows(2) {
                out.pointer(offsets[pair[0]] + 20, offsets[pair[1]])?;
                out.pointer(offsets[pair[1]] + 24, offsets[pair[0]])?;
            }
        }
        let local = if let Some(value) = dependencies.local_transforms.get(&node.id) {
            *value
        } else {
            let bytes = write_transform(&TransformValue::Affine(
                node.transform.clone().unwrap_or_default(),
            ))?
            .unwrap();
            let offset = out.allocate(format!("nodes/{}/local", node.id), &bytes)?;
            target(offset)?
        };
        out.external_pointer(p + 32, local)?;
        local_transforms.insert(node.id.clone(), local);
        let bytes = write_transform(&TransformValue::Affine(globals[&node.id].clone()))?.unwrap();
        let g = out.allocate(format!("nodes/{}/global", node.id), &bytes)?;
        out.pointer(p + 36, g)?;
        global_transforms.insert(node.id.clone(), target(g)?);
        let initial = node
            .properties
            .initial_global_transform
            .as_ref()
            .unwrap_or(&globals[&node.id]);
        let bytes = write_transform(&TransformValue::Affine(initial.clone()))?.unwrap();
        let g = out.allocate(format!("nodes/{}/initial-global", node.id), &bytes)?;
        out.pointer(p + 56, g)?;
        for (field, reference, layout) in [
            (
                48,
                node.light.as_ref().map(|r| &r.path),
                &dependencies.lights,
            ),
            (
                52,
                node.properties.bounding_geometry.as_ref().map(|r| &r.path),
                &dependencies.bounds,
            ),
        ] {
            if let Some(path) = reference {
                out.external_pointer(
                    p + field,
                    *layout
                        .get(path)
                        .with_context(|| format!("missing scene dependency {path}"))?,
                )?;
            }
        }
    }
    for id in dependencies
        .data
        .keys()
        .chain(dependencies.local_transforms.keys())
    {
        ensure!(
            nodes.contains_key(id),
            "layout supplied for unknown scene node {id}"
        );
    }
    out.rows.sort_by_key(|r| r.value);
    Ok(SceneGraphExport {nodes,local_transforms,global_transforms,fragment:GeometryExport {
        rtb:emit_relocations(out.block,&out.rows)?,pointer_count:out.rows.len(),bytes:out.bytes,ranges:out.ranges,
        coverage:vec!["Full SO records, ordered hierarchy, opacity, source-kind identity, initial transforms and derived global matrices reconstructed. Runtime matrix cache counters are invalidated; component/embedded-matrix targets use explicit caller layouts. Whole-SNA assembly remains separate.".into()],
    }})
}
