//! Controlled collision reconstruction from useful topology and material semantics.
use super::{
    container::{emit_relocations, encoded},
    geometry::{GeometryExport, Layout, float_data, short_data},
    records::*,
};
use crate::model::{
    AssetPath, AssetRef,
    collision::*,
    io::{Content, Load},
};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

pub fn write_collision(
    collision: &Collision,
    source: &mut (impl Load<crate::model::geometry::Material> + Load<crate::model::geometry::Model>),
    base: u32,
) -> Result<GeometryExport> {
    collision.validate()?;
    let collision = collision.resolve(source)?;
    ensure!(
        base != 0 && base.is_multiple_of(4),
        "collision base must be nonzero/aligned"
    );
    let mut out = Layout {
        bytes: Vec::new(),
        rows: Vec::new(),
        ranges: Vec::new(),
        base,
        block: BlockKey(6, 2),
    };
    let geometry = out.allocate(
        "collision",
        &encoded(&GeometryWire {
            point_count: collision.positions.len().try_into()?,
            point_reserved: 0,
            points: 0,
            normals: 0,
            point_materials: 0,
            unknown_10: 0,
            element_count: collision.elements.len().try_into()?,
            element_reserved: 0,
            element_types: 0,
            elements: 0,
            spatial_data: 0,
            unknown_tail: [0; 3],
        })?,
    )?;
    let p = out.allocate("positions", &float_data(&collision.positions))?;
    out.pointer(geometry + 4, p)?;
    if !collision.normals.is_empty() {
        let p = out.allocate("normals", &float_data(&collision.normals))?;
        out.pointer(geometry + 8, p)?;
    }
    let kinds: Vec<_> = collision
        .elements
        .iter()
        .map(|e| {
            [match e {
                CollisionElement::Triangles { .. } => 1,
                CollisionElement::Spheres(_) => 7,
                CollisionElement::Boxes(_) => 8,
            }]
        })
        .collect();
    let p = out.allocate("element_types", &short_data(&kinds))?;
    out.pointer(geometry + 24, p)?;
    let table = out.allocate("elements", &vec![0; collision.elements.len() * 4])?;
    out.pointer(geometry + 28, table)?;
    let mut materials = BTreeMap::new();
    for (i, element) in collision.elements.iter().enumerate() {
        let name = format!("elements/{i}");
        let address = match element {
            CollisionElement::Triangles {
                material,
                indices,
                normals,
                texcoords,
                uv_indices,
                vertex_indices,
            } => {
                let address = out.allocate(
                    &name,
                    &encoded(&TrianglesWire {
                        material: 0,
                        triangle_count: indices.len().try_into()?,
                        uv_count: texcoords.len().try_into()?,
                        triangles: 0,
                        uv_indices: 0,
                        normals: 0,
                        uvs: 0,
                        point_count: vertex_indices.len().try_into()?,
                        point_reserved: 0,
                        points: 0,
                    })?,
                )?;
                material_pointer(&mut out, source, &mut materials, address, material)?;
                for (field, label, bytes) in [
                    (8, "indices", short_data(indices)),
                    (12, "uv_indices", short_data(uv_indices)),
                    (16, "normals", float_data(normals)),
                    (20, "texcoords", float_data(texcoords)),
                    (
                        28,
                        "vertex_indices",
                        short_data(&vertex_indices.iter().map(|&v| [v]).collect::<Vec<_>>()),
                    ),
                ] {
                    if !bytes.is_empty() {
                        let p = out.allocate(format!("{name}/{label}"), &bytes)?;
                        out.pointer(address + field, p)?;
                    }
                }
                address
            }
            CollisionElement::Spheres(spheres) => {
                let address = out.allocate(
                    &name,
                    &encoded(&CollisionArrayWire {
                        count: spheres.len().try_into()?,
                        reserved: 0,
                        data: 0,
                    })?,
                )?;
                if !spheres.is_empty() {
                    let mut data = Vec::new();
                    for s in spheres {
                        data.extend(encoded(&CollisionSphereWire {
                            center: s.center,
                            reserved: 0,
                            radius: s.radius,
                            material: 0,
                        })?);
                    }
                    let p = out.allocate(format!("{name}/spheres"), &data)?;
                    out.pointer(address + 4, p)?;
                    for (j, s) in spheres.iter().enumerate() {
                        material_pointer(
                            &mut out,
                            source,
                            &mut materials,
                            p + j * 12 + 8,
                            &s.material,
                        )?;
                    }
                }
                address
            }
            CollisionElement::Boxes(boxes) => {
                let address = out.allocate(
                    &name,
                    &encoded(&CollisionArrayWire {
                        count: boxes.len().try_into()?,
                        reserved: 0,
                        data: 0,
                    })?,
                )?;
                if !boxes.is_empty() {
                    let mut data = Vec::new();
                    for b in boxes {
                        data.extend(encoded(&CollisionBoxWire {
                            min: b.min,
                            max: b.max,
                            material: 0,
                        })?);
                    }
                    let p = out.allocate(format!("{name}/boxes"), &data)?;
                    out.pointer(address + 4, p)?;
                    for (j, b) in boxes.iter().enumerate() {
                        material_pointer(
                            &mut out,
                            source,
                            &mut materials,
                            p + j * 8 + 4,
                            &b.material,
                        )?;
                    }
                }
                address
            }
        };
        out.pointer(table + i * 4, address)?;
    }
    if !collision.vertex_materials.is_empty() {
        let p = out.allocate(
            "vertex_materials",
            &vec![0; collision.vertex_materials.len() * 4],
        )?;
        out.pointer(geometry + 12, p)?;
        for (i, material) in collision.vertex_materials.iter().enumerate() {
            material_pointer(&mut out, source, &mut materials, p + i * 4, material)?;
        }
    }
    out.rows.sort_by_key(|r| r.value);
    Ok(GeometryExport { rtb: emit_relocations(BlockKey(6, 2), &out.rows)?, pointer_count: out.rows.len(), bytes: out.bytes, ranges: out.ranges, coverage: vec![
        "Collision GEO topology, triangle UV/normal/subset data, spheres, boxes, collision properties and sound-material IDs reconstructed with generated pointers.".into(),
        "Visual/mechanical components of collision game materials, source spatial acceleration, allocator state and unknown GEO state are outside the collision fragment.".into(),
    ] })
}

fn material_pointer(
    out: &mut Layout,
    source: &mut impl Load<crate::model::geometry::Material>,
    materials: &mut BTreeMap<AssetPath, usize>,
    site: usize,
    reference: &Option<AssetRef<crate::model::geometry::Material>>,
) -> Result<()> {
    let Some(reference) = reference else {
        return Ok(());
    };
    let address = if let Some(&address) = materials.get(&reference.path) {
        address
    } else {
        let material = source
            .load(reference)
            .with_context(|| format!("collision material {}", reference.path))?;
        material.validate_reference(&reference.path)?;
        let name = format!("materials/{}", materials.len());
        let address = out.allocate(
            &name,
            &encoded(&GameMaterialWire {
                visual: 0,
                mechanics: 0,
                sound: material.sound_material,
                collision: if material.collision == CollisionProperties::Default {
                    u32::MAX
                } else {
                    0
                },
            })?,
        )?;
        if let CollisionProperties::Surface { kind, identifier } = material.collision {
            let p = out.allocate(
                format!("{name}/collision"),
                &encoded(&CollisionMaterialWire { kind, identifier })?,
            )?;
            out.pointer(address + 12, p)?;
        }
        materials.insert(reference.path.clone(), address);
        address
    };
    out.pointer(site, address)
}
