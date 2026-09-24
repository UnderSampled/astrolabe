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
    rooted: &BTreeMap<Address, AssetRef<Perso>>,
) -> Result<Vec<Address>> {
    let mut pending: Vec<_> = rooted.keys().copied().collect();
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
                        if !rooted.contains_key(&target) && !templates.contains(&target) {
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
) -> BTreeMap<Address, BehaviorRef> {
    [BehaviorGroup::Normal, BehaviorGroup::Reflex]
        .into_iter()
        .zip(sets)
        .flat_map(|(group, set)| {
            set.iter().flat_map(move |s| {
                s.behaviors.iter().enumerate().map(move |(i, b)| {
                    (
                        b.address,
                        BehaviorRef {
                            model: AssetRef::from_path(path.clone()),
                            group,
                            behavior: i as u32,
                        },
                    )
                })
            })
        })
        .collect()
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
    }
    codec.read(&script.bytes)
}
fn decode_set(
    s: &SourceSession,
    sounds: &SoundSites,
    states: &BTreeMap<Address, StateRef>,
    actors: &BTreeMap<Address, AssetRef<Perso>>,
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
                                    decode_script(s, sounds, states, actors, behaviors, script)
                                })
                                .transpose()
                        })
                        .collect::<Result<_>>()?,
                    schedule: b
                        .schedule
                        .as_ref()
                        .map(|script| decode_script(s, sounds, states, actors, behaviors, script))
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
    index: Option<usize>,
    sets: [Option<SourceSet>; 2],
    designer: DesignerLayout,
    unsupported: String,
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
        let (value, size) = source_view::designer_value(
            &mut source_view::FileView(s),
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
        )?;
        out.push(value);
        ranges.push((offset, size));
    }
    // Declarations need not follow storage order, and some scalar fields are
    // packed. Both orders are valid; all bytes must still be accounted for.
    ranges.sort_unstable();
    let mut end: usize = 0;
    for (offset, size) in ranges {
        let alignment = size.min(4);
        ensure!(
            offset == end || offset == end.div_ceil(alignment) * alignment,
            "overlapping or unexplained designer memory gap at {offset} after {end}"
        );
        ensure!(
            bytes[end..offset].iter().all(|&b| b == 0),
            "nonzero designer alignment padding unsupported"
        );
        end = offset + size;
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
            let sets = [source_set(s, model, 0)?, source_set(s, model, 4)?];
            let behaviors = behavior_bindings(&path, &sets);
            let mut missing = BTreeSet::new();
            let designer = designer_layout(s, model)?;
            let DesignerLayout {
                definition,
                fields: infos,
            } = &designer;
            for info in infos {
                if !DesignerValueCodec::supports(info.kind) {
                    missing.insert(format!("designer type {}", info.kind));
                }
            }
            let index = if missing.is_empty() {
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
                    })
                    .transpose()?
                    .unwrap_or_default();
                let value = AiModel {
                    path: path.clone(),
                    name: perso.name.clone(),
                    normal: sets[0]
                        .as_ref()
                        .map(|set| decode_set(s, &sounds, states, actors, &behaviors, set))
                        .transpose()?,
                    reflex: sets[1]
                        .as_ref()
                        .map(|set| decode_set(s, &sounds, states, actors, &behaviors, set))
                        .transpose()?,
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
                Some(index)
            } else {
                None
            };
            entry.insert(CachedModel {
                path,
                index,
                sets,
                designer,
                unsupported: missing.into_iter().collect::<Vec<_>>().join(", "),
            });
        }
        let CachedModel {
            path,
            index,
            sets,
            designer,
            unsupported: missing,
        } = &cache[&model];
        let DesignerLayout {
            definition,
            fields: infos,
        } = designer;
        let Some(index) = index else {
            result.diagnostics.push(Diagnostic {
                code: "openspace.ai.unsupported".into(),
                asset: Some(perso.path.clone()),
                severity: Severity::Warning,
                message: format!("AI model {path} is not exported: unsupported {missing}"),
            });
            continue;
        };
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
                memory = values(s, states, actors, a, infos, h.length as usize)?;
            }
            initial = s
                .pointer(mem, 4)?
                .map(|a| values(s, states, actors, a, infos, h.length as usize))
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
