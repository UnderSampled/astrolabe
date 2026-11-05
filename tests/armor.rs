//! Fix-owned armor catalog import, Rete persistence, and native export.
use anyhow::{Context, Result, ensure};
use astrolabe::{
    godot,
    model::{
        AssetPath, AssetRef, Selection,
        ai::{
            AiModel, Behavior, BehaviorSet, DesignerValue, DesignerVariable, Expression, Script,
            SourceObjectIdentity, SourceRole, Statement,
        },
        armor::{ArmorArtwork, ArmorCatalog, ArmorDefinition, ArmorPiece},
        io::{Content, ContentRoot, Load, Save},
        scene::{NodeKind, Scene, SceneNode, SceneNodeProperties},
        texture::{
            OpenSpaceColorKey, Texture, TextureColors, TextureLevel, TexturePixels, TextureUsage,
        },
    },
    openspace::{self, ai::AiScriptCodec, codec::FormatCodec},
    rete,
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
fn compiled_armor_setup_uses_saved_level_selected_definition() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let project = temp.path().join("godot");
    godot::support::initialize(&project)?;
    fs::write(
        project.join("check_compiled_armor_setup.gd"),
        include_bytes!("check_compiled_armor_setup.gd"),
    )?;
    // World154's first-visit schedule uses Proc430(0) before another actor
    // selects armor. Exercise that same guard and native shared-state binding.
    let script = Script {
        statements: vec![
            Statement::SetArmorResistance(Expression::Integer(0)),
            Statement::RepairArmor,
            Statement::Assign {
                variable: 0,
                value: Expression::ArmorResistance,
            },
            Statement::Assign {
                variable: 1,
                value: Expression::ArmorCount,
            },
        ],
    };
    let bytes = AiScriptCodec::default().write(&script)?;
    ensure!(AiScriptCodec::default().read(&bytes)? == script);
    let model = AiModel {
        path: AssetPath::new("level:/ai/models/154")?,
        name: "Armor setup slice".into(),
        normal: Some(BehaviorSet {
            behaviors: vec![Behavior {
                id: 0,
                rules: vec![],
                schedule: Some(script),
                unknown: [0; 3],
            }],
            default_behavior: Some(0),
            action_capacity: 0,
            unknown: [0; 3],
        }),
        reflex: None,
        variables: (0..2)
            .map(|_| DesignerVariable {
                default: DesignerValue::Integer(-1),
                save: 0,
                initialize: 0,
            })
            .collect(),
        designer_unknown: [0; 3],
        unknown: [0; 4],
    };
    let code = godot::ai::compile(&rete::decode::<AiModel>(&rete::encode(&model)?)?, &[])?;
    ensure!(
        code.contains("has_method(\"selected_definition\")") && !code.contains("call(\"active\")")
    );
    fs::write(project.join("compiled_armor_setup.gd"), code)?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let parser_log = project.join("armor-setup-parser.log");
    godot::run_godot(
        &engine,
        &project,
        &[
            "--headless",
            "--log-file",
            parser_log.to_str().context("log path")?,
            "--check-only",
            "--script",
            "res://check_compiled_armor_setup.gd",
        ],
        Duration::from_secs(10),
    )?;

    let selection = Selection::hype("manoir", "en")?;
    let mut store = rete::Store::new(temp.path().join("rete"), selection.clone())?;
    let mut armor = catalog()?;
    armor.definitions[0].resistance_rank = 3;
    armor.definitions[0].pieces[0].broken = 1;
    armor.definitions[0].pieces[0].strikes = 7;
    armor.definitions[0].broken_count = 1;
    let armor_ref = AssetRef::from_path(armor.path.clone());
    store.save(&armor_ref, &armor)?;
    store.save(&AssetRef::new("texture:/armor/catalog-test")?, &texture()?)?;
    let scene = Scene {
        path: AssetPath::new("level:/scene")?,
        name: "ArmorSetup".into(),
        main_actor: None,
        armor: Some(armor_ref),
        inventory: None,
        map_names: None,
        hud: None,
        loading: None,
        global_actors: vec![],
        always_active: Vec::new(),
        nodes: vec![SceneNode {
            id: "world".into(),
            name: "world".into(),
            parent: None,
            kind: NodeKind::World,
            properties: SceneNodeProperties::default(),
            transform: None,
            instance: None,
            perso: None,
            collision: None,
            draw_mask: 0,
            sector: None,
            light: None,
        }],
    };
    store.save(&AssetRef::from_path(scene.path.clone()), &scene)?;
    drop(store);
    let mut store = rete::Store::new(temp.path().join("rete"), selection.clone())?;
    godot::export(
        &mut store,
        &[ContentRoot {
            kind: Scene::KIND.into(),
            reference: scene.path,
        }],
        selection,
        &project,
        &engine,
    )?;
    let log = project.join("armor-setup-native.log");
    let output = godot::run_godot(
        &engine,
        &project,
        &[
            "--headless",
            "--log-file",
            log.to_str().context("log path")?,
            "--script",
            "res://check_compiled_armor_setup.gd",
        ],
        Duration::from_secs(15),
    )?;
    ensure!(output.contains("COMPILED_ARMOR_SETUP_OK"), "{output}");
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
