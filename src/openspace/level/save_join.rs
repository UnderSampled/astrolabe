//! Research-only joins from save identities to bounded source allocations.
//! These locations are source context, not portable model or Godot identities.

use super::{
    container::wire,
    records::{
        Address, BlockKey, DesignerWire, GaugeChildWire, GaugeRootWire, InfoWire, Owner,
        PersoSectorWire, SectorCharacterListWire, SectorWire, StandardSaveWire, SuperObjectWire,
    },
    session::SourceSession,
};
use crate::{
    model::{Selection, ai::SourceRole},
    openspace::ai_values::DesignerValueCodec,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Debug, Clone, Deserialize)]
pub struct SaveRecordLocation {
    pub id: String,
    pub block: u8,
    pub module: u8,
    pub offset: u32,
    /// Bytes in the current source allocation, not the encoded reference size.
    pub live_width: usize,
    pub kind: String,
    pub flags: String,
    /// Kind 0x11 target from the decoded save, if present.
    #[serde(default)]
    pub reference_target: Option<(u8, u8, u32)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveAllocationCandidate {
    pub owner: &'static str,
    pub block_size: usize,
    pub allocation_index: Option<u32>,
    pub allocation_data_start: Option<usize>,
    pub allocation_end: Option<usize>,
    pub allocation_issue: Option<String>,
    /// Bounded source sample for reference-field research only.
    pub data_prefix_hex: Option<String>,
    pub source_object: Option<String>,
    pub source_appearance: Option<String>,
    pub source_field: Option<String>,
    pub source_meaning: Option<String>,
    pub source_rule: Option<&'static str>,
    pub source_type: Option<String>,
    pub field_match_ambiguity: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveAllocationJoin {
    pub id: String,
    pub status: &'static str,
    pub candidates: Vec<SaveAllocationCandidate>,
    pub reference_target: Option<SaveReferenceTargetJoin>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveReferenceTargetJoin {
    pub identity: String,
    pub status: &'static str,
    pub owner: Option<&'static str>,
    pub allocation_index: Option<u32>,
    pub allocation_data_start: Option<usize>,
    pub allocation_end: Option<usize>,
    pub offset_in_allocation: Option<usize>,
    /// Research sample from the source allocation; never a portable asset value.
    pub data_prefix_hex: Option<String>,
    pub superobject_kind: Option<u32>,
    pub superobject_data_identity: Option<String>,
    pub rooted_perso: Option<String>,
    pub instantiated_item_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveJoinResult {
    pub joins: Vec<SaveAllocationJoin>,
    pub designer_discovery: DesignerDiscovery,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DesignerDiscovery {
    pub perso_nodes: usize,
    pub persos_without_brain: usize,
    pub descriptors_seen: usize,
    pub saved_descriptors: usize,
    pub matched_saved_descriptors: usize,
    pub unmatched_saved_descriptors: Vec<String>,
    pub saved_fields: Vec<DesignerFieldEvidence>,
    pub failed_actors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DesignerFieldEvidence {
    pub source_object: String,
    pub source_appearance: Option<String>,
    pub field: String,
    pub designer_type: u32,
    pub save_flags: u32,
    pub initialize_flags: u32,
    pub extent: usize,
    pub source_location: String,
}

#[derive(Debug, Clone)]
struct DesignerCandidate {
    object: String,
    appearance: Option<String>,
    field: String,
    kind: u32,
    save: u32,
    extent: usize,
}

type ActorFieldCandidates = BTreeMap<Address, Vec<(String, Option<String>)>>;

#[derive(Debug, Clone)]
struct GaugeSaveField {
    object: String,
    field: String,
    meaning: Option<String>,
}

fn allocation_extent(session: &SourceSession, address: Address) -> Result<usize> {
    let start = address.offset;
    let words = u32::from_le_bytes(
        session
            .bytes(
                Address {
                    offset: start - 4,
                    ..address
                },
                4,
            )?
            .try_into()?,
    ) as usize;
    Ok(words * 4 - 4)
}

/// Finds the unique three-root Hype gauge chain through declared Fix relocations.
/// Root/child ordinals are retained as source identities; UI names are not inferred.
fn gauge_save_fields(session: &SourceSession) -> Result<BTreeMap<usize, GaugeSaveField>> {
    // Save identities spell this block as 0:5 (block:module); SourceSession
    // keys follow the SNA header's module:id order, hence (5, 0).
    let key = BlockKey(5, 0);
    if !session
        .blocks
        .get(&Owner::Fix)
        .is_some_and(|blocks| blocks.contains_key(&key))
    {
        return Ok(BTreeMap::new());
    }
    let starts = session.allocation_indices(Owner::Fix, key)?;
    let size_by_start = starts
        .keys()
        .map(|&offset| {
            let address = Address {
                owner: Owner::Fix,
                key,
                offset,
            };
            Ok((offset, allocation_extent(session, address)?))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let root_starts = starts
        .keys()
        .filter_map(|&offset| {
            (size_by_start.get(&offset) == Some(&40)).then_some(Address {
                owner: Owner::Fix,
                key,
                offset,
            })
        })
        .collect::<Vec<_>>();
    let mut matches = Vec::new();
    for head in root_starts {
        let candidate = (|| -> Result<BTreeMap<usize, GaugeSaveField>> {
            let mut fields = BTreeMap::new();
            let mut roots = BTreeSet::new();
            let mut all_children = BTreeSet::new();
            let mut current = Some(head);
            while let Some(root) = current {
                anyhow::ensure!(
                    root.owner == Owner::Fix && root.key == key,
                    "gauge root leaves Fix 0:5"
                );
                anyhow::ensure!(roots.insert(root), "gauge root cycle/alias");
                anyhow::ensure!(roots.len() <= 3, "too many gauge roots");
                anyhow::ensure!(
                    size_by_start.get(&root.offset) == Some(&40),
                    "gauge root extent"
                );
                let layout: GaugeRootWire = super::container::wire(session.bytes(root, 40)?)?;
                anyhow::ensure!(
                    matches!(layout.child_count, 5 | 7),
                    "unexpected gauge child count"
                );
                let root_path = format!("Fix gauge root #{} (source list order)", roots.len() - 1);
                fields.insert(
                    root.offset,
                    GaugeSaveField {
                        object: root_path.clone(),
                        field: "GaugeRoot.next_root".into(),
                        meaning: Some(
                            "link to the next gauge panel root in source list order".into(),
                        ),
                    },
                );
                fields.insert(
                    root.offset + 0x1e,
                    GaugeSaveField {
                        object: root_path.clone(),
                        field: "GaugeRoot.completed_children".into(),
                        meaning: Some(
                            "number of gauge child items completed for this panel".into(),
                        ),
                    },
                );
                fields.insert(
                    root.offset + 0x1f,
                    GaugeSaveField {
                        object: root_path,
                        field: "GaugeRoot.threshold_selector".into(),
                        meaning: None,
                    },
                );
                let mut child = session.pointer(root, 0x10)?;
                let mut children = BTreeSet::new();
                for ordinal in 0..usize::from(layout.child_count) {
                    let item = child.ok_or_else(|| anyhow::anyhow!("short gauge child list"))?;
                    anyhow::ensure!(
                        item.owner == Owner::Fix && item.key == key,
                        "gauge child leaves Fix 0:5"
                    );
                    anyhow::ensure!(children.insert(item), "gauge child cycle");
                    anyhow::ensure!(all_children.insert(item), "gauge child alias");
                    anyhow::ensure!(
                        size_by_start.get(&item.offset) == Some(&24),
                        "gauge child extent"
                    );
                    let _: GaugeChildWire = super::container::wire(session.bytes(item, 24)?)?;
                    let object = format!(
                        "Fix gauge root #{} item #{ordinal} (source list order)",
                        roots.len() - 1
                    );
                    fields.insert(item.offset + 0x14, GaugeSaveField {
                        object: object.clone(),
                        field: "GaugeChild.visual_phase".into(),
                        meaning: Some("per-item visual phase selected for rendering; advances when progress passes the selected threshold".into()),
                    });
                    fields.insert(item.offset + 0x15, GaugeSaveField {
                        object,
                        field: "GaugeChild.progress_tick".into(),
                        meaning: Some("per-item progress tick incremented while visual phase is zero and compared with the selected threshold".into()),
                    });
                    child = session.pointer(item, 0)?;
                    if let Some(next) = child {
                        anyhow::ensure!(
                            next.owner == Owner::Fix && next.key == key,
                            "gauge child link leaves Fix 0:5"
                        );
                    }
                }
                anyhow::ensure!(child.is_none(), "gauge child count/list mismatch");
                current = session.pointer(root, 0)?;
                if let Some(next) = current {
                    anyhow::ensure!(
                        next.owner == Owner::Fix && next.key == key,
                        "gauge root link leaves Fix 0:5"
                    );
                }
            }
            anyhow::ensure!(
                roots.len() == 3 && all_children.len() == 17,
                "gauge chain shape mismatch"
            );
            Ok(fields)
        })();
        if let Ok(fields) = candidate {
            matches.push(fields);
        }
    }
    anyhow::ensure!(
        matches.len() <= 1,
        "ambiguous structurally valid gauge chains: {}",
        matches.len()
    );
    Ok(matches.pop().unwrap_or_default())
}

fn designer_candidates(
    session: &SourceSession,
) -> Result<(BTreeMap<Address, Vec<DesignerCandidate>>, DesignerDiscovery)> {
    let mut fields = BTreeMap::<Address, Vec<DesignerCandidate>>::new();
    let mut discovery = DesignerDiscovery::default();
    let fix_global = super::scene::global_perso_source(session)?.map(|(address, _)| address);
    for node in session.tree()? {
        let header: SuperObjectWire = wire(session.bytes(node.address, 64)?)?;
        if header.kind != 4 {
            continue;
        }
        discovery.perso_nodes += 1;
        let result = (|| -> Result<()> {
            let perso = session.required(node.address, 4)?;
            let rooted_path = format!("level:/persos/{}", node.path);
            let (canonical_path, appearance) = if Some(perso) == fix_global {
                ("fix:/persos/15".to_owned(), Some(rooted_path))
            } else {
                (rooted_path, None)
            };
            let Some(brain) = session.pointer(perso, 16)? else {
                discovery.persos_without_brain += 1;
                return Ok(());
            };
            let mind = session.required(brain, 0)?;
            let model = session.required(mind, 0)?;
            let Some(definition) = session.pointer(model, 8)? else {
                return Ok(());
            };
            let layout: DesignerWire = wire(session.bytes(definition, 16)?)?;
            if layout.count == 0 {
                return Ok(());
            }
            let infos = session.required(definition, 4)?;
            let memory = session.required(mind, 12)?;
            let current = session.required(memory, 8)?;
            for index in 0..usize::from(layout.count) {
                let info: InfoWire = wire(session.bytes(
                    Address {
                        offset: infos.offset + index * 16,
                        ..infos
                    },
                    16,
                )?)?;
                discovery.descriptors_seen += 1;
                if info.save & 0x0f00 == 0 {
                    continue;
                }
                discovery.saved_descriptors += 1;
                if !DesignerValueCodec::supports(info.kind) {
                    anyhow::bail!("unsupported designer type {} at index {index}", info.kind);
                }
                let field = Address {
                    offset: current.offset + info.offset as usize,
                    ..current
                };
                let prefix = match info.kind {
                    0..=2 => 1,
                    3..=4 => 2,
                    5..=8 | 10..=11 | 14 => 4,
                    12 => 12,
                    20..=23 | 25 => 8,
                    _ => unreachable!(),
                };
                let codec = DesignerValueCodec {
                    kind: info.kind,
                    ..Default::default()
                };
                let extent = codec.extent(session.bytes(field, prefix)?)?;
                anyhow::ensure!(
                    info.offset as usize + extent <= layout.length as usize,
                    "designer field exceeds declared memory"
                );
                session.bytes(field, extent)?;
                discovery.saved_fields.push(DesignerFieldEvidence {
                    source_object: canonical_path.clone(),
                    source_appearance: appearance.clone(),
                    field: format!("DSG[{index}]"),
                    designer_type: info.kind,
                    save_flags: info.save,
                    initialize_flags: info.initialize,
                    extent,
                    source_location: format!(
                        "{:?}:{}:{}:{}",
                        field.owner, field.key.1, field.key.0, field.offset
                    ),
                });
                fields.entry(field).or_default().push(DesignerCandidate {
                    object: canonical_path.clone(),
                    appearance: appearance.clone(),
                    field: format!("DSG[{index}]"),
                    kind: info.kind,
                    save: info.save,
                    extent,
                });
            }
            Ok(())
        })();
        if let Err(error) = result {
            discovery
                .failed_actors
                .push(format!("{}: {error:#}", node.path));
        }
    }
    Ok((fields, discovery))
}

fn inventory_member_name(session: &SourceSession, member: Address) -> Option<String> {
    let record = session.bytes(member, 30).ok()?;
    let id = i16::from_le_bytes(record[28..30].try_into().ok()?);
    if !(0..=1000).contains(&id) {
        return None;
    }
    let name = session.required(member, 12).ok()?;
    let bytes = session.bytes(name, 32).ok()?;
    let length = bytes.iter().position(|byte| *byte == 0)?;
    let label = std::str::from_utf8(&bytes[..length]).ok()?;
    if label.is_empty()
        || !label.bytes().next()?.is_ascii_alphabetic()
        || !label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return None;
    }
    Some(format!("{label} {id}"))
}

fn inventory_handle_name(session: &SourceSession, start: usize, end: usize) -> Option<String> {
    if end - start != 24 {
        return None;
    }
    let handle = Address {
        owner: Owner::Fix,
        key: BlockKey(10, 0),
        offset: start,
    };
    let member = session.required(handle, 0).ok()?;
    Some(format!(
        "Fix inventory {}",
        inventory_member_name(session, member)?
    ))
}

/// Associates a main inventory-list handle with the separately allocated item
/// it points at. In Hype, the handle is 24 bytes and the item is 32 bytes;
/// registration sites save the item's +0x14 maximum-capacity word.
fn inventory_item_names(session: &SourceSession) -> BTreeMap<Address, Vec<String>> {
    let key = BlockKey(10, 0);
    let Ok(allocations) = session.allocation_indices(Owner::Fix, key) else {
        return BTreeMap::new();
    };
    let Some(block) = session
        .blocks
        .get(&Owner::Fix)
        .and_then(|blocks| blocks.get(&key))
    else {
        return BTreeMap::new();
    };
    let mut result = BTreeMap::<Address, Vec<String>>::new();
    let mut unnamed_targets = Vec::new();
    for &start in allocations.keys() {
        let header = start.saturating_sub(4);
        let size = block
            .data
            .get(header..start)
            .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
            .map(u32::from_le_bytes)
            .and_then(|words| usize::try_from(words).ok())
            .and_then(|words| words.checked_mul(4))
            .and_then(|size| size.checked_sub(4));
        if size != Some(24) {
            continue;
        }
        let handle = Address {
            owner: Owner::Fix,
            key,
            offset: start,
        };
        let Ok(member) = session.required(handle, 0) else {
            continue;
        };
        if member.owner != Owner::Fix
            || member.key != key
            || !allocations.contains_key(&member.offset)
        {
            continue;
        }
        let item_header = member.offset.saturating_sub(4);
        let item_size = block
            .data
            .get(item_header..member.offset)
            .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
            .map(u32::from_le_bytes)
            .and_then(|words| usize::try_from(words).ok())
            .and_then(|words| words.checked_mul(4))
            .and_then(|size| size.checked_sub(4));
        if item_size != Some(32) {
            continue;
        }
        let Some(name) = inventory_member_name(session, member) else {
            unnamed_targets.push((member, start));
            continue;
        };
        result
            .entry(member)
            .or_default()
            .push(format!("Fix inventory {name}"));
    }
    // Retain a root-derived generic item owner only when exactly one bounded
    // target lacks a member name. Its cursor attribution remains unproven.
    if unnamed_targets.len() == 1 {
        let (item, _handle_start) = unnamed_targets[0];
        result
            .entry(item)
            .or_default()
            .push("Fix inventory item (member name unavailable)".into());
    }
    result
}

fn reverse_perso_owners_of_standard(
    session: &SourceSession,
    standard: Address,
) -> Result<Vec<(Address, String)>> {
    let mut owners = BTreeMap::new();
    for (&(source_owner, source_key, source_virtual), &target_key) in &session.pointers {
        if target_key != standard.key {
            continue;
        }
        let Some(block) = session
            .blocks
            .get(&source_owner)
            .and_then(|blocks| blocks.get(&source_key))
        else {
            continue;
        };
        let Some(site_offset) = source_virtual
            .checked_sub(block.base)
            .and_then(|offset| usize::try_from(offset).ok())
        else {
            continue;
        };
        let Some(parent_offset) = site_offset.checked_sub(4) else {
            continue;
        };
        let site = Address {
            owner: source_owner,
            key: source_key,
            offset: site_offset,
        };
        if session.pointer(site, 0).ok().flatten() != Some(standard) {
            continue;
        }
        let parent = Address {
            owner: source_owner,
            key: source_key,
            offset: parent_offset,
        };
        if session.allocation_index(parent).is_err() {
            continue;
        }
        let Some(header) = parent_offset
            .checked_sub(4)
            .and_then(|offset| block.data.get(offset..parent_offset))
        else {
            continue;
        };
        let Some(size) = u32::from_le_bytes(header.try_into()?)
            .checked_mul(4)
            .and_then(|size| size.checked_sub(4))
        else {
            continue;
        };
        if size != 76 || site_offset + 4 > block.data.len() {
            continue;
        }
        if super::source_view::perso_parts(&mut super::source_view::FileView(session), parent)
            .is_err()
        {
            continue;
        }
        let identity = session.source_identity(parent)?;
        let role = if identity.role == SourceRole::Fix {
            "fix"
        } else {
            "level"
        };
        owners.insert(
            parent,
            format!(
                "{role}:/persos/{}-{}-{}",
                identity.block[0], identity.block[1], identity.allocation
            ),
        );
    }
    Ok(owners.into_iter().collect())
}

fn standard_save_candidates(
    session: &SourceSession,
    records: &[SaveRecordLocation],
) -> Result<ActorFieldCandidates> {
    let fix_global = super::scene::global_perso_source(session)?.map(|(address, _)| address);
    let mut fields = BTreeMap::<Address, BTreeMap<Address, (String, Option<String>)>>::new();
    let mut rooted = BTreeSet::new();
    for node in session.tree()? {
        let header: SuperObjectWire = wire(session.bytes(node.address, 64)?)?;
        if header.kind != 4 {
            continue;
        }
        let perso = session.required(node.address, 4)?;
        rooted.insert(perso);
        let standard = session.required(perso, 4)?;
        let source: StandardSaveWire = wire(session.bytes(standard, 56)?)?;
        let _ = (
            source.hit_points_init,
            source.hit_points_max_init,
            source.hit_points_max_max,
            source.never_back_latch,
            source.active,
            source.custom_bits_init,
        );
        let appearance = format!("level:/persos/{}", node.path);
        let actor = if Some(perso) == fix_global {
            ("fix:/persos/15".to_owned(), Some(appearance))
        } else {
            (appearance, None)
        };
        if source.return_policy == 4 {
            for relative in [27, 28] {
                let field = Address {
                    offset: standard.offset + relative,
                    ..standard
                };
                fields
                    .entry(field)
                    .or_default()
                    .insert(perso, actor.clone());
            }
        }
        let field = Address {
            offset: standard.offset + 44,
            ..standard
        };
        fields.entry(field).or_default().insert(perso, actor);
    }
    if let Some(global) = fix_global {
        rooted.insert(global);
    }
    for template in super::ai::object_templates(session, &rooted)? {
        let identity = session.source_identity(template)?;
        let standard = session.required(template, 4)?;
        let source: StandardSaveWire = wire(session.bytes(standard, 56)?)?;
        let _ = source.custom_bits_init;
        let role = if identity.role == SourceRole::Fix {
            "fix"
        } else {
            "level"
        };
        let actor = format!(
            "{role}:/persos/{}-{}-{}",
            identity.block[0], identity.block[1], identity.allocation
        );
        fields
            .entry(Address {
                offset: standard.offset + 44,
                ..standard
            })
            .or_default()
            .insert(template, (actor, None));
    }
    for record in records {
        if record.kind != "0x04" || record.flags != "0x0204" || record.live_width != 4 {
            continue;
        }
        let key = BlockKey(record.module, record.block);
        let Some(standard_offset) = usize::try_from(record.offset)
            .ok()
            .and_then(|offset| offset.checked_sub(44))
        else {
            continue;
        };
        for &owner in &[Owner::Fix, Owner::Level] {
            let standard = Address {
                owner,
                key,
                offset: standard_offset,
            };
            let field = Address {
                offset: standard_offset + 44,
                ..standard
            };
            if session.allocation_index(standard).is_err() {
                continue;
            }
            let Some(header_offset) = standard_offset.checked_sub(4) else {
                continue;
            };
            let Ok(header) = session.bytes(
                Address {
                    offset: header_offset,
                    ..standard
                },
                4,
            ) else {
                continue;
            };
            let Some(size) = u32::from_le_bytes(header.try_into()?)
                .checked_mul(4)
                .and_then(|size| size.checked_sub(4))
            else {
                continue;
            };
            if size != 56 || session.bytes(standard, 56).is_err() {
                continue;
            }
            let parents = reverse_perso_owners_of_standard(session, standard)?;
            for (parent, actor) in parents {
                fields
                    .entry(field)
                    .or_default()
                    .entry(parent)
                    .or_insert((actor, None));
            }
        }
    }
    Ok(fields
        .into_iter()
        .map(|(field, actors)| (field, actors.into_values().collect()))
        .collect())
}

/// Maps the three script-authored hit-point bytes only for rooted StandardGame
/// records whose return policy enables their source registration.
fn standard_hitpoint_candidates(session: &SourceSession) -> Result<ActorFieldCandidates> {
    let fix_global = super::scene::global_perso_source(session)?.map(|(address, _)| address);
    let mut fields = BTreeMap::<Address, Vec<(String, Option<String>)>>::new();
    for node in session.tree()? {
        let header: SuperObjectWire = wire(session.bytes(node.address, 64)?)?;
        if header.kind != 4 {
            continue;
        }
        let perso = session.required(node.address, 4)?;
        let standard = session.required(perso, 4)?;
        let source: StandardSaveWire = wire(session.bytes(standard, 56)?)?;
        if source.return_policy != 4 {
            continue;
        }
        let appearance = format!("level:/persos/{}", node.path);
        let actor = if Some(perso) == fix_global {
            ("fix:/persos/15".to_owned(), Some(appearance))
        } else {
            (appearance, None)
        };
        for relative in [24, 25, 26] {
            fields
                .entry(Address {
                    offset: standard.offset + relative,
                    ..standard
                })
                .or_default()
                .push(actor.clone());
        }
    }
    Ok(fields)
}

fn sector_character_candidates(session: &SourceSession) -> Result<ActorFieldCandidates> {
    let tree = session.tree()?;
    let fix_global = super::scene::global_perso_source(session)?.map(|(address, _)| address);
    let mut fields = BTreeMap::<Address, Vec<(String, Option<String>)>>::new();
    for node in tree {
        let header: SuperObjectWire = wire(session.bytes(node.address, 64)?)?;
        if header.kind != 4 {
            continue;
        }
        let perso = session.required(node.address, 4)?;
        let layout: PersoSectorWire = wire(session.bytes(perso, 44)?)?;
        if layout.sector_character_list == 0 {
            continue;
        }
        let Some(list) = session.pointer(perso, 40)? else {
            continue;
        };
        let _: SectorCharacterListWire = wire(session.bytes(list, 24)?)?;
        let appearance = format!("level:/persos/{}", node.path);
        let owner = if Some(perso) == fix_global {
            ("fix:/persos/15".to_owned(), Some(appearance))
        } else {
            (appearance, None)
        };
        fields
            .entry(Address {
                offset: list.offset + 4,
                ..list
            })
            .or_default()
            .push(owner);
    }
    Ok(fields)
}

fn rooted_sector_data(session: &SourceSession) -> Result<BTreeMap<Address, String>> {
    let mut sectors = BTreeMap::new();
    for node in session.tree()? {
        let header: SuperObjectWire = wire(session.bytes(node.address, 64)?)?;
        if header.kind != 8 {
            continue;
        }
        let data = session.required(node.address, 4)?;
        let _: SectorWire = wire(session.bytes(data, 160)?)?;
        sectors.insert(data, format!("level:/scene/{}", node.path));
    }
    Ok(sectors)
}

fn join_reference_target(
    session: &SourceSession,
    (block, module, offset): (u8, u8, u32),
    rooted_persos: &BTreeMap<Address, String>,
) -> SaveReferenceTargetJoin {
    let key = BlockKey(module, block);
    let identity = format!("{block}:{module}:{offset}");
    let mut result = SaveReferenceTargetJoin {
        identity,
        status: "no_active_source_block",
        owner: None,
        allocation_index: None,
        allocation_data_start: None,
        allocation_end: None,
        offset_in_allocation: None,
        data_prefix_hex: None,
        superobject_kind: None,
        superobject_data_identity: None,
        rooted_perso: None,
        instantiated_item_name: None,
    };
    let Some(&owner) = session.actual.get(&key) else {
        return result;
    };
    result.owner = Some(match owner {
        Owner::Fix => "fix",
        Owner::Level => "level",
    });
    let Some(data) = session
        .blocks
        .get(&owner)
        .and_then(|blocks| blocks.get(&key))
        .map(|block| &block.data)
    else {
        return result;
    };
    let offset = offset as usize;
    if offset >= data.len() {
        result.status = "outside_source_block";
        return result;
    }
    let Ok(starts) = session.allocation_indices(owner, key) else {
        result.status = "allocation_parse_failed";
        return result;
    };
    let Some((&start, &index)) = starts.range(..=offset).next_back() else {
        result.status = "outside_parsed_allocations";
        return result;
    };
    let size = u32::from_le_bytes(
        data[start - 4..start]
            .try_into()
            .expect("allocation header"),
    ) as usize
        * 4;
    let end = start - 4 + size;
    if offset >= end {
        result.status = "between_allocations";
        return result;
    }
    result.status = "bounded_allocation";
    result.allocation_index = Some(index);
    result.allocation_data_start = Some(start);
    result.allocation_end = Some(end);
    result.offset_in_allocation = Some(offset - start);
    result.data_prefix_hex = Some(
        data[start..end.min(start + 64)]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    );
    if offset == start && end - start == 64 {
        let a = Address { owner, key, offset };
        if let Ok(header) = wire::<SuperObjectWire>(&data[start..end])
            && matches!(header.kind, 4 | 0x12)
        {
            result.superobject_kind = Some(header.kind);
            if header.kind == 4 {
                result.rooted_perso = rooted_persos.get(&a).cloned();
            }
            if let Some(data) = session.pointer(a, 4).ok().flatten() {
                result.superobject_data_identity =
                    Some(format!("{}:{}:{}", data.key.1, data.key.0, data.offset));
                if header.kind == 0x12
                    && data.owner == Owner::Fix
                    && data.key == BlockKey(10, 0)
                    && let Ok(starts) = session.allocation_indices(data.owner, data.key)
                    && let Some((&start, _)) = starts.range(..=data.offset).next_back()
                {
                    let block = &session.blocks[&data.owner][&data.key];
                    let size = u32::from_le_bytes(
                        block.data[start - 4..start]
                            .try_into()
                            .expect("allocation header"),
                    ) as usize
                        * 4;
                    if start == data.offset {
                        result.instantiated_item_name =
                            inventory_handle_name(session, start, start - 4 + size);
                    }
                }
            }
        }
    }
    result
}

pub fn join_save_allocations(
    game: &Path,
    selection: &Selection,
    records: &[SaveRecordLocation],
) -> Result<SaveJoinResult> {
    let session = SourceSession::open(game, selection)?;
    let (designer_fields, mut designer_discovery) = designer_candidates(&session)?;
    let standard_fields = standard_save_candidates(&session, records)?;
    let standard_hitpoint_fields = standard_hitpoint_candidates(&session)?;
    let sector_character_fields = sector_character_candidates(&session)?;
    let sectors = rooted_sector_data(&session)?;
    let mut rooted_persos = BTreeMap::<Address, String>::new();
    for node in session.tree()? {
        let header: SuperObjectWire = wire(session.bytes(node.address, 64)?)?;
        if header.kind == 4 {
            rooted_persos.insert(node.address, format!("level:/persos/{}", node.path));
        }
    }
    let inventory_items = inventory_item_names(&session);
    let gauge_fields = gauge_save_fields(&session)?;
    let mut matched_designer_addresses = BTreeSet::new();
    let mut allocation_cache =
        BTreeMap::<(Owner, BlockKey), std::result::Result<BTreeMap<usize, u32>, String>>::new();
    let mut joins = Vec::with_capacity(records.len());
    for record in records {
        let key = BlockKey(record.module, record.block);
        let mut candidates = Vec::new();
        for (owner, label) in [(Owner::Fix, "fix"), (Owner::Level, "level")] {
            let Some(block) = session
                .blocks
                .get(&owner)
                .and_then(|blocks| blocks.get(&key))
            else {
                continue;
            };
            let offset = record.offset as usize;
            let Some(end) = offset.checked_add(record.live_width) else {
                continue;
            };
            if end > block.data.len() {
                continue;
            }
            let index = allocation_cache.entry((owner, key)).or_insert_with(|| {
                session
                    .allocation_indices(owner, key)
                    .map_err(|error| format!("{error:#}"))
            });
            let (allocation_index, allocation_data_start, allocation_end, allocation_issue) =
                match index {
                    Ok(starts) => {
                        if let Some((&start, &index)) = starts.range(..=offset).next_back() {
                            let header = start - 4;
                            let size = u32::from_le_bytes(
                                block.data[header..start]
                                    .try_into()
                                    .expect("four-byte header"),
                            ) as usize
                                * 4;
                            let end_of_allocation = header + size;
                            if end <= end_of_allocation {
                                (Some(index), Some(start), Some(end_of_allocation), None)
                            } else {
                                (
                                    None,
                                    None,
                                    None,
                                    Some("record crosses allocation boundary".into()),
                                )
                            }
                        } else {
                            (
                                None,
                                None,
                                None,
                                Some("record is outside parsed allocations".into()),
                            )
                        }
                    }
                    Err(error) => (None, None, None, Some(format!("allocation parse: {error}"))),
                };
            let (mut source_object, mut source_field, mut source_meaning, mut source_rule) =
                if owner == Owner::Fix && key == BlockKey(10, 0) {
                    if let (Some(start), Some(end)) = (allocation_data_start, allocation_end) {
                        if let Some(object) = inventory_handle_name(&session, start, end) {
                            let relative = offset - start;
                            let (field, meaning, rule) = if relative == 8
                                && record.kind == "0x01"
                                && record.flags == "0x0204"
                            {
                                (
                                    Some("handle.selected".into()),
                                    Some("whether this inventory handle is selected".into()),
                                    Some("hype-inventory-handle-selected-v1"),
                                )
                            } else if relative == 10 && record.kind == "0x02" {
                                (Some("handle+0x0a".into()), None, None)
                            } else {
                                (None, None, None)
                            };
                            (Some(object), field, meaning, rule)
                        } else {
                            (None, None, None, None)
                        }
                    } else {
                        (None, None, None, None)
                    }
                } else {
                    (None, None, None, None)
                };
            let mut source_type = None;
            let mut source_appearance = None;
            let mut field_match_ambiguity = Vec::new();
            if owner == Owner::Fix
                && key == BlockKey(5, 0)
                && record.kind == "0x01"
                && record.flags == "0x0204"
                && record.live_width == 1
                && let Some(field) = gauge_fields.get(&offset)
            {
                source_object = Some(field.object.clone());
                source_field = Some(field.field.clone());
                source_meaning = field.meaning.clone();
                source_rule = Some("hype-gauge-pointer-chain-v1");
                source_type = Some("u8".into());
            }
            if owner == Owner::Fix
                && key == BlockKey(5, 0)
                && record.kind == "0x11"
                && record.flags == "0x0204"
                && record.live_width == 4
                && allocation_data_start
                    .zip(allocation_end)
                    .is_some_and(|(start, end)| offset == start && end - start == 40)
                && let Some(field) = gauge_fields.get(&offset)
                && field.field == "GaugeRoot.next_root"
            {
                let root = Address { owner, key, offset };
                let expected = session.pointer(root, 0)?;
                let target_matches = match (expected, record.reference_target) {
                    (Some(next), Some((0, 5, target_offset))) => {
                        next.owner == Owner::Fix
                            && next.key == key
                            && next.offset == target_offset as usize
                    }
                    (None, Some((255, 255, 0))) => true,
                    _ => false,
                };
                if target_matches {
                    source_object = Some(field.object.clone());
                    source_field = Some(field.field.clone());
                    source_meaning = field.meaning.clone();
                    source_rule = Some("hype-gauge-root-link-v1");
                    source_type = Some("gauge-root reference".into());
                }
            }
            if owner == Owner::Fix
                && key == BlockKey(10, 0)
                && record.kind == "0x02"
                && record.flags == "0x0204"
                && record.live_width == 2
                && allocation_data_start
                    .zip(allocation_end)
                    .is_some_and(|(start, end)| end - start == 32 && offset == start + 20)
                && let Some(start) = allocation_data_start
            {
                let item = Address {
                    owner,
                    key,
                    offset: start,
                };
                if let Some(names) = inventory_items.get(&item) {
                    if names.len() == 1 {
                        source_object = Some(names[0].clone());
                        source_field = Some("item.max_element_capacity".into());
                        source_meaning = Some(
                            "maximum element capacity used to clamp inventory item additions"
                                .into(),
                        );
                        source_rule = Some("hype-inventory-item-max-capacity-v1");
                        source_type = Some("16-bit maximum".into());
                    } else if names.len() > 1 {
                        field_match_ambiguity = names.clone();
                    }
                }
            }
            if source_object.is_none()
                && record.kind == "0x01"
                && record.flags == "0x0204"
                && record.live_width == 1
            {
                let address = Address { owner, key, offset };
                if let Some(actors) = standard_hitpoint_fields.get(&address) {
                    let relative = allocation_data_start.map(|start| offset - start);
                    let matches_allocation = allocation_data_start
                        .zip(allocation_end)
                        .is_some_and(|(start, end)| end - start == 56 && offset < end)
                        && matches!(relative, Some(24..=26));
                    if matches_allocation && actors.len() == 1 {
                        let index = relative.unwrap() - 24;
                        let (field, meaning) = match index {
                            0 => (
                                "StandardGame.HitPointsInit",
                                "script-authored initial hit points value; runtime current-HP byte",
                            ),
                            1 => (
                                "StandardGame.HitPointsMaxInit",
                                "script-authored initial maximum; runtime mutable HP ceiling",
                            ),
                            _ => (
                                "StandardGame.HitPointsMaxMax",
                                "script-authored hard maximum for the runtime HP ceiling",
                            ),
                        };
                        source_object = Some(actors[0].0.clone());
                        source_appearance = actors[0].1.clone();
                        source_field = Some(field.into());
                        source_meaning = Some(meaning.into());
                        source_rule = Some("hype-standard-hitpoints-v1");
                        source_type = Some("u8".into());
                    } else if matches_allocation && actors.len() > 1 {
                        field_match_ambiguity =
                            actors.iter().map(|actor| actor.0.clone()).collect();
                    }
                }
            }
            if source_object.is_none() && record.kind == "0x04" && record.flags == "0x0204" {
                let address = Address { owner, key, offset };
                if let Some(actors) = standard_fields.get(&address) {
                    let matches_allocation = allocation_data_start
                        .zip(allocation_end)
                        .is_some_and(|(start, end)| start + 44 == offset && end - start == 56);
                    if matches_allocation && actors.len() == 1 && record.live_width == 4 {
                        source_object = Some(actors[0].0.clone());
                        source_appearance = actors[0].1.clone();
                        source_field = Some("StandardGame.CustomBitsInit".into());
                        source_rule = Some("hype-standard-custom-bits-init-v1");
                        source_type =
                            Some("u32 bitfield; individual bit meanings incomplete".into());
                    } else if matches_allocation && actors.len() > 1 {
                        field_match_ambiguity =
                            actors.iter().map(|actor| actor.0.clone()).collect();
                    }
                }
            }
            if source_object.is_none() && record.kind == "0x01" && record.flags == "0x0a0c" {
                let address = Address { owner, key, offset };
                if let Some(actors) = standard_fields.get(&address) {
                    let relative = allocation_data_start.map(|start| offset - start);
                    let matches_allocation = allocation_data_start
                        .zip(allocation_end)
                        .is_some_and(|(start, end)| end - start == 56 && offset < end)
                        && matches!(relative, Some(27 | 28));
                    if matches_allocation && actors.len() == 1 && record.live_width == 1 {
                        source_object = Some(actors[0].0.clone());
                        source_appearance = actors[0].1.clone();
                        source_rule = Some("hype-standard-neverback-activation-v1");
                        source_type = Some("u8 state flag".into());
                        if relative == Some(27) {
                            source_field = Some("StandardGame.never_back_latch".into());
                            source_meaning = Some(
                                "nonzero suppresses this NeverBack actor's return and forces its active byte to zero"
                                    .into(),
                            );
                        } else {
                            source_field = Some("StandardGame.active".into());
                            source_meaning = Some(
                                "active state for this NeverBack actor, gated by its suppression latch"
                                    .into(),
                            );
                        }
                    } else if matches_allocation && actors.len() > 1 {
                        field_match_ambiguity =
                            actors.iter().map(|actor| actor.0.clone()).collect();
                    }
                }
            }
            if source_object.is_none() && record.kind == "0x11" && record.flags == "0x0204" {
                let address = Address { owner, key, offset };
                if let Some(actors) = sector_character_fields.get(&address) {
                    let matches_allocation = allocation_data_start
                        .zip(allocation_end)
                        .is_some_and(|(start, end)| start + 4 == offset && end - start == 24);
                    if matches_allocation && actors.len() == 1 && record.live_width == 4 {
                        source_object = Some(actors[0].0.clone());
                        source_appearance = actors[0].1.clone();
                        source_field = Some("SectorCharacterList.sector_link".into());
                        if let Some((2, 6, target_offset)) = record.reference_target {
                            let element = Address {
                                owner: Owner::Level,
                                key: BlockKey(6, 2),
                                offset: target_offset as usize,
                            };
                            if session.allocation_index(element).is_ok() {
                                let sector = session.pointer(element, 4)?;
                                source_meaning = sector
                                    .and_then(|sector| sectors.get(&sector))
                                    .map(|path| format!("actor's assigned sector-list element resolves to {path}"));
                            }
                        }
                        source_rule = Some("hype-character-sector-link-v1");
                        source_type = Some("sector-list element reference".into());
                    } else if matches_allocation && actors.len() > 1 {
                        field_match_ambiguity =
                            actors.iter().map(|(actor, _)| actor.clone()).collect();
                    }
                }
            }
            if source_object.is_none() {
                let address = Address { owner, key, offset };
                if let Some(fields) = designer_fields.get(&address) {
                    let flags = u32::from_str_radix(record.flags.trim_start_matches("0x"), 16)
                        .unwrap_or_default();
                    let matching = fields
                        .iter()
                        .filter(|field| {
                            field.extent == record.live_width
                                && field.save & 0x0f00 == flags & 0x0f00
                        })
                        .collect::<Vec<_>>();
                    if matching.len() == 1 {
                        let field = matching[0];
                        matched_designer_addresses.insert(address);
                        source_object = Some(field.object.clone());
                        source_appearance = field.appearance.clone();
                        source_field = Some(field.field.clone());
                        source_rule = Some("hype-dsg-descriptor-v1");
                        source_type = Some(format!("designer type {}", field.kind));
                    } else if matching.len() > 1 {
                        field_match_ambiguity = matching
                            .iter()
                            .map(|field| format!("{} {}", field.object, field.field))
                            .collect();
                    }
                }
            }
            candidates.push(SaveAllocationCandidate {
                owner: label,
                block_size: block.data.len(),
                allocation_index,
                allocation_data_start,
                allocation_end,
                allocation_issue,
                data_prefix_hex: if record.kind == "0x11" {
                    allocation_data_start
                        .zip(allocation_end)
                        .map(|(start, end)| {
                            block.data[start..end.min(start + 64)]
                                .iter()
                                .map(|byte| format!("{byte:02x}"))
                                .collect()
                        })
                } else {
                    None
                },
                source_object,
                source_appearance,
                source_field,
                source_meaning,
                source_rule,
                source_type,
                field_match_ambiguity,
            });
        }
        let status = match candidates.len() {
            0 => "no_bounded_source_block",
            1 if candidates[0].allocation_index.is_some() => "single_bounded_allocation",
            1 => "single_block_allocation_unresolved",
            _ => "ambiguous_owner",
        };
        joins.push(SaveAllocationJoin {
            id: record.id.clone(),
            status,
            candidates,
            reference_target: record
                .reference_target
                .map(|target| join_reference_target(&session, target, &rooted_persos)),
        });
    }
    designer_discovery.matched_saved_descriptors = matched_designer_addresses.len();
    designer_discovery.unmatched_saved_descriptors = designer_fields
        .keys()
        .filter(|address| !matched_designer_addresses.contains(address))
        .map(|address| {
            format!(
                "{:?}:{}:{}:{}",
                address.owner, address.key.1, address.key.0, address.offset
            )
        })
        .collect();
    Ok(SaveJoinResult {
        joins,
        designer_discovery,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::ensure;

    #[test]
    #[ignore = "requires supplied Hype source corpus; selected by --local"]
    fn source_save_join_checks_inventory_designer_and_allocation_bounds() -> Result<()> {
        let game = std::env::var_os("ASTROLABE_GAME_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "disc".into());
        let selection = Selection::hype("manoir", "en")?;
        let records = [
            SaveRecordLocation {
                id: "0:10:1560".into(),
                block: 0,
                module: 10,
                offset: 1560,
                live_width: 1,
                kind: "0x01".into(),
                flags: "0x0204".into(),
                reference_target: None,
            },
            SaveRecordLocation {
                id: "0:17:297732".into(),
                block: 0,
                module: 17,
                offset: 297732,
                live_width: 1,
                kind: "0x10".into(),
                flags: "0x0204".into(),
                reference_target: None,
            },
            SaveRecordLocation {
                id: "1:17:187072".into(),
                block: 1,
                module: 17,
                offset: 187072,
                live_width: 1,
                kind: "0x10".into(),
                flags: "0x0204".into(),
                reference_target: None,
            },
            SaveRecordLocation {
                id: "crosses-allocation".into(),
                block: 0,
                module: 10,
                offset: 1575,
                live_width: 2,
                kind: "0x02".into(),
                flags: "0x0204".into(),
                reference_target: None,
            },
            SaveRecordLocation {
                id: "missing-module".into(),
                block: 0,
                module: 99,
                offset: 0,
                live_width: 1,
                kind: "0x01".into(),
                flags: "0x0204".into(),
                reference_target: None,
            },
            SaveRecordLocation {
                id: "1:5:1041160".into(),
                block: 1,
                module: 5,
                offset: 1041160,
                live_width: 4,
                kind: "0x04".into(),
                flags: "0x0204".into(),
                reference_target: None,
            },
            SaveRecordLocation {
                id: "0:17:4721".into(),
                block: 0,
                module: 17,
                offset: 4721,
                live_width: 1,
                kind: "0x10".into(),
                flags: "0x0204".into(),
                reference_target: None,
            },
            SaveRecordLocation {
                id: "0:5:1632".into(),
                block: 0,
                module: 5,
                offset: 1632,
                live_width: 4,
                kind: "0x11".into(),
                flags: "0x0204".into(),
                reference_target: Some((2, 6, 553948)),
            },
            SaveRecordLocation {
                id: "sector-target-unresolved".into(),
                block: 0,
                module: 5,
                offset: 1632,
                live_width: 4,
                kind: "0x11".into(),
                flags: "0x0204".into(),
                reference_target: Some((2, 6, 999999)),
            },
            SaveRecordLocation {
                id: "1:6:4441632".into(),
                block: 1,
                module: 6,
                offset: 4441632,
                live_width: 4,
                kind: "0x11".into(),
                flags: "0x0204".into(),
                reference_target: Some((1, 6, 4455340)),
            },
            SaveRecordLocation {
                id: "1:5:1041143".into(),
                block: 1,
                module: 5,
                offset: 1041143,
                live_width: 1,
                kind: "0x01".into(),
                flags: "0x0a0c".into(),
                reference_target: None,
            },
            SaveRecordLocation {
                id: "1:5:1041144".into(),
                block: 1,
                module: 5,
                offset: 1041144,
                live_width: 1,
                kind: "0x01".into(),
                flags: "0x0a0c".into(),
                reference_target: None,
            },
            SaveRecordLocation {
                id: "1:5:1205048".into(),
                block: 1,
                module: 5,
                offset: 1205048,
                live_width: 1,
                kind: "0x01".into(),
                flags: "0x0a0c".into(),
                reference_target: None,
            },
            SaveRecordLocation {
                id: "0:5:1343".into(),
                block: 0,
                module: 5,
                offset: 1343,
                live_width: 1,
                kind: "0x01".into(),
                flags: "0x0a0c".into(),
                reference_target: None,
            },
            SaveRecordLocation {
                id: "0:10:132".into(),
                block: 0,
                module: 10,
                offset: 132,
                live_width: 2,
                kind: "0x02".into(),
                flags: "0x0204".into(),
                reference_target: None,
            },
            SaveRecordLocation {
                id: "0:10:5790".into(),
                block: 0,
                module: 10,
                offset: 5790,
                live_width: 2,
                kind: "0x02".into(),
                flags: "0x0204".into(),
                reference_target: None,
            },
            SaveRecordLocation {
                id: "0:10:5828".into(),
                block: 0,
                module: 10,
                offset: 5828,
                live_width: 2,
                kind: "0x02".into(),
                flags: "0x0204".into(),
                reference_target: None,
            },
            SaveRecordLocation {
                id: "0:5:4543492".into(),
                block: 0,
                module: 5,
                offset: 4543492,
                live_width: 4,
                kind: "0x04".into(),
                flags: "0x0204".into(),
                reference_target: None,
            },
            SaveRecordLocation {
                id: "0:5:4972224".into(),
                block: 0,
                module: 5,
                offset: 4972224,
                live_width: 4,
                kind: "0x04".into(),
                flags: "0x0204".into(),
                reference_target: None,
            },
            SaveRecordLocation {
                id: "0:17:4420".into(),
                block: 0,
                module: 17,
                offset: 4420,
                live_width: 4,
                kind: "0x11".into(),
                flags: "0x0204".into(),
                reference_target: Some((2, 6, 2170476)),
            },
        ];
        let report = join_save_allocations(&game, &selection, &records)?;
        let joined = report.joins;
        let session = SourceSession::open(&game, &selection)?;
        let inventory_items = inventory_item_names(&session);
        ensure!(inventory_items.len() == 63);
        ensure!(joined[0].status == "single_bounded_allocation");
        ensure!(
            joined[0].candidates[0].source_object.as_deref() == Some("Fix inventory MagicIce 54")
        );
        ensure!(joined[0].candidates[0].source_field.as_deref() == Some("handle.selected"));
        ensure!(
            joined[1].candidates[0].source_object.as_deref() == Some("level:/persos/world/0/27")
        );
        ensure!(joined[1].candidates[0].source_field.as_deref() == Some("DSG[8]"));
        ensure!(joined[1].candidates[0].source_type.as_deref() == Some("designer type 0"));
        ensure!(
            joined[2].candidates[0].source_object.as_deref() == Some("level:/persos/world/0/174")
        );
        ensure!(joined[2].candidates[0].source_field.as_deref() == Some("DSG[1]"));
        ensure!(joined[3].status == "single_block_allocation_unresolved");
        ensure!(joined[4].status == "no_bounded_source_block");
        ensure!(joined[5].status == "single_bounded_allocation");
        ensure!(
            joined[5].candidates[0]
                .source_object
                .as_deref()
                .is_some_and(|object| object.starts_with("level:/persos/"))
        );
        ensure!(
            joined[5].candidates[0].source_field.as_deref() == Some("StandardGame.CustomBitsInit")
        );
        ensure!(joined[6].candidates[0].source_object.as_deref() == Some("fix:/persos/15"));
        ensure!(
            joined[6].candidates[0].source_appearance.as_deref() == Some("level:/persos/world/0/1")
        );
        ensure!(joined[6].candidates[0].source_field.as_deref() == Some("DSG[0]"));
        ensure!(joined[7].candidates[0].source_object.as_deref() == Some("fix:/persos/15"));
        ensure!(
            joined[7].candidates[0].source_appearance.as_deref() == Some("level:/persos/world/0/1")
        );
        ensure!(
            joined[7].candidates[0].source_meaning.as_deref()
                == Some("actor's assigned sector-list element resolves to level:/scene/world/1/28")
        );
        ensure!(joined[8].candidates[0].source_meaning.is_none());
        ensure!(joined[9].candidates[0].source_object.is_none());
        ensure!(
            joined[9]
                .reference_target
                .as_ref()
                .unwrap()
                .superobject_kind
                == Some(0x12)
        );
        ensure!(
            joined[9]
                .reference_target
                .as_ref()
                .unwrap()
                .instantiated_item_name
                .as_deref()
                == Some("Fix inventory NotFound 50")
        );
        let checkpoint_target = joined.last().unwrap().reference_target.as_ref().unwrap();
        ensure!(checkpoint_target.superobject_kind == Some(4));
        ensure!(checkpoint_target.rooted_perso.as_deref() == Some("level:/persos/world/0/35"));
        ensure!(
            joined[10].candidates[0].source_field.as_deref()
                == Some("StandardGame.never_back_latch")
        );
        ensure!(joined[11].candidates[0].source_field.as_deref() == Some("StandardGame.active"));
        ensure!(joined[10].candidates[0].source_object == joined[11].candidates[0].source_object);
        ensure!(joined[10].candidates[0].source_meaning.is_some());
        ensure!(joined[11].candidates[0].source_meaning.is_some());
        ensure!(joined[12].candidates[0].source_object.is_none());
        ensure!(joined[13].candidates[0].source_object.as_deref() == Some("fix:/persos/15"));
        ensure!(
            joined[13].candidates[0].source_appearance.as_deref()
                == Some("level:/persos/world/0/1")
        );
        ensure!(
            joined[14].candidates[0].source_field.as_deref() == Some("item.max_element_capacity")
        );
        ensure!(
            joined[14].candidates[0].source_rule == Some("hype-inventory-item-max-capacity-v1")
        );
        ensure!(
            joined[14].candidates[0].source_object.as_deref() == Some("Fix inventory Weapon 81")
        );
        ensure!(joined[14].candidates[0].source_meaning.is_some());
        ensure!(joined[15].candidates[0].source_field.is_none());
        ensure!(joined[15].candidates[0].source_object.is_none());
        ensure!(
            joined[16].candidates[0].source_field.as_deref() == Some("item.max_element_capacity")
        );
        ensure!(
            joined[16].candidates[0].source_object.as_deref()
                == Some("Fix inventory item (member name unavailable)")
        );
        ensure!(joined[17].status == "single_bounded_allocation");
        ensure!(joined[17].candidates.len() == 1);
        ensure!(joined[17].candidates[0].source_object.as_deref() == Some("fix:/persos/5-0-24465"));
        ensure!(
            joined[17].candidates[0].source_field.as_deref() == Some("StandardGame.CustomBitsInit")
        );
        ensure!(joined[17].candidates[0].field_match_ambiguity.is_empty());
        ensure!(joined[18].status == "single_bounded_allocation");
        ensure!(joined[18].candidates.len() == 1);
        ensure!(joined[18].candidates[0].source_object.as_deref() == Some("fix:/persos/5-0-27269"));
        ensure!(
            joined[18].candidates[0].source_field.as_deref() == Some("StandardGame.CustomBitsInit")
        );
        ensure!(joined[18].candidates[0].field_match_ambiguity.is_empty());
        Ok(())
    }

    #[test]
    #[ignore = "requires supplied Hype source corpus; selected by --local"]
    fn source_save_join_maps_only_registered_standard_hitpoint_bytes() -> Result<()> {
        let game = std::env::var_os("ASTROLABE_GAME_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "disc".into());
        let selection = Selection::hype("manoir", "en")?;
        let session = SourceSession::open(&game, &selection)?;
        let fields = standard_hitpoint_candidates(&session)?;
        let total: usize = fields.values().map(Vec::len).sum();
        ensure!(
            total == 84,
            "expected 28 rooted policy-4 actors × 3 bytes; got {total}"
        );

        let mut expected_by_owner = BTreeMap::<Owner, usize>::new();
        let mut records = Vec::new();
        for (address, actors) in &fields {
            ensure!(
                actors.len() == 1,
                "ambiguous rooted StandardGame field at {address:?}"
            );
            *expected_by_owner.entry(address.owner).or_default() += 1;
            records.push(SaveRecordLocation {
                id: format!(
                    "fixture:{}:{}:{}",
                    address.owner as u8, address.key.0, address.offset
                ),
                block: address.key.1,
                module: address.key.0,
                offset: u32::try_from(address.offset)?,
                live_width: 1,
                kind: "0x01".into(),
                flags: "0x0204".into(),
                reference_target: None,
            });
        }
        ensure!(expected_by_owner.get(&Owner::Level) == Some(&78));
        ensure!(expected_by_owner.get(&Owner::Fix) == Some(&6));

        let sample = *fields.keys().next().expect("nonempty hitpoint field map");
        for (id, kind, flags, width) in [
            ("wrong-kind", "0x02", "0x0204", 1),
            ("wrong-flags", "0x01", "0x0a0c", 1),
            ("wrong-width", "0x01", "0x0204", 2),
            ("wrong-offset", "0x01", "0x0204", 1),
        ] {
            records.push(SaveRecordLocation {
                id: id.into(),
                block: sample.key.1,
                module: sample.key.0,
                offset: u32::try_from(sample.offset - u32::from(id == "wrong-offset") as usize)?,
                live_width: width,
                kind: kind.into(),
                flags: flags.into(),
                reference_target: None,
            });
        }
        let joined = join_save_allocations(&game, &selection, &records)?.joins;
        let mut actual_by_field = BTreeMap::<String, usize>::new();
        for join in &joined[..84] {
            let candidate = join
                .candidates
                .iter()
                .find(|candidate| candidate.source_field.is_some())
                .ok_or_else(|| anyhow::anyhow!("registered byte failed source join"))?;
            let field = candidate.source_field.as_deref().unwrap();
            *actual_by_field.entry(field.to_owned()).or_default() += 1;
            ensure!(candidate.source_type.as_deref() == Some("u8"));
            ensure!(candidate.source_rule == Some("hype-standard-hitpoints-v1"));
            ensure!(
                candidate.allocation_end.unwrap() - candidate.allocation_data_start.unwrap() == 56
            );
        }
        ensure!(actual_by_field.get("StandardGame.HitPointsInit") == Some(&28));
        ensure!(actual_by_field.get("StandardGame.HitPointsMaxInit") == Some(&28));
        ensure!(actual_by_field.get("StandardGame.HitPointsMaxMax") == Some(&28));
        for join in &joined[84..] {
            ensure!(
                join.candidates
                    .iter()
                    .all(|candidate| candidate.source_field.is_none())
            );
        }
        Ok(())
    }

    #[test]
    #[ignore = "requires supplied Hype source corpus; selected by --local"]
    fn source_save_join_maps_relocated_gauge_pointer_chain() -> Result<()> {
        let game = std::env::var_os("ASTROLABE_GAME_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "disc".into());
        let selection = Selection::hype("manoir", "en")?;
        let child_starts = [
            4_985_364, 4_985_392, 4_985_420, 4_985_448, 4_985_476, 4_985_504, 4_985_532, 4_985_728,
            4_985_756, 4_985_784, 4_985_812, 4_985_840, 4_986_036, 4_986_064, 4_986_092, 4_986_120,
            4_986_148,
        ];
        let root_starts = [4_985_156, 4_985_560, 4_985_868];
        let mut records = Vec::new();
        for (ordinal, start) in child_starts.iter().enumerate() {
            for (relative, field) in [(0x14, "phase"), (0x15, "tick")] {
                records.push(SaveRecordLocation {
                    id: format!("gauge-child-{ordinal}-{field}"),
                    block: 0,
                    module: 5,
                    offset: u32::try_from(start + relative)?,
                    live_width: 1,
                    kind: "0x01".into(),
                    flags: "0x0204".into(),
                    reference_target: None,
                });
            }
        }
        for (ordinal, start) in root_starts.iter().enumerate() {
            for (relative, field) in [(0x1e, "completed"), (0x1f, "selector")] {
                records.push(SaveRecordLocation {
                    id: format!("gauge-root-{ordinal}-{field}"),
                    block: 0,
                    module: 5,
                    offset: u32::try_from(start + relative)?,
                    live_width: 1,
                    kind: "0x01".into(),
                    flags: "0x0204".into(),
                    reference_target: None,
                });
            }
        }
        // The same-block near miss is a bounded 24-byte allocation, but no chain reaches it.
        for (id, offset, kind, flags, width) in [
            ("gauge-near-miss", 4_986_312 + 0x14, "0x01", "0x0204", 1),
            (
                "gauge-wrong-kind",
                child_starts[0] + 0x14,
                "0x02",
                "0x0204",
                1,
            ),
            (
                "gauge-wrong-flags",
                child_starts[0] + 0x14,
                "0x01",
                "0x0a0c",
                1,
            ),
            (
                "gauge-wrong-width",
                child_starts[0] + 0x14,
                "0x01",
                "0x0204",
                2,
            ),
        ] {
            records.push(SaveRecordLocation {
                id: id.into(),
                block: 0,
                module: 5,
                offset: u32::try_from(offset)?,
                live_width: width,
                kind: kind.into(),
                flags: flags.into(),
                reference_target: None,
            });
        }
        for (id, source, target, flags) in [
            (
                "gauge-next-root-0",
                root_starts[0],
                root_starts[1],
                "0x0204",
            ),
            (
                "gauge-next-root-1",
                root_starts[1],
                root_starts[2],
                "0x0204",
            ),
            (
                "gauge-wrong-root-target",
                root_starts[0],
                root_starts[2],
                "0x0204",
            ),
            (
                "gauge-wrong-root-flags",
                root_starts[0],
                root_starts[1],
                "0x0a0c",
            ),
        ] {
            records.push(SaveRecordLocation {
                id: id.into(),
                block: 0,
                module: 5,
                offset: u32::try_from(source)?,
                live_width: 4,
                kind: "0x11".into(),
                flags: flags.into(),
                reference_target: Some((0, 5, u32::try_from(target)?)),
            });
        }

        let joins = join_save_allocations(&game, &selection, &records)?.joins;
        let gauge = joins[..40]
            .iter()
            .map(|join| {
                join.candidates
                    .iter()
                    .find(|candidate| candidate.owner == "fix")
                    .ok_or_else(|| anyhow::anyhow!("missing Fix candidate for {}", join.id))
            })
            .collect::<Result<Vec<_>>>()?;
        for (index, candidate) in gauge[..34].iter().enumerate() {
            ensure!(
                candidate.source_rule == Some("hype-gauge-pointer-chain-v1")
                    && candidate.source_type.as_deref() == Some("u8")
                    && candidate.source_meaning.is_some(),
                "unmatched gauge record {}: {:?} {:?} {:?}",
                joins[index].id,
                candidate.source_object,
                candidate.source_field,
                candidate.source_rule
            );
            let (root, item) = if index < 14 {
                (0, index / 2)
            } else if index < 24 {
                (1, (index - 14) / 2)
            } else {
                (2, (index - 24) / 2)
            };
            ensure!(
                candidate.source_object.as_deref()
                    == Some(
                        format!("Fix gauge root #{root} item #{item} (source list order)").as_str()
                    ),
                "unexpected gauge source ordinal at index {index}"
            );
        }
        ensure!(
            gauge
                .iter()
                .filter(|candidate| candidate.source_meaning.is_some())
                .count()
                == 37
        );
        ensure!(
            gauge
                .iter()
                .filter(|candidate| candidate.source_field.is_some()
                    && candidate.source_meaning.is_none())
                .count()
                == 3
        );
        ensure!([34, 36, 38].iter().all(|&index| {
            let candidate = gauge[index];
            candidate.source_field.as_deref() == Some("GaugeRoot.completed_children")
                && candidate.source_meaning.is_some()
        }));
        ensure!([35, 37, 39].iter().all(|&index| {
            let candidate = gauge[index];
            candidate.source_field.as_deref() == Some("GaugeRoot.threshold_selector")
                && candidate.source_object.is_some()
                && candidate.source_meaning.is_none()
        }));
        ensure!(
            gauge[0].source_object.as_deref()
                == Some("Fix gauge root #0 item #0 (source list order)")
        );
        ensure!(
            gauge[14].source_object.as_deref()
                == Some("Fix gauge root #1 item #0 (source list order)")
        );
        ensure!(
            gauge[24].source_object.as_deref()
                == Some("Fix gauge root #2 item #0 (source list order)")
        );
        ensure!(gauge[0].source_field.as_deref() == Some("GaugeChild.visual_phase"));
        ensure!(gauge[1].source_field.as_deref() == Some("GaugeChild.progress_tick"));
        ensure!(
            gauge[34].source_object.as_deref() == Some("Fix gauge root #0 (source list order)")
        );
        ensure!(
            gauge[38].source_object.as_deref() == Some("Fix gauge root #2 (source list order)")
        );
        ensure!(joins[40..44].iter().all(|join| {
            join.candidates
                .iter()
                .all(|candidate| candidate.source_field.is_none())
        }));
        for (index, ordinal) in [(44, 0), (45, 1)] {
            let candidate = &joins[index].candidates[0];
            ensure!(candidate.source_rule == Some("hype-gauge-root-link-v1"));
            ensure!(candidate.source_field.as_deref() == Some("GaugeRoot.next_root"));
            ensure!(
                candidate.source_object.as_deref()
                    == Some(format!("Fix gauge root #{ordinal} (source list order)").as_str())
            );
            ensure!(candidate.source_meaning.is_some());
        }
        ensure!(joins[46..].iter().all(|join| {
            join.candidates
                .iter()
                .all(|candidate| candidate.source_field.is_none())
        }));
        Ok(())
    }
}
