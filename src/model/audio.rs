//! Useful audio values, independent of source containers and playback targets.
use super::io::Content;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AudioSamples {
    Pcm16(Vec<i16>),
    /// Unclipped decoder output. Quantization for playback belongs to the target adapter.
    Float32(Vec<f32>),
}
impl AudioSamples {
    pub fn len(&self) -> usize {
        match self {
            Self::Pcm16(v) => v.len(),
            Self::Float32(v) => v.len(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn interleaved_i16(&self) -> Vec<i16> {
        match self {
            Self::Pcm16(v) => v.clone(),
            Self::Float32(v) => v
                .iter()
                .map(|&x| (x * 32768.0).round().clamp(-32768.0, 32767.0) as i16)
                .collect(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SampleLoop {
    pub start_frame: u64,
    /// Exclusive end, measured in sample frames rather than interleaved samples.
    pub end_frame: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SurroundComponent {
    /// A separately decoded component; the engine's pre-synthesis mixing is still unverified.
    pub channels: u16,
    pub samples: AudioSamples,
    pub relationship: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sound {
    pub name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: AudioSamples,
    pub loop_region: Option<SampleLoop>,
    /// Original bank resource identity, never a pointer or byte offset.
    pub resource_id: Option<u32>,
    pub surround: Option<SurroundComponent>,
    pub fidelity_notes: Vec<String>,
}
impl Sound {
    pub fn frame_count(&self) -> usize {
        self.samples.len() / usize::from(self.channels.max(1))
    }
    pub fn duration_seconds(&self) -> f64 {
        self.frame_count() as f64 / f64::from(self.sample_rate.max(1))
    }
    pub fn interleaved_i16(&self) -> Vec<i16> {
        self.samples.interleaved_i16()
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExternalSound {
    pub resource_id: u32,
    pub name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub loop_region: Option<SampleLoop>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundEvent {
    pub name: String,
    pub resource_id: Option<u32>,
    /// Named fields are kept until the sound-script grammar has been corroborated.
    pub attributes: Vec<(String, String)>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundBank {
    pub name: String,
    pub sounds: Vec<Sound>,
    pub external_sounds: Vec<ExternalSound>,
    pub events: Vec<SoundEvent>,
    pub fidelity_notes: Vec<String>,
    /// Source features whose content is not yet represented by this asset.
    pub reconstruction_gaps: Vec<AudioReconstructionGap>,
}

/// Named sound-library relationships from the editor's CSB/LCB/SIF sidecars.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundDefinitions {
    pub name: String,
    pub kind: SoundDefinitionKind,
    pub default_language: Option<String>,
    pub groups: Vec<SoundGroup>,
    pub banks: Vec<SoundBankSelection>,
    pub levels: Vec<LevelSoundSelection>,
    pub bindings: Vec<SoundDefinitionBinding>,
    pub events: Vec<SoundEvent>,
    pub resources: Vec<SoundResourceDefinition>,
    pub selectors: Vec<SoundSelector>,
    pub fidelity_notes: Vec<String>,
    /// Source features whose content is not yet represented by this asset.
    pub reconstruction_gaps: Vec<AudioReconstructionGap>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SoundDefinitionKind {
    Events,
    Library,
    Selectors,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundGroup {
    pub name: String,
    pub id: u32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundBankSelection {
    pub id: u32,
    pub events: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundResourceDefinition {
    pub id: u32,
    pub name: String,
    pub kind: String,
    pub file: Option<String>,
    pub volume: Option<f32>,
    pub looped: bool,
    pub loop_region: Option<SampleLoop>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub members: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundSelector {
    pub name: String,
    pub id: u32,
    pub values: Vec<(String, i32)>,
}

impl Sound {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            (1..=32).contains(&self.channels),
            "invalid audio channel count"
        );
        anyhow::ensure!(
            (1..=192000).contains(&self.sample_rate),
            "invalid audio sample rate"
        );
        anyhow::ensure!(
            self.samples
                .len()
                .is_multiple_of(usize::from(self.channels)),
            "incomplete interleaved sample frame"
        );
        if let AudioSamples::Float32(samples) = &self.samples {
            anyhow::ensure!(
                samples.iter().all(|v| v.is_finite()),
                "non-finite audio sample"
            );
        }
        if let Some(region) = &self.loop_region {
            anyhow::ensure!(
                region.start_frame < region.end_frame
                    && region.end_frame <= self.frame_count() as u64,
                "invalid audio loop"
            );
        }
        if let Some(component) = &self.surround {
            anyhow::ensure!(
                (1..=32).contains(&component.channels),
                "invalid surround channel count"
            );
            anyhow::ensure!(
                component.samples.len() % usize::from(component.channels) == 0,
                "incomplete surround frame"
            );
            anyhow::ensure!(
                component.samples.len() / usize::from(component.channels) == self.frame_count(),
                "surround duration differs from primary component"
            );
            if let AudioSamples::Float32(samples) = &component.samples {
                anyhow::ensure!(
                    samples.iter().all(|v| v.is_finite()),
                    "non-finite surround sample"
                );
            }
        }
        Ok(())
    }
}
impl SoundBank {
    pub fn validate(&self) -> anyhow::Result<()> {
        for sound in &self.sounds {
            sound.validate()?;
        }
        for external in &self.external_sounds {
            anyhow::ensure!(!external.name.is_empty(), "external sound has no file name");
            anyhow::ensure!(
                external.sample_rate > 0 && (1..=32).contains(&external.channels),
                "invalid external sound properties"
            );
        }
        Ok(())
    }
}
impl SoundDefinitions {
    pub fn validate(&self) -> anyhow::Result<()> {
        for resource in &self.resources {
            if let Some(volume) = resource.volume {
                anyhow::ensure!(volume.is_finite(), "non-finite sound volume");
            }
            if let Some(rate) = resource.sample_rate {
                anyhow::ensure!(rate > 0, "zero sample rate");
            }
            if let Some(channels) = resource.channels {
                anyhow::ensure!(
                    (1..=32).contains(&channels),
                    "invalid resource channel count"
                );
            }
        }
        for selector in &self.selectors {
            anyhow::ensure!(!selector.name.is_empty(), "empty sound selector name");
            let names: std::collections::BTreeSet<_> =
                selector.values.iter().map(|(name, _)| name).collect();
            anyhow::ensure!(
                names.len() == selector.values.len(),
                "duplicate sound selector value"
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LevelSoundSelection {
    pub level: String,
    pub events: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundDefinitionBinding {
    pub reference: String,
    pub bank: u32,
    pub member: u32,
    pub is_event: bool,
}

/// Explicit reconstruction blockers. Informational notes never control export support.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AudioReconstructionGap {
    BinaryEvents { count: u32 },
    ResourceType { resource_id: u32, kind: u32 },
    SoundScriptBlock { name: String },
    EventBehavior,
}

impl Content for Sound {
    const KIND: &'static str = "sound";
    fn validate(&self) -> Result<()> {
        Sound::validate(self)
    }
}

impl Content for SoundBank {
    const KIND: &'static str = "sound-bank";
    fn validate(&self) -> Result<()> {
        SoundBank::validate(self)
    }
}

impl Content for SoundDefinitions {
    const KIND: &'static str = "sound-definitions";
    fn validate(&self) -> Result<()> {
        SoundDefinitions::validate(self)
    }
}
