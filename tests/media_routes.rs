//! Real per-asset persistence and reconstruction through shared content contracts.
use anyhow::{Context, Result, ensure};
use astrolabe::{
    model::{
        AssetPath, AssetRef, Selection,
        content::TextureSet,
        io::{Content, ContentRoot, Load, Save},
        texture::{Texture, TextureColors, TexturePixels, TextureUsage},
    },
    openspace::{
        content::{import_textures, rebuild_texture_set},
        texture::decode_cnt,
    },
    rete::{Package, Store},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

fn transfer<T: Content + Serialize>(
    from: &mut impl Load<T>,
    to: &mut impl Save<T>,
    path: &AssetPath,
) -> Result<()> {
    let reference = AssetRef::<T>::new(path.to_string())?;
    let value = from.load(&reference)?;
    to.save(&reference, value.as_ref())
}

/// Same whole-content consumer for an in-memory source import and Rete resolver.
fn rebuild<R: Load<Texture> + Load<TextureSet>>(
    source: &mut R,
    roots: &[ContentRoot],
) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut result = BTreeMap::new();
    for root in roots {
        ensure!(
            root.kind == TextureSet::KIND,
            "unexpected root in texture selection"
        );
        let set = source.load(&AssetRef::<TextureSet>::new(root.reference.to_string())?)?;
        ensure!(
            result
                .insert(set.name.clone(), rebuild_texture_set(&set, source)?)
                .is_none(),
            "duplicate archive root"
        );
    }
    Ok(result)
}

