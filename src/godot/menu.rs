//! Bake menu sprite quads into ordinary textures before the authored game runs.
use super::{Project, disk_path, quoted};
use crate::model::{AssetRef, content::ContentSource, io::Save, menu::*, texture::Texture};
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeMap, fmt::Write as _};

fn raster(
    item: &MenuItem,
    sprites: &[MenuSprite],
    source: &mut impl ContentSource,
) -> Result<Vec<u8>> {
    // Two samples per virtual pixel retain fractional source glyph metrics.
    let width = item.rect[2] as u32 * 2;
    let height = item.rect[3] as u32 * 2;
    ensure!(
        width > 0 && height > 0 && width <= 1280 && height <= 960,
        "menu raster extent differs"
    );
    let mut rgba = vec![0u8; width as usize * height as usize * 4];
    for p in rgba.as_chunks_mut::<4>().0 {
        p[3] = 255;
    }
    let mut textures = BTreeMap::new();
    for sprite in sprites {
        if !textures.contains_key(&sprite.texture.path) {
            let texture = source.load(&sprite.texture)?;
            textures.insert(
                sprite.texture.path.clone(),
                (texture.width(), texture.height(), texture.rgba8(0)?),
            );
        }
        let (tw, th, pixels) = &textures[&sprite.texture.path];
        let [x, y, w, h] = sprite.rect;
        for py in (y * 2.).floor().max(0.) as u32..((y + h) * 2.).ceil().min(height as f32) as u32 {
            for px in
                (x * 2.).floor().max(0.) as u32..((x + w) * 2.).ceil().min(width as f32) as u32
            {
                let u = ((px as f32 + 0.5) / 2. - x) / w;
                let v = ((py as f32 + 0.5) / 2. - y) / h;
                if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
                    continue;
                }
                let tx = (sprite.uv[0][0] + sprite.uv[1][0] * u) * (*tw - 1) as f32;
                let ty = (1. - sprite.uv[0][1] - sprite.uv[1][1] * (1. - v)) * (*th - 1) as f32;
                let lo_x = tx.floor().clamp(0., (*tw - 1) as f32) as u32;
                let lo_y = ty.floor().clamp(0., (*th - 1) as f32) as u32;
                let hi_x = (lo_x + 1).min(*tw - 1);
                let hi_y = (lo_y + 1).min(*th - 1);
                let fx = tx - tx.floor();
                let fy = ty - ty.floor();
                for c in 0..3 {
                    let at = |sx: u32, sy: u32| pixels[((sy * tw + sx) * 4) as usize + c] as f32;
                    let value = (at(lo_x, lo_y) * (1. - fx) + at(hi_x, lo_y) * fx) * (1. - fy)
                        + (at(lo_x, hi_y) * (1. - fx) + at(hi_x, hi_y) * fx) * fy;
                    let index = ((py * width + px) * 4) as usize + c;
                    rgba[index] = rgba[index].saturating_add(value.round() as u8);
                }
            }
        }
    }
    super::material::png(width, height, &rgba)
}
pub(super) fn export(
    project: &mut Project,
    source: &mut impl ContentSource,
    value: &MenuPresentation,
) -> Result<()> {
    let directory = disk_path(&project.selection, &value.path, "");
    let directory = directory.trim_end_matches('.');
    for item in &value.items {
        for (name, sprites) in [("normal", &item.normal), ("selected", &item.selected)] {
            let bytes = raster(item, sprites, source)?;
            project.write(format!("{directory}/{}_{name}.png", item.name), &bytes)?;
        }
    }
    let configuration_directory = if let Some(configuration) = &value.configuration {
        let directory = disk_path(&project.selection, &configuration.path, "");
        let directory = directory.trim_end_matches('.').to_owned();
        for item in &configuration.items {
            for (state, sprites) in [("normal", &item.normal), ("selected", &item.selected)] {
                project.write(
                    format!("{directory}/{}_{state}.png", item.name),
                    &raster(item, sprites, source)?,
                )?;
            }
        }
        let background = source.load(&configuration.background)?;
        project.save(&configuration.background, background.as_ref())?;
        for picture in &configuration.title {
            let texture = source.load(&picture.texture)?;
            project.save(&picture.texture, texture.as_ref())?;
        }
        if let Some(controls) = &configuration.controls {
            export_controls(project, source, controls)?;
        }
        Some(directory)
    } else {
        None
    };
    for reference in [&value.navigation, &value.confirmation] {
        let sound = source.load(reference)?;
        project.save(reference, sound.as_ref())?;
    }
    if let Some(reference) = &value.background {
        let background = source.load(reference)?;
        project.save(reference, background.as_ref())?;
    }
    if let Some(credits) = &value.credits {
        export_credits(project, source, credits)?;
    }
    if let (Some(configuration), Some(directory)) = (&value.configuration, &configuration_directory)
    {
        project.write(
            format!("{directory}/menu.tscn"),
            configuration_scene(project, configuration, directory)?.as_bytes(),
        )?;
    }
    project.write(
        format!("{directory}/menu.tscn"),
        scene(project, value, directory)?.as_bytes(),
    )?;
    project.diagnostic(format!("{}: English source layout/actions/navigation exported as editable native Controls; original normal/glow artwork baked ahead of runtime; Sound settings preserve source mixer values and stereo actions, Images maps source shadow/framing settings and marks source texture swapping unsupported, and Controls opens the original remapping page whose bindings feed every script action",value.path));
    Ok(())
}

fn export_credits(
    project: &mut Project,
    source: &mut impl ContentSource,
    credits: &CreditsPresentation,
) -> Result<()> {
    credits.validate()?;
    let directory = disk_path(&project.selection, &credits.path, "");
    let directory = directory.trim_end_matches('.');
    let atlas_path = disk_path(
        &project.selection,
        &credits.font.atlas.texture,
        "credits.png",
    );
    let font_path = disk_path(&project.selection, &credits.path, "font.tres");
    let atlas = source.load(&AssetRef::<Texture>::from_path(
        credits.font.atlas.texture.clone(),
    ))?;
    ensure!(
        [atlas.width(), atlas.height()] == credits.font.atlas.size,
        "credits atlas dimensions differ"
    );
    let rgba = atlas.rgba8(0)?;
    let rgba = if credits.font.atlas.color_key {
        super::material::keyed_rgba(&atlas, &rgba)
    } else {
        rgba
    };
    project.write(
        atlas_path.clone(),
        &super::material::png(atlas.width(), atlas.height(), &rgba)?,
    )?;
    project.reserve(&font_path)?;
    super::bitmap_font::emit(
        project,
        super::bitmap_font::FontInput {
            name: &credits.font.name,
            atlas: &format!("res://{atlas_path}"),
            atlas_size: credits.font.atlas.size,
            glyphs: &credits.font.glyphs,
            tracking: credits.font.tracking,
            scale: 640.0 / (2.0 * 4.65),
            path: &format!("res://{font_path}"),
            outline: None,
        },
    )?;
    project.write(
        format!("{directory}/lines.tscn"),
        credits_scene(credits, &font_path)?.as_bytes(),
    )?;
    project.diagnostic(format!(
        "{}: imported {} English credit keys/text handles and native Playmo_Font2_0 glyph atlas; source draw count is {}; +/- keys retain left/right paired-row alignment",
        credits.path,
        credits.entries.len(),
        credits.displayed_entries
    ));
    Ok(())
}

