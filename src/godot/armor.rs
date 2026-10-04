//! Armor catalog definitions become native editable resources, with shared state.
use super::{Project, disk_path, quoted};
use crate::model::{armor::*, content::ContentSource, io::Content};
use anyhow::Result;
use std::{collections::BTreeMap, fmt::Write as _};

pub(super) fn export(
    project: &mut Project,
    source: &mut impl ContentSource,
    value: &ArmorCatalog,
) -> Result<()> {
    value.validate()?;
    let mut artwork = BTreeMap::new();
    for image in value
        .definitions
        .iter()
        .flat_map(|v| v.artwork.iter().flatten())
    {
        let key = (image.texture.path.clone(), image.color_key);
        if artwork.contains_key(&key) {
            continue;
        }
        let texture = source.load(&image.texture)?;
        texture.validate_reference(&image.texture.path)?;
        let rgba = texture.rgba8(0)?;
        let rgba = if image.color_key {
            super::material::keyed_rgba(&texture, &rgba)
        } else {
            rgba
        };
        let path = disk_path(
            &project.selection,
            &image.texture.path,
            if image.color_key {
                "armor-keyed.png"
            } else {
                "armor.png"
            },
        );
        project.write(
            path.clone(),
            &super::material::png(texture.width(), texture.height(), &rgba)?,
        )?;
        artwork.insert(
            key,
            (format!("image{}", artwork.len()), format!("res://{path}")),
        );
    }
    let resources = value
        .definitions
        .iter()
        .map(|v| v.pieces.len() + 1)
        .sum::<usize>();
    let mut text = format!(
        "[gd_resource type=\"Resource\" load_steps={} format=3]\n",
        artwork.len() + resources + 4
    );
    for (id, script) in [
        ("catalog", "armor_catalog"),
        ("definition", "armor_definition"),
        ("piece", "armor_piece"),
    ] {
        writeln!(
            text,
            "[ext_resource type=\"Script\" path=\"res://addons/astrolabe/character/{script}.gd\" id=\"{id}\"]"
        )?;
    }
    for (id, path) in artwork.values() {
        writeln!(
            text,
            "[ext_resource type=\"Texture2D\" path={} id=\"{id}\"]",
            quoted(path)
        )?;
    }
    for (n, definition) in value.definitions.iter().enumerate() {
        for (p, piece) in definition.pieces.iter().enumerate() {
            let r = piece.rectangle;
            writeln!(
                text,
                "\n[sub_resource type=\"Resource\" id=\"piece{n}_{p}\"]\nresource_local_to_scene = true\nscript = ExtResource(\"piece\")\nsource_key = {}\nrectangle = Vector4({:?},{:?},{:?},{:?})\nbroken = {}\nstrikes = {}",
                quoted(&serde_json::to_string(&piece.source)?),
                r[0],
                r[1],
                r[2],
                r[3],
                piece.broken,
                piece.strikes
            )?;
        }
        writeln!(
            text,
            "\n[sub_resource type=\"Resource\" id=\"definition{n}\"]\nresource_local_to_scene = true\nscript = ExtResource(\"definition\")\nsource_key = {}",
            quoted(&serde_json::to_string(&definition.source)?)
        )?;
        for (index, name) in ["intact_artwork", "broken_artwork", "background_artwork"]
            .iter()
            .enumerate()
        {
            if let Some(image) = &definition.artwork[index] {
                writeln!(
                    text,
                    "{name} = ExtResource({})",
                    quoted(&artwork[&(image.texture.path.clone(), image.color_key)].0)
                )?;
            }
        }
        let sizes = definition
            .artwork
            .iter()
            .map(|a| {
                a.as_ref().map_or("Vector2i(0,0)".into(), |a| {
                    format!("Vector2i({},{})", a.size[0], a.size[1])
                })
            })
            .collect::<Vec<_>>()
            .join(",");
        let pieces = (0..definition.pieces.len())
            .map(|p| format!("SubResource(\"piece{n}_{p}\")"))
            .collect::<Vec<_>>()
            .join(",");
        let factors = definition
            .damage_factors
            .iter()
            .flatten()
            .map(|n| format!("{n:?}"))
            .collect::<Vec<_>>()
            .join(",");
        let thresholds = definition
            .break_thresholds
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(",");
        writeln!(
            text,
            "artwork_sizes = Array[Vector2i]([{sizes}])\npieces = Array[Resource]([{pieces}])\ndamage_factors = PackedFloat32Array({factors})\nbreak_thresholds = PackedByteArray({thresholds})\nminimum_damage = {}\nbroken_count = {}\nresistance_rank = {}\ncurrent_damage_piece = {}",
            definition.minimum_damage,
            definition.broken_count,
            definition.resistance_rank,
            definition.current_damage_piece.map_or(-1, i32::from)
        )?;
    }
    let definitions = (0..value.definitions.len())
        .map(|n| format!("SubResource(\"definition{n}\")"))
        .collect::<Vec<_>>()
        .join(",");
    writeln!(
        text,
        "\n[resource]\nresource_local_to_scene = true\nscript = ExtResource(\"catalog\")\nsource_path = {}\ndefinitions = Array[Resource]([{definitions}])\nselected = {}",
        quoted(value.path.as_str()),
        value.selected.map_or(-1, i32::from)
    )?;
    project.write(
        disk_path(&project.selection, &value.path, "tres"),
        text.as_bytes(),
    )
}
