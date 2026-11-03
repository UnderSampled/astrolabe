use anyhow::{Context, Result, ensure};
use astrolabe::{
    godot,
    model::{
        AssetPath, AssetRef, Selection,
        collision::CollisionProperties,
        geometry::{Material, TextureInfo, VisualMaterial},
        io::{Load, Save},
        texture::{
            OpenSpaceColorKey, Texture, TextureColors, TextureLevel, TexturePixels, TextureUsage,
        },
    },
    rete,
};
use std::{fs, path::Path, time::Duration};

fn level(width: u32, height: u32, seed: u8) -> TextureLevel {
    let pixels = (0..width * height)
        .map(|index| {
            let index = index as u8;
            [
                seed.wrapping_add(index),
                17u8.wrapping_add(index),
                201u8.wrapping_sub(index),
                255u8.wrapping_sub(index),
            ]
        })
        .collect();
    TextureLevel {
        width,
        height,
        pixels: TexturePixels::Components(pixels),
    }
}

fn component_texture(levels: Vec<TextureLevel>) -> Texture {
    Texture {
        usage: TextureUsage::Material,
        colors: TextureColors::Components { bits: [8, 8, 8, 8] },
        levels,
        openspace_color_key: OpenSpaceColorKey {
            rgb: [84, 45, 24],
            palette_index: -1,
        },
    }
}

fn keyed_texture() -> Texture {
    let indices = [
        (4, 2, vec![0, 1, 0, 1, 1, 0, 1, 0]),
        (2, 1, vec![0, 1]),
        (1, 1, vec![0]),
    ];
    Texture {
        usage: TextureUsage::Screen,
        colors: TextureColors::PaletteRgba(vec![[20, 30, 40, 200], [20, 30, 40, 200]]),
        levels: indices
            .into_iter()
            .map(|(width, height, indices)| TextureLevel {
                width,
                height,
                pixels: TexturePixels::Indices(indices),
            })
            .collect(),
        openspace_color_key: OpenSpaceColorKey {
            rgb: [20, 30, 40],
            palette_index: 0,
        },
    }
}

fn material(
    name: &str,
    texture: &AssetRef<Texture>,
    dimensions: [u32; 2],
    texture_flags: u32,
) -> Result<Material> {
    Ok(Material {
        path: AssetPath::new(format!("level:/materials/{name}"))?,
        visual: Some(VisualMaterial {
            texture: Some(texture.path.clone()),
            texture_info: Some(TextureInfo {
                source_name: None,
                stored_size: dimensions,
                logical_size: dimensions,
                flags: texture_flags,
                addressing: 0,
            }),
            ambient: [1.; 4],
            diffuse: [0.; 4],
            specular: [0.; 4],
            color: [1.; 4],
            flags: 0x20000209,
            scroll_speed: [0.; 2],
            scroll_position: [0.; 2],
            scroll_mode: 0,
        }),
        sound_material: 0,
        collision: CollisionProperties::Absent,
    })
}

