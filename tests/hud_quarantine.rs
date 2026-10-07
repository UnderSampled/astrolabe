//! Focused regressions for the source-unverified Fix HUD boundary.
use anyhow::{Result, ensure};
use astrolabe::{
    app, godot,
    model::{
        AssetPath, AssetRef, Selection,
        content::Collection,
        hud::HudPresentation,
        io::{Content, ContentRoot, Load, Save},
    },
    openspace::content,
    rete,
};
use serde::Serialize;
use std::{fs, path::Path};

fn legacy_hud() -> Result<HudPresentation> {
    Ok(HudPresentation {
        path: AssetPath::new("fix:/hud/en")?,
        canvas_size: [640, 480],
        panels: Vec::new(),
        source_textures: std::array::from_fn(|_| None),
        rectangle_tables: [Vec::new(), Vec::new()],
        source_words: [0; 2],
        text_offsets: [[0.0; 2]; 2],
        elements: Vec::new(),
    })
}

#[test]
fn explicit_hud_import_is_omitted_with_a_persisted_warning() -> Result<()> {
    let mut imported = content::import_hud(
        Path::new("unused-source-path"),
        Selection::hype("manoir", "en")?,
    )?;
    ensure!(!imported.assets.contains(&AssetPath::new("fix:/hud/en")?));
    ensure!(imported.inventory.is_empty());
    ensure!(imported.roots.is_empty());
    ensure!(imported.diagnostics.len() == 1);
    let warning = &imported.diagnostics[0];
    ensure!(warning.code == "openspace.hud.layout_unverified");
    ensure!(warning.severity == astrolabe::model::io::Severity::Warning);
    ensure!(warning.asset.is_none());

    let temp = tempfile::tempdir()?;
    app::save_rete(&mut imported, temp.path())?;
    let (_store, package) = rete::Store::open(temp.path())?;
    ensure!(package.roots.is_empty());
    ensure!(package.diagnostics == imported.diagnostics);
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
    let mut source = Collection::default();
    let output = temp.path().join("godot");
    let export_error = godot::export(
        &mut source,
        &[root],
        Selection::hype("manoir", "en")?,
        &output,
        "engine-must-not-start",
    )
    .expect_err("legacy HUD root was silently exported");
    ensure!(
        export_error
            .to_string()
            .contains("refuses legacy HUD metadata")
    );
    ensure!(!output.exists(), "rejected export created a project");
    Ok(())
}
