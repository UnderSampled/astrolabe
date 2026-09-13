//! Family/state graphs with intrusive lists rebuilt from ordered useful values.
use super::{
    configuration_export::emit_configuration,
    container::{emit_relocations, encoded},
    geometry::{GeometryExport, Layout, ReferenceTarget},
    records::*,
};
use crate::model::{AssetPath, PackageRole, character::*, io::Content};
use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub struct FamilyDependencies<'a> {
    pub animations: &'a BTreeMap<AssetPath, ReferenceTarget>,
    pub physical_objects: &'a BTreeMap<AssetPath, ReferenceTarget>,
    pub actions: &'a BTreeMap<AssetPath, ReferenceTarget>,
    pub bounds: &'a BTreeMap<AssetPath, ReferenceTarget>,
    pub mechanics: &'a BTreeMap<AssetPath, ReferenceTarget>,
    /// Configuration roots emitted in another package's group.
    pub configurations: &'a BTreeMap<AssetPath, ReferenceTarget>,
}
pub struct FamiliesExport {
    pub fragment: GeometryExport,
    pub families: BTreeMap<AssetPath, ReferenceTarget>,
    pub states: BTreeMap<(AssetPath, u32), ReferenceTarget>,
    pub configurations: BTreeMap<AssetPath, ReferenceTarget>,
}

