//! Session-bound AI model/instance import with preserved unsupported script slots.
#[cfg(test)]
#[path = "hype_control_probe.rs"]
mod control_tests;
#[cfg(test)]
#[path = "ai_probe.rs"]
mod tests;
#[cfg(test)]
#[path = "hype_turn_probe.rs"]
mod turn_tests;
use super::{
    container::{u32_at, wire},
    records::*,
    scene::SceneImport,
    session::SourceSession,
    sound::SoundSites,
    source_view,
};
use crate::{
    model::{
        AssetPath, AssetRef, Selection,
        ai::*,
        character::{Perso, StateRef},
        io::{Content, Diagnostic, Severity},
    },
    openspace::{
        ai::{AiScriptCodec, NodeWire},
        ai_values::DesignerValueCodec,
        codec::FormatCodec,
    },
};
use anyhow::{Context, Result, ensure};
use binrw::{BinRead, BinWrite};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(BinRead, BinWrite)]
#[brw(little)]
struct BehaviorWire {
    rules: u32,
    schedule: u32,
    count: u8,
    unknown: [u8; 3],
}

/// Verified PC IPT entry: 004a5ad0 uses 28-byte entries, names at +8.
#[derive(BinRead)]
#[br(little)]
struct InputEntryWire {
    _keyword_count: u32,
    _keywords: u32,
    _name: u32,
    _unknown: u32,
    _held_frames: i32,
    _unknown_20: u32,
    _active: u8,
    _tail: [u8; 3],
}
fn source_string(s: &SourceSession, site: Address) -> Result<String> {
    let target = s.required(site, 0)?;
    let bytes = &s.block(target)?.data[target.offset..];
    let end = bytes
        .iter()
        .take(256)
        .position(|&b| b == 0)
        .context("unterminated AI string")?;
    ensure!(
        end > 0 && bytes[..end].is_ascii(),
        "unsupported AI string encoding"
    );
    Ok(std::str::from_utf8(&bytes[..end])?.to_owned())
}
fn input_action(s: &SourceSession, site: Address) -> Result<String> {
    let entry = s.required(site, 0)?;
    let _: InputEntryWire = wire(s.bytes(entry, 28)?)?;
    let name = s.required(entry, 8)?;
    let bytes = &s.block(name)?.data[name.offset..];
    let end = bytes
        .iter()
        .take(256)
        .position(|&b| b == 0)
        .context("unterminated input action name")?;
    ensure!(
        end > 0 && bytes[..end].is_ascii(),
        "invalid logical input action name"
    );
    Ok(std::str::from_utf8(&bytes[..end])?.to_owned())
}
fn state_reference(
    s: &SourceSession,
    states: &BTreeMap<Address, StateRef>,
    site: Address,
) -> Result<Option<StateRef>> {
    if matches!(u32_at(s.bytes(site, 4)?, 0)?, 0 | u32::MAX) {
        return Ok(None);
    }
    let target = s.required(site, 0)?;
    Ok(Some(
        states
            .get(&target)
            .context("action outside imported families")?
            .clone(),
    ))
}