fn credits_scene(credits: &CreditsPresentation, font_path: &str) -> Result<String> {
    const SCALE: f32 = 640.0 / (2.0 * 4.65);
    let mut text = format!(
        "[gd_scene load_steps=2 format=3]\n\n[ext_resource type=\"FontFile\" path={} id=\"font\"]\n\n[node name=\"CreditsLines\" type=\"Control\"]\nlayout_mode = 0\noffset_right = 640.0\noffset_bottom = 480.0\nmetadata/source_registered_entries = {}\nmetadata/source_displayed_entries = {}\nmetadata/source_max_visible_lines = {}\nmetadata/source_text_spacing = {}\nmetadata/source_text_position = Vector2({}, {})\n",
        quoted(&format!("res://{font_path}")),
        credits.entries.len(),
        credits.displayed_entries,
        credits.max_visible_lines,
        credits.text_spacing,
        credits.text_position[0],
        credits.text_position[1],
    );
    let mut right_aligned_before = 0usize;
    let mut final_world_y = -4.3f32;
    for (index, entry) in credits
        .entries
        .iter()
        .take(credits.displayed_entries)
        .enumerate()
    {
        // FUN_0040eb10 aligns +/- entries horizontally; its local_28 counter
        // removes one row step after a right-aligned (+) entry, pairing the
        // following left-aligned (-) name on that same baseline.
        let row = index - right_aligned_before;
        let world_y = -4.3 - row as f32 * credits.text_spacing;
        let pixel_y = (3.49 - world_y) * SCALE;
        let center_x = (4.65 + credits.text_position[0]) * SCALE;
        let (left, right) = match entry.alignment {
            0 => (center_x, 640.0),
            2 => (0.0, center_x),
            _ => (0.0, 640.0),
        };
        final_world_y = world_y;
        writeln!(
            text,
            "\n[node name=\"Credit{index:03}\" type=\"Label\" parent=\".\"]\nlayout_mode = 0\noffset_left = {}\noffset_top = {}\noffset_right = {}\noffset_bottom = {}\ntext = {}\nhorizontal_alignment = {}\nvertical_alignment = 1\nautowrap_mode = 0\ntheme_override_fonts/font = ExtResource(\"font\")\ntheme_override_font_sizes/font_size = 21\ntheme_override_colors/font_color = Color({}, {}, {}, {})\nmetadata/source_key = {}\nmetadata/source_world_y = {}",
            left,
            pixel_y - 11.0,
            right,
            pixel_y + 11.0,
            quoted(&entry.text),
            entry.alignment,
            credits.font.color[0],
            credits.font.color[1],
            credits.font.color[2],
            credits.font.color[3],
            quoted(&entry.key),
            world_y
        )?;
        if entry.alignment == 2 {
            right_aligned_before += 1;
        }
    }
    let scroll_distance = 4.3 - final_world_y;
    writeln!(
        text,
        "\nmetadata/source_scroll_distance = {scroll_distance}"
    )?;
    Ok(text)
}
fn node_name(name: &str) -> String {
    name.split('_')
        .map(|part| {
            let mut chars = part.chars();
            chars.next().unwrap().to_ascii_uppercase().to_string() + chars.as_str()
        })
        .collect()
}
fn scene(project: &Project, value: &MenuPresentation, directory: &str) -> Result<String> {
    let mut text = format!(
        "[gd_scene load_steps={} format=3]\n\n",
        value.items.len() * 2 + 7 + usize::from(value.background.is_some())
    );
    for (id, kind, path) in [
        ("menu", "Script", "addons/astrolabe/ui/menu/menu.gd"),
        ("button", "Script", "addons/astrolabe/ui/menu/button.gd"),
        ("theme", "Theme", "addons/astrolabe/ui/menu/theme.tres"),
    ] {
        writeln!(
            text,
            "[ext_resource type={} path={} id={}]",
            quoted(kind),
            quoted(&format!("res://{path}")),
            quoted(id)
        )?;
    }
    if let Some(reference) = &value.background {
        writeln!(
            text,
            "[ext_resource type=\"Texture2D\" path={} id=\"background\"]",
            quoted(&format!(
                "res://{}",
                disk_path(&project.selection, &reference.path, "png")
            ))
        )?;
    }
    for item in &value.items {
        for state in ["normal", "selected"] {
            writeln!(
                text,
                "[ext_resource type=\"Texture2D\" path={} id={}]",
                quoted(&format!("res://{directory}/{}_{state}.png", item.name)),
                quoted(&format!("{}_{state}", item.name))
            )?;
        }
    }
    for (name, reference) in [
        ("navigation", &value.navigation),
        ("confirmation", &value.confirmation),
    ] {
        writeln!(
            text,
            "[ext_resource type=\"AudioStream\" path={} id={}]",
            quoted(&format!(
                "res://{}",
                disk_path(&project.selection, &reference.path, "tres")
            )),
            quoted(name)
        )?;
    }
    let root_name = if value.background.is_some() {
        "MainMenu"
    } else {
        "PauseMenu"
    };
    writeln!(
        text,
        "\n[sub_resource type=\"CanvasItemMaterial\" id=\"Additive\"]\nblend_mode = 1\n\n[node name=\"{root_name}\" type=\"Control\"]\nlayout_mode = 3\nanchors_preset = 15\nanchor_right = 1.0\nanchor_bottom = 1.0\ngrow_horizontal = 2\ngrow_vertical = 2\ntheme = ExtResource(\"theme\")\nscript = ExtResource(\"menu\")"
    )?;
    if value.background.is_none() {
        text.push_str("background_required = false\n");
    }
    let initial = value
        .items
        .iter()
        .find(|i| i.action == value.initial_item)
        .context("initial menu action absent")?;
    writeln!(
        text,
        "initial_focus = NodePath({})",
        quoted(&format!("Canvas/{}", node_name(&initial.name)))
    )?;
    text.push_str("\n[node name=\"Canvas\" type=\"Control\" parent=\".\"]\nlayout_mode = 0\noffset_right = 640.0\noffset_bottom = 480.0\nmouse_filter = 2\n");
    if value.background.is_some() {
        text.push_str("\n[node name=\"Background\" type=\"TextureRect\" parent=\"Canvas\"]\nlayout_mode = 0\noffset_right = 640.0\noffset_bottom = 480.0\nmouse_filter = 2\ntexture = ExtResource(\"background\")\nexpand_mode = 1\nstretch_mode = 0\n");
    }
    for item in &value.items {
        let name = node_name(&item.name);
        let [relative_x, relative_y, w, h] = item.rect;
        let x = item.position[0] + relative_x;
        let y = item.position[1] + relative_y;
        writeln!(
            text,
            "\n[node name={} type=\"Button\" parent=\"Canvas\"]\nlayout_mode = 0\noffset_left = {x}\noffset_top = {y}\noffset_right = {}\noffset_bottom = {}\ntext = {}\nscript = ExtResource(\"button\")\naction = &{}\nnormal_texture = ExtResource({})\nselected_texture = ExtResource({})\nmetadata/source_anchor = Vector2({}, {})\nmetadata/source_alignment = {}",
            quoted(&name),
            x + w,
            y + h,
            quoted(&item.label),
            quoted(&item.action),
            quoted(&format!("{}_normal", item.name)),
            quoted(&format!("{}_selected", item.name)),
            item.position[0],
            item.position[1],
            item.horizontal_alignment
        )?;
        for (index, direction) in ["top", "bottom", "left", "right"].into_iter().enumerate() {
            let target = item.neighbors[index]
                .as_ref()
                .and_then(|a| value.items.iter().find(|i| &i.action == a))
                .map_or_else(|| name.clone(), |i| node_name(&i.name));
            writeln!(
                text,
                "focus_neighbor_{direction} = NodePath({})",
                quoted(&format!("../{target}"))
            )?;
            if index < 2 {
                writeln!(
                    text,
                    "focus_{} = NodePath({})",
                    if index == 0 { "previous" } else { "next" },
                    quoted(&format!("../{target}"))
                )?;
            }
        }
        writeln!(
            text,
            "\n[node name=\"Artwork\" type=\"TextureRect\" parent={}]\nmaterial = SubResource(\"Additive\")\nlayout_mode = 1\nanchors_preset = 15\nanchor_right = 1.0\nanchor_bottom = 1.0\ngrow_horizontal = 2\ngrow_vertical = 2\nmouse_filter = 2\ntexture = ExtResource({})\nexpand_mode = 1\nstretch_mode = 0",
            quoted(&format!("Canvas/{name}")),
            quoted(&format!("{}_normal", item.name))
        )?;
    }
    text.push_str("\n[node name=\"Audio\" type=\"Node\" parent=\".\"]\n");
    for (name, id, gain) in [
        ("Navigation", "navigation", value.navigation_gain),
        ("Confirmation", "confirmation", value.confirmation_gain),
    ] {
        writeln!(
            text,
            "\n[node name={} type=\"AudioStreamPlayer\" parent=\"Audio\"]\nbus = \"Astrolabe Effects\"\nstream = ExtResource({})\nvolume_db = {}",
            quoted(name),
            quoted(id),
            if gain > 0. { 20. * gain.log10() } else { -80. }
        )?;
    }
    Ok(text)
}

