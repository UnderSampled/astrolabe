//! Inventory values become native controls/resources before the game starts.
use super::{Project, disk_path, quoted};
use crate::model::{
    AssetRef,
    content::ContentSource,
    inventory::*,
    io::{Content, Save},
    texture::Texture,
};
use anyhow::{Result, ensure};
use std::{collections::BTreeMap, fmt::Write as _};
fn vector(v: [f32; 2]) -> String {
    format!("Vector2({:?},{:?})", v[0], v[1])
}
fn color(v: [f32; 4]) -> String {
    format!("Color({:?},{:?},{:?},{:?})", v[0], v[1], v[2], v[3])
}
fn motion_animation(name: &str, path: &str, offset: [f32; 2], motion: InventoryMotion) -> String {
    let duration = motion.duration_seconds();
    format!(
        "[gd_resource type=\"Animation\" format=3]\n\n[resource]\nresource_name = {}\nlength = {:?}\nloop_mode = 0\nstep = {:?}\ntracks/0/type = \"value\"\ntracks/0/path = NodePath({})\ntracks/0/interp = 1\ntracks/0/loop_wrap = false\ntracks/0/keys = {{\n\"times\": PackedFloat32Array(0,{:?}),\n\"transitions\": PackedFloat32Array(1,1),\n\"update\": 0,\n\"values\": [{},Vector2(0,0)]\n}}\n",
        quoted(name),
        duration,
        duration,
        quoted(path),
        duration,
        vector(offset),
    )
}
fn write_motion_animations(
    project: &mut Project,
    directory: &str,
    value: &InventoryPresentation,
) -> Result<String> {
    let path = format!("{directory}/motion");
    let animations = [
        (
            "category_up",
            "Canvas/Categories:position",
            [0.0, value.category_motion.distance],
            value.category_motion,
        ),
        (
            "category_down",
            "Canvas/Categories:position",
            [0.0, -value.category_motion.distance],
            value.category_motion,
        ),
        (
            "items_left",
            "Canvas/Slots:position",
            [-value.item_motion.distance, 0.0],
            value.item_motion,
        ),
        (
            "items_right",
            "Canvas/Slots:position",
            [value.item_motion.distance, 0.0],
            value.item_motion,
        ),
    ];
    let mut library =
        String::from("[gd_resource type=\"AnimationLibrary\" load_steps=5 format=3]\n\n");
    for (index, (name, track, offset, motion)) in animations.iter().enumerate() {
        let resource_path = format!("{path}/{name}.tres");
        project.write(
            resource_path.clone(),
            motion_animation(name, track, *offset, *motion).as_bytes(),
        )?;
        writeln!(
            library,
            "[ext_resource type=\"Animation\" path={} id=\"animation{index}\"]",
            quoted(&format!("res://{resource_path}"))
        )?;
    }
    library.push_str("\n[resource]\n_data = {\n");
    for (index, (name, _, _, _)) in animations.iter().enumerate() {
        writeln!(
            library,
            "{}: ExtResource(\"animation{index}\"),",
            quoted(name)
        )?;
    }
    library.push_str("}\n");
    let library_path = format!("{directory}/motion.animlib.tres");
    project.write(library_path.clone(), library.as_bytes())?;
    Ok(library_path)
}
fn image(
    project: &mut Project,
    source: &mut impl ContentSource,
    reference: &AssetRef<Texture>,
    key: bool,
) -> Result<String> {
    let texture = source.load(reference)?;
    project.retain_texture_mips(reference, &texture)?;
    let rgba = texture.rgba8(0)?;
    let rgba = if key {
        super::material::keyed_rgba(&texture, &rgba)
    } else {
        rgba
    };
    let path = disk_path(&project.selection, &reference.path, "inventory.png");
    project.write(
        path.clone(),
        &super::material::png(texture.width(), texture.height(), &rgba)?,
    )?;
    Ok(format!("res://{path}"))
}
pub(super) fn export(
    project: &mut Project,
    source: &mut impl ContentSource,
    value: &InventoryPresentation,
) -> Result<()> {
    value.validate()?;
    let directory = disk_path(&project.selection, &value.path, "");
    let directory = directory.trim_end_matches('.');
    let motion_library_path = write_motion_animations(project, directory, value)?;
    let mut images = BTreeMap::new();
    for layer in value
        .categories
        .iter()
        .flat_map(|c| &c.images)
        .chain(value.items.iter().flat_map(|i| &i.images))
    {
        if !images.contains_key(&layer.texture.path) {
            images.insert(
                layer.texture.path.clone(),
                image(project, source, &layer.texture, layer.color_key)?,
            );
        }
    }
    let atlas = image(
        project,
        source,
        &AssetRef::from_path(value.font.atlas.texture.clone()),
        value.font.atlas.color_key,
    )?;
    let font_path = format!("{directory}/counter.font.tres");
    project.reserve(&font_path)?;
    super::bitmap_font::emit(
        project,
        super::bitmap_font::FontInput {
            name: &value.font.name,
            atlas: &atlas,
            atlas_size: value.font.atlas.size,
            glyphs: &value.font.glyphs,
            tracking: value.font.tracking,
            scale: 640. / 9.,
            path: &format!("res://{font_path}"),
            outline: None,
        },
    )?;
    let backdrop = source.load(&value.background)?;
    project.save(&value.background, backdrop.as_ref())?;
    for item in &value.items {
        let mut text = format!(
            "[gd_resource type=\"Resource\" load_steps={} format=3]\n[ext_resource type=\"Script\" path=\"res://addons/astrolabe/ui/inventory/item.gd\" id=\"script\"]\n",
            item.images.len() + 2
        );
        for (n, layer) in item.images.iter().enumerate() {
            writeln!(
                text,
                "[ext_resource type=\"Texture2D\" path={} id=\"image{n}\"]",
                quoted(&images[&layer.texture.path])
            )?;
        }
        let refs = (0..item.images.len())
            .map(|n| format!("ExtResource(\"image{n}\")"))
            .collect::<Vec<_>>()
            .join(",");
        let sizes = item
            .images
            .iter()
            .map(|l| vector(l.size))
            .collect::<Vec<_>>()
            .join(",");
        writeln!(
            text,
            "\n[resource]\nscript = ExtResource(\"script\")\nitem_id = {}\nkind = {}\nlabel = {}\ncategory = &{}\ntextures = Array[Texture2D]([{}])\nsizes = Array[Vector2]([{}])\ndraw_counter = {}\ndraw_when_zero = {}\navailable = {}",
            item.id,
            quoted(&item.kind),
            quoted(&item.label),
            quoted(&item.category),
            refs,
            sizes,
            item.draw_counter,
            item.draw_when_zero,
            item.available
        )?;
        project.write(
            format!("{directory}/item_{}.tres", item.id),
            text.as_bytes(),
        )?;
    }
    let mut text = format!(
        "[gd_scene load_steps={} format=3]\n",
        value.items.len() + images.len() + 9
    );
    for (id, script) in [
        ("screen", "screen"),
        ("category", "category"),
        ("slot", "slot"),
    ] {
        writeln!(
            text,
            "[ext_resource type=\"Script\" path=\"res://addons/astrolabe/ui/inventory/{script}.gd\" id=\"{id}\"]"
        )?;
    }
    writeln!(
        text,
        "[ext_resource type=\"Texture2D\" path={} id=\"background\"]\n[ext_resource type=\"FontFile\" path={} id=\"font\"]\n[ext_resource type=\"Theme\" path=\"res://addons/astrolabe/ui/menu/theme.tres\" id=\"theme\"]\n[ext_resource type=\"AnimationLibrary\" path={} id=\"motion\"]",
        quoted(&format!(
            "res://{}",
            disk_path(&project.selection, &value.background.path, "png")
        )),
        quoted(&format!("res://{font_path}")),
        quoted(&format!("res://{motion_library_path}"))
    )?;
    for (n, path) in images.values().enumerate() {
        writeln!(
            text,
            "[ext_resource type=\"Texture2D\" path={} id=\"image{n}\"]",
            quoted(path)
        )?;
    }
    let refs_by_path: BTreeMap<_, _> = images
        .iter()
        .enumerate()
        .map(|(n, (path, _))| (path.clone(), format!("image{n}")))
        .collect();
    for item in &value.items {
        writeln!(
            text,
            "[ext_resource type=\"Resource\" path={} id=\"item{}\"]",
            quoted(&format!("res://{directory}/item_{}.tres", item.id)),
            item.id
        )?;
    }
    let item_refs = value
        .items
        .iter()
        .map(|i| format!("ExtResource(\"item{}\")", i.id))
        .collect::<Vec<_>>()
        .join(",");
    writeln!(
        text,
        "\n[node name=\"Inventory\" type=\"Control\"]\nlayout_mode = 3\nanchors_preset = 15\nanchor_right = 1.0\nanchor_bottom = 1.0\ngrow_horizontal = 2\ngrow_vertical = 2\nprocess_mode = 3\nscript = ExtResource(\"screen\")\ninitial_category = &{}\ncounter_font = ExtResource(\"font\")\nitems = Array[Resource]([{}])",
        quoted(&value.initial_category),
        item_refs
    )?;
    writeln!(
        text,
        "\n[node name=\"Canvas\" type=\"Control\" parent=\".\"]\noffset_right = 640.0\noffset_bottom = 480.0\nmouse_filter = 2\ntheme = ExtResource(\"theme\")\n\n[node name=\"Background\" type=\"TextureRect\" parent=\"Canvas\"]\noffset_right = 640.0\noffset_bottom = 480.0\ntexture = ExtResource(\"background\")\nexpand_mode = 1\nmouse_filter = 2\n\n[node name=\"Categories\" type=\"Control\" parent=\"Canvas\"]\nmouse_filter = 2"
    )?;
    for c in &value.categories {
        let parent = c
            .parent
            .as_ref()
            .map_or("Canvas/Categories".to_owned(), |p| {
                format!("Canvas/Categories/{p}/Artwork")
            });
        let pos = c.position;
        let half = if c.parent.is_some() { 32. } else { 64. };
        writeln!(
            text,
            "\n[node name={} type=\"Button\" parent={}]\noffset_left = {:?}\noffset_top = {:?}\noffset_right = {:?}\noffset_bottom = {:?}\nscript = ExtResource(\"category\")\ncategory = &{}\nparent_category = &{}\nnormal_scale = {:?}\nselected_scale = {:?}\nnormal_color = {}\nselected_color = {}\ntext = {}",
            quoted(&c.key),
            quoted(&parent),
            pos[0] - half,
            pos[1] - half,
            pos[0] + half,
            pos[1] + half,
            quoted(&c.key),
            quoted(c.parent.as_deref().unwrap_or("")),
            if c.parent.is_some() {
                1.
            } else {
                value.category_scale[0]
            },
            if c.parent.is_some() {
                1.
            } else {
                value.category_scale[1]
            },
            color(if c.parent.is_some() {
                [1.; 4]
            } else {
                value.category_color[0]
            }),
            color(if c.parent.is_some() {
                [1.; 4]
            } else {
                value.category_color[1]
            }),
            quoted(&c.key)
        )?;
        for (n, side) in ["top", "bottom", "left", "right"].iter().enumerate() {
            if let Some(next) = &c.neighbors[n] {
                let target = &value.categories.iter().find(|c| &c.key == next).unwrap();
                let path = if c.parent == target.parent {
                    format!("../{next}")
                } else {
                    format!("../../{next}")
                };
                writeln!(text, "focus_neighbor_{side} = NodePath({})", quoted(&path))?;
            }
        }
        writeln!(
            text,
            "\n[node name=\"Artwork\" type=\"Control\" parent={}]\noffset_left = {:?}\noffset_top = {:?}\nmouse_filter = 2",
            quoted(&format!("{parent}/{}", c.key)),
            half,
            half
        )?;
        for (n, l) in c.images.iter().enumerate() {
            writeln!(
                text,
                "\n[node name=\"Layer{n}\" type=\"TextureRect\" parent={}]\noffset_left = {:?}\noffset_top = {:?}\noffset_right = {:?}\noffset_bottom = {:?}\ntexture = ExtResource({})\nexpand_mode = 1\nmouse_filter = 2",
                quoted(&format!("{parent}/{}/Artwork", c.key)),
                -l.size[0] * 0.5,
                -l.size[1] * 0.5,
                l.size[0] * 0.5,
                l.size[1] * 0.5,
                quoted(&refs_by_path[&l.texture.path])
            )?;
        }
    }
    writeln!(
        text,
        "\n[node name=\"Slots\" type=\"Control\" parent=\"Canvas\"]\nmouse_filter = 2"
    )?;
    let layers = value.items.iter().map(|i| i.images.len()).max().unwrap();
    ensure!(layers <= 16, "inventory layer bound");
    for (n, s) in value.slots.iter().enumerate() {
        writeln!(
            text,
            "\n[node name=\"Slot{n}\" type=\"Button\" parent=\"Canvas/Slots\"]\noffset_left = {:?}\noffset_top = {:?}\noffset_right = {:?}\noffset_bottom = {:?}\nscript = ExtResource(\"slot\")\nnormal_scale = {:?}\nequipped_scale = {:?}\ncurrent_scale = {:?}\nnormal_color = {}\ncurrent_color = {}\nfocus_neighbor_top = NodePath(\"../../Categories/{}\")\nfocus_neighbor_bottom = NodePath(\"../../Categories/{}\")",
            s.position[0] - 32.,
            s.position[1] - 32.,
            s.position[0] + 32.,
            s.position[1] + 32.,
            value.item_scale[0],
            value.item_scale[1],
            value.item_scale[2],
            color(value.item_color[0]),
            color(value.item_color[1]),
            value.initial_category,
            value.initial_category
        )?;
        for (i, side) in ["top", "bottom", "left", "right"].iter().enumerate() {
            if let Some(next) = s.neighbors[i] {
                writeln!(text, "focus_neighbor_{side} = NodePath(\"../Slot{next}\")")?;
            }
        }
        writeln!(
            text,
            "\n[node name=\"Artwork\" type=\"Control\" parent=\"Canvas/Slots/Slot{n}\"]\noffset_left = 32.0\noffset_top = 32.0\nmouse_filter = 2"
        )?;
        for i in 0..layers {
            writeln!(
                text,
                "\n[node name=\"Layer{i}\" type=\"TextureRect\" parent=\"Canvas/Slots/Slot{n}/Artwork\"]\nvisible = false\nmouse_filter = 2\nexpand_mode = 1"
            )?;
        }
        writeln!(
            text,
            "\n[node name=\"Counter\" type=\"Label\" parent=\"Canvas/Slots/Slot{n}\"]\noffset_left = {:?}\noffset_top = {:?}\noffset_right = {:?}\noffset_bottom = {:?}\nmouse_filter = 2\ntheme_override_fonts/font = ExtResource(\"font\")\ntheme_override_font_sizes/font_size = 21\ntheme_override_colors/font_color = Color(1,1,1,1)\nhorizontal_alignment = 1",
            value.counter_offset[0],
            value.counter_offset[1] + 14.,
            64. + value.counter_offset[0],
            42. + value.counter_offset[1]
        )?;
    }
    writeln!(
        text,
        "\n[node name=\"Detail\" type=\"Control\" parent=\"Canvas\"]\noffset_left = {:?}\noffset_top = {:?}\nscale = {}\nmouse_filter = 2\n\n[node name=\"Description\" type=\"Label\" parent=\"Canvas\"]\noffset_left = {:?}\noffset_top = {:?}\noffset_right = {:?}\noffset_bottom = {:?}\nmouse_filter = 2\ntheme_override_fonts/font = ExtResource(\"font\")\ntheme_override_font_sizes/font_size = 21\ntheme_override_colors/font_color = Color(1,1,1,1)\nhorizontal_alignment = 1\nautowrap_mode = 3",
        value.detail_position[0],
        value.detail_position[1],
        vector([value.detail_scale; 2]),
        value.description_position[0] - 170.,
        value.description_position[1] - 18.,
        value.description_position[0] + 170.,
        value.description_position[1] + 100.
    )?;
    for index in 0..layers {
        writeln!(
            text,
            "\n[node name=\"Layer{index}\" type=\"TextureRect\" parent=\"Canvas/Detail\"]\nvisible = false\nmouse_filter = 2\nexpand_mode = 1"
        )?;
    }
    // The app owns unavailable-action status. This generated label is an editable seam.
    writeln!(
        text,
        "\n[node name=\"Status\" type=\"Label\" parent=\"Canvas\"]\nvisible = false\noffset_left = 180.0\noffset_top = 420.0\noffset_right = 620.0\noffset_bottom = 470.0\ntheme_override_fonts/font = ExtResource(\"font\")\ntheme_override_font_sizes/font_size = 21\ntheme_override_colors/font_color = Color(1,1,1,1)\nautowrap_mode = 3\n\n[node name=\"AnimationPlayer\" type=\"AnimationPlayer\" parent=\".\"]\nroot_node = NodePath(\"..\")\nlibraries = {{\n\"\": ExtResource(\"motion\")\n}}"
    )?;
    project.write(format!("{directory}/inventory.tscn"), text.as_bytes())?;
    project.diagnostic(format!("{}: native inventory presentation with original category/slot layout, icons/font and localized labels; consume/magic effects remain separate",value.path));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Selection, io::Load};
    use anyhow::Context;
    #[test]
    #[ignore = "requires local Hype data and Godot; runs the source-to-Rete-to-native inventory UI check"]
    fn local_inventory_source_and_rete_native_conversion() -> Result<()> {
        let game = std::env::var_os("ASTROLABE_GAME_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("disc"));
        let scratch = tempfile::tempdir()?;
        let output = std::env::var_os("ASTROLABE_INVENTORY_OUTPUT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("ASTROLABE_EVIDENCE_DIR").map_or_else(
                    || scratch.path().join("inventory-source"),
                    |p| std::path::PathBuf::from(p).join("inventory-source"),
                )
            });
        ensure!(!output.exists(), "source evidence path must be fresh");
        let godot = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
        let syntax = scratch.path().join("syntax");
        super::super::support::initialize(&syntax)?;
        let harness = include_str!("../../godot/tests/inventory_scene_runtime.gd");
        std::fs::write(syntax.join("inventory_runtime_validation.gd"), harness)?;
        let parser_log = syntax.join("parser.log");
        crate::godot::run_godot(
            &godot,
            &syntax,
            &[
                "--headless",
                "--log-file",
                parser_log.to_str().context("parser log path")?,
                "--check-only",
                "--script",
                "res://inventory_runtime_validation.gd",
            ],
            std::time::Duration::from_secs(10),
        )?;
        let parser_diagnostics = std::fs::read_to_string(parser_log)?;
        ensure!(
            !parser_diagnostics.contains("ERROR:"),
            "inventory syntax check: {parser_diagnostics}"
        );
        let mut imported =
            crate::openspace::content::import_inventory(&game, Selection::hype("manoir", "en")?)?;
        ensure!(
            imported.selection.level == "inventory",
            "independent inventory publication required"
        );
        let reference = AssetRef::<InventoryPresentation>::new("fix:/inventory/en")?;
        let original = imported.load(&reference)?;
        ensure!(
            original.categories.len() == 8
                && original.slots.len() == 6
                && original.items.len() == 46,
            "source catalogue/layout differs"
        );
        ensure!(
            original.initial_category == "category_1"
                && original.slots[0].position == [241., 80.]
                && original.slots[5].position == [546., 80.],
            "source initial category or row geometry differs"
        );
        ensure!(
            original.category_scale == [0.5, 0.85] && original.item_scale == [0.75, 0.75, 1.0],
            "source selection scales differ"
        );
        ensure!(
            original.category_motion
                == InventoryMotion {
                    distance: 96.0,
                    speed_per_millisecond: 0.3,
                }
                && original.item_motion
                    == InventoryMotion {
                        distance: 50.0,
                        speed_per_millisecond: 0.7,
                    }
                && (original.category_motion.duration_seconds() - 0.320).abs() < 0.00001
                && (original.item_motion.duration_seconds() - (50.0 / 0.7 / 1000.0)).abs()
                    < 0.00001,
            "source inventory transition values differ"
        );
        ensure!(
            original.text_position == [480.0, 375.0] && original.stats_position == [64.0, 64.0],
            "source inventory text/stats origins differ"
        );
        for (id, label) in [(80, "Crossbow bolts"), (81, "Modern Crossbow bolts")] {
            let item = original
                .items
                .iter()
                .find(|item| item.id == id)
                .context("source weapon absent")?;
            ensure!(
                item.label == label && item.kind == "Weapon" && item.category == "category_4",
                "localized weapon identity differs"
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
        ensure!(*original == *loaded, "inventory Rete persistence differs");
        let mut project = Project::new(output.join("project"), imported.selection.clone())?;
        export(&mut project, &mut reloaded, &loaded)?;
        let scene = std::fs::read_to_string(
            project
                .stage
                .path()
                .join("generated/hype/shared/inventory/en/inventory.tscn"),
        )?;
        ensure!(
            scene.matches(" type=\"Button\"").count() == 14
                && scene.contains("focus_neighbor_right = NodePath(\"../Slot1\")"),
            "native category/slot controls or navigation absent"
        );
        ensure!(
            scene.contains("type=\"AnimationLibrary\"")
                && scene.contains("[node name=\"AnimationPlayer\"")
                && scene.contains("motion.animlib.tres"),
            "native source-motion AnimationPlayer is absent"
        );
        let generated = project
            .stage
            .path()
            .join("generated/hype/shared/inventory/en/motion");
        let category_up = std::fs::read_to_string(generated.join("category_up.tres"))?;
        let item_left = std::fs::read_to_string(generated.join("items_left.tres"))?;
        ensure!(
            category_up.contains("length = 0.32")
                && category_up.contains("PackedFloat32Array(0,0.32)")
                && category_up.contains("Vector2(0.0,96.0)")
                && category_up.contains("Canvas/Categories:position")
                && item_left.contains("length = 0.071428575")
                && item_left.contains("Vector2(-50.0,0.0)"),
            "native animations do not preserve source distance/rate/timing"
        );
        ensure!(
            project.builder.find("ui_font_0.fixed_size = 21")
                < project.builder.find("ui_font_0.set_font_name"),
            "font cache setup order regressed"
        );
        ensure!(
            project.builder.contains("set_glyph_uv_rect")
                && project.builder.contains("ui_font_0.set_glyph_uv_rect")
                && project
                    .builder
                    .contains("res://generated/hype/shared/inventory/en/counter.font.tres"),
            "native counter font builder absent"
        );
        let manifest = project.finish(&godot)?;
        std::fs::write(
            output.join("inventory.json"),
            serde_json::to_vec_pretty(loaded.as_ref())?,
        )?;
        std::fs::write(
            output.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        let runtime_check = include_str!("../../godot/tests/inventory_scene_runtime.gd");
        let project_path = output.join("project");
        std::fs::write(
            project_path.join("inventory_runtime_validation.gd"),
            runtime_check,
        )?;
        let runtime_log = project_path.join("inventory-runtime.log");
        let evidence = crate::godot::run_godot(
            &godot,
            &project_path,
            &[
                "--headless",
                "--log-file",
                runtime_log.to_str().context("runtime log path")?,
                "--script",
                "res://inventory_runtime_validation.gd",
            ],
            std::time::Duration::from_secs(45),
        )?;
        let runtime_diagnostics = std::fs::read_to_string(runtime_log)?;
        ensure!(
            !runtime_diagnostics.contains("ERROR:"),
            "inventory runtime: {runtime_diagnostics}"
        );
        ensure!(
            evidence.contains("INVENTORY_UI_OK"),
            "Godot inventory runtime evidence is missing: {evidence}"
        );
        Ok(())
    }
}
