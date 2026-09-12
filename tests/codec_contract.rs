//! The same typed codec consumer inspects and edits useful values across formats.
use astrolabe::{
    model::{
        io::Content,
        texture::{NamedTexture, Texture, TextureArchive, TextureColors, TextureUsage},
    },
    openspace::codec::{Cnt, FormatCodec, Gf, SoundScript},
};
use std::fmt::Debug;

// Independently laid out Montreal GF: 2x2 red indexed image plus 1x1 mip,
// two BGR palette entries, and a four-sample RLE run plus terminal sample.
const RED_GF: &[u8] = &[
    1, 2, 0, 0, 0, 2, 0, 0, 0, 1, 0, 2, 0, 3, 0, 0, 0, 255, 255, 255, 255, 5, 0, 0, 0, 5, 0, 0, 0,
    0, 0, 255, 0, 1, 4, 1,
];
const SELECTORS: &[u8] =
    b"{SIF_Type:Material[%ld](1)\n{SIF_Value:Pierre\xe9\nSetValue[%ld](3)\n}\n}\n";

/// A caller needs one operation shape, with format-specific meaning checks.
/// This harness deliberately does not inspect an intermediate wire-record model.
fn edit_content<C>(
    codec: &C,
    source: &[u8],
    edit: impl FnOnce(&mut C::Value),
    inspect_result: impl FnOnce(&C::Value),
) -> Vec<u8>
where
    C: FormatCodec,
    C::Value: Content + PartialEq + Debug,
{
    let support = codec.support();
    assert!(support.reads && support.writes);
    assert!(!support.format.is_empty() && !support.notes.is_empty());
    let mut value = codec.read(source).unwrap();
    value.validate().unwrap();
    edit(&mut value);
    value.validate().unwrap();
    let rebuilt = codec.write(&value).unwrap();
    assert_ne!(rebuilt, source, "the useful edit must reach the output");
    let restored = codec.read(&rebuilt).unwrap();
    restored.validate().unwrap();
    inspect_result(&restored);
    assert_eq!(restored, value);
    rebuilt
}

#[test]
fn one_content_codec_consumer_resizes_and_recolors_a_texture() {
    let codec = Gf {
        usage: TextureUsage::Material,
    };
    let rebuilt = edit_content(
        &codec,
        RED_GF,
        |image| {
            assert_eq!(
                (image.width(), image.height(), image.levels.len()),
                (2, 2, 2)
            );
            assert_eq!(image.rgba8(0).unwrap(), [255, 0, 0, 255].repeat(4));
            image.levels.remove(0);
            let TextureColors::PaletteRgb(palette) = &mut image.colors else {
                panic!("known GF fixture must expose an editable palette")
            };
            palette[1] = [10, 200, 30];
        },
        |image| {
            assert_eq!(
                (image.width(), image.height(), image.levels.len()),
                (1, 1, 1)
            );
            assert_eq!(image.rgba8(0).unwrap(), [10, 200, 30, 255]);
        },
    );
    assert!(
        rebuilt.len() < RED_GF.len(),
        "resizing regenerates the GF payload"
    );
    assert_eq!(u32::from_le_bytes(rebuilt[1..5].try_into().unwrap()), 1);
    assert_eq!(u32::from_le_bytes(rebuilt[21..25].try_into().unwrap()), 1);
}

#[test]
fn same_content_codec_consumer_edits_named_sound_selectors() {
    let codec = SoundScript {
        name: "materials.sif".into(),
    };
    let rebuilt = edit_content(
        &codec,
        SELECTORS,
        |definitions| {
            assert_eq!(definitions.selectors[0].name, "Material");
            assert_eq!(definitions.selectors[0].values, [("Pierreé".into(), 3)]);
            definitions.selectors[0].values[0] = ("PierreÉ".into(), 7);
            definitions.selectors[0].values.push(("Wood".into(), 4));
        },
        |definitions| {
            assert_eq!(definitions.selectors[0].id, 1);
            assert_eq!(
                definitions.selectors[0].values,
                [("PierreÉ".into(), 7), ("Wood".into(), 4)]
            );
        },
    );
    assert!(rebuilt.contains(&0xc9), "source text uses one-byte Latin-1");
    assert!(!rebuilt.windows(2).any(|pair| pair == [0xc3, 0x89]));
}

