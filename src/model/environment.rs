//! Shared sector environments. Physical and visual components retain their
//! independent identities; acoustic and water components remain unresolved.
use super::{AssetPath, AssetRef, io::Content};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Environment {
    pub path: AssetPath,
    pub physical: Option<AssetRef<PhysicalEnvironment>>,
    pub visual: Option<AssetRef<VisualEnvironment>>,
    pub unresolved_sound: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysicalEnvironment {
    pub path: AssetPath,
    /// Magnitude in source units per second squared. Direction is `vertical`.
    pub gravity: f32,
    /// Source-coordinate vertical vector, retained without normalization.
    pub vertical: [f32; 3],
    /// Unpromoted source fields at +16..+47 of the 48-byte record.
    pub retained: [u8; 32],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualEnvironment {
    pub path: AssetPath,
    pub fog: Option<AssetRef<Fog>>,
    pub unresolved_water: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fog {
    pub path: AssetPath,
    pub near: f32,
    pub far: f32,
    /// Retained source distance; the active Hype Glide fog driver ignores it.
    pub infinite: f32,
    pub color: [f32; 3],
    pub unknown: u32,
}
impl Content for Environment {
    const KIND: &'static str = "environment";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}
impl Content for PhysicalEnvironment {
    const KIND: &'static str = "physical_environment";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            self.gravity.is_finite() && self.vertical.iter().all(|v| v.is_finite()),
            "nonfinite physical environment value"
        );
        Ok(())
    }
}
impl Content for VisualEnvironment {
    const KIND: &'static str = "visual_environment";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}
impl Content for Fog {
    const KIND: &'static str = "fog";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            [self.near, self.far, self.infinite]
                .iter()
                .chain(&self.color)
                .all(|v| v.is_finite()),
            "nonfinite fog value"
        );
        Ok(())
    }
}
