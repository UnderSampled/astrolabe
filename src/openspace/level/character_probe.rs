fn check_configuration_reconstruction(
    session: &super::session::SourceSession,
    value: &super::SceneImport,
) -> anyhow::Result<()> {
    use super::{
        ReferenceTarget,
        container::{reloc, u32_at, wire},
        records::*,
    };
    use crate::model::PackageRole;
    use std::collections::{BTreeMap, BTreeSet};
    let mut family_addresses = BTreeSet::new();
    for node in session.tree()? {
        if u32_at(session.bytes(node.address, 4)?, 0)? == 4 {
            let perso = session.required(node.address, 4)?;
            family_addresses.insert(session.required(session.required(perso, 0)?, 20)?);
        }
    }
    assert_eq!(family_addresses.len(), 21);
    let mut mechanic_cards = BTreeSet::new();
    let mut bound_geometries = BTreeSet::new();
    let mut next_state_nulls = BTreeMap::new();
    for address in family_addresses {
        assert_eq!(
            u32_at(&session.block(address)?.data, address.offset - 4)? * 4 - 4,
            56
        );
        let header: FamilyWire = wire(session.bytes(address, 56)?)?;
        let family = value
            .families
            .iter()
            .find(|f| f.path.file() == format!("families/{}", header.index))
            .unwrap();
        assert_eq!(family.animation_bank, header.animation_bank);
        assert_eq!(family.unknown, header.unknown);
        assert_eq!(super::container::encoded(&header)?.len(), 56);
        if let Some(a) = session.pointer(address, 44)? {
            bound_geometries.insert(a);
        }
        assert_eq!(header.extra_data, 0);
        let mut state = session.pointer(address, 16)?;
        while let Some(a) = state {
            let next = u32_at(session.bytes(a, 44)?, 40)?;
            *next_state_nulls
                .entry(if next == 0 {
                    "null"
                } else if next == u32::MAX {
                    "minus_one"
                } else {
                    "state"
                })
                .or_insert(0usize) += 1;
            assert_eq!(u32_at(&session.block(a)?.data, a.offset - 4)? * 4 - 4, 64);
            assert_eq!(u32_at(session.bytes(a, 56)?, 52)?, 0);
            if let Some(card) = session.pointer(a, 48)? {
                mechanic_cards.insert(card);
            }
            state = session.pointer(a, 0)?;
        }
    }
    assert_eq!(mechanic_cards.len(), 10);
    assert_eq!(bound_geometries.len(), 18);
    assert_eq!(
        next_state_nulls,
        BTreeMap::from([("null", 387), ("state", 198)])
    );
    for a in bound_geometries {
        let h: GeometryWire = wire(session.bytes(a, 48)?)?;
        let types =
            session.short_arrays::<1>(session.required(a, 24)?, h.element_count as usize)?;
        assert_eq!(h.point_count, 1);
        assert_eq!(types, vec![[7]]);
    }
    let mut reset_count = 0;
    let mut total_parts = 0;
    for config in &value.configurations {
        let (owner, key) = match config.path.role() {
            PackageRole::Fix => (Owner::Fix, BlockKey(5, 0)),
            _ => (Owner::Level, BlockKey(5, 1)),
        };
        let id: u32 = config.path.file().rsplit('/').next().unwrap().parse()?;
        let offset = session
            .allocation_indices(owner, key)?
            .into_iter()
            .find(|&(_, i)| i == id)
            .unwrap()
            .0;
        let address = Address { owner, key, offset };
        assert_eq!(
            u32_at(&session.block(address)?.data, offset - 4)? * 4 - 4,
            24
        );
        let header: ObjectListWire = wire(session.bytes(address, 24)?)?;
        assert_eq!(header.count as usize, config.parts.len());
        assert_eq!(header.unknown, config.unknown);
        assert_eq!(header.reset_parts != 0, config.reset_parts.is_some());
        reset_count += usize::from(config.reset_parts.is_some());
        let mut models = BTreeMap::new();
        let mut actions = BTreeMap::new();
        for (field, parts) in [(12, Some(&config.parts)), (16, config.reset_parts.as_ref())] {
            let Some(parts) = parts else {
                continue;
            };
            let table = session.required(address, field)?;
            for (i, part) in parts.iter().enumerate() {
                let entry = Address {
                    offset: table.offset + i * 20,
                    ..table
                };
                let target = ReferenceTarget {
                    address: u32_at(session.bytes(entry, 8)?, 4)?,
                    block: session
                        .pointer(entry, 4)?
                        .map(|p| (p.key.0, p.key.1))
                        .unwrap_or((0, 0)),
                };
                if let Some(r) = &part.instance {
                    models.insert(r.path.clone(), target);
                }
                if let Some(r) = &part.action {
                    actions.insert(r.path.clone(), target);
                }
            }
        }
        let base = 0x70000000;
        let out = super::write_configuration(config, &models, &actions, base)?;
        let relocations = reloc(&out.rtb)?;
        let written: ObjectListWire = wire(&out.bytes)?;
        assert_eq!(written.unknown, header.unknown);
        assert_eq!(written.count, header.count);
        let mut expected_sites = BTreeSet::new();
        for (field, target, parts) in [
            (12, written.parts, Some(&config.parts)),
            (16, written.reset_parts, config.reset_parts.as_ref()),
        ] {
            let Some(parts) = parts else {
                assert_eq!(target, 0);
                continue;
            };
            let source = session.required(address, field)?;
            let start = (target - base) as usize;
            expected_sites.insert(base + field as u32);
            for i in 0..parts.len() {
                let original = Address {
                    offset: source.offset + i * 20,
                    ..source
                };
                let bytes = session.bytes(original, 20)?;
                let at = start + i * 20;
                assert_eq!(
                    &out.bytes[at + 4..at + 20],
                    &bytes[4..20],
                    "configuration content/type/unknown values changed: {} slot {i}",
                    config.path
                );
                if let Some(scale) = session.pointer(original, 0)? {
                    let p = (u32_at(&out.bytes, at)? - base) as usize;
                    assert_eq!(&out.bytes[p..p + 12], session.bytes(scale, 12)?);
                    expected_sites.insert(base + at as u32);
                } else {
                    assert_eq!(u32_at(&out.bytes, at)?, 0);
                }
                if let Some(content) = session.pointer(original, 4)? {
                    expected_sites.insert(base + at as u32 + 4);
                    assert_eq!(
                        relocations[&key]
                            .iter()
                            .find(|r| r.value == base + at as u32 + 4)
                            .map(|r| BlockKey(r.module, r.id)),
                        Some(content.key)
                    );
                }
                total_parts += 1;
            }
        }
        assert_eq!(
            relocations[&key]
                .iter()
                .map(|r| r.value)
                .collect::<BTreeSet<_>>(),
            expected_sites
        );
    }
    assert_eq!(reset_count, 2);
    eprintln!(
        "Configuration reconstruction: 17 assets, {total_parts} active/reset entries, 2 saved snapshots; 21 family records verified at 56 bytes"
    );
    Ok(())
}

