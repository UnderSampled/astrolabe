//! Bake menu sprite quads into ordinary textures before the authored game runs.
use super::{Project, disk_path, quoted};
use crate::model::{content::ContentSource, io::Save, menu::*};
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
    project.diagnostic(format!("{}: English source layout/actions/navigation exported as editable native Controls; original normal/glow artwork baked ahead of runtime; the source-backed sound configuration page is exported with native sliders",value.path));
    Ok(())
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
    let sound_tab = configuration
        .items
        .iter()
        .find(|item| item.action == "Sound" && item.label == "sound")
        .context("source Sound tab absent")?;
    let rows = ["music", "sound", "voices"].map(|label| {
        configuration
            .items
            .iter()
            .find(|item| item.action == "DisplayWithSound" && item.label == label)
            .with_context(|| format!("source {label} volume label absent"))
    });
    let rows = rows.into_iter().collect::<Result<Vec<_>>>()?;
    let mut text = String::from("[gd_scene load_steps=11 format=3]\n\n");
    for (id, kind, path) in [
        ("settings", "Script", "game/settings_menu.gd"),
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
    writeln!(
        text,
        "[ext_resource type=\"Texture2D\" path={} id=\"background\"]",
        quoted(&format!(
            "res://{}",
            disk_path(&project.selection, &configuration.background.path, "png")
        ))
    )?;
    for (item, state) in std::iter::once((sound_tab, "normal"))
        .chain(std::iter::once((sound_tab, "selected")))
        .chain(rows.iter().map(|item| (*item, "normal")))
    {
        writeln!(
            text,
            "[ext_resource type=\"Texture2D\" path={} id={}]",
            quoted(&format!("res://{directory}/{}_{state}.png", item.name)),
            quoted(&format!("{}_{}", item.name, state))
        )?;
    }
    let load_steps = 4 + 2 + rows.len() + 1 + 1;
    writeln!(
        text,
        "\n[sub_resource type=\"CanvasItemMaterial\" id=\"Additive\"]\nblend_mode = 1\n\n[node name=\"ConfigurationMenu\" type=\"Control\"]\nlayout_mode = 3\nanchors_preset = 15\nanchor_right = 1.0\nanchor_bottom = 1.0\ngrow_horizontal = 2\ngrow_vertical = 2\ntheme = ExtResource(\"theme\")\nscript = ExtResource(\"settings\")\ncanvas_size = Vector2(640, 480)\ninitial_focus = NodePath(\"Canvas/Audio/Music\")\nmetadata/source_initial_item = {}\n\n[node name=\"Canvas\" type=\"Control\" parent=\".\"]\nlayout_mode = 0\noffset_right = 640.0\noffset_bottom = 480.0\nmouse_filter = 2\n\n[node name=\"Background\" type=\"TextureRect\" parent=\"Canvas\"]\nlayout_mode = 0\noffset_right = 640.0\noffset_bottom = 480.0\nmouse_filter = 2\ntexture = ExtResource(\"background\")\nexpand_mode = 1\nstretch_mode = 0",
        quoted(&configuration.initial_item)
    )?;
    let [x, y, w, h] = sound_tab.rect;
    let left = sound_tab.position[0] + x;
    let top = sound_tab.position[1] + y;
    writeln!(
        text,
        "\n[node name=\"Sound\" type=\"Button\" parent=\"Canvas\"]\nlayout_mode = 0\noffset_left = {left}\noffset_top = {top}\noffset_right = {}\noffset_bottom = {}\ntext = \"\"\nscript = ExtResource(\"button\")\naction = &\"Sound\"\nnormal_texture = ExtResource({})\nselected_texture = ExtResource({})\nmetadata/source_action = \"Sound\"",
        left + w,
        top + h,
        quoted(&format!("{}_normal", sound_tab.name)),
        quoted(&format!("{}_selected", sound_tab.name))
    )?;
    writeln!(
        text,
        "\n[node name=\"Artwork\" type=\"TextureRect\" parent=\"Canvas/Sound\"]\nmaterial = SubResource(\"Additive\")\nlayout_mode = 1\nanchors_preset = 15\nanchor_right = 1.0\nanchor_bottom = 1.0\ngrow_horizontal = 2\ngrow_vertical = 2\nmouse_filter = 2\ntexture = ExtResource({})\nexpand_mode = 1\nstretch_mode = 0",
        quoted(&format!("{}_normal", sound_tab.name))
    )?;
    writeln!(
        text,
        "\n[node name=\"Audio\" type=\"Control\" parent=\"Canvas\"]\nlayout_mode = 0\nanchors_preset = 15\nanchor_right = 1.0\nanchor_bottom = 1.0\noffset_right = 640.0\noffset_bottom = 480.0\nmouse_filter = 2"
    )?;
    for (index, (item, bus)) in rows.iter().zip(["Music", "Effects", "Voices"]).enumerate() {
        let id = format!("{}_normal", item.name);
        let x = item.position[0] + item.rect[0];
        let y = item.position[1] + item.rect[1];
        writeln!(
            text,
            "\n[node name={} type=\"TextureRect\" parent=\"Canvas\"]\nmaterial = SubResource(\"Additive\")\nlayout_mode = 0\noffset_left = {x}\noffset_top = {y}\noffset_right = {}\noffset_bottom = {}\nmouse_filter = 2\ntexture = ExtResource({})\nexpand_mode = 1\nstretch_mode = 0\nmetadata/source_action = \"DisplayWithSound\"\nmetadata/volume_bus = {}",
            quoted(&item.name),
            x + item.rect[2],
            y + item.rect[3],
            quoted(&id),
            quoted(bus)
        )?;
        // The original background places each source label above a vertical slot.
        // FUN_00413bd0/FUN_00412030 map 0..127 from the slot bottom to its top.
        let center_x = item.position[0];
        writeln!(
            text,
            "\n[node name={} type=\"VSlider\" parent=\"Canvas/Audio\"]\nlayout_mode = 0\noffset_left = {}\noffset_top = 220.0\noffset_right = {}\noffset_bottom = 338.0\nmin_value = 0.0\nmax_value = 127.0\nstep = 1.0\nvalue = 127.0\nmetadata/source_slider = \"Slider{}\"\nmetadata/source_position = Vector3({}, {}, {})\nmetadata/source_action = \"DisplayWithSound\"\nmetadata/volume_bus = {}",
            quoted(bus),
            center_x - 10.0,
            center_x + 10.0,
            index + 1,
            configuration.slider_positions[index][0],
            configuration.slider_positions[index][1],
            configuration.slider_positions[index][2],
            quoted(bus)
        )?;
    }
    writeln!(
        text,
        "\n[node name=\"Back\" type=\"Button\" parent=\"Canvas\"]\nlayout_mode = 0\noffset_left = 32.0\noffset_top = 424.0\noffset_right = 180.0\noffset_bottom = 464.0\ntext = \"Back\"\nfocus_neighbor_top = NodePath(\"../Audio/Music\")"
    )?;
    if load_steps != 11 {
        anyhow::bail!("configuration scene resource count differs");
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
                configuration_text.matches(" type=\"VSlider\"").count() == 3
                    && configuration_text.contains("metadata/source_slider = \"Slider1\"")
                    && configuration_text.contains("type=\"Texture2D\" path=\"res://generated/hype/textures/MENU/Config_back.gf.png\"")
                    && configuration_text.contains("metadata/source_action = \"DisplayWithSound\""),
                "native configuration labels/source sliders/backdrop differ"
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
