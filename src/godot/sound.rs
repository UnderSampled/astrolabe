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
    // Actor components can have independent traversal sets. The native project
    // owns the shared definition script once, while every selected member still
    // resolves its waveform and voice dependencies below.
    let definitions_script = disk_path(&project.selection, &reference.definitions.path, "gd");
    written.insert(reference.definitions.path.clone());
    if !project.generated.contains(&definitions_script) {
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
                SoundResourceData::Theme { .. } => {
                    writeln!(
                        script,
                        "    var source_voice = load({})\n    if source_voice == null:\n        return null\n    var voice = source_voice.duplicate()\n    voice.gain = gain\n    voice.trace = trace\n    return voice",
                        quoted(&resource_path(project, &member, "voices"))
                    )?;
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
        project.write(definitions_script, script.as_bytes())?;
        project.diagnostic(format!("{}: native streams and compiled weighted/selector sound choices; plain object samples use single-Micro source byte attenuation; pan, resource gain, random generator, sector eligibility, reverb and release envelopes remain approximations; live switch updates/material sensing are unsupported",reference.definitions.path));
    }
    match &selected.data {
        SoundResourceData::Sample { sample, controls } => {
            let sample = sample
                .as_ref()
                .with_context(|| format!("unbound sample {}", identity(reference)))?;
            export_sample_voice(
                project, source, &member, sample, controls, selected, written,
            )?;
        }
        SoundResourceData::Theme { .. } => {
            for child in selected.data.members() {
                export_resource(project, source, child, written, active)?;
            }
            export_theme_voice(project, source, &member, selected, written)?;
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

fn validate_native_theme_shape(theme: &SoundResourceDefinition, label: &str) -> Result<()> {
    let SoundResourceData::Theme {
        start_loop, parts, ..
    } = &theme.data
    else {
        anyhow::bail!("{label}: expected theme")
    };
    // Natural part advance (004fa5c0/004fa770) never reads a part's transition
    // or fade fields, nor the theme's main-loop parameter: those matter only
    // when a request switches themes, which stays unsupported at runtime.
    ensure!(
        !parts.is_empty()
            && *start_loop >= 0
            && (*start_loop as usize) < parts.len()
            && theme.looped,
        "{}: theme needs parts, a loop entry within them and looped playback",
        label
    );
    Ok(())
}

/// A repeat count at or above this is treated as unbounded. The source counts
/// down a 32-bit word from 0xffffffff when no group set it, which is hours of
/// playback; folding such counts keeps the simulated sequence finite.
const THEME_UNBOUNDED_REPEATS: u32 = 1 << 16;

/// The part order the source plays with no theme switch, as a finite prefix
/// plus a cycle: (part index per step, step the last step advances to).
/// Port of the natural advance 004fa5c0 (hype-decomp src/sound/theme_parts.c),
/// started as 004fa700 starts it. Part fields: start_loop (+4), end_loop (+8),
/// loops (+0x0c); theme start index (+0x14).
fn theme_natural_sequence(
    parts: &[ThemeSoundPart],
    start_index: usize,
) -> Result<(Vec<usize>, usize)> {
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    struct State {
        current: usize,
        repeat_start: Option<usize>,
        remaining: u32,
    }
    let count = parts.len();
    let folded = |n: u32| n.min(THEME_UNBOUNDED_REPEATS);
    let advance = |state: &mut State| {
        let next;
        let mut next_index = state.current;
        'record: {
            if parts[state.current].end_loop && state.remaining != 0 {
                if state.remaining < THEME_UNBOUNDED_REPEATS {
                    state.remaining -= 1;
                }
                next = state.repeat_start.unwrap_or(0);
                next_index = next;
                break 'record;
            }
            if state.remaining == 0 {
                state.repeat_start = None;
            }
            next_index += 1;
            if next_index < count {
                next = next_index;
            } else {
                next_index = start_index;
                next = start_index;
                let mut scan = start_index as i64;
                while !parts[scan as usize].start_loop {
                    scan -= 1;
                    if scan < 0 || parts[scan as usize].end_loop {
                        break 'record;
                    }
                }
                state.repeat_start = Some(scan as usize);
                state.remaining = folded(parts[scan as usize].loops as u32);
            }
        }
        if parts[next].start_loop && state.repeat_start.is_none() {
            state.repeat_start = Some(next_index);
            state.remaining = folded(parts[next].loops as u32);
        }
        state.current = next;
    };
    // 004fa700 then the advance it calls: the first part plays first.
    let mut state = State {
        current: 0,
        repeat_start: None,
        remaining: THEME_UNBOUNDED_REPEATS,
    };
    if parts[0].start_loop {
        state.repeat_start = Some(0);
        state.remaining = folded(parts[0].loops as u32);
    }
    let mut seen = std::collections::HashMap::new();
    let mut order = Vec::new();
    while !seen.contains_key(&state) {
        ensure!(
            order.len() <= 4096,
            "theme natural sequence does not settle into a cycle"
        );
        seen.insert(state, order.len());
        order.push(state.current);
        advance(&mut state);
    }
    Ok((order, seen[&state]))
}

fn export_theme_voice(
    project: &mut Project,
    source: &mut impl ContentSource,
    reference: &SoundMemberRef,
    theme: &SoundResourceDefinition,
    _written: &mut BTreeSet<AssetPath>,
) -> Result<()> {
    let SoundResourceData::Theme {
        start_loop,
        main_loops: _,
        parts,
    } = &theme.data
    else {
        unreachable!()
    };
    let voice = resource_path(project, reference, "voices");
    if project.generated.contains(&voice[6..]) {
        return Ok(());
    }
    validate_native_theme_shape(theme, &identity(reference))?;
    let (order, cycle) = theme_natural_sequence(parts, *start_loop as usize)?;
    // Above the interactive stream's 63 clips, one PCM stream plays the same
    // unrolled order and loops from the cycle's first step.
    if order.len() > 63 {
        return export_flat_theme_voice(project, source, reference, theme);
    }
    let stream = resource_path(project, reference, "streams");
    project.reserve(&stream[6..])?;
    let n = project.sound_count;
    project.sound_count += 1;
    writeln!(
        project.builder,
        "    var theme_stream_{n} = AudioStreamInteractive.new()\n    theme_stream_{n}.clip_count = {}\n    theme_stream_{n}.initial_clip = 0",
        order.len()
    )?;
    // One clip per step of the source's natural part order; the last step
    // advances into the repeating cycle.
    for (i, part_index) in order.iter().copied().enumerate() {
        let part = &parts[part_index];
        let definitions = source.load(&part.resource.definitions)?;
        let resource = definitions.resource(part.resource.id)?;
        ensure!(
            matches!(resource.data, SoundResourceData::Sample { .. }),
            "{}: nested theme parts are unsupported",
            identity(reference)
        );
        let gain = resource.volume.map_or(1.0, |v| (v / 127.0).clamp(0.0, 1.0));
        let next = if i + 1 < order.len() { i + 1 } else { cycle };
        writeln!(
            project.builder,
            "    var theme_part_{n}_{i} = load({})\n    var theme_gain_{n}_{i} = AudioStreamSynchronized.new()\n    theme_gain_{n}_{i}.stream_count = 1\n    theme_gain_{n}_{i}.set_sync_stream(0, theme_part_{n}_{i}.stream)\n    theme_gain_{n}_{i}.set_sync_stream_volume(0, linear_to_db(maxf({gain}, 0.000001)))\n    theme_stream_{n}.set_clip_name({i}, {})\n    theme_stream_{n}.set_clip_stream({i}, theme_gain_{n}_{i})\n    theme_stream_{n}.set_clip_auto_advance({i}, AudioStreamInteractive.AUTO_ADVANCE_ENABLED)\n    theme_stream_{n}.set_clip_auto_advance_next_clip({i}, {next})\n    theme_stream_{n}.add_transition({i}, {next}, AudioStreamInteractive.TRANSITION_FROM_TIME_END, AudioStreamInteractive.TRANSITION_TO_TIME_START, AudioStreamInteractive.FADE_DISABLED, 0.0)",
            quoted(&resource_path(project, &part.resource, "voices")),
            quoted(&format!("step_{i}_part_{part_index}"))
        )?;
    }
    writeln!(
        project.builder,
        "    if ResourceSaver.save(theme_stream_{n}, {}) != OK:\n        push_error(\"Cannot save native music theme\")\n        quit(1)\n        return",
        quoted(&stream)
    )?;
    project.write(voice[6..].into(), format!(
        "[gd_resource type=\"Resource\" load_steps=3 format=3]\n[ext_resource type=\"Script\" path=\"res://addons/astrolabe/audio/voice.gd\" id=\"script\"]\n[ext_resource type=\"AudioStreamInteractive\" path={} id=\"stream\"]\n\n[resource]\nscript = ExtResource(\"script\")\nstream = ExtResource(\"stream\")\nspatial = false\npitchable = false\nsource_sample = {}\ntheme_identity = {}\n", quoted(&stream), quoted(&stream), quoted(&identity(reference))).as_bytes())?;
    project.diagnostic(format!("{}: native ordered theme parts and loop entry; natural boundaries queue the next part without invented crossfade. Switching an active theme remains unsupported pending audio-driver timing; driver buffering and volume quantization are not matched", identity(reference)));
    Ok(())
}

/// Long, unconditional zero-fade themes become a native sample loop. This is
/// target conversion of meaningful PCM; source parts remain shared Model data.
fn export_flat_theme_voice(
    project: &mut Project,
    source: &mut impl ContentSource,
    reference: &SoundMemberRef,
    theme: &SoundResourceDefinition,
) -> Result<()> {
    let sound = flatten_theme_samples(source, theme, &identity(reference))?;
    let stream = resource_path(project, reference, "streams");
    project.reserve(&stream[6..])?;
    let n = project.sound_count;
    project.sound_count += 1;
    let input = format!("build_inputs/theme_{n}.pcm");
    let AudioSamples::Pcm16(samples) = &sound.samples else {
        unreachable!()
    };
    let bytes: Vec<_> = samples
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect();
    project.write_build_input(&input, &bytes)?;
    let region = sound
        .loop_region
        .as_ref()
        .context("missing native theme loop")?;
    writeln!(
        project.builder,
        "    var flat_theme_{n} = AudioStreamWAV.new()\n    flat_theme_{n}.format = AudioStreamWAV.FORMAT_16_BITS\n    flat_theme_{n}.mix_rate = {}\n    flat_theme_{n}.stereo = {}\n    flat_theme_{n}.data = FileAccess.get_file_as_bytes({})\n    flat_theme_{n}.loop_mode = AudioStreamWAV.LOOP_FORWARD\n    flat_theme_{n}.loop_begin = {}\n    flat_theme_{n}.loop_end = {}\n    if ResourceSaver.save(flat_theme_{n}, {}) != OK:\n        push_error(\"Cannot save native flattened theme\")\n        quit(1)\n        return",
        sound.sample_rate,
        sound.channels == 2,
        quoted(&format!("res://{input}")),
        region.start_frame,
        region.end_frame,
        quoted(&stream)
    )?;
    let voice = resource_path(project, reference, "voices");
    project.write(voice[6..].into(), format!(
        "[gd_resource type=\"Resource\" load_steps=3 format=3]\n[ext_resource type=\"Script\" path=\"res://addons/astrolabe/audio/voice.gd\" id=\"script\"]\n[ext_resource type=\"AudioStreamWAV\" path={} id=\"stream\"]\n\n[resource]\nscript = ExtResource(\"script\")\nstream = ExtResource(\"stream\")\nspatial = false\npitchable = false\nsource_sample = {}\ntheme_identity = {}\n", quoted(&stream), quoted(&stream), quoted(&identity(reference))).as_bytes())?;
    project.diagnostic(format!(
        "{}: long zero-fade ordered theme converted to native PCM sample loop; resource gain baked per part, authored loop entry retained; PCM quantization and driver buffering are not source matched",
        identity(reference)
    ));
    Ok(())
}

fn flatten_theme_samples(
    source: &mut impl ContentSource,
    theme: &SoundResourceDefinition,
    label: &str,
) -> Result<Sound> {
    validate_native_theme_shape(theme, label)?;
    let SoundResourceData::Theme {
        start_loop, parts, ..
    } = &theme.data
    else {
        unreachable!()
    };
    // The source's natural part order (repeat groups unrolled), looping from
    // the cycle's first step.
    let (order, cycle) = theme_natural_sequence(parts, *start_loop as usize)?;
    let mut samples = Vec::new();
    let mut format = None;
    let mut loop_begin = 0;
    for (step, part) in order.iter().map(|&index| &parts[index]).enumerate() {
        let definitions = source.load(&part.resource.definitions)?;
        let resource = definitions.resource(part.resource.id)?;
        let SoundResourceData::Sample {
            sample: Some(sample),
            ..
        } = &resource.data
        else {
            anyhow::bail!("{label}: flattened theme part is not a bound sample")
        };
        ensure!(
            !resource.looped,
            "{label}: flattened theme part must finish naturally"
        );
        let gain = f64::from(resource.volume.map_or(1.0, |v| (v / 127.0).clamp(0.0, 1.0)));
        let mut append = |sound: &Sound| -> Result<()> {
            sound.validate()?;
            ensure!(
                sound.channels <= 2,
                "{label}: native theme supports mono/stereo"
            );
            let current = (sound.sample_rate, sound.channels);
            let format = *format.get_or_insert(current);
            ensure!(
                current == format,
                "{label}: mixed sample rates/channels require resampling"
            );
            ensure!(!sound.samples.is_empty(), "{label}: empty theme part");
            if step == cycle {
                loop_begin = (samples.len() / usize::from(sound.channels)) as u64;
            }
            match &sound.samples {
                AudioSamples::Pcm16(values) => samples.extend(values.iter().map(|&value| {
                    (f64::from(value) * gain).round().clamp(-32768.0, 32767.0) as i16
                })),
                AudioSamples::Float32(values) => samples.extend(values.iter().map(|&value| {
                    (f64::from(value) * gain * 32768.0)
                        .round()
                        .clamp(-32768.0, 32767.0) as i16
                })),
            }
            Ok(())
        };
        match sample {
            SoundSampleRef::Sound(reference) => append(source.load(reference)?.as_ref())?,
            SoundSampleRef::Bank { bank, resource_id } => {
                let bank = source.load(bank)?;
                append(
                    bank.sounds
                        .iter()
                        .find(|sound| sound.resource_id == Some(*resource_id))
                        .context("missing bank-owned theme sample")?,
                )?;
            }
        }
    }
    let (sample_rate, channels) = format.context("empty theme")?;
    let end_frame = (samples.len() / usize::from(channels)) as u64;
    ensure!(
        end_frame <= i32::MAX as u64,
        "{label}: native WAV loop frame limit exceeded"
    );
    Ok(Sound {
        name: theme.name.clone(),
        sample_rate,
        channels,
        samples: AudioSamples::Pcm16(samples),
        loop_region: Some(SampleLoop {
            start_frame: loop_begin,
            end_frame,
        }),
        resource_id: None,
        surround: None,
        fidelity_notes: vec![],
    })
}

fn export_sample_voice(
    project: &mut Project,
    source: &mut impl ContentSource,
    member: &SoundMemberRef,
    sample: &SoundSampleRef,
    controls: &SoundSampleControls,
    resource: &SoundResourceDefinition,
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
        i32::from(resource.looped)
    )?;
    if resource.looped {
        if let Some(region) = &resource.loop_region {
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
            "[gd_resource type=\"Resource\" load_steps=3 format=3]\n[ext_resource type=\"Script\" path=\"res://addons/astrolabe/audio/voice.gd\" id=\"script\"]\n[ext_resource type=\"AudioStreamWAV\" path={} id=\"stream\"]\n\n[resource]\nscript = ExtResource(\"script\")\nstream = ExtResource(\"stream\")\nspatial = {}\npitchable = {}\nsource_parameter_conversion = true\nsource_sample = {}\n",
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
            written.insert(reference.path.clone());
            if !project
                .generated
                .contains(&disk_path(&project.selection, &reference.path, "tres"))
            {
                project.save(reference, source.load(reference)?.as_ref())?;
            }
            reference.clone()
        }
        SoundSampleRef::Bank { bank, resource_id } => {
            let reference =
                AssetRef::<Sound>::new(format!("{}/resource_{resource_id}", bank.path))?;
            written.insert(reference.path.clone());
            if !project
                .generated
                .contains(&disk_path(&project.selection, &reference.path, "tres"))
            {
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