fn add(a: Address, n: usize) -> Address {
    Address {
        offset: a.offset + n,
        ..a
    }
}
fn path(s: &SourceSession, a: Address) -> Result<AssetPath> {
    AssetPath::new(format!(
        "{}:/ai/models/{}",
        if a.owner == Owner::Fix {
            "fix"
        } else {
            "level"
        },
        s.allocation_index(a)?
    ))
}
fn allocated(s: &SourceSession, a: Address, n: usize) -> Result<&[u8]> {
    s.allocation_index(a)?;
    ensure!(
        u32_at(&s.block(a)?.data, a.offset - 4)? as usize * 4 - 4 == n,
        "AI allocation size differs for {a:?}"
    );
    s.bytes(a, n)
}
struct SourceScript {
    address: Address,
    bytes: Vec<u8>,
}
struct SourceBehavior {
    address: Address,
    rules: Vec<Option<SourceScript>>,
    schedule: Option<SourceScript>,
    unknown: [u8; 3],
}
struct SourceSet {
    address: Address,
    behaviors: Vec<SourceBehavior>,
    default: Option<u32>,
    capacity: u8,
    unknown: [u8; 3],
}
fn member(s: &SourceSession, site: Address, field: usize, set: &SourceSet) -> Result<Option<u32>> {
    s.pointer(site, field)?
        .map(|a| {
            set.behaviors
                .iter()
                .position(|b| b.address == a)
                .map(|n| n as u32)
                .context("AI behavior target outside its owning set")
        })
        .transpose()
}
fn source_script(s: &SourceSession, slot: Address) -> Result<Option<SourceScript>> {
    let Some(a) = s.pointer(slot, 0)? else {
        return Ok(None);
    };
    s.allocation_index(a)?;
    let length = u32_at(&s.block(a)?.data, a.offset - 4)? as usize * 4 - 4;
    ensure!(
        length > 0 && length.is_multiple_of(8) && length / 8 <= 65535,
        "invalid AI script allocation"
    );
    let bytes = s.bytes(a, length)?.to_vec();
    for (i, bytes) in bytes.as_chunks::<8>().0.iter().enumerate() {
        let n: NodeWire = wire(bytes)?;
        if i + 1 == length / 8 {
            ensure!(
                n.kind == 8 && n.indent == 0 && n.parameter == 0,
                "invalid AI sentinel"
            );
        } else {
            ensure!(
                n.kind != 8 && n.indent > 0 && n.indent <= 64,
                "invalid AI node"
            );
        }
    }
    Ok(Some(SourceScript { address: a, bytes }))
}
fn source_set(s: &SourceSession, model: Address, field: usize) -> Result<Option<SourceSet>> {
    let Some(a) = s.pointer(model, field)? else {
        return Ok(None);
    };
    let h: SetWire = wire(allocated(s, a, 16)?)?;
    ensure!(h.count <= 65535, "too many AI behaviors");
    let mut behaviors = Vec::new();
    if h.count > 0 {
        let entries = s.required(a, 0)?;
        allocated(s, entries, h.count as usize * 12)?;
        for i in 0..h.count as usize {
            let address = add(entries, i * 12);
            let b: BehaviorWire = wire(s.bytes(address, 12)?)?;
            let mut rules = Vec::new();
            if b.count > 0 {
                let slots = s.required(address, 0)?;
                allocated(s, slots, b.count as usize * 4)?;
                for n in 0..b.count as usize {
                    rules.push(source_script(s, add(slots, n * 4))?);
                }
            }
            let schedule = s
                .pointer(address, 4)?
                .map(|slot| source_script(s, slot))
                .transpose()?
                .flatten();
            behaviors.push(SourceBehavior {
                address,
                rules,
                schedule,
                unknown: b.unknown,
            });
        }
    }
    let mut set = SourceSet {
        address: a,
        behaviors,
        default: None,
        capacity: h.capacity,
        unknown: h.unknown,
    };
    set.default = member(s, a, 8, &set)?;
    Ok(Some(set))
}

