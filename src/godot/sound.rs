//! Compile supported sound selection into GDScript over native AudioStream resources.
use super::{Project, disk_path, quoted};
use crate::model::{AssetPath, AssetRef, audio::*, content::ContentSource, io::Save};
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeSet, fmt::Write as _};
#[cfg(test)]
#[path = "sound_tests.rs"]
mod tests;

fn identity(reference: &SoundMemberRef) -> String {
    format!("{}#{}", reference.definitions.path, reference.id)
}
fn resource_path(project: &Project, reference: &SoundMemberRef, kind: &str) -> String {
    format!(
        "res://{}",
        disk_path(
            &project.selection,
            &reference.definitions.path,
            &format!("{kind}/{}.tres", reference.id)
        )
    )
}
fn call(project: &Project, reference: &SoundMemberRef) -> String {
    format!(
        "load({}).new().select_{}(emitter, gain, trace)",
        quoted(&format!(
            "res://{}",
            disk_path(&project.selection, &reference.definitions.path, "gd")
        )),
        reference.id
    )
}
pub(super) fn export_event(
    project: &mut Project,
    source: &mut impl ContentSource,
    link: &SoundEventLink,
    written: &mut BTreeSet<AssetPath>,
) -> Result<String> {
    let library = source.load(&link.library)?;
    library.validate()?;
    let SoundDefinitionTarget::Event(reference) = &link.binding(&library)?.target else {
        unreachable!()
    };
    let path = resource_path(project, reference, "events");
    if project.generated.contains(&path[6..]) {
        return Ok(path);
    }
    let definitions = source.load(&reference.definitions)?;
    definitions.validate()?;
    let event = definitions.event(reference.id)?;
    let script_path = path.trim_end_matches("tres").to_string() + "gd";
    let mut script = format!(
        "extends Resource\n## Compiled sound event {}. No source program is loaded at runtime.\n@export var identity: String = {}\n@export var stoppable: bool = false\n@export var dynamic: bool = false\n@export var pitch_variation: float = 0.0\n",
        label_comment(&event.name),
        quoted(&identity(reference))
    );
    let mut properties = String::new();
    match &event.operation {
        SoundOperation::Play {
            resource,
            pitch_variation,
            stoppable,
            dynamic,
        } => {
            let expression = if let Some(r) = resource {
                export_resource(project, source, r, written, &mut BTreeSet::new())?;
                call(project, r)
            } else {
                "null".into()
            };
            writeln!(
                script,
                "func select(emitter: Node3D) -> Resource:\n    var gain: float = 1.0\n    var trace: PackedStringArray = []\n    return {expression}\n\nfunc execute(emitter: Node3D) -> void:\n    emitter.play_voice(select(emitter), identity, pitch_variation, stoppable, dynamic)"
            )?;
            writeln!(
                properties,
                "pitch_variation = {pitch_variation}\nstoppable = {stoppable}\ndynamic = {dynamic}"
            )?;
        }
        SoundOperation::Stop { event } => {
            if let Some(r) = event {
                source.load(&r.definitions)?.event(r.id)?;
            }
            writeln!(
                script,
                "func execute(emitter: Node3D) -> void:\n    emitter.stop_event({})",
                event
                    .as_ref()
                    .map_or_else(|| "null".into(), |r| quoted(&identity(r)))
            )?;
        }
        SoundOperation::Other { name } => {
            project.diagnostic(format!(
                "{}: unsupported sound operation {name}",
                identity(reference)
            ));
            writeln!(
                script,
                "func execute(_emitter: Node3D) -> void:\n    push_warning({})",
                quoted(&format!(
                    "Unsupported sound operation {name} ({})",
                    identity(reference)
                ))
            )?;
        }
    }
    project.write(script_path[6..].into(), script.as_bytes())?;
    project.write(path[6..].into(),format!("[gd_resource type=\"Resource\" load_steps=2 format=3]\n[ext_resource type=\"Script\" path={} id=\"script\"]\n\n[resource]\nscript = ExtResource(\"script\")\n{properties}",quoted(&script_path)).as_bytes())?;
    Ok(path)
}
fn label_comment(name: &str) -> String {
    name.replace(['\n', '\r'], " ")
}
fn export_resource(
    project: &mut Project,
    source: &mut impl ContentSource,
    reference: &SoundMemberRef,
    written: &mut BTreeSet<AssetPath>,
    active: &mut BTreeSet<(AssetPath, u32)>,
) -> Result<()> {
    reference.validate()?;
    let key = (reference.definitions.path.clone(), reference.id);
    ensure!(
        active.insert(key.clone()) && active.len() <= 64,
        "cyclic or excessive sound program nesting"
    );
    let definitions = source.load(&reference.definitions)?;
    definitions.validate()?;
    let selected = definitions.resource(reference.id)?;
    let member = SoundMemberRef {
        definitions: reference.definitions.clone(),
        id: reference.id,
    };
    if written.insert(reference.definitions.path.clone()) {
        let mut script = String::from(
            "extends RefCounted\n## Compiled sound choices; each branch resolves to native waveform resources.\n",
        );
        for resource in &definitions.resources {
            let member = SoundMemberRef {
                definitions: reference.definitions.clone(),
                id: resource.id,
            };
            writeln!(
                script,
                "\nfunc select_{}(emitter: Node3D, gain: float, previous_trace: PackedStringArray) -> Resource:\n    var trace := previous_trace.duplicate()\n    trace.append({})",
                resource.id,
                quoted(&identity(&member))
            )?;
            if let Some(volume) = resource.volume {
                writeln!(script, "    gain *= {}", (volume / 127.).clamp(0., 1.))?;
            }
            match &resource.data {
                SoundResourceData::Sample { sample, .. } => {
                    if sample.is_some() {
                        let voice = resource_path(project, &member, "voices");
                        writeln!(
                            script,
                            "    var source_voice = load({})\n    if source_voice == null:\n        return null\n    var voice = source_voice.duplicate()\n    voice.gain = gain\n    voice.trace = trace\n    return voice",
                            quoted(&voice)
                        )?;
                    } else {
                        script.push_str("    return null\n");
                        project
                            .diagnostic(format!("{}: unbound sound waveform", identity(&member)));
                    }
                }
                SoundResourceData::Random {
                    silence_probability,
                    repeat_silence,
                    choices,
                } => {
                    writeln!(
                        script,
                        "    match emitter.choose_weighted({}, PackedFloat32Array([{}]), PackedByteArray([{}]), {silence_probability}, {repeat_silence}):",
                        quoted(&identity(&member)),
                        choices
                            .iter()
                            .map(|c| c.probability.to_string())
                            .collect::<Vec<_>>()
                            .join(","),
                        choices
                            .iter()
                            .map(|c| u8::from(c.repeat).to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    )?;
                    for (i, choice) in choices.iter().enumerate() {
                        writeln!(
                            script,
                            "        {i}: return {}",
                            call(project, &choice.resource)
                        )?;
                    }
                    script.push_str("        _: return null\n");
                }
                SoundResourceData::Switch {
                    selector,
                    default,
                    choices,
                    ..
                } => {
                    writeln!(
                        script,
                        "    var wanted: String = emitter.selectors.get({}, {})",
                        quoted(&format!("{}#{}", selector.definitions.path, selector.name)),
                        quoted(default)
                    )?;
                    for choice in choices {
                        writeln!(
                            script,
                            "    if wanted == {}: return {}",
                            quoted(&choice.value),
                            call(project, &choice.resource)
                        )?;
                    }
                    writeln!(
                        script,
                        "    return {}",
                        choices
                            .iter()
                            .find(|c| &c.value == default)
                            .map_or_else(|| "null".into(), |c| call(project, &c.resource))
                    )?;
                }
                SoundResourceData::Other { kind, .. } => {
                    project.diagnostic(format!(
                        "{}: unsupported sound resource {kind}",
                        identity(&member)
                    ));
                    writeln!(
                        script,
                        "    push_warning({})\n    return null",
                        quoted(&format!(
                            "Unsupported sound resource {kind} ({})",
                            identity(&member)
                        ))
                    )?;
                }
            }
        }
        project.write(
            disk_path(&project.selection, &reference.definitions.path, "gd"),
            script.as_bytes(),
        )?;
        project.diagnostic(format!("{}: native streams and compiled weighted/selector sound choices; attenuation, random generator, reverb and release envelopes remain approximations; live switch updates/material sensing are unsupported",reference.definitions.path));
    }
    match &selected.data {
        SoundResourceData::Sample { sample, controls } => {
            let sample = sample
                .as_ref()
                .with_context(|| format!("unbound sample {}", identity(reference)))?;
            export_sample_voice(
                project,
                source,
                &member,
                sample,
                controls,
                selected.looped,
                selected.loop_region.as_ref(),
                written,
            )?;
        }
        _ => {
            for child in selected.data.members() {
                export_resource(project, source, child, written, active)?;
            }
        }
    }
    active.remove(&key);
    Ok(())
}

fn export_sample_voice(
    project: &mut Project,
    source: &mut impl ContentSource,
    member: &SoundMemberRef,
    sample: &SoundSampleRef,
    controls: &SoundSampleControls,
    looped: bool,
    loop_region: Option<&crate::model::audio::SampleLoop>,
    written: &mut BTreeSet<AssetPath>,
) -> Result<()> {
    let voice = resource_path(project, member, "voices");
    if project.generated.contains(&voice[6..]) {
        return Ok(());
    }
    let stream = export_sample(project, source, sample, written)?;
    let waveform = resource_path(project, member, "streams");
    project.reserve(&waveform[6..])?;
    let n = project.sound_count;
    project.sound_count += 1;
    writeln!(
        project.builder,
        "    var voice_stream_{n} = load({}).duplicate()\n    voice_stream_{n}.loop_mode = {}",
        quoted(&stream),
        i32::from(looped)
    )?;
    if looped {
        if let Some(region) = loop_region {
            writeln!(
                project.builder,
                "    voice_stream_{n}.loop_begin = {}\n    voice_stream_{n}.loop_end = {}",
                region.start_frame, region.end_frame
            )?;
        } else {
            writeln!(
                project.builder,
                "    if voice_stream_{n}.loop_end <= voice_stream_{n}.loop_begin:\n        voice_stream_{n}.loop_begin = 0\n        voice_stream_{n}.loop_end = roundi(voice_stream_{n}.get_length() * voice_stream_{n}.mix_rate)"
            )?;
        }
    }
    writeln!(
        project.builder,
        "    if ResourceSaver.save(voice_stream_{n}, {}) != OK:\n        push_error(\"Cannot save native sound stream\")\n        quit(1)\n        return",
        quoted(&waveform)
    )?;
    project.write(
        voice[6..].into(),
        format!(
            "[gd_resource type=\"Resource\" load_steps=3 format=3]\n[ext_resource type=\"Script\" path=\"res://addons/astrolabe/audio/voice.gd\" id=\"script\"]\n[ext_resource type=\"AudioStreamWAV\" path={} id=\"stream\"]\n\n[resource]\nscript = ExtResource(\"script\")\nstream = ExtResource(\"stream\")\nspatial = {}\npitchable = {}\nsource_sample = {}\n",
            quoted(&waveform),
            controls.spatial,
            controls.pitchable,
            quoted(&stream)
        )
        .as_bytes(),
    )?;
    Ok(())
}

pub(super) fn export_sample(
    project: &mut Project,
    source: &mut impl ContentSource,
    sample: &SoundSampleRef,
    written: &mut BTreeSet<AssetPath>,
) -> Result<String> {
    let reference = match sample {
        SoundSampleRef::Sound(reference) => {
            if written.insert(reference.path.clone()) {
                project.save(reference, source.load(reference)?.as_ref())?;
            }
            reference.clone()
        }
        SoundSampleRef::Bank { bank, resource_id } => {
            let reference =
                AssetRef::<Sound>::new(format!("{}/resource_{resource_id}", bank.path))?;
            if written.insert(reference.path.clone()) {
                let bank = source.load(bank)?;
                let value = bank
                    .sounds
                    .iter()
                    .find(|s| s.resource_id == Some(*resource_id))
                    .context("missing bank-owned sample")?;
                project.save(&reference, value)?;
            }
            reference
        }
    };
    Ok(format!(
        "res://{}",
        disk_path(&project.selection, &reference.path, "tres")
    ))
}
