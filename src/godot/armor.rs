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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AssetPath, AssetRef,
        ai::{SourceObjectIdentity, SourceRole},
        content::Collection,
        texture::{
            OpenSpaceColorKey, Texture, TextureColors, TextureLevel, TexturePixels, TextureUsage,
        },
    };

    #[test]
    fn armor_catalog_writes_native_resource_graph_and_png_artwork() -> Result<()> {
        let texture_ref = AssetRef::new("texture:/armor/catalog-test")?;
        let source = |allocation| SourceObjectIdentity {
            role: SourceRole::Fix,
            block: [5, 0],
            allocation,
        };
        let catalog = ArmorCatalog {
            path: AssetPath::new("fix:/gameplay/armor")?,
            definitions: vec![ArmorDefinition {
                source: source(0),
                artwork: [
                    Some(ArmorArtwork {
                        texture: texture_ref.clone(),
                        size: [2, 2],
                        color_key: false,
                    }),
                    Some(ArmorArtwork {
                        texture: texture_ref.clone(),
                        size: [2, 2],
                        color_key: false,
                    }),
                    None,
                ],
                pieces: vec![ArmorPiece {
                    source: source(1),
                    rectangle: [0.0, 0.0, 1.0, 1.0],
                    broken: 0,
                    strikes: 0,
                    unknown: [0; 2],
                }],
                damage_factors: vec![vec![1.0, 0.5]; 5],
                break_thresholds: [1; 5],
                minimum_damage: 1,
                broken_count: 0,
                resistance_rank: 0,
                current_damage_piece: None,
                unknown: [0; 3],
            }],
            selected: Some(0),
        };
        let texture = Texture {
            usage: TextureUsage::Material,
            colors: TextureColors::Components { bits: [8, 8, 8, 8] },
            levels: vec![TextureLevel {
                width: 2,
                height: 2,
                pixels: TexturePixels::Components(vec![
                    [240, 220, 160, 255],
                    [180, 130, 80, 255],
                    [90, 60, 40, 255],
                    [30, 20, 10, 255],
                ]),
            }],
            openspace_color_key: OpenSpaceColorKey {
                rgb: [0, 0, 0],
                palette_index: -1,
            },
        };
        let mut content = Collection::default();
        content.insert(&AssetRef::from_path(catalog.path.clone()), catalog.clone())?;
        content.insert(&texture_ref, texture)?;

        let output = tempfile::tempdir()?;
        let mut project = Project::new(
            output.path().join("godot"),
            crate::model::Selection::hype("armor-test", "en")?,
        )?;
        export(&mut project, &mut content, &catalog)?;
        let resource = std::fs::read_to_string(
            project
                .stage
                .path()
                .join("generated/hype/shared/gameplay/armor.tres"),
        )?;
        assert!(resource.contains("script = ExtResource(\"catalog\")"));
        assert!(resource.contains("script = ExtResource(\"definition\")"));
        assert!(resource.contains("script = ExtResource(\"piece\")"));
        assert!(resource.contains("selected = 0"));
        let png = std::fs::read(
            project
                .stage
                .path()
                .join("generated/hype/textures/armor/catalog-test.armor.png"),
        )?;
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        Ok(())
    }
}
