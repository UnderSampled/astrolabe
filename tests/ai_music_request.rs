//! Source music/fade operands through typed AI, Rete and native Godot audio.
use anyhow::{Result, ensure};
use astrolabe::{
    godot,
    model::{
        AssetPath, AssetRef, Selection,
        ai::*,
        audio::*,
        io::{Content, ContentRoot, Load, Save},
    },
    openspace::{ai::AiScriptCodec, codec::FormatCodec},
    rete,
};
use std::{fs, time::Duration};

fn event(member: u16) -> Result<SoundEventLink> {
    Ok(SoundEventLink {
        library: AssetRef::new("sound:/Sound/music.lcb")?,
        bank: 36,
        member,
    })
}
fn model() -> Result<AiModel> {
    Ok(AiModel {
        path: AssetPath::new("level:/ai/music")?,
        name: "Music request fixture".into(),
        normal: Some(BehaviorSet {
            behaviors: vec![Behavior {
                id: 0,
                rules: vec![],
                schedule: Some(Script {
                    statements: vec![
                        Statement::SoundFadeMusic {
                            arg0: Expression::Integer(127),
                            arg1: Expression::Integer(1),
                        },
                        Statement::MusicRequest {
                            event: Some(event(0)?),
                        },
                        Statement::MusicRequest { event: None },
                        Statement::SoundFadeInObject {
                            actor: ActorTarget::Keyword(ActorKeyword::MainActor),
                            target: Expression::Variable(0),
                            duration_ms: Expression::Variable(1),
                            event: Some(event(1)?),
                        },
                        Statement::SoundOnActor {
                            actor: ActorTarget::Keyword(ActorKeyword::MainActor),
                            event: Some(event(1)?),
                        },
                        Statement::Assign {
                            variable: 2,
                            value: Expression::Integer(1),
                        },
                        Statement::WaitFrames(Expression::Integer(1000)),
                    ],
                }),
                unknown: [0; 3],
            }],
            default_behavior: Some(0),
            action_capacity: 0,
            unknown: [0; 3],
        }),
        reflex: None,
        variables: [90, 80, 0]
            .into_iter()
            .map(|v| DesignerVariable {
                default: DesignerValue::Integer(v),
                save: 0,
                initialize: 0,
            })
            .collect(),
        designer_unknown: [0; 3],
        unknown: [0; 4],
    })
}

#[test]
fn music_and_ultra_object_fade_preserve_source_operands_and_dependencies() -> Result<()> {
    let model = model()?;
    let mut codec = AiScriptCodec {
        sound_handles: vec![
            (Some(event(0)?), 100),
            (None, u32::MAX),
            (Some(event(1)?), 101),
        ],
        ..Default::default()
    };
    let script = model.normal.as_ref().unwrap().behaviors[0]
        .schedule
        .as_ref()
        .unwrap();
    let bytes = codec.write(script)?;
    for (i, node) in bytes.as_chunks::<8>().0.iter().enumerate() {
        if node[7] == 34 {
            let handle = u32::from_le_bytes(node[..4].try_into()?);
            let binding = codec
                .sound_handles
                .iter()
                .find(|(_, h)| *h == handle)
                .unwrap()
                .0
                .clone();
            codec.sounds.insert(i, binding);
        }
    }
    assert_eq!(codec.read(&bytes)?, *script);
    let reopened = rete::decode::<AiModel>(&rete::encode(&model)?)?;
    assert_eq!(reopened, model);
    assert_eq!(
        reopened.sound_events(),
        [&event(0)?, &event(1)?, &event(1)?]
    );
    let code = godot::ai::compile(
        &model,
        &[
            (event(0)?, "res://music.tres".into()),
            (event(1)?, "res://effects.tres".into()),
        ],
    )?;
    assert!(code.contains("send_music(sound_0)"));
    assert!(code.contains("send_music(null)"));
    assert!(code.contains("var sound_receiver_0: Node3D = main_actor"));
    assert_eq!(code.matches("int(dsg_0)").count(), 1);
    assert_eq!(code.matches("int(dsg_1)").count(), 1);
    assert!(code.find("var sound_level_0").unwrap() < code.find("var sound_duration_0").unwrap());
    assert!(!code.contains("Proc_SendMusicRequest requires"));
    assert!(
        godot::ai::compile(&model, &[]).is_err(),
        "resolved events must be exported before compilation"
    );
    Ok(())
}

