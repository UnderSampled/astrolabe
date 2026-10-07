//! Focused regressions for source-verified Fix HUD import, persistence and export.
use anyhow::{Result, ensure};
use astrolabe::{
    app, godot,
    model::{
        AssetPath, AssetRef, Selection,
        hud::{HudPresentation, HudTextureRole},
        io::{Content, ContentRoot, Load, Save},
        texture::{Texture, TextureUsage},
    },
    openspace::content,
    rete,
};
use serde::Serialize;
use std::{fs, path::PathBuf};

fn legacy_hud() -> Result<HudPresentation> {
    Ok(HudPresentation {
        path: AssetPath::new("fix:/hud/en")?,
        canvas_size: [640, 480],
        source_layout_version: 0,
        panels: Vec::new(),
        source_textures: std::array::from_fn(|_| None),
        artwork: Vec::new(),
        rectangle_tables: [Vec::new(), Vec::new()],
        source_words: [0; 2],
        text_offsets: [[0.0; 2]; 2],
        elements: Vec::new(),
    })
}

#[test]
fn verified_hud_import_rete_roundtrip_and_godot_export() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("disc"));
    let selection = Selection::hype("manoir", "en")?;
    let mut imported = content::import_hud(&game, selection.clone())?;
    let hud_path = AssetPath::new("fix:/hud/en")?;
    ensure!(imported.roots.len() == 1);
    ensure!(imported.roots[0].reference == hud_path);
    ensure!(
        imported.inventory.len() == 7,
        "six texture dependencies and one HUD asset expected"
    );
    ensure!(imported.diagnostics.is_empty());

    let hud = imported.load(&AssetRef::<HudPresentation>::from_path(hud_path.clone()))?;
    ensure!(hud.source_layout_version == 1);
    ensure!(hud.panels.iter().all(|panel| !panel.enabled));
    ensure!(hud.elements.iter().any(|element| {
        element.role == "enemy_meter_background_fill_pairs"
            && element.entries == [11, 12, 13, 14, 15, 16]
    }));
    let names = [
        "fixtex\\stencil\\jauges\\ennemis\\enemyfg_nz.tga",
        "fixtex\\stencil\\jauges\\ennemis\\enemybg_nz.tga",
        "fixtex\\stencil\\jauges\\vie\\poigne_nz.tga",
        "fixtex\\stencil\\jauges\\vie\\vie0.tga",
        "fixtex\\stencil\\jauges\\vie\\vie1.tga",
        "fixtex\\stencil\\jauges\\gold\\gold_icone_nz.tga",
    ];
    let indices = [22, 23, 13, 11, 12, 30];
    let sizes = [[64, 8], [128, 16], [32, 32], [64, 16], [64, 16], [128, 64]];
    let roles = [
        HudTextureRole::EnemyMeterFill,
        HudTextureRole::EnemyMeterBackground,
        HudTextureRole::PlayerLifeHandle,
        HudTextureRole::PlayerLifeRemainder,
        HudTextureRole::PlayerLifeFill,
        HudTextureRole::CurrencyIcon,
    ];
    for slot in 0..6 {
        let artwork = &hud.artwork[slot];
        ensure!(artwork.source_name == names[slot]);
        ensure!(artwork.fix_ptx_index == indices[slot]);
        ensure!(artwork.logical_size == sizes[slot]);
        ensure!(artwork.role == roles[slot]);
        let texture = imported.load(&AssetRef::<Texture>::from_path(artwork.texture.clone()))?;
        ensure!([texture.width(), texture.height()] == sizes[slot]);
        ensure!(texture.usage == TextureUsage::Material);
    }

    let temp = tempfile::tempdir()?;
    let rete_dir = temp.path().join("rete");
    app::save_rete(&mut imported, &rete_dir)?;
    let (mut store, package) = rete::Store::open(&rete_dir)?;
    ensure!(package.roots == imported.roots);
    ensure!(package.diagnostics.is_empty());
    let reopened_hud = store.load(&AssetRef::<HudPresentation>::from_path(hud_path.clone()))?;
    ensure!(*reopened_hud == *hud);

    let output = temp.path().join("godot");
    let manifest = godot::export(
        &mut store,
        &package.roots,
        package.selection,
        &output,
        &std::env::var("GODOT_BIN").unwrap_or_else(|_| "godot".into()),
    )?;
    let scene_rel = "generated/hype/shared/hud/en.tscn";
    let script_rel = "generated/hype/shared/hud/en.gd";
    ensure!(manifest.files.contains_key(scene_rel));
    ensure!(manifest.files.contains_key(script_rel));
    let scene = fs::read_to_string(output.join(scene_rel))?;
    let script = fs::read_to_string(output.join(script_rel))?;
    for node in [
        "PlayerLifeHandle",
        "PlayerLifeFill",
        "PlayerLifeRemainder",
        "EnemyMeter0Background",
        "EnemyMeter0Fill",
        "EnemyMeter1Background",
        "EnemyMeter1Fill",
        "EnemyMeter2Background",
        "EnemyMeter2Fill",
        "CurrencyIcon",
    ] {
        ensure!(
            scene.contains(&format!("name=\"{node}\"")),
            "native HUD node {node} missing"
        );
    }
    ensure!(script.contains("func set_life_fraction"));
    ensure!(script.contains("func set_enemy_meter"));
    for artwork in &hud.artwork {
        let image_rel = format!("generated/hype/textures/{}.hud.png", artwork.texture.file());
        ensure!(
            manifest.files.contains_key(&image_rel),
            "native HUD image missing: {image_rel}"
        );
    }
    Ok(())
}