fn packed_bytes(bytes: &[u8]) -> String {
    format!(
        "PackedByteArray([{}])",
        bytes
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn expected_keyed_bytes(texture: &Texture) -> Result<Vec<u8>> {
    let TexturePixels::Indices(_) = &texture.levels[0].pixels else {
        anyhow::bail!("keyed mip fixture is not palette based")
    };
    let mut expected = Vec::new();
    for (level_index, level) in texture.levels.iter().enumerate() {
        let mut rgba = texture.rgba8(level_index)?;
        let TexturePixels::Indices(indices) = &level.pixels else {
            unreachable!()
        };
        for (index, pixel) in rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            if indices[index] as i32 == texture.openspace_color_key.palette_index {
                pixel[3] = 0;
            }
        }
        expected.extend(rgba);
    }
    Ok(expected)
}

fn asset_path(manifest: &godot::Manifest, suffix: &str) -> Result<String> {
    manifest
        .files
        .keys()
        .find(|path| path.ends_with(suffix))
        .cloned()
        .with_context(|| format!("generated output ending in {suffix} is missing"))
}

fn check_probe_syntax(engine: &str, project: &Path, script: &[u8]) -> Result<()> {
    fs::create_dir_all(project)?;
    fs::write(project.join("project.godot"), "config_version=5\n")?;
    fs::write(project.join("check.gd"), script)?;
    let log = project.join("godot-parser.log");
    godot::run_godot(
        engine,
        project,
        &[
            "--headless",
            "--log-file",
            log.to_str().context("non-UTF-8 mip parser log")?,
            "--check-only",
            "--script",
            "res://check.gd",
        ],
        Duration::from_secs(10),
    )?;
    ensure!(
        fs::metadata(&log)?.len() <= 1024 * 1024,
        "mip parser log exceeds 1MiB"
    );
    let diagnostics = fs::read_to_string(log)?;
    ensure!(
        !diagnostics.contains("SCRIPT ERROR:") && !diagnostics.contains("ERROR:"),
        "mip regression parser log reported errors: {diagnostics}"
    );
    Ok(())
}

#[test]
#[ignore = "requires local Godot"]
fn source_mip_chains_survive_rete_and_native_godot_projection() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let selection = Selection::hype("mip-material-test", "en")?;
    let full_ref = AssetRef::<Texture>::new("texture:/mips/full.gf")?;
    let partial_ref = AssetRef::<Texture>::new("texture:/mips/partial.gf")?;
    let base_ref = AssetRef::<Texture>::new("texture:/mips/base_only.gf")?;
    let keyed_ref = AssetRef::<Texture>::new("texture:/mips/keyed.gf")?;
    let full = component_texture(vec![level(4, 2, 11), level(2, 1, 71), level(1, 1, 151)]);
    let partial = component_texture(vec![level(4, 2, 23), level(2, 1, 93)]);
    let base_only = component_texture(vec![level(8, 1, 31)]);
    let keyed = keyed_texture();
    let mut store = rete::Store::new(temp.path().join("rete"), selection.clone())?;
    store.save(&full_ref, &full)?;
    store.save(&partial_ref, &partial)?;
    store.save(&base_ref, &base_only)?;
    store.save(&keyed_ref, &keyed)?;
    drop(store);
    let mut store = rete::Store::new(temp.path().join("rete"), selection.clone())?;
    let loaded_full = store.load(&full_ref)?;
    let loaded_partial = store.load(&partial_ref)?;
    let loaded_base = store.load(&base_ref)?;
    let loaded_keyed = store.load(&keyed_ref)?;
    assert_eq!(*loaded_full, full);
    assert_eq!(*loaded_partial, partial);
    assert_eq!(*loaded_base, base_only);
    assert_eq!(*loaded_keyed, keyed);

    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let preflight = temp.path().join("preflight");
    let syntax_probe = include_str!("check_texture_mips.gd")
        .replace("FULL_PNG", "res://placeholder.png")
        .replace("FULL_MIPS", "res://placeholder.mips.dds")
        .replace("PARTIAL_MIPS", "res://placeholder-partial.mips.dds")
        .replace("BASE_MIPS", "res://placeholder-base.mips.dds")
        .replace("KEYED_MIPS", "res://placeholder-keyed.mips.dds")
        .replace("FULL_BYTES", "PackedByteArray()")
        .replace("PARTIAL_PREFIX", "PackedByteArray()")
        .replace("PARTIAL_LAST", "PackedByteArray()")
        .replace("BASE_BYTES", "PackedByteArray()")
        .replace("KEYED_BYTES", "PackedByteArray()");
    check_probe_syntax(&engine, &preflight, syntax_probe.as_bytes())?;
    let output = temp.path().join("godot");
    let mut project = godot::Project::new(&output, selection)?;
    project.save(&full_ref, &loaded_full)?;
    project.save(&partial_ref, &loaded_partial)?;
    project.save(&base_ref, &loaded_base)?;
    project.save(&keyed_ref, &loaded_keyed)?;
    for (name, reference, dimensions, flags) in [
        ("full", &full_ref, [4, 2], 0),
        ("partial", &partial_ref, [4, 2], 0),
        ("base-only", &base_ref, [8, 1], 0),
        ("keyed", &keyed_ref, [4, 2], 2),
    ] {
        project.write_material(&material(name, reference, dimensions, flags)?)?;
    }
    let manifest = project.finish(&engine)?;
    assert!(manifest.diagnostics.iter().any(|note| {
        note.contains("partial.gf.mips.dds") && note.contains("1 tail level(s) generated")
    }));
    assert!(manifest.diagnostics.iter().any(|note| {
        note.contains("base_only.gf.mips.dds") && note.contains("3 tail level(s) generated")
    }));
    assert!(manifest.diagnostics.iter().any(|note| {
        note.contains("full.gf.mips.dds") && note.contains("0 tail level(s) generated")
    }));

    let full_png = asset_path(&manifest, "full.gf.png")?;
    let full_mips = asset_path(&manifest, "full.gf.mips.dds")?;
    let partial_mips = asset_path(&manifest, "partial.gf.mips.dds")?;
    let base_mips = asset_path(&manifest, "base_only.gf.mips.dds")?;
    let keyed_mips = asset_path(&manifest, "keyed.gf.color-key.mips.dds")?;
    let check = include_str!("check_texture_mips.gd")
        .replace("FULL_PNG", &format!("res://{full_png}"))
        .replace("FULL_MIPS", &format!("res://{full_mips}"))
        .replace("PARTIAL_MIPS", &format!("res://{partial_mips}"))
        .replace("BASE_MIPS", &format!("res://{base_mips}"))
        .replace("KEYED_MIPS", &format!("res://{keyed_mips}"))
        .replace("FULL_BYTES", &packed_bytes(&full_rgba(&full)?))
        .replace("PARTIAL_PREFIX", &packed_bytes(&prefix_rgba(&partial)?))
        .replace("PARTIAL_LAST", &packed_bytes(&partial.rgba8(1)?))
        .replace("BASE_BYTES", &packed_bytes(&base_rgba(&base_only)?))
        .replace("KEYED_BYTES", &packed_bytes(&expected_keyed_bytes(&keyed)?));
    fs::create_dir_all(output.join("logs"))?;
    fs::write(output.join("check.gd"), check)?;
    let log = output.join("logs/mip-regression.log");
    godot::run_godot(
        &engine,
        &output,
        &[
            "--headless",
            "--log-file",
            log.to_str().context("non-UTF-8 mip runtime log")?,
            "--script",
            "res://check.gd",
        ],
        Duration::from_secs(30),
    )?;
    assert_eq!(*store.load(&full_ref)?, full);
    Ok(())
}

fn full_rgba(texture: &Texture) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    for index in 0..texture.levels.len() {
        bytes.extend(texture.rgba8(index)?);
    }
    Ok(bytes)
}

fn prefix_rgba(texture: &Texture) -> Result<Vec<u8>> {
    full_rgba(texture)
}

fn base_rgba(texture: &Texture) -> Result<Vec<u8>> {
    texture.rgba8(0)
}
