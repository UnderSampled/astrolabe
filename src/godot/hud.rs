//! Editable Godot projection of Fix's source-authored gameplay HUD.
use super::{Project, disk_path, quoted};
use crate::model::{
    ai::SourceObjectIdentity, content::ContentSource, hud::HudPresentation, io::Content,
};
use anyhow::Result;
use std::fmt::Write as _;

const HUD_TEXTURE_GLOBALS: [&str; 6] = [
    "DAT_0060f62c",
    "DAT_0060f630",
    "DAT_0060f620",
    "DAT_0060f624",
    "DAT_0060f628",
    "DAT_0060f63c",
];

fn float(value: f32) -> String {
    format!("{value:?}")
}

fn source_identity_text(identity: &SourceObjectIdentity) -> String {
    format!(
        "{:?}:{:02x}:{:02x}:{}",
        identity.role, identity.block[0], identity.block[1], identity.allocation
    )
}

fn float_array(values: impl IntoIterator<Item = f32>) -> String {
    format!(
        "PackedFloat32Array({})",
        values.into_iter().map(float).collect::<Vec<_>>().join(", ")
    )
}

fn string_array(values: impl IntoIterator<Item = String>) -> String {
    format!(
        "PackedStringArray({})",
        values
            .into_iter()
            .map(|value| quoted(&value))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(super) fn export(
    project: &mut Project,
    _source: &mut impl ContentSource,
    value: &HudPresentation,
    _written: &mut std::collections::BTreeSet<crate::model::AssetPath>,
) -> Result<()> {
    value.validate()?;
    let scene_path = disk_path(&project.selection, &value.path, "tscn");
    let mut text = String::new();
    text.push_str(
        "[gd_scene load_steps=2 format=3]\n\n[ext_resource type=\"Script\" path=\"res://addons/astrolabe/ui/hud.gd\" id=\"hud_script\"]\n\n",
    );
    writeln!(
        text,
        "[node name=\"SourceHud\" type=\"Control\"]\nlayout_mode = 3\nanchors_preset = 15\nanchor_right = 1.0\nanchor_bottom = 1.0\ngrow_horizontal = 2\ngrow_vertical = 2\nmouse_filter = 2\nscript = ExtResource(\"hud_script\")\nmetadata/source_canvas_size = Vector2i({}, {})\nmetadata/source_words = PackedInt64Array({}, {})\nmetadata/source_text_offsets = {}\nmetadata/source_texture_handles = {}\nmetadata/source_texture_globals = {}",
        value.canvas_size[0],
        value.canvas_size[1],
        value.source_words[0],
        value.source_words[1],
        float_array(value.text_offsets.into_iter().flatten()),
        string_array(value.source_textures.iter().map(|identity| {
            identity
                .as_ref()
                .map(source_identity_text)
                .unwrap_or_default()
        })),
        string_array(HUD_TEXTURE_GLOBALS.iter().map(|name| (*name).to_owned()))
    )?;
    for (table_index, table) in value.rectangle_tables.iter().enumerate() {
        writeln!(
            text,
            "metadata/source_rectangle_table_{table_index} = {}",
            float_array(table.iter().flat_map(|row| row.iter().copied()))
        )?;
    }

    for (panel_index, panel) in value.panels.iter().enumerate() {
        writeln!(
            text,
            "\n[node name=\"Panel{panel_index}\" type=\"Control\" parent=\".\"]\nlayout_mode = 0\noffset_right = 640.0\noffset_bottom = 480.0\nmouse_filter = 2\nvisible = {}\nmetadata/source_layer_count = {}\nmetadata/source_unknown_bytes = PackedByteArray({}, {})",
            panel.enabled,
            panel.source_layer_count,
            panel.source_unknown_bytes[0],
            panel.source_unknown_bytes[1]
        )?;
        if let Some(identity) = &panel.source_layers {
            writeln!(
                text,
                "metadata/source_layers_identity = {}",
                quoted(&source_identity_text(identity))
            )?;
        }
        if let Some(identity) = &panel.material {
            writeln!(
                text,
                "metadata/source_material_identity = {}",
                quoted(&source_identity_text(identity))
            )?;
        }
        if let Some(identity) = &panel.unknown_reference {
            writeln!(
                text,
                "metadata/source_unknown_identity = {}",
                quoted(&source_identity_text(identity))
            )?;
        }
        if let Some(bytes) = panel.unknown_word {
            writeln!(
                text,
                "metadata/source_unknown_word = PackedByteArray({}, {}, {}, {})",
                bytes[0], bytes[1], bytes[2], bytes[3]
            )?;
        }
        for (layer_index, layer) in panel.layers.iter().enumerate() {
            let [left, right, top, bottom] = layer.destination;
            writeln!(
                text,
                "\n[node name=\"Layer{layer_index}\" type=\"TextureRect\" parent=\"Panel{panel_index}\"]\nlayout_mode = 0\noffset_left = {}\noffset_top = {}\noffset_right = {}\noffset_bottom = {}\nmouse_filter = 2\nmetadata/source_uv = {}",
                float(left),
                float(top),
                float(right),
                float(bottom),
                float_array(layer.uv)
            )?;
            if let Some(identity) = &layer.texture {
                writeln!(
                    text,
                    "metadata/source_texture_identity = {}",
                    quoted(&source_identity_text(identity))
                )?;
            }
            if let Some(bytes) = layer.opaque_texture_word {
                writeln!(
                    text,
                    "metadata/source_texture_handle_word = PackedByteArray({}, {}, {}, {})",
                    bytes[0], bytes[1], bytes[2], bytes[3]
                )?;
            }
        }
    }

    // The source's second Fix.sda table supplies the life background and bar
    // geometry. Empty TextureRect/TextureProgressBar nodes retain the native
    // edit points and actor values without drawing substitute artwork.
    let layout = &value.rectangle_tables[1];
    let base = layout[0];
    let life = layout[1];
    writeln!(
        text,
        "\n[node name=\"LifeBackground\" type=\"TextureRect\" parent=\".\"]\nlayout_mode = 0\noffset_left = {}\noffset_top = {}\noffset_right = {}\noffset_bottom = {}\nmouse_filter = 2\nvisible = false\nmetadata/source_slot = 2",
        float(base[0]),
        float(base[1]),
        float(base[2]),
        float(base[3])
    )?;
    writeln!(
        text,
        "\n[node name=\"Life\" type=\"TextureProgressBar\" parent=\".\"]\nlayout_mode = 0\noffset_left = {}\noffset_top = {}\noffset_right = {}\noffset_bottom = {}\nmouse_filter = 2\nshow_percentage = false\nfill_mode = 0\nvisible = false\nmetadata/source_slots = PackedByteArray(4, 3)\nmetadata/source_rectangle = {}",
        float(base[2]),
        float(life[1]),
        float(base[2] + life[4]),
        float(life[1] + life[5]),
        float_array([base[2], life[1], base[2] + life[4], life[1] + life[5]])
    )?;

    for (name, role, table, entry, kind) in [
        ("Armor", "armor", 1usize, 2usize, "TextureProgressBar"),
        ("Magic", "magic", 1usize, 5usize, "TextureProgressBar"),
        (
            "SelectedObject",
            "selected_object",
            1usize,
            10usize,
            "TextureRect",
        ),
        ("Weapon", "weapon", 1usize, 22usize, "TextureRect"),
    ] {
        let row = value.rectangle_tables[table][entry];
        let positioned = matches!(role, "selected_object" | "weapon");
        write!(
            text,
            "\n[node name={name:?} type=\"{kind}\" parent=\".\"]\nlayout_mode = 0\nmouse_filter = 2\nvisible = false\nmetadata/source_role = {}\nmetadata/source_table = {table}\nmetadata/source_entry = {entry}\nmetadata/source_rectangle = {}",
            quoted(role),
            float_array(row)
        )?;
        if positioned {
            writeln!(
                text,
                "\noffset_left = {}\noffset_top = {}\noffset_right = {}\noffset_bottom = {}",
                float(row[0]),
                float(row[1]),
                float(row[2]),
                float(row[3])
            )?;
        }
        writeln!(text)?;
    }

    project.write(scene_path, text.as_bytes())?;
    Ok(())
}
