//! Shared mechanics identity cards selected by character states. Simulation is separate.
use super::{AssetPath, io::Content};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mechanics {
    pub path: AssetPath,
    pub data: MechanicsData,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MechanicsData {
    Character {
        mode: u32,
        settings: CharacterMechanics,
    },
    Mass {
        mass: f32,
    },
    /// Preserved, unpromoted fields of a documented pointer-free card allocation.
    Unresolved {
        kind: u32,
        #[serde(with = "serde_bytes")]
        body: Vec<u8>,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterMechanics {
    pub mass: f32,
    pub stairs_step_height: f32,
    pub min_linear_speed: f32,
    pub min_angular_speed: f32,
    pub max_linear_speed: f32,
    pub max_angular_speed: f32,
    pub fsl_available: u8,
    pub fsl_unknown: [u8; 3],
    pub ground_limit: f32,
    pub slope_limit: f32,
    pub wall_limit: f32,
    pub attic_limit: f32,
    pub step_front_limit: f32,
    pub step_high_limit: f32,
    pub orientation_free: u8,
    pub orientation_unknown: [u8; 3],
    pub adaptive_velocity: f32,
    pub dt_forced: u8,
    pub dt_unknown: [u8; 3],
    pub dt_limit: f32,
    pub against_wall: u8,
    pub wall_unknown: [u8; 3],
    pub adaptive_wall_velocity: f32,
    pub bring_down_height: f32,
    pub air_resisting_surface: f32,
    pub ratio: f32,
}
impl Content for Mechanics {
    const KIND: &'static str = "mechanics";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        match &self.data {
            MechanicsData::Character { mode, settings: s } => {
                ensure!(
                    matches!(mode, 0 | 4 | 18),
                    "unsupported character mechanics mode"
                );
                ensure!(
                    [
                        s.mass,
                        s.stairs_step_height,
                        s.min_linear_speed,
                        s.min_angular_speed,
                        s.max_linear_speed,
                        s.max_angular_speed,
                        s.ground_limit,
                        s.slope_limit,
                        s.wall_limit,
                        s.attic_limit,
                        s.step_front_limit,
                        s.step_high_limit,
                        s.adaptive_velocity,
                        s.dt_limit,
                        s.adaptive_wall_velocity,
                        s.bring_down_height,
                        s.air_resisting_surface,
                        s.ratio
                    ]
                    .iter()
                    .all(|v| v.is_finite()),
                    "nonfinite mechanics parameter"
                );
            }
            MechanicsData::Mass { mass } => ensure!(mass.is_finite(), "nonfinite mass"),
            MechanicsData::Unresolved { kind, body } => {
                let size = match kind {
                    3 => 12,
                    6 => 88,
                    7 => 48,
                    11 => 16,
                    19 => 4,
                    _ => anyhow::bail!("unsupported opaque mechanics kind"),
                };
                ensure!(body.len() == size, "opaque mechanics field length differs");
            }
        }
        Ok(())
    }
}
