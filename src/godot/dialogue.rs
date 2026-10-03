//! Localized source values become native Resources and Translation catalogues.
use super::{Project, disk_path, quoted};
use crate::model::{AssetPath, content::ContentSource, dialogue::Dialogue};
use anyhow::Result;
use std::{collections::BTreeSet, fmt::Write as _};
pub(super) fn export(
    project: &mut Project,
    source: &mut impl ContentSource,
    value: &Dialogue,
    written: &mut BTreeSet<AssetPath>,
) -> Result<()> {
    let path = disk_path(&project.selection, &value.path, "tres");
    let translation = disk_path(&project.selection, &value.path, "translation.tres");
    project.reserve(&translation)?;
    let n = project.dialogue_resources.len();
    writeln!(
        project.builder,
        "    var dialogue_translation_{n} = Translation.new()\n    dialogue_translation_{n}.locale = {}",
        quoted(&value.locale)
    )?;
    let voice = value
        .voice
        .as_ref()
        .map(|v| super::sound::export_event(project, source, v, written))
        .transpose()?;
    let mut resources = String::new();
    let mut ids = Vec::new();
    let mut count = 0;
    // Debug uses round-trippable float literals, including an exponent for the
    // 3.4e38 untimed sentinel. Display can produce an integer-looking decimal
    // that Godot attempts to parse as an overflowing signed 64-bit integer.
    for (i, line) in value.lines.iter().enumerate() {
        let mut spans = Vec::new();
        for (j, span) in line.spans.iter().enumerate() {
            let key = format!(
                "hype.dialogue.{}.{}.line{i}.span{j}",
                value.name, value.locale
            );
            writeln!(
                project.builder,
                "    dialogue_translation_{n}.add_message({}, {})",
                quoted(&key),
                quoted(&span.text)
            )?;
            let id = format!("span_{i}_{j}");
            spans.push(format!("SubResource(\"{id}\")"));
            count += 1;
            writeln!(
                resources,
                "[sub_resource type=\"Resource\" id=\"{id}\"]\nscript = ExtResource(\"span\")\nkey = {}\nsource_offset = {:?}\nemphasis = {}\n",
                quoted(&key),
                span.offset,
                span.emphasis
            )?;
        }
        let id = format!("line_{i}");
        ids.push(format!("SubResource(\"{id}\")"));
        count += 1;
        writeln!(
            resources,
            "[sub_resource type=\"Resource\" id=\"{id}\"]\nscript = ExtResource(\"line\")\nspans = Array[Resource]([{}])\nend_seconds = {:?}\nflags = {}\n",
            spans.join(", "),
            line.end_seconds,
            line.flags
        )?;
    }
    writeln!(
        project.builder,
        "    if ResourceSaver.save(dialogue_translation_{n}, {}) != OK:\n        push_error(\"Cannot save dialogue Translation\")\n        quit(1)\n        return",
        quoted(&format!("res://{translation}"))
    )?;
    let callback = value.completion.as_ref();
    let voice_ext = voice
        .as_ref()
        .map(|p| {
            format!(
                "[ext_resource type=\"Resource\" path={} id=\"voice\"]\n",
                quoted(p)
            )
        })
        .unwrap_or_default();
    let text = format!(
        "[gd_resource type=\"Resource\" load_steps={} format=3]\n[ext_resource type=\"Script\" path=\"res://addons/astrolabe/ui/dialogue_item.gd\" id=\"item\"]\n[ext_resource type=\"Script\" path=\"res://addons/astrolabe/ui/dialogue_line.gd\" id=\"line\"]\n[ext_resource type=\"Script\" path=\"res://addons/astrolabe/ui/dialogue_span.gd\" id=\"span\"]\n[ext_resource type=\"Translation\" path={} id=\"translation\"]\n{voice_ext}\n{resources}\n[resource]\nscript = ExtResource(\"item\")\nidentity = {}\nlocale = {}\ntranslation = ExtResource(\"translation\")\nbox = {}\nlines = Array[Resource]([{}])\nowner_identity = {}\ncompletion_model = {}\ncompletion_normal = {}\n{}",
        count + 5 + usize::from(voice.is_some()),
        quoted(&format!("res://{translation}")),
        quoted(&value.name),
        quoted(&value.locale),
        value.box_style,
        ids.join(", "),
        quoted(value.owner.path.as_str()),
        quoted(callback.map(|c| c.model.path.as_str()).unwrap_or("")),
        callback.map_or(-1, |c| i64::from(c.behavior)),
        if voice.is_some() {
            "voice = ExtResource(\"voice\")\n"
        } else {
            ""
        }
    );
    project.write(path.clone(), text.as_bytes())?;
    project.dialogue_resources.push(path);
    project.diagnostic(format!("{}: localized native text/voice/timing/callback; font metrics/span offsets and line-reveal animation await fidelity validation",value.path));
    Ok(())
}
