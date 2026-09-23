//! Selected Hype inspection through relocated memory. OS access and pausing are
//! supplied by optional tooling; declarations, roots and meaning stay in OpenSpace.
//! Observations are transient projections, not a second persistent asset model.
use super::{
    container::{u32_at, wire},
    records::{FamilyWire, MainActorWire, PersoHeaderWire, SuperObjectWire},
    source_view::{self, RecordSource},
};
use crate::model::{
    AssetRef,
    ai::{BehaviorSelection, DesignerArray, DesignerValue},
    animation::TransformValue,
    character::{Family, Perso, StateRef},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{borrow::Cow, collections::BTreeSet};

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
        let bytes = source.bytes(super::hype_runtime::STATUS, 261)?;
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
        self.actor_with_actions(memory, &[47])
    }
    /// Inspect only the explicitly requested invariant action definitions. The
    /// family list is still traversed to preserve ordering and resolve links.
    pub fn actor_with_actions(
        &self,
        memory: &mut impl ReadMemory,
        actions: &[u32],
    ) -> Result<ActorObservation> {
        let status = self.status(memory)?;
        ensure!(
            status.phase == 9 && !status.level.is_empty(),
            "actor inspection requires a loaded gameplay phase"
        );
        let mut source = MemoryView(memory);
        // FUN_00443470 returns this global; FUN_00443460 reads its entry at +4.
        // File counterpart is GPT+0x2ae0. Do not infer other globals from a base.
        let entry = source.required(0x756e70, 0)?;
        actor(&mut source, entry, actions, |a| format!("live:{a:08x}"))
    }

    /// Inspect the selected actor plus its live behavior selections and only
    /// the requested designer values. All roots are resolved from the verified
    /// build; callers never supply process addresses. Returned labels are
    /// capture-local evidence and must not be persisted as asset identities.
    pub fn actor_with_ai(
        &self,
        memory: &mut impl ReadMemory,
        actions: &[u32],
        designer_values: &[u32],
    ) -> Result<ActorAiObservation> {
        ensure!(
            designer_values
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len()
                == designer_values.len(),
            "duplicate selected designer value"
        );
        let status = self.status(memory)?;
        ensure!(
            status.phase == 9 && !status.level.is_empty(),
            "actor inspection requires a loaded gameplay phase"
        );
        let mut source = MemoryView(memory);
        let entry = source.required(0x756e70, 0)?;
        let actor = actor(&mut source, entry, actions, |a| format!("live:{a:08x}"))?;
        let node = source.required(entry, 4)?;
        let perso = source.required(node, 4)?;
        let parts = source_view::perso_parts(&mut source, perso)?;
        let action_members = source_view::list(&mut source, parts.family, 16)?;
        let ai = source_view::ai_parts(&mut source, perso)?;
        let normal_set = source_view::behavior_set(&mut source, ai.model, 0)?;
        let reflex_set = source_view::behavior_set(&mut source, ai.model, 4)?;
        let normal = source_view::behavior_selection(&mut source, ai.mind, 4, normal_set.as_ref())?;
        let reflex = source_view::behavior_selection(&mut source, ai.mind, 8, reflex_set.as_ref())?;
        let designer = source_view::designer_view(&mut source, ai.model, ai.mind)?;
        ensure!(
            designer.is_some() || designer_values.is_empty(),
            "actor AI has no designer values"
        );
        let mut selected_designer_values = Vec::with_capacity(designer_values.len());
        if let Some(designer) = designer {
            let family = AssetRef::new("level:/families/__live_observation__")?;
            let actor_ref = AssetRef::new("level:/persos/__live_observation__")?;
            for &index in designer_values {
                let info = *designer
                    .fields
                    .get(index as usize)
                    .context("selected designer value outside definition")?;
                let selected_actor = std::cell::Cell::new(None);
                let (value, _) = source_view::designer_value(
                    &mut source,
                    designer.values,
                    info,
                    designer.length,
                    |target| {
                        target
                            .map(|target| {
                                action_members
                                    .iter()
                                    .position(|member| *member == target)
                                    .map(|state| StateRef {
                                        family: family.clone(),
                                        state: state as u32,
                                    })
                                    .context("designer action outside selected actor family")
                            })
                            .transpose()
                    },
                    |target| {
                        selected_actor.set(target);
                        Ok(target.map(|_| actor_ref.clone()))
                    },
                )?;
                let value = match value {
                    DesignerValue::Action(value) => {
                        ObservedDesignerValue::Action(value.map(|value| value.state))
                    }
                    DesignerValue::ActionArray(value) => {
                        ObservedDesignerValue::ActionArray(DesignerArray {
                            values: value
                                .values
                                .into_iter()
                                .map(|value| value.map(|value| value.state))
                                .collect(),
                            unknown: value.unknown,
                            element_unknown: value.element_unknown,
                        })
                    }
                    DesignerValue::Actor(value) => {
                        let target = selected_actor.get();
                        ensure!(
                            value.is_some() == target.is_some(),
                            "designer actor binding differs"
                        );
                        if let Some(target) = target {
                            let _: PersoHeaderWire = wire(&source.bytes(target, 20)?)?;
                            source_view::perso_parts(&mut source, target)
                                .context("designer actor is not a readable perso")?;
                        }
                        ObservedDesignerValue::Actor(
                            target.map(|target| format!("live:{target:08x}")),
                        )
                    }
                    value => ObservedDesignerValue::Value(value),
                };
                selected_designer_values.push(SelectedDesignerValue {
                    index,
                    kind: info.kind,
                    value,
                });
            }
        }
        Ok(ActorAiObservation {
            actor,
            normal,
            reflex,
            designer_values: selected_designer_values,
        })
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Status {
    pub phase: u8,
    pub level: String,
}

/// Immutable provenance for exactly the level source files consumed by the
/// selected `SourceSession`. This is report evidence, never a Rete asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFingerprint {
    pub profile: String,
    pub level: String,
    pub locale: String,
    pub files: Vec<SourceFileFingerprint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFileFingerprint {
    pub relative_path: String,
    pub sha256: String,
    pub byte_count: u64,
}

pub fn source_fingerprint(
    game: &std::path::Path,
    selection: &crate::model::Selection,
) -> Result<SourceFingerprint> {
    // Opening first proves these are the exact parseable inputs selected by the
    // source adapter, rather than a parallel list that merely happens to exist.
    let (session, files) =
        super::session::SourceSession::open_with_fingerprint_files(game, selection)?;
    drop(session);
    let files = files
        .entries()
        .map(|(relative, absolute)| {
            let bytes = std::fs::read(absolute)
                .with_context(|| format!("fingerprint {}", absolute.display()))?;
            Ok(SourceFileFingerprint {
                relative_path: relative
                    .to_str()
                    .context("non-UTF-8 source relative path")?
                    .replace(std::path::MAIN_SEPARATOR, "/"),
                sha256: format!("{:x}", Sha256::digest(&bytes)),
                byte_count: bytes.len().try_into()?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(SourceFingerprint {
        profile: selection.profile.clone(),
        level: selection.level.clone(),
        locale: selection.locale.clone(),
        files,
    })
}
/// Source-local locators are evidence within this capture only. Equal values do
/// not merge locators; reuse of an address across captures requires generation
/// and replacement evidence from the controller, which is not supplied here.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorObservation {
    pub node: String,
    pub perso: String,
    pub data3d: String,
    pub family: String,
    pub object_types: [u32; 3],
    pub family_type: u32,
    pub action_members: Vec<String>,
    pub action_definitions: Vec<ActionDefinitionProjection>,
    pub initial_action: Option<u32>,
    pub current_action: Option<u32>,
    pub secondary_action: Option<u32>,
    pub global_transform: TransformValue,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorAiObservation {
    pub actor: ActorObservation,
    pub normal: Option<BehaviorSelection>,
    pub reflex: Option<BehaviorSelection>,
    pub designer_values: Vec<SelectedDesignerValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedDesignerValue {
    pub index: u32,
    pub kind: u32,
    pub value: ObservedDesignerValue,
}

/// Reference-bearing values use family member ids or capture-local actor labels;
/// every other kind is exactly the useful model value decoded by the shared codec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObservedDesignerValue {
    Value(DesignerValue),
    Action(Option<u32>),
    ActionArray(DesignerArray<Option<u32>>),
    Actor(Option<String>),
}

/// Cross-adapter meaning for the selected controlled actor and its resolved
/// family. Source locators and runtime generations remain observation evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorProjection {
    pub role: ActorRole,
    pub object_types: [u32; 3],
    pub family: FamilyProjection,
    /// File initialization or named-fixture state rather than definition data.
    pub selections: ActorSelections,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorRole {
    MainActor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FamilyProjection {
    pub object_type: u32,
    pub action_count: u32,
    /// Requested definitions only; other family actions remain unselected.
    pub selected_actions: Vec<ActionDefinitionProjection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionDefinitionProjection {
    pub id: u32,
    pub speed: u8,
    pub repeats: u8,
    pub transition_lock: u8,
    pub next: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorSelections {
    pub initial: Option<u32>,
    pub current: Option<u32>,
    pub secondary: Option<u32>,
}

impl ActorObservation {
    pub fn projection(&self) -> ActorProjection {
        ActorProjection {
            role: ActorRole::MainActor,
            object_types: self.object_types,
            family: FamilyProjection {
                object_type: self.family_type,
                action_count: self.action_members.len() as u32,
                selected_actions: self.action_definitions.clone(),
            },
            selections: ActorSelections {
                initial: self.initial_action,
                current: self.current_action,
                secondary: self.secondary_action,
            },
        }
    }
}

impl ActorProjection {
    /// The caller supplies the family resolved by `actor.family`; this verifies
    /// that relationship instead of joining by an address, ordinal or value hash.
    pub fn from_model(actor: &Perso, family: &Family, actions: &[u32]) -> Result<Self> {
        ensure!(
            actions.iter().copied().collect::<BTreeSet<_>>().len() == actions.len(),
            "duplicate selected action"
        );
        ensure!(
            actor.family.path == family.path,
            "actor family reference does not resolve to supplied family"
        );
        ensure!(
            family.object_type == actor.object_types[0],
            "actor object type and family index differ"
        );
        ensure!(
            family
                .states
                .iter()
                .enumerate()
                .all(|(index, state)| state.id == index as u32),
            "noncanonical family state order"
        );
        let selected_actions = actions
            .iter()
            .map(|&id| {
                let state = family
                    .states
                    .get(id as usize)
                    .context("selected action outside family")?;
                Ok(ActionDefinitionProjection {
                    id: state.id,
                    speed: state.speed,
                    repeats: state.repeats,
                    transition_lock: state.transition_lock,
                    next: state.next,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            role: ActorRole::MainActor,
            object_types: actor.object_types,
            family: FamilyProjection {
                object_type: family.object_type,
                action_count: family.states.len().try_into()?,
                selected_actions,
            },
            selections: ActorSelections {
                initial: actor.initial_state,
                current: actor.current_state,
                secondary: actor.secondary_state,
            },
        })
    }
}
pub(super) fn actor<S: RecordSource>(
    source: &mut S,
    entry: S::Location,
    selected_actions: &[u32],
    label: impl Fn(S::Location) -> String,
) -> Result<ActorObservation> {
    ensure!(
        selected_actions
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .len()
            == selected_actions.len(),
        "duplicate selected action"
    );
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
    let action_definitions = selected_actions
        .iter()
        .map(|&id| {
            let member = *members
                .get(id as usize)
                .context("selected action outside family")?;
            let state = source_view::state_definition(source, member, &members)?;
            Ok(ActionDefinitionProjection {
                id,
                speed: state.speed,
                repeats: state.repeats,
                transition_lock: state.transition_lock,
                next: state.next,
            })
        })
        .collect::<Result<Vec<_>>>()?;
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
        action_members: members.iter().copied().map(&label).collect(),
        action_definitions,
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
    actor(&mut source_view::FileView(&session), root, &[47], |a| {
        format!(
            "file:{:?}:{:02x}:{:02x}+{:x}",
            a.owner, a.key.0, a.key.1, a.offset
        )
    })
}