fn configuration_scene(
    project: &Project,
    configuration: &MenuConfiguration,
    directory: &str,
) -> Result<String> {
    let find = |action: &str, label: &str| {
        configuration
            .items
            .iter()
            .find(|item| item.action == action && item.label == label)
            .with_context(|| format!("source configuration item {action}/{label} absent"))
    };
    let tabs = [
        ("Images", find("Images", "image")?),
        ("Controls", find("Controls", "controls")?),
        ("Sound", find("Sound", "sound")?),
    ];
    let sound_rows = [
        (find("DisplayWithSound", "music")?, "Music"),
        (find("DisplayWithSound", "sound")?, "Effects"),
        (find("DisplayWithSound", "voices")?, "Voices"),
    ];
    let stereo_rows = [
        find("DisplayWithSound", "stereo test")?,
        find("DisplayWithSound", "reverse stereo")?,
    ];
    // Keep the source settings' meanings. The texture-swapping ratio has no
    // corresponding Godot renderer setting, so its original row is explicit
    // about being unsupported instead of substituting material filtering.
    let image_rows = [
        (find("DisplayWithImage", "graphic")?, "Graphic"),
        (find("DisplayWithImage", "screen")?, "Screen"),
        (find("DisplayWithImage", "textures")?, "Textures"),
    ];
    let initial = configuration
        .items
        .iter()
        .find(|item| item.name == configuration.initial_item)
        .context("source configuration initial item absent")?;
    let initial_page = match initial.action.as_str() {
        "Images" | "Controls" | "Sound" => initial.action.as_str(),
        action => anyhow::bail!("unsupported source configuration initial action {action}"),
    };
    let ext_count = 2
        + 1
        + configuration.title.len()
        + tabs.len() * 2
        + sound_rows.len()
        + stereo_rows.len() * 2
        + image_rows.len();
    let load_steps = ext_count + 2; // one material subresource and the scene itself
    let mut text = format!("[gd_scene load_steps={load_steps} format=3]\n\n");
    for (id, kind, path) in [
        ("settings", "Script", "game/settings_menu.gd"),
        ("theme", "Theme", "addons/astrolabe/ui/menu/theme.tres"),
    ] {
        writeln!(
            text,
            "[ext_resource type={} path={} id={}]",
            quoted(kind),
            quoted(&format!("res://{path}")),
            quoted(id)
        )?;
    }
    writeln!(
        text,
        "[ext_resource type=\"Texture2D\" path={} id=\"background\"]",
        quoted(&format!(
            "res://{}",
            disk_path(&project.selection, &configuration.background.path, "png")
        ))
    )?;
    for picture in &configuration.title {
        writeln!(
            text,
            "[ext_resource type=\"Texture2D\" path={} id={}]",
            quoted(&format!(
                "res://{}",
                disk_path(&project.selection, &picture.texture.path, "png")
            )),
            quoted(&picture.name)
        )?;
    }
    for (_, item) in tabs {
        for state in ["normal", "selected"] {
            writeln!(
                text,
                "[ext_resource type=\"Texture2D\" path={} id={}]",
                quoted(&format!("res://{directory}/{}_{}.png", item.name, state)),
                quoted(&format!("{}_{}", item.name, state))
            )?;
        }
    }
    for (item, _) in sound_rows.iter().chain(&image_rows) {
        writeln!(
            text,
            "[ext_resource type=\"Texture2D\" path={} id={}]",
            quoted(&format!("res://{directory}/{}_normal.png", item.name)),
            quoted(&format!("{}_normal", item.name))
        )?;
    }
    for item in stereo_rows {
        for state in ["normal", "selected"] {
            writeln!(
                text,
                "[ext_resource type=\"Texture2D\" path={} id={}]",
                quoted(&format!("res://{directory}/{}_{state}.png", item.name)),
                quoted(&format!("{}_{state}", item.name))
            )?;
        }
    }
    writeln!(
        text,
        "\n[sub_resource type=\"CanvasItemMaterial\" id=\"Additive\"]\nblend_mode = 1\n\n[node name=\"ConfigurationMenu\" type=\"Control\"]\nlayout_mode = 3\nanchors_preset = 15\nanchor_right = 1.0\nanchor_bottom = 1.0\ngrow_horizontal = 2\ngrow_vertical = 2\ntheme = ExtResource(\"theme\")\nscript = ExtResource(\"settings\")\ncanvas_size = Vector2(640, 480)\ninitial_focus = NodePath(\"Canvas/Tab{}\")\ninitial_page = &{}\nmetadata/source_initial_item = {}\n\n[node name=\"Canvas\" type=\"Control\" parent=\".\"]\nlayout_mode = 0\noffset_right = 640.0\noffset_bottom = 480.0\nmouse_filter = 2\n\n[node name=\"Background\" type=\"TextureRect\" parent=\"Canvas\"]\nlayout_mode = 0\noffset_right = 640.0\noffset_bottom = 480.0\nmouse_filter = 2\ntexture = ExtResource(\"background\")\nexpand_mode = 1\nstretch_mode = 0",
        initial_page,
        quoted(initial_page),
        quoted(&configuration.initial_item)
    )?;
    // The root's stained-glass title tiles, drawn over the backdrop.
    for picture in &configuration.title {
        writeln!(
            text,
            "\n[node name={} type=\"TextureRect\" parent=\"Canvas\"]\nlayout_mode = 0\noffset_left = {}\noffset_top = {}\noffset_right = {}\noffset_bottom = {}\nmouse_filter = 2\ntexture = ExtResource({})\nexpand_mode = 1\nstretch_mode = 0",
            quoted(&picture.name),
            picture.center[0] - picture.size[0] * 0.5,
            picture.center[1] - picture.size[1] * 0.5,
            picture.center[0] + picture.size[0] * 0.5,
            picture.center[1] + picture.size[1] * 0.5,
            quoted(&picture.name)
        )?;
    }
    // Keep the original source tab artwork and graph navigation. The page state
    // remains selected after focus moves to a native control.
    for (page, item) in tabs {
        append_configuration_tab(&mut text, configuration, page, item)?;
    }
    for (page, visible) in [
        ("SoundPage", initial_page == "Sound"),
        ("ImagesPage", initial_page == "Images"),
        ("ControlsPage", initial_page == "Controls"),
    ] {
        writeln!(
            text,
            "\n[node name={} type=\"Control\" parent=\"Canvas\"]\nlayout_mode = 0\noffset_right = 640.0\noffset_bottom = 480.0\nmouse_filter = 2\nvisible = {}",
            quoted(page),
            visible
        )?;
        if page == "ControlsPage"
            && let Some(controls) = &configuration.controls
        {
            let directory = disk_path(&project.selection, &controls.path, "");
            writeln!(
                text,
                "metadata/controls_scene = {}",
                quoted(&format!(
                    "res://{}/menu.tscn",
                    directory.trim_end_matches('.')
                ))
            )?;
        }
    }
    let [music, effects, voices] = sound_rows;
    for (item, bus) in [
        (music.0, music.1),
        (effects.0, effects.1),
        (voices.0, voices.1),
    ] {
        append_configuration_label(&mut text, item, "Canvas/SoundPage", bus)?;
    }
    text.push_str(
        "\n[node name=\"Audio\" type=\"Control\" parent=\"Canvas/SoundPage\"]\nlayout_mode = 0\noffset_right = 640.0\noffset_bottom = 480.0\nmouse_filter = 2",
    );
    for (index, (item, bus)) in sound_rows.iter().enumerate() {
        let center_x = item.position[0];
        let (minimum, maximum) = if *bus == "Voices" {
            (-20.0, 107.0)
        } else {
            (0.0, 127.0)
        };
        writeln!(
            text,
            "\n[node name={} type=\"VSlider\" parent=\"Canvas/SoundPage/Audio\"]\nlayout_mode = 0\noffset_left = {}\noffset_top = 220.0\noffset_right = {}\noffset_bottom = 338.0\nmin_value = {}\nmax_value = {}\nstep = 1.0\nvalue = 100.0\nmetadata/source_slider = \"SoundSlider{}\"\nmetadata/source_position = Vector3({}, {}, {})\nmetadata/source_action = \"DisplayWithSound\"\nmetadata/volume_bus = {}",
            quoted(bus),
            center_x - 10.0,
            center_x + 10.0,
            minimum,
            maximum,
            index + 1,
            configuration.slider_positions[index][0],
            configuration.slider_positions[index][1],
            configuration.slider_positions[index][2],
            quoted(bus)
        )?;
    }
    for (index, (item, label)) in image_rows.iter().enumerate() {
        let item = *item;
        let label = *label;
        append_configuration_label(
            &mut text,
            item,
            "Canvas/ImagesPage",
            &format!("{label}Artwork"),
        )?;
        let center_x = item.position[0];
        if label == "Textures" {
            writeln!(
                text,
                "\n[node name=\"TexturesUnsupported\" type=\"Label\" parent=\"Canvas/ImagesPage\"]\nlayout_mode = 0\noffset_left = {}\noffset_top = 254.0\noffset_right = {}\noffset_bottom = 304.0\nhorizontal_alignment = 1\nvertical_alignment = 1\nautowrap_mode = 3\ntext = \"Unsupported: Godot has no source texture-swapping ratio.\"\ntooltip_text = \"The source control changes GLI texture swapping. This port does not substitute texture filtering.\"\nmetadata/source_slider = \"ImageSlider3\"\nmetadata/source_action = \"DisplayWithImage\"\nmetadata/source_support = \"unsupported: no native texture-swapping ratio\"\ntheme_override_font_sizes/font_size = 12",
                center_x - 74.0,
                center_x + 74.0,
            )?;
            continue;
        }
        let (setting, minimum, maximum) = match label {
            "Graphic" => ("shadow_quality", 1.0, 4.0),
            _ => ("screen_framing", 60.0, 100.0),
        };
        let tooltip = match label {
            "Graphic" => "Four source shadow levels map to Godot's native shadow filter quality.",
            "Screen" => {
                "The source uses a centered viewport crop. Godot scales the camera projection to match; this does not change window resolution."
            }
            _ => unreachable!(),
        };
        writeln!(
            text,
            "\n[node name={} type=\"VSlider\" parent=\"Canvas/ImagesPage\"]\nlayout_mode = 0\noffset_left = {}\noffset_top = 220.0\noffset_right = {}\noffset_bottom = 338.0\nmin_value = {}\nmax_value = {}\nstep = 1.0\nvalue = {}\ntooltip_text = {}\nmetadata/source_slider = {}\nmetadata/source_action = \"DisplayWithImage\"\nmetadata/native_setting = {}",
            quoted(label),
            center_x - 10.0,
            center_x + 10.0,
            minimum,
            maximum,
            maximum,
            quoted(tooltip),
            quoted(&format!("ImageSlider{}", index + 1)),
            quoted(setting),
        )?;
        writeln!(
            text,
            "\n[node name={} type=\"Label\" parent=\"Canvas/ImagesPage\"]\nlayout_mode = 0\noffset_left = {}\noffset_top = 341.0\noffset_right = {}\noffset_bottom = 359.0\nhorizontal_alignment = 1\ntext = {}\ntheme_override_font_sizes/font_size = 12",
            quoted(&format!("{label}Value")),
            center_x - 27.0,
            center_x + 27.0,
            quoted(&maximum.to_string()),
        )?;
    }
    writeln!(
        text,
        "\n[node name=\"ImageMappingNote\" type=\"Label\" parent=\"Canvas/ImagesPage\"]\nlayout_mode = 0\noffset_left = 192.0\noffset_top = 367.0\noffset_right = 615.0\noffset_bottom = 408.0\ntext = {}\nautowrap_mode = 3\nhorizontal_alignment = 1\ntheme_override_font_sizes/font_size = 11",
        quoted(
            "Screen framing matches the source centered camera crop. Textures is unsupported: its source texture-swapping ratio has no Godot equivalent."
        )
    )?;
    for (item, node) in stereo_rows.iter().zip(["StereoTest", "ReverseStereo"]) {
        append_configuration_label(
            &mut text,
            item,
            "Canvas/SoundPage",
            &format!("{node}Artwork"),
        )?;
        let [relative_x, relative_y, width, height] = item.rect;
        let x = item.position[0] + relative_x;
        let y = item.position[1] + relative_y;
        writeln!(
            text,
            "\n[node name={} type=\"TextureRect\" parent=\"Canvas/SoundPage\"]\nmaterial = SubResource(\"Additive\")\nlayout_mode = 0\noffset_left = {x}\noffset_top = {y}\noffset_right = {}\noffset_bottom = {}\nmouse_filter = 2\ntexture = ExtResource({})\nexpand_mode = 1\nstretch_mode = 0\nvisible = false",
            quoted(&format!("{node}Selected")),
            x + width,
            y + height,
            quoted(&format!("{}_selected", item.name)),
        )?;
        writeln!(
            text,
            "\n[node name={} type=\"Button\" parent=\"Canvas/SoundPage\"]\nlayout_mode = 0\noffset_left = {x}\noffset_top = {y}\noffset_right = {}\noffset_bottom = {}\nflat = true\ntext = \"\"\ntooltip_text = {}\nmetadata/source_action = {}",
            quoted(node),
            x + width,
            y + height,
            quoted(if node == "StereoTest" {
                "Play a left-channel then right-channel test tone."
            } else {
                "Toggle stereo channel reversal for stereo PCM WAV streams."
            }),
            quoted(if node == "StereoTest" {
                "TestButton"
            } else {
                "ReverseButton"
            }),
        )?;
    }
    text.push_str(
        "\n[node name=\"StereoStatus\" type=\"Label\" parent=\"Canvas/SoundPage\"]\nlayout_mode = 0\noffset_left = 370.0\noffset_top = 386.0\noffset_right = 620.0\noffset_bottom = 413.0\nautowrap_mode = 3\ntheme_override_font_sizes/font_size = 12",
    );
    writeln!(
        text,
        "\n[node name=\"Back\" type=\"Button\" parent=\"Canvas\"]\nlayout_mode = 0\noffset_left = 32.0\noffset_top = 424.0\noffset_right = 180.0\noffset_bottom = 464.0\ntext = \"Back\"\ntheme_override_colors/font_color = Color(1, 1, 1, 1)\ntheme_override_colors/font_focus_color = Color(1, 1, 1, 1)\ntheme_override_colors/font_hover_color = Color(1, 1, 1, 1)\ntheme_override_colors/font_pressed_color = Color(1, 1, 1, 1)\ntheme_override_font_sizes/font_size = 16\nfocus_neighbor_top = NodePath(\"../TabSound\")"
    )?;
    ensure!(
        load_steps == ext_count + 2,
        "configuration scene resource count differs"
    );
    Ok(text)
}