/// Find Perso definitions referenced by object-producing AI actions before
/// character import is finalized. A template need not have a rooted scene node;
/// it still owns a family, configuration and possibly a Brain. This scan reads
/// only the typed operand position of the known Hype meta-action, so unrelated
/// relocated words cannot become guessed Persos.
pub(super) fn object_templates(
    s: &SourceSession,
    rooted: &BTreeSet<Address>,
) -> Result<Vec<Address>> {
    let mut pending: Vec<_> = rooted.iter().copied().collect();
    let mut seen_persos = BTreeSet::new();
    let mut seen_models = BTreeSet::new();
    let mut templates = Vec::new();
    while let Some(perso) = pending.pop() {
        if !seen_persos.insert(perso) {
            continue;
        }
        ensure!(
            seen_persos.len() <= 2048,
            "AI object template graph is too large"
        );
        let header: PersoHeaderWire = wire(s.bytes(perso, 20)?)?;
        if header.brain == 0 {
            continue;
        }
        let Some(brain) = s.pointer(perso, 16)? else {
            continue;
        };
        let mind = s.required(brain, 0)?;
        let model = s.required(mind, 0)?;
        if !seen_models.insert(model) {
            continue;
        }
        for set in [source_set(s, model, 0)?, source_set(s, model, 4)?]
            .into_iter()
            .flatten()
        {
            for behavior in set.behaviors {
                for script in behavior
                    .rules
                    .iter()
                    .flatten()
                    .chain(behavior.schedule.iter())
                {
                    let nodes: Vec<NodeWire> = script
                        .bytes
                        .as_chunks::<8>()
                        .0
                        .iter()
                        .map(|bytes| wire(bytes))
                        .collect::<Result<_>>()?;
                    for (i, pair) in nodes.windows(2).enumerate() {
                        let [action, operand] = pair else {
                            unreachable!()
                        };
                        if action.kind != 5
                            || action.parameter != 15
                            || operand.kind != 24
                            || operand.indent != action.indent + 1
                        {
                            continue;
                        }
                        let Some(target) = s.pointer(add(script.address, (i + 1) * 8), 0)? else {
                            continue;
                        };
                        let _: PersoHeaderWire = wire(s.bytes(target, 20)?)?;
                        source_view::perso_parts(&mut source_view::FileView(s), target)?;
                        if !rooted.contains(&target) && !templates.contains(&target) {
                            templates.push(target);
                        }
                        pending.push(target);
                    }
                }
            }
        }
    }
    Ok(templates)
}
fn behavior_bindings(
    path: &AssetPath,
    sets: &[Option<SourceSet>; 2],
) -> Result<BTreeMap<Address, BehaviorRef>> {
    let mut bindings = BTreeMap::new();
    for (group, set) in [BehaviorGroup::Normal, BehaviorGroup::Reflex]
        .into_iter()
        .zip(sets)
    {
        for (index, behavior) in set.iter().flat_map(|set| &set.behaviors).enumerate() {
            let reference = BehaviorRef {
                model: AssetRef::from_path(path.clone()),
                group,
                behavior: index as u32,
            };
            ensure!(
                bindings.insert(behavior.address, reference).is_none(),
                "duplicate source AI behavior address {:?} within {}",
                behavior.address,
                path
            );
        }
    }
    Ok(bindings)
}
/// Category35 is the original evaluator's ObjectTableRef literal, not a
/// guessed pointer inside another expression. Import these before finishing
/// character geometry/material dependencies, including lists outside Family.
pub(super) fn configuration_sources(
    s: &SourceSession,
    actors: &BTreeMap<Address, AssetRef<Perso>>,
) -> Result<Vec<Address>> {
    ensure!(
        actors.len() <= 2048,
        "AI configuration actor graph is too large"
    );
    let mut models = BTreeSet::new();
    let mut configurations = BTreeSet::new();
    for &actor in actors.keys() {
        let header: PersoHeaderWire = wire(s.bytes(actor, 20)?)?;
        if header.brain == 0 {
            continue;
        }
        let Some(brain) = s.pointer(actor, 16)? else {
            continue;
        };
        let model = s.required(s.required(brain, 0)?, 0)?;
        if !models.insert(model) {
            continue;
        }
        for set in [source_set(s, model, 0)?, source_set(s, model, 4)?]
            .into_iter()
            .flatten()
        {
            for behavior in set.behaviors {
                for script in behavior
                    .rules
                    .iter()
                    .flatten()
                    .chain(behavior.schedule.iter())
                {
                    for (index, bytes) in script.bytes.as_chunks::<8>().0.iter().enumerate() {
                        let node: NodeWire = wire(bytes)?;
                        if node.kind != 35 {
                            continue;
                        }
                        let Some(target) = s.pointer(add(script.address, index * 8), 0)? else {
                            continue;
                        };
                        s.allocation_index(target)?;
                        let _: ObjectListWire = wire(s.bytes(target, 24)?)?;
                        configurations.insert(target);
                        ensure!(
                            configurations.len() <= 65536,
                            "AI configuration graph is too large"
                        );
                    }
                }
            }
        }
    }
    Ok(configurations.into_iter().collect())
}
fn preserve_script(
    s: &SourceSession,
    sounds: &SoundSites,
    states: &BTreeMap<Address, StateRef>,
    behaviors: &BTreeMap<Address, BehaviorRef>,
    script: &SourceScript,
) -> Result<Vec<PreservedAiNode>> {
    let mut nodes = Vec::with_capacity(script.bytes.len() / 8);
    for (i, bytes) in script.bytes.as_chunks::<8>().0.iter().enumerate() {
        let n: NodeWire = wire(bytes)?;
        let site = add(script.address, i * 8);
        let operand = (|| -> Result<PreservedAiOperand> {
            Ok(match n.kind {
                14 if matches!(n.parameter, 0 | u32::MAX) => PreservedAiOperand::Source(None),
                14 => match s.pointer(site, 0)? {
                    Some(_) => PreservedAiOperand::Input(input_action(s, site)?),
                    None => PreservedAiOperand::Source(None),
                },
                20 => PreservedAiOperand::String(source_string(s, site)?),
                25 => PreservedAiOperand::Action(state_reference(s, states, site)?),
                32 if matches!(n.parameter, 0 | u32::MAX) => PreservedAiOperand::Source(None),
                32 => match s.pointer(site, 0)? {
                    None => PreservedAiOperand::Source(None),
                    Some(target) => {
                        if let Some(reference) = behaviors.get(&target) {
                            PreservedAiOperand::Behavior(reference.clone())
                        } else {
                            match s.source_identity(target) {
                                Ok(identity) => PreservedAiOperand::Source(Some(identity)),
                                Err(_) => PreservedAiOperand::Incomplete(
                                    "external behavior operand lacks a typed member identity"
                                        .into(),
                                ),
                            }
                        }
                    }
                },
                34 => PreservedAiOperand::Sound(
                    sounds
                        .events
                        .get(&site)
                        .context("AI sound operand has no localized SND binding")?
                        .clone(),
                ),
                _ if s.relocated(site)? => {
                    let target = s.pointer(site, 0)?;
                    match target {
                        None => PreservedAiOperand::Source(None),
                        Some(target) => match s.source_identity(target) {
                            Ok(identity) => PreservedAiOperand::Source(Some(identity)),
                            Err(_) => PreservedAiOperand::Incomplete(
                                "relocated interior source reference lacks a typed member identity"
                                    .into(),
                            ),
                        },
                    }
                }
                _ if s.resembles_source_address(n.parameter) => PreservedAiOperand::Incomplete(
                    "unrelocated word resembles a source virtual address".into(),
                ),
                _ => PreservedAiOperand::NonRelocated(bytes[..4].try_into()?),
            })
        })()
        .with_context(|| format!("preserving AI node {i} category {}", n.kind))?;
        nodes.push(PreservedAiNode {
            category: n.kind,
            indent: n.indent,
            reserved: n.reserved,
            operand,
        });
    }
    Ok(nodes)
}

