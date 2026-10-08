//! Native, editable projection of the installed Fix HUD's direct-draw slice.
use super::{Project, disk_path, quoted};
use crate::model::{
    AssetPath, AssetRef,
    content::ContentSource,
    hud::{HudArtwork, HudCounterFont, HudPresentation},
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

fn font_atlas_image(
    project: &mut Project,
    source: &mut impl ContentSource,
    font: &HudCounterFont,
) -> Result<String> {
    let reference = AssetRef::<Texture>::from_path(font.atlas.texture.clone());
    let texture = source.load(&reference)?;
    ensure!(
        [texture.width(), texture.height()] == font.atlas.size,
        "HUD source font atlas dimensions changed"
    );
    let mut rgba = texture.rgba8(0)?;
    if font.atlas.color_key {
        rgba = super::material::keyed_rgba(&texture, &rgba);
    }
    let path = disk_path(&project.selection, &font.atlas.texture, "hud-font.png");
    project.write(
        path.clone(),
        &super::material::png(texture.width(), texture.height(), &rgba)?,
    )?;
    Ok(format!("res://{path}"))
}

fn emit(
    value: &HudPresentation,
    images: &BTreeMap<u8, String>,
    font_path: &str,
    selection: &crate::model::Selection,
) -> Result<(String, String)> {
    let script_path = disk_path(selection, &value.path, "gd");
    let mut scene = String::from("[gd_scene load_steps=9 format=3]\n");
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
        "[ext_resource type=\"FontFile\" path={} id=\"counter_font\"]",
        quoted(&format!("res://{font_path}"))
    )?;
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

    let currency_font = value
        .currency_font
        .as_ref()
        .expect("validated HUD currency font");
    let currency_row = table[23];
    let currency_left = currency_row[0] + value.text_offsets[0][0];
    let currency_top = currency_row[1] + value.text_offsets[0][1];
    writeln!(
        scene,
        "\n[node name=\"CurrencyCount\" type=\"Label\" parent=\"Canvas/Source\"]\nlayout_mode = 0\noffset_left = {:?}\noffset_top = {:?}\noffset_right = {:?}\noffset_bottom = {:?}\nmouse_filter = 2\ntheme_override_fonts/font = ExtResource(\"counter_font\")\ntheme_override_colors/font_color = Color({:?}, {:?}, {:?}, {:?})\ntext = \"\"\nvisible = false",
        currency_left,
        currency_top,
        currency_left + 128.0,
        currency_top + 28.0,
        currency_font.color[0],
        currency_font.color[1],
        currency_font.color[2],
        currency_font.color[3]
    )?;

    let script = format!(
        "extends CanvasLayer\n\n@onready var _source: Control = $Canvas/Source\n@onready var _life_fill: TextureRect = $Canvas/Source/PlayerLifeFill\n@onready var _life_remainder: TextureRect = $Canvas/Source/PlayerLifeRemainder\n@onready var _enemy_backgrounds: Array[TextureRect] = [\n    $Canvas/Source/EnemyMeter0Background,\n    $Canvas/Source/EnemyMeter1Background,\n    $Canvas/Source/EnemyMeter2Background,\n]\n@onready var _enemy_fills: Array[TextureRect] = [\n    $Canvas/Source/EnemyMeter0Fill,\n    $Canvas/Source/EnemyMeter1Fill,\n    $Canvas/Source/EnemyMeter2Fill,\n]\n@onready var _currency_label: Label = $Canvas/Source/CurrencyCount\n\nconst LIFE_LEFT: float = {:?}\nconst LIFE_TOP: float = {:?}\nconst LIFE_FILL_WIDTH: float = {:?}\nconst LIFE_RIGHT: float = {:?}\nconst LIFE_BOTTOM: float = {:?}\nconst CURRENCY_ITEM_ID: int = {}\nvar _actor: Node\nvar _last_currency_count: int = -2147483648\n\nfunc _ready() -> void:\n    get_viewport().size_changed.connect(_resize_source_canvas)\n    _resize_source_canvas()\n    _refresh_currency()\n\nfunc _process(_delta: float) -> void:\n    _refresh_currency()\n\nfunc bind_actor(actor: Node) -> void:\n    _actor = actor\n    _last_currency_count = -2147483648\n    if is_node_ready(): _refresh_currency()\n\nfunc _refresh_currency() -> void:\n    if not is_instance_valid(_actor):\n        set_currency_count(-1)\n        return\n    var inventory := _actor.get(\"source_inventory\") as Resource\n    if inventory == null or not inventory.has_method(\"quantity\"):\n        set_currency_count(-1)\n        return\n    set_currency_count(int(inventory.call(\"quantity\", CURRENCY_ITEM_ID)))\n\nfunc set_currency_count(value: int) -> void:\n    if value == _last_currency_count: return\n    _last_currency_count = value\n    _currency_label.visible = value >= 0\n    _currency_label.text = str(value) if value >= 0 else \"\"\n\nfunc _resize_source_canvas() -> void:\n    _source.scale = get_viewport().get_visible_rect().size / Vector2(640.0, 480.0)\n\nfunc set_life_fraction(value: float) -> void:\n    var fraction := clampf(value, 0.0, 1.0)\n    var fill_right := LIFE_LEFT + LIFE_FILL_WIDTH * fraction\n    _life_fill.offset_left = LIFE_LEFT\n    _life_fill.offset_right = fill_right\n    _life_remainder.offset_left = fill_right\n    _life_remainder.offset_right = LIFE_RIGHT\n    var fill_size := _life_fill.texture.get_size()\n    var remainder_size := _life_remainder.texture.get_size()\n    _life_fill.region_rect = Rect2(0.0, 0.0, fill_size.x * fraction, fill_size.y)\n    _life_remainder.region_rect = Rect2(fill_size.x * fraction, 0.0, remainder_size.x * (1.0 - fraction), remainder_size.y)\n    _life_fill.visible = fraction > 0.0\n    _life_remainder.visible = fraction < 1.0 and LIFE_RIGHT > fill_right\n\nfunc set_enemy_meter(slot: int, value: float) -> bool:\n    if slot < 0 or slot >= _enemy_fills.size():\n        return false\n    var fraction := clampf(value, 0.0, 1.0)\n    var fill := _enemy_fills[slot]\n    var left := float(fill.get_meta(\"source_hype_left\"))\n    var right := float(fill.get_meta(\"source_hype_right\"))\n    var extent := float(fill.get_meta(\"source_hype_extent\"))\n    fill.offset_left = left + (1.0 - fraction) * extent\n    fill.offset_right = right\n    var size := fill.texture.get_size()\n    fill.region_rect = Rect2(size.x * (1.0 - fraction), 0.0, size.x * fraction, size.y)\n    _enemy_backgrounds[slot].visible = true\n    fill.visible = fraction > 0.0\n    return true\n\nfunc clear_enemy_meter(slot: int) -> void:\n    if slot < 0 or slot >= _enemy_fills.size():\n        return\n    _enemy_backgrounds[slot].visible = false\n    _enemy_fills[slot].visible = false\n",
        life_left,
        life_top,
        life[4],
        life_right,
        life_bottom,
        value
            .currency_item_id
            .expect("validated HUD currency item id"),
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
    let font = value
        .currency_font
        .as_ref()
        .expect("validated HUD currency font");
    let font_atlas = font_atlas_image(project, source, font)?;
    let directory = disk_path(&project.selection, &value.path, "");
    let directory = directory.trim_end_matches('.');
    let font_path = format!("{directory}/counter.font.tres");
    project.reserve(&font_path)?;
    super::bitmap_font::emit(
        project,
        super::bitmap_font::FontInput {
            name: &font.name,
            atlas: &font_atlas,
            atlas_size: font.atlas.size,
            glyphs: &font.glyphs,
            tracking: font.tracking,
            scale: 640. / 9.,
            path: &format!("res://{font_path}"),
            outline: None,
        },
    )?;
    let (scene, script) = emit(value, &images, &font_path, &project.selection)?;
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
