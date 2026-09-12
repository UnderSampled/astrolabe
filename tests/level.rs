use anyhow::Result;
use astrolabe::{
    model::{
        Selection,
        geometry::{Material, Model, ModelInstance},
    },
    openspace::level::{
        MaterialTarget, import_first_model, write_geometry, write_model_fragment, write_sna_block,
    },
    rete,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf};
fn hash(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
/// This uses local user-owned content; the source hash and independently observed values are fixed.
#[test]
#[ignore = "requires the local Hype astrolabe disc fixture; run explicitly with --ignored"]
fn real_rooted_model_survives_rete_edit_and_source_free_reconstruction() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("disc"));
    let sna = std::fs::read(game.join("Gamedata/World/Levels/astrolabe/astrolabe.sna"))?;
    assert_eq!(
        hash(&sna),
        "0a678d2d15b5850e8847bfbfa13e2d22bfae3b4e1df099a0309ae844305bf772"
    );
    let import = import_first_model(&game, &Selection::hype("astrolabe", "EN")?)?;
    assert!(import.source.actual_world.contains("0x093f19f4"));
    assert!(import.source.dynamic_world.contains("0x093f182c"));
    assert_eq!(
        (import.source.rooted_nodes, import.source.rooted_models),
        (150, 22)
    );
    assert_eq!(import.source.texture_references, 125);
    for (name, length) in [
        ("Level:05:01", 904349),
        ("Level:06:02", 1360321),
        ("Level:11:01", 103461),
    ] {
        assert!(
            import
                .source
                .populated_blocks
                .contains(&(name.to_string(), length))
        );
    }
    let original = &import.model;
    assert_eq!(original.name, "world/1/0/0");
    assert_eq!(original.positions.len(), 26);
    assert_eq!(
        original
            .submeshes
            .iter()
            .map(|s| s.triangles.len())
            .collect::<Vec<_>>(),
        [13, 5, 2, 12]
    );
    assert_eq!(original.submeshes[0].triangles[0], [3, 0, 14]);
    assert_eq!(original.submeshes[0].uv_indices[0], [6, 7, 8]);
    assert_eq!(original.vertex_materials.len(), 26);
    assert_eq!(
        import.materials[0].texture.as_ref().unwrap().as_str(),
        "texture:/astrolabe/2ze01txy.tga"
    );
    let positions: Vec<u8> = original
        .positions
        .iter()
        .flatten()
        .flat_map(|v| v.to_bits().to_le_bytes())
        .collect();
    assert_eq!(
        hash(&positions),
        "2547f1d553a966352bfad4e91ad1204375c5797fbbdc509689e231bc67753874"
    );
    assert_eq!(import.instance.radiosity.len(), 26);
    assert_eq!(import.instance.radiosity[0], [133, 41, 0, 0]);
    assert_eq!(import.instance.radiosity[2], [143, 52, -57, 0]);
    assert_eq!(import.instance.radiosity[11], [396, 48, -238, 0]);
    assert_eq!(
        import.instance.radiosity_rgba(11),
        Some([396. / 256., 48. / 256., -238. / 256., 0.])
    );
    assert!(import.instance.radiosity_rgba(26).is_none());
    let colors: Vec<u8> = import
        .instance
        .radiosity
        .iter()
        .flatten()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    assert_eq!(
        hash(&colors),
        "46a6eb8f51f9a3607ec340d87cdcccea481aed6e7a99575b8c14a6c7c9019fa5"
    );
    let normals: Vec<u8> = original
        .normals
        .iter()
        .flatten()
        .flat_map(|v| v.to_bits().to_le_bytes())
        .collect();
    assert_eq!(
        hash(&normals),
        "95f8bee09620dc162b7df1617cfc5bb31937842c8802ab24eb6c043f7d9ded57"
    );
    let bytes = rete::encode(original)?;
    let mut reloaded: Model = rete::decode(&bytes)?;
    assert_eq!(&reloaded, original);
    let mut instance: ModelInstance = rete::decode(&rete::encode(&import.instance)?)?;
    assert_eq!(instance, import.instance);
    let materials: Vec<Material> = import
        .materials
        .iter()
        .map(|m| rete::decode(&rete::encode(m).unwrap()).unwrap())
        .collect();
    let targets: BTreeMap<_, _> = materials
        .iter()
        .enumerate()
        .map(|(i, m)| {
            (
                m.path.clone(),
                MaterialTarget {
                    address: 0x10000 + i as u32 * 0x100,
                    block: (6, 1),
                },
            )
        })
        .collect();
    let rebuilt = write_geometry(&reloaded, &targets, 0x1000)?;
    let range = rebuilt
        .ranges
        .iter()
        .find(|r| r.name == "positions")
        .unwrap();
    assert_eq!(
        &rebuilt.bytes[range.offset..range.offset + range.length],
        &positions
    );
    let first = write_model_fragment(&reloaded, &instance, &materials, 0x1000)?;
    assert_eq!(first.ptx.len(), 8208);
    let word = |b: &[u8], at: usize| u32::from_le_bytes(b[at..at + 4].try_into().unwrap());
    assert_eq!(
        (
            word(&first.ptx, 0),
            word(&first.ptx, 4),
            word(&first.ptx, 4104),
            word(&first.ptx, 8204)
        ),
        (1024, 1024, 4, 1)
    );
    let pointer_values: std::collections::BTreeSet<_> = first.ptx[8..4104]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| word(b, 0))
        .filter(|&v| v != 0)
        .collect();
    let expected_targets: std::collections::BTreeSet<_> = first
        .ranges
        .iter()
        .filter(|r| r.name.starts_with("textures/"))
        .map(|r| 0x1000 + r.offset as u32)
        .collect();
    assert_eq!(pointer_values, expected_targets);
    assert_eq!((&first.rtt[..3], word(&first.rtt, 3)), (&[1, 5, 1][..], 4));
    let (rows, consumed) =
        astrolabe::openspace::compression::decode_envelope(&first.rtt[7..], 1024)?;
    assert_eq!(consumed + 7, first.rtt.len());
    let rebuilt_rtt: std::collections::BTreeSet<_> = rows
        .as_chunks::<8>()
        .0
        .iter()
        .map(|row| {
            assert_eq!(&row[4..8], &[6, 2, 0, 0]);
            word(row, 0)
        })
        .collect();
    assert_eq!(rebuilt_rtt, pointer_values);
    let framed = write_sna_block(&first.bytes, (6, 2), 0x1000)?;
    assert_eq!(&framed[..2], &[6, 2]);
    assert_eq!(word(&framed, 2), 0x1000);
    assert_eq!(word(&framed, 18), first.bytes.len() as u32);
    let (decoded, consumed) =
        astrolabe::openspace::compression::decode_envelope(&framed[22..], 16 * 1024 * 1024)?;
    assert_eq!(consumed + 22, framed.len());
    assert_eq!(decoded, first.bytes);
    // 59 geometry/material-table + 8 material + 4 PO/LOD + 4 IPO/ISI/LOD sites.
    assert_eq!(first.pointer_count, 75);
    let color_range = first
        .ranges
        .iter()
        .find(|r| r.name == "radiosity/colors")
        .unwrap();
    assert_eq!(
        &first.bytes[color_range.offset..color_range.offset + color_range.length],
        &colors
    );
    let at =
        |offset: usize| u32::from_le_bytes(first.bytes[offset..offset + 4].try_into().unwrap());
    let isi_offset = (at(first.instantiated_object_offset + 4) - 0x1000) as usize;
    let lod_offset = (at(isi_offset + 4) - 0x1000) as usize;
    assert_eq!(
        at(first.instantiated_object_offset),
        0x1000 + first.physical_object_offset as u32
    );
    assert_eq!(at(isi_offset), 1);
    assert_eq!(at(lod_offset), 26);
    assert_eq!(at(lod_offset + 4), 0x1000 + color_range.offset as u32);
    let mut invalid = instance.clone();
    invalid.radiosity.pop();
    assert!(invalid.validate_model(&reloaded).is_err());
    assert!(write_model_fragment(&reloaded, &invalid, &materials, 0x1000).is_err());
    instance.radiosity[2][2] = -128;
    reloaded.positions[0][0] += 1.;
    let mut edited_materials = materials.clone();
    edited_materials[0].color[0] = 0.25;
    let changed = write_model_fragment(&reloaded, &instance, &edited_materials, 0x8000)?;
    assert_ne!(first.bytes, changed.bytes);
    let edited_colors = changed
        .ranges
        .iter()
        .find(|r| r.name == "radiosity/colors")
        .unwrap();
    assert_eq!(
        &changed.bytes[edited_colors.offset + 20..edited_colors.offset + 22],
        &(-128i16).to_le_bytes()
    );
    let vm = changed
        .ranges
        .iter()
        .find(|r| r.name.ends_with("materials/0/visual"))
        .unwrap();
    assert_eq!(
        &changed.bytes[vm.offset + 52..vm.offset + 56],
        &0.25f32.to_le_bytes()
    );
    assert_eq!(
        write_model_fragment(&reloaded, &instance, &edited_materials, 0x8000)?.bytes,
        changed.bytes
    );
    // Resolving a .tga dependency to its actual shared .gf content must preserve the authored name.
    let mut resolved_materials = materials.clone();
    let original_name = resolved_materials[0]
        .texture_info
        .as_ref()
        .unwrap()
        .source_name
        .clone()
        .unwrap();
    resolved_materials[0].texture = Some(astrolabe::model::AssetPath::new(
        "texture:/astrolabe/shared-alias.gf",
    )?);
    let resolved = write_model_fragment(&reloaded, &instance, &resolved_materials, 0x4000)?;
    let texture = resolved
        .ranges
        .iter()
        .find(|r| r.name.starts_with("textures/") && r.name.contains("shared-alias.gf"))
        .unwrap();
    assert_eq!(
        &resolved.bytes[texture.offset + 100..texture.offset + 100 + original_name.len()],
        original_name.as_bytes()
    );
    assert!(
        import
            .diagnostics
            .iter()
            .any(|d| d.contains("26 exact signed"))
    );
    Ok(())
}