/// One Fix or Level group. The first 12 bytes are a derived family-list header.
/// Configurations emit once; list ownership is distinct from default references.
/// GPT binding and allocator/package assembly remain separate.
pub fn write_families(
    families: &[Family],
    configurations: &[ObjectList],
    deps: &FamilyDependencies<'_>,
    base: u32,
) -> Result<FamiliesExport> {
    ensure!(
        base > 1 && base.is_multiple_of(4),
        "invalid family group base"
    );
    let role = families
        .first()
        .map(|f| f.path.role())
        .or_else(|| configurations.first().map(|c| c.path.role()))
        .context("empty family group")?;
    let block = match role {
        PackageRole::Fix => BlockKey(5, 0),
        PackageRole::Level => BlockKey(5, 1),
        _ => anyhow::bail!("invalid family group role"),
    };
    let mut out = Layout {
        bytes: Vec::new(),
        ranges: Vec::new(),
        rows: Vec::new(),
        base,
        block,
    };
    let header = out.allocate("families", &encoded(&empty_list())?)?;
    let mut family_offsets = BTreeMap::new();
    let mut state_offsets = BTreeMap::new();
    let mut config_offsets = BTreeMap::new();
    let mut object_types = BTreeSet::new();
    for family in families {
        family.validate()?;
        ensure!(
            family.path.role() == role,
            "family group crosses package roles"
        );
        ensure!(
            object_types.insert(family.object_type),
            "duplicate family object-type identity"
        );
        let root = out.allocate(
            format!("{}/family", family.path),
            &encoded(&FamilyWire {
                next: 0,
                previous: 0,
                header: 0,
                index: family.object_type,
                states: empty_list(),
                default_configuration: 0,
                configurations: empty_list(),
                bounding_volume: 0,
                extra_data: 0,
                animation_bank: family.animation_bank,
                unknown: family.unknown,
            })?,
        )?;
        ensure!(
            family_offsets.insert(family.path.clone(), root).is_none(),
            "duplicate family asset"
        );
        for state in &family.states {
            let offset = out.allocate(
                format!("{}/states/{}", family.path, state.id),
                &encoded(&CharacterStateWire {
                    next: 0,
                    previous: 0,
                    header: 0,
                    animation: 0,
                    transitions: empty_list(),
                    prohibited: empty_list(),
                    next_state: 0,
                    unknown_reference: 0,
                    mechanics: 0,
                    environment_samples: 0,
                    unknown: state.unknown[..3].try_into()?,
                    speed: state.speed,
                    unknown_tail: state.unknown[3..].try_into()?,
                })?,
            )?;
            state_offsets.insert((family.path.clone(), state.id), offset);
        }
    }
    for config in configurations {
        ensure!(
            config.path.role() == role,
            "configuration group crosses package roles"
        );
        ensure!(
            !deps.configurations.contains_key(&config.path),
            "configuration already emitted by another group"
        );
        let offset = emit_configuration(
            &mut out,
            config,
            deps.physical_objects,
            deps.actions,
            &format!("{}/", config.path),
        )?;
        ensure!(
            config_offsets.insert(config.path.clone(), offset).is_none(),
            "duplicate configuration asset"
        );
    }
    out.linked_list(
        header,
        &families
            .iter()
            .map(|f| family_offsets[&f.path])
            .collect::<Vec<_>>(),
        0,
    )?;
    let mut owned_configs = BTreeSet::new();
    for family in families {
        let root = family_offsets[&family.path];
        let states = family
            .states
            .iter()
            .map(|s| state_offsets[&(family.path.clone(), s.id)])
            .collect::<Vec<_>>();
        out.linked_list(root + 16, &states, 0)?;
        let mut configs = Vec::new();
        for reference in &family.configurations {
            ensure!(
                owned_configs.insert(reference.path.clone()),
                "configuration belongs to multiple intrusive family lists"
            );
            configs.push(
                *config_offsets
                    .get(&reference.path)
                    .context("family owns a configuration outside this group")?,
            );
        }
        out.linked_list(root + 32, &configs, 0)?;
        if let Some(reference) = &family.default_configuration {
            if let Some(offset) = config_offsets.get(&reference.path) {
                out.pointer(root + 28, *offset)?;
            } else {
                out.external_pointer(
                    root + 28,
                    *deps
                        .configurations
                        .get(&reference.path)
                        .context("family default configuration missing")?,
                )?;
            }
        }
        if let Some(reference) = &family.bounding_geometry {
            out.external_pointer(
                root + 44,
                *deps
                    .bounds
                    .get(&reference.path)
                    .context("family bounding geometry missing")?,
            )?;
        }
        for state in &family.states {
            let offset = state_offsets[&(family.path.clone(), state.id)];
            if let Some(reference) = &state.mechanics {
                out.external_pointer(
                    offset + 48,
                    *deps
                        .mechanics
                        .get(&reference.path)
                        .context("state mechanics missing")?,
                )?;
            }
            if let Some(reference) = &state.animation {
                out.external_pointer(
                    offset + 12,
                    *deps
                        .animations
                        .get(&reference.path)
                        .context("state animation missing")?,
                )?;
            }
            if let Some(id) = state.next {
                out.pointer(offset + 40, state_offsets[&(family.path.clone(), id)])?;
            }
            let mut entries = Vec::new();
            for (i, transition) in state.transitions.iter().enumerate() {
                let entry = out.allocate(
                    format!("{}/states/{}/transitions/{i}", family.path, state.id),
                    &encoded(&StateTransitionWire {
                        next: 0,
                        previous: 0,
                        header: 0,
                        target: 0,
                        via: 0,
                        linking_type: transition.linking_type,
                        unknown: transition.unknown,
                    })?,
                )?;
                out.pointer(
                    entry + 12,
                    state_offsets[&(family.path.clone(), transition.target)],
                )?;
                out.pointer(
                    entry + 16,
                    state_offsets[&(family.path.clone(), transition.via)],
                )?;
                entries.push(entry);
            }
            out.linked_list(offset + 16, &entries, 0)?;
            entries.clear();
            for (i, &id) in state.prohibited.iter().enumerate() {
                let entry = out.allocate(
                    format!("{}/states/{}/prohibited/{i}", family.path, state.id),
                    &encoded(&ProhibitedStateWire {
                        next: 0,
                        previous: 0,
                        header: 0,
                        state: 0,
                    })?,
                )?;
                out.pointer(entry + 12, state_offsets[&(family.path.clone(), id)])?;
                entries.push(entry);
            }
            out.linked_list(offset + 28, &entries, 0)?;
        }
    }
    base.checked_add(out.bytes.len().try_into()?)
        .context("family group address overflow")?;
    let target = |offset: usize| -> Result<ReferenceTarget> {
        Ok(ReferenceTarget {
            address: base
                .checked_add(offset.try_into()?)
                .context("family target overflow")?,
            block: (block.0, block.1),
        })
    };
    let families = family_offsets
        .into_iter()
        .map(|(p, o)| Ok((p, target(o)?)))
        .collect::<Result<_>>()?;
    let states = state_offsets
        .into_iter()
        .map(|(p, o)| Ok((p, target(o)?)))
        .collect::<Result<_>>()?;
    let configurations = config_offsets
        .into_iter()
        .map(|(p, o)| Ok((p, target(o)?)))
        .collect::<Result<_>>()?;
    out.rows.sort_by_key(|r| r.value);
    Ok(FamiliesExport { families,states,configurations,fragment:GeometryExport {
        bytes:out.bytes,rtb:emit_relocations(block,&out.rows)?,ranges:out.ranges,pointer_count:out.rows.len(),
        coverage:vec!["Family/state headers, ordered intrusive lists, transitions, prohibited/next-state links, configurations, shared animation, mechanics-card and bounding-geometry links rebuilt from values. Non-null extra family data, unclassified state +44 references and environment-sample lists remain unsupported. GPT binding, allocator-compatible package assembly and complete Perso records remain pending.".into()],
    }})
}
fn empty_list() -> ListWire {
    ListWire {
        first: 0,
        last: 0,
        count: 0,
    }
}
