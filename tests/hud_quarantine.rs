//! Focused regressions for source-verified Fix HUD import, persistence and export.
use anyhow::{Context, Result, ensure};
use astrolabe::{
    app, godot,
    model::{
        AssetPath, AssetRef, Selection,
        hud::{HudPresentation, HudTextureRole},
        io::{Content, ContentRoot, Load, Save},
        scene::{NodeKind, Scene, SceneNode, SceneNodeProperties, TransformSpace},
        texture::{Texture, TextureUsage},
    },
    openspace::content,
    rete,
};
use serde::Serialize;
use std::{fs, path::PathBuf, process::Command};

fn legacy_hud() -> Result<HudPresentation> {
    Ok(HudPresentation {
        path: AssetPath::new("fix:/hud/en")?,
        canvas_size: [640, 480],
        source_layout_version: 0,
        panels: Vec::new(),
        source_textures: std::array::from_fn(|_| None),
        artwork: Vec::new(),
        currency_source_object: None,
        currency_item_id: None,
        currency_font: None,
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
    let mut imported_scene = content::import_scene(&game, selection.clone())?;
    let scene_root = imported_scene
        .roots
        .iter()
        .find(|root| root.kind == Scene::KIND)
        .context("scene import omitted its root")?;
    let imported_scene_root =
        imported_scene.load(&AssetRef::<Scene>::from_path(scene_root.reference.clone()))?;
    let mut imported = content::import_hud(&game, selection.clone())?;
    let hud_path = AssetPath::new("fix:/hud/en")?;
    ensure!(
        imported_scene_root
            .hud
            .as_ref()
            .is_some_and(|hud| hud.path == hud_path)
    );
    ensure!(imported_scene.assets.contains(&hud_path));
    ensure!(imported.roots.len() == 1);
    ensure!(imported.roots[0].reference == hud_path);
    ensure!(
        imported.inventory.len() == 8,
        "six HUD images, one font atlas, and one HUD asset expected"
    );
    ensure!(imported.diagnostics.is_empty());

    let hud = imported.load(&AssetRef::<HudPresentation>::from_path(hud_path.clone()))?;
    ensure!(hud.source_layout_version == 1);
    ensure!(hud.panels.iter().all(|panel| !panel.enabled));
    ensure!(hud.currency_item_id == Some(0));
    ensure!(hud.currency_source_object.as_ref().is_some_and(|object| {
        object.role == astrolabe::model::ai::SourceRole::Fix && object.block == [6, 1]
    }));
    let currency_font = hud
        .currency_font
        .as_ref()
        .expect("source HUD font should be imported");
    ensure!(currency_font.name == "InventoryFont");
    ensure!(currency_font.source_object.block == [19, 1]);
    ensure!(currency_font.glyphs.len() == 127);
    ensure!(currency_font.atlas.texture.as_str() == "texture:/FixTex/MOTORTXT/lettrageb.gf");
    ensure!(currency_font.atlas.size == [256, 256]);
    let font_atlas = imported.load(&AssetRef::<Texture>::from_path(
        currency_font.atlas.texture.clone(),
    ))?;
    ensure!([font_atlas.width(), font_atlas.height()] == [256, 256]);
    ensure!(font_atlas.usage == TextureUsage::Material);
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

    let level_path = AssetPath::new("level:/hud_attachment_check")?;
    let source_scene = Scene {
        path: level_path.clone(),
        name: "HudAttachmentCheck".into(),
        main_actor: None,
        armor: None,
        hud: Some(AssetRef::from_path(hud_path.clone())),
        global_actors: vec![],
        nodes: vec![SceneNode {
            properties: SceneNodeProperties {
                source_kind: Some(0),
                source_visual_lod_count: None,
                transform_space: TransformSpace::Local,
                initial_global_transform: None,
                bounding_geometry: None,
                opacity: 255.0,
            },
            id: "world".into(),
            name: "World".into(),
            parent: None,
            kind: NodeKind::World,
            transform: None,
            instance: None,
            perso: None,
            collision: None,
            draw_mask: 0,
            sector: None,
            light: None,
        }],
    };
    store.save(
        &AssetRef::<Scene>::from_path(level_path.clone()),
        &source_scene,
    )?;

    let output = temp.path().join("godot");
    let godot_bin = std::env::var("GODOT_BIN").unwrap_or_else(|_| "godot".into());
    let roots = [ContentRoot {
        kind: Scene::KIND.into(),
        reference: level_path,
    }];
    let manifest = godot::export(
        &mut store,
        &roots,
        package.selection.clone(),
        &output,
        &godot_bin,
    )?;
    let scene_rel = "generated/hype/shared/hud/en.tscn";
    let script_rel = "generated/hype/shared/hud/en.gd";
    let screen_rel = "generated/hype/shared/ui/screen_en.tscn";
    let level_rel = "generated/hype/levels/manoir/level.tscn";
    ensure!(manifest.files.contains_key(scene_rel));
    ensure!(manifest.files.contains_key(script_rel));
    ensure!(manifest.files.contains_key(screen_rel));
    ensure!(manifest.files.contains_key(level_rel));
    let scene = fs::read_to_string(output.join(scene_rel))?;
    let script = fs::read_to_string(output.join(script_rel))?;
    let screen_scene = fs::read_to_string(output.join(screen_rel))?;
    let level_scene = fs::read_to_string(output.join(level_rel))?;
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
        "CurrencyCount",
    ] {
        ensure!(
            scene.contains(&format!("name=\"{node}\"")),
            "native HUD node {node} missing"
        );
    }
    ensure!(script.contains("func set_life_fraction"));
    ensure!(script.contains("func set_enemy_meter"));
    ensure!(script.contains("const CURRENCY_ITEM_ID: int = 0"));
    ensure!(script.contains("func bind_actor(actor: Node)"));
    ensure!(script.contains("func set_currency_count(value: int)"));
    ensure!(
        !script.contains(".region_rect"),
        "TextureRect crop must not use the unsupported region_rect property"
    );
    ensure!(scene.contains("offset_left = 330.0\noffset_top = 454.0"));
    let font_rel = "generated/hype/shared/hud/en/counter.font.tres";
    ensure!(
        manifest.files.contains_key(font_rel),
        "native HUD FontFile missing"
    );
    let font_image_rel = "generated/hype/textures/FixTex/MOTORTXT/lettrageb.gf.hud-font.png";
    ensure!(
        manifest.files.contains_key(font_image_rel),
        "native HUD font atlas image missing"
    );
    ensure!(level_scene.contains(screen_rel));
    ensure!(screen_scene.contains(scene_rel));
    for artwork in &hud.artwork {
        let image_rel = format!("generated/hype/textures/{}.hud.png", artwork.texture.file());
        ensure!(
            manifest.files.contains_key(&image_rel),
            "native HUD image missing: {image_rel}"
        );
    }

    // Exercise the emitted scene, native FontFile and actor-owned source value.
    let actor_script = output.join("hud_counter_actor.gd");
    fs::write(
        &actor_script,
        "extends Node\nsignal hud_visibility_requested(indicator: String, enabled: bool)\nvar source_inventory: Resource\nvar source_hit_points: Resource\nvar source_hud_display: Dictionary = {}\n",
    )?;
    let check_script = output.join("check_hud_counter.gd");
    fs::write(
        &check_script,
        r#"extends SceneTree
func _initialize() -> void:
    call_deferred("_run")

func _check(value: bool, message: String) -> bool:
    if not value:
        push_error(message)
        quit(1)
        return false
    return true

func _run() -> void:
    var packed := load("res://generated/hype/shared/hud/en.tscn") as PackedScene
    if not _check(packed != null, "native HUD scene did not load"):
        return
    var hud := packed.instantiate()
    root.add_child(hud)
    await process_frame
    var actor_script := load("res://hud_counter_actor.gd") as Script
    var inventory_script := load("res://addons/astrolabe/character/inventory.gd") as Script
    var hit_points_script := load("res://addons/astrolabe/character/hit_points.gd") as Script
    if not _check(actor_script != null and inventory_script != null and hit_points_script != null, "native actor state scripts did not load"):
        return
    var actor := actor_script.new() as Node
    var inventory := inventory_script.new() as Resource
    var hit_points := hit_points_script.new() as Resource
    hit_points.set("current", 50)
    hit_points.set("maximum", 100)
    actor.set("source_inventory", inventory)
    actor.set("source_hit_points", hit_points)
    if not _check(int(inventory.call("quantity", 0)) == 0, "source currency default was not zero"):
        return
    if not _check(int(inventory.call("quantity", 21)) == -1, "ordinary missing item sentinel changed"):
        return
    hud.call("bind_actor", actor)
    await process_frame
    var label := hud.get_node("Source/CurrencyCount") as Label
    hud.call("_process", 0.0)
    var life_fill := hud.get_node("Source/PlayerLifeFill") as TextureRect
    var life_remainder := hud.get_node("Source/PlayerLifeRemainder") as TextureRect
    var life_atlas := life_fill.texture as AtlasTexture
    var remainder_atlas := life_remainder.texture as AtlasTexture
    if not _check(life_atlas != null and remainder_atlas != null, "life textures were not wrapped as native atlas regions"):
        return
    var life_size := life_atlas.atlas.get_size()
    var remainder_size := remainder_atlas.atlas.get_size()
    if not _check(absf(life_atlas.region.size.x - life_size.x * 0.5) < 0.001, "life fill atlas region did not crop to the requested fraction"):
        return
    if not _check(absf(remainder_atlas.region.position.x - life_size.x * 0.5) < 0.001 and absf(remainder_atlas.region.size.x - remainder_size.x * 0.5) < 0.001, "life remainder atlas region did not start at the fill edge"):
        return
    if not _check(bool(hud.call("set_enemy_meter", 0, 0.25)), "enemy fill update was rejected"):
        return
    var enemy_fill := hud.get_node("Source/EnemyMeter0Fill") as TextureRect
    var enemy_atlas := enemy_fill.texture as AtlasTexture
    if not _check(enemy_atlas != null, "enemy fill texture was not wrapped as a native atlas region"):
        return
    var enemy_size := enemy_atlas.atlas.get_size()
    if not _check(absf(enemy_atlas.region.position.x - enemy_size.x * 0.75) < 0.001 and absf(enemy_atlas.region.size.x - enemy_size.x * 0.25) < 0.001, "enemy fill atlas region did not crop to the requested fraction"):
        return
    actor.emit_signal("hud_visibility_requested", "life", true)
    var life_handle := hud.get_node("Source/PlayerLifeHandle") as TextureRect
    if not _check(life_handle.visible, "actor life visibility did not reach the source HUD"):
        return
    if not _check(life_fill.visible and life_remainder.visible, "actor hit points did not activate the life fill/remainder"):
        return
    if not _check(label.visible and label.text == "0", "zero source currency was not visible"):
        return
    inventory.set("quantities", {0: 27})
    hud.call("_process", 0.0)
    if not _check(label.visible and label.text == "27", "actor currency update did not reach HUD text"):
        return
    inventory.set("quantities", {0: -1})
    hud.call("_process", 0.0)
    if not _check(not label.visible and label.text == "", "negative source currency was not hidden"):
        return
    inventory.set("quantities", {0: 0})
    hud.call("_process", 0.0)
    if not _check(label.visible and label.text == "0", "currency counter did not reappear at zero"):
        return
    print("HUD native texture regions and currency/font binding passed")
    quit(0)
"#,
    )?;
    let result = Command::new(&godot_bin)
        .arg("--headless")
        .arg("--path")
        .arg(&output)
        .arg("--script")
        .arg("res://check_hud_counter.gd")
        .output()?;
    ensure!(
        result.status.success(),
        "Godot HUD counter visibility check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let stdout = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);
    ensure!(
        stdout.contains("HUD native texture regions and currency/font binding passed"),
        "Godot did not complete the focused HUD runtime checks\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    ensure!(
        !stdout.contains("Invalid assignment of property or key 'region_rect'")
            && !stderr.contains("Invalid assignment of property or key 'region_rect'"),
        "Godot still reported the unsupported TextureRect.region_rect assignment\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
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
