//! Native, editable projection of the installed Fix HUD's direct-draw slice.
use super::{Project, disk_path, quoted};
use crate::model::{
    AssetPath, AssetRef,
    content::ContentSource,
    hud::{HudArtwork, HudPresentation},
    io::Content,
    texture::Texture,
};
use anyhow::{Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
};

fn image(
    project: &mut Project,
    source: &mut impl ContentSource,
    artwork: &HudArtwork,
) -> Result<String> {
    let reference = AssetRef::<Texture>::from_path(artwork.texture.clone());
    let texture = source.load(&reference)?;
    ensure!(
        [texture.width(), texture.height()] == artwork.logical_size,
        "HUD texture {} changed from its verified Fix GLI size",
        artwork.texture
    );
    let mut rgba = texture.rgba8(0)?;
    if artwork.source_flags & 2 != 0 {
        rgba = super::material::keyed_rgba(&texture, &rgba);
    }
    ensure!(
        artwork.sample_flags == 0 && artwork.addressing == 0,
        "unsupported Fix HUD sampler state"
    );
    let path = disk_path(&project.selection, &artwork.texture, "hud.png");
    project.write(
        path.clone(),
        &super::material::png(texture.width(), texture.height(), &rgba)?,
    )?;
    Ok(format!("res://{path}"))
}

fn node(
    text: &mut String,
    name: &str,
    texture: &str,
    rect: [f32; 4],
    visible: bool,
    region: bool,
    metadata: Option<(&str, f32)>,
) -> Result<()> {
    writeln!(
        text,
        "\n[node name={} type=\"TextureRect\" parent=\"Canvas/Source\"]\nlayout_mode = 0\noffset_left = {:?}\noffset_top = {:?}\noffset_right = {:?}\noffset_bottom = {:?}\nmouse_filter = 2\ntexture = ExtResource({})\nexpand_mode = 1\nstretch_mode = 0\ntexture_filter = 2\nvisible = {}\nregion_enabled = {}",
        quoted(name),
        rect[0],
        rect[1],
        rect[2],
        rect[3],
        quoted(texture),
        visible,
        region
    )?;
    if let Some((key, value)) = metadata {
        writeln!(text, "metadata/{key} = {value:?}")?;
    }
    Ok(())
}

fn row_rect(table: &[[f32; 6]], index: usize) -> [f32; 4] {
    let row = table[index];
    [row[0], row[1], row[2], row[3]]
}

