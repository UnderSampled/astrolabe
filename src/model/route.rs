//! Shared route geometry and actor-local bindings. Progress belongs to each actor.
use super::{AssetPath, AssetRef, ai::SourceObjectIdentity, io::Content};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Waypoint {
    pub path: AssetPath,
    pub source: SourceObjectIdentity,
    pub position: [f32; 3],
    pub radius: f32,
    /// Unpromoted actor-relative binding; fixed points have no binding.
    pub relative_actor: Option<SourceObjectIdentity>,
    pub unknown: [u8; 4],
    pub tail: [u8; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Route {
    pub path: AssetPath,
    pub source: SourceObjectIdentity,
    pub points: Vec<AssetRef<Waypoint>>,
    pub links: Vec<Option<RouteLink>>,
    pub current: u8,
    pub previous: u8,
    pub unknown: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteLink {
    pub source: SourceObjectIdentity,
    /// Other connection geometry remains an explicit preservation placeholder.
    pub geometry: Option<SourceObjectIdentity>,
    /// Promoted StaticLine endpoint (00530e50/00530ff0); other kinds are pending.
    pub static_endpoint: Option<AssetRef<Waypoint>>,
    pub connection: u32,
    pub dynamic_kind: u32,
    pub sample: u8,
    pub used: u8,
    pub length: f32,
    pub unknown: [u8; 2],
    #[serde(with = "serde_bytes")]
    pub dynamics: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteBinding {
    pub route: Option<AssetRef<Route>>,
    pub progress: [u8; 4],
}

impl Content for Waypoint {
    const KIND: &'static str = "waypoint";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            self.position.iter().all(|v| v.is_finite())
                && self.radius.is_finite()
                && self.radius >= 0.0,
            "invalid waypoint geometry"
        );
        Ok(())
    }
}
impl Content for Route {
    const KIND: &'static str = "route";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            self.points.len() <= 255 && self.links.len() == self.points.len().saturating_sub(1),
            "route point/link counts differ"
        );
        ensure!(
            self.points.iter().all(|r| r.path.member().is_none()),
            "route point must identify its asset"
        );
        for link in self.links.iter().flatten() {
            ensure!(
                link.length.is_finite() && link.length >= 0.0 && link.dynamics.len() == 52,
                "invalid route link"
            );
            ensure!(
                link.static_endpoint.is_none() || link.connection == 0,
                "static endpoint on another connection kind"
            );
        }
        Ok(())
    }
}
