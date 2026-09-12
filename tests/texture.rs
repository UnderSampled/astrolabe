use astrolabe::model::texture::*;
use astrolabe::openspace::texture::*;

fn rgba_texture() -> Texture {
    Texture {
        usage: TextureUsage::Material,
        colors: TextureColors::Components { bits: [5, 6, 5, 0] },
        levels: vec![TextureLevel {
            width: 2,
            height: 2,
            pixels: TexturePixels::Components(vec![
                [31, 0, 0, 0],
                [0, 63, 0, 0],
                [0, 0, 31, 0],
                [31, 63, 31, 0],
            ]),
        }],
        openspace_color_key: OpenSpaceColorKey {
            rgb: [0, 0, 0],
            palette_index: -1,
        },
    }
}

#[test]
fn exact_gf_fixture_retains_palette_mips_and_indices() {
    // Independently laid out 26-byte Montreal header, BGR palette, planar RLE.
    // The original encoder keeps the final channel sample out of preceding runs.
    let bytes = vec![
        1, 2, 0, 0, 0, 2, 0, 0, 0, 1, 0, 2, 0, 3, 0, 0, 0, 255, 255, 255, 255, 5, 0, 0, 0, 5, 0, 0,
        0, 0, 0, 255, 0, 1, 4, 1,
    ];
    let image = decode_gf(&bytes, TextureUsage::Material).unwrap();
    assert_eq!(
        (image.width(), image.height(), image.levels.len()),
        (2, 2, 2)
    );
    assert_eq!(
        image.colors,
        TextureColors::PaletteRgb(vec![[0, 0, 0], [255, 0, 0]])
    );
    assert_eq!(image.levels[0].pixels, TexturePixels::Indices(vec![1; 4]));
    assert_eq!(image.levels[1].pixels, TexturePixels::Indices(vec![1]));
    assert_eq!(image.rgba8(0).unwrap(), [255, 0, 0, 255].repeat(4));
    assert_eq!(encode_gf(&image).unwrap(), bytes);
    // A smaller edited asset must generate new counts and a different stream.
    let mut edited = image.clone();
    edited.levels.remove(0);
    let rebuilt = encode_gf(&edited).unwrap();
    assert!(rebuilt.len() < bytes.len());
    assert_eq!(decode_gf(&rebuilt, TextureUsage::Material).unwrap(), edited);
}

#[test]
fn direct_components_orientation_precision_and_rete_values() {
    for bits in [[5, 6, 5, 0], [5, 5, 5, 1], [4, 4, 4, 4]] {
        for usage in [TextureUsage::Material, TextureUsage::Screen] {
            let mut image = rgba_texture();
            image.usage = usage;
            image.colors = TextureColors::Components { bits };
            if let TexturePixels::Components(pixels) = &mut image.levels[0].pixels {
                for (i, pixel) in pixels.iter_mut().enumerate() {
                    for c in 0..4 {
                        pixel[c] = (i as u8 * 3 + c as u8) & ((1u16 << bits[c]) - 1) as u8;
                    }
                }
            }
            let bytes = encode_gf(&image).unwrap();
            assert_eq!(decode_gf(&bytes, usage).unwrap(), image);
            let rete = rmp_serde::to_vec_named(&image).unwrap();
            let reloaded: Texture = rmp_serde::from_slice(&rete).unwrap();
            assert_eq!(reloaded, image);
            assert_eq!(encode_gf(&reloaded).unwrap(), bytes);
        }
    }
    assert_eq!(
        rgba_texture().rgba8(0).unwrap(),
        vec![
            0, 0, 255, 255, 255, 255, 255, 255, 255, 0, 0, 255, 0, 255, 0, 255
        ]
    );
    // Palette alpha/index identity is retained even with duplicate color entries.
    let image = Texture {
        colors: TextureColors::PaletteRgba(vec![[1, 2, 3, 0], [1, 2, 3, 0], [4, 5, 6, 255]]),
        levels: vec![TextureLevel {
            width: 2,
            height: 2,
            pixels: TexturePixels::Indices(vec![0, 1, 2, 1]),
        }],
        ..rgba_texture()
    };
    assert_eq!(
        decode_gf(&encode_gf(&image).unwrap(), image.usage).unwrap(),
        image
    );
}