#[test]
#[ignore = "requires local game data; included by tools/test.py --local"]
fn source_character_inventory_and_compact_matrix_reconstruction() -> anyhow::Result<()> {
    use super::{container::u32_at, records::*, session::SourceSession};
    use crate::model::{PackageRole, Selection};
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let selection = Selection::hype("astrolabe", "en")?;
    let value = super::import_scene(&game, &selection)?;
    assert_eq!(
        (
            value.persos.len(),
            value.families.len(),
            value.configurations.len(),
            value.animations.len(),
            value.transform_pools.len(),
            value.models.len()
        ),
        (108, 21, 17, 492, 2, 436)
    );
    assert_eq!(
        value.animations.iter().map(|a| a.cues.len()).sum::<usize>(),
        593
    );
    assert_eq!(
        value
            .animations
            .iter()
            .filter(|a| !a.cues.is_empty())
            .count(),
        304
    );
    assert_eq!(
        value
            .configurations
            .iter()
            .flat_map(|c| &c.parts)
            .filter(|p| p.action.is_some())
            .count(),
        145
    );
    let action_paths: std::collections::BTreeSet<_> =
        value.actions.iter().map(|a| &a.path).collect();
    assert_eq!(action_paths.len(), value.actions.len());
    for cue in value.animations.iter().flat_map(|a| &a.cues) {
        assert!(action_paths.contains(&cue.action.path));
    }
    for part in value.configurations.iter().flat_map(|c| &c.parts) {
        if let Some(action) = &part.action {
            assert!(action_paths.contains(&action.path));
        }
    }
    eprintln!(
        "Animation actions: {} shared descriptors, 593 cues, 145 configuration action slots",
        value.actions.len()
    );
    let hero = value.families.iter().find(|f| f.name == "hype").unwrap();
    assert_eq!(hero.path.role(), PackageRole::Fix);
    assert_eq!(hero.states.len(), 389);
    let session = SourceSession::open(&game, &selection)?;
    check_configuration_reconstruction(&session, &value)?;
    super::family_probe::check(&session, &value)?;
    super::scene_record_probe::check(&session, &value)?;
    super::environment::check(&session, &value)?;
    // Descriptor scalars/unknowns reconstruct independently of the saved runtime handle.
    let sounds = super::sound::SoundSites::open(&game, &selection, &session)?;
    for (role, owner, key) in [
        (PackageRole::Fix, Owner::Fix, BlockKey(5, 0)),
        (PackageRole::Level, Owner::Level, BlockKey(5, 1)),
    ] {
        let actions: Vec<_> = value
            .actions
            .iter()
            .filter(|a| a.path.role() == role)
            .cloned()
            .collect();
        let out = super::write_animation_actions(&actions, 0x10000000)?;
        let offsets: std::collections::BTreeMap<_, _> = session
            .allocation_indices(owner, key)?
            .into_iter()
            .map(|(o, i)| (i, o))
            .collect();
        let rts = super::container::pointer_file(&out.rts)?;
        let snd: std::collections::BTreeMap<_, _> = out
            .snd
            .as_chunks::<8>()
            .0
            .iter()
            .map(|b| {
                let entry: SoundSiteWire = super::container::wire(b).unwrap();
                (entry.site, entry.event)
            })
            .collect();
        for action in &actions {
            let id: u32 = action.path.file().rsplit('/').next().unwrap().parse()?;
            let source = Address {
                owner,
                key,
                offset: offsets[&id],
            };
            let bytes = session.bytes(source, 12)?;
            let target = out.targets[&action.path];
            let offset = (target.address - 0x10000000) as usize;
            assert_eq!(
                &out.bytes[offset + 4..offset + 12],
                &bytes[4..],
                "descriptor type/unknown reconstruction"
            );
            if let crate::model::animation::AnimationOperation::Sound { event } = &action.operation
            {
                assert_eq!(
                    event.as_ref(),
                    sounds.events.get(&source).and_then(|e| e.as_ref())
                );
                assert_eq!(&out.bytes[offset..offset + 4], &[0; 4]);
                if let Some(event) = event {
                    assert_eq!(
                        snd[&target.address],
                        (u32::from(event.bank) << 16) | u32::from(event.member)
                    );
                    assert_eq!(rts[&target.address], key);
                }
            } else {
                assert_eq!(&out.bytes[offset..offset + 4], &bytes[..4]);
            }
        }
    }
    assert_eq!(value.lights.len(), 54);
    super::sector::check_reconstruction(&value.scene)?;
    let sectors: Vec<_> = value
        .scene
        .nodes
        .iter()
        .filter_map(|n| n.sector.as_ref())
        .collect();
    assert_eq!(sectors.iter().filter(|s| s.boundary.is_some()).count(), 13);
    eprintln!(
        "Sector links: graphics={}, optional={}, collision={}, activity={}, sound={}, persos={}",
        sectors.iter().map(|s| s.graphics.len()).sum::<usize>(),
        sectors
            .iter()
            .map(|s| s.optional_graphics.len())
            .sum::<usize>(),
        sectors.iter().map(|s| s.collision.len()).sum::<usize>(),
        sectors.iter().map(|s| s.activity.len()).sum::<usize>(),
        sectors.iter().map(|s| s.sound.len()).sum::<usize>(),
        sectors.iter().map(|s| s.persos.len()).sum::<usize>()
    );
    assert_eq!(
        value
            .scene
            .nodes
            .iter()
            .filter(|n| n.sector.is_some())
            .count(),
        17
    );
    let light_offsets: std::collections::BTreeMap<_, _> = session
        .allocation_indices(Owner::Level, BlockKey(6, 2))?
        .into_iter()
        .map(|(offset, index)| (index, offset))
        .collect();
    for light in &value.lights {
        let id: u32 = light.path.file().rsplit('/').next().unwrap().parse()?;
        let offset = light_offsets[&id];
        let original = &session.blocks[&Owner::Level][&BlockKey(6, 2)].data[offset..offset + 200];
        let written = super::write_light(light)?;
        assert_eq!(&original[..36], &written[..36]);
        assert_eq!(
            &original[40..],
            &written[40..],
            "light scalar/unknown bytes changed"
        );
        assert_eq!(super::read_light(&written, light.path.clone())?, *light);
    }
    let mut exact = 0;
    let mut changed = std::collections::BTreeMap::new();
    let mut types = std::collections::BTreeMap::new();
    for pool in &value.transform_pools {
        let (owner, key) = if pool.path.role() == PackageRole::Fix {
            (Owner::Fix, BlockKey(6, 1))
        } else {
            (Owner::Level, BlockKey(6, 2))
        };
        let offsets: std::collections::BTreeMap<_, _> = session
            .allocation_indices(owner, key)?
            .into_iter()
            .map(|(offset, index)| (index, offset))
            .collect();
        let block = &session.blocks[&owner][&key].data;
        for entry in &pool.entries {
            let offset = offsets[&entry.source_index.unwrap()];
            let flags = u16::from_le_bytes(block[offset..offset + 2].try_into()?);
            *types.entry(flags).or_insert(0usize) += 1;
            let length = match flags {
                0x11 => 8,
                0x22 => 10,
                0x33 => 16,
                0x77 => 18,
                0x7b => 22,
                0x7f => 28,
                _ => panic!("unexpected source form {flags:x}"),
            };
            let output = super::write_transform(&entry.value)?;
            let decoded = output
                .as_ref()
                .map(|b| super::read_transform(b))
                .transpose()?
                .unwrap_or_default();
            assert_eq!(
                decoded.affine(),
                entry.value.affine(),
                "derived compact form changed matrix meaning"
            );
            if output.as_deref() == Some(&block[offset..offset + length]) {
                exact += 1;
            } else {
                *changed
                    .entry((
                        flags,
                        output
                            .as_ref()
                            .map(|b| u32_at(&[b[0], b[1], 0, 0], 0).unwrap()),
                    ))
                    .or_insert(0usize) += 1;
            }
        }
    }
    eprintln!(
        "Compact matrices: {exact} byte-exact; derived-form changes {changed:?}; source forms {types:?}"
    );
    assert_eq!(types.len(), 6);
    assert_eq!(types.values().sum::<usize>(), 204927);
    assert_eq!(exact, 204474);
    assert_eq!(
        changed,
        std::collections::BTreeMap::from([
            ((0x33, None), 24),
            ((0x33, Some(0x11)), 294),
            ((0x33, Some(0x22)), 112),
            ((0x77, Some(0x33)), 23)
        ])
    );
    Ok(())
}
