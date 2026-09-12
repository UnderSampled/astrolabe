use super::*;
#[test]
fn generated_sna_block_preserves_values_and_resolves_generated_relocations() -> Result<()> {
    let m = super::tests::model();
    let targets = BTreeMap::from([(
        m.submeshes[0].material.path.clone(),
        MaterialTarget {
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