fn decode_script(
    s: &SourceSession,
    sounds: &SoundSites,
    states: &BTreeMap<Address, StateRef>,
    actors: &BTreeMap<Address, AssetRef<Perso>>,
    waypoints: &BTreeMap<Address, AssetRef<crate::model::route::Waypoint>>,
    behaviors: &BTreeMap<Address, BehaviorRef>,
    script: &SourceScript,
) -> Result<Script> {
    let mut codec = AiScriptCodec {
        preserved_nodes: preserve_script(s, sounds, states, behaviors, script)?,
        ..Default::default()
    };
    for (i, bytes) in script.bytes.as_chunks::<8>().0.iter().enumerate() {
        let n: NodeWire = wire(bytes)?;
        if n.kind == 14
            && !matches!(n.parameter, 0 | u32::MAX)
            && s.pointer(add(script.address, i * 8), 0)?.is_some()
        {
            codec
                .buttons
                .insert(i, input_action(s, add(script.address, i * 8))?);
        }
        if n.kind == 20 {
            codec
                .strings
                .insert(i, source_string(s, add(script.address, i * 8))?);
        }
        if n.kind == 25 {
            codec
                .actions
                .insert(i, state_reference(s, states, add(script.address, i * 8))?);
        }
        if n.kind == 24
            && let Some(target) = s.pointer(add(script.address, i * 8), 0)?
            && let Some(actor) = actors.get(&target)
        {
            codec.actors.insert(i, actor.clone());
        }
        if n.kind == 35 {
            let configuration = s
                .pointer(add(script.address, i * 8), 0)?
                .map(|target| -> Result<_> {
                    AssetRef::new(format!(
                        "{}:/configurations/{}",
                        if target.owner == Owner::Fix {
                            "fix"
                        } else {
                            "level"
                        },
                        s.allocation_index(target)?
                    ))
                })
                .transpose()?;
            codec.configurations.insert(i, configuration);
        }
        if n.kind == 29
            && let Some(target) = s.pointer(add(script.address, i * 8), 0)?
            && let Some(point) = waypoints.get(&target)
        {
            codec.waypoints.insert(i, point.clone());
        }
        if n.kind == 32
            && !matches!(n.parameter, 0 | u32::MAX)
            && let Some(target) = s.pointer(add(script.address, i * 8), 0)?
            && let Some(reference) = behaviors.get(&target)
        {
            codec.behaviors.insert(i, reference.clone());
        }
        if n.kind == 34 {
            let event = sounds
                .events
                .get(&add(script.address, i * 8))
                .context("AI sound operand has no localized SND binding")?;
            codec.sounds.insert(i, event.clone());
        }
        if n.kind == 37
            && let Some(target) = s.pointer(add(script.address, i * 8), 0)?
        {
            codec
                .particle_generators
                .insert(i, super::particle::reference(s, target)?);
        }
    }
    match codec.read(&script.bytes) {
        Ok(script) => Ok(script),
        Err(error) => {
            let mut nodes = codec.preserved_nodes;
            ensure!(
                nodes.last().is_some_and(|node| node.category == 8),
                "failed to preserve rejected source AI script after typed decode error: {error:#}"
            );
            nodes.pop();
            ensure!(
                !nodes.is_empty(),
                "failed source AI script has no executable nodes"
            );
            Ok(Script {
                statements: vec![Statement::Unsupported(UnsupportedFragment {
                    reason: format!("source AI script could not be typed: {error:#}"),
                    start: 0,
                    nodes,
                })],
            })
        }
    }
}
fn decode_set(
    s: &SourceSession,
    sounds: &SoundSites,
    states: &BTreeMap<Address, StateRef>,
    actors: &BTreeMap<Address, AssetRef<Perso>>,
    waypoints: &BTreeMap<Address, AssetRef<crate::model::route::Waypoint>>,
    behaviors: &BTreeMap<Address, BehaviorRef>,
    set: &SourceSet,
) -> Result<BehaviorSet> {
    Ok(BehaviorSet {
        behaviors: set
            .behaviors
            .iter()
            .enumerate()
            .map(|(id, b)| {
                Ok(Behavior {
                    id: id as u32,
                    rules: b
                        .rules
                        .iter()
                        .map(|slot| {
                            slot.as_ref()
                                .map(|script| {
                                    decode_script(
                                        s, sounds, states, actors, waypoints, behaviors, script,
                                    )
                                })
                                .transpose()
                        })
                        .collect::<Result<_>>()?,
                    schedule: b
                        .schedule
                        .as_ref()
                        .map(|script| {
                            decode_script(s, sounds, states, actors, waypoints, behaviors, script)
                        })
                        .transpose()?,
                    unknown: b.unknown,
                })
            })
            .collect::<Result<_>>()?,
        default_behavior: set.default,
        action_capacity: set.capacity,
        unknown: set.unknown,
    })
}
fn unsupported_script_diagnostics(model: &AiModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for (group, set) in [("normal", &model.normal), ("reflex", &model.reflex)] {
        for behavior in set.iter().flat_map(|set| &set.behaviors) {
            let scripts = behavior
                .rules
                .iter()
                .enumerate()
                .filter_map(|(slot, script)| {
                    script
                        .as_ref()
                        .map(|script| (format!("rule {slot}"), script))
                })
                .chain(
                    behavior
                        .schedule
                        .iter()
                        .map(|script| ("schedule".into(), script)),
                );
            for (slot, script) in scripts {
                for fragment in script.unsupported_fragments() {
                    diagnostics.push(Diagnostic {
                        code: "openspace.ai.unsupported_script".into(),
                        asset: Some(model.path.clone()),
                        severity: Severity::Warning,
                        message: format!(
                            "{group} behavior {} {slot} node {}: {}",
                            behavior.id, fragment.start, fragment.reason
                        ),
                    });
                }
            }
        }
    }
    diagnostics
}
struct DesignerLayout {
    definition: Option<(Address, DesignerWire)>,
    fields: Vec<InfoWire>,
}
struct CachedModel {
    path: AssetPath,
    index: usize,
    sets: [Option<SourceSet>; 2],
    designer: DesignerLayout,
}
fn designer_layout(s: &SourceSession, model: Address) -> Result<DesignerLayout> {
    let Some(a) = s.pointer(model, 8)? else {
        return Ok(DesignerLayout {
            definition: None,
            fields: Vec::new(),
        });
    };
    let h: DesignerWire = wire(allocated(s, a, 16)?)?;
    let mut infos = Vec::new();
    if h.count > 0 {
        let entries = s.required(a, 4)?;
        allocated(s, entries, h.count as usize * 16)?;
        for i in 0..h.count as usize {
            infos.push(wire(s.bytes(add(entries, i * 16), 16)?)?);
        }
    }
    Ok(DesignerLayout {
        definition: Some((a, h)),
        fields: infos,
    })
}
fn values(
    s: &SourceSession,
    states: &BTreeMap<Address, StateRef>,
    actors: &BTreeMap<Address, AssetRef<crate::model::character::Perso>>,
    a: Address,
    infos: &[InfoWire],
    length: usize,
) -> Result<Vec<DesignerValue>> {
    let bytes = s.bytes(a, length)?;
    let mut out = Vec::new();
    let mut ranges = Vec::new();
    for info in infos {
        let offset = info.offset as usize;
        ensure!(offset < bytes.len(), "designer offset outside memory");
        let is_opaque = !DesignerValueCodec::supports(info.kind);
        let (value, size) = if is_opaque {
            // The next declaration bounds this uninterpreted slot. Retain its
            // bytes as-is; never follow any words inside it as pointers.
            let end = infos
                .iter()
                .map(|following| following.offset as usize)
                .filter(|&following| following > offset)
                .min()
                .unwrap_or(length);
            ensure!(
                end <= length && end > offset,
                "opaque designer field has no bounded extent"
            );
            let size = end - offset;
            (
                DesignerValue::Opaque {
                    kind: info.kind,
                    bytes: s.bytes(add(a, offset), size)?.to_vec(),
                },
                size,
            )
        } else {
            source_view::designer_value(
                &mut super::source_view::FileView(s),
                a,
                *info,
                length,
                |target| {
                    target
                        .map(|target| {
                            states
                                .get(&target)
                                .cloned()
                                .context("action outside imported families")
                        })
                        .transpose()
                },
                |target| Ok(target.and_then(|target| actors.get(&target).cloned())),
            )?
        };
        out.push(value);
        ranges.push((offset, size, is_opaque));
    }
    // Declarations need not follow storage order, and some source definitions
    // intentionally alias fields at the same offset. Preserve both values for
    // such exact-start aliases when at least one declaration is opaque. A
    // partial overlap remains ambiguous and is rejected. Zero-filled padding
    // is not a designer field; nonzero unclaimed bytes remain an import error.
    ranges.sort_unstable_by_key(|&(offset, size, _)| (offset, size));
    let mut end: usize = 0;
    let mut group_start = None;
    let mut group_has_opaque = false;
    for (offset, size, is_opaque) in ranges {
        if offset < end {
            ensure!(
                group_start == Some(offset) && (is_opaque || group_has_opaque),
                "ambiguous overlapping designer memory field at {offset} after {end}"
            );
        } else {
            ensure!(
                bytes[end..offset].iter().all(|&b| b == 0),
                "nonzero designer alignment padding unsupported"
            );
            group_start = Some(offset);
            group_has_opaque = false;
        }
        group_has_opaque |= is_opaque;
        end = end.max(offset + size);
    }
    ensure!(end == length, "unrepresented designer memory tail");
    Ok(out)
}
fn selection(
    s: &SourceSession,
    mind: Address,
    field: usize,
    set: Option<&SourceSet>,
) -> Result<Option<BehaviorSelection>> {
    let Some(a) = s.pointer(mind, field)? else {
        return Ok(None);
    };
    let _: IntelligenceWire = wire(allocated(s, a, 24)?)?;
    let set = set.context("intelligence lacks behavior set")?;
    let slot = s.required(a, 0)?;
    ensure!(
        s.required(slot, 0)? == set.address,
        "intelligence binds another behavior set"
    );
    Ok(Some(BehaviorSelection {
        current: member(s, a, 8, set)?,
        previous: member(s, a, 12, set)?,
        initial: member(s, a, 20, set)?,
    }))
}

