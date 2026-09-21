use super::super::{
    live::{self, HypeBuild, ReadMemory},
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
        model::{io::Load, Selection},
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
        assert!(fingerprint
            .files
            .iter()
            .all(|file| file.byte_count > 0 && file.sha256.len() == 64));
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
