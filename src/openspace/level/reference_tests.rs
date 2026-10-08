use super::*;
use crate::model::Selection;
use anyhow::Context;

#[test]
#[ignore = "requires local Hype source data; verifies retained Brigand AI fields"]
fn brigand_actor_brain_keeps_opaque_designer_fields_and_bindings() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("disc"));
    let selection = Selection::hype("brigand", "en")?;
    let imported = scene::import_scene(&game, &selection)?;
    let target_path = AssetPath::new("level:/persos/world/0/88")?;
    let target = imported
        .persos
        .iter()
        .find(|perso| perso.path == target_path)
        .context("Brigand source actor world/0/88 was not imported")?;
    let brain = target
        .brain
        .as_ref()
        .context("Brigand source actor world/0/88 has a Brain in the source data")?;
    assert_eq!(brain.model.path, AssetPath::new("level:/ai/models/1060")?);

    let model = imported
        .ai_models
        .iter()
        .find(|model| model.path == brain.model.path)
        .context("Brigand actor Brain model 1060 was not imported")?;
    for (index, kind, size) in [(46, 19, 44), (51, 13, 4), (52, 13, 4)] {
        let Some(crate::model::ai::DesignerValue::Opaque {
            kind: actual_kind,
            bytes,
        }) = model.variables.get(index).map(|variable| &variable.default)
        else {
            anyhow::bail!("designer field {index} was not retained as opaque bytes");
        };
        assert_eq!(*actual_kind, kind, "designer field {index} source type");
        assert_eq!(bytes.len(), size, "designer field {index} source span");
    }
    for index in [16, 36] {
        assert!(
            matches!(
                model.variables.get(index).map(|variable| &variable.default),
                Some(crate::model::ai::DesignerValue::Boolean(_))
            ),
            "field {index} used by source scripts should remain a Boolean"
        );
    }

    let writes = imported
        .ai_models
        .iter()
        .flat_map(crate::model::ai::AiModel::actor_field_writes)
        .filter_map(|(actor, field)| (actor.path == target_path).then_some(field))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(writes, [16, 36].into_iter().collect());
    let reads = imported
        .ai_models
        .iter()
        .flat_map(crate::model::ai::AiModel::actor_field_reads)
        .filter_map(|(actor, field)| (actor.path == target_path).then_some(field))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(reads, [36].into_iter().collect());
    Ok(())
}
#[test]
fn generated_sna_block_preserves_values_and_resolves_generated_relocations() -> Result<()> {
    let m = super::tests::model();
    let targets = BTreeMap::from([(
        m.submeshes[0].material.path.clone(),
        ReferenceTarget {
            address: 0x2000,
            block: (6, 1),
        },
    )]);
    let fragment = write_geometry(&m, &targets, 0x1000)?;
    let mut framed = write_sna_block(&fragment.bytes, (6, 2), 0x1000)?;
    let header: SnaInfo = wire(&framed[6..22])?;
    assert_eq!(
        header.capacity_end,
        0x1000 + fragment.bytes.len() as u32 - 1
    );
    assert_eq!(header.high_water, header.capacity_end + 1);
    assert_eq!(header.free_list, 0);
    framed.extend(write_sna_block(&[42; 16], (6, 1), 0x2000)?);
    let blocks = sna(&framed)?;
    assert_eq!(blocks[&BlockKey(6, 2)].data, fragment.bytes);
    let mut pointers = PointerMap::new();
    merge_relocations(&mut pointers, Owner::Level, reloc(&fragment.rtb)?)?;
    let ctx = SourceSession {
        fix_bindings: Default::default(),
        allocation_cache: Default::default(),
        actual: blocks.keys().map(|&k| (k, Owner::Level)).collect(),
        blocks: BTreeMap::from([(Owner::Level, blocks)]),
        pointers,
        gpt: vec![],
        rtp: BTreeMap::new(),
        ptx: vec![],
        rtt: BTreeMap::new(),
    };
    let geo = Address {
        owner: Owner::Level,
        key: BlockKey(6, 2),
        offset: 0,
    };
    assert_eq!(
        ctx.floats::<3>(ctx.required(geo, GeometryWire::POINTS)?, m.positions.len())?,
        m.positions
    );
    let table = ctx.required(geo, GeometryWire::ELEMENTS)?;
    let triangle = ctx.required(table, 0)?;
    assert_eq!(ctx.bytes(ctx.required(triangle, 0)?, 16)?, &[42; 16]);
    assert_eq!(
        ctx.short_arrays::<3>(ctx.required(triangle, 12)?, 1)?,
        m.submeshes[0].uv_indices
    );
    let mut changed = m.clone();
    changed.positions[0][0] = 7.;
    let edited = write_geometry(&changed, &targets, 0x5000)?;
    let roundtrip = sna(&write_sna_block(&edited.bytes, (6, 2), 0x5000)?)?;
    assert_eq!(
        f32::from_le_bytes(roundtrip[&BlockKey(6, 2)].data[48..52].try_into()?),
        7.
    );
    assert!(write_sna_block(&[], (6, 2), 0x1000).is_err());
    assert!(write_sna_block(&[0; 16], (6, 2), 0x7ffffffc).is_err());
    assert!(sna(&framed[..framed.len() - 1]).is_err());
    Ok(())
}
#[test]
fn fix_to_level_binding_sentinels_conflicts_and_missing_sites_are_explicit() -> Result<()> {
    let fk = BlockKey(6, 1);
    let lk = BlockKey(6, 2);
    let mut data = vec![0; 16];
    data[0..4].copy_from_slice(&0x9004u32.to_le_bytes());
    data[4..8].copy_from_slice(&0x9004u32.to_le_bytes());
    data[12..16].copy_from_slice(&0x8000u32.to_le_bytes());
    let group = |site, target: BlockKey| {
        BTreeMap::from([(
            fk,
            vec![RelocRow {
                value: site,
                module: target.0,
                id: target.1,
                unknown: [0; 2],
            }],
        )])
    };
    let mut pointers = PointerMap::new();
    merge_relocations(&mut pointers, Owner::Fix, group(0x2000, BlockKey(255, 255)))?;
    merge_relocations(&mut pointers, Owner::Fix, group(0x2000, lk))?;
    merge_relocations(&mut pointers, Owner::Fix, group(0x2000, BlockKey(255, 255)))?;
    assert_eq!(pointers[&(Owner::Fix, fk, 0x2000)], lk);
    assert!(merge_relocations(&mut pointers, Owner::Fix, group(0x2000, fk)).is_err());
    merge_relocations(&mut pointers, Owner::Fix, group(0x200c, lk))?;
    let mut c = SourceSession {
        fix_bindings: Default::default(),
        allocation_cache: Default::default(),
        blocks: BTreeMap::from([
            (
                Owner::Fix,
                BTreeMap::from([
                    (fk, Block { base: 0x2000, data }),
                    (
                        lk,
                        Block {
                            base: 0x9000,
                            data: vec![],
                        },
                    ),
                ]),
            ),
            (
                Owner::Level,
                BTreeMap::from([(
                    lk,
                    Block {
                        base: 0x1000,
                        data: vec![42; 16],
                    },
                )]),
            ),
        ]),
        actual: BTreeMap::from([(fk, Owner::Fix), (lk, Owner::Level)]),
        pointers,
        gpt: vec![],
        rtp: BTreeMap::new(),
        ptx: vec![],
        rtt: BTreeMap::new(),
    };
    let origin = Address {
        owner: Owner::Fix,
        key: fk,
        offset: 0,
    };
    assert_eq!(
        c.required(origin, 0)?,
        Address {
            owner: Owner::Level,
            key: lk,
            offset: 4
        }
    );
    assert!(c.pointer(origin, 4).is_err());
    assert!(c.pointer(origin, 8)?.is_none());
    assert!(c.pointer(origin, 12).is_err());
    merge_relocations(
        &mut c.pointers,
        Owner::Fix,
        group(0x2004, BlockKey(255, 255)),
    )?;
    assert!(c.pointer(origin, 4)?.is_none());
    let duplicates = [
        RelocRow {
            value: 0x1000,
            module: 6,
            id: 1,
            unknown: [0; 2],
        },
        RelocRow {
            value: 0x1000,
            module: 6,
            id: 2,
            unknown: [0; 2],
        },
    ];
    assert!(pointer_file(&emit_relocations(BlockKey(6, 2), &duplicates)?).is_err());
    Ok(())
}
#[test]
fn ptx_validates_real_pointer_slots_and_not_header_or_channel_words() -> Result<()> {
    let table = LevelTexturesWire {
        capacity: 1024,
        level_slots: 3,
        pointers: vec![0x1004, 0, 0xdeadbeef],
        loaded_textures: 1,
        memory_channels: vec![2; 1024],
        current_channel: 2,
    };
    let key = BlockKey(6, 2);
    let mut c = SourceSession {
        fix_bindings: Default::default(),
        allocation_cache: Default::default(),
        blocks: BTreeMap::from([(
            Owner::Level,
            BTreeMap::from([(
                key,
                Block {
                    base: 0x1000,
                    data: vec![42; 16],
                },
            )]),
        )]),
        actual: BTreeMap::from([(key, Owner::Level)]),
        pointers: PointerMap::new(),
        gpt: vec![],
        rtp: BTreeMap::new(),
        ptx: encoded(&table)?,
        rtt: BTreeMap::from([(0x1004, key), (0xdeadbeef, BlockKey(255, 255))]),
    };
    assert_eq!(c.texture_addresses()?.len(), 1);
    c.rtt.remove(&0x1004);
    assert!(c.texture_addresses().is_err());
    c.rtt.insert(0x1004, key);
    c.ptx.push(0);
    assert!(c.texture_addresses().is_err());
    c.ptx.pop();
    c.ptx[8..12].copy_from_slice(&0x1020u32.to_le_bytes());
    c.rtt.insert(0x1020, key);
    assert!(c.texture_addresses().is_err());
    c.ptx = encoded(&table)?;
    c.ptx[4..8].copy_from_slice(&1025u32.to_le_bytes());
    assert!(c.texture_addresses().is_err());
    assert!(level_textures(&[0; 7]).is_err());
    Ok(())
}
#[test]
#[ignore = "requires local Hype source blocks; run under the overall suite deadline"]
fn real_blocks_and_relocation_rows_match_independent_lib_lzo_observations() -> Result<()> {
    use sha2::{Digest, Sha256};
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("disc"));
    let level = game.join("Gamedata/World/Levels/astrolabe");
    let blocks = sna(&file(&level.join("astrolabe.sna"))?)?;
    for (key, length, digest) in [
        (
            BlockKey(5, 1),
            904349,
            "53bbd8d87ce6e4a6c12af64028f13ea7c793489ac61d362ce78468c6b2ec8897",
        ),
        (
            BlockKey(6, 2),
            1360321,
            "7c856793e4bdbcf55db34f9f1eb0630112736327ba8769f52938ff2d68e87532",
        ),
        (
            BlockKey(17, 1),
            103461,
            "beb80a6b1765224016c80b03f5ac6621f91e07df845b87378c3259595f85b8aa",
        ),
    ] {
        assert_eq!(blocks[&key].data.len(), length);
        assert_eq!(format!("{:x}", Sha256::digest(&blocks[&key].data)), digest);
    }
    for (ext, key, count, first, digest) in [
        (
            "rtb",
            BlockKey(5, 1),
            51616,
            0x09d00160,
            "bda4b8873e7817b2ec27f782d8e0e91549dd2ae3036f648585d907b195697db4",
        ),
        (
            "rtb",
            BlockKey(6, 2),
            15926,
            0x09370048,
            "7b553e238c3f4f98153ed975f17d38cc1fb564a13b9fb8702358c1d2420a92ab",
        ),
        (
            "rtb",
            BlockKey(17, 1),
            2380,
            0x09000024,
            "550ac3138dbc5b16e84911683a65b1bb09d46119283abbd894fcaa4aa9c9a5bd",
        ),
        (
            "rtp",
            BlockKey(5, 1),
            86,
            0x01bded28,
            "d1aaab8e8e2b78237bfe432d5213a7860e6d0868d60e868d78cb590875c2d9c3",
        ),
        (
            "rtt",
            BlockKey(5, 1),
            125,
            0x093f7190,
            "c94047490d5fd46d11f4418e3f177c6500757951582cb566894316bda53f4e28",
        ),
    ] {
        let groups = reloc(&file(&level.join(format!("astrolabe.{ext}")))?)?;
        let rows = &groups[&key];
        assert_eq!(rows.len(), count);
        assert_eq!(rows[0].value, first);
        let mut data = Vec::new();
        for row in rows {
            data.extend(encoded(row)?);
        }
        assert_eq!(format!("{:x}", Sha256::digest(&data)), digest);
    }
    let table = level_textures(&file(&level.join("astrolabe.ptx"))?)?;
    assert_eq!(
        (
            table.capacity,
            table.level_slots,
            table.loaded_textures,
            table.current_channel
        ),
        (1024, 589, 560, 2)
    );
    assert_eq!(table.pointers.iter().filter(|&&p| p != 0).count(), 125);
    Ok(())
}

