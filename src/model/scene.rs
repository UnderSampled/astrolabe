//! Ordered scene ownership, distinct from reusable geometry and instance lighting.
use super::{
    AssetPath, AssetRef,
    geometry::{ModelInstance, SourceTransform},
    io::Content,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    pub path: AssetPath,
    pub name: String,
    /// Explicit actor role, independent of a perso's display name or tree order.
    pub main_actor: Option<AssetRef<super::character::Perso>>,
    /// Shared source actors referenced by AI but not rooted in this level's scene tree.
    /// These are useful assets with source identity, not fabricated scene nodes.
    #[serde(default)]
    pub global_actors: Vec<GlobalActor>,
    /// Parents precede their children. IDs are stable within this asset.
    pub nodes: Vec<SceneNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalActor {
    pub source: super::ai::SourceObjectIdentity,
    pub perso: AssetRef<super::character::Perso>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneNode {
    pub properties: SceneNodeProperties,
    pub id: String,
    pub name: String,
    pub parent: Option<String>,
    pub kind: NodeKind,
    pub transform: Option<SourceTransform>,
    /// The instance holds geometry selection and radiosity; this node owns placement.
    pub instance: Option<AssetRef<ModelInstance>>,
    pub perso: Option<AssetRef<super::character::Perso>>,
    pub collision: Option<AssetRef<super::collision::Collision>>,
    pub draw_mask: u32,
    pub sector: Option<Sector>,
    pub light: Option<AssetRef<super::light::Light>>,
}

/// Node-owned state, independent of its reusable geometry and its data component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneNodeProperties {
    /// Preserve Hype's two IPO kind identities without splitting useful objects.
    pub source_kind: Option<u32>,
    pub transform_space: TransformSpace,
    pub initial_global_transform: Option<SourceTransform>,
    /// Bounds used for source visibility/picking; these are not collision bodies.
    pub bounding_geometry: Option<AssetRef<super::collision::Collision>>,
    /// Hype's source opacity scale: 255 is fully opaque, zero is transparent.
    pub opacity: f32,
}

impl Scene {
    /// Resolve placement using useful values only, including source static/world
    /// transforms. Runtime cache counters and source virtual memory are irrelevant.
    pub fn global_transforms(&self) -> Result<std::collections::BTreeMap<String, SourceTransform>> {
        self.validate()?;
        fn multiply(a: &[[f32; 3]; 3], b: &[[f32; 3]; 3]) -> [[f32; 3]; 3] {
            std::array::from_fn(|c| {
                std::array::from_fn(|r| (0..3).map(|k| a[k][r] * b[c][k]).sum())
            })
        }
        let mut values: std::collections::BTreeMap<String, SourceTransform> = Default::default();
        for node in &self.nodes {
            let mut t = node.transform.clone().unwrap_or_default();
            if node.properties.transform_space == TransformSpace::Local
                && let Some(parent) = &node.parent
            {
                let p = &values[parent];
                t = SourceTransform {
                    translation: std::array::from_fn(|r| {
                        p.translation[r]
                            + (0..3)
                                .map(|k| p.basis[k][r] * t.translation[k])
                                .sum::<f32>()
                    }),
                    rotation: multiply(&p.rotation, &t.rotation),
                    basis: multiply(&p.basis, &t.basis),
                };
            }
            ensure!(
                t.translation
                    .iter()
                    .chain(t.rotation.iter().flatten())
                    .chain(t.basis.iter().flatten())
                    .all(|v| v.is_finite()),
                "scene global transform overflow"
            );
            values.insert(node.id.clone(), t);
        }
        Ok(values)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransformSpace {
    Local,
    World,
}
impl Default for SceneNodeProperties {
    fn default() -> Self {
        Self {
            source_kind: None,
            transform_space: TransformSpace::Local,
            initial_global_transform: None,
            bounding_geometry: None,
            opacity: 255.,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeKind {
    World,
    Sector,
    Object,
    Perso,
    Unknown(u32),
}

impl Content for Scene {
    const KIND: &'static str = "scene";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        if let Some(actor) = &self.main_actor {
            ensure!(
                self.nodes
                    .iter()
                    .filter(|n| n.perso.as_ref() == Some(actor))
                    .count()
                    == 1,
                "main actor must resolve to one scene perso"
            );
        }
        let mut global_sources = Vec::new();
        let mut global_persos = BTreeSet::new();
        for actor in &self.global_actors {
            ensure!(
                actor.source.role == super::ai::SourceRole::Fix,
                "global actor source must be Fix-owned"
            );
            ensure!(
                !global_sources.contains(&actor.source),
                "duplicate global actor source identity"
            );
            global_sources.push(actor.source.clone());
            ensure!(
                global_persos.insert(actor.perso.path.clone()),
                "duplicate global actor perso reference"
            );
        }
        ensure!(
            !self.nodes.is_empty() && self.nodes.len() <= 100_000,
            "invalid scene node count"
        );
        let mut seen = BTreeSet::new();
        for node in &self.nodes {
            ensure!(
                node.properties.opacity.is_finite(),
                "nonfinite node opacity"
            );
            if let Some(kind) = node.properties.source_kind {
                ensure!(
                    match node.kind {
                        NodeKind::World => kind == 0,
                        NodeKind::Sector => kind == 8,
                        NodeKind::Perso => kind == 4,
                        NodeKind::Object => kind == 0xd || kind == 0x15,
                        NodeKind::Unknown(n) => kind == n,
                    },
                    "source node kind disagrees with useful node kind"
                );
            }
            ensure!(
                !node.id.is_empty()
                    && node.id.split('/').all(|s| !s.is_empty()
                        && s.chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')),
                "invalid scene node ID"
            );
            ensure!(
                node.id.rsplit_once('/').map(|(p, _)| p) == node.parent.as_deref(),
                "scene ID does not match its parent path"
            );
            if let Some(parent) = &node.parent {
                ensure!(
                    seen.contains(parent),
                    "scene parent missing or not before child"
                );
            }
            ensure!(seen.insert(node.id.clone()), "duplicate scene node ID");
            for t in node
                .transform
                .iter()
                .chain(&node.properties.initial_global_transform)
            {
                ensure!(
                    t.translation
                        .iter()
                        .chain(t.rotation.iter().flatten())
                        .chain(t.basis.iter().flatten())
                        .all(|n| n.is_finite()),
                    "nonfinite scene transform"
                );
            }
        }
        let nodes: std::collections::BTreeMap<_, _> =
            self.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        for node in &self.nodes {
            if let Some(sector) = &node.sector {
                ensure!(
                    node.kind == NodeKind::Sector,
                    "sector component on non-sector node"
                );
                ensure!(
                    !sector.name.contains('\0') && sector.name.chars().all(|c| u32::from(c) <= 255),
                    "invalid sector name"
                );
                if let Some(boundary) = &sector.boundary {
                    boundary.validate()?;
                }
                for id in sector
                    .graphics
                    .iter()
                    .chain(&sector.optional_graphics)
                    .map(|s| &s.sector)
                    .chain(&sector.collision)
                    .chain(&sector.activity)
                    .chain(sector.sound.iter().map(|s| &s.sector))
                    .chain(
                        sector
                            .boundary
                            .iter()
                            .flat_map(|b| &b.surfaces)
                            .filter_map(|s| s.sector.as_ref()),
                    )
                {
                    ensure!(
                        nodes
                            .get(id.as_str())
                            .is_some_and(|n| n.kind == NodeKind::Sector && n.sector.is_some()),
                        "missing sector node: {id}"
                    );
                }
                for id in &sector.persos {
                    ensure!(
                        nodes
                            .get(id.as_str())
                            .is_some_and(|n| n.kind == NodeKind::Perso),
                        "missing sector perso node: {id}"
                    );
                }
            }
        }
        Ok(())
    }
}

/// Scene-owned membership and routing. All node IDs refer to this Scene.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Sector {
    pub name: String,
    pub virtual_sector: u8,
    pub activation: u32,
    pub environments: Vec<AssetRef<super::environment::Environment>>,
    pub static_lights: Vec<AssetRef<super::light::Light>>,
    pub dynamic_lights: Vec<AssetRef<super::light::Light>>,
    pub boundary: Option<SectorBoundary>,
    pub persos: Vec<String>,
    pub graphics: Vec<SectorVisibility>,
    pub optional_graphics: Vec<SectorVisibility>,
    pub collision: Vec<String>,
    pub activity: Vec<String>,
    pub sound: Vec<SectorSoundPropagation>,
    /// Source bytes following the virtual-sector flag; their semantics are unresolved.
    pub unknown: [u8; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectorVisibility {
    pub sector: String,
    pub level: u16,
    pub unknown: u16,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectorSoundPropagation {
    pub sector: String,
    pub volume: i32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectorBoundary {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub surfaces: Vec<SectorSurface>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectorSurface {
    /// A crossed face can lead to another sector. A null target is retained.
    pub sector: Option<String>,
    pub triangles: Vec<[u16; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub vertex_indices: Vec<u16>,
    pub texcoords: Vec<[f32; 2]>,
    pub uv_indices: Vec<[u16; 3]>,
}
impl SectorBoundary {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.positions.is_empty() && self.positions.len() <= i16::MAX as usize,
            "invalid sector boundary point count"
        );
        ensure!(
            self.normals.is_empty() || self.normals.len() == self.positions.len(),
            "sector boundary normal count mismatch"
        );
        ensure!(
            self.positions
                .iter()
                .chain(&self.normals)
                .flatten()
                .all(|v| v.is_finite()),
            "nonfinite sector boundary"
        );
        ensure!(
            self.surfaces.len() <= i16::MAX as usize,
            "too many sector surfaces"
        );
        for surface in &self.surfaces {
            ensure!(
                surface.texcoords.len() <= i16::MAX as usize
                    && surface.texcoords.iter().flatten().all(|v| v.is_finite()),
                "invalid sector surface UVs"
            );
            ensure!(
                surface.uv_indices.is_empty()
                    || surface.uv_indices.len() == surface.triangles.len(),
                "sector surface UV index count mismatch"
            );
            ensure!(
                surface
                    .uv_indices
                    .iter()
                    .flatten()
                    .all(|&v| usize::from(v) < surface.texcoords.len()),
                "sector UV index out of range"
            );
            ensure!(
                surface.triangles.len() <= i16::MAX as usize
                    && surface.vertex_indices.len() <= i16::MAX as usize,
                "too many sector triangles/vertices"
            );
            ensure!(
                surface.normals.is_empty() || surface.normals.len() == surface.triangles.len(),
                "sector surface normal count mismatch"
            );
            ensure!(
                surface.normals.iter().flatten().all(|v| v.is_finite()),
                "nonfinite sector surface normal"
            );
            ensure!(
                surface
                    .triangles
                    .iter()
                    .flatten()
                    .chain(&surface.vertex_indices)
                    .all(|&v| usize::from(v) < self.positions.len()),
                "sector boundary index out of range"
            );
        }
        Ok(())
    }
}
