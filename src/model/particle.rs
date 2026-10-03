//! Particle emission definitions; actor selection belongs to its component.
use super::{AssetPath, AssetRef, ai::SourceObjectIdentity, io::Content, sprite::Sprite};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

/// One generator owns these settings in the verified manor dependency graph.
/// Revisit the embedded boundary if another source graph independently shares
/// its emission, system or environment definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticleGenerator {
    pub path: AssetPath,
    pub source_identity: Option<ParticleIdentity>,
    pub capacity: u16,
    pub sprite: AssetRef<Sprite>,
    pub emission: ParticleEmission,
    pub environment: ParticleEnvironment,
    pub relative_environment: bool,
    pub medium_filter: u32,
    pub unknown: [u8; 2],
    pub system_flags: [u8; 4],
    pub system_unknown_16: [u8; 8],
    pub system_unknown_30: [u8; 2],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticleIdentity {
    pub generator: SourceObjectIdentity,
    pub emission: SourceObjectIdentity,
    pub system: SourceObjectIdentity,
    pub environment: SourceObjectIdentity,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticleEmission {
    /// 0 disabled, 1 continuous, 2 probability, 3 alternating periods.
    pub mode: u8,
    pub unknown_1: [u8; 3],
    pub probability: f32,
    pub paused_updates: u16,
    pub period_updates: u16,
    pub number_mode: u8,
    pub unknown_15: u8,
    pub number: u16,
    pub number_range: [u16; 2],
    /// 0 point, 1 parallelogram (three corners), 2 parallelepiped (four).
    pub shape: u8,
    /// 0 orientation only, 1 translation only, 2 full actor transform.
    pub coordinate_mode: u8,
    pub corners: [[f32; 3]; 4],
    pub velocity: [f32; 3],
    pub velocity_spread: [f32; 3],
    /// 0 infinite, 1 constant, 2 random between the bounds.
    pub lifetime_mode: u8,
    pub unknown_109: [u8; 3],
    pub lifetime: [f32; 2],
    /// Unresolved segment-generation tail; retained for source reconstruction.
    pub unknown_120: [u8; 12],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticleEnvironment {
    /// Zero selects box; one selects sphere.
    pub bounds_kind: u32,
    pub box_bounds: [[f32; 3]; 2],
    pub sphere_center: [f32; 3],
    pub sphere_radius: f32,
    pub acceleration: [f32; 3],
    pub acceleration_spread: [f32; 3],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticleComponent {
    pub selected: Option<AssetRef<ParticleGenerator>>,
    pub enabled: bool,
    pub unknown_4: [u8; 4],
    pub unknown_10: [u8; 6],
}
impl Content for ParticleGenerator {
    const KIND: &'static str = "particle-generator";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        let e = &self.emission;
        let v = &self.environment;
        ensure!(
            self.capacity > 0 && self.sprite.path.member().is_none(),
            "invalid particle capacity/sprite"
        );
        ensure!(
            e.mode <= 3
                && e.number_mode <= 1
                && e.shape <= 2
                && e.coordinate_mode <= 2
                && e.lifetime_mode <= 2,
            "unsupported particle emission profile"
        );
        ensure!(
            v.bounds_kind <= 1 && v.sphere_radius >= 0.,
            "invalid particle boundary"
        );
        ensure!(
            e.corners
                .iter()
                .flatten()
                .chain(&e.velocity)
                .chain(&e.velocity_spread)
                .chain(&e.lifetime)
                .chain(std::iter::once(&e.probability))
                .chain(v.box_bounds.iter().flatten())
                .chain(&v.sphere_center)
                .chain(std::iter::once(&v.sphere_radius))
                .chain(&v.acceleration)
                .chain(&v.acceleration_spread)
                .all(|n| n.is_finite()),
            "nonfinite particle definition"
        );
        ensure!(
            e.velocity_spread
                .iter()
                .chain(&v.acceleration_spread)
                .all(|n| *n >= 0.),
            "negative particle random extent"
        );
        Ok(())
    }
}
