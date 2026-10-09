//! Actor-owned camera framing. Source runtime targets and renderer handles
//! are session state, not persisted camera identities.
use super::ai::SourceObjectIdentity;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    pub source: SourceObjectIdentity,
    /// Whether the owning Perso has its motion binding. UpdatePosition's
    /// post-frame cleanup is conditional on this binding (00545d80).
    pub owner_motion_binding_present: bool,
    pub clip_planes: [f32; 2],
    /// Unpromoted component scalar bytes. Parameter, target and renderer
    /// pointer words are excluded; no source virtual addresses are retained.
    #[serde(with = "serde_bytes")]
    pub unknown: Vec<u8>,
    /// Source order is retained. Set 0 is the initial preset copied by reset;
    /// set 1 is active framing. The other sets retain their separate identities.
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
    /// `00440e30` reads the camera controller flags at source offset `+0x92`.
    /// The `0051f520` input writer ORs source action bits into this word.
    pub fn controller_flags(&self) -> u16 {
        u16::from_le_bytes(
            self.unknown[90..92]
                .try_into()
                .expect("validated camera parameter extent"),
        )
    }

    /// `00440ea0` reads the camera parameter flags at source offset `+0x94`.
    pub fn angle_flags(&self) -> u16 {
        u16::from_le_bytes(
            self.unknown[92..94]
                .try_into()
                .expect("validated camera parameter extent"),
        )
    }

    /// `00440bf0` / Function_Camera_GetShiftAlpha read the camera parameter
    /// float at source offset `+0x44`; the consumer treats it as an angular
    /// window width in radians, but the getter returns this value unchanged.
    pub fn shift_alpha(&self) -> f32 {
        f32::from_le_bytes(
            self.unknown[20..24]
                .try_into()
                .expect("validated camera extent"),
        )
    }

    /// `00440c30` / Function_Camera_GetShiftTeta read the camera parameter
    /// float at source offset `+0x4c` without changing its value.
    pub fn shift_teta(&self) -> f32 {
        f32::from_le_bytes(
            self.unknown[24..28]
                .try_into()
                .expect("validated camera extent"),
        )
    }

    /// This same source value is the current mode-zero upper pitch limit.
    pub fn pitch_upper_bound(&self) -> f32 {
        self.shift_teta()
    }

    /// 0051e350 clamps twice the target's motion speed to +6c..+68.
    /// 00528030 supplies that bound to the native translation consumer.
    pub fn follow_linear_speed_limits(&self) -> [f32; 2] {
        [self.follow_scalar(0x6c), self.follow_scalar(0x68)]
    }

    /// 00528030 supplies +74 as the distance-response denominator used by
    /// 00578588; +70 supplies its acceleration divisor through camera+70.
    pub fn follow_linear_response_distance(&self) -> f32 {
        self.follow_scalar(0x74)
    }

    pub fn follow_linear_acceleration_divisor(&self) -> f32 {
        self.follow_scalar(0x70)
    }

    /// 00528030 maps +78/+80/+7c to the angular speed, response scale and
    /// acceleration divisor consumed by 0057ab7d, respectively.
    pub fn follow_angular_speed(&self) -> f32 {
        self.follow_scalar(0x78)
    }

    pub fn follow_angular_response_scale(&self) -> f32 {
        self.follow_scalar(0x80)
    }

    pub fn follow_angular_acceleration_divisor(&self) -> f32 {
        self.follow_scalar(0x7c)
    }

    /// 0051db70 supplies +50/+54/+58 as the pitch, roll and yaw adjustments
    /// subsequently composed by 0057b17f. These are source radians.
    pub fn follow_view_angles(&self) -> [f32; 3] {
        [
            self.follow_scalar(0x50),
            self.follow_scalar(0x54),
            self.follow_scalar(0x58),
        ]
    }

    /// +84 limits changes to those three adjustments when action flags are zero.
    pub fn follow_view_angle_speed(&self) -> f32 {
        self.follow_scalar(0x84)
    }

    fn follow_scalar(&self, source_offset: usize) -> f32 {
        // The contiguous +4c..+8b span follows 24 retained scalar bytes.
        let index = 24 + source_offset - 0x4c;
        f32::from_le_bytes(
            self.unknown[index..index + 4]
                .try_into()
                .expect("validated camera parameter extent"),
        )
    }

    /// `0043daf0` reads the registered byte at parameter offset `+0x90` and
    /// the selected viewport context at `+0x9c`. These are retained in the
    /// source-order unknown tail rather than turned into source pointers.
    pub fn registered_viewport(&self) -> Option<u32> {
        if !self.registration_active() {
            return None;
        }
        let context = self.viewport_context();
        (context != 0).then_some(context)
    }

    /// Selection sets availability on the destination. Resetting the previous
    /// camera clears its active availability; switching without that reset
    /// clears its viewport context separately from this flag.
    pub fn registration_active(&self) -> bool {
        self.unknown.get(88).copied().unwrap_or(0) != 0
    }

    /// The selected viewport identity is scalar session state, not a pointer.
    pub fn viewport_context(&self) -> u32 {
        self.unknown
            .get(96..100)
            .and_then(|value| value.try_into().ok())
            .map(u32::from_le_bytes)
            .unwrap_or(0)
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

impl Camera {
    /// 0052bb50 updates the distinct signed retry counter at camera +ca.
    /// The pointer-free component tail begins at +20 after eight +14 bytes.
    pub fn static_retry_frames(&self) -> i16 {
        i16::from_le_bytes(
            self.unknown[178..180]
                .try_into()
                .expect("validated camera component extent"),
        )
    }

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
    fn camera_shift_alpha_and_shift_teta_are_read_from_source_offsets() {
        let mut value = parameters();
        value.unknown[20..24].copy_from_slice(&1.25_f32.to_le_bytes());
        value.unknown[24..28].copy_from_slice(&0.75_f32.to_le_bytes());
        assert_eq!(value.shift_alpha(), 1.25);
        assert_eq!(value.shift_teta(), 0.75);
        assert_eq!(value.pitch_upper_bound(), 0.75);
    }

    #[test]
    fn camera_angle_flags_are_read_from_source_offset_94() {
        let mut value = parameters();
        value.unknown[92..94].copy_from_slice(&0x0004_u16.to_le_bytes());
        assert_eq!(value.angle_flags(), 4);
    }

    #[test]
    fn camera_controller_flags_are_read_from_source_offset_92() {
        let mut value = parameters();
        value.unknown[90..92].copy_from_slice(&0x8040_u16.to_le_bytes());
        assert_eq!(value.controller_flags(), 0x8040);
    }

    #[test]
    fn camera_follow_controls_use_the_frame_consumers_source_offsets() {
        let mut value = parameters();
        for (offset, real) in [
            (0x68, 200.),
            (0x6c, 12.),
            (0x70, 25.),
            (0x74, 7.),
            (0x78, 3.25),
            (0x7c, 10.),
            (0x80, 5.),
            (0x50, 0.25),
            (0x54, -0.125),
            (0x58, 0.5),
            (0x84, 2.),
        ] {
            let index = offset - 0x4c + 24;
            value.unknown[index..index + 4].copy_from_slice(&f32::to_le_bytes(real));
        }
        assert_eq!(value.follow_linear_speed_limits(), [12., 200.]);
        assert_eq!(value.follow_linear_response_distance(), 7.);
        assert_eq!(value.follow_linear_acceleration_divisor(), 25.);
        assert_eq!(value.follow_angular_speed(), 3.25);
        assert_eq!(value.follow_angular_response_scale(), 5.);
        assert_eq!(value.follow_angular_acceleration_divisor(), 10.);
        assert_eq!(value.follow_view_angles(), [0.25, -0.125, 0.5]);
        assert_eq!(value.follow_view_angle_speed(), 2.);
    }

    #[test]
    fn camera_static_retry_count_is_a_distinct_signed_word() {
        let mut value = Camera {
            source: parameters().source,
            owner_motion_binding_present: true,
            clip_planes: [0.25, 10000.],
            unknown: vec![0; 196],
            parameters: std::array::from_fn(|_| parameters()),
        };
        value.unknown[178..180].copy_from_slice(&(-7_i16).to_le_bytes());
        value.unknown[172..174].copy_from_slice(&21_i16.to_le_bytes());
        assert_eq!(value.static_retry_frames(), -7);
    }
}