#[test]
#[ignore = "requires native Godot; focused audio selection"]
fn exported_music_and_object_fade_continue_ai_and_mix_native_audio() -> Result<()> {
    let model = model()?;
    let definitions_ref = AssetRef::<SoundDefinitions>::new("sound:/Sound/music.csb")?;
    let library_ref = event(0)?.library;
    let wave = AssetRef::<Sound>::new("sound:/synthetic/tone")?;
    let member = |id| SoundMemberRef {
        definitions: definitions_ref.clone(),
        id,
    };
    let empty = || SoundDefinitions {
        name: "music".into(),
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
    let sample = |id, volume, looped| SoundResourceDefinition {
        id,
        name: format!("tone{id}"),
        data: SoundResourceData::Sample {
            sample: Some(SoundSampleRef::Sound(wave.clone())),
            controls: SoundSampleControls {
                pitchable: false,
                volumeable: true,
                pannable: true,
                spatial: true,
                reverb: false,
            },
        },
        file: None,
        volume: Some(volume),
        looped,
        loop_region: None,
        sample_rate: Some(8000),
        channels: Some(1),
    };
    let mut definitions = empty();
    definitions.resources = vec![
        SoundResourceDefinition {
            id: 0,
            name: "Intro2-shaped theme".into(),
            data: SoundResourceData::Theme {
                start_loop: 0,
                main_loops: 0,
                parts: [1, 2, 2]
                    .into_iter()
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
            file: None,
            volume: Some(75.0),
            looped: true,
            loop_region: None,
            sample_rate: None,
            channels: None,
        },
        sample(1, 127.0, false),
        sample(2, 48.0, false),
        sample(3, 127.0, true),
    ];
    definitions.events = [0, 3]
        .into_iter()
        .enumerate()
        .map(|(id, resource)| SoundEvent {
            name: format!("play{id}"),
            resource_id: Some(id as u32),
            operation: SoundOperation::Play {
                resource: Some(member(resource)),
                pitch_variation: 0.0,
                stoppable: true,
                dynamic: true,
            },
            attributes: vec![],
        })
        .collect();
    let mut library = empty();
    library.kind = SoundDefinitionKind::Library;
    library.bindings = (0..2)
        .map(|id| SoundDefinitionBinding {
            target: SoundDefinitionTarget::Event(member(id)),
            bank: 36,
            member: id,
        })
        .collect();
    let sound = Sound {
        name: "synthetic".into(),
        sample_rate: 8000,
        channels: 1,
        samples: AudioSamples::Pcm16(
            (0..1600)
                .map(|i| if i % 40 < 20 { 7000 } else { -7000 })
                .collect(),
        ),
        loop_region: None,
        resource_id: None,
        surround: None,
        fidelity_notes: vec![],
    };
    let temp = tempfile::tempdir()?;
    let selection = Selection::hype("music-test", "en")?;
    let model_ref = AssetRef::<AiModel>::from_path(model.path.clone());
    let mut store = rete::Store::new(temp.path().join("rete"), selection.clone())?;
    store.save(&model_ref, &model)?;
    store.save(&definitions_ref, &definitions)?;
    store.save(&library_ref, &library)?;
    store.save(&wave, &sound)?;
    drop(store);
    let mut store = rete::Store::new(temp.path().join("rete"), selection.clone())?;
    assert_eq!(*store.load(&model_ref)?, model);
    let project = temp.path().join("godot");
    godot::support::initialize(&project)?;
    fs::write(
        project.join("check_ai_music_request.gd"),
        include_bytes!("check_ai_music_request.gd"),
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    godot::run_godot(
        &engine,
        &project,
        &[
            "--headless",
            "--log-file",
            "music-parser.log",
            "--check-only",
            "--script",
            "res://check_ai_music_request.gd",
        ],
        Duration::from_secs(10),
    )?;
    let manifest = godot::export(
        &mut store,
        &[ContentRoot {
            kind: AiModel::KIND.into(),
            reference: model.path.clone(),
        }],
        selection,
        &project,
        &engine,
    )?;
    ensure!(
        manifest
            .files
            .keys()
            .any(|p| p.ends_with("music.csb.streams/0.tres")),
        "theme dependency missing"
    );
    let evidence = godot::run_godot(
        &engine,
        &project,
        &[
            "--headless",
            "--log-file",
            "music-behavior.log",
            "--script",
            "res://check_ai_music_request.gd",
        ],
        Duration::from_secs(20),
    )?;
    ensure!(evidence.contains("AI_MUSIC_REQUEST_OK"), "{evidence}");
    println!("{evidence}");
    Ok(())
}

#[test]
fn named_audio_receivers_and_null_events_reconstruct_source_and_rete() -> Result<()> {
    use astrolabe::model::character::Perso;
    let actor = AssetRef::<Perso>::new("level:/persos/world/0/294")?;
    let script = Script {
        statements: vec![
            Statement::SoundOnActor {
                actor: ActorTarget::Reference(actor.clone()),
                event: None,
            },
            Statement::SoundFadeInObject {
                actor: ActorTarget::Reference(actor.clone()),
                target: Expression::Integer(90),
                duration_ms: Expression::Integer(80),
                event: None,
            },
            Statement::SoundFadeInObject {
                actor: ActorTarget::Keyword(ActorKeyword::SelfActor),
                target: Expression::Integer(127),
                duration_ms: Expression::Integer(1),
                event: Some(event(0)?),
            },
        ],
    };
    let mut codec = AiScriptCodec {
        actor_handles: vec![(actor.clone(), 1234)],
        sound_handles: vec![(None, u32::MAX), (Some(event(0)?), 100)],
        ..Default::default()
    };
    let bytes = codec.write(&script)?;
    for (i, node) in bytes.as_chunks::<8>().0.iter().enumerate() {
        match node[7] {
            24 => {
                codec.actors.insert(i, actor.clone());
            }
            34 => {
                let handle = u32::from_le_bytes(node[..4].try_into()?);
                codec.sounds.insert(
                    i,
                    codec
                        .sound_handles
                        .iter()
                        .find(|(_, h)| *h == handle)
                        .unwrap()
                        .0
                        .clone(),
                );
            }
            _ => {}
        }
    }
    assert_eq!(codec.read(&bytes)?, script);
    let mut model = model()?;
    model.normal.as_mut().unwrap().behaviors[0].schedule = Some(script);
    assert_eq!(rete::decode::<AiModel>(&rete::encode(&model)?)?, model);
    let generated = godot::ai::compile(&model, &[(event(0)?, "res://event.tres".into())])?;
    assert!(generated.contains("character().source_actor_node(\"level:/persos/world/0/294\")"));
    assert!(generated.contains("send_sound_to_actor(sound_receiver_0, null)"));
    assert!(
        generated.contains(
            "sound_fade_in_object(sound_receiver_1, sound_level_1, sound_duration_1, null)"
        )
    );
    Ok(())
}
