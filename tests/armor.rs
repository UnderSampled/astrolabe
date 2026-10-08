//! Fix-owned armor catalog import, Rete persistence, and native export.
use anyhow::Result;
use astrolabe::{
    godot,
    model::{
        AssetPath, AssetRef, Selection,
        ai::{SourceObjectIdentity, SourceRole},
        armor::{ArmorArtwork, ArmorCatalog, ArmorDefinition, ArmorPiece},
        io::{Load, Save},
        texture::{
            OpenSpaceColorKey, Texture, TextureColors, TextureLevel, TexturePixels, TextureUsage,
        },
    },
    openspace, rete,
};
use std::{fs, time::Duration};

fn texture() -> Result<Texture> {
    Ok(Texture {
        usage: TextureUsage::Material,
        colors: TextureColors::Components { bits: [8, 8, 8, 8] },
        levels: vec![TextureLevel {
            width: 2,
            height: 2,
            pixels: TexturePixels::Components(vec![
                [240, 220, 160, 255],
                [180, 130, 80, 255],
                [90, 60, 40, 255],
                [30, 20, 10, 255],
            ]),
        }],
        openspace_color_key: OpenSpaceColorKey {
            rgb: [0, 0, 0],
            palette_index: -1,
        },
    })
}

fn catalog() -> Result<ArmorCatalog> {
    let texture = AssetRef::new("texture:/armor/catalog-test")?;
    let source = |allocation| SourceObjectIdentity {
        role: SourceRole::Fix,
        block: [5, 0],
        allocation,
    };
    Ok(ArmorCatalog {
        path: AssetPath::new("fix:/gameplay/armor")?,
        selected: Some(0),
        definitions: vec![ArmorDefinition {
            source: source(0),
            artwork: [
                Some(ArmorArtwork {
                    texture: texture.clone(),
                    size: [2, 2],
                    color_key: false,
                }),
                Some(ArmorArtwork {
                    texture,
                    size: [2, 2],
                    color_key: false,
                }),
                None,
            ],
            pieces: vec![ArmorPiece {
                source: source(1),
                rectangle: [0.0, 0.0, 1.0, 1.0],
                broken: 0,
                strikes: 0,
                unknown: [0; 2],
            }],
            damage_factors: vec![vec![1.0, 0.5]; 5],
            break_thresholds: [1; 5],
            minimum_damage: 1,
            broken_count: 0,
            resistance_rank: 0,
            current_damage_piece: None,
            unknown: [0; 3],
        }],
    })
}

#[test]
#[ignore = "requires local English Hype Fix data; run the bounded armor corpus check"]
fn fix_armor_catalog_import_uses_fix_relocations_and_source_allocations() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("disc"));
    let selection = Selection::hype("manoir", "en")?;
    let catalog = openspace::level::import_armor(&game, &selection)?;

    assert_eq!(catalog.path, AssetPath::new("fix:/gameplay/armor")?);
    assert_eq!(catalog.selected, Some(0));
    assert_eq!(
        catalog
            .definitions
            .iter()
            .map(|definition| definition.pieces.len())
            .collect::<Vec<_>>(),
        [7, 5, 5]
    );
    for definition in &catalog.definitions {
        assert_eq!(definition.source.role, SourceRole::Fix);
        assert_eq!(definition.source.block, [5, 0]);
        assert_eq!(definition.damage_factors.len(), 5);
        assert!(definition.artwork[0].is_some() && definition.artwork[1].is_some());
        assert!(definition.artwork.iter().flatten().all(|item| {
            item.texture.path.role() == astrolabe::model::PackageRole::Texture
                && item.size.iter().all(|dimension| *dimension > 0)
        }));
        assert!(
            definition.pieces.iter().all(|piece| {
                piece.source.role == SourceRole::Fix && piece.source.block == [5, 0]
            })
        );
    }
    Ok(())
}

#[test]
fn armor_catalog_survives_rete_storage_reload() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let selection = Selection::hype("armor-test", "en")?;
    let catalog = catalog()?;
    let catalog_ref = AssetRef::from_path(catalog.path.clone());
    let texture_ref = AssetRef::new("texture:/armor/catalog-test")?;
    let rete_dir = temp.path().join("rete");
    let mut store = rete::Store::new(&rete_dir, selection.clone())?;
    store.save(&texture_ref, &texture()?)?;
    store.save(&catalog_ref, &catalog)?;
    drop(store);

    let mut store = rete::Store::new(&rete_dir, selection.clone())?;
    let loaded = store.load(&catalog_ref)?;
    assert_eq!(*loaded, catalog);
    Ok(())
}

#[test]
fn native_snapshot_helper_and_scene_local_catalog_clone_work() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let project = temp.path().join("godot");
    godot::support::initialize(&project)?;
    fs::write(
        project.join("check_snapshot_values.gd"),
        include_bytes!("check_snapshot_values.gd"),
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let evidence = godot::run_godot(
        &engine,
        &project,
        &[
            "--headless",
            "--script",
            "res://check_snapshot_values.gd",
            "--log-file",
            "snapshot-values.log",
        ],
        Duration::from_secs(20),
    )?;
    assert!(evidence.contains("SNAPSHOT_VALUES_OK"), "{evidence}");
    Ok(())
}
