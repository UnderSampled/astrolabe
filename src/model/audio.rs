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
    pub operation: SoundOperation,
    /// Commands outside the represented operation retain their source spelling.
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
    pub data: SoundResourceData,
    pub file: Option<String>,
    pub volume: Option<f32>,
    pub looped: bool,
    pub loop_region: Option<SampleLoop>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
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
        use std::collections::BTreeSet;
        let event_ids: BTreeSet<_> = self.events.iter().filter_map(|e| e.resource_id).collect();
        anyhow::ensure!(
            event_ids.len()
                == self
                    .events
                    .iter()
                    .filter(|e| e.resource_id.is_some())
                    .count(),
            "duplicate sound event identity"
        );
        let resource_ids: BTreeSet<_> = self.resources.iter().map(|r| r.id).collect();
        anyhow::ensure!(
            resource_ids.len() == self.resources.len(),
            "duplicate sound resource identity"
        );
        let mut bindings = BTreeSet::new();
        for binding in &self.bindings {
            binding.target.member().validate()?;
            anyhow::ensure!(
                bindings.insert((
                    matches!(binding.target, SoundDefinitionTarget::Event(_)),
                    binding.bank,
                    binding.member
                )),
                "duplicate sound library binding"
            );
        }
        for event in &self.events {
            match &event.operation {
                SoundOperation::Play {
                    resource,
                    pitch_variation,
                    ..
                } => {
                    anyhow::ensure!(
                        pitch_variation.is_finite() && (0.0..=1.0).contains(pitch_variation),
                        "invalid random pitch range"
                    );
                    if let Some(resource) = resource {
                        resource.validate()?;
                    }
                }
                SoundOperation::Stop { event } => {
                    if let Some(event) = event {
                        event.validate()?;
                    }
                }
                SoundOperation::Other { .. } => {}
            }
        }
        for resource in &self.resources {
            for reference in resource.data.members() {
                reference.validate()?;
            }
            match &resource.data {
                SoundResourceData::Random {
                    silence_probability,
                    choices,
                    ..
                } => {
                    for p in std::iter::once(silence_probability)
                        .chain(choices.iter().map(|c| &c.probability))
                    {
                        anyhow::ensure!(
                            p.is_finite() && (0.0..=1.0).contains(p),
                            "invalid random sound probability"
                        );
                    }
                }
                SoundResourceData::Switch {
                    selector,
                    default,
                    choices,
                    ..
                } => {
                    anyhow::ensure!(
                        selector.definitions.path.member().is_none() && !selector.name.is_empty(),
                        "invalid sound selector owner/name"
                    );
                    let keys: BTreeSet<_> = choices.iter().map(|c| &c.value).collect();
                    anyhow::ensure!(
                        keys.len() == choices.len() && !default.is_empty(),
                        "duplicate switch value or empty default"
                    );
                }
                SoundResourceData::Sample {
                    sample: Some(sample),
                    ..
                } => {
                    let path = match sample {
                        SoundSampleRef::Sound(r) => &r.path,
                        SoundSampleRef::Bank { bank, .. } => &bank.path,
                    };
                    anyhow::ensure!(
                        path.member().is_none(),
                        "sample link must name its whole owner"
                    );
                }
                _ => {}
            }
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
    pub target: SoundDefinitionTarget,
    pub bank: u32,
    pub member: u32,
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

/// An event in a shared sound library. Bank/member are content identities, not handles.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundEventLink {
    pub library: super::AssetRef<SoundDefinitions>,
    pub bank: u16,
    pub member: u16,
}
impl SoundEventLink {
    pub fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            self.library.path.member().is_none(),
            "sound library must name a whole asset"
        );
        anyhow::ensure!(self.bank < 0xfffe, "reserved sound bank ID");
        Ok(())
    }
    pub fn binding<'a>(&self, library: &'a SoundDefinitions) -> Result<&'a SoundDefinitionBinding> {
        self.validate()?;
        library
            .bindings
            .iter()
            .find(|b| {
                matches!(b.target, SoundDefinitionTarget::Event(_))
                    && b.bank == u32::from(self.bank)
                    && b.member == u32::from(self.member)
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "missing sound event {}:{} in {}",
                    self.bank,
                    self.member,
                    self.library.path
                )
            })
    }
}

