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
    ensure!(
        artwork.sample_flags == 0 && artwork.addressing == 0,
        "unsupported Fix HUD sampler state"
    );
    texture_image(
        project,
        source,
        &artwork.texture,
        artwork.logical_size,
        artwork.source_flags,
    )
}

fn texture_image(
    project: &mut Project,
    source: &mut impl ContentSource,
    asset: &AssetPath,
    logical_size: [u32; 2],
    flags: u32,
) -> Result<String> {
    let reference = AssetRef::<Texture>::from_path(asset.clone());
    let texture = source.load(&reference)?;
    project.retain_texture_mips(&reference, &texture)?;
    ensure!(
        [texture.width(), texture.height()] == logical_size,
        "HUD texture {asset} changed from its verified Fix GLI size"
    );
    let mut rgba = texture.rgba8(0)?;
    if flags & 2 != 0 {
        rgba = super::material::keyed_rgba(&texture, &rgba);
    }
    let path = disk_path(&project.selection, asset, "hud.png");
    project.write(
        path.clone(),
        &super::material::png(texture.width(), texture.height(), &rgba)?,
    )?;
    Ok(format!("res://{path}"))
}

/// One native TextureRect per Fix panel layer record, under a hidden Control per
/// panel. The draw API puts v=1 at the top row, so the unflipped image is
/// u 0..1 with v 1..0; the exporter maps the reversed cases to flip_h / flip_v.
fn panel_nodes(
    text: &mut String,
    value: &HudPresentation,
    panel_images: &BTreeMap<AssetPath, usize>,
) -> Result<()> {
    for (index, panel) in value.panels.iter().enumerate() {
        writeln!(
            text,
            "\n[node name=\"Panel{index}\" type=\"Control\" parent=\"Source\"]\nlayout_mode = 0\noffset_right = 640.0\noffset_bottom = 480.0\nmouse_filter = 2\nvisible = false\nmetadata/source_panel = {index}"
        )?;
        for (layer_index, layer) in panel.layers.iter().enumerate() {
            let Some(asset) = &layer.texture_asset else {
                continue;
            };
            let [x0, x1, y0, y1] = layer.destination;
            let [u0, u1, v0, v1] = layer.uv;
            ensure!(
                [u0, u1, v0, v1].iter().all(|v| *v == 0.0 || *v == 1.0) && u0 != u1 && v0 != v1,
                "HUD panel layer UVs are not whole-texture flips"
            );
            writeln!(
                text,
                "\n[node name=\"Layer{layer_index}\" type=\"TextureRect\" parent=\"Source/Panel{index}\"]\nlayout_mode = 0\noffset_left = {x0:?}\noffset_top = {y0:?}\noffset_right = {x1:?}\noffset_bottom = {y1:?}\nmouse_filter = 2\ntexture = ExtResource(\"panel{}\")\nexpand_mode = 1\nstretch_mode = 0\ntexture_filter = 2\nflip_h = {}\nflip_v = {}",
                panel_images[asset],
                u0 > u1,
                v0 < v1
            )?;
        }
    }
    Ok(())
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
        "\n[node name={} type=\"TextureRect\" parent=\"Source\"]\nlayout_mode = 0\noffset_left = {:?}\noffset_top = {:?}\noffset_right = {:?}\noffset_bottom = {:?}\nmouse_filter = 2\ntexture = ExtResource({})\nexpand_mode = 1\nstretch_mode = 0\ntexture_filter = 2\nvisible = {}\nregion_enabled = {}",
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
    project.retain_texture_mips(&reference, &texture)?;
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
    panel_paths: &[(AssetPath, String)],
    font_path: &str,
    selection: &crate::model::Selection,
) -> Result<(String, String)> {
    let script_path = disk_path(selection, &value.path, "gd");
    let mut scene = String::from("[gd_scene load_steps=24 format=3]\n");
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
    if let Some(magic) = &value.magic {
        for artwork in [&magic.slot_overlay, &magic.points_arc] {
            writeln!(
                scene,
                "[ext_resource type=\"Texture2D\" path={} id={}]",
                quoted(&images[&artwork.slot]),
                quoted(&format!("image{}", artwork.slot))
            )?;
        }
    }
    let mut panel_images = BTreeMap::new();
    for (index, (asset, path)) in panel_paths.iter().enumerate() {
        writeln!(
            scene,
            "[ext_resource type=\"Texture2D\" path={} id={}]",
            quoted(path),
            quoted(&format!("panel{index}"))
        )?;
        panel_images.insert(asset.clone(), index);
    }
    writeln!(
        scene,
        "[ext_resource type=\"FontFile\" path={} id=\"counter_font\"]",
        quoted(&format!("res://{font_path}"))
    )?;
    writeln!(
        scene,
        "\n[node name=\"FixHud\" type=\"Control\"]\nlayout_mode = 3\nanchors_preset = 15\nanchor_right = 1.0\nanchor_bottom = 1.0\ngrow_horizontal = 2\ngrow_vertical = 2\nmouse_filter = 2\nscript = ExtResource(\"script\")\n\n[node name=\"Source\" type=\"Control\" parent=\".\"]\nlayout_mode = 0\noffset_right = 640.0\noffset_bottom = 480.0\nmouse_filter = 2\ntexture_filter = 2"
    )?;
    // Spell slots and the magic-points arc (0043a450, 00439e50) are drawn by
    // hud_slots.gd from these source inputs and the active rectangle table.
    if let Some(magic) = &value.magic {
        let rows: Vec<String> = value.rectangle_tables[1]
            .iter()
            .flatten()
            .map(|v| format!("{v:?}"))
            .collect();
        writeln!(
            scene,
            "metadata/source_rectangles = PackedFloat32Array({})\nmetadata/magic_slot_kinds = PackedStringArray({})\nmetadata/magic_slot_rows = PackedInt32Array({})\nmetadata/magic_slot_overlay = ExtResource({})\nmetadata/magic_points_arc = ExtResource({})",
            rows.join(", "),
            magic.slot_kinds.iter().map(|k| quoted(k)).collect::<Vec<_>>().join(", "),
            magic.slot_rows.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(", "),
            quoted(&format!("image{}", magic.slot_overlay.slot)),
            quoted(&format!("image{}", magic.points_arc.slot)),
        )?;
    }

    panel_nodes(&mut scene, value, &panel_images)?;

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

    // FUN_0040ca40 draws the weapon count at row 10's corner plus the second
    // Fix.sda text offset (DAT_0060f5b0); hud_slots places its label there.
    let weapon_row = table[10];
    writeln!(
        scene,
        "\n[node name=\"WeaponCountAnchor\" type=\"Control\" parent=\"Source\"]\nlayout_mode = 0\noffset_left = {:?}\noffset_top = {:?}\noffset_right = {:?}\noffset_bottom = {:?}\nmouse_filter = 2",
        weapon_row[0] + value.text_offsets[1][0],
        weapon_row[1] + value.text_offsets[1][1],
        weapon_row[0] + value.text_offsets[1][0],
        weapon_row[1] + value.text_offsets[1][1],
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
        "\n[node name=\"CurrencyCount\" type=\"Label\" parent=\"Source\"]\nlayout_mode = 0\noffset_left = {:?}\noffset_top = {:?}\noffset_right = {:?}\noffset_bottom = {:?}\nmouse_filter = 2\ntheme_override_fonts/font = ExtResource(\"counter_font\")\ntheme_override_colors/font_color = Color({:?}, {:?}, {:?}, {:?})\ntext = \"\"\nvisible = false",
        currency_left,
        currency_top,
        currency_left + 128.0,
        currency_top + 28.0,
        currency_font.color[0],
        currency_font.color[1],
        currency_font.color[2],
        currency_font.color[3]
    )?;

    let script_template = r#"extends Control

@onready var _life_handle: TextureRect = $Source/PlayerLifeHandle
@onready var _life_fill: TextureRect = $Source/PlayerLifeFill
@onready var _life_remainder: TextureRect = $Source/PlayerLifeRemainder
@onready var _currency_icon: TextureRect = $Source/CurrencyIcon
@onready var _currency_label: Label = $Source/CurrencyCount
@onready var _enemy_backgrounds: Array[TextureRect] = [
    $Source/EnemyMeter0Background,
    $Source/EnemyMeter1Background,
    $Source/EnemyMeter2Background,
]
@onready var _enemy_fills: Array[TextureRect] = [
    $Source/EnemyMeter0Fill,
    $Source/EnemyMeter1Fill,
    $Source/EnemyMeter2Fill,
]
## Fix panel descriptor 1, the right-edge frame: FUN_0040c4a0 enables it with
## the first enemy row and FUN_0040c8a0 disables it once no row remains.
@onready var _enemy_panel: Control = $Source/Panel1
var _life_fill_atlas: AtlasTexture
var _life_remainder_atlas: AtlasTexture
var _enemy_fill_atlases: Array[AtlasTexture] = []

const LIFE_LEFT: float = @@LIFE_LEFT@@
const LIFE_TOP: float = @@LIFE_TOP@@
const LIFE_FILL_WIDTH: float = @@LIFE_FILL_WIDTH@@
const LIFE_RIGHT: float = @@LIFE_RIGHT@@
const LIFE_BOTTOM: float = @@LIFE_BOTTOM@@
const CURRENCY_ITEM_ID: int = @@CURRENCY_ITEM_ID@@

var suppressed: bool = false
var _actor: Node
var _requested: Dictionary = {"life": false, "armor": false, "magic": false, "selected_object": false, "weapon": false, "cash": false}
var _enemy_active: Array[bool] = [false, false, false]
var _enemy_fill_active: Array[bool] = [false, false, false]
var _life_fill_active: bool = false
var _life_remainder_active: bool = true
var _last_currency_count: int = -2147483648

func _ready() -> void:
    _life_fill_atlas = _wrap_texture_region(_life_fill)
    _life_remainder_atlas = _wrap_texture_region(_life_remainder)
    for fill in _enemy_fills:
        _enemy_fill_atlases.append(_wrap_texture_region(fill))
    refresh()

func _wrap_texture_region(rect: TextureRect) -> AtlasTexture:
    var source := rect.texture
    var atlas := AtlasTexture.new()
    atlas.atlas = source
    atlas.region = Rect2(Vector2.ZERO, source.get_size())
    rect.texture = atlas
    return atlas

func bind_actor(actor: Node) -> void:
    if is_instance_valid(_actor) and _actor.has_signal("hud_visibility_requested"):
        var old_callback := Callable(self, "set_indicator_visible")
        if _actor.is_connected("hud_visibility_requested", old_callback):
            _actor.disconnect("hud_visibility_requested", old_callback)
    _actor = actor
    _last_currency_count = -2147483648
    for key in _requested: _requested[key] = false
    if is_instance_valid(_actor):
        if _actor.has_signal("hud_visibility_requested"):
            _actor.connect("hud_visibility_requested", Callable(self, "set_indicator_visible"))
        var values: Variant = _actor.get("source_hud_display")
        if values is Dictionary:
            for key in values:
                if _requested.has(key): _requested[key] = bool(values[key])
    if is_node_ready():
        _refresh_currency()
        refresh()

func set_indicator_visible(indicator: String, enabled: bool) -> void:
    if not _requested.has(indicator): return
    _requested[indicator] = enabled
    refresh()

func refresh() -> void:
    var show_life := bool(_requested.get("life", false)) and not suppressed
    _life_handle.visible = show_life
    _life_fill.visible = show_life and _life_fill_active
    _life_remainder.visible = show_life and _life_remainder_active
    for slot in _enemy_fills.size():
        _enemy_backgrounds[slot].visible = _enemy_active[slot] and not suppressed
        _enemy_fills[slot].visible = _enemy_active[slot] and _enemy_fill_active[slot] and not suppressed
    _enemy_panel.visible = _enemy_active.has(true) and not suppressed
    var show_cash := bool(_requested.get("cash", false)) and not suppressed
    _currency_icon.visible = show_cash
    _currency_label.visible = _last_currency_count >= 0 and show_cash

func _process(_delta: float) -> void:
    if not is_instance_valid(_actor): return
    var points: Resource = _actor.get("source_hit_points") as Resource
    if points != null:
        var maximum := int(points.get("maximum"))
        if maximum > 0:
            set_life_fraction(float(points.get("current")) / float(maximum))
    _refresh_currency()

func _refresh_currency() -> void:
    if not is_instance_valid(_actor):
        set_currency_count(-1)
        return
    var inventory := _actor.get("source_inventory") as Resource
    if inventory == null or not inventory.has_method("quantity"):
        set_currency_count(-1)
        return
    set_currency_count(int(inventory.call("quantity", CURRENCY_ITEM_ID)))

func set_currency_count(value: int) -> void:
    if value == _last_currency_count: return
    _last_currency_count = value
    _currency_label.text = str(value) if value >= 0 else ""
    _currency_label.visible = value >= 0 and bool(_requested.get("cash", false)) and not suppressed

func set_life_fraction(value: float) -> void:
    var fraction := clampf(value, 0.0, 1.0)
    var fill_right := LIFE_LEFT + LIFE_FILL_WIDTH * fraction
    _life_fill.offset_left = LIFE_LEFT
    _life_fill.offset_right = fill_right
    _life_remainder.offset_left = fill_right
    _life_remainder.offset_right = LIFE_RIGHT
    var fill_size := _life_fill_atlas.atlas.get_size()
    var remainder_size := _life_remainder_atlas.atlas.get_size()
    if fraction > 0.0:
        _life_fill_atlas.region = Rect2(0.0, 0.0, fill_size.x * fraction, fill_size.y)
    if fraction < 1.0:
        _life_remainder_atlas.region = Rect2(fill_size.x * fraction, 0.0, remainder_size.x * (1.0 - fraction), remainder_size.y)
    _life_fill_active = fraction > 0.0
    _life_remainder_active = fraction < 1.0 and LIFE_RIGHT > fill_right
    refresh()

func set_enemy_meter(slot: int, value: float) -> bool:
    if slot < 0 or slot >= _enemy_fills.size():
        return false
    var fraction := clampf(value, 0.0, 1.0)
    var fill := _enemy_fills[slot]
    var left := float(fill.get_meta("source_hype_left"))
    var right := float(fill.get_meta("source_hype_right"))
    var extent := float(fill.get_meta("source_hype_extent"))
    fill.offset_left = left + (1.0 - fraction) * extent
    fill.offset_right = right
    var atlas := _enemy_fill_atlases[slot]
    var tex_size := atlas.atlas.get_size()
    if fraction > 0.0:
        atlas.region = Rect2(tex_size.x * (1.0 - fraction), 0.0, tex_size.x * fraction, tex_size.y)
    _enemy_active[slot] = true
    _enemy_fill_active[slot] = fraction > 0.0
    refresh()
    return true

func clear_enemy_meter(slot: int) -> void:
    if slot < 0 or slot >= _enemy_fills.size():
        return
    _enemy_active[slot] = false
    _enemy_fill_active[slot] = false
    refresh()
"#;
    let script = script_template
        .replace("@@LIFE_LEFT@@", &format!("{life_left:?}"))
        .replace("@@LIFE_TOP@@", &format!("{life_top:?}"))
        .replace("@@LIFE_FILL_WIDTH@@", &format!("{:?}", life[4]))
        .replace("@@LIFE_RIGHT@@", &format!("{life_right:?}"))
        .replace("@@LIFE_BOTTOM@@", &format!("{life_bottom:?}"))
        .replace(
            "@@CURRENCY_ITEM_ID@@",
            &value
                .currency_item_id
                .expect("validated HUD currency item id")
                .to_string(),
        );
    Ok((scene, script))
}

pub(super) fn export(
    project: &mut Project,
    source: &mut impl ContentSource,
    value: &HudPresentation,
    written: &mut BTreeSet<AssetPath>,
) -> Result<()> {
    if !written.insert(value.path.clone()) {
        return Ok(());
    }
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
    if let Some(magic) = &value.magic {
        for artwork in [&magic.slot_overlay, &magic.points_arc] {
            images.insert(artwork.slot, image(project, source, artwork)?);
        }
    }
    let mut panel_paths = Vec::<(AssetPath, String)>::new();
    for layer in value.panels.iter().flat_map(|panel| &panel.layers) {
        if let Some(asset) = &layer.texture_asset
            && !panel_paths.iter().any(|(seen, _)| seen == asset)
        {
            let path = texture_image(
                project,
                source,
                asset,
                layer.texture_logical_size,
                layer.texture_flags,
            )?;
            panel_paths.push((asset.clone(), path));
        }
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
    let (scene, script) = emit(value, &images, &panel_paths, &font_path, &project.selection)?;
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
