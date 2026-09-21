//! Selected Hype inspection through relocated memory. OS access and pausing are
//! supplied by optional tooling; declarations, roots and meaning stay in OpenSpace.
//! Observations are transient projections, not a second persistent asset model.
use super::{
    container::{u32_at, wire},
    records::{FamilyWire, MainActorWire, SuperObjectWire},
    source_view::{self, RecordSource},
};
use crate::model::animation::TransformValue;
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::borrow::Cow;

pub const HYPE_EXECUTABLE_SHA256: &str =
    "048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da";
/// The controller validates readable mappings, process identity and its chosen
/// capture consistency. Implementations must reject missing/truncated ranges.
/// A completed-update contract is separate; this trait cannot establish one.
pub trait ReadMemory {
    fn read(&mut self, address: u32, size: usize) -> Result<Vec<u8>>;
}
struct MemoryView<'a, M>(&'a mut M);
impl<M: ReadMemory> RecordSource for MemoryView<'_, M> {
    type Location = u32;
    fn add(&self, address: u32, offset: usize) -> Result<u32> {
        address
            .checked_add(offset.try_into()?)
            .context("32-bit source address overflow")
    }
    fn bytes(&mut self, address: u32, size: usize) -> Result<Cow<'_, [u8]>> {
        ensure!(
            (1..=1048576).contains(&size),
            "invalid bounded memory read size"
        );
        self.add(address, size - 1)?;
        let bytes = self.0.read(address, size)?;
        ensure!(bytes.len() == size, "truncated memory source response");
        Ok(Cow::Owned(bytes))
    }
    fn pointer(&mut self, address: u32, offset: usize) -> Result<Option<u32>> {
        let site = self.add(address, offset)?;
        let pointer = u32_at(&self.bytes(site, 4)?, 0)?;
        Ok((pointer != 0).then_some(pointer))
    }
}

/// Constructed only after a full executable-file hash matches. This guard does
/// not by itself prove the live module mapping; the controller must check that.
pub struct HypeBuild(());
impl HypeBuild {
    pub fn verified(executable_sha256: &str) -> Result<Self> {
        ensure!(
            executable_sha256 == HYPE_EXECUTABLE_SHA256,
            "unrecognized Hype executable build"
        );
        Ok(Self(()))
    }
    pub fn status(&self, memory: &mut impl ReadMemory) -> Result<Status> {
        let mut source = MemoryView(memory);
        let bytes = source.bytes(0x754420, 261)?;
        let end = bytes[1..]
            .iter()
            .position(|b| *b == 0)
            .context("unterminated current level")?;
        ensure!(bytes[1..1 + end].is_ascii(), "non-ASCII current level");
        Ok(Status {
            phase: bytes[0],
            level: std::str::from_utf8(&bytes[1..1 + end])?.to_owned(),
        })
    }
    pub fn actor(&self, memory: &mut impl ReadMemory) -> Result<ActorObservation> {
        let status = self.status(memory)?;
        ensure!(
            status.phase == 9 && !status.level.is_empty(),
            "actor inspection requires a loaded gameplay phase"
        );
        let mut source = MemoryView(memory);
        // FUN_00443470 returns this global; FUN_00443460 reads its entry at +4.
        // File counterpart is GPT+0x2ae0. Do not infer other globals from a base.
        let entry = source.required(0x756e70, 0)?;
        actor(&mut source, entry, |a| format!("live:{a:08x}"))
    }
}
#[derive(Debug, Serialize)]
pub struct Status {
    pub phase: u8,
    pub level: String,
}
/// Source-local locators are evidence within this capture only. Equal values do
/// not merge locators; reuse of an address across captures requires generation
/// and replacement evidence from the controller, which is not supplied here.
#[derive(Debug, Serialize)]
pub struct ActorObservation {
    pub node: String,
    pub perso: String,
    pub data3d: String,
    pub family: String,
    pub object_types: [u32; 3],
    pub family_type: u32,
    pub action_members: Vec<String>,
    pub initial_action: Option<u32>,
    pub current_action: Option<u32>,
    pub secondary_action: Option<u32>,
    pub global_transform: TransformValue,
}
pub(super) fn actor<S: RecordSource>(
    source: &mut S,
    entry: S::Location,
    label: impl Fn(S::Location) -> String,
) -> Result<ActorObservation> {
    let _: MainActorWire = wire(&source.bytes(entry, 20)?)?;
    let node = source.required(entry, 4)?;
    let so: SuperObjectWire = wire(&source.bytes(node, 64)?)?;
    ensure!(so.kind == 4, "controlled node is not a perso");
    let perso = source.required(node, 4)?;
    let parts = source_view::perso_parts(source, perso)?;
    let data3d = parts.data3d;
    let object_types = parts.object_types;
    let family = parts.family;
    let header: FamilyWire = wire(&source.bytes(family, 56)?)?;
    ensure!(
        header.index == object_types[0],
        "actor object type and family index differ"
    );
    let members = source_view::list(source, family, 16)?;
    let initial_action = source_view::optional_member(source, data3d, 0, &members)?;
    let current_action = source_view::optional_member(source, data3d, 4, &members)?;
    let secondary_action = source_view::optional_member(source, data3d, 8, &members)?;
    let transform = source.required(node, 36)?;
    let global_transform = source_view::transform(source, transform)?;
    Ok(ActorObservation {
        node: label(node),
        perso: label(perso),
        data3d: label(data3d),
        family: label(family),
        object_types,
        family_type: header.index,
        action_members: members.into_iter().map(label).collect(),
        initial_action,
        current_action,
        secondary_action,
        global_transform,
    })
}
/// File-side diagnostic projection uses the same readers and native file
/// relocations, without importing a second document model or constructing live pointers.
pub fn inspect_file_actor(
    game: &std::path::Path,
    selection: &crate::model::Selection,
) -> Result<ActorObservation> {
    let session = super::session::SourceSession::open(game, selection)?;
    let root = session.root(0x2ae0)?;
    actor(&mut source_view::FileView(&session), root, |a| {
        format!(
            "file:{:?}:{:02x}:{:02x}+{:x}",
            a.owner, a.key.0, a.key.1, a.offset
        )
    })
}
