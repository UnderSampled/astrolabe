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
    /// Read the active inventory list used by functions 201, 208 and 209 in
    /// this verified build. The list is capture-local; no source address is
    /// persisted as gameplay state.
    pub fn inventory(&self, memory: &mut impl ReadMemory) -> Result<InventoryObservation> {
        let status = self.status(memory)?;
        ensure!(
            status.phase == 9,
            "inventory requires loaded gameplay phase"
        );
        let mut source = MemoryView(memory);
        let mut entries = Vec::new();
        let mut visited = BTreeSet::new();
        let mut next = if let Some(root) = source.pointer(0x6107b0, 0)? {
            source.pointer(root, 8)?
        } else {
            None
        };
        while let Some(entry) = next {
            ensure!(visited.len() < 256, "inventory list exceeds 256 entries");
            ensure!(visited.insert(entry), "inventory list cycle");
            let handle = source.required(entry, 4)?;
            let handle_bytes = source.bytes(handle, 9)?;
            if handle_bytes[8] != 0 {
                let member = u32_at(&handle_bytes, 0)?;
                ensure!(member != 0, "active inventory handle has no member");
                let (name_pointer, id, quantity) = {
                    let record = source.bytes(member, 30)?;
                    (
                        u32_at(&record, 12)?,
                        i16::from_le_bytes(record[28..30].try_into()?),
                        i16::from_le_bytes(record[10..12].try_into()?),
                    )
                };
                ensure!(name_pointer != 0, "active inventory member has no kind");
                let mut name = Vec::new();
                for offset in 0..64 {
                    let address = source.add(name_pointer, offset)?;
                    let byte = source.bytes(address, 1)?[0];
                    if byte == 0 {
                        break;
                    }
                    ensure!(byte.is_ascii(), "inventory kind is not ASCII");
                    name.push(byte);
                }
                ensure!(name.len() < 64, "unterminated inventory kind");
                entries.push(InventoryItemObservation {
                    kind: String::from_utf8(name)?,
                    id,
                    quantity,
                });
            }
            next = source.pointer(entry, 20)?;
        }
        let selected_weapon = entries
            .iter()
            .find(|item| item.kind == "Weapon")
            .map_or(-1, |item| item.id);
        Ok(InventoryObservation {
            total_entries: visited.len(),
            active: entries,
            selected_weapon,
        })
    }
    /// Source function 232 selects one of three runtime magic-name rows, then
    /// returns the matching selected inventory item's signed info field.
    /// Preserve an unmatched row as None instead of inventing a spell code.
    pub fn magic_selection(
        &self,
        memory: &mut impl ReadMemory,
        inventory: &InventoryObservation,
    ) -> Result<MagicSelectionObservation> {
        ensure!(
            self.status(memory)?.phase == 9,
            "magic selection requires loaded gameplay phase"
        );
        let mut source = MemoryView(memory);
        let table = source.required(0x610b98, 0)?;
        let values = source.bytes(source.add(table, 0x24)?, 5)?;
        let current_points = u16::from_le_bytes(values[0..2].try_into()?);
        let maximum_points = u16::from_le_bytes(values[2..4].try_into()?);
        let selected_index = values[4];
        ensure!(
            selected_index <= 2,
            "magic selection outside three name rows"
        );
        let row = source.bytes(source.add(table, usize::from(selected_index) * 10)?, 10)?;
        let end = row
            .iter()
            .position(|value| *value == 0)
            .context("unterminated magic name")?;
        ensure!(end > 0 && row[..end].is_ascii(), "invalid magic name");
        let selected_name = String::from_utf8(row[..end].to_vec())?;
        let source_code = inventory
            .active
            .iter()
            .find(|item| item.kind == selected_name)
            .map(|item| item.id);
        Ok(MagicSelectionObservation {
            current_points,
            maximum_points,
            selected_index,
            selected_name,
            source_code,
        })
    }
    /// The verified build keeps the laser target point and presentation flag
    /// in one small global record. The target remains readable while hidden.
    pub fn laser_pointer(&self, memory: &mut impl ReadMemory) -> Result<LaserPointerObservation> {
        let status = self.status(memory)?;
        ensure!(
            status.phase == 9,
            "laser read requires loaded gameplay phase"
        );
        let mut source = MemoryView(memory);
        let bytes = source.bytes(0x74b310, 22)?;
        let mut point = [0.0; 3];
        for (i, value) in point.iter_mut().enumerate() {
            *value = f32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into()?);
            ensure!(value.is_finite(), "nonfinite laser target component");
        }
        let active = u16::from_le_bytes(bytes[20..22].try_into()?);
        ensure!(active <= 1, "laser presentation flag is not Boolean");
        Ok(LaserPointerObservation {
            point,
            active: active != 0,
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
        let status = self.status(memory)?;
        ensure!(
            status.phase == 9 && !status.level.is_empty(),
            "actor inspection requires a loaded gameplay phase"
        );
        let mut source = MemoryView(memory);
        let entry = source.required(0x756e70, 0)?;
        let node = source.required(entry, 4)?;
        actor_ai_at_node(&mut source, node, actions, designer_values)
    }

    /// Resolve a rooted actor by its source scene path within this capture.
    /// Paths and source type IDs are checked afresh; no live pointer is retained.
    pub fn scene_actor_with_ai(
        &self,
        memory: &mut impl ReadMemory,
        level: &str,
        child_path: &[u32],
        expected_object_types: [u32; 3],
        actions: &[u32],
        designer_values: &[u32],
    ) -> Result<ActorAiObservation> {
        let status = self.status(memory)?;
        ensure!(
            status.phase == 9 && status.level == level,
            "scene actor inspection requires the selected loaded level"
        );
        let mut source = MemoryView(memory);
        let node = scene_node(&mut source, child_path)?;
        let observation = actor_ai_at_node(&mut source, node, actions, designer_values)?;
        ensure!(
            observation.actor.object_types == expected_object_types,
            "scene actor source object types differ"
        );
        Ok(observation)
    }

    /// Identify the newest DynamicWorld children, where source object creation
    /// links generated Persos. Ordinals are capture-local, not asset IDs.
    pub fn scene_tail(
        &self,
        memory: &mut impl ReadMemory,
        limit: usize,
    ) -> Result<SceneTailObservation> {
        ensure!((1..=16).contains(&limit), "scene tail limit outside 1..16");
        let status = self.status(memory)?;
        ensure!(
            status.phase == 9,
            "scene tail requires loaded gameplay phase"
        );
        let mut source = MemoryView(memory);
        let root_address = source.required(0x75701c, 0)?;
        let root: SuperObjectWire = wire(&source.bytes(root_address, 64)?)?;
        ensure!(root.kind == 0, "DynamicWorld root is not a world node");
        ensure!(
            root.child_count <= 100_000,
            "scene child count exceeds bound"
        );
        let count = usize::try_from(root.child_count)?;
        if count == 0 {
            ensure!(
                root.first_child == 0 && root.last_child == 0,
                "empty scene root has child endpoints"
            );
        } else {
            ensure!(
                root.first_child != 0 && root.last_child != 0,
                "scene root lacks child endpoints"
            );
        }
        let mut address = root.last_child;
        let mut following = 0;
        let mut children = Vec::new();
        for index in (count.saturating_sub(limit)..count).rev() {
            ensure!(address != 0, "scene tail ended before child count");
            let child: SuperObjectWire = wire(&source.bytes(address, 64)?)?;
            ensure!(
                child.parent == root_address,
                "scene tail parent link differs"
            );
            ensure!(child.next == following, "scene tail next link differs");
            let (object_types, ai) = if child.kind == 4 {
                let perso = source.required(address, SuperObjectWire::DATA)?;
                let object_types = source_view::perso_parts(&mut source, perso)?.object_types;
                let ai = if source.pointer(perso, 16)?.is_none() {
                    SceneTailAi::BrainAbsent
                } else {
                    match source_view::ai_parts(&mut source, perso) {
                        Ok(parts) => SceneTailAi::Ready {
                            model_locator: format!("live:{:08x}", parts.model),
                        },
                        Err(error) => SceneTailAi::Unreadable {
                            reason: error.to_string(),
                        },
                    }
                };
                (Some(object_types), Some(ai))
            } else {
                (None, None)
            };
            let global_transform = if child.kind == 4 {
                let transform = source.required(address, 36)?;
                Some(source_view::transform(&mut source, transform)?)
            } else {
                None
            };
            children.push(SceneTailChild {
                index: u32::try_from(index)?,
                kind: child.kind,
                object_types,
                global_transform,
                ai,
            });
            following = address;
            address = child.previous;
        }
        if count <= limit {
            ensure!(
                address == 0 && following == root.first_child,
                "scene root first child link differs"
            );
        }
        children.reverse();
        Ok(SceneTailObservation {
            child_count: root.child_count,
            children,
        })
    }
}