fn emit(
    value: &HudPresentation,
    images: &BTreeMap<u8, String>,
    selection: &crate::model::Selection,
) -> Result<(String, String)> {
    let script_path = disk_path(selection, &value.path, "gd");
    let mut scene = String::from("[gd_scene load_steps=8 format=3]\n");
    writeln!(
        scene,
        "[ext_resource type=\"Script\" path={} id=\"script\"]",
        quoted(&format!("res://{script_path}"))
    )?;
    for artwork in &value.artwork {
        writeln!(
            scene,
            "[ext_resource type=\"Texture2D\" path={} id={}]",
            quoted(&images[&artwork.slot]),
            quoted(&format!("image{}", artwork.slot))
        )?;
    }
    writeln!(
        scene,
        "\n[node name=\"FixHud\" type=\"CanvasLayer\"]\nlayer = 20\nscript = ExtResource(\"script\")\n\n[node name=\"Canvas\" type=\"Control\" parent=\".\"]\nlayout_mode = 3\nanchors_preset = 15\nanchor_right = 1.0\nanchor_bottom = 1.0\ngrow_horizontal = 2\ngrow_vertical = 2\nmouse_filter = 2\n\n[node name=\"Source\" type=\"Control\" parent=\"Canvas\"]\nlayout_mode = 0\noffset_right = 640.0\noffset_bottom = 480.0\nmouse_filter = 2\ntexture_filter = 2"
    )?;

    let table = &value.rectangle_tables[1];
    node(
        &mut scene,
        "PlayerLifeHandle",
        "image2",
        row_rect(table, 0),
        true,
        false,
        None,
    )?;
    let life = table[1];
    let life_left = table[0][2];
    let life_top = life[1];
    let life_fill_right = life_left + life[4];
    let life_right = life_left + life[2];
    let life_bottom = life_top + life[5];
    node(
        &mut scene,
        "PlayerLifeFill",
        "image4",
        [life_left, life_top, life_fill_right, life_bottom],
        false,
        true,
        None,
    )?;
    node(
        &mut scene,
        "PlayerLifeRemainder",
        "image3",
        [life_left, life_top, life_right, life_bottom],
        false,
        true,
        None,
    )?;

    for (slot, background_row, fill_row) in [(0, 11, 12), (1, 13, 14), (2, 15, 16)] {
        node(
            &mut scene,
            &format!("EnemyMeter{slot}Background"),
            "image1",
            row_rect(table, background_row),
            false,
            false,
            None,
        )?;
        let fill = table[fill_row];
        node(
            &mut scene,
            &format!("EnemyMeter{slot}Fill"),
            "image0",
            row_rect(table, fill_row),
            false,
            true,
            Some(("source_hype_extent", fill[4])),
        )?;
        writeln!(
            scene,
            "metadata/source_hype_left = {:?}\nmetadata/source_hype_right = {:?}",
            fill[0], fill[2]
        )?;
    }
    node(
        &mut scene,
        "CurrencyIcon",
        "image5",
        row_rect(table, 23),
        true,
        false,
        None,
    )?;

    let script = format!(
        "extends CanvasLayer\n\n@onready var _source: Control = $Canvas/Source\n@onready var _life_fill: TextureRect = $Canvas/Source/PlayerLifeFill\n@onready var _life_remainder: TextureRect = $Canvas/Source/PlayerLifeRemainder\n@onready var _enemy_backgrounds: Array[TextureRect] = [\n    $Canvas/Source/EnemyMeter0Background,\n    $Canvas/Source/EnemyMeter1Background,\n    $Canvas/Source/EnemyMeter2Background,\n]\n@onready var _enemy_fills: Array[TextureRect] = [\n    $Canvas/Source/EnemyMeter0Fill,\n    $Canvas/Source/EnemyMeter1Fill,\n    $Canvas/Source/EnemyMeter2Fill,\n]\n\nconst LIFE_LEFT: float = {:?}\nconst LIFE_TOP: float = {:?}\nconst LIFE_FILL_WIDTH: float = {:?}\nconst LIFE_RIGHT: float = {:?}\nconst LIFE_BOTTOM: float = {:?}\n\nfunc _ready() -> void:\n    get_viewport().size_changed.connect(_resize_source_canvas)\n    _resize_source_canvas()\n\nfunc _resize_source_canvas() -> void:\n    _source.scale = get_viewport().get_visible_rect().size / Vector2(640.0, 480.0)\n\nfunc set_life_fraction(value: float) -> void:\n    var fraction := clampf(value, 0.0, 1.0)\n    var fill_right := LIFE_LEFT + LIFE_FILL_WIDTH * fraction\n    _life_fill.offset_left = LIFE_LEFT\n    _life_fill.offset_right = fill_right\n    _life_remainder.offset_left = fill_right\n    _life_remainder.offset_right = LIFE_RIGHT\n    var fill_size := _life_fill.texture.get_size()\n    var remainder_size := _life_remainder.texture.get_size()\n    _life_fill.region_rect = Rect2(0.0, 0.0, fill_size.x * fraction, fill_size.y)\n    _life_remainder.region_rect = Rect2(fill_size.x * fraction, 0.0, remainder_size.x * (1.0 - fraction), remainder_size.y)\n    _life_fill.visible = fraction > 0.0\n    _life_remainder.visible = fraction < 1.0 and LIFE_RIGHT > fill_right\n\nfunc set_enemy_meter(slot: int, value: float) -> bool:\n    if slot < 0 or slot >= _enemy_fills.size():\n        return false\n    var fraction := clampf(value, 0.0, 1.0)\n    var fill := _enemy_fills[slot]\n    var left := float(fill.get_meta(\"source_hype_left\"))\n    var right := float(fill.get_meta(\"source_hype_right\"))\n    var extent := float(fill.get_meta(\"source_hype_extent\"))\n    fill.offset_left = left + (1.0 - fraction) * extent\n    fill.offset_right = right\n    var size := fill.texture.get_size()\n    fill.region_rect = Rect2(size.x * (1.0 - fraction), 0.0, size.x * fraction, size.y)\n    _enemy_backgrounds[slot].visible = true\n    fill.visible = fraction > 0.0\n    return true\n\nfunc clear_enemy_meter(slot: int) -> void:\n    if slot < 0 or slot >= _enemy_fills.size():\n        return\n    _enemy_backgrounds[slot].visible = false\n    _enemy_fills[slot].visible = false\n",
        life_left, life_top, life[4], life_right, life_bottom,
    );
    Ok((scene, script))
}

pub(super) fn export(
    project: &mut Project,
    source: &mut impl ContentSource,
    value: &HudPresentation,
    _written: &mut BTreeSet<AssetPath>,
) -> Result<()> {
    value.validate()?;
    ensure!(
        value
            .elements
            .iter()
            .any(|element| element.role == "enemy_meter_background_fill_pairs"),
        "HUD enemy bar row map is missing"
    );
    let mut images = BTreeMap::<u8, String>::new();
    for artwork in &value.artwork {
        images.insert(artwork.slot, image(project, source, artwork)?);
    }
    let (scene, script) = emit(value, &images, &project.selection)?;
    project.write(
        disk_path(&project.selection, &value.path, "gd"),
        script.as_bytes(),
    )?;
    project.write(
        disk_path(&project.selection, &value.path, "tscn"),
        scene.as_bytes(),
    )?;
    Ok(())
}
