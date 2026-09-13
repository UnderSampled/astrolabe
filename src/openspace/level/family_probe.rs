use super::{
    container::{u32_at, wire},
    session::SourceSession,
    *,
};
use crate::model::{AssetPath, PackageRole, io::Content};
use anyhow::{Context, Result};
use std::collections::BTreeMap;

pub(super) fn check(session: &SourceSession, value: &SceneImport) -> Result<()> {
    let mut offsets = BTreeMap::new();
    for (owner, key) in [
        (Owner::Fix, BlockKey(5, 0)),
        (Owner::Fix, BlockKey(6, 1)),
        (Owner::Level, BlockKey(5, 1)),
        (Owner::Level, BlockKey(6, 2)),
    ] {
        offsets.insert(
            (owner, key),
            session
                .allocation_indices(owner, key)?
                .into_iter()
                .map(|(offset, id)| (id, offset))
                .collect::<BTreeMap<_, _>>(),
        );
    }
    let address = |path: &AssetPath, module| -> Result<Address> {
        let owner = if path.role() == PackageRole::Fix {
            Owner::Fix
        } else {
            Owner::Level
        };
        let key = BlockKey(
            module,
            match (module, owner) {
                (5, Owner::Fix) => 0,
                (5, Owner::Level) | (6, Owner::Fix) => 1,
                _ => 2,
            },
        );
        let id: u32 = path.file().rsplit('/').next().unwrap().parse()?;
        Ok(Address {
            owner,
            key,
            offset: *offsets[&(owner, key)].get(&id).with_context(|| {
                format!("missing source allocation {id} for {path} in {owner:?} {key:?}")
            })?,
        })
    };
    let target = |path: &AssetPath, module| -> Result<ReferenceTarget> {
        let a = address(path, module)?;
        Ok(ReferenceTarget {
            address: session.block(a)?.base + a.offset as u32,
            block: (a.key.0, a.key.1),
        })
    };
    let collect = |paths: Vec<AssetPath>, module| -> Result<BTreeMap<_, _>> {
        paths
            .into_iter()
            .map(|p| Ok((p.clone(), target(&p, module)?)))
            .collect()
    };
    let clips = collect(value.animations.iter().map(|v| v.path.clone()).collect(), 5)?;
    let configs = collect(
        value
            .configurations
            .iter()
            .map(|v| v.path.clone())
            .collect(),
        5,
    )?;
    let actions = collect(value.actions.iter().map(|v| v.path.clone()).collect(), 5)?;
    let models = collect(
        value
            .configurations
            .iter()
            .flat_map(|c| c.all_parts())
            .filter_map(|p| p.instance.as_ref())
            .map(|r| r.path.clone())
            .collect(),
        6,
    )?;
    let bounds = collect(
        value
            .families
            .iter()
            .filter_map(|f| f.bounding_geometry.as_ref())
            .map(|r| r.path.clone())
            .collect(),
        6,
    )?;
    let mechanics = collect(value.mechanics.iter().map(|v| v.path.clone()).collect(), 6)?;
    assert_eq!(bounds.len(), 18);
    assert_eq!(mechanics.len(), 10);
    for profile in &value.mechanics {
        let a = address(&profile.path, 6)?;
        let length = u32_at(&session.block(a)?.data, a.offset - 4)? as usize * 4 - 4;
        let original = session.bytes(a, length)?;
        assert_eq!(write_mechanics(profile)?, original);
        assert_eq!(read_mechanics(original, profile.path.clone())?, *profile);
        assert_eq!(
            crate::rete::decode::<crate::model::mechanics::Mechanics>(&crate::rete::encode(
                profile
            )?)?,
            *profile
        );
    }
    let mut source_families = BTreeMap::new();
    for node in session.tree()? {
        if u32_at(session.bytes(node.address, 4)?, 0)? != 4 {
            continue;
        }
        let a = session.required(session.required(session.required(node.address, 4)?, 0)?, 20)?;
        source_families.insert(u32_at(session.bytes(a, 16)?, 12)?, a);
    }
    let mut state_count = 0;
    let mut transition_count = 0;
    for role in [PackageRole::Fix, PackageRole::Level] {
        let families = value
            .families
            .iter()
            .filter(|f| f.path.role() == role)
            .cloned()
            .collect::<Vec<_>>();
        let owned = value
            .configurations
            .iter()
            .filter(|f| f.path.role() == role)
            .cloned()
            .collect::<Vec<_>>();
        let external = configs
            .iter()
            .filter(|(p, _)| p.role() != role)
            .map(|(p, t)| (p.clone(), *t))
            .collect();
        let base = 0x70000000;
        let out = write_families(
            &families,
            &owned,
            &FamilyDependencies {
                animations: &clips,
                physical_objects: &models,
                actions: &actions,
                bounds: &bounds,
                mechanics: &mechanics,
                configurations: &external,
            },
            base,
        )?;
        assert_eq!(list(&out.fragment.bytes, 0, base)?.len(), families.len());
        for family in &families {
            family.validate()?;
            let a = source_families[&family.object_type];
            let src: FamilyWire = wire(session.bytes(a, 56)?)?;
            let root = (out.families[&family.path].address - base) as usize;
            let written: FamilyWire = wire(&out.fragment.bytes[root..root + 56])?;
            assert_eq!(written.index, src.index);
            assert_eq!(written.bounding_volume, src.bounding_volume);
            assert_eq!(written.animation_bank, src.animation_bank);
            assert_eq!(written.unknown, src.unknown);
            let configured = list(&out.fragment.bytes, root + 32, base)?;
            assert_eq!(
                configured
                    .iter()
                    .map(|&p| base + p as u32)
                    .collect::<Vec<_>>(),
                family
                    .configurations
                    .iter()
                    .map(|r| out.configurations[&r.path].address)
                    .collect::<Vec<_>>()
            );
            let states = list(&out.fragment.bytes, root + 16, base)?;
            assert_eq!(states.len(), family.states.len());
            let mut source_state = session.pointer(a, 16)?;
            for (state, &offset) in family.states.iter().zip(&states) {
                let st = source_state.unwrap();
                let original: CharacterStateWire = wire(session.bytes(st, 64)?)?;
                let written: CharacterStateWire = wire(&out.fragment.bytes[offset..offset + 64])?;
                assert_eq!(written.animation, original.animation);
                assert_eq!(written.mechanics, original.mechanics);
                assert_eq!(
                    (written.unknown, written.unknown_tail, written.speed),
                    (original.unknown, original.unknown_tail, original.speed)
                );
                assert_eq!(
                    written.next_state,
                    state
                        .next
                        .map(|id| out.states[&(family.path.clone(), id)].address)
                        .unwrap_or(0)
                );
                let transitions = list(&out.fragment.bytes, offset + 16, base)?;
                assert_eq!(transitions.len(), original.transitions.count as usize);
                let mut tr = session.pointer(st, 16)?;
                for (transition, &p) in state.transitions.iter().zip(&transitions) {
                    let t = tr.unwrap();
                    let original: StateTransitionWire = wire(session.bytes(t, 24)?)?;
                    let written: StateTransitionWire = wire(&out.fragment.bytes[p..p + 24])?;
                    assert_eq!(
                        (written.linking_type, written.unknown),
                        (original.linking_type, original.unknown)
                    );
                    assert_eq!(
                        written.target,
                        out.states[&(family.path.clone(), transition.target)].address
                    );
                    assert_eq!(
                        written.via,
                        out.states[&(family.path.clone(), transition.via)].address
                    );
                    tr = session.pointer(t, 0)?;
                    transition_count += 1;
                }
                assert!(tr.is_none());
                let prohibited = list(&out.fragment.bytes, offset + 28, base)?;
                assert_eq!(prohibited.len(), original.prohibited.count as usize);
                for (&id, &p) in state.prohibited.iter().zip(&prohibited) {
                    assert_eq!(
                        u32_at(&out.fragment.bytes, p + 12)?,
                        out.states[&(family.path.clone(), id)].address
                    );
                }
                source_state = session.pointer(st, 0)?;
                state_count += 1;
            }
            assert!(source_state.is_none());
        }
    }
    assert_eq!(state_count, 585);
    eprintln!(
        "Family reconstruction: 21 families, {state_count} states, {transition_count} transitions, 18 shared bounds, 10 mechanics cards (5 typed and 5 explicitly unpromoted)"
    );
    Ok(())
}

fn list(bytes: &[u8], header: usize, base: u32) -> Result<Vec<usize>> {
    let count = u32_at(bytes, header + 8)? as usize;
    assert!(count <= 65536);
    let mut pointer = u32_at(bytes, header)?;
    let mut previous = 0;
    let mut result = Vec::new();
    for _ in 0..count {
        let offset = (pointer - base) as usize;
        assert_eq!(u32_at(bytes, offset + 4)?, previous);
        assert_eq!(u32_at(bytes, offset + 8)?, base + header as u32);
        result.push(offset);
        previous = pointer;
        pointer = u32_at(bytes, offset)?;
    }
    assert_eq!(pointer, 0);
    assert_eq!(u32_at(bytes, header + 4)?, previous);
    Ok(result)
}
