//! Provisional, complete physical-input recording shared by both engine adapters.
//! Ordinals count verified eligible consumers, never rendered frames or elapsed
//! milliseconds. This format is not frozen until both adapters pass acceptance.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const MAX_UPDATES: usize = 4096;
pub const MAX_BYTES: usize = 1024 * 1024;

/// Physical keyboard positions. An adapter translates these to its device codes;
/// it must not translate them into a gameplay action or change bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Key {
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    LeftShift,
    LeftControl,
    X,
    Z,
    Space,
    Enter,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Edge {
    Press,
    Release,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    /// Only the fixture's primary keyboard is supported by this initial profile.
    pub keyboard: u8,
    pub key: Key,
    pub edge: Edge,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Update {
    pub ordinal: u32,
    /// Ordered edges before this consumer. Empty entries explicitly preserve
    /// neutral/held/observation updates independently of controller delays.
    pub events: Vec<Event>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    pub name: String,
    pub completed_ordinal: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recording {
    pub schema: u32,
    pub name: String,
    /// Named Given fixture. A new setup/reset requires a separate recording;
    /// loading, pausing or losing the verified epoch aborts this initial profile.
    pub ready_state: String,
    pub initial_held: Vec<Key>,
    pub updates: Vec<Update>,
    pub checkpoints: Vec<Checkpoint>,
}
impl Recording {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.schema == 1, "unsupported provisional recording schema");
        ensure!(
            !self.name.is_empty()
                && self.name.len() <= 128
                && !self.ready_state.is_empty()
                && self.ready_state.len() <= 128,
            "recording and ready-state names must contain 1..128 bytes"
        );
        ensure!(
            self.initial_held.is_empty(),
            "initial profile requires a neutral ready state"
        );
        ensure!(
            (1..=MAX_UPDATES).contains(&self.updates.len()),
            "recording update capacity exceeded or empty"
        );
        let mut held = BTreeSet::new();
        for (index, update) in self.updates.iter().enumerate() {
            ensure!(
                update.ordinal as usize == index + 1,
                "missing, repeated or reordered input ordinal"
            );
            ensure!(
                update.events.len() <= 5,
                "too many keyboard events at one ordinal"
            );
            let mut changed = BTreeSet::new();
            for event in &update.events {
                ensure!(
                    event.keyboard == 0,
                    "only primary physical keyboard is supported"
                );
                ensure!(
                    changed.insert(event.key),
                    "a key may change once per eligible update"
                );
                match event.edge {
                    Edge::Press => ensure!(
                        held.insert(event.key),
                        "repeated press is unsupported; held state persists without repeated events"
                    ),
                    Edge::Release => {
                        ensure!(held.remove(&event.key), "release without a preceding press")
                    }
                }
            }
        }
        ensure!(
            held.is_empty(),
            "recording must explicitly release every held key"
        );
        ensure!(
            self.checkpoints.len() <= MAX_UPDATES,
            "checkpoint capacity exceeded"
        );
        let mut names = BTreeSet::new();
        for checkpoint in &self.checkpoints {
            ensure!(
                !checkpoint.name.is_empty()
                    && checkpoint.name.len() <= 128
                    && names.insert(&checkpoint.name),
                "checkpoint names must be unique and contain 1..128 bytes"
            );
            ensure!(
                (1..=self.updates.len()).contains(&(checkpoint.completed_ordinal as usize)),
                "checkpoint is outside recording"
            );
        }
        Ok(())
    }
}

/// Preserve and hash the exact bytes once. Adapters receive this same value;
/// neither may regenerate the recording from engine-specific step definitions.
pub struct PreparedRecording {
    recording: Recording,
    bytes: Vec<u8>,
    sha256: String,
}
impl PreparedRecording {
    pub fn compile(recording: Recording) -> Result<Self> {
        recording.validate()?;
        Self::read(serde_json::to_vec(&recording)?)
    }
    pub fn read(bytes: Vec<u8>) -> Result<Self> {
        ensure!(bytes.len() <= MAX_BYTES, "recording byte capacity exceeded");
        let recording: Recording =
            serde_json::from_slice(&bytes).context("invalid recording JSON")?;
        recording.validate()?;
        let sha256 = format!("{:x}", Sha256::digest(&bytes));
        Ok(Self {
            recording,
            bytes,
            sha256,
        })
    }
    pub fn recording(&self) -> &Recording {
        &self.recording
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

/// The existing source-rule boundary fixture expressed once for both adapters.
/// This is input/consumer component evidence, not full player initialization.
pub fn boundary_recording() -> Result<PreparedRecording> {
    let mut updates: Vec<_> = (1..=18)
        .map(|ordinal| Update {
            ordinal,
            events: vec![],
        })
        .collect();
    for (ordinal, key, edge) in [
        (3, Key::ArrowLeft, Edge::Press),
        (4, Key::ArrowLeft, Edge::Release),
        (5, Key::ArrowLeft, Edge::Press),
        (6, Key::ArrowLeft, Edge::Release),
        (8, Key::LeftShift, Edge::Press),
        (8, Key::ArrowLeft, Edge::Press),
        (13, Key::ArrowLeft, Edge::Release),
        (13, Key::LeftShift, Edge::Release),
    ] {
        updates[ordinal - 1].events.push(Event {
            keyboard: 0,
            key,
            edge,
        });
    }
    PreparedRecording::compile(Recording {
        schema: 1,
        name: "turn-pulses-and-chord".into(),
        ready_state: "manor-turn-boundary-component".into(),
        initial_held: vec![],
        updates,
        checkpoints: vec![
            Checkpoint {
                name: "first-pulse".into(),
                completed_ordinal: 3,
            },
            Checkpoint {
                name: "settled-release".into(),
                completed_ordinal: 18,
            },
        ],
    })
}
