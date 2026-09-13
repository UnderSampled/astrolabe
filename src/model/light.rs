//! Shared source lights. Signed color values and separate switches remain editable.
use super::{AssetPath, geometry::SourceTransform, io::Content};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Light {
    pub path: AssetPath,
    pub name: String,
    pub kind: u32,
    pub on: u8,
    pub cast_shadows: u8,
    pub rotating: u8,
    pub pulsing: u8,
    pub far: f32,
    pub near: f32,
    /// Spotlight angles, or fog infinite/blend-near parameters for kind 6.
    pub angles: [f32; 2],
    pub rotation_step: f32,
    pub pulse_step: f32,
    pub transform: SourceTransform,
    pub color: [f32; 4],
    pub painting: u32,
    pub creates_shadows: u32,
    pub send: u8,
    /// Minimum intensity, or fog blend-far for kind 6.
    pub intensity_min: f32,
    /// The referenced dimmer is not decoded yet; writers must not silently drop it.
    pub unresolved_dimmer: bool,
    pub unknown: LightUnknown,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightUnknown {
    pub header: u32,
    pub after_transform: [u32; 4],
    pub after_painting: u32,
    pub before_send: u8,
    pub after_send: [u8; 2],
    pub name_tail: Vec<u8>,
}
impl Content for Light {
    const KIND: &'static str = "light";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            self.name.chars().all(|c| c != '\0' && (c as u32) <= 255),
            "invalid source light name"
        );
        ensure!(
            self.name.chars().count() + self.unknown.name_tail.len() <= 20,
            "light name exceeds source capacity"
        );
        ensure!(
            self.unknown.name_tail.is_empty() || self.unknown.name_tail[0] == 0,
            "invalid light name tail"
        );
        ensure!(
            [
                self.far,
                self.near,
                self.rotation_step,
                self.pulse_step,
                self.intensity_min
            ]
            .iter()
            .chain(&self.angles)
            .chain(&self.color)
            .all(|v| v.is_finite()),
            "nonfinite light value"
        );
        super::animation::TransformValue::Affine(self.transform.clone()).validate()
    }
}