/// Whole-owner loading with explicit member lookup; never a generic member hydration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundMemberRef {
    pub definitions: super::AssetRef<SoundDefinitions>,
    pub id: u32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SoundDefinitionTarget {
    Event(SoundMemberRef),
    Resource(SoundMemberRef),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SoundOperation {
    Play {
        resource: Option<SoundMemberRef>,
        pitch_variation: f32,
        stoppable: bool,
        dynamic: bool,
    },
    Stop {
        /// Stop instances of this event for the emitting actor/bus.
        event: Option<SoundMemberRef>,
    },
    Other {
        name: String,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SoundSampleRef {
    Sound(super::AssetRef<Sound>),
    Bank {
        bank: super::AssetRef<SoundBank>,
        resource_id: u32,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundSampleControls {
    pub pitchable: bool,
    pub volumeable: bool,
    pub pannable: bool,
    pub spatial: bool,
    pub reverb: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SoundResourceData {
    Sample {
        sample: Option<SoundSampleRef>,
        controls: SoundSampleControls,
    },
    Random {
        silence_probability: f32,
        repeat_silence: bool,
        choices: Vec<RandomSoundChoice>,
    },
    Switch {
        selector: SoundSelectorRef,
        default: String,
        dynamic: bool,
        maintain: bool,
        choices: Vec<SwitchSoundChoice>,
    },
    Other {
        kind: String,
        members: Vec<SoundMemberRef>,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RandomSoundChoice {
    pub resource: SoundMemberRef,
    pub probability: f32,
    pub repeat: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundSelectorRef {
    pub definitions: super::AssetRef<SoundDefinitions>,
    pub name: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SwitchSoundChoice {
    pub resource: SoundMemberRef,
    pub value: String,
}
impl SoundDefinitionTarget {
    pub fn member(&self) -> &SoundMemberRef {
        match self {
            Self::Event(v) | Self::Resource(v) => v,
        }
    }
    pub fn member_mut(&mut self) -> &mut SoundMemberRef {
        match self {
            Self::Event(v) | Self::Resource(v) => v,
        }
    }
}
impl SoundResourceData {
    pub fn members(&self) -> Vec<&SoundMemberRef> {
        match self {
            Self::Sample { .. } => Vec::new(),
            Self::Random { choices, .. } => choices.iter().map(|v| &v.resource).collect(),
            Self::Switch { choices, .. } => choices.iter().map(|v| &v.resource).collect(),
            Self::Other { members, .. } => members.iter().collect(),
        }
    }
    pub fn members_mut(&mut self) -> Vec<&mut SoundMemberRef> {
        match self {
            Self::Sample { .. } => Vec::new(),
            Self::Random { choices, .. } => choices.iter_mut().map(|v| &mut v.resource).collect(),
            Self::Switch { choices, .. } => choices.iter_mut().map(|v| &mut v.resource).collect(),
            Self::Other { members, .. } => members.iter_mut().collect(),
        }
    }
}
impl SoundDefinitions {
    pub fn event(&self, id: u32) -> Result<&SoundEvent> {
        self.events
            .iter()
            .find(|v| v.resource_id == Some(id))
            .ok_or_else(|| anyhow::anyhow!("missing sound event {id} in {}", self.name))
    }
    pub fn resource(&self, id: u32) -> Result<&SoundResourceDefinition> {
        self.resources
            .iter()
            .find(|v| v.id == id)
            .ok_or_else(|| anyhow::anyhow!("missing sound resource {id} in {}", self.name))
    }
    pub fn resource_mut(&mut self, id: u32) -> Result<&mut SoundResourceDefinition> {
        self.resources
            .iter_mut()
            .find(|v| v.id == id)
            .ok_or_else(|| anyhow::anyhow!("missing sound resource {id} in {}", self.name))
    }
}

impl SoundMemberRef {
    pub fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            self.definitions.path.member().is_none(),
            "sound member link must name its whole owner"
        );
        Ok(())
    }
}