#[test]
fn cnt_regenerates_member_locations_checksums_and_directory_membership() {
    let archive = TextureArchive {
        directories: vec!["Empty".into(), "Textures/Étage".into()],
        textures: vec![
            NamedTexture {
                path: "Textures/Étage/First.gf".into(),
                texture: rgba_texture(),
                shared_with: None,
            },
            NamedTexture {
                path: "Second.gf".into(),
                texture: rgba_texture(),
                shared_with: None,
            },
        ],
    };
    assert_eq!(
        archive
            .find_by_source_name("textures\\étage\\FIRST.tga")
            .unwrap()
            .path,
        "Textures/Étage/First.gf"
    );
    let bytes = encode_cnt(&archive).unwrap();
    assert_eq!(decode_cnt(&bytes, TextureUsage::Material).unwrap(), archive);
    let mut edited = archive.clone();
    edited.textures[0].texture.levels[0] = TextureLevel {
        width: 1,
        height: 1,
        pixels: TexturePixels::Components(vec![[1, 2, 3, 0]]),
    };
    edited.textures.push(NamedTexture {
        path: "New/Third.gf".into(),
        texture: rgba_texture(),
        shared_with: None,
    });
    edited.directories.push("New".into());
    let rebuilt = encode_cnt(&edited).unwrap();
    assert_eq!(
        decode_cnt(&rebuilt, TextureUsage::Material).unwrap(),
        edited
    );
    let mut corrupted = bytes.clone();
    // Last two bytes are outside a complete CNT checksum word; corrupt header
    // data instead to independently exercise the regenerated word checksum.
    let pos = bytes.len() - encode_gf(&archive.textures[1].texture).unwrap().len();
    // Keep the preflight header structurally valid so this exercises checksum
    // verification specifically, rather than the earlier GF version check.
    corrupted[pos + 14] ^= 1;
    assert!(
        decode_cnt(&corrupted, TextureUsage::Material)
            .unwrap_err()
            .to_string()
            .contains("checksum")
    );
}

#[test]
fn malformed_gf_and_cnt_are_diagnosed() {
    let bytes = encode_gf(&rgba_texture()).unwrap();
    for end in 0..bytes.len() {
        assert!(
            decode_gf(&bytes[..end], TextureUsage::Material).is_err(),
            "accepted truncation at {end}"
        );
    }
    let mut bad = bytes.clone();
    bad[21..25].copy_from_slice(&3u32.to_le_bytes());
    assert!(decode_gf(&bad, TextureUsage::Material).is_err());
    let mut bad = bytes.clone();
    bad[26..29].copy_from_slice(&[bytes[10], 0, 0]);
    assert!(decode_gf(&bad, TextureUsage::Material).is_err());
    let mut bad = bytes.clone();
    bad[26..29].copy_from_slice(&[bytes[10], 0, 255]);
    assert!(decode_gf(&bad, TextureUsage::Material).is_err());
    let mut bad = bytes.clone();
    bad.push(0);
    assert!(decode_gf(&bad, TextureUsage::Material).is_err());
    let archive = TextureArchive {
        directories: vec![],
        textures: vec![NamedTexture {
            path: "../escape.gf".into(),
            texture: rgba_texture(),
            shared_with: None,
        }],
    };
    assert!(encode_cnt(&archive).is_err());
    assert!(decode_cnt(&[255; 64], TextureUsage::Material).is_err());
}

