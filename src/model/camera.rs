//! Actor-owned camera framing. Source runtime targets and renderer handles
//! are session state, not persisted camera identities.
use super::ai::SourceObjectIdentity;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    pub source: SourceObjectIdentity,
    pub clip_planes: [f32; 2],
    /// Unpromoted component scalar bytes. Parameter, target and renderer
    /// pointer words are excluded; no source virtual addresses are retained.
    #[serde(with = "serde_bytes")]
    pub unknown: Vec<u8>,
    /// Source order is retained. Parameter set 1 supplies the camera's initial
    /// framing; the other sets are preserved without inventing preset roles.
    pub parameters: [CameraParameters; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraParameters {
    pub source: SourceObjectIdentity,
    pub target_offset: [f32; 3],
    pub look_offset: [f32; 3],
    /// Actor-relative base shift shared by target and look calculations (0051c8d0).
    pub actor_offset: [f32; 3],
    pub distance: [f32; 2],
    pub yaw: f32,
    pub pitch: f32,
    pub horizontal_fov: f32,
    /// Source getter 00440f40; mode-specific motion is not inferred from its number.
    pub mode: u32,
    /// Unpromoted scalar fields, in source order around the named values.
    /// These records contain no relocated pointers in the demonstrated build.
    #[serde(with = "serde_bytes")]
    pub unknown: Vec<u8>,
}

impl CameraParameters {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.unknown.len() == 104,
            "camera parameter unknown extent differs"
        );
        ensure!(
            self.target_offset
                .iter()
                .chain(&self.look_offset)
                .chain(&self.actor_offset)
                .chain(&self.distance)
                .chain([&self.yaw, &self.pitch, &self.horizontal_fov])
                .all(|v| v.is_finite()),
            "nonfinite camera framing"
        );
        // Empty preserved parameter sets legitimately have zero FoV.
        ensure!(
            self.horizontal_fov >= 0. && self.horizontal_fov < std::f32::consts::PI,
            "camera horizontal FoV outside range"
        );
        ensure!(
            self.distance[0] >= 0. && self.distance[0] <= self.distance[1],
            "camera distance limits differ"
        );
        Ok(())
    }
}

impl Camera {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.unknown.len() == 196,
            "camera component unknown extent differs"
        );
        ensure!(
            self.clip_planes.iter().all(|v| v.is_finite())
                && self.clip_planes[0] > 0.
                && self.clip_planes[0] < self.clip_planes[1],
            "invalid camera clip planes"
        );
        for parameters in &self.parameters {
            parameters.validate()?;
        }
        ensure!(
            self.parameters[1].horizontal_fov > 0.,
            "initial camera FoV is empty"
        );
        Ok(())
    }
}
