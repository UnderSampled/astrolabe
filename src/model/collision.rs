//! Collision geometry can share a model or own a mesh; placement belongs to the scene.
use super::{
    AssetPath, AssetRef,
    geometry::{Material, Model},
    io::{Content, Load},
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collision {
    pub path: AssetPath,
    pub geometry: CollisionGeometry,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CollisionGeometry {
    Model(AssetRef<Model>),
    Mesh(CollisionMesh),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollisionMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub elements: Vec<CollisionElement>,
    pub vertex_materials: Vec<Option<AssetRef<Material>>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CollisionElement {
    Points {
        fatness: f32,
        indices: Vec<u16>,
        material: Option<AssetRef<Material>>,
        unknown: [u8; 2],
    },
    Triangles {
        material: Option<AssetRef<Material>>,
        indices: Vec<[u16; 3]>,
        normals: Vec<[f32; 3]>,
        texcoords: Vec<[f32; 2]>,
        uv_indices: Vec<[u16; 3]>,
        vertex_indices: Vec<u16>,
    },
    Spheres(Vec<CollisionSphere>),
    Boxes(Vec<CollisionBox>),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollisionSphere {
    pub center: u16,
    pub radius: f32,
    pub material: Option<AssetRef<Material>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollisionBox {
    pub min: u16,
    pub max: u16,
    pub material: Option<AssetRef<Material>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CollisionProperties {
    Absent,
    Default,
    Surface { kind: u16, identifier: u16 },
}
impl CollisionProperties {
    pub fn identifier(&self) -> u16 {
        match *self {
            CollisionProperties::Surface { identifier, .. } => identifier,
            _ => 0,
        }
    }
}
impl Content for Collision {
    const KIND: &'static str = "collision";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        match &self.geometry {
            CollisionGeometry::Model(_) => Ok(()),
            CollisionGeometry::Mesh(mesh) => mesh.validate(),
        }
    }
}
impl Collision {
    pub fn resolve(&self, source: &mut impl Load<Model>) -> Result<CollisionMesh> {
        let mesh = match &self.geometry {
            CollisionGeometry::Mesh(mesh) => mesh.clone(),
            CollisionGeometry::Model(reference) => {
                let model = source.load(reference)?;
                model.validate_reference(&reference.path)?;
                model.validate()?;
                CollisionMesh {
                    positions: model.positions.clone(),
                    normals: model.normals.clone(),
                    vertex_materials: model.vertex_materials.clone(),
                    elements: model
                        .submeshes
                        .iter()
                        .map(|part| CollisionElement::Triangles {
                            material: Some(part.material.clone()),
                            indices: part.triangles.clone(),
                            normals: part.face_normals.clone(),
                            texcoords: part.texcoords.clone(),
                            uv_indices: part.uv_indices.clone(),
                            vertex_indices: part.vertex_indices.clone(),
                        })
                        .collect(),
                }
            }
        };
        mesh.validate()?;
        Ok(mesh)
    }
}
impl CollisionMesh {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.elements.len() <= i16::MAX as usize,
            "too many collision elements"
        );
        ensure!(
            self.vertex_materials.is_empty() || self.vertex_materials.len() == self.positions.len(),
            "collision vertex material count mismatch"
        );
        ensure!(
            !self.positions.is_empty() && self.positions.len() <= i16::MAX as usize,
            "invalid collision vertex count"
        );
        ensure!(
            self.normals.is_empty() || self.normals.len() == self.positions.len(),
            "collision normal count mismatch"
        );
        ensure!(
            self.positions
                .iter()
                .chain(&self.normals)
                .flatten()
                .all(|n| n.is_finite()),
            "nonfinite collision geometry"
        );
        let valid = |i: u16| usize::from(i) < self.positions.len();
        for element in &self.elements {
            match element {
                CollisionElement::Points {
                    fatness, indices, ..
                } => {
                    ensure!(
                        fatness.is_finite()
                            && indices.len() <= i16::MAX as usize
                            && indices.iter().all(|&i| valid(i)),
                        "invalid collision points"
                    );
                }
                CollisionElement::Triangles {
                    indices,
                    normals,
                    texcoords,
                    uv_indices,
                    vertex_indices,
                    ..
                } => {
                    ensure!(
                        indices.len() <= i16::MAX as usize
                            && texcoords.len() <= i16::MAX as usize
                            && vertex_indices.len() <= i16::MAX as usize,
                        "collision triangle counts exceed source limits"
                    );
                    ensure!(
                        uv_indices.is_empty() || uv_indices.len() == indices.len(),
                        "collision UV count mismatch"
                    );
                    ensure!(
                        uv_indices
                            .iter()
                            .flatten()
                            .all(|&i| usize::from(i) < texcoords.len()),
                        "collision UV index out of range"
                    );
                    ensure!(
                        texcoords.iter().flatten().all(|n| n.is_finite()),
                        "nonfinite collision UV"
                    );
                    ensure!(
                        vertex_indices.iter().all(|&i| valid(i)),
                        "collision vertex subset out of range"
                    );
                    ensure!(
                        indices.iter().flatten().all(|&i| valid(i)),
                        "collision triangle index out of range"
                    );
                    ensure!(
                        normals.is_empty() || normals.len() == indices.len(),
                        "collision face normal count mismatch"
                    );
                    ensure!(
                        normals.iter().flatten().all(|n| n.is_finite()),
                        "nonfinite collision normal"
                    );
                }
                CollisionElement::Spheres(spheres) => {
                    ensure!(
                        spheres.len() <= i16::MAX as usize,
                        "too many collision spheres"
                    );
                    for sphere in spheres {
                        ensure!(
                            // HIE_fn_vComputeBoundingVolumeOfSuperObjectNode emits
                            // a zero-radius sphere for an empty node.
                            valid(sphere.center)
                                && sphere.radius.is_finite()
                                && sphere.radius >= 0.,
                            "invalid collision sphere: center={}, radius={}",
                            sphere.center,
                            sphere.radius
                        );
                    }
                }
                CollisionElement::Boxes(boxes) => {
                    ensure!(boxes.len() <= i16::MAX as usize, "too many collision boxes");
                    for b in boxes {
                        ensure!(
                            valid(b.min) && valid(b.max),
                            "collision box index out of range"
                        );
                        ensure!(
                            (0..3).all(|i| self.positions[b.min as usize][i]
                                <= self.positions[b.max as usize][i]),
                            "inverted collision box"
                        );
                    }
                }
            }
        }
        Ok(())
    }
}