fn append_configuration_label(
    text: &mut String,
    item: &MenuItem,
    parent: &str,
    node: &str,
) -> Result<()> {
    let [relative_x, relative_y, width, height] = item.rect;
    let x = item.position[0] + relative_x;
    let y = item.position[1] + relative_y;
    writeln!(
        text,
        "\n[node name={} type=\"TextureRect\" parent={}]\nmaterial = SubResource(\"Additive\")\nlayout_mode = 0\noffset_left = {x}\noffset_top = {y}\noffset_right = {}\noffset_bottom = {}\nmouse_filter = 2\ntexture = ExtResource({})\nexpand_mode = 1\nstretch_mode = 0\nmetadata/source_action = {}",
        quoted(node),
        quoted(parent),
        x + width,
        y + height,
        quoted(&format!("{}_normal", item.name)),
        quoted(&item.action)
    )?;
    Ok(())
}

fn append_configuration_tab(
    text: &mut String,
    configuration: &MenuConfiguration,
    page: &str,
    item: &MenuItem,
) -> Result<()> {
    let [relative_x, relative_y, width, height] = item.rect;
    let x = item.position[0] + relative_x;
    let y = item.position[1] + relative_y;
    let node = format!("Tab{page}");
    writeln!(
        text,
        "\n[node name={} type=\"Button\" parent=\"Canvas\"]\nlayout_mode = 0\noffset_left = {x}\noffset_top = {y}\noffset_right = {}\noffset_bottom = {}\ntext = \"\"\nmetadata/source_action = {}\nmetadata/source_anchor = Vector2({}, {})",
        quoted(&node),
        x + width,
        y + height,
        quoted(&item.action),
        item.position[0],
        item.position[1]
    )?;
    for (index, direction) in ["top", "bottom", "left", "right"].into_iter().enumerate() {
        let target = item.neighbors[index]
            .as_ref()
            .and_then(|name| {
                configuration
                    .items
                    .iter()
                    .find(|candidate| &candidate.name == name)
            })
            .filter(|candidate| {
                matches!(candidate.action.as_str(), "Images" | "Controls" | "Sound")
            })
            .map_or_else(|| node.clone(), |target| format!("Tab{}", target.action));
        writeln!(
            text,
            "focus_neighbor_{direction} = NodePath({})",
            quoted(&format!("../{target}"))
        )?;
        if index < 2 {
            writeln!(
                text,
                "focus_{} = NodePath({})",
                if index == 0 { "previous" } else { "next" },
                quoted(&format!("../{target}"))
            )?;
        }
    }
    let parent = format!("Canvas/{node}");
    for (state, child_name, visible) in [
        ("normal", "Artwork", true),
        ("selected", "SelectedArtwork", false),
    ] {
        writeln!(
            text,
            "\n[node name={} type=\"TextureRect\" parent={}]\nmaterial = SubResource(\"Additive\")\nlayout_mode = 1\nanchors_preset = 15\nanchor_right = 1.0\nanchor_bottom = 1.0\ngrow_horizontal = 2\ngrow_vertical = 2\nmouse_filter = 2\ntexture = ExtResource({})\nexpand_mode = 1\nstretch_mode = 0\nvisible = {}",
            quoted(child_name),
            quoted(&parent),
            quoted(&format!("{}_{}", item.name, state)),
            visible
        )?;
    }
    Ok(())
}
/// Godot physical key and location (0 any, 1 left, 2 right) for a DirectInput
/// scancode. Physical keycodes name US-layout positions, as scancodes do.
fn godot_key(scancode: u16) -> Option<(i64, i64)> {
    const SPECIAL: i64 = 1 << 22;
    let letters = |row: &str, first: u16| {
        row.bytes()
            .enumerate()
            .find(|(i, _)| first + *i as u16 == scancode)
            .map(|(_, b)| (i64::from(b), 0))
    };
    if let Some(key) = letters("1234567890-=", 0x02)
        .or_else(|| letters("QWERTYUIOP[]", 0x10))
        .or_else(|| letters("ASDFGHJKL;'`", 0x1e))
        .or_else(|| letters("ZXCVBNM,./", 0x2c))
    {
        return Some(key);
    }
    let special = |n: i64| Some((SPECIAL + n, 0));
    match scancode {
        0x01 => special(1),              // escape
        0x0e => special(4),              // backspace
        0x0f => special(2),              // tab
        0x1c => special(5),              // enter
        0x9c => special(6),              // keypad enter
        0x1d => Some((SPECIAL + 22, 1)), // left ctrl
        0x9d => Some((SPECIAL + 22, 2)),
        0x2a => Some((SPECIAL + 21, 1)), // left shift
        0x36 => Some((SPECIAL + 21, 2)),
        0x38 => Some((SPECIAL + 24, 1)), // left alt
        0xb8 => Some((SPECIAL + 24, 2)),
        0x2b => Some((92, 0)),                                   // backslash
        0x39 => Some((32, 0)),                                   // space
        0x3a => special(25),                                     // caps lock
        0x3b..=0x44 => special(28 + i64::from(scancode - 0x3b)), // F1..F10
        0x57 => special(38),                                     // F11
        0x58 => special(39),                                     // F12
        0x45 => special(26),                                     // num lock
        0x46 => special(27),                                     // scroll lock
        0x37 => special(129),                                    // keypad *
        0xb5 => special(130),                                    // keypad /
        0x4a => special(131),                                    // keypad -
        0x53 => special(132),                                    // keypad .
        0x4e => special(133),                                    // keypad +
        0x52 => special(134),                                    // keypad 0
        0x4f => special(135),                                    // keypad 1
        0x50 => special(136),
        0x51 => special(137),
        0x4b => special(138),
        0x4c => special(139),
        0x4d => special(140),
        0x47 => special(141),
        0x48 => special(142),
        0x49 => special(143), // keypad 9
        0xd2 => special(7),   // insert
        0xd3 => special(8),   // delete
        0xc7 => special(13),  // home
        0xcf => special(14),  // end
        0xcb => special(15),  // left
        0xc8 => special(16),  // up
        0xcd => special(17),  // right
        0xd0 => special(18),  // down
        0xc9 => special(19),  // page up
        0xd1 => special(20),  // page down
        _ => None,
    }
}

