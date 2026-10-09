use super::*;
use crate::model::{Selection, io::Load};

#[test]
#[ignore = "requires native Godot; source software distance publication and PCM"]
fn native_source_audio_attenuation_preserves_static_publication() -> Result<()> {
    let temp = tempfile::tempdir()?;
    crate::godot::support::initialize(temp.path())?;
    std::fs::write(
        temp.path().join("check_audio_attenuation.gd"),
        include_bytes!("../../tests/check_audio_attenuation.gd"),
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let log = crate::godot::run_godot(
        &engine,
        temp.path(),
        &["--headless", "--script", "res://check_audio_attenuation.gd"],
        std::time::Duration::from_secs(20),
    )?;
    assert!(log.contains("AUDIO_ATTENUATION_OK"), "{log}");
    Ok(())
}

#[test]
#[ignore = "requires native Godot; source fades, preferences and DWORD gain"]
fn native_source_fades_preserve_preferences_and_publish_byte_gain() -> Result<()> {
    let temp = tempfile::tempdir()?;
    crate::godot::support::initialize(temp.path())?;
    std::fs::write(
        temp.path().join("check_audio_mixer.gd"),
        include_bytes!("../../tests/check_audio_mixer.gd"),
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let log = crate::godot::run_godot(
        &engine,
        temp.path(),
        &["--headless", "--script", "res://check_audio_mixer.gd"],
        std::time::Duration::from_secs(20),
    )?;
    assert!(log.contains("AUDIO_MIXER_OK"), "{log}");
    Ok(())
}

#[test]
#[ignore = "requires native Godot; source sound module and request lifecycle"]
fn native_sound_requests_keep_module_scope_and_complete_removed_holders() -> Result<()> {
    let temp = tempfile::tempdir()?;
    crate::godot::support::initialize(temp.path())?;
    std::fs::write(
        temp.path().join("check_audio_channels.gd"),
        include_bytes!("../../tests/check_audio_channels.gd"),
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let log = crate::godot::run_godot(
        &engine,
        temp.path(),
        &["--headless", "--script", "res://check_audio_channels.gd"],
        std::time::Duration::from_secs(20),
    )?;
    assert!(log.contains("AUDIO_CHANNELS_OK"), "{log}");
    Ok(())
}

#[test]
fn independent_actor_sound_visits_share_scripts_and_waveforms() -> Result<()> {
    let library_ref = AssetRef::<SoundDefinitions>::new("sound:/shared.lcb")?;
    let package_a = AssetRef::<SoundDefinitions>::new("sound:/a.csb")?;
    let package_b = AssetRef::<SoundDefinitions>::new("sound:/b.csb")?;
    let waveform = AssetRef::<Sound>::new("sound:/shared-tone")?;
    let empty = || SoundDefinitions {
        name: "shared".into(),
        kind: SoundDefinitionKind::Events,
        default_language: None,
        groups: vec![],
        banks: vec![],
        levels: vec![],
        bindings: vec![],
        events: vec![],
        resources: vec![],
        selectors: vec![],
        fidelity_notes: vec![],
        reconstruction_gaps: vec![],
    };
    let member = |definitions: &AssetRef<SoundDefinitions>, id| SoundMemberRef {
        definitions: definitions.clone(),
        id,
    };
    let mut source = crate::model::content::Collection::default();
    for (package, count) in [(&package_a, 2), (&package_b, 1)] {
        let mut definitions = empty();
        for id in 0..count {
            definitions.resources.push(SoundResourceDefinition {
                id,
                name: format!("sample{id}"),
                data: SoundResourceData::Sample {
                    sample: Some(SoundSampleRef::Sound(waveform.clone())),
                    controls: SoundSampleControls {
                        pitchable: false,
                        volumeable: true,
                        pannable: false,
                        spatial: false,
                        reverb: false,
                    },
                },
                file: None,
                volume: Some(127.),
                looped: false,
                loop_region: None,
                sample_rate: None,
                channels: None,
            });
            definitions.events.push(SoundEvent {
                name: format!("play{id}"),
                resource_id: Some(id),
                operation: SoundOperation::Play {
                    resource: Some(member(package, id)),
                    pitch_variation: 0.,
                    stoppable: false,
                    dynamic: false,
                },
                attributes: vec![],
            });
        }
        source.insert(package, definitions)?;
    }
    let mut library = empty();
    library.kind = SoundDefinitionKind::Library;
    library.bindings = [
        member(&package_a, 0),
        member(&package_a, 1),
        member(&package_b, 0),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, event)| SoundDefinitionBinding {
        target: SoundDefinitionTarget::Event(event),
        bank: 0,
        member: i as u32,
    })
    .collect();
    source.insert(&library_ref, library)?;
    source.insert(
        &waveform,
        Sound {
            name: "tone".into(),
            sample_rate: 8000,
            channels: 1,
            samples: AudioSamples::Pcm16(vec![1234; 80]),
            loop_region: None,
            resource_id: None,
            surround: None,
            fidelity_notes: vec![],
        },
    )?;
    let temp = tempfile::tempdir()?;
    let mut project = Project::new(
        temp.path().join("godot"),
        Selection::hype("shared-audio", "en")?,
    )?;
    for member in [0, 1, 2, 0] {
        // Scene-global and generated actor components each own a traversal set.
        export_event(
            &mut project,
            &mut source,
            &SoundEventLink {
                library: library_ref.clone(),
                bank: 0,
                member,
            },
            &mut BTreeSet::new(),
        )?;
    }
    assert_eq!(
        project
            .generated
            .iter()
            .filter(|p| p.ends_with(".csb.gd"))
            .count(),
        2
    );
    assert_eq!(
        project
            .generated
            .iter()
            .filter(|p| p.ends_with(".wav"))
            .count(),
        1
    );
    assert_eq!(
        project
            .generated
            .iter()
            .filter(|p| p.contains(".voices/"))
            .count(),
        3
    );
    Ok(())
}

#[test]
#[ignore = "requires native Godot; included by tools/test.py --local"]
fn sound_program_rete_edit_native_choice_loop_stop_and_mixer() -> Result<()> {
    let library_ref = AssetRef::<SoundDefinitions>::new("sound:/Sound/library.lcb")?;
    let definitions_ref = AssetRef::<SoundDefinitions>::new("sound:/Sound/test.csb")?;
    let bank_ref = AssetRef::<SoundBank>::new("sound:/bank")?;
    let member = |id| SoundMemberRef {
        definitions: definitions_ref.clone(),
        id,
    };
    let empty = || SoundDefinitions {
        name: "test".into(),
        kind: SoundDefinitionKind::Events,
        default_language: None,
        groups: vec![],
        banks: vec![],
        levels: vec![],
        bindings: vec![],
        events: vec![],
        resources: vec![],
        selectors: vec![],
        fidelity_notes: vec![],
        reconstruction_gaps: vec![],
    };
    let mut definitions = empty();
    let sample = |id: u32| SoundResourceData::Sample {
        sample: Some(SoundSampleRef::Bank {
            bank: bank_ref.clone(),
            resource_id: id + 41,
        }),
        controls: SoundSampleControls {
            pitchable: true,
            volumeable: true,
            pannable: true,
            spatial: false,
            reverb: false,
        },
    };
    let resource = |id, data| SoundResourceDefinition {
        id,
        name: format!("resource{id}"),
        data,
        file: None,
        volume: Some(127.),
        looped: false,
        loop_region: None,
        sample_rate: None,
        channels: None,
    };
    definitions.resources = vec![
        resource(0, sample(0)),
        resource(1, sample(1)),
        resource(
            2,
            SoundResourceData::Random {
                silence_probability: 0.,
                repeat_silence: false,
                choices: vec![
                    RandomSoundChoice {
                        resource: member(0),
                        probability: 0.8,
                        repeat: false,
                    },
                    RandomSoundChoice {
                        resource: member(1),
                        probability: 0.2,
                        repeat: false,
                    },
                ],
            },
        ),
        resource(
            3,
            SoundResourceData::Switch {
                selector: SoundSelectorRef {
                    definitions: AssetRef::new("sound:/Sound/types.sif")?,
                    name: "Material".into(),
                },
                default: "Stone".into(),
                dynamic: false,
                maintain: false,
                choices: vec![
                    SwitchSoundChoice {
                        resource: member(2),
                        value: "Stone".into(),
                    },
                    SwitchSoundChoice {
                        resource: member(1),
                        value: "Wood".into(),
                    },
                ],
            },
        ),
    ];
    definitions.events = (0..2)
        .map(|id| SoundEvent {
            name: format!("play{id}"),
            resource_id: Some(id),
            operation: SoundOperation::Play {
                resource: Some(member(3)),
                pitch_variation: 0.1,
                stoppable: true,
                dynamic: true,
            },
            attributes: vec![],
        })
        .collect();
    definitions.events.push(SoundEvent {
        name: "stop0".into(),
        resource_id: Some(2),
        operation: SoundOperation::Stop {
            event: Some(member(0)),
        },
        attributes: vec![],
    });
    let mut library = empty();
    library.kind = SoundDefinitionKind::Library;
    library.bindings = (0..3)
        .map(|id| SoundDefinitionBinding {
            target: SoundDefinitionTarget::Event(member(id)),
            bank: 0,
            member: id,
        })
        .collect();
    let bank = SoundBank {
        name: "samples".into(),
        sounds: (0..2)
            .map(|i| Sound {
                name: format!("wave{i}"),
                sample_rate: 8000,
                channels: 1,
                samples: AudioSamples::Pcm16(
                    (0..4000)
                        .map(|n| if n % 40 < 20 { 5000 } else { -5000 })
                        .collect(),
                ),
                loop_region: None,
                resource_id: Some(i + 41),
                surround: None,
                fidelity_notes: vec![],
            })
            .collect(),
        external_sounds: vec![],
        events: vec![],
        fidelity_notes: vec![],
        reconstruction_gaps: vec![],
    };
    let temp = tempfile::tempdir()?;
    let selection = Selection::hype("sound-test", "en")?;
    let mut store = crate::rete::Store::new(temp.path().join("rete"), selection.clone())?;
    store.save(&definitions_ref, &definitions)?;
    store.save(&library_ref, &library)?;
    store.save(&bank_ref, &bank)?;
    drop(store);
    let mut store = crate::rete::Store::new(temp.path().join("rete"), selection.clone())?;
    assert_eq!(*store.load(&definitions_ref)?, definitions);
    definitions.resources[0].volume = Some(63.5);
    definitions.resources[0].looped = true;
    definitions.resources[0].loop_region = Some(SampleLoop {
        start_frame: 100,
        end_frame: 600,
    });
    store.save(&definitions_ref, &definitions)?;
    drop(store);
    let mut store = crate::rete::Store::new(temp.path().join("rete"), selection.clone())?;
    let output = temp.path().join("godot");
    let mut project = Project::new(&output, selection)?;
    let mut written = BTreeSet::new();
    let mut events = Vec::new();
    for member in 0..3 {
        events.push(export_event(
            &mut project,
            &mut store,
            &SoundEventLink {
                library: library_ref.clone(),
                bank: 0,
                member,
            },
            &mut written,
        )?);
    }
    assert_eq!(
        project
            .generated
            .iter()
            .filter(|p| p.ends_with(".wav"))
            .count(),
        2,
        "events must share bank waveforms"
    );
    project.write(
        "generated/hype/sound-test/events.json".into(),
        &serde_json::to_vec(&events)?,
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    project.finish(&engine)?;
    std::fs::write(
        output.join("sound_check.gd"),
        include_str!("scripts/sound_check.gd"),
    )?;
    let log = super::super::run_godot(
        &engine,
        &output,
        &[
            "--headless",
            "--verbose",
            "--script",
            "res://sound_check.gd",
        ],
        std::time::Duration::from_secs(15),
    )?;
    assert!(log.contains("SOUND_PROGRAM_OK"));
    Ok(())
}

#[test]
#[ignore = "requires native Godot; source microphone pose API"]
fn native_microphone_basis_selects_position_and_orientation_sources() -> Result<()> {
    use std::time::Duration;
    let temp = tempfile::tempdir()?;
    crate::godot::support::initialize(temp.path())?;
    let script = r#"extends SceneTree
const Audio = preload("res://addons/astrolabe/audio/emitter.gd")
func _initialize(): call_deferred("check")
func check():
    var requester = Node.new()
    root.add_child(requester)
    var actor = Node3D.new()
    root.add_child(actor)
    actor.position = Vector3(3, 4, 5)
    actor.rotation.y = 0.4
    var emitter = Audio.new()
    emitter.name = "SoundEmitter"
    actor.add_child(emitter)
    assert(Audio.microphone_basis_for(actor) == null, "unimported Micro basis must stay explicit")
    var global_actor = Node3D.new()
    global_actor.name = "Global"
    root.add_child(global_actor)
    assert(Audio.microphone_basis_for(global_actor) == null)
    assert(Audio.switch_old_micro_perso(requester, global_actor, null), "orientation setter does not require a position selection")
    var listener = root.get_node("AstrolabeListener")
    assert(listener.microphone_position_actor == null and listener.microphone_orientation_actor == global_actor)
    assert(listener.microphone_orientation_basis == null and not listener.microphone_pose_known)
    assert(Audio.switch_micro_perso(requester, global_actor, actor, null), "null Micro does not reject actor selection")
    assert(listener.microphone_position_actor == global_actor and listener.microphone_orientation_actor == global_actor)
    assert(listener.microphone_orientation_basis == null and not listener.microphone_pose_known)
    assert(not listener.is_current(), "unknown source pose is not fabricated")
    var offset = Basis(Vector3.RIGHT, 0.25)
    emitter.microphone_basis = offset
    emitter.microphone_basis_known = true
    assert(Audio.microphone_basis_for(actor).is_equal_approx(offset))
    assert(Audio.switch_micro_perso(requester, null, actor, offset))
    assert(listener is AudioListener3D and listener.is_current())
    assert(listener.global_position.is_equal_approx(actor.global_position))
    assert(listener.global_basis.is_equal_approx(actor.global_basis * offset))
    var known_pose = listener.global_transform
    assert(Audio.switch_micro_perso(requester, global_actor, actor, null))
    assert(listener.microphone_position_actor == global_actor and listener.microphone_orientation_basis == null)
    assert(not listener.microphone_pose_known and listener.global_transform.is_equal_approx(known_pose))
    assert(Audio.switch_old_micro_perso(requester, actor, null))
    assert(listener.microphone_position_actor == global_actor and listener.microphone_orientation_actor == actor)
    assert(listener.microphone_orientation_basis == null and not listener.microphone_pose_known)
    assert(Audio.switch_micro_perso(requester, actor, actor, offset))
    requester.queue_free()
    actor.position = Vector3(-3, 1, 8)
    actor.rotation.y = -0.6
    await create_timer(0.05).timeout
    assert(listener.is_current())
    assert(listener.global_position.is_equal_approx(actor.global_position), "listener follows actor after caller destruction")
    var other = Node3D.new()
    other.position = Vector3(90, 80, 70)
    other.rotation.z = 0.8
    root.add_child(other)
    assert(Audio.switch_old_micro_perso(other, other, Basis.IDENTITY))
    assert(listener.global_position.is_equal_approx(actor.global_position), "old microphone setter keeps current position")
    assert(listener.global_basis.is_equal_approx(other.global_basis))
    assert(Audio.switch_micro_perso(other, other, actor, Basis.IDENTITY))
    assert(listener.global_position.is_equal_approx(other.global_position))
    assert(listener.microphone_position_actor == other and listener.microphone_orientation_actor == other)
    other.queue_free()
    await create_timer(0.05).timeout
    assert(not listener.is_current(), "freed microphone actor invalidates binding")
    actor.free()
    global_actor.free()
    listener.free()
    print("NATIVE_MICROPHONE_OK")
    quit(0)
"#;
    std::fs::write(temp.path().join("check_microphone.gd"), script)?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    crate::godot::run_godot(
        &engine,
        temp.path(),
        &[
            "--headless",
            "--log-file",
            "microphone-parser.log",
            "--check-only",
            "--script",
            "res://check_microphone.gd",
        ],
        Duration::from_secs(10),
    )?;
    let evidence = crate::godot::run_godot(
        &engine,
        temp.path(),
        &[
            "--headless",
            "--log-file",
            "microphone.log",
            "--script",
            "res://check_microphone.gd",
        ],
        Duration::from_secs(10),
    )?;
    ensure!(evidence.contains("NATIVE_MICROPHONE_OK"), "{evidence}");
    println!("{evidence}");
    Ok(())
}

#[test]
#[ignore = "requires native Godot; long theme native PCM loop"]
fn long_music_theme_preserves_native_pcm_gain_and_loop_entry() -> Result<()> {
    let definitions_ref = AssetRef::<SoundDefinitions>::new("sound:/Sound/long.csb")?;
    let library_ref = AssetRef::<SoundDefinitions>::new("sound:/Sound/long.lcb")?;
    let wave_ref = AssetRef::<Sound>::new("sound:/synthetic/long-tone")?;
    let member = |id| SoundMemberRef {
        definitions: definitions_ref.clone(),
        id,
    };
    let empty = || SoundDefinitions {
        name: "long music".into(),
        kind: SoundDefinitionKind::Events,
        default_language: None,
        groups: vec![],
        banks: vec![],
        levels: vec![],
        bindings: vec![],
        events: vec![],
        resources: vec![],
        selectors: vec![],
        fidelity_notes: vec![],
        reconstruction_gaps: vec![],
    };
    let resource = |id, volume, data| SoundResourceDefinition {
        id,
        name: format!("part{id}"),
        data,
        file: None,
        volume: Some(volume),
        looped: id == 75,
        loop_region: None,
        sample_rate: None,
        channels: None,
    };
    let mut definitions = empty();
    definitions.resources = (0..75)
        .map(|id| {
            resource(
                id,
                (40 + id) as f32,
                SoundResourceData::Sample {
                    sample: Some(SoundSampleRef::Sound(wave_ref.clone())),
                    controls: SoundSampleControls {
                        pitchable: false,
                        volumeable: true,
                        pannable: true,
                        spatial: false,
                        reverb: false,
                    },
                },
            )
        })
        .collect();
    definitions.resources.push(resource(
        75,
        75.0,
        SoundResourceData::Theme {
            start_loop: 7,
            main_loops: 0,
            parts: (0..75)
                .map(|id| ThemeSoundPart {
                    resource: member(id),
                    transition: ThemeTransition::Crossfade,
                    start_loop: false,
                    end_loop: false,
                    loops: 0,
                    transition_duration: 0.0,
                })
                .collect(),
        },
    ));
    definitions.events.push(SoundEvent {
        name: "Play_Long".into(),
        resource_id: Some(0),
        operation: SoundOperation::Play {
            resource: Some(member(75)),
            pitch_variation: 0.0,
            stoppable: true,
            dynamic: false,
        },
        attributes: vec![],
    });
    let mut library = empty();
    library.kind = SoundDefinitionKind::Library;
    library.bindings.push(SoundDefinitionBinding {
        target: SoundDefinitionTarget::Event(member(0)),
        bank: 0,
        member: 0,
    });
    let sound = Sound {
        name: "short tone".into(),
        sample_rate: 8000,
        channels: 2,
        samples: AudioSamples::Pcm16(
            (0..256)
                .flat_map(|i| {
                    let value = if i % 40 < 20 { 7000 } else { -7000 };
                    [value, -value / 2]
                })
                .collect(),
        ),
        loop_region: None,
        resource_id: None,
        surround: None,
        fidelity_notes: vec![],
    };
    let temp = tempfile::tempdir()?;
    let selection = Selection::hype("long-music-test", "en")?;
    let mut store = crate::rete::Store::new(temp.path().join("rete"), selection.clone())?;
    store.save(&definitions_ref, &definitions)?;
    store.save(&library_ref, &library)?;
    store.save(&wave_ref, &sound)?;
    drop(store);
    let mut store = crate::rete::Store::new(temp.path().join("rete"), selection.clone())?;
    let output = temp.path().join("godot");
    let mut project = Project::new(&output, selection)?;
    let started = std::time::Instant::now();
    println!("LONG_THEME_SETUP {}", output.display());
    let event = export_event(
        &mut project,
        &mut store,
        &SoundEventLink {
            library: library_ref,
            bank: 0,
            member: 0,
        },
        &mut BTreeSet::new(),
    )?;
    println!("LONG_THEME_EXPORTED {:?}", started.elapsed());
    assert_eq!(
        project
            .generated
            .iter()
            .filter(|p| p.ends_with(".wav"))
            .count(),
        1,
        "shared original waveform; derived theme PCM is embedded in its native resource"
    );
    project.write(
        project.level_path("event.json"),
        &serde_json::to_vec(&event)?,
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let script = r#"extends SceneTree
const Audio = preload("res://addons/astrolabe/audio/emitter.gd")
func _initialize(): call_deferred("check")
func check():
    var event = load(JSON.parse_string(FileAccess.get_file_as_string("res://generated/hype/levels/long-music-test/event.json")))
    var requester = Node.new()
    root.add_child(requester)
    var music = Audio.music_channel(requester)
    var voice = event.select(music)
    var stream = voice.stream
    print("LONG_THEME_NATIVE " + JSON.stringify({"class": stream.get_class(), "format": stream.format, "stereo": stream.stereo, "rate": stream.mix_rate, "loop": stream.loop_mode, "begin": stream.loop_begin, "end": stream.loop_end, "bytes": stream.data.size()}))
    assert(stream is AudioStreamWAV and stream.format == AudioStreamWAV.FORMAT_16_BITS)
    assert(stream.stereo and stream.mix_rate == 8000)
    assert(stream.loop_mode == AudioStreamWAV.LOOP_FORWARD)
    assert(stream.loop_begin == 7 * 256 and stream.loop_end == 75 * 256)
    assert(is_equal_approx(voice.gain, 75.0 / 127.0))
    assert(stream.data.size() == 75 * 256 * 2 * 2)
    for part in 75:
        var expected = roundi(7000.0 * float(40 + part) / 127.0)
        assert(abs(stream.data.decode_s16(part * 256 * 4) - expected) <= 1, "part gain/order")
        assert(abs(stream.data.decode_s16((part * 256 + 20) * 4) + expected) <= 1, "signed PCM gain")
        var right = roundi(-3500.0 * float(40 + part) / 127.0)
        assert(abs(stream.data.decode_s16(part * 256 * 4 + 2) - right) <= 1, "stereo interleaving")
    var capture = AudioEffectCapture.new()
    capture.buffer_length = 8.0
    AudioServer.add_bus_effect(AudioServer.get_bus_index(&"Astrolabe Music"), capture)
    assert(music.request_music(event))
    var player = music.voices[0].player
    var previous = 0.0
    var wrapped = false
    var loop_position = -1.0
    var deadline = Time.get_ticks_msec() + 5000
    while Time.get_ticks_msec() < deadline and not wrapped:
        await create_timer(0.01).timeout
        var position = player.get_playback_position()
        if position + 0.05 < previous:
            wrapped = true
            loop_position = position
        previous = position
    assert(wrapped and player.playing, "native waveform did not loop")
    assert(loop_position >= 7.0 * 256.0 / 8000.0 - 0.03 and loop_position < 0.5, "native loop entry " + str(loop_position))
    var frames = capture.get_buffer(capture.get_frames_available())
    var peak = 0.0
    for frame in frames: peak = maxf(peak, maxf(absf(frame.x), absf(frame.y)))
    assert(frames.size() > 0 and peak > 0.01)
    music.kill_all()
    music.queue_free()
    requester.queue_free()
    await create_timer(0.15).timeout
    AudioServer.remove_bus(AudioServer.get_bus_index(&"Astrolabe Music"))
    print("LONG_THEME_OK " + JSON.stringify({"parts": 75, "loop_frame": stream.loop_begin, "loop_position": loop_position, "frames": frames.size(), "peak": peak}))
    quit(0)
"#;
    std::fs::write(project.stage.path().join("check_long_theme.gd"), script)?;
    let parser_log = project.stage.path().join("long-parser.log");
    crate::godot::run_godot(
        &engine,
        project.stage.path(),
        &[
            "--headless",
            "--log-file",
            parser_log
                .to_str()
                .context("non-UTF-8 long theme parser log")?,
            "--check-only",
            "--script",
            "res://check_long_theme.gd",
        ],
        std::time::Duration::from_secs(10),
    )?;
    project.finish(&engine)?;
    println!("LONG_THEME_BUILT {:?}", started.elapsed());
    std::fs::write(output.join("check_long_theme.gd"), script)?;
    let evidence = crate::godot::run_godot(
        &engine,
        &output,
        &["--headless", "--script", "res://check_long_theme.gd"],
        std::time::Duration::from_secs(10),
    )?;
    ensure!(evidence.contains("LONG_THEME_OK"), "{evidence}");
    println!("{evidence}");
    Ok(())
}

#[test]
#[ignore = "requires local sound definitions; all collected Manor music branches"]
fn collected_manor_music_themes_pass_native_shape_validation() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("disc"));
    // Unique MusicRequest bindings in the lead's complete 71-model Manor audit,
    // including alternate/revisit/credits branches, not only the finite intro.
    let requests = [
        (0, 75),
        (32, 0),
        (32, 1),
        (34, 0),
        (34, 1),
        (35, 0),
        (35, 1),
        (36, 0),
        (36, 1),
        (36, 2),
        (36, 3),
        (37, 0),
        (104, 0),
    ];
    let library_ref = AssetRef::<SoundDefinitions>::new("sound:/Sound/playmo_b.lcb")?;
    let library = crate::openspace::audio::read_sound_definitions(
        &std::fs::read_to_string(game.join(library_ref.path.file()))?,
        library_ref.path.file(),
    )?;
    for (bank, member) in requests {
        let link = SoundEventLink {
            library: library_ref.clone(),
            bank,
            member,
        };
        let SoundDefinitionTarget::Event(event_ref) = &link.binding(&library)?.target else {
            unreachable!()
        };
        let definitions = crate::openspace::audio::read_sound_definitions(
            &std::fs::read_to_string(game.join(event_ref.definitions.path.file()))?,
            event_ref.definitions.path.file(),
        )?;
        let event = definitions.event(event_ref.id)?;
        if let SoundOperation::Play {
            resource: Some(resource),
            ..
        } = &event.operation
        {
            let theme = definitions.resource(resource.id)?;
            if let SoundResourceData::Theme { parts, .. } = &theme.data {
                validate_native_theme_shape(theme, &identity(resource))?;
                for part in parts {
                    let part_definitions = crate::openspace::audio::read_sound_definitions(
                        &std::fs::read_to_string(game.join(part.resource.definitions.path.file()))?,
                        part.resource.definitions.path.file(),
                    )?;
                    let sample = part_definitions.resource(part.resource.id)?;
                    ensure!(
                        matches!(sample.data, SoundResourceData::Sample { .. }),
                        "{bank}/{member}: non-sample theme part"
                    );
                    if parts.len() > 63 {
                        ensure!(!sample.looped, "{bank}/{member}: looping long theme sample");
                    }
                }
                println!(
                    "MUSIC_BRANCH {bank}/{member} {} theme{} parts{} accepted",
                    event.name,
                    resource.id,
                    parts.len()
                );
            } else {
                println!(
                    "MUSIC_BRANCH {bank}/{member} {} non-theme {:?}",
                    event.name, theme.data
                );
            }
        } else {
            println!(
                "MUSIC_BRANCH {bank}/{member} {} {:?}",
                event.name, event.operation
            );
        }
    }
    // Verify the actual waveform closure before attempting a complete scene
    // conversion: native long-theme PCM requires uniform decoded formats.
    let links: Vec<_> = requests
        .into_iter()
        .map(|(bank, member)| SoundEventLink {
            library: library_ref.clone(),
            bank,
            member,
        })
        .collect();
    let graph = crate::openspace::audio::graph::SoundGraph::import(
        &game,
        &Selection::hype("manoir", "en")?,
        &[(library_ref.path.clone(), library.clone())],
        &links,
    )?;
    let mut source = crate::model::content::Collection::default();
    for (path, value) in graph.definitions {
        source.insert(&AssetRef::from_path(path), value)?;
    }
    for (path, value) in graph.banks {
        source.insert(&AssetRef::from_path(path), value)?;
    }
    for (path, value) in graph.sounds {
        source.insert(&AssetRef::from_path(path), value)?;
    }
    for link in &links {
        let SoundDefinitionTarget::Event(event_ref) = &link.binding(&library)?.target else {
            unreachable!()
        };
        let definitions = source.load(&event_ref.definitions)?;
        let SoundOperation::Play {
            resource: Some(resource),
            ..
        } = &definitions.event(event_ref.id)?.operation
        else {
            continue;
        };
        let theme = definitions.resource(resource.id)?;
        if let SoundResourceData::Theme { parts, .. } = &theme.data
            && parts.len() > 63
        {
            let flat = flatten_theme_samples(&mut source, theme, &identity(resource))?;
            ensure!(
                flat.frame_count() > 0 && flat.loop_region.is_some(),
                "long source theme has no native PCM/loop"
            );
            println!(
                "MUSIC_WAVE {}/{} {} frames{} rate{} channels{}",
                link.bank,
                link.member,
                theme.name,
                flat.frame_count(),
                flat.sample_rate,
                flat.channels
            );
        }
    }
    Ok(())
}
