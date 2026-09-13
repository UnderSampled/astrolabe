//! Session-bound AI model/instance import. Unsupported programs are diagnosed whole.
#[cfg(test)]
#[path = "ai_probe.rs"]
mod tests;
use super::{
    container::{u32_at, wire},
    records::*,
    scene::SceneImport,
    session::SourceSession,
    sound::SoundSites,
};
use crate::{
    model::{
        AssetPath, AssetRef, Selection,
        ai::*,
        io::{Content, Diagnostic, Severity},
    },
    openspace::{
        ai::{AiScriptCodec, NodeWire},
        codec::FormatCodec,
    },
};
use anyhow::{Context, Result, ensure};
use binrw::{BinRead, BinWrite};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// Montreal's extra word follows the three name pointers; Brain is at +16.
/// Other Perso fields are decoded by the character importer.
#[derive(BinRead)]
#[br(little)]
struct PersoBrainWire {
    _names: [u32; 3],
    _montreal: u32,
    brain: u32,
}

#[derive(BinRead, BinWrite)]
#[brw(little)]
struct BrainWire {
    mind: u32,
    unknown: u32,
}
/// GPT+0x2ae0 selects the first controlled actor. 00443470/00443460 and
/// MainActor keyword 00549a00 follow this 20-byte entry's node at +4.
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct MainActorWire {
    unknown: u32,
    node: u32,
    next: u32,
    previous: u32,
    header: u32,
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct MindWire {
    model: u32,
    normal: u32,
    reflex: u32,
    variables: u32,
    unknown: [u8; 4],
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct ModelWire {
    normal: u32,
    reflex: u32,
    variables: u32,
    unknown: [u8; 4],
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct SetWire {
    entries: u32,
    count: u32,
    default: u32,
    capacity: u8,
    unknown: [u8; 3],
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct BehaviorWire {
    rules: u32,
    schedule: u32,
    count: u8,
    unknown: [u8; 3],
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct IntelligenceWire {
    model_slot: u32,
    execution: u32,
    current: u32,
    previous: u32,
    actions: u32,
    initial: u32,
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct MemoryWire {
    definition_slot: u32,
    initial: u32,
    current: u32,
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct DesignerWire {
    defaults: u32,
    infos: u32,
    length: u32,
    count: u8,
    unknown: [u8; 3],
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
struct InfoWire {
    offset: u32,
    kind: u32,
    save: u32,
    initialize: u32,
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
fn unsupported(sets: &[Option<SourceSet>; 2]) -> Result<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    for set in sets.iter().flatten() {
        for b in &set.behaviors {
            for script in b.rules.iter().flatten().chain(b.schedule.iter()) {
                for bytes in script.bytes.as_chunks::<8>().0 {
                    let n: NodeWire = wire(bytes)?;
                    let supported = match n.kind {
                        0 => matches!(n.parameter, 0..=2 | 5),
                        1 => n.parameter <= 8,
                        2 => n.parameter == 11,
                        3 => n.parameter == 4,
                        4 => matches!(n.parameter, 127 | 206),
                        5 => n.parameter == 37,
                        8 | 11 | 12 | 13 | 34 => true,
                        _ => false,
                    };
                    if !supported {
                        // Reference operands contain source VM values. Keep those
                        // out of diagnostics persisted with useful/Rete assets.
                        out.insert(if n.kind <= 5 {
                            format!("category {} operation {}", n.kind, n.parameter)
                        } else {
                            format!("node category {}", n.kind)
                        });
                    }
                }
            }
        }
    }
    Ok(out)
}
fn decode_script(sounds: &SoundSites, script: &SourceScript) -> Result<Script> {
    let mut codec = AiScriptCodec::default();
    for (i, bytes) in script.bytes.as_chunks::<8>().0.iter().enumerate() {
        let n: NodeWire = wire(bytes)?;
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
fn decode_set(sounds: &SoundSites, set: &SourceSet) -> Result<BehaviorSet> {
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
                        .map(|s| s.as_ref().map(|s| decode_script(sounds, s)).transpose())
                        .collect::<Result<_>>()?,
                    schedule: b
                        .schedule
                        .as_ref()
                        .map(|s| decode_script(sounds, s))
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
    a: Address,
    infos: &[InfoWire],
    length: usize,
) -> Result<Vec<DesignerValue>> {
    let bytes = s.bytes(a, length)?;
    let mut out = Vec::new();
    let mut offset: usize = 0;
    for info in infos {
        let size = if info.kind == 0 { 1 } else { 4 };
        let aligned = offset.div_ceil(size) * size;
        ensure!(
            info.offset as usize == aligned,
            "unsupported designer storage order/alignment"
        );
        ensure!(
            bytes
                .get(offset..aligned)
                .context("short designer padding")?
                .iter()
                .all(|&b| b == 0),
            "nonzero designer padding unsupported"
        );
        offset = aligned;
        let value = match info.kind {
            0 => {
                let b = *bytes.get(offset).context("short Boolean designer memory")?;
                ensure!(b <= 1, "noncanonical Boolean designer value");
                DesignerValue::Boolean(b != 0)
            }
            5 => DesignerValue::Integer(u32_at(bytes, offset)? as i32),
            7 => DesignerValue::Real(f32::from_bits(u32_at(bytes, offset)?)),
            _ => anyhow::bail!("unsupported designer type {}", info.kind),
        };
        value.validate()?;
        out.push(value);
        offset += size;
    }
    ensure!(offset == length, "unrepresented designer buffer tail");
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
    for node in s.tree()? {
        if u32_at(s.bytes(node.address, 4)?, 0)? != 4 {
            continue;
        }
        let p = s.required(node.address, 4)?;
        let header: PersoBrainWire = wire(s.bytes(p, 20)?)?;
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
            .find(|p| p.path.file() == format!("persos/{}", node.path))
            .context("AI perso missing from scene")?;
        if let std::collections::btree_map::Entry::Vacant(entry) = cache.entry(model) {
            let path = path(s, model)?;
            let h: ModelWire = wire(allocated(s, model, 16)?)?;
            let sets = [source_set(s, model, 0)?, source_set(s, model, 4)?];
            let mut missing = unsupported(&sets)?;
            let designer = designer_layout(s, model)?;
            let DesignerLayout {
                definition,
                fields: infos,
            } = &designer;
            for info in infos {
                if !matches!(info.kind, 0 | 5 | 7) {
                    missing.insert(format!("designer type {}", info.kind));
                }
            }
            let index = if missing.is_empty() {
                let defaults = definition
                    .as_ref()
                    .map(|(a, h)| values(s, s.required(*a, 0)?, infos, h.length as usize))
                    .transpose()?
                    .unwrap_or_default();
                let value = AiModel {
                    path: path.clone(),
                    name: perso.name.clone(),
                    normal: sets[0]
                        .as_ref()
                        .map(|set| decode_set(&sounds, set))
                        .transpose()?,
                    reflex: sets[1]
                        .as_ref()
                        .map(|set| decode_set(&sounds, set))
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
                memory = values(s, a, infos, h.length as usize)?;
            }
            initial = s
                .pointer(mem, 4)?
                .map(|a| values(s, a, infos, h.length as usize))
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
