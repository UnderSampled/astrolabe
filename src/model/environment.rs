//! Shared sector environments. Physical, acoustic and water components remain
//! explicitly unresolved; visual environments and fog retain independent identity.
use super::{AssetPath, AssetRef, io::Content};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Environment {
    pub path: AssetPath,
    pub visual: Option<AssetRef<VisualEnvironment>>,
    pub unresolved_mechanics: bool,
    pub unresolved_sound: bool,
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