#[test]
#[ignore = "requires local Hype source data; inspect one shared Fix geometry"]
fn shared_fix_geometry_13949_source_values_match_between_manoir_and_brigand() -> Result<()> {
    use crate::model::Selection;
    use anyhow::Context;

    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("disc"));
    let path = AssetPath::new("fix:/models/geometry/13949")?;
    let manoir_selection = Selection::hype("manoir", "en")?;
    let brigand_selection = Selection::hype("brigand", "en")?;
    let manoir = scene::import_scene(&game, &manoir_selection)?;
    let brigand = scene::import_scene(&game, &brigand_selection)?;
    let manoir_model = manoir
        .models
        .iter()
        .find(|model| model.path == path)
        .context("Manoir does not import geometry 13949")?;
    let brigand_model = brigand
        .models
        .iter()
        .find(|model| model.path == path)
        .context("Brigand does not import geometry 13949")?;
    assert!(
        manoir_model.positions == brigand_model.positions,
        "geometry positions differ"
    );
    let materials = |model: &crate::model::geometry::Model| {
        model
            .submeshes
            .iter()
            .map(|part| part.material.path.to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        materials(manoir_model),
        materials(brigand_model),
        "material identities differ"
    );
    for reference in manoir_model
        .submeshes
        .iter()
        .map(|part| &part.material.path)
    {
        let manoir_material = manoir
            .materials
            .iter()
            .find(|material| &material.path == reference)
            .context("Manoir is missing a referenced Fix material")?;
        let brigand_material = brigand
            .materials
            .iter()
            .find(|material| &material.path == reference)
            .context("Brigand is missing the same referenced Fix material")?;
        assert!(
            manoir_material == brigand_material,
            "material content changed under shared identity {reference}"
        );
    }
    assert!(
        manoir_model == brigand_model,
        "other shared geometry fields differ"
    );
    let manoir_sounds = import_sound_graph(&game, &manoir_selection, &manoir)?;
    let brigand_sounds = import_sound_graph(&game, &brigand_selection, &brigand)?;
    let sound_path = AssetPath::new("sound:/Sound/PA_Dragons.csb")?;
    let manoir_dragon_sounds = manoir_sounds
        .definitions
        .get(&sound_path)
        .context("Manoir sound graph does not import PA_Dragons.csb")?;
    let brigand_dragon_sounds = brigand_sounds
        .definitions
        .get(&sound_path)
        .context("Brigand sound graph does not import PA_Dragons.csb")?;
    assert_eq!(
        manoir_dragon_sounds, brigand_dragon_sounds,
        "shared sound definitions must resolve every canonical resource independent of the selected level"
    );

    let manoir_shared = shared_scene_values(&manoir, &manoir_sounds)?;
    let brigand_shared = shared_scene_values(&brigand, &brigand_sounds)?;
    let intersection: Vec<_> = manoir_shared
        .iter()
        .filter_map(|(path, left)| {
            brigand_shared
                .get(path)
                .map(|right| (path.as_str().to_owned(), left == right))
        })
        .collect();
    let differences: Vec<_> = intersection
        .iter()
        .filter(|(_, same)| !same)
        .map(|(path, _)| path.clone())
        .collect();
    assert!(
        differences.is_empty(),
        "shared source asset identity/content differs across Manoir and Brigand: {differences:?}"
    );
    eprintln!(
        "shared source audit: {} identities in the cross-map intersection; all Fix, sound, and texture values match",
        intersection.len()
    );
    Ok(())
}