/// Independently walk only the source CNT directory to compare generated GF
/// bytes to actual source members; this is not a second production decoder.
fn original_members(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    fn word(bytes: &[u8], p: &mut usize) -> u32 {
        let n = u32::from_le_bytes(bytes[*p..*p + 4].try_into().unwrap());
        *p += 4;
        n
    }
    fn name(bytes: &[u8], p: &mut usize, k: u8) -> String {
        let n = word(bytes, p) as usize;
        let s = bytes[*p..*p + n].iter().map(|b| (b ^ k) as char).collect();
        *p += n;
        s
    }
    let (mut p, mut result) = (0, Vec::new());
    let nd = word(bytes, &mut p);
    let nf = word(bytes, &mut p);
    let key = if bytes[8] != 0 { bytes[10] } else { 0 };
    p = 11;
    let dirs: Vec<_> = (0..nd).map(|_| name(bytes, &mut p, key)).collect();
    p += 1;
    for _ in 0..nf {
        let dir = word(bytes, &mut p) as i32;
        let file = name(bytes, &mut p, key);
        let xor = word(bytes, &mut p).to_le_bytes();
        let _checksum = word(bytes, &mut p);
        let start = word(bytes, &mut p) as usize;
        let size = word(bytes, &mut p) as usize;
        let mut gf = bytes[start..start + size].to_vec();
        for i in 0..size / 4 * 4 {
            gf[i] ^= xor[i % 4];
        }
        result.push((
            (if dir < 0 {
                file
            } else {
                format!("{}/{}", dirs[dir as usize], file)
            })
            .replace('\\', "/"),
            gf,
        ));
    }
    result
}

// External comparison normalization: erase only CNT XOR obfuscation in a
// reference copy. The production exporter never receives this copy or its keys.
fn without_cnt_obfuscation(bytes: &[u8]) -> Vec<u8> {
    fn word(bytes: &[u8], p: &mut usize) -> u32 {
        let value = u32::from_le_bytes(bytes[*p..*p + 4].try_into().unwrap());
        *p += 4;
        value
    }
    fn string(output: &mut [u8], p: &mut usize, key: u8) {
        let length = word(output, p) as usize;
        for b in &mut output[*p..*p + length] {
            *b ^= key;
        }
        *p += length;
    }
    let mut output = bytes.to_vec();
    let mut p = 0;
    let directories = word(bytes, &mut p);
    let files = word(bytes, &mut p);
    let key = if bytes[8] == 1 { bytes[10] } else { 0 };
    output[8] = 0;
    output[10] = 0;
    p = 11;
    for _ in 0..directories {
        string(&mut output, &mut p, key);
    }
    p += 1;
    let mut visited = std::collections::HashSet::new();
    for _ in 0..files {
        p += 4;
        string(&mut output, &mut p, key);
        let key_start = p;
        let file_key = word(bytes, &mut p).to_le_bytes();
        output[key_start..key_start + 4].fill(0);
        p += 4;
        let start = word(bytes, &mut p) as usize;
        let length = word(bytes, &mut p) as usize;
        if visited.insert((start, length)) {
            for i in 0..length / 4 * 4 {
                output[start + i] ^= file_key[i % 4];
            }
        }
    }
    output
}

