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
    /// `00440ea0` reads the camera parameter flags at source offset `+0x94`.
    pub fn angle_flags(&self) -> u16 {
        u16::from_le_bytes(
            self.unknown[92..94]
                .try_into()
                .expect("validated camera parameter extent"),
        )
    }

    /// `00440bf0` reads the camera parameter float at source offset `+0x44`.
    /// The current useful model keeps this scalar in its source-ordered tail.
    pub fn yaw_upper_bound(&self) -> f32 {
        f32::from_le_bytes(
            self.unknown[20..24]
                .try_into()
                .expect("validated camera extent"),
        )
    }

    /// `00440c30` reads the camera parameter float at source offset `+0x4c`.
    pub fn pitch_upper_bound(&self) -> f32 {
        f32::from_le_bytes(
            self.unknown[24..28]
                .try_into()
                .expect("validated camera extent"),
        )
    }

    /// `0043daf0` reads the registered byte at parameter offset `+0x90` and
    /// the selected viewport context at `+0x9c`. These are retained in the
    /// source-order unknown tail rather than turned into source pointers.
    pub fn registered_viewport(&self) -> Option<u32> {
        if self.unknown.get(88).copied()? == 0 {
            return None;
        }
        let context = u32::from_le_bytes(self.unknown.get(96..100)?.try_into().ok()?);
        (context != 0).then_some(context)
    }

    /// `00440f80` reads the target child selector at parameter byte `+0xa0`.
    /// In this model's pointer-free 104-byte unknown tail that is byte 100;
    /// -1 means use the selected Perso itself.
    pub fn target_child_index(&self) -> i8 {
        self.unknown[100] as i8
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    fn parameters() -> CameraParameters {
        CameraParameters {
            source: SourceObjectIdentity {
                role: super::super::ai::SourceRole::Fix,
                block: [5, 0],
                allocation: 1,
            },
            target_offset: [0.; 3],
            look_offset: [0.; 3],
            actor_offset: [0.; 3],
            distance: [0.; 2],
            yaw: 0.,
            pitch: 0.,
            horizontal_fov: 0.,
            mode: 0,
            unknown: vec![0; 104],
        }
    }

    #[test]
    fn camera_registration_uses_active_byte_and_context_word() {
        let mut value = parameters();
        value.unknown[96..100].copy_from_slice(&1_u32.to_le_bytes());
        assert_eq!(value.registered_viewport(), None);
        value.unknown[88] = 1;
        assert_eq!(value.registered_viewport(), Some(1));
        value.unknown[96..100].copy_from_slice(&2_u32.to_le_bytes());
        assert_eq!(value.registered_viewport(), Some(2));
        value.unknown[96..100].copy_from_slice(&0_u32.to_le_bytes());
        assert_eq!(value.registered_viewport(), None);
    }

    #[test]
    fn camera_target_child_selector_uses_source_parameter_byte_a0() {
        let mut value = parameters();
        assert_eq!(value.target_child_index(), 0);
        value.unknown[100] = 0xff;
        assert_eq!(value.target_child_index(), -1);
        value.unknown[100] = 7;
        assert_eq!(value.target_child_index(), 7);
    }

    #[test]
    fn camera_angle_upper_bounds_are_read_from_source_offsets() {
        let mut value = parameters();
        value.unknown[20..24].copy_from_slice(&1.25_f32.to_le_bytes());
        value.unknown[24..28].copy_from_slice(&0.75_f32.to_le_bytes());
        assert_eq!(value.yaw_upper_bound(), 1.25);
        assert_eq!(value.pitch_upper_bound(), 0.75);
    }

    #[test]
    fn camera_angle_flags_are_read_from_source_offset_94() {
        let mut value = parameters();
        value.unknown[92..94].copy_from_slice(&0x0004_u16.to_le_bytes());
        assert_eq!(value.angle_flags(), 4);
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