fn import_sound_graph(
    game: &Path,
    selection: &crate::model::Selection,
    scene: &SceneImport,
) -> Result<crate::openspace::audio::graph::SoundGraph> {
    use crate::model::animation::AnimationOperation;

    crate::openspace::audio::graph::SoundGraph::import(
        game,
        selection,
        &scene.sound_definitions,
        scene
            .actions
            .iter()
            .filter_map(|action| match &action.operation {
                AnimationOperation::Sound { event: Some(event) } => Some(event),
                _ => None,
            })
            .chain(
                scene
                    .ai_models
                    .iter()
                    .flat_map(|model| model.sound_events()),
            )
            .chain(
                scene
                    .dialogues
                    .iter()
                    .filter_map(|dialogue| dialogue.voice.as_ref()),
            ),
    )
}

fn shared_scene_values(
    scene: &SceneImport,
    sounds: &crate::openspace::audio::graph::SoundGraph,
) -> Result<BTreeMap<AssetPath, Vec<u8>>> {
    fn insert<T: serde::Serialize>(
        result: &mut BTreeMap<AssetPath, Vec<u8>>,
        path: &AssetPath,
        value: &T,
    ) -> Result<()> {
        insert_value(result, path, serde_json::to_value(value)?)
    }
    fn insert_value(
        result: &mut BTreeMap<AssetPath, Vec<u8>>,
        path: &AssetPath,
        mut value: serde_json::Value,
    ) -> Result<()> {
        if path.role() != crate::model::PackageRole::Level {
            normalize_level_bindings(&mut value);
            let encoded = serde_json::to_vec(&value)?;
            if let Some(old) = result.get(path) {
                anyhow::ensure!(
                    old == &encoded,
                    "shared source identity {path} has two payloads"
                );
            } else {
                result.insert(path.clone(), encoded);
            }
        }
        Ok(())
    }
    fn normalize_level_bindings(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::String(path) if path.starts_with("level:/") => {
                *path = "level:/<session-bound-reference>".into();
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    normalize_level_bindings(value);
                }
            }
            serde_json::Value::Object(fields) => {
                for value in fields.values_mut() {
                    normalize_level_bindings(value);
                }
            }
            _ => {}
        }
    }

    let mut result = BTreeMap::new();
    macro_rules! insert_all {
        ($field:ident) => {
            for value in &scene.$field {
                insert(&mut result, &value.path, value)?;
            }
        };
    }
    if let Some(value) = &scene.dialogue_presentation {
        insert(&mut result, &value.path, value)?;
    }
    insert_all!(dialogues);
    // Fix AI definitions may carry paths resolved against the current level;
    // the exporter compiles that projection per level. The actor-derived label
    // is likewise not part of the shared source model identity.
    for value in &scene.ai_models {
        let mut normalized = serde_json::to_value(value)?;
        if let Some(fields) = normalized.as_object_mut() {
            fields.insert(
                "name".into(),
                serde_json::Value::String("<shared-ai-model>".into()),
            );
        }
        insert_value(&mut result, &value.path, normalized)?;
    }
    insert_all!(environments);
    insert_all!(physical_environments);
    insert_all!(visual_environments);
    insert_all!(fogs);
    insert_all!(collision_profiles);
    insert_all!(mechanics);
    insert_all!(actions);
    insert_all!(lights);
    insert_all!(sprites);
    insert_all!(particle_generators);
    insert_all!(families);
    insert_all!(persos);
    insert_all!(routes);
    insert_all!(waypoints);
    insert_all!(configurations);
    insert_all!(animations);
    insert_all!(transform_pools);
    insert_all!(models);
    insert_all!(collisions);
    insert_all!(instances);
    insert_all!(materials);
    insert(&mut result, &scene.scene.path, &scene.scene)?;
    for (path, value) in &sounds.definitions {
        insert(&mut result, path, value)?;
    }
    for (path, value) in &sounds.banks {
        insert(&mut result, path, value)?;
    }
    for (path, value) in &sounds.sounds {
        insert(&mut result, path, value)?;
    }
    Ok(result)
}