/// This exporter has only a Rete root. Neither a source path nor source bytes
/// can be supplied through its API, and no import session survives the call.
fn export_reopened_rete(root: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    let (mut store, package) = Store::open(root)?;
    rebuild(&mut store, &package.roots)
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

// External reference normalization removes only source XOR obfuscation. This
// expected byte buffer is never passed to the production reconstruction path.
fn without_obfuscation(bytes: &[u8]) -> Vec<u8> {
    fn word(bytes: &[u8], p: &mut usize) -> u32 {
        let v = u32::from_le_bytes(bytes[*p..*p + 4].try_into().unwrap());
        *p += 4;
        v
    }
    fn string(bytes: &mut [u8], p: &mut usize, k: u8) {
        let n = word(bytes, p) as usize;
        for b in &mut bytes[*p..*p + n] {
            *b ^= k;
        }
        *p += n;
    }
    let mut expected = bytes.to_vec();
    let mut p = 0;
    let dirs = word(bytes, &mut p);
    let files = word(bytes, &mut p);
    let key = if bytes[8] != 0 { bytes[10] } else { 0 };
    expected[8] = 0;
    expected[10] = 0;
    p = 11;
    for _ in 0..dirs {
        string(&mut expected, &mut p, key);
    }
    p += 1;
    let mut visited = std::collections::BTreeSet::new();
    for _ in 0..files {
        p += 4;
        string(&mut expected, &mut p, key);
        let key_start = p;
        let xor = word(bytes, &mut p).to_le_bytes();
        expected[key_start..key_start + 4].fill(0);
        p += 4;
        let start = word(bytes, &mut p) as usize;
        let size = word(bytes, &mut p) as usize;
        if visited.insert((start, size)) {
            for i in 0..size / 4 * 4 {
                expected[start + i] ^= xor[i % 4];
            }
        }
    }
    expected
}

#[test]
#[ignore = "requires local Hype texture archives; run through the complete bounded --local suite"]
fn complete_texture_packages_survive_per_asset_rete_and_shared_resize() -> Result<()> {
    let started = std::time::Instant::now();
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("disc"));
    let selection = Selection::hype("astrolabe", "EN")?;
    let temporary = tempfile::tempdir()?;
    let rete_root = temporary.path().join("rete");
    let direct_hashes;
    {
        let mut imported = import_textures(&game, selection.clone())?;
        assert_eq!(imported.roots.len(), 2);
        // 3,578 names minus214 aliases, plus two archive roots.
        assert_eq!(imported.inventory.len(), 3366);
        let roots = imported.roots.clone();
        direct_hashes = rebuild(&mut imported, &roots)?
            .into_iter()
            .map(|(name, data)| (name, hash(&data)))
            .collect::<BTreeMap<_, _>>();
        let mut store = Store::new(&rete_root, selection.clone())?;
        // Saving root files before their dependencies exercises real forward links.
        for entry in imported.inventory.clone().into_iter().rev() {
            match entry.kind.as_str() {
                Texture::KIND => transfer::<Texture>(&mut imported, &mut store, &entry.reference)?,
                TextureSet::KIND => {
                    transfer::<TextureSet>(&mut imported, &mut store, &entry.reference)?
                }
                kind => anyhow::bail!("unexpected imported kind {kind}"),
            }
            assert!(store.path(&entry.reference).is_file());
        }
        store.save_package(&Package {
            diagnostics: vec![],
            selection: selection.clone(),
            roots,
        })?;
    } // Drop both importer and store, including all caches/source content.
    let rebuilt = export_reopened_rete(&rete_root)?;
    for (name, bytes) in &rebuilt {
        assert_eq!(
            hash(bytes),
            direct_hashes[name],
            "direct and persisted routes differ: {name}"
        );
        let original = fs::read(game.join("Gamedata").join(name))?;
        assert!(
            *bytes == without_obfuscation(&original),
            "rebuilt CNT/GF bytes differ beyond XOR: {name}"
        );
        let usage = if name == "Textures.cnt" {
            TextureUsage::Material
        } else {
            TextureUsage::Screen
        };
        let expected = decode_cnt(&original, usage)?;
        let recovered = decode_cnt(bytes, usage)?;
        assert!(
            expected == recovered,
            "archive fields, exact images, membership/order or sharing differ: {name}"
        );
        println!(
            "{name}: {} members survive per-asset Rete, complete reconstructed bytes match without XOR",
            recovered.textures.len()
        );
    }
    let changed_ref;
    let affected_names;
    let edited;
    {
        let (mut store, package) = Store::open(&rete_root)?;
        let root = package
            .roots
            .iter()
            .find(|root| root.reference.file() == "archives/Textures.cnt")
            .context("texture archive root missing")?;
        let set = store.load(&AssetRef::<TextureSet>::new(root.reference.to_string())?)?;
        let mut owners = BTreeMap::<AssetPath, Vec<String>>::new();
        for entry in &set.entries {
            owners
                .entry(entry.texture.path.clone())
                .or_default()
                .push(entry.name.clone());
        }
        let (reference, names) = owners
            .into_iter()
            .find(|(_, names)| names.len() > 1)
            .context("expected real shared image")?;
        changed_ref = AssetRef::<Texture>::new(reference.to_string())?;
        affected_names = names;
        let previous = store.load(&changed_ref)?;
        let mut value = (*previous).clone();
        // Resize one actually shared asset to a deliberately edited 1x1 image.
        // All original palette entries remain useful values, including unused ones.
        value.levels.truncate(1);
        value.levels[0].width = 1;
        value.levels[0].height = 1;
        match (&mut value.colors, &mut value.levels[0].pixels) {
            (TextureColors::PaletteRgb(palette), TexturePixels::Indices(pixels)) => {
                pixels.truncate(1);
                palette[pixels[0] as usize][0] ^= 1;
            }
            (TextureColors::PaletteRgba(palette), TexturePixels::Indices(pixels)) => {
                pixels.truncate(1);
                palette[pixels[0] as usize][0] ^= 1;
            }
            (TextureColors::Components { .. }, TexturePixels::Components(pixels)) => {
                pixels.truncate(1);
                pixels[0][0] ^= 1;
            }
            _ => unreachable!(),
        }
        value.validate()?;
        assert_ne!(*previous, value);
        store.save(&changed_ref, &value)?;
        assert_eq!(
            *store.load(&changed_ref)?,
            value,
            "edit did not invalidate the cached asset"
        );
        edited = value;
    }
    let changed = export_reopened_rete(&rete_root)?;
    assert_eq!(
        changed["Vignette.cnt"], rebuilt["Vignette.cnt"],
        "unrelated package changed"
    );
    assert!(
        changed["Textures.cnt"].len() < rebuilt["Textures.cnt"].len(),
        "resize did not change archive placement"
    );
    let original = decode_cnt(&rebuilt["Textures.cnt"], TextureUsage::Material)?;
    let after = decode_cnt(&changed["Textures.cnt"], TextureUsage::Material)?;
    assert_eq!(original.directories, after.directories);
    for (before, current) in original.textures.iter().zip(&after.textures) {
        assert_eq!(before.path, current.path);
        assert_eq!(before.shared_with, current.shared_with);
        if affected_names.contains(&current.path) {
            assert_eq!(current.texture, edited);
        } else {
            assert!(
                before.texture == current.texture,
                "unrelated texture changed: {}",
                current.path
            );
        }
    }
    println!(
        "Edited {} through one Rete file: {} shared names changed, original sharing retained; route elapsed {:?}",
        changed_ref.path,
        affected_names.len(),
        started.elapsed()
    );
    Ok(())
}