fn actor_ai_at_node(
    source: &mut MemoryView<'_, impl ReadMemory>,
    node: u32,
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
    let actor = actor_at_node(source, node, actions, |a| format!("live:{a:08x}"))?;
    let perso = source.required(node, 4)?;
    let parts = source_view::perso_parts(source, perso)?;
    let action_members = source_view::list(source, parts.family, 16)?;
    let ai = source_view::ai_parts(source, perso)?;
    let normal_set = source_view::behavior_set(source, ai.model, 0)?;
    let reflex_set = source_view::behavior_set(source, ai.model, 4)?;
    let normal = source_view::behavior_selection(source, ai.mind, 4, normal_set.as_ref())?;
    let reflex = source_view::behavior_selection(source, ai.mind, 8, reflex_set.as_ref())?;
    let designer = source_view::designer_view(source, ai.model, ai.mind)?;
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
                source,
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
                        source_view::perso_parts(source, target)
                            .context("designer actor is not a readable perso")?;
                    }
                    ObservedDesignerValue::Actor(target.map(|target| format!("live:{target:08x}")))
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

fn scene_node(source: &mut MemoryView<'_, impl ReadMemory>, child_path: &[u32]) -> Result<u32> {
    ensure!(
        !child_path.is_empty() && child_path.len() <= 32,
        "scene child path depth outside 1..32"
    );
    // The verified Hype build stores ActualWorld here. Resolve it anew for
    // each capture; source child ordinals are not runtime addresses.
    let mut parent = source.required(0x75ec6c, 0)?;
    let root: SuperObjectWire = wire(&source.bytes(parent, 64)?)?;
    ensure!(root.kind == 0, "ActualWorld root is not a world node");
    let mut traversed = 0_u32;
    for &index in child_path {
        let header: SuperObjectWire = wire(&source.bytes(parent, 64)?)?;
        ensure!(
            header.child_count <= 100_000 && index < header.child_count,
            "scene child index outside bounded list"
        );
        traversed = traversed
            .checked_add(index + 1)
            .context("scene path overflow")?;
        ensure!(traversed <= 100_000, "scene path traversal limit");
        let mut child = source.required(parent, 8)?;
        let mut previous = None;
        for step in 0..=index {
            let item: SuperObjectWire = wire(&source.bytes(child, 64)?)?;
            ensure!(item.parent == parent, "scene child parent link differs");
            if let Some(previous) = previous {
                ensure!(item.previous == previous, "scene sibling link differs");
            }
            previous = Some(child);
            // The next pointer is only needed before the requested ordinal.
            // Its absence is caught by `required` on the next iteration.
            if step < index {
                child = source.required(child, 20)?;
            }
        }
        parent = child;
    }
    let terminal: SuperObjectWire = wire(&source.bytes(parent, 64)?)?;
    ensure!(terminal.kind == 4, "scene child path is not a perso");
    Ok(parent)
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryItemObservation {
    pub kind: String,
    pub id: i16,
    pub quantity: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryObservation {
    pub total_entries: usize,
    pub active: Vec<InventoryItemObservation>,
    /// Function 201 returns -1 when the active Weapon entry is absent.
    pub selected_weapon: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicSelectionObservation {
    pub current_points: u16,
    pub maximum_points: u16,
    pub selected_index: u8,
    pub selected_name: String,
    pub source_code: Option<i16>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaserPointerObservation {
    pub point: [f32; 3],
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneTailChild {
    pub index: u32,
    pub kind: u32,
    pub object_types: Option<[u32; 3]>,
    pub global_transform: Option<TransformValue>,
    pub ai: Option<SceneTailAi>,
}

/// A validated live model link is capture-local evidence. Resolve it against
/// source allocations before claiming any imported `ai/models/*` identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SceneTailAi {
    BrainAbsent,
    Ready { model_locator: String },
    Unreadable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneTailObservation {
    pub child_count: u32,
    pub children: Vec<SceneTailChild>,
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
    let _: MainActorWire = wire(&source.bytes(entry, 20)?)?;
    let node = source.required(entry, 4)?;
    actor_at_node(source, node, selected_actions, label)
}

fn actor_at_node<S: RecordSource>(
    source: &mut S,
    node: S::Location,
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