#[test]
fn radiosity_uses_signed_values_and_checks_selected_lod_and_buffer_bounds() -> Result<()> {
    let key = BlockKey(6, 2);
    // IPO -> ISI -> LOD -> one color. No engine allocator padding is included.
    let mut bytes = Vec::new();
    bytes.extend(encoded(&InstantiatedObjectWire {
        physical_object: 0,
        radiosity: 0x1008,
    })?);
    bytes.extend(encoded(&RadiosityWire {
        lod_count: 1,
        reserved: 0,
        lods: 0x1010,
    })?);
    bytes.extend(encoded(&RadiosityLodWire {
        vertex_count: 1,
        reserved: 0,
        colors: 0x1018,
    })?);
    bytes.extend(encoded(&RadiosityColorWire {
        components: [396, -238, i16::MIN, i16::MAX],
    })?);
    let mut c = SourceSession {
        fix_bindings: Default::default(),
        allocation_cache: Default::default(),
        blocks: BTreeMap::from([(
            Owner::Level,
            BTreeMap::from([(
                key,
                Block {
                    base: 0x1000,
                    data: bytes,
                },
            )]),
        )]),
        actual: BTreeMap::from([(key, Owner::Level)]),
        pointers: [0x1004, 0x100c, 0x1014]
            .into_iter()
            .map(|site| ((Owner::Level, key, site), key))
            .collect(),
        gpt: vec![],
        rtp: BTreeMap::new(),
        ptx: vec![],
        rtt: BTreeMap::new(),
    };
    let ipo = Address {
        owner: Owner::Level,
        key,
        offset: 0,
    };
    assert_eq!(
        c.radiosity(ipo, 0, 1, 1)?,
        [[396, -238, i16::MIN, i16::MAX]]
    );
    assert!(c.radiosity(ipo, 1, 1, 1).is_err());
    assert!(c.radiosity(ipo, 0, 2, 1).is_err());
    assert!(c.radiosity(ipo, 0, 1, 2).is_err());
    c.blocks
        .get_mut(&Owner::Level)
        .unwrap()
        .get_mut(&key)
        .unwrap()
        .data
        .pop();
    assert!(c.radiosity(ipo, 0, 1, 1).is_err());
    assert!(wire::<RadiosityWire>(&[255, 255, 0, 0, 0, 0, 0, 0]).is_err());
    assert!(wire::<RadiosityLodWire>(&[1, 0, 1, 0, 0, 0, 0, 0]).is_err());
    Ok(())
}
#[test]
fn source_local_bases_resolve_shared_blocks_and_invalid_references_fail() -> Result<()> {
    let mut level = vec![0; 16];
    level[0..4].copy_from_slice(&0x8004u32.to_le_bytes());
    level[8..12].copy_from_slice(&0x9000u32.to_le_bytes());
    let mut fix = BTreeMap::new();
    fix.insert(
        BlockKey(6, 1),
        Block {
            base: 0x2000,
            data: vec![42; 16],
        },
    );
    let mut lvl = BTreeMap::new();
    lvl.insert(
        BlockKey(6, 1),
        Block {
            base: 0x8000,
            data: Vec::new(),
        },
    );
    lvl.insert(
        BlockKey(6, 2),
        Block {
            base: 0x1000,
            data: level,
        },
    );
    let c = SourceSession {
        fix_bindings: Default::default(),
        allocation_cache: Default::default(),
        blocks: BTreeMap::from([(Owner::Fix, fix), (Owner::Level, lvl)]),
        actual: BTreeMap::from([(BlockKey(6, 1), Owner::Fix), (BlockKey(6, 2), Owner::Level)]),
        pointers: BTreeMap::from([
            ((Owner::Level, BlockKey(6, 2), 0x1000), BlockKey(6, 1)),
            ((Owner::Level, BlockKey(6, 2), 0x1008), BlockKey(6, 1)),
        ]),
        gpt: vec![],
        rtp: BTreeMap::new(),
        ptx: vec![],
        rtt: BTreeMap::new(),
    };
    let source = Address {
        owner: Owner::Level,
        key: BlockKey(6, 2),
        offset: 0,
    };
    let target = c.required(source, 0)?;
    assert_eq!(
        target,
        Address {
            owner: Owner::Fix,
            key: BlockKey(6, 1),
            offset: 4
        }
    );
    assert_eq!(c.bytes(target, 1)?, [42]);
    assert!(c.pointer(source, 4)?.is_none());
    assert!(c.pointer(source, 8).is_err());
    Ok(())
}
