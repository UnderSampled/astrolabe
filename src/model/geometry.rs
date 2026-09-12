//! Useful source geometry. Source axis order and separate corner indices survive persistence.
use super::{AssetPath, AssetRef};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceTransform {
    pub translation: [f32; 3],
    /// Column-major unscaled rotation and full affine basis, kept without normalization.
    pub rotation: [[f32; 3]; 3],
    pub basis: [[f32; 3]; 3],
}
impl Default for SourceTransform {
    fn default() -> Self {
        Self {
            translation: [0.; 3],
            rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            basis: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Model {
    pub path: AssetPath,
    pub name: String,
    pub lod_distance: f32,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    /// Selected instance/LOD per-position lighting, signed fixed point with scale 1/256.
    /// Empty if absent. Components are unclamped lighting contributions, not albedo/opacity.
    pub radiosity: Vec<[i16; 4]>,
    pub submeshes: Vec<Submesh>,
    /// Optional source per-point gameplay material assignments, independent of face groups.
    pub vertex_materials: Vec<AssetRef<Material>>,
    /// Root-to-instance transforms. Model coordinates themselves remain local.
    pub instance_transforms: Vec<SourceTransform>,
}
impl Model {
    /// Present exact source lighting as floats without losing negative or HDR components.
    pub fn radiosity_rgba(&self, index: usize) -> Option<[f32; 4]> {
        self.radiosity
            .get(index)
            .map(|v| v.map(|c| f32::from(c) / 256.))
    }
    pub fn bounds(&self) -> Option<([f32; 3], [f32; 3])> {
        let mut lo = *self.positions.first()?;
        let mut hi = lo;
        for p in &self.positions {
            for i in 0..3 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
        }
        Some((lo, hi))
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(self.lod_distance.is_finite(), "nonfinite LOD distance");
        for t in &self.instance_transforms {
            ensure!(
                t.translation
                    .iter()
                    .chain(t.rotation.iter().flatten())
                    .chain(t.basis.iter().flatten())
                    .all(|n| n.is_finite()),
                "nonfinite instance transform"
            );
        }

        ensure!(
            !self.positions.is_empty() && self.positions.len() <= i16::MAX as usize,
            "unsupported vertex count"
        );
        ensure!(
            self.normals.is_empty() || self.normals.len() == self.positions.len(),
            "normal count differs from vertex count"
        );
        ensure!(
            self.radiosity.is_empty() || self.radiosity.len() == self.positions.len(),
            "radiosity count differs from vertex count"
        );
        ensure!(
            self.vertex_materials.is_empty() || self.vertex_materials.len() == self.positions.len(),
            "point material count differs from vertex count"
        );
        ensure!(
            self.positions
                .iter()
                .chain(&self.normals)
                .flatten()
                .all(|n| n.is_finite()),
            "nonfinite geometry"
        );
        ensure!(
            self.submeshes.len() <= i16::MAX as usize,
            "too many submeshes"
        );
        for sm in &self.submeshes {
            ensure!(
                sm.triangles.len() <= i16::MAX as usize && sm.texcoords.len() <= i16::MAX as usize,
                "submesh exceeds Hype index limit"
            );
            ensure!(
                sm.triangles.len() == sm.uv_indices.len(),
                "UV corner count differs from triangles"
            );
            ensure!(
                sm.face_normals.is_empty() || sm.face_normals.len() == sm.triangles.len(),
                "face normal count differs from triangles"
            );
            ensure!(
                sm.triangles
                    .iter()
                    .flatten()
                    .all(|&i| usize::from(i) < self.positions.len()),
                "triangle index out of range"
            );
            ensure!(
                sm.uv_indices
                    .iter()
                    .flatten()
                    .all(|&i| usize::from(i) < sm.texcoords.len()),
                "UV index out of range"
            );
            ensure!(
                sm.vertex_indices
                    .iter()
                    .all(|&i| usize::from(i) < self.positions.len()),
                "point subset index out of range"
            );
            ensure!(
                sm.texcoords
                    .iter()
                    .flatten()
                    .chain(sm.face_normals.iter().flatten())
                    .all(|n| n.is_finite()),
                "nonfinite submesh values"
            );
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Submesh {
    pub material: AssetRef<Material>,
    pub triangles: Vec<[u16; 3]>,
    pub texcoords: Vec<[f32; 2]>,
    pub uv_indices: Vec<[u16; 3]>,
    pub face_normals: Vec<[f32; 3]>,
    /// Source point subset used by the triangle element; not an encoded stream.
    pub vertex_indices: Vec<u16>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Material {
    pub path: AssetPath,
    pub texture: Option<AssetPath>,
    pub texture_info: Option<TextureInfo>,
    pub ambient: [f32; 4],
    pub diffuse: [f32; 4],
    pub specular: [f32; 4],
    pub color: [f32; 4],
    pub flags: u32,
    pub scroll_speed: [f32; 2],
    pub scroll_position: [f32; 2],
    pub scroll_mode: u32,
    pub sound_material: u32,
}

/// Texture reference properties useful to sampling and source export; pixel content stays in its asset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextureInfo {
    /// Original authored texture name, independent of the resolved content file/alias.
    /// Retains extension and case when the dependency resolves to a shared GF asset.
    pub source_name: Option<String>,
    pub stored_size: [u32; 2],
    pub logical_size: [u32; 2],
    pub flags: u32,
    pub addressing: u8,
}
impl Material {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.texture
                .as_ref()
                .is_none_or(|p| p.role() == super::PackageRole::Texture),
            "material texture reference has wrong package role"
        );
        ensure!(
            self.ambient
                .iter()
                .chain(&self.diffuse)
                .chain(&self.specular)
                .chain(&self.color)
                .chain(&self.scroll_speed)
                .chain(&self.scroll_position)
                .all(|n| n.is_finite()),
            "nonfinite material field"
        );
        ensure!(
            self.texture.is_some() == self.texture_info.is_some(),
            "texture reference and sampling information disagree"
        );
        if let Some(info) = &self.texture_info {
            if let Some(name) = &info.source_name {
                ensure!(
                    !name.is_empty() && name.len() < 80 && !name.contains('\0'),
                    "invalid source texture name"
                );
            }
            ensure!(
                info.stored_size
                    .iter()
                    .chain(&info.logical_size)
                    .all(|&v| v > 0 && v <= 65536),
                "invalid texture dimensions"
            );
        }
        Ok(())
    }
}
