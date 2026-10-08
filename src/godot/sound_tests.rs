use super::*;
use crate::model::{Selection, io::Load};

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
