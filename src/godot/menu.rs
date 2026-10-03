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
    for reference in [&value.navigation, &value.confirmation] {
        let sound = source.load(reference)?;
        project.save(reference, sound.as_ref())?;
    }
    let background = source.load(&value.background)?;
    project.save(&value.background, background.as_ref())?;
    project.write(
        format!("{directory}/menu.tscn"),
        scene(project, value, directory)?.as_bytes(),
    )?;
    project.diagnostic(format!("{}: English source layout/actions/navigation exported as editable native Controls; original normal/glow artwork baked ahead of runtime; music and submenu implementations remain separate",value.path));
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
        value.items.len() * 2 + 8
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
    writeln!(
        text,
        "[ext_resource type=\"Texture2D\" path={} id=\"background\"]",
        quoted(&format!(
            "res://{}",
            disk_path(&project.selection, &value.background.path, "png")
        ))
    )?;
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
    text.push_str("\n[sub_resource type=\"CanvasItemMaterial\" id=\"Additive\"]\nblend_mode = 1\n\n[node name=\"MainMenu\" type=\"Control\"]\nlayout_mode = 3\nanchors_preset = 15\nanchor_right = 1.0\nanchor_bottom = 1.0\ngrow_horizontal = 2\ngrow_vertical = 2\ntheme = ExtResource(\"theme\")\nscript = ExtResource(\"menu\")\n");
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
    text.push_str("\n[node name=\"Canvas\" type=\"Control\" parent=\".\"]\nlayout_mode = 0\noffset_right = 640.0\noffset_bottom = 480.0\nmouse_filter = 2\n\n[node name=\"Background\" type=\"TextureRect\" parent=\"Canvas\"]\nlayout_mode = 0\noffset_right = 640.0\noffset_bottom = 480.0\nmouse_filter = 2\ntexture = ExtResource(\"background\")\nexpand_mode = 1\nstretch_mode = 0\n");
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
            "\n[node name={} type=\"AudioStreamPlayer\" parent=\"Audio\"]\nstream = ExtResource({})\nvolume_db = {}",
            quoted(name),
            quoted(id),
            if gain > 0. { 20. * gain.log10() } else { -80. }
        )?;
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
        use crate::model::{Selection, io::Load, menu::MenuPresentation};
        let game = std::path::PathBuf::from(
            std::env::var_os("ASTROLABE_GAME_DIR").context("ASTROLABE_GAME_DIR required")?,
        );
        let output = std::path::PathBuf::from(
            std::env::var_os("ASTROLABE_MENU_OUTPUT").context("ASTROLABE_MENU_OUTPUT required")?,
        );
        ensure!(!output.exists(), "source evidence path must be fresh");
        let mut imported =
            crate::openspace::content::import_menu(&game, Selection::hype("manoir", "en")?)?;
        ensure!(
            imported.selection.level == "menu",
            "menu must have independent publication ownership"
        );
        let reference = AssetRef::<MenuPresentation>::new("fix:/menu/main/en")?;
        let menu = imported.load(&reference)?;
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
            .join("generated/hype/shared/menu/main/en/menu.tscn");
        let text = std::fs::read_to_string(&scene)?;
        ensure!(
            text.matches(" type=\"Button\"").count() == 6
                && text.contains("action = &\"SavedGames\"")
                && text.contains("focus_neighbor_top = NodePath(\"../Quit\")"),
            "native menu layout/action/nav differs"
        );
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