#[test]
fn unsupported_sound_text_is_rejected_before_lossy_encoding() {
    let codec = SoundScript {
        name: "materials.sif".into(),
    };
    for name in ["Silver €", "Dragon 🐉"] {
        let mut definitions = codec.read(SELECTORS).unwrap();
        definitions.selectors[0].values[0].0 = name.into();
        let error = codec.write(&definitions).unwrap_err();
        assert!(error.to_string().contains("Latin-1"), "{error:#}");
    }
}

#[test]
fn codec_usage_mismatch_does_not_silently_reinterpret_texture_content() {
    let material = Gf {
        usage: TextureUsage::Material,
    }
    .read(RED_GF)
    .unwrap();
    assert!(
        Gf {
            usage: TextureUsage::Screen
        }
        .write(&material)
        .is_err()
    );
    let archive = TextureArchive {
        directories: vec![],
        textures: vec![NamedTexture {
            path: "Red.gf".into(),
            texture: material,
            shared_with: None,
        }],
    };
    assert!(
        Cnt {
            usage: TextureUsage::Screen
        }
        .write(&archive)
        .is_err()
    );
}

#[test]
fn malformed_sound_labels_and_excessive_nesting_return_diagnostics() {
    let codec = SoundScript {
        name: "invalid.sif".into(),
    };
    for bytes in [
        b"{SIF_Type\n}\n".as_slice(),
        b"{SIF_Type:Material[%ld](1)\n{SIF_Value\nSetValue(3)\n}\n}\n",
        b"{LcbHeader:\n}\n{SndMapE\n}\n",
        b"{SIF_Type:Material[%ld](1)\n{SIF_Value:Stone\nSetValue(3)\n}\n",
    ] {
        assert!(
            codec.read(bytes).is_err(),
            "accepted malformed script: {bytes:?}"
        );
    }
    let nested = format!(
        "{}{}",
        "{SIF_Type:Material[%ld](1)\n".repeat(65),
        "}\n".repeat(65)
    );
    assert!(codec.read(nested.as_bytes()).is_err());
}

#[test]
fn rete_untrusted_lengths_and_truncated_headers_are_rejected() {
    // Huge declared strings/arrays/maps carry no payload; they must not be
    // accepted as content or require allocation proportional to those counts.
    for bytes in [
        &[0xdb, 0xff, 0xff, 0xff, 0xff][..],
        &[0xdd, 0xff, 0xff, 0xff, 0xff],
        &[0xdf, 0xff, 0xff, 0xff, 0xff],
    ] {
        assert!(astrolabe::rete::decode::<Texture>(bytes).is_err());
    }
    let codec = Gf {
        usage: TextureUsage::Material,
    };
    for end in 0..RED_GF.len() {
        assert!(codec.read(&RED_GF[..end]).is_err());
    }
    let mut huge = RED_GF.to_vec();
    huge[21..25].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(codec.read(&huge).is_err());
    assert!(
        Cnt {
            usage: TextureUsage::Material
        }
        .read(&[255; 32])
        .is_err()
    );
}

#[test]
fn exhausted_sound_group_ids_return_diagnostics_in_every_build_profile() {
    let codec = SoundScript {
        name: "library.lcb".into(),
    };
    let bytes = b"{LcbHeader:\n}\n{SndEventGroupList:\nLoadEventGroup(Test,4294967295)\n}\n";
    let definitions = codec.read(bytes).unwrap();
    assert_eq!(definitions.groups[0].id, u32::MAX);
    let error = codec.write(&definitions).unwrap_err();
    assert!(error.to_string().contains("next free u32 ID"), "{error:#}");
}