#[test]
fn legacy_hud_is_inspection_only_and_cannot_be_saved_or_exported() -> Result<()> {
    let path = AssetPath::new("fix:/hud/en")?;
    let value = legacy_hud()?;
    let temp = tempfile::tempdir()?;
    let mut store = rete::Store::new(temp.path().join("rete"), Selection::hype("manoir", "en")?)?;
    let save_error = store
        .save(&AssetRef::from_path(path.clone()), &value)
        .expect_err("legacy HUD metadata was saved");
    ensure!(
        save_error
            .to_string()
            .contains("legacy HUD assets are inspection-only")
    );

    #[derive(Serialize)]
    struct LegacyEnvelope<T> {
        format: String,
        schema: u32,
        kind: String,
        value: T,
    }
    let legacy_bytes = rmp_serde::to_vec_named(&LegacyEnvelope {
        format: "astrolabe-rete".into(),
        schema: rete::SCHEMA,
        kind: HudPresentation::KIND.into(),
        value,
    })?;
    let legacy_path = store.path(&path);
    fs::create_dir_all(legacy_path.parent().expect("asset path has a parent"))?;
    fs::write(&legacy_path, legacy_bytes)?;
    let legacy_root = ContentRoot {
        kind: HudPresentation::KIND.into(),
        reference: path.clone(),
    };
    let inspected = store.inspect(&legacy_root)?;
    ensure!(inspected.contains("fix:/hud/en"));
    let load_error = store
        .load(&AssetRef::<HudPresentation>::from_path(path.clone()))
        .expect_err("legacy HUD metadata was available as normal content");
    ensure!(
        format!("{load_error:#}").contains("legacy HUD assets are inspection-only"),
        "unexpected legacy load error: {load_error:#}"
    );

    let root = ContentRoot {
        kind: HudPresentation::KIND.into(),
        reference: path,
    };
    let output = temp.path().join("godot");
    let export_error = godot::export(
        &mut store,
        &[root],
        Selection::hype("manoir", "en")?,
        &output,
        "engine-must-not-start",
    )
    .expect_err("legacy HUD root was silently exported");
    ensure!(
        format!("{export_error:#}").contains("legacy HUD assets are inspection-only"),
        "unexpected legacy HUD export error: {export_error:#}"
    );
    ensure!(!output.exists(), "rejected export created a project");
    Ok(())
}
