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