#[test]
#[ignore = "requires local Hype disc data; run through the bounded corpus test command"]
fn supplied_texture_corpus_decodes_and_reconstructs_every_gf() {
    use std::time::Instant;
    let start = Instant::now();
    for (file, usage, count) in [
        ("Textures.cnt", TextureUsage::Material, 3555),
        ("Vignette.cnt", TextureUsage::Screen, 23),
    ] {
        let game = std::env::var_os("ASTROLABE_GAME_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("disc"));
        let source = std::fs::read(game.join("Gamedata").join(file))
            .expect("supplied game archive is required");
        use sha2::{Digest, Sha256};
        let expected_hash = if file == "Textures.cnt" {
            "bde3b7951b0809b18f556fc09eaa58d22f9cdbd1ae066c8684d72fb1674121da"
        } else {
            "8a221aca15583ca94763f5f03047951909309a4c7cdd114acb37f0eb541b30f4"
        };
        assert_eq!(
            format!("{:x}", Sha256::digest(&source)),
            expected_hash,
            "source edition/fixture changed"
        );
        let original = original_members(&source);
        let archive = decode_cnt(&source, usage).unwrap();
        assert_eq!(archive.textures.len(), count);
        if file == "Textures.cnt" {
            assert_eq!(
                archive
                    .textures
                    .iter()
                    .filter(|t| t.shared_with.is_some())
                    .count(),
                214
            );
            let argb = &archive
                .find_by_source_name("astrolabe/2fentxynz.tga")
                .unwrap()
                .texture;
            assert_eq!(
                (argb.width(), argb.height(), argb.levels.len()),
                (256, 256, 9)
            );
            assert_eq!(
                argb.colors,
                TextureColors::Components { bits: [5, 5, 5, 1] }
            );
            assert_eq!(argb.openspace_color_key.rgb, [84, 45, 24]);
            if let TexturePixels::Components(p) = &argb.levels[0].pixels {
                assert_eq!(p[0], [12, 6, 0, 1]);
            } else {
                panic!("expected direct color");
            }
            let indexed = &archive
                .find_by_source_name("Monast1/2mon1txy.tga")
                .unwrap()
                .texture;
            assert_eq!(
                (indexed.width(), indexed.height(), indexed.levels.len()),
                (64, 128, 8)
            );
            if let TexturePixels::Indices(p) = &indexed.levels[0].pixels {
                assert_eq!(&p[..8], &[133, 127, 139, 142, 169, 139, 142, 157]);
            } else {
                panic!("expected indices");
            }
            if let TextureColors::PaletteRgb(p) = &indexed.colors {
                assert_eq!(p[1], [157, 185, 193]);
            } else {
                panic!("expected RGB palette");
            }
            let argb4 = &archive
                .find_by_source_name("FixTex/MOTORTXT/menus/567.tga")
                .unwrap()
                .texture;
            if let TexturePixels::Components(p) = &argb4.levels[0].pixels {
                assert_eq!(p[0], [15, 7, 2, 0]);
            } else {
                panic!("expected direct color");
            }
        } else {
            let screen = &archive
                .find_by_source_name("INVENTOR/Background.tga")
                .unwrap()
                .texture;
            assert_eq!(
                (screen.width(), screen.height(), screen.levels.len()),
                (640, 480, 1)
            );
            assert_eq!(&screen.rgba8(0).unwrap()[..4], &[49, 16, 0, 255]);
        }
        for (named, (path, bytes)) in archive.textures.iter().zip(original.iter()) {
            assert_eq!(&named.path, path);
            let encoded = encode_gf(&named.texture).unwrap();
            assert!(
                encoded == *bytes,
                "GF bytes differ for {} at {:?} ({} vs {} bytes)",
                named.path,
                encoded.iter().zip(bytes).position(|(a, b)| a != b),
                encoded.len(),
                bytes.len()
            );
            assert_eq!(decode_gf(bytes, usage).unwrap(), named.texture);
        }
        let reconstructed = encode_cnt(&archive).unwrap();
        assert!(
            reconstructed == without_cnt_obfuscation(&source),
            "CNT bytes differ beyond intentionally removed XOR obfuscation: {file}"
        );
        let recovered = decode_cnt(&reconstructed, usage).unwrap();
        assert!(recovered == archive, "rebuilt CNT model differs");
        println!(
            "{file}: {count} GF members byte-exact; rebuilt CNT {} bytes (original {})",
            reconstructed.len(),
            source.len()
        );
    }
    println!("texture corpus elapsed: {:?}", start.elapsed());
}

#[test]
fn archive_sharing_is_explicit_and_detachable() {
    let texture = rgba_texture();
    let mut archive = TextureArchive {
        directories: vec![],
        textures: vec![
            NamedTexture {
                path: "first.gf".into(),
                texture: texture.clone(),
                shared_with: None,
            },
            NamedTexture {
                path: "second.gf".into(),
                texture,
                shared_with: Some("first.gf".into()),
            },
        ],
    };
    let shared = encode_cnt(&archive).unwrap();
    assert_eq!(
        decode_cnt(&shared, TextureUsage::Material).unwrap(),
        archive
    );
    archive.textures[1].texture.openspace_color_key.rgb = [1, 2, 3];
    assert!(encode_cnt(&archive).is_err());
    archive.textures[1].shared_with = None;
    let detached = encode_cnt(&archive).unwrap();
    assert!(detached.len() > shared.len());
    assert_eq!(
        decode_cnt(&detached, TextureUsage::Material).unwrap(),
        archive
    );
}
