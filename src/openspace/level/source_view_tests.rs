use super::super::{
    hype_runtime,
    live::{self, HypeBuild, ReadMemory, SceneTailAi, SceneTailDynamics},
    records::{Block, BlockKey, Owner},
};
use super::*;
use std::collections::BTreeMap;

struct Memory {
    ranges: BTreeMap<u32, Vec<u8>>,
}
impl ReadMemory for Memory {
    fn read(&mut self, address: u32, size: usize) -> Result<Vec<u8>> {
        let (&start, data) = self
            .ranges
            .range(..=address)
            .next_back()
            .context("unmapped memory")?;
        let offset = (address - start) as usize;
        data.get(offset..offset.checked_add(size).context("overflow")?)
            .map(|s| s.to_vec())
            .context("short memory range")
    }
}
fn word(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
#[test]
fn verified_live_inventory_filters_inactive_members_and_rejects_cycles() -> Result<()> {
    let build = HypeBuild::verified(live::HYPE_EXECUTABLE_SHA256)?;
    let mut ranges = BTreeMap::new();
    let mut status = vec![0; 261];
    status[0] = 9;
    status[1..7].copy_from_slice(b"manoir");
    ranges.insert(hype_runtime::STATUS, status);
    ranges.insert(0x6107b0, 0x1000_u32.to_le_bytes().to_vec());
    let mut root = vec![0; 12];
    word(&mut root, 8, 0x1100);
    ranges.insert(0x1000, root);
    let mut first = vec![0; 24];
    word(&mut first, 4, 0x1200);
    word(&mut first, 20, 0x1140);
    ranges.insert(0x1100, first);
    let mut second = vec![0; 24];
    word(&mut second, 4, 0x1240);
    ranges.insert(0x1140, second);
    let mut active = vec![0; 9];
    word(&mut active, 0, 0x1300);
    active[8] = 1;
    ranges.insert(0x1200, active);
    ranges.insert(0x1240, vec![0; 9]);
    let mut member = vec![0; 30];
    word(&mut member, 12, 0x1400);
    member[28..30].copy_from_slice(&80_i16.to_le_bytes());
    ranges.insert(0x1300, member);
    ranges.insert(0x1400, b"Weapon\0".to_vec());
    let mut memory = Memory { ranges };
    let observed = build.inventory(&mut memory)?;
    assert_eq!(observed.total_entries, 2);
    assert_eq!(observed.active.len(), 1);
    assert_eq!(observed.active[0].kind, "Weapon");
    assert_eq!(observed.active[0].id, 80);
    assert_eq!(observed.active[0].quantity, 0);
    assert_eq!(observed.selected_weapon, 80);
    word(memory.ranges.get_mut(&0x1140).unwrap(), 20, 0x1100);
    assert!(
        build
            .inventory(&mut memory)
            .unwrap_err()
            .to_string()
            .contains("cycle")
    );
    Ok(())
}
#[test]
fn verified_live_magic_selection_resolves_selected_inventory_code() -> Result<()> {
    let build = HypeBuild::verified(live::HYPE_EXECUTABLE_SHA256)?;
    let mut ranges = BTreeMap::new();
    let mut status = vec![0; 261];
    status[0] = 9;
    status[1..7].copy_from_slice(b"manoir");
    ranges.insert(hype_runtime::STATUS, status);
    ranges.insert(0x6107b0, 0x1000_u32.to_le_bytes().to_vec());
    let mut root = vec![0; 12];
    word(&mut root, 8, 0x1100);
    ranges.insert(0x1000, root);
    let mut entry = vec![0; 24];
    word(&mut entry, 4, 0x1200);
    ranges.insert(0x1100, entry);
    let mut handle = vec![0; 9];
    word(&mut handle, 0, 0x1300);
    handle[8] = 1;
    ranges.insert(0x1200, handle);
    let mut member = vec![0; 30];
    word(&mut member, 12, 0x1400);
    member[28..30].copy_from_slice(&61_i16.to_le_bytes());
    ranges.insert(0x1300, member);
    ranges.insert(0x1400, b"MagicFire\0".to_vec());
    ranges.insert(0x610b98, 0x2000_u32.to_le_bytes().to_vec());
    let mut magic = vec![0; 42];
    magic[..5].copy_from_slice(b"Fire\0");
    magic[10..14].copy_from_slice(b"Ice\0");
    magic[36..38].copy_from_slice(&30_u16.to_le_bytes());
    magic[38..40].copy_from_slice(&50_u16.to_le_bytes());
    ranges.insert(0x2000, magic);
    let mut memory = Memory { ranges };
    let inventory = build.inventory(&mut memory)?;
    let selected = build.magic_selection(&mut memory, &inventory)?;
    assert_eq!(selected.current_points, 30);
    assert_eq!(selected.maximum_points, 50);
    assert_eq!(selected.selected_index, 0);
    assert_eq!(selected.selected_name, "Fire");
    assert_eq!(selected.source_code, Some(61));
    memory.ranges.get_mut(&0x2000).unwrap()[40] = 1;
    let unmatched = build.magic_selection(&mut memory, &inventory)?;
    assert_eq!(unmatched.selected_name, "Ice");
    assert_eq!(unmatched.source_code, None);
    memory.ranges.get_mut(&0x2000).unwrap()[40] = 3;
    assert!(
        build
            .magic_selection(&mut memory, &inventory)
            .unwrap_err()
            .to_string()
            .contains("three name rows")
    );
    Ok(())
}
#[test]
fn verified_live_laser_reads_target_independently_of_visibility() -> Result<()> {
    let build = HypeBuild::verified(live::HYPE_EXECUTABLE_SHA256)?;
    let mut ranges = BTreeMap::new();
    let mut status = vec![0; 261];
    status[0] = 9;
    status[1..7].copy_from_slice(b"manoir");
    ranges.insert(hype_runtime::STATUS, status);
    let mut laser = vec![0; 22];
    for (i, value) in [12.0_f32, -45.0, 3.5].iter().enumerate() {
        laser[i * 4..i * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
    ranges.insert(0x74b310, laser);
    let mut memory = Memory { ranges };
    let observed = build.laser_pointer(&mut memory)?;
    assert_eq!(observed.point, [12.0, -45.0, 3.5]);
    assert!(!observed.active);
    memory.ranges.get_mut(&0x74b310).unwrap()[20] = 1;
    assert!(build.laser_pointer(&mut memory)?.active);
    Ok(())
}
#[test]
fn verified_live_scene_tail_checks_reverse_links_and_bounds() -> Result<()> {
    let build = HypeBuild::verified(live::HYPE_EXECUTABLE_SHA256)?;
    let mut ranges = BTreeMap::new();
    let mut status = vec![0; 261];
    status[0] = 9;
    status[1..7].copy_from_slice(b"manoir");
    ranges.insert(hype_runtime::STATUS, status);
    ranges.insert(0x75701c, 0x2000_u32.to_le_bytes().to_vec());
    let mut root = vec![0; 64];
    word(&mut root, 8, 0x2100);
    word(&mut root, 12, 0x2200);
    word(&mut root, 16, 2);
    ranges.insert(0x2000, root);
    let mut first = vec![0; 64];
    word(&mut first, 20, 0x2200);
    word(&mut first, 28, 0x2000);
    ranges.insert(0x2100, first);
    let mut second = vec![0; 64];
    word(&mut second, 24, 0x2100);
    word(&mut second, 28, 0x2000);
    ranges.insert(0x2200, second);
    let mut memory = Memory { ranges };
    let tail = build.scene_tail(&mut memory, 2)?;
    assert_eq!(tail.child_count, 2);
    assert_eq!(tail.children.len(), 2);
    assert_eq!(tail.children[0].index, 0);
    assert_eq!(tail.children[1].index, 1);
    assert_eq!(tail.children[0].node_locator, "live:00002100");
    assert_eq!(tail.children[1].node_locator, "live:00002200");
    assert_eq!(tail.children[1].perso_locator, None);
    assert_eq!(tail.children[1].dynamics, None);
    assert!(
        tail.children
            .iter()
            .all(|child| child.object_types.is_none())
    );
    assert!(tail.children.iter().all(|child| child.ai.is_none()));
    let child = memory.ranges.get_mut(&0x2200).unwrap();
    word(child, 0, 4);
    word(child, 4, 0x3000);
    word(child, 36, 0x3500);
    let mut perso = vec![0; 20];
    word(&mut perso, 0, 0x3100);
    word(&mut perso, 4, 0x3200);
    memory.ranges.insert(0x3000, perso);
    let mut data3d = vec![0; 24];
    word(&mut data3d, 20, 0x3400);
    memory.ranges.insert(0x3100, data3d);
    let mut standard = vec![0; 12];
    word(&mut standard, 0, 8);
    word(&mut standard, 4, 10);
    word(&mut standard, 8, 65536);
    memory.ranges.insert(0x3200, standard);
    memory
        .ranges
        .insert(0x3500, vec![0x11, 0, 0, 0, 0, 0, 0, 0]);
    let tail = build.scene_tail(&mut memory, 1)?;
    assert_eq!(tail.children[0].object_types, Some([8, 10, 65536]));
    assert_eq!(tail.children[0].node_locator, "live:00002200");
    assert_eq!(
        tail.children[0].perso_locator.as_deref(),
        Some("live:00003000")
    );
    assert_eq!(tail.children[0].dynamics, Some(SceneTailDynamics::Absent));
    assert_eq!(tail.children[0].ai, Some(SceneTailAi::BrainAbsent));
    word(memory.ranges.get_mut(&0x3000).unwrap(), 8, 0x3600);
    let mut wrapper = vec![0; 0x100];
    word(&mut wrapper, 16, 0x4000);
    word(&mut wrapper, 0x14, 0x6000);
    word(&mut wrapper, 0x18, 8);
    word(&mut wrapper, 0xfc, 0x5000);
    memory.ranges.insert(0x3600, wrapper);
    let mut dynamics = vec![0; 0x349];
    dynamics[0x348] = 1;
    memory.ranges.insert(0x4000, dynamics);
    let mut cards = vec![0; 0x2c];
    cards[0] = 2;
    word(&mut cards, 4, 7);
    word(&mut cards, 8, 8);
    word(&mut cards, 0x28, 0x6000);
    memory.ranges.insert(0x5000, cards);
    memory.ranges.insert(0x6000, 8_u32.to_le_bytes().to_vec());
    let tail = build.scene_tail(&mut memory, 1)?;
    assert_eq!(
        tail.children[0].dynamics,
        Some(SceneTailDynamics::Ready {
            wrapper_locator: "live:00003600".into(),
            object_locator: "live:00004000".into(),
            override_flag: 1,
            cached_card_locator: Some("live:00006000".into()),
            cached_card_type: 8,
            list_locator: Some("live:00005000".into()),
            card_slots: vec![
                live::SceneTailCardSlot {
                    index: 0,
                    key: 7,
                    card_locator: None,
                    card_type: None,
                },
                live::SceneTailCardSlot {
                    index: 1,
                    key: 8,
                    card_locator: Some("live:00006000".into()),
                    card_type: Some(8),
                },
            ],
            selected_card_locator: Some("live:00006000".into()),
            selected_card_type: Some(8),
        })
    );
    word(memory.ranges.get_mut(&0x3000).unwrap(), 16, 0x3300);
    let mut brain = vec![0; 8];
    word(&mut brain, 0, 0x3320);
    memory.ranges.insert(0x3300, brain);
    let mut mind = vec![0; 20];
    word(&mut mind, 0, 0x3380);
    memory.ranges.insert(0x3320, mind);
    memory.ranges.insert(0x3380, vec![0; 16]);
    let tail = build.scene_tail(&mut memory, 1)?;
    assert_eq!(
        tail.children[0].ai,
        Some(SceneTailAi::Ready {
            model_locator: "live:00003380".into(),
            normal_count: 0,
            reflex_count: 0,
            designer_field_count: 0,
            normal: None,
            reflex: None,
        })
    );
    word(memory.ranges.get_mut(&0x3300).unwrap(), 0, 0x3330);
    let tail = build.scene_tail(&mut memory, 1)?;
    assert!(matches!(
        tail.children[0].ai,
        Some(SceneTailAi::Unreadable { .. })
    ));
    word(memory.ranges.get_mut(&0x2200).unwrap(), 28, 0x2200);
    assert!(
        build
            .scene_tail(&mut memory, 2)
            .unwrap_err()
            .to_string()
            .contains("parent link")
    );
    Ok(())
}
fn fixture(base: u32) -> (Memory, SourceSession) {
    // Independent known-byte fixture. These locations are not derived by the
    // codec's writer: two distinct members with equal timing properties.
    let mut data = vec![0; 0x1000];
    let sites = [
        (4, 0x100),
        (0x104, 0x200),
        (0x124, 0x700),
        (0x200, 0x300),
        (0x204, 0x400),
        (0x300, 0x600),
        (0x304, 0x640),
        (0x314, 0x500),
        (0x510, 0x600),
        (0x514, 0x640),
        (0x600, 0x640),
        (0x628, 0x640),
        (0x210, 0xa00),
        (0xa00, 0xa20),
        (0xa20, 0xa40),
        (0xa24, 0xa80),
        (0xa2c, 0xac0),
        (0xa40, 0xb00),
        (0xa48, 0xb40),
        (0xa80, 0xba0),
        (0xa88, 0xb8c),
        (0xa8c, 0xb80),
        (0xa94, 0xb80),
        (0xac0, 0xbc0),
        (0xac8, 0xc20),
        (0xb00, 0xb80),
        (0xb40, 0xc00),
        (0xb44, 0xc40),
        (0xba0, 0xb00),
        (0xbc0, 0xb40),
        (0xc24, 0x640),
        (0xc28, 0x200),
    ];
    for (site, target) in sites {
        word(&mut data, site, base + target);
    }
    word(&mut data, 0x100, 4);
    word(&mut data, 0x308, u32::MAX);
    for (i, value) in [5, 9, 42].into_iter().enumerate() {
        word(&mut data, 0x400 + i * 4, value);
    }
    word(&mut data, 0x50c, 5);
    word(&mut data, 0x518, 2);
    word(&mut data, 0xb04, 2);
    word(&mut data, 0xb48, 12);
    data[0xb4c] = 3;
    word(&mut data, 0xc20, (-7i32) as u32);
    for (index, (offset, kind)) in [(0u32, 5u32), (4, 14), (8, 10)].into_iter().enumerate() {
        let info = 0xc40 + index * 16;
        word(&mut data, info, offset);
        word(&mut data, info + 4, kind);
    }
    data[0x63a..0x63d].copy_from_slice(&[3, 7, 1]);
    data[0x67a..0x67d].copy_from_slice(&[2, 9, 0]);
    word(&mut data, 0x700, 0x90);
    for (i, value) in [1.25f32, -2., 3.5].into_iter().enumerate() {
        word(&mut data, 0x704 + i * 4, value.to_bits());
    }
    for offset in [0x710, 0x734] {
        for i in [0, 4, 8] {
            word(&mut data, offset + i * 4, 1f32.to_bits());
        }
    }
    let key = BlockKey(6, 2);
    let session = SourceSession {
        fix_bindings: Default::default(),
        allocation_cache: Default::default(),
        blocks: BTreeMap::from([(
            Owner::Level,
            BTreeMap::from([(
                key,
                Block {
                    base,
                    data: data.clone(),
                },
            )]),
        )]),
        actual: BTreeMap::from([(key, Owner::Level)]),
        pointers: sites
            .into_iter()
            .map(|(site, _)| ((Owner::Level, key, base + site as u32), key))
            .collect(),
        gpt: vec![],
        rtp: BTreeMap::new(),
        ptx: vec![],
        rtt: BTreeMap::new(),
    };
    let mut status = vec![0; 261];
    status[0] = 9;
    status[1..7].copy_from_slice(b"manoir");
    (
        Memory {
            ranges: BTreeMap::from([
                (base, data),
                (0x756e70, base.to_le_bytes().to_vec()),
                (0x754420, status),
            ]),
        },
        session,
    )
}

#[test]
fn live_ai_reader_resolves_behavior_and_requested_designer_values() -> Result<()> {
    let build = HypeBuild::verified(live::HYPE_EXECUTABLE_SHA256)?;
    for base in [0x1000, 0x21000] {
        let (mut memory, _) = fixture(base);
        let observed = build.actor_with_ai(&mut memory, &[1], &[0, 1, 2])?;
        assert_eq!(
            observed.normal,
            Some(crate::model::ai::BehaviorSelection {
                current: Some(1),
                previous: Some(0),
                initial: Some(0),
            })
        );
        assert_eq!(observed.reflex, None);
        assert_eq!(
            observed.designer_values,
            vec![
                live::SelectedDesignerValue {
                    index: 0,
                    kind: 5,
                    value: live::ObservedDesignerValue::Value(
                        crate::model::ai::DesignerValue::Integer(-7),
                    ),
                },
                live::SelectedDesignerValue {
                    index: 1,
                    kind: 14,
                    value: live::ObservedDesignerValue::Action(Some(1)),
                },
                live::SelectedDesignerValue {
                    index: 2,
                    kind: 10,
                    value: live::ObservedDesignerValue::Actor(Some(format!(
                        "live:{:08x}",
                        base + 0x200
                    ))),
                },
            ]
        );
        assert_eq!(observed.actor.current_action, Some(1));
        assert!(
            build.actor_with_ai(&mut memory, &[], &[0, 0]).is_err(),
            "duplicate selection must not alias one captured value"
        );
    }
    Ok(())
}

#[test]
fn live_scene_actor_path_checks_source_identity() -> Result<()> {
    let build = HypeBuild::verified(live::HYPE_EXECUTABLE_SHA256)?;
    let (mut memory, _) = fixture(0x1000);
    let data = memory.ranges.get_mut(&0x1000).unwrap();
    // A world with one existing Perso child, using independent fixture bytes.
    word(data, 0x800, 0);
    word(data, 0x808, 0x1100);
    word(data, 0x810, 1);
    word(data, 0x11c, 0x1800);
    memory
        .ranges
        .insert(0x75ec6c, 0x1800_u32.to_le_bytes().to_vec());
    let actor = build.scene_actor_with_ai(&mut memory, "manoir", &[0], [5, 9, 42], &[1], &[0])?;
    assert_eq!(actor.actor.current_action, Some(1));
    assert!(
        build
            .scene_actor_with_ai(&mut memory, "other", &[0], [5, 9, 42], &[1], &[])
            .is_err()
    );
    assert!(
        build
            .scene_actor_with_ai(&mut memory, "manoir", &[1], [5, 9, 42], &[1], &[])
            .is_err()
    );
    assert!(
        build
            .scene_actor_with_ai(&mut memory, "manoir", &[0], [5, 9, 41], &[1], &[])
            .is_err()
    );
    Ok(())
}

#[test]
fn live_ai_reader_rejects_malformed_truncated_and_foreign_references() -> Result<()> {
    let build = HypeBuild::verified(live::HYPE_EXECUTABLE_SHA256)?;
    for (offset, value, expected) in [
        (0xa88, 0xd00, "outside its family"),
        (0xba0, 0xb40, "binds another behavior set"),
        (0xc24, 0xd00, "outside selected actor family"),
        (0xb48, 8, "exceeds declared memory"),
    ] {
        let (mut memory, _) = fixture(0x1000);
        word(
            memory.ranges.get_mut(&0x1000).unwrap(),
            offset,
            if offset == 0xb48 {
                value
            } else {
                0x1000 + value
            },
        );
        let error = format!(
            "{:#}",
            build.actor_with_ai(&mut memory, &[], &[1, 2]).unwrap_err()
        );
        assert!(error.contains(expected), "{offset:x}: {error}");
    }

    let (mut memory, _) = fixture(0x1000);
    memory.ranges.get_mut(&0x1000).unwrap().truncate(0xc43);
    let error = format!(
        "{:#}",
        build.actor_with_ai(&mut memory, &[], &[0]).unwrap_err()
    );
    assert!(error.contains("short memory range"), "{error}");
    Ok(())
}
#[test]
fn file_and_relocated_memory_share_bounded_actor_and_transform_readers() -> Result<()> {
    let build = HypeBuild::verified(live::HYPE_EXECUTABLE_SHA256)?;
    assert!(HypeBuild::verified(&"0".repeat(64)).is_err());
    for base in [0x1000, 0x21000] {
        let (mut memory, file) = fixture(base);
        let observed = build.actor_with_actions(&mut memory, &[0, 1])?;
        let file_observed = live::actor(
            &mut FileView(&file),
            Address {
                owner: Owner::Level,
                key: BlockKey(6, 2),
                offset: 0,
            },
            &[0, 1],
            |a| format!("{a:?}"),
        )?;
        assert_eq!(observed.object_types, [5, 9, 42]);
        assert_eq!(observed.family_type, 5);
        assert_eq!(
            (
                observed.initial_action,
                observed.current_action,
                observed.secondary_action
            ),
            (Some(0), Some(1), None)
        );
        assert_eq!(
            observed.global_transform.affine().translation,
            [1.25, -2., 3.5]
        );
        assert_eq!(
            observed.global_transform.affine().basis,
            [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]
        );
        assert_eq!(observed.action_members.len(), 2);
        assert_eq!(
            observed.action_definitions,
            vec![
                live::ActionDefinitionProjection {
                    id: 0,
                    speed: 7,
                    repeats: 3,
                    transition_lock: 1,
                    next: Some(1),
                },
                live::ActionDefinitionProjection {
                    id: 1,
                    speed: 9,
                    repeats: 2,
                    transition_lock: 0,
                    next: None,
                },
            ]
        );
        assert_ne!(observed.action_members[0], observed.action_members[1]);
        assert_eq!(observed.object_types, file_observed.object_types);
        assert_eq!(observed.current_action, file_observed.current_action);
        assert_eq!(observed.projection(), file_observed.projection());
        assert_eq!(observed.global_transform, file_observed.global_transform);
        assert_eq!(build.status(&mut memory)?.level, "manoir");
        // A distinct equal-valued actor has separate node/perso identity while
        // referring to the same source family and same transform definition.
        let data = memory.ranges.get_mut(&base).unwrap();
        data.copy_within(0x100..0x140, 0x800);
        data.copy_within(0x200..0x208, 0x900);
        word(data, 4, base + 0x800);
        word(data, 0x804, base + 0x900);
        let second = build.actor_with_actions(&mut memory, &[0, 1])?;
        assert_ne!(observed.node, second.node);
        assert_ne!(observed.perso, second.perso);
        assert_eq!(observed.family, second.family);
        assert_eq!(observed.projection(), second.projection());
        assert_eq!(observed.global_transform, second.global_transform);
    }
    Ok(())
}
#[test]
fn source_views_reject_cycles_short_records_unknown_transforms_and_foreign_members() -> Result<()> {
    let build = HypeBuild::verified(live::HYPE_EXECUTABLE_SHA256)?;
    let (mut menu, _) = fixture(0x1000);
    menu.ranges.get_mut(&0x754420).unwrap()[0] = 8;
    assert!(format!("{:#}", build.actor(&mut menu).unwrap_err()).contains("gameplay phase"));
    for (offset, value, expected) in [
        (0x600, 0x1600, "cyclic"),
        (0x600, 0, "short linked"),
        (0x518, 65537, "excessive"),
        (0x304, 0x1900, "outside its family"),
        (0x124, u32::MAX, "overflow"),
        (0x700, 0x21, "unsupported compact"),
        (0x50c, 6, "object type"),
    ] {
        let (mut memory, _) = fixture(0x1000);
        word(memory.ranges.get_mut(&0x1000).unwrap(), offset, value);
        let error = format!(
            "{:#}",
            build.actor_with_actions(&mut memory, &[0, 1]).unwrap_err()
        );
        assert!(error.contains(expected), "{offset:x}: {error}");
    }
    let (mut memory, _) = fixture(0x1000);
    memory.ranges.get_mut(&0x1000).unwrap().truncate(0x757);
    assert!(build.actor_with_actions(&mut memory, &[0, 1]).is_err());
    // Compact translation ends exactly at the range boundary. Maximum-size
    // speculative reads would fail here; only the eight declared bytes exist.
    let data = memory.ranges.get_mut(&0x1000).unwrap();
    data.truncate(0x708);
    data[0x700..].copy_from_slice(&[0x11, 0, 0, 2, 0, 0xfc, 0, 7]);
    assert_eq!(
        build
            .actor_with_actions(&mut memory, &[0, 1])?
            .global_transform
            .affine()
            .translation,
        [1., -2., 3.5]
    );
    Ok(())
}

#[test]
fn source_fingerprint_rejects_invalid_selection_before_filesystem_access() -> Result<()> {
    let selection = crate::model::Selection {
        level: "../manoir".into(),
        locale: "en".into(),
        profile: "hype-pc-montreal".into(),
    };
    let error = live::source_fingerprint(std::path::Path::new("missing"), &selection)
        .unwrap_err()
        .to_string();
    assert!(error.contains("invalid level selection"), "{error}");
    Ok(())
}

#[test]
fn fingerprint_path_resolution_accepts_unique_case_and_rejects_ambiguity() -> Result<()> {
    let temp = tempfile::tempdir()?;
    std::fs::create_dir_all(temp.path().join("GameData/World/Levels"))?;
    let resolved = super::super::session::resolve_unique_case_insensitive(
        temp.path(),
        std::path::Path::new("Gamedata/world/levels"),
    )?;
    assert_eq!(resolved, temp.path().join("GameData/World/Levels"));
    std::fs::create_dir(temp.path().join("gamedata"))?;
    let error = super::super::session::resolve_unique_case_insensitive(
        temp.path(),
        std::path::Path::new("Gamedata/World"),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("2 case-insensitive matches"), "{error}");
    Ok(())
}

#[test]
#[ignore = "requires local manoir/astrolabe files; selected by tools/test.py --local"]
fn shared_actor_reader_matches_useful_file_import_for_both_levels() -> Result<()> {
    use crate::{
        app,
        model::{Selection, io::Load},
        rete,
    };
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    for level in ["manoir", "astrolabe"] {
        let selection = Selection::hype(level, "en")?;
        let fingerprint = live::source_fingerprint(&game, &selection)?;
        assert_eq!(fingerprint, live::source_fingerprint(&game, &selection)?);
        if let Some(installed) = std::env::var_os("ASTROLABE_INSTALLED_GAME_DIR") {
            assert_eq!(
                fingerprint,
                live::source_fingerprint(std::path::Path::new(&installed), &selection)?,
                "installed fixture source differs from disc corpus"
            );
        }
        assert_eq!(fingerprint.files.len(), 9);
        assert_eq!(fingerprint.profile, selection.profile);
        assert_eq!(fingerprint.level, level);
        assert_eq!(fingerprint.locale, "en");
        assert!(
            fingerprint
                .files
                .iter()
                .all(|file| file.byte_count > 0 && file.sha256.len() == 64)
        );
        assert_eq!(
            fingerprint
                .files
                .iter()
                .map(|file| file.relative_path.clone())
                .collect::<Vec<_>>(),
            vec![
                "Gamedata/World/Levels/Fix.sna".to_owned(),
                "Gamedata/World/Levels/Fix.rtb".to_owned(),
                format!("Gamedata/World/Levels/{level}/{level}.sna"),
                format!("Gamedata/World/Levels/{level}/{level}.rtb"),
                format!("Gamedata/World/Levels/{level}/fixlvl.rtb"),
                format!("Gamedata/World/Levels/{level}/{level}.rtp"),
                format!("Gamedata/World/Levels/{level}/{level}.rtt"),
                format!("Gamedata/World/Levels/{level}/{level}.gpt"),
                format!("Gamedata/World/Levels/{level}/{level}.ptx"),
            ]
        );
        let observed = live::inspect_file_actor(&game, &selection)?;
        let mut imported = crate::openspace::content::import_scene(&game, selection)?;
        let scene: std::sync::Arc<crate::model::scene::Scene> =
            imported.load(&crate::model::AssetRef::new("level:/scene")?)?;
        let actor = imported.load(scene.main_actor.as_ref().context("missing main actor")?)?;
        let family = imported.load(&actor.family)?;
        let direct = live::ActorProjection::from_model(&actor, &family, &[47])?;
        assert_eq!(observed.projection(), direct);
        let selected = direct
            .family
            .selected_actions
            .first()
            .context("main actor family lacks selected action 47")?;
        assert_eq!(selected.id, 47);
        assert_eq!(observed.object_types, actor.object_types);
        assert_eq!(observed.family_type, family.object_type);
        assert_eq!(observed.current_action, actor.current_state);
        assert_eq!(observed.initial_action, actor.initial_state);
        assert_eq!(observed.action_members.len(), family.states.len());

        let temp = tempfile::tempdir()?;
        app::save_rete(&mut imported, temp.path())?;
        let scene_ref = crate::model::AssetRef::new("level:/scene")?;
        drop(family);
        drop(actor);
        drop(scene);
        drop(imported);
        let (mut reopened, _) = rete::Store::open(temp.path())?;
        let scene: std::sync::Arc<crate::model::scene::Scene> = reopened.load(&scene_ref)?;
        let actor = reopened.load(scene.main_actor.as_ref().context("missing main actor")?)?;
        let family = reopened.load(&actor.family)?;
        let persisted = live::ActorProjection::from_model(&actor, &family, &[47])?;
        assert_eq!(direct, persisted);
        println!(
            "{level}: {} actions, current {:?}, transform {:?}",
            family.states.len(),
            observed.current_action,
            observed.global_transform
        );
    }
    Ok(())
}