/// Godot key identity used by UserSettings: keycode plus location << 32.
fn godot_key_id(scancode: u16) -> Option<i64> {
    godot_key(scancode).map(|(code, location)| code + (location << 32))
}

/// Original pad buttons keep their index as the Godot joypad button; pad
/// directions are the fixed D-pad buttons (JOY_BUTTON_DPAD_UP = 11 onward).
fn godot_input(input: &ControlInput) -> Option<(&'static str, i64)> {
    match input {
        ControlInput::Key(code) => godot_key_id(*code).map(|id| ("key", id)),
        ControlInput::PadButton(button) => Some(("pad", i64::from(*button))),
        ControlInput::PadDirection(direction) if *direction < 4 => {
            Some(("pad_direction", 11 + i64::from(*direction)))
        }
        ControlInput::PadDirection(_) => None,
    }
}

fn export_controls(
    project: &mut Project,
    source: &mut impl ContentSource,
    controls: &MenuControls,
) -> Result<()> {
    let directory = disk_path(&project.selection, &controls.path, "");
    let directory = directory.trim_end_matches('.').to_owned();
    for item in controls.items() {
        if item.normal.is_empty() {
            continue;
        }
        let states: &[(&str, &[MenuSprite])] = if controls.tabs.contains(item) {
            &[("normal", &item.normal), ("selected", &item.selected)]
        } else {
            &[("normal", &item.normal)]
        };
        for (state, sprites) in states {
            project.write(
                format!("{directory}/{}_{state}.png", item.name),
                &raster(item, sprites, source)?,
            )?;
        }
    }
    let background = source.load(&controls.background)?;
    project.save(&controls.background, background.as_ref())?;
    for picture in &controls.pictures {
        let texture = source.load(&picture.texture)?;
        project.save(&picture.texture, texture.as_ref())?;
    }
    let bindings_path = format!("{directory}/bindings.tres");
    project.write(
        bindings_path.clone(),
        controls_bindings(controls, &directory)?.as_bytes(),
    )?;
    project.write(
        format!("{directory}/menu.tscn"),
        controls_scene(project, controls, &directory, &bindings_path)?.as_bytes(),
    )?;
    Ok(())
}

fn rect2(item: &MenuItem) -> String {
    format!(
        "Rect2({}, {}, {}, {})",
        item.position[0] + item.rect[0],
        item.position[1] + item.rect[1],
        item.rect[2],
        item.rect[3]
    )
}