pub(super) fn import(
    s: &SourceSession,
    game: &Path,
    selection_value: &Selection,
    states: &BTreeMap<Address, StateRef>,
    actors: &BTreeMap<Address, AssetRef<Perso>>,
    result: &mut SceneImport,
    material_ids: &mut BTreeMap<Address, AssetPath>,
) -> Result<()> {
    if u32_at(&s.gpt, 0x2ae0)? != 0 {
        let a = s.root(0x2ae0)?;
        let _: MainActorWire = wire(allocated(s, a, 20)?)?;
        let node = s.required(a, 4)?;
        let tree = s.tree()?;
        let node = tree
            .iter()
            .find(|n| n.address == node)
            .context("main actor outside rooted scene")?;
        result.scene.main_actor = Some(AssetRef::new(format!("level:/persos/{}", node.path))?);
    }
    let sounds = SoundSites::open(game, selection_value, s)?;
    for &perso in actors.keys() {
        if let Some(component) = s.pointer(perso, 64)?
            && let Some(generator) = s.pointer(component, 0)?
        {
            super::particle::import(s, generator, result, material_ids)?;
        }
    }
    // Resolve kind-32 references against every Brain model in the active
    // rooted/template actor graph before decoding scripts. A behavior member
    // is an interior address in its owning model's allocation, so source
    // allocation identity alone cannot identify an external behavior target.
    let mut source_sets: BTreeMap<Address, [Option<SourceSet>; 2]> = BTreeMap::new();
    for &perso in actors.keys() {
        let header: PersoHeaderWire = wire(s.bytes(perso, 20)?)?;
        if header.brain == 0 {
            continue;
        }
        let Some(brain) = s.pointer(perso, 16)? else {
            continue;
        };
        let _: BrainWire = wire(allocated(s, brain, 8)?)?;
        let mind = s.required(brain, 0)?;
        let _: MindWire = wire(allocated(s, mind, 20)?)?;
        let model = s.required(mind, 0)?;
        if let std::collections::btree_map::Entry::Vacant(entry) = source_sets.entry(model) {
            entry.insert([source_set(s, model, 0)?, source_set(s, model, 4)?]);
        }
    }
    let mut behavior_index: BTreeMap<Address, BehaviorRef> = BTreeMap::new();
    for (&model, sets) in &source_sets {
        let model_path = path(s, model)?;
        for (address, reference) in behavior_bindings(&model_path, sets)? {
            if let Some(previous) = behavior_index.get(&address) {
                ensure!(
                    previous == &reference,
                    "ambiguous source AI behavior address {address:?}: {} and {}",
                    previous.model.path,
                    reference.model.path
                );
            } else {
                behavior_index.insert(address, reference);
            }
        }
    }
    // Script-only waypoints are meaningful shared assets too. Use the same
    // promoted declaration and identity cache as actor-local route points.
    let mut waypoints = BTreeMap::new();
    for set in source_sets.values().flat_map(|sets| sets.iter().flatten()) {
        for behavior in &set.behaviors {
            for script in behavior
                .rules
                .iter()
                .flatten()
                .chain(behavior.schedule.iter())
            {
                for (i, bytes) in script.bytes.as_chunks::<8>().0.iter().enumerate() {
                    let node: NodeWire = wire(bytes)?;
                    if node.kind == 37
                        && let Some(target) = s.pointer(add(script.address, i * 8), 0)?
                    {
                        super::particle::import(s, target, result, material_ids)?;
                    }
                    if node.kind == 29
                        && let Some(target) = s.pointer(add(script.address, i * 8), 0)?
                        && !waypoints.contains_key(&target)
                    {
                        waypoints.insert(target, super::way::import_waypoint(s, target, result)?);
                    }
                }
            }
        }
    }
    let mut cache: BTreeMap<Address, CachedModel> = BTreeMap::new();
    for (&p, perso_reference) in actors {
        let header: PersoHeaderWire = wire(s.bytes(p, 20)?)?;
        if header.brain == 0 {
            continue;
        }
        let Some(a) = s.pointer(p, 16)? else { continue };
        let b: BrainWire = wire(allocated(s, a, 8)?)?;
        let mind = s.required(a, 0)?;
        let m: MindWire = wire(allocated(s, mind, 20)?)?;
        let model = s.required(mind, 0)?;
        let perso = result
            .persos
            .iter_mut()
            .find(|p| p.path == perso_reference.path)
            .with_context(|| {
                format!(
                    "AI perso {} missing from imported content",
                    perso_reference.path
                )
            })?;
        if let std::collections::btree_map::Entry::Vacant(entry) = cache.entry(model) {
            let path = path(s, model)?;
            let h: ModelWire = wire(allocated(s, model, 16)?)?;
            let sets = source_sets
                .remove(&model)
                .context("AI model absent from active actor pre-index")?;
            let designer = designer_layout(s, model)?;
            let DesignerLayout {
                definition,
                fields: infos,
            } = &designer;
            for (index, info) in infos.iter().enumerate() {
                if !DesignerValueCodec::supports(info.kind) {
                    result.diagnostics.push(Diagnostic {
                        code: "openspace.ai.opaque_designer_field".into(),
                        asset: Some(path.clone()),
                        severity: Severity::Warning,
                        message: format!(
                            "designer field {index} source type {} is retained as opaque bytes; script access remains unsupported",
                            info.kind
                        ),
                    });
                }
            }
            let defaults = definition
                .as_ref()
                .map(|(a, h)| {
                    values(
                        s,
                        states,
                        actors,
                        s.required(*a, 0)?,
                        infos,
                        h.length as usize,
                    )
                    .with_context(|| format!("AI model {path} designer defaults"))
                })
                .transpose()?
                .unwrap_or_default();
            let value = AiModel {
                path: path.clone(),
                name: perso.name.clone(),
                normal: sets[0]
                    .as_ref()
                    .map(|set| {
                        decode_set(s, &sounds, states, actors, &waypoints, &behavior_index, set)
                    })
                    .transpose()
                    .with_context(|| format!("AI model {path} normal behavior set"))?,
                reflex: sets[1]
                    .as_ref()
                    .map(|set| {
                        decode_set(s, &sounds, states, actors, &waypoints, &behavior_index, set)
                    })
                    .transpose()
                    .with_context(|| format!("AI model {path} reflex behavior set"))?,
                variables: infos
                    .iter()
                    .zip(defaults)
                    .map(|(i, default)| DesignerVariable {
                        default,
                        save: i.save,
                        initialize: i.initialize,
                    })
                    .collect(),
                designer_unknown: definition.as_ref().map_or([0; 3], |(_, h)| h.unknown),
                unknown: h.unknown,
            };
            value.validate()?;
            result
                .diagnostics
                .extend(unsupported_script_diagnostics(&value));
            let index = result.ai_models.len();
            result.ai_models.push(value);
            entry.insert(CachedModel {
                path,
                index,
                sets,
                designer,
            });
        }
        let CachedModel {
            path,
            index,
            sets,
            designer,
        } = &cache[&model];
        let DesignerLayout {
            definition,
            fields: infos,
        } = designer;
        let ai = &result.ai_models[*index];
        let mut memory = ai.variables.iter().map(|v| v.default.clone()).collect();
        let mut initial = None;
        if let Some(mem) = s.pointer(mind, 12)? {
            let _: MemoryWire = wire(allocated(s, mem, 12)?)?;
            let (definition, h) = definition
                .as_ref()
                .context("designer memory lacks definitions")?;
            ensure!(
                s.required(s.required(mem, 0)?, 0)? == *definition,
                "designer memory refers to another definition"
            );
            if let Some(a) = s.pointer(mem, 8)? {
                memory = values(s, states, actors, a, infos, h.length as usize)
                    .with_context(|| format!("AI model {path} current designer memory"))?;
            }
            initial = s
                .pointer(mem, 4)?
                .map(|a| {
                    values(s, states, actors, a, infos, h.length as usize)
                        .with_context(|| format!("AI model {path} initial designer memory"))
                })
                .transpose()?;
        }
        let brain = Brain {
            model: AssetRef::from_path(path.clone()),
            normal: selection(s, mind, 4, sets[0].as_ref())?,
            reflex: selection(s, mind, 8, sets[1].as_ref())?,
            variables: memory,
            initial_variables: initial,
            unknown: b.unknown,
            mind_unknown: m.unknown,
        };
        brain.validate_model(ai)?;
        perso.brain = Some(brain);
    }
    Ok(())
}
