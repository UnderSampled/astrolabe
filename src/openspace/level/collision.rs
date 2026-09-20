//! Hype collision layouts share GEO vertices/element headers, not visual semantics.
use super::{container::wire, records::*, session::SourceSession};
use crate::model::{AssetPath, AssetRef, collision::*, geometry::Material};
use anyhow::{Result, ensure};
use std::collections::BTreeMap;

pub(super) struct CollisionImporter<'a> {
    pub(super) session: &'a SourceSession,
    pub(super) geometries: Vec<Collision>,
    pub(super) materials: &'a mut Vec<Material>,
    pub(super) diagnostics: &'a mut Vec<String>,
    pub(super) visual_geometries: &'a BTreeMap<Address, AssetPath>,
    pub(super) geometry_ids: BTreeMap<Address, AssetPath>,
    pub(super) material_ids: &'a mut BTreeMap<Address, AssetPath>,
}
impl CollisionImporter<'_> {
    fn material(
        &mut self,
        site: Address,
        field: usize,
        path: &AssetPath,
    ) -> Result<Option<AssetRef<Material>>> {
        let Some(address) = self.session.pointer(site, field)? else {
            return Ok(None);
        };
        super::import::load_material(
            self.session,
            address,
            path.as_str(),
            self.materials,
            self.material_ids,
            self.diagnostics,
        )
        .map(Some)
    }
    pub(super) fn physical(
        &mut self,
        po: Address,
        node: &str,
    ) -> Result<Option<AssetRef<Collision>>> {
        let Some(set) = self
            .session
            .pointer(po, PhysicalObjectWire::COLLISION_SET)?
        else {
            return Ok(None);
        };
        // ZDM/ZDD/ZDE/ZDR are separate collision purposes. Static environment uses ZDR.
        let Some(geometry) = self.session.pointer(set, 12)? else {
            return Ok(None);
        };
        self.geometry(geometry, node).map(Some)
    }
    pub(super) fn geometry(&mut self, address: Address, node: &str) -> Result<AssetRef<Collision>> {
        if let Some(id) = self.geometry_ids.get(&address) {
            return Ok(AssetRef::from_path(id.clone()));
        }
        let role = match address.owner {
            Owner::Fix => "fix",
            Owner::Level => "level",
        };
        let path = AssetPath::new(format!("{role}:/collision/{node}"))?;
        if let Some(model) = self.visual_geometries.get(&address) {
            self.geometry_ids.insert(address, path.clone());
            self.geometries.push(Collision {
                path: path.clone(),
                geometry: CollisionGeometry::Model(AssetRef::from_path(model.clone())),
            });
            return Ok(AssetRef::from_path(path));
        }
        let header: GeometryWire = wire(self.session.bytes(address, 48)?)?;
        ensure!(
            header.point_count > 0 && header.point_count <= i16::MAX as u16,
            "invalid collision point count"
        );
        let mut collision = CollisionMesh {
            positions: self.session.floats(
                self.session.required(address, GeometryWire::POINTS)?,
                header.point_count as usize,
            )?,
            normals: Vec::new(),
            elements: Vec::new(),
            vertex_materials: Vec::new(),
        };
        if let Some(a) = self.session.pointer(address, GeometryWire::NORMALS)? {
            collision.normals = self.session.floats(a, header.point_count as usize)?;
        }
        let kinds = self.session.short_arrays::<1>(
            self.session
                .required(address, GeometryWire::ELEMENT_TYPES)?,
            header.element_count as usize,
        )?;
        let table = self.session.required(address, GeometryWire::ELEMENTS)?;
        for (i, [kind]) in kinds.into_iter().enumerate() {
            let element = self.session.required(table, i * 4)?;
            collision.elements.push(match kind {
                5 => {
                    let header: CollisionPointsWire = wire(self.session.bytes(element, 16)?)?;
                    let indices = if header.count == 0 {
                        Vec::new()
                    } else {
                        self.session
                            .short_arrays::<1>(
                                self.session.required(element, 8)?,
                                header.count as usize,
                            )?
                            .into_iter()
                            .map(|v| v[0])
                            .collect()
                    };
                    CollisionElement::Points {
                        fatness: header.fatness,
                        indices,
                        material: self.material(element, 12, &path)?,
                        unknown: header.unknown,
                    }
                }
                1 => {
                    let h: TrianglesWire = wire(self.session.bytes(element, 32)?)?;
                    let texcoords = if h.uv_count == 0 {
                        Vec::new()
                    } else {
                        self.session.floats(
                            self.session.required(element, TrianglesWire::UVS)?,
                            h.uv_count as usize,
                        )?
                    };
                    let uv_indices = self
                        .session
                        .pointer(element, TrianglesWire::UV_INDICES)?
                        .map(|a| self.session.short_arrays(a, h.triangle_count as usize))
                        .transpose()?
                        .unwrap_or_default();
                    let vertex_indices = if h.point_count == 0 {
                        Vec::new()
                    } else {
                        self.session
                            .short_arrays::<1>(
                                self.session.required(element, TrianglesWire::POINTS)?,
                                h.point_count as usize,
                            )?
                            .into_iter()
                            .map(|v| v[0])
                            .collect()
                    };
                    let indices = self.session.short_arrays(
                        self.session.required(element, TrianglesWire::TRIANGLES)?,
                        h.triangle_count as usize,
                    )?;
                    let normals = self
                        .session
                        .pointer(element, TrianglesWire::NORMALS)?
                        .map(|a| self.session.floats(a, h.triangle_count as usize))
                        .transpose()?
                        .unwrap_or_default();
                    CollisionElement::Triangles {
                        material: self.material(element, 0, &path)?,
                        indices,
                        normals,
                        texcoords,
                        uv_indices,
                        vertex_indices,
                    }
                }
                7 | 8 => {
                    let h: CollisionArrayWire = wire(self.session.bytes(element, 8)?)?;
                    ensure!(h.count >= 0, "negative collision element count");
                    let data = if h.count == 0 {
                        element
                    } else {
                        self.session.required(element, 4)?
                    };
                    if kind == 7 {
                        let mut spheres = Vec::new();
                        for j in 0..h.count as usize {
                            let a = Address {
                                offset: data.offset + j * 12,
                                ..data
                            };
                            let s: CollisionSphereWire = wire(self.session.bytes(a, 12)?)?;
                            spheres.push(CollisionSphere {
                                center: s.center,
                                radius: s.radius,
                                material: self.material(a, 8, &path)?,
                            });
                        }
                        CollisionElement::Spheres(spheres)
                    } else {
                        let mut boxes = Vec::new();
                        for j in 0..h.count as usize {
                            let a = Address {
                                offset: data.offset + j * 8,
                                ..data
                            };
                            let b: CollisionBoxWire = wire(self.session.bytes(a, 8)?)?;
                            boxes.push(CollisionBox {
                                min: b.min,
                                max: b.max,
                                material: self.material(a, 4, &path)?,
                            });
                        }
                        CollisionElement::Boxes(boxes)
                    }
                }
                _ => anyhow::bail!("unsupported collision geometric element {kind} in {node}"),
            });
        }
        if let Some(table) = self
            .session
            .pointer(address, GeometryWire::POINT_MATERIALS)?
        {
            for i in 0..header.point_count as usize {
                collision
                    .vertex_materials
                    .push(self.material(table, i * 4, &path)?);
            }
        }
        collision.validate()?;
        self.geometry_ids.insert(address, path.clone());
        self.geometries.push(Collision {
            path: path.clone(),
            geometry: CollisionGeometry::Mesh(collision),
        });
        Ok(AssetRef::from_path(path))
    }
}