fn controls_bindings(controls: &MenuControls, directory: &str) -> Result<String> {
    let mut entries = Vec::new();
    for entry in &controls.entries {
        let (kind, default) = entry
            .default
            .as_ref()
            .and_then(godot_input)
            .unwrap_or(("none", 0));
        entries.push(format!(
            "{{\"name\": {}, \"kind\": {}, \"default\": {default}}}",
            quoted(&entry.name),
            quoted(kind)
        ));
    }
    let actions = controls
        .actions
        .iter()
        .map(|action| {
            format!(
                "{}: PackedStringArray({})",
                quoted(&action.name),
                action
                    .entries
                    .iter()
                    .map(|entry| quoted(entry))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect::<Vec<_>>();
    // Fixed joystick ranges as Vector3i(axis, min, max) on the source +-100 scale.
    let axes = controls
        .actions
        .iter()
        .filter(|action| !action.axes.is_empty())
        .map(|action| {
            format!(
                "{}: [{}]",
                quoted(&action.name),
                action
                    .axes
                    .iter()
                    .map(|axis| format!("Vector3i({}, {}, {})", axis.axis, axis.min, axis.max))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect::<Vec<_>>();
    let holds = controls
        .actions
        .iter()
        .filter(|action| !action.hold_axes.is_empty())
        .map(|action| {
            format!(
                "{}: [{}]",
                quoted(&action.name),
                action
                    .hold_axes
                    .iter()
                    .map(|axis| format!("Vector3i({}, {}, {})", axis.axis, axis.min, axis.max))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect::<Vec<_>>();
    let mut key_names = Vec::new();
    let mut pad_names = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for name in &controls.input_names {
        let Some((kind, id)) = godot_input(&name.input) else {
            continue;
        };
        // The first table record wins, as IPT_fn_szGetStringOf scans in order.
        if !seen.insert((kind, id)) {
            continue;
        }
        let value = format!(
            "{{\"label\": {}, \"texture\": {}, \"rect\": {}}}",
            quoted(&name.artwork.label),
            quoted(&format!(
                "res://{directory}/{}_normal.png",
                name.artwork.name
            )),
            rect2(&name.artwork)
        );
        if kind == "key" {
            key_names.push(format!("{id}: {value}"));
        } else {
            pad_names.push(format!("{id}: {value}"));
        }
    }
    ensure!(
        key_names.len() >= 80 && pad_names.len() == 32,
        "controls key-name mapping is incomplete"
    );
    Ok(format!(
        "[gd_resource type=\"Resource\" script_class=\"ControlBindings\" load_steps=2 format=3]\n\n[ext_resource type=\"Script\" path=\"res://game/control_bindings.gd\" id=\"script\"]\n\n[resource]\nscript = ExtResource(\"script\")\nentries = [{}]\nactions = {{\n{}\n}}\naxes = {{\n{}\n}}\nhold_axes = {{\n{}\n}}\nkey_names = {{\n{}\n}}\npad_names = {{\n{}\n}}\ncapture = {{\"label\": \"???\", \"texture\": {}, \"rect\": {}}}\n",
        entries.join(", "),
        actions.join(",\n"),
        axes.join(",\n"),
        holds.join(",\n"),
        key_names.join(",\n"),
        pad_names.join(",\n"),
        quoted(&format!(
            "res://{directory}/{}_normal.png",
            controls.capture.name
        )),
        rect2(&controls.capture)
    ))
}

fn controls_scene(
    project: &Project,
    controls: &MenuControls,
    directory: &str,
    bindings_path: &str,
) -> Result<String> {
    let mut resources = vec![
        (
            "Script".to_owned(),
            "res://game/controls_menu.gd".to_owned(),
            "script".to_owned(),
        ),
        (
            "Resource".to_owned(),
            format!("res://{bindings_path}"),
            "bindings".to_owned(),
        ),
        (
            "Texture2D".to_owned(),
            format!(
                "res://{}",
                disk_path(&project.selection, &controls.background.path, "png")
            ),
            "background".to_owned(),
        ),
    ];
    for picture in &controls.pictures {
        resources.push((
            "Texture2D".into(),
            format!(
                "res://{}",
                disk_path(&project.selection, &picture.texture.path, "png")
            ),
            picture.name.clone(),
        ));
    }
    for tab in &controls.tabs {
        for state in ["normal", "selected"] {
            resources.push((
                "Texture2D".into(),
                format!("res://{directory}/{}_{state}.png", tab.name),
                format!("{}_{state}", tab.name),
            ));
        }
    }
    for row in controls.keyboard.iter().chain(&controls.gamepad) {
        if !row.item.normal.is_empty() {
            resources.push((
                "Texture2D".into(),
                format!("res://{directory}/{}_normal.png", row.item.name),
                format!("{}_normal", row.item.name),
            ));
        }
    }
    let mut text = format!("[gd_scene load_steps={} format=3]\n\n", resources.len() + 2);
    for (kind, path, id) in &resources {
        writeln!(
            text,
            "[ext_resource type={} path={} id={}]",
            quoted(kind),
            quoted(path),
            quoted(id)
        )?;
    }
    let first_row = controls
        .keyboard
        .first()
        .context("controls keyboard list empty")?;
    writeln!(
        text,
        "\n[sub_resource type=\"CanvasItemMaterial\" id=\"Additive\"]\nblend_mode = 1\n\n[node name=\"ControlsMenu\" type=\"Control\"]\nlayout_mode = 3\nanchors_preset = 15\nanchor_right = 1.0\nanchor_bottom = 1.0\ngrow_horizontal = 2\ngrow_vertical = 2\nscript = ExtResource(\"script\")\nbindings = ExtResource(\"bindings\")\ncanvas_size = Vector2(640, 480)\nselect_offset = {}\nmetadata/source_hint = {}\n\n[node name=\"Canvas\" type=\"Control\" parent=\".\"]\nlayout_mode = 0\noffset_right = 640.0\noffset_bottom = 480.0\nmouse_filter = 2\n\n[node name=\"Background\" type=\"TextureRect\" parent=\"Canvas\"]\nlayout_mode = 0\noffset_right = 640.0\noffset_bottom = 480.0\nmouse_filter = 2\ntexture = ExtResource(\"background\")\nexpand_mode = 1\nstretch_mode = 0",
        controls.select_first_y - first_row.item.position[1],
        quoted(&controls.hint)
    )?;
    for picture in &controls.pictures {
        writeln!(
            text,
            "\n[node name={} type=\"TextureRect\" parent=\"Canvas\"]\nlayout_mode = 0\noffset_left = {}\noffset_top = {}\noffset_right = {}\noffset_bottom = {}\nmouse_filter = 2\ntexture = ExtResource({})\nexpand_mode = 1\nstretch_mode = 0\nvisible = {}",
            quoted(&picture.name),
            picture.center[0] - picture.size[0] * 0.5,
            picture.center[1] - picture.size[1] * 0.5,
            picture.center[0] + picture.size[0] * 0.5,
            picture.center[1] + picture.size[1] * 0.5,
            quoted(&picture.name),
            !matches!(picture.name.as_str(), "DisplayWithGamepad" | "SelectBox")
        )?;
    }
    for tab in &controls.tabs {
        let node = if tab.name.ends_with("keyboard") {
            "TabKeyboard"
        } else {
            "TabGamepad"
        };
        let [relative_x, relative_y, width, height] = tab.rect;
        let x = tab.position[0] + relative_x;
        let y = tab.position[1] + relative_y;
        writeln!(
            text,
            "\n[node name={} type=\"Control\" parent=\"Canvas\"]\nlayout_mode = 0\noffset_left = {x}\noffset_top = {y}\noffset_right = {}\noffset_bottom = {}\nmouse_filter = 2\nmetadata/source_action = {}",
            quoted(node),
            x + width,
            y + height,
            quoted(&tab.action)
        )?;
        for (state, child, visible) in [
            ("normal", "Artwork", true),
            ("selected", "SelectedArtwork", false),
        ] {
            writeln!(
                text,
                "\n[node name={} type=\"TextureRect\" parent={}]\nmaterial = SubResource(\"Additive\")\nlayout_mode = 1\nanchors_preset = 15\nanchor_right = 1.0\nanchor_bottom = 1.0\ngrow_horizontal = 2\ngrow_vertical = 2\nmouse_filter = 2\ntexture = ExtResource({})\nexpand_mode = 1\nstretch_mode = 0\nvisible = {visible}",
                quoted(child),
                quoted(&format!("Canvas/{node}")),
                quoted(&format!("{}_{state}", tab.name))
            )?;
        }
    }
    for (list, rows, visible) in [
        ("KeyboardList", &controls.keyboard, true),
        ("GamepadList", &controls.gamepad, false),
    ] {
        writeln!(
            text,
            "\n[node name={} type=\"Control\" parent=\"Canvas\"]\nlayout_mode = 0\noffset_right = 640.0\noffset_bottom = 480.0\nmouse_filter = 2\nvisible = {visible}",
            quoted(list)
        )?;
        for (index, row) in rows.iter().enumerate() {
            let parent = format!("Canvas/{list}");
            writeln!(
                text,
                "\n[node name=\"Row{index}\" type=\"Control\" parent={}]\nlayout_mode = 0\noffset_right = 640.0\noffset_bottom = 480.0\nmouse_filter = 2\nmetadata/source_action = {}\nmetadata/source_entry = {}\nmetadata/source_anchor = Vector2({}, {})",
                quoted(&parent),
                quoted(&row.item.action),
                quoted(row.entry.as_deref().unwrap_or("")),
                row.item.position[0],
                row.item.position[1]
            )?;
            let row_parent = format!("{parent}/Row{index}");
            if !row.item.normal.is_empty() {
                let [relative_x, relative_y, width, height] = row.item.rect;
                let x = row.item.position[0] + relative_x;
                let y = row.item.position[1] + relative_y;
                writeln!(
                    text,
                    "\n[node name=\"Label\" type=\"TextureRect\" parent={}]\nmaterial = SubResource(\"Additive\")\nlayout_mode = 0\noffset_left = {x}\noffset_top = {y}\noffset_right = {}\noffset_bottom = {}\nmouse_filter = 2\ntexture = ExtResource({})\nexpand_mode = 1\nstretch_mode = 0",
                    quoted(&row_parent),
                    x + width,
                    y + height,
                    quoted(&format!("{}_normal", row.item.name))
                )?;
            }
            if row.entry.is_some() {
                writeln!(
                    text,
                    "\n[node name=\"Input\" type=\"TextureRect\" parent={}]\nmaterial = SubResource(\"Additive\")\nlayout_mode = 0\nmouse_filter = 2\nexpand_mode = 1\nstretch_mode = 0",
                    quoted(&row_parent)
                )?;
            }
        }
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AssetRef, content::Collection};
    #[test]
    #[ignore = "local source-only menu import/Rete/native-text conversion; never starts an engine"]
    fn local_menu_source_and_rete_native_conversion() -> Result<()> {
        local_menu_conversion(false)
    }
    #[test]
    #[ignore = "local source-only pause import/Rete/native-text conversion; never starts an engine"]
    fn local_pause_source_and_rete_native_conversion() -> Result<()> {
        local_menu_conversion(true)
    }
    fn local_menu_conversion(pause: bool) -> Result<()> {
        use crate::model::{Selection, io::Load, menu::MenuPresentation};
        let game = std::env::var_os("ASTROLABE_GAME_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "disc".into());
        let temporary = tempfile::tempdir()?;
        let output = std::env::var_os(if pause {
            "ASTROLABE_PAUSE_OUTPUT"
        } else {
            "ASTROLABE_MENU_OUTPUT"
        })
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| temporary.path().join("menu"));
        ensure!(!output.exists(), "source evidence path must be fresh");
        let mut imported = if pause {
            crate::openspace::content::import_pause_menu(&game, Selection::hype("manoir", "en")?)?
        } else {
            crate::openspace::content::import_menu(&game, Selection::hype("manoir", "en")?)?
        };
        let owner = if pause { "pause-menu" } else { "menu" };
        let variant = if pause { "pause" } else { "main" };
        ensure!(
            imported.selection.level == owner,
            "menu must have independent publication ownership"
        );
        let reference = AssetRef::<MenuPresentation>::new(format!("fix:/menu/{variant}/en"))?;
        let menu = imported.load(&reference)?;
        if pause {
            ensure!(
                menu.background.is_none() && menu.initial_item == "Resume Game",
                "pause root/default differs"
            );
            ensure!(
                menu.items
                    .iter()
                    .map(|i| (i.action.as_str(), i.label.as_str()))
                    .collect::<Vec<_>>()
                    == vec![
                        ("Resume Game", "resume game"),
                        ("NextGame", "start a new adventure"),
                        ("Options", "configure"),
                        ("Main Menu", "main menu")
                    ],
                "installed pause actions/localization differ"
            );
            for (i, item) in menu.items.iter().enumerate() {
                ensure!(
                    (item.position[0] - 320.).abs() < 0.001
                        && (item.position[1] - (171.527 + i as f32 * 45.763)).abs() < 0.02,
                    "pause source depth/anchor differs"
                );
            }
        } else {
            let credits = menu.credits.as_ref().context("source credits absent")?;
            ensure!(
                credits.entries.len() == 400
                    && credits.displayed_entries == 397
                    && credits.max_visible_lines == 40
                    && credits.font.name == "Playmo_Font2_0"
                    && credits.entries[0].text == "Hype  -  The Time Quest"
                    && credits.entries[1].text.trim().is_empty()
                    && credits.entries[2].text.trim() == "Original Idea and Producer"
                    && credits.entries[3].text.trim() == "Alain Tascan"
                    && credits.entries[397].key == "CDT2_B_PROD40"
                    && credits.entries[399].key == "CDT2_B_PROD42",
                "English credits registry/count/font mapping differs"
            );
        }
        let mut store = crate::rete::Store::new(output.join("rete"), imported.selection.clone())?;
        for root in &imported.inventory.clone() {
            let asset = crate::model::content::Asset::load(&mut imported, root)?;
            store.save_asset(&root.reference, &asset)?;
        }
        let mut reloaded =
            crate::rete::Store::new(output.join("rete"), imported.selection.clone())?;
        let loaded = reloaded.load(&reference)?;
        ensure!(*menu == *loaded, "menu Rete persistence differs");
        let project_output = output.join("project");
        let mut project = Project::new(&project_output, imported.selection.clone())?;
        export(&mut project, &mut reloaded, &loaded)?;
        let scene = project
            .stage
            .path()
            .join(format!("generated/hype/shared/menu/{variant}/en/menu.tscn"));
        let text = std::fs::read_to_string(&scene)?;
        if pause {
            ensure!(
                text.matches(" type=\"Button\"").count() == 4
                    && text.contains("action = &\"NextGame\"")
                    && text.contains("initial_focus = NodePath(\"Canvas/ResumeGame\")")
                    && text.contains("focus_neighbor_top = NodePath(\"../MainMenu\")")
                    && text.contains("background_required = false")
                    && !text.contains("id=\"background\"")
                    && !text.contains("type=\"Texture2D\" path=\"res://generated/hype/texture/"),
                "native pause actions/overlay/nav differ"
            );
        } else {
            ensure!(
                text.matches(" type=\"Button\"").count() == 6
                    && text.contains("action = &\"SavedGames\"")
                    && text.contains("focus_neighbor_top = NodePath(\"../Quit\")"),
                "native menu layout/action/nav differs"
            );
            let configuration_scene = project
                .stage
                .path()
                .join("generated/hype/shared/menu/configuration/en/menu.tscn");
            let configuration_text = std::fs::read_to_string(configuration_scene)?;
            ensure!(
                configuration_text.matches(" type=\"VSlider\"").count() == 5
                    && configuration_text.matches(" type=\"Button\"").count() == 6
                    && configuration_text.contains("initial_page = &\"Images\"")
                    && configuration_text.contains("theme_override_font_sizes/font_size = 16")
                    && configuration_text.contains("metadata/source_slider = \"SoundSlider1\"")
                    && configuration_text.contains("metadata/source_slider = \"ImageSlider1\"")
                    && configuration_text.contains("metadata/source_slider = \"ImageSlider2\"")
                    && configuration_text.contains("metadata/source_slider = \"ImageSlider3\"")
                    && configuration_text.contains("metadata/native_setting = \"shadow_quality\"")
                    && configuration_text.contains("metadata/native_setting = \"screen_framing\"")
                    && configuration_text.contains("name=\"GraphicArtwork\" type=\"TextureRect\"")
                    && configuration_text.contains("name=\"ScreenArtwork\" type=\"TextureRect\"")
                    && configuration_text.contains("name=\"TexturesArtwork\" type=\"TextureRect\"")
                    && configuration_text.contains("name=\"TexturesUnsupported\" type=\"Label\"")
                    && configuration_text.contains("unsupported: no native texture-swapping ratio")
                    && configuration_text.contains("min_value = 1\nmax_value = 4\nstep = 1.0")
                    && configuration_text.contains("min_value = 60\nmax_value = 100\nstep = 1.0")
                    && configuration_text.matches("min_value = 0\nmax_value = 127\nstep = 1.0").count() == 2
                    && configuration_text.contains("min_value = -20\nmax_value = 107")
                    && configuration_text.contains("value = 100.0\nmetadata/source_slider = \"SoundSlider1\"")
                    && configuration_text.matches("value = 100.0\nmetadata/source_slider = \"SoundSlider").count() == 3
                    && !configuration_text.contains("[node name=\"Textures\" type=\"VSlider\"")
                    && configuration_text.contains("name=\"StereoTest\" type=\"Button\"")
                    && configuration_text.contains("name=\"ReverseStereo\" type=\"Button\"")
                    && configuration_text.contains("The source uses a centered viewport crop. Godot scales the camera projection to match")
                    && configuration_text.contains("source texture-swapping ratio has no Godot equivalent")
                    && !configuration_text.contains("UnsupportedImageSettings")
                    && configuration_text.contains("metadata/controls_scene = \"res://generated/hype/shared/menu/controls/en/menu.tscn\"")
                    && !configuration_text.contains("UnsupportedControls")
                    && configuration_text.contains("type=\"Texture2D\" path=\"res://generated/hype/textures/MENU/Config_back.gf.png\"")
                    && configuration_text.contains("metadata/source_action = \"DisplayWithSound\""),
                "native configuration labels/source sliders/backdrop differ"
            );
            let controls_directory = project
                .stage
                .path()
                .join("generated/hype/shared/menu/controls/en");
            let controls_text = std::fs::read_to_string(controls_directory.join("menu.tscn"))?;
            let bindings_text = std::fs::read_to_string(controls_directory.join("bindings.tres"))?;
            let controls_checks = [
                controls_text.matches("[node name=\"Row").count() == 26,
                controls_text.matches("[node name=\"Input\"").count() == 24,
                controls_text.contains("metadata/source_entry = \"Look_Key\""),
                controls_text.contains("metadata/source_entry = \"ModeLook_Pad\""),
                controls_text.contains("path=\"res://generated/hype/textures/FixTex/menus/controlsmenu/select.gf.png\""),
                controls_text.contains("Use the arrows to select the action"),
                bindings_text.contains("\"Button_Hit\": PackedStringArray(\"Action_Key\", \"Action_Pad\")"),
                bindings_text.contains("\"BT_Camera_Look\": PackedStringArray(\"Look_Key\", \"ModeLook_Pad\")"),
                bindings_text.contains("{\"name\": \"Look_Key\", \"kind\": \"key\", \"default\": 4194438}"),
                bindings_text.contains("{\"name\": \"Jump_Key\", \"kind\": \"key\", \"default\": 4299161622}"),
                bindings_text.contains("\"label\": \"pad0\""),
                bindings_text.contains("\"Button_SpeedUp\": [Vector3i(1, -110, -90)]"),
                bindings_text.contains("hold_axes = {\n\"Button_Up\": [Vector3i(1, -110, -20)]"),
                configuration_text.contains("[node name=\"Title0\" type=\"TextureRect\""),
            ];
            ensure!(
                controls_checks.iter().all(|passed| *passed),
                "native controls page/bindings differ: {controls_checks:?}"
            );
            let credits_scene = project
                .stage
                .path()
                .join("generated/hype/shared/menu/credits/en/lines.tscn");
            let credits_text = std::fs::read_to_string(credits_scene)?;
            let credits_checks = [
                credits_text.matches(" type=\"Label\"").count() == 397,
                credits_text.contains("metadata/source_registered_entries = 400"),
                credits_text.contains("metadata/source_displayed_entries = 397"),
                credits_text.contains("metadata/source_key = \"+CDT2_B_MGNT0\""),
                credits_text.contains("text = \"Original Idea and Producer  \""),
                credits_text.contains("horizontal_alignment = 2"),
                credits_text.contains("text = \"  Alain Tascan\""),
                credits_text.contains("horizontal_alignment = 0"),
                credits_text.contains("metadata/source_key = \"CDT2_B_PROD39c\""),
                !credits_text.contains("metadata/source_key = \"CDT2_B_PROD40\""),
                project
                    .generated
                    .iter()
                    .any(|path| path.ends_with(".credits.png")),
                project
                    .builder
                    .contains("ui_font_0.set_font_name(\"Playmo_Font2_0\")"),
            ];
            ensure!(
                credits_checks.into_iter().all(|check| check),
                "native credits scene/font/data differ"
            );
        }
        std::fs::create_dir_all(&project_output)?;
        for path in &project.generated {
            let source = project.stage.path().join(path);
            if source.is_file() {
                let target = project_output.join(path);
                std::fs::create_dir_all(target.parent().unwrap())?;
                std::fs::copy(source, target)?;
            }
        }
        std::fs::write(
            output.join("menu.json"),
            serde_json::to_vec_pretty(loaded.as_ref())?,
        )?;
        Ok(())
    }
    #[test]
    fn menu_raster_preserves_additive_overlap_and_bottom_left_uvs() -> Result<()> {
        use crate::model::texture::*;
        let reference = AssetRef::new("texture:/test.gf")?;
        let texture = Texture {
            usage: TextureUsage::Screen,
            colors: TextureColors::Components { bits: [8, 8, 8, 0] },
            levels: vec![TextureLevel {
                width: 2,
                height: 2,
                pixels: TexturePixels::Components(vec![
                    [20, 0, 0, 0],
                    [20, 0, 0, 0],
                    [0, 40, 0, 0],
                    [0, 40, 0, 0],
                ]),
            }],
            openspace_color_key: OpenSpaceColorKey {
                rgb: [0; 3],
                palette_index: -1,
            },
        };
        texture.validate()?;
        let mut source = Collection::default();
        source.insert(&reference, texture)?;
        let sprite = MenuSprite {
            texture: reference,
            rect: [0., 0., 1., 1.],
            uv: [[0., 0.], [1., 1.]],
        };
        let item = MenuItem {
            name: "test".into(),
            label: "test".into(),
            action: "Test".into(),
            position: [0., 0.],
            horizontal_alignment: 1,
            neighbors: [None, None, None, None],
            rect: [0., 0., 1., 1.],
            normal: vec![sprite.clone(), sprite],
            selected: vec![],
        };
        let bytes = raster(&item, &item.normal, &mut source)?;
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::IDENTITY);
        let mut reader = decoder.read_info()?;
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        reader.next_frame(&mut pixels)?;
        ensure!(
            pixels[..4] == [30, 20, 0, 255] && pixels[8..12] == [10, 60, 0, 255],
            "menu additive/UV projection differs: {pixels:?}"
        );
        Ok(())
    }
}
