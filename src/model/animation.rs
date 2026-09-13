//! Sampled animation transforms preserve quaternion values and full symmetric stretch.
use super::{AssetPath, AssetRef, geometry::SourceTransform, io::Content};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TransformValue {
    /// Authored quaternion in [w,x,y,z] order. Quantized values are not normalized.
    Components {
        translation: [f32; 3],
        quaternion: [f32; 4],
        stretch: [f32; 6],
    },
    Affine(SourceTransform),
}
impl Default for TransformValue {
    fn default() -> Self {
        Self::Components {
            translation: [0.; 3],
            quaternion: [1., 0., 0., 0.],
            stretch: [1., 0., 0., 1., 0., 1.],
        }
    }
}
impl TransformValue {
    pub fn validate(&self) -> Result<()> {
        let t = self.affine();
        ensure!(
            t.translation
                .iter()
                .chain(t.rotation.iter().flatten())
                .chain(t.basis.iter().flatten())
                .all(|x| x.is_finite()),
            "nonfinite animation transform"
        );
        Ok(())
    }
    pub fn affine(&self) -> SourceTransform {
        match self {
            Self::Affine(t) => t.clone(),
            Self::Components {
                translation,
                quaternion: [w, x, y, z],
                stretch: [a, b, c, d, e, f],
            } => {
                // POS_fn_vQuatToMat (004bf250), without quaternion normalization.
                let rotation = [
                    [
                        1. - (2. * z * z + 2. * y * y),
                        2. * y * x - 2. * z * w,
                        2. * z * x + 2. * y * w,
                    ],
                    [
                        2. * y * x + 2. * z * w,
                        1. - (2. * z * z + 2. * x * x),
                        2. * z * y - 2. * x * w,
                    ],
                    [
                        2. * z * x - 2. * y * w,
                        2. * z * y + 2. * x * w,
                        1. - (2. * y * y + 2. * x * x),
                    ],
                ];
                let scale = [[*a, *b, *c], [*b, *d, *e], [*c, *e, *f]];
                let basis = std::array::from_fn(|column| {
                    std::array::from_fn(|row| {
                        (0..3).map(|k| rotation[k][row] * scale[column][k]).sum()
                    })
                });
                SourceTransform {
                    translation: *translation,
                    rotation,
                    basis,
                }
            }
        }
    }
}

/// Shared transforms in original relative allocation order within a source pool.
/// Entries with equal values remain distinct; channel references carry their index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransformPool {
    pub path: AssetPath,
    pub entries: Vec<TransformEntry>,
}
impl Content for TransformPool {
    const KIND: &'static str = "transform-pool";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        ensure!(self.entries.len() <= 1_000_000, "excessive transform pool");
        ensure!(
            self.entries.windows(2).all(|p| p[0].id < p[1].id),
            "transform IDs must be ordered and unique"
        );
        let mut previous = None;
        for entry in &self.entries {
            if let Some(index) = entry.source_index {
                ensure!(
                    previous.is_none_or(|last| last < index),
                    "transform source order is not strictly increasing"
                );
                previous = Some(index);
            }
            entry.value.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransformEntry {
    pub id: u32,
    /// Allocation ordinal within this pool; distinct equal values retain separate entries.
    pub source_index: Option<u32>,
    pub value: TransformValue,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransformLink {
    pub pool: super::AssetRef<TransformPool>,
    pub entry: u32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationClip {
    pub path: AssetPath,
    pub channel_count: u8,
    pub frame_rate: u8,
    pub speed: SourceTransform,
    pub samples: Vec<ChannelSample>,
    pub frames: Vec<AnimationFrame>,
    pub cues: Vec<AnimationCue>,
    /// Source vertex-morph count; morph payloads are not yet represented.
    pub morph_count: u16,
    pub unknown: Vec<u8>,
}
/// A shared action can be selected by several configurations and animation cues.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationAction {
    pub path: AssetPath,
    pub operation: AnimationOperation,
    /// Uninterpreted descriptor bytes +5..+11, including source suppression state.
    pub unknown: [u8; 7],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnimationOperation {
    Sound {
        event: Option<super::audio::SoundEventLink>,
    },
    Mechanics {
        command: u32,
    },
    Attachment {
        value: u32,
    },
    AttachmentControl {
        command: u32,
    },
    Camera {
        command: u32,
    },
    Unknown {
        kind: u8,
        payload: [u8; 4],
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationCue {
    pub action: AssetRef<AnimationAction>,
    pub frame: u8,
    /// Frame column, not the sample's logical channel ID.
    pub channel: u8,
    /// Original configuration slot, retained independently of descriptor sharing.
    pub configuration_slot: u16,
}
impl Content for AnimationAction {
    const KIND: &'static str = "animation-action";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        if let AnimationOperation::Sound { event: Some(event) } = &self.operation {
            event.validate()?;
        }
        if let AnimationOperation::Unknown { kind, .. } = self.operation {
            ensure!(
                kind > 4,
                "known animation operation must use its typed variant"
            );
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChannelSample {
    pub transform: Option<TransformLink>,
    /// Configuration slot for Object, auxiliary light slot for Light.
    pub object: Option<u16>,
    /// Logical channel identity, independent of this frame's column order.
    pub channel: Option<u8>,
    /// Source default visibility (0 hidden, nonzero visible); runtime overrides are separate.
    pub visibility: u8,
    pub opacity: u8,
    pub kind: ChannelKind,
    pub unknown: [u8; 6],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelKind {
    Object,
    Hidden,
    Light,
    Unknown(u8),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationFrame {
    pub samples: Vec<u32>,
    /// Ordered [child,parent] pairs. Unlisted channels attach to the character root.
    pub hierarchy: Vec<[u8; 2]>,
    pub motion_rotation: Option<[[f32; 3]; 3]>,
    pub motion_translation: Option<[f32; 3]>,
}
impl Content for AnimationClip {
    const KIND: &'static str = "animation";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            !self.frames.is_empty() && self.frames.len() <= 255 && self.channel_count > 0,
            "invalid animation dimensions"
        );
        TransformValue::Affine(self.speed.clone()).validate()?;
        ensure!(self.cues.len() <= 255, "excessive animation cue count");
        ensure!(
            self.cues.windows(2).all(|w| w[0].frame <= w[1].frame),
            "animation cues must be in frame order"
        );
        for cue in &self.cues {
            ensure!(
                (cue.frame as usize) < self.frames.len() && cue.channel < self.channel_count,
                "animation cue frame/channel out of range"
            );
        }
        for sample in &self.samples {
            ensure!(
                sample.channel != Some(255) && sample.object != Some(u16::MAX),
                "animation sentinel must use absent reference"
            );
        }
        for frame in &self.frames {
            ensure!(
                frame
                    .motion_rotation
                    .iter()
                    .flatten()
                    .flatten()
                    .chain(frame.motion_translation.iter().flatten())
                    .all(|x| x.is_finite()),
                "nonfinite frame motion"
            );
            ensure!(
                frame.samples.len() == self.channel_count as usize,
                "animation channel count mismatch"
            );
            ensure!(
                frame
                    .samples
                    .iter()
                    .all(|&i| (i as usize) < self.samples.len()),
                "animation sample index out of range"
            );
            let mut parents = vec![None; self.channel_count as usize];
            for &[child, parent] in &frame.hierarchy {
                ensure!(
                    child < self.channel_count
                        && parent < self.channel_count
                        && parents[child as usize].is_none(),
                    "invalid or repeated animation parent"
                );
                parents[child as usize] = Some(parent);
            }
            for start in 0..self.channel_count {
                let mut parent = parents[start as usize];
                let mut steps = 0;
                while let Some(p) = parent {
                    steps += 1;
                    ensure!(steps < self.channel_count, "cyclic animation hierarchy");
                    parent = parents[p as usize];
                }
            }
        }
        Ok(())
    }
}
