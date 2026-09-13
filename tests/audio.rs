use astrolabe::{
    model::audio::{AudioReconstructionGap, AudioSamples, Sound, SoundBank},
    openspace::audio::{read_apm, read_bnm, write_apm, write_bnm},
};
fn fixture() -> Sound {
    Sound {
        name: "edited.wav".into(),
        sample_rate: 22050,
        channels: 2,
        samples: AudioSamples::Pcm16(vec![
            0, 500, 1, 501, 4, 504, 8, 508, 12, 512, 16, 516, 20, 520,
        ]),
        loop_region: None,
        resource_id: Some(42),
        surround: None,
        fidelity_notes: vec![],
    }
}
#[test]
fn apm_writer_derives_header_and_quantizes_current_samples() {
    let mut sound = fixture();
    let bytes = write_apm(&sound).unwrap();
    let decoded = read_apm(&bytes, "edited.wav").unwrap();
    assert_eq!(decoded.sample_rate, 22050);
    assert_eq!(decoded.channels, 2);
    assert_eq!(decoded.frame_count(), 7);
    assert_eq!(decoded.samples.len(), 14);
    let samples = decoded.interleaved_i16();
    assert!(
        samples
            .iter()
            .zip(sound.interleaved_i16())
            .all(|(a, b)| (i32::from(*a) - i32::from(b)).abs() <= 3)
    );
    let old_size = bytes.len();
    sound.sample_rate = 11025;
    // A deliberately resized clip exercises generated sizes and data placement.
    sound.samples = AudioSamples::Pcm16((0..20).flat_map(|i| [1000 + i, -1000 - i]).collect());
    let changed = write_apm(&sound).unwrap();
    assert_ne!(changed.len(), old_size);
    assert_eq!(read_apm(&changed, "new").unwrap().frame_count(), 20);
    assert_eq!(u32::from_le_bytes(changed[4..8].try_into().unwrap()), 11025);
}
#[test]
fn pcm_bank_roundtrip_rete_and_size_change() {
    let mut bank = SoundBank {
        name: "test".into(),
        sounds: vec![fixture()],
        external_sounds: vec![],
        events: vec![],
        fidelity_notes: vec![],
        reconstruction_gaps: vec![],
    };
    let bytes = write_bnm(&bank).unwrap();
    let decoded = read_bnm(&bytes, "test").unwrap();
    assert_eq!(decoded, bank);
    let stored = astrolabe::rete::encode(&decoded).unwrap();
    let mut loaded: SoundBank = astrolabe::rete::decode(&stored).unwrap();
    assert_eq!(write_bnm(&loaded).unwrap(), bytes);
    loaded.sounds[0].samples = AudioSamples::Pcm16(vec![321, -123, 322, -124]);
    loaded.sounds.push(Sound {
        name: "second.wav".into(),
        resource_id: Some(100),
        ..fixture()
    });
    let edited = write_bnm(&loaded).unwrap();
    assert_eq!(read_bnm(&edited, "test").unwrap(), loaded);
    bank.fidelity_notes
        .push("an informational observation".into());
    assert_eq!(write_bnm(&bank).unwrap(), bytes);
    bank.reconstruction_gaps
        .push(AudioReconstructionGap::BinaryEvents { count: 1 });
    bank.fidelity_notes.clear();
    let bank: SoundBank =
        astrolabe::rete::decode(&astrolabe::rete::encode(&bank).unwrap()).unwrap();
    assert!(write_bnm(&bank).is_err());
}
#[test]
fn audio_rejects_truncated_and_out_of_bounds_payloads() {
    let apm = write_apm(&fixture()).unwrap();
    for end in [0, 4, 20, 44, 96, 99, apm.len() - 1] {
        assert!(read_apm(&apm[..end], "bad").is_err());
    }
    let mut invalid = apm.clone();
    invalid[0x30..0x34].copy_from_slice(&89i32.to_le_bytes());
    assert!(read_apm(&invalid, "bad predictor").is_err());
    let bank = SoundBank {
        name: "test".into(),
        sounds: vec![fixture()],
        external_sounds: vec![],
        events: vec![],
        fidelity_notes: vec![],
        reconstruction_gaps: vec![],
    };
    let mut bytes = write_bnm(&bank).unwrap();
    bytes[0x0c..0x10].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(read_bnm(&bytes, "bad").is_err());
}
#[test]
#[ignore = "requires local disc; run under the overall suite deadline"]
fn local_audio_representative_fixtures() {
    let disc = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("disc"));
    let root = disc.join("Gamedata/World/Sound");
    let mut apm_count = 0;
    for entry in std::fs::read_dir(&root).unwrap() {
        let path = entry.unwrap().path();
        if path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.eq_ignore_ascii_case("apm"))
        {
            let sound = read_apm(
                &std::fs::read(&path).unwrap(),
                path.file_name().unwrap().to_string_lossy(),
            )
            .unwrap_or_else(|e| panic!("{}: {e:#}", path.display()));
            assert!(sound.frame_count() > 0 && sound.duration_seconds() > 0.0);
            // Reference hashes were produced by the archived C# ApmReader and
            // ImaAdpcmDecoder, compiled independently under /tmp without edits.
            let expected = match path.file_name().unwrap().to_str().unwrap() {
                "Achat.apm" => {
                    Some("754f5dd96f06172b0f566d0990d83040cef71c43725a031d8909018239c20a83")
                }
                "AmbSilen.apm" => {
                    Some("565b44aa2bc34b742b4ec996d1c2e760b1a43ef308dbd32bd0bfe646103585df")
                }
                "AmbCiel.apm" => {
                    Some("2a9b87676e8dcbd2e93ff32e19cc1315c6b4d90a1864a74402fab30f329bfa30")
                }
                _ => None,
            };
            if let Some(expected) = expected {
                use sha2::{Digest, Sha256};
                let bytes: Vec<u8> = sound
                    .interleaved_i16()
                    .into_iter()
                    .take(4096)
                    .flat_map(i16::to_le_bytes)
                    .collect();
                assert_eq!(format!("{:x}", Sha256::digest(&bytes)), expected);
            }
            let bytes = astrolabe::rete::encode(&sound).unwrap();
            let restored: Sound = astrolabe::rete::decode(&bytes).unwrap();
            assert_eq!(sound, restored);
            apm_count += 1;
        }
    }
    assert_eq!(
        apm_count, 39,
        "local fixture corpus changed; inspect coverage"
    );
    for name in ["Bnk_0.bnm", "Bnk_1.bnm", "English/Bnk_155.bnm"] {
        let bank = read_bnm(&std::fs::read(root.join(name)).unwrap(), name)
            .unwrap_or_else(|e| panic!("{name}: {e:#}"));
        assert!(!bank.sounds.is_empty());
        assert!(bank.sounds.iter().all(|s| s.frame_count() > 0));
        let packed = astrolabe::rete::encode(&bank).unwrap();
        let restored: SoundBank = astrolabe::rete::decode(&packed).unwrap();
        assert_eq!(restored, bank);
        eprintln!(
            "{name}: {} embedded sounds, {} external, {} surround",
            bank.sounds.len(),
            bank.external_sounds.len(),
            bank.sounds.iter().filter(|s| s.surround.is_some()).count()
        );
    }
}
#[test]
fn named_sound_definitions_are_useful_and_editable() {
    use astrolabe::openspace::audio::{read_sound_definitions, write_sound_definitions};
    let text = "{SIF_Type:Material[%ld](1)\n{SIF_Value:Stone\nSetValue[%ld](3)\n}\n}\n";
    let mut definitions = read_sound_definitions(text, "materials.sif").unwrap();
    assert_eq!(definitions.selectors[0].name, "Material");
    assert_eq!(definitions.selectors[0].values[0], ("Stone".into(), 3));
    definitions.selectors[0].values.push(("Wood".into(), 4));
    let rebuilt = write_sound_definitions(&definitions).unwrap();
    assert_eq!(
        read_sound_definitions(&rebuilt, "materials.sif").unwrap(),
        definitions
    );
    assert!(read_sound_definitions("{LcbHeader:\n", "invalid.lcb").is_err());
    let mut library = read_sound_definitions("{LcbHeader:\nSetDefaultLanguage(English)\n}\n{SndEventIdList:\nSetEvent(voice.csb^SndEventM:7,0,2)\n}\n", "library.lcb").unwrap();
    library.bindings[0].target.member_mut().definitions =
        astrolabe::model::AssetRef::new("sound:/Sound/English/voice.csb").unwrap();
    let written = write_sound_definitions(&library).unwrap();
    let decoded = read_sound_definitions(&written, "library.lcb").unwrap();
    assert_eq!(
        decoded.bindings[0]
            .target
            .member()
            .definitions
            .path
            .as_str(),
        "sound:/Sound/voice.csb"
    );
    assert_eq!(decoded.bindings[0].member, 2);
}

#[test]
fn sound_operations_preserve_event_targets_and_quoted_source_arguments() {
    use astrolabe::model::audio::*;
    use astrolabe::openspace::audio::read_sound_definitions;
    let text = r#"{CsbHeader:
}
{SndEventM:0(EVT_SON_PLAY)
SetParam1[ar]("test.csb^SndResourceM:9",n)
SetParam2[%lf]("0.125")
SetStopable[ab](True)
SetDynamic[ab](False)
}
{SndEventM:1(EVT_SON_STOP)
SetParam1(test.csb^SndEventM:0)
}
{SndResourceM:9
{SndResourceDisk:9(TYPE_RANDOM)
SetProbNothing[%lf]("0.1")
SetNothingCanBeChoosenTwice[ab](False)
}
{SndRandomListM:
{SndRandomElementM:0
LoadResource("test.csb^SndResourceM:10")
SetProbability[%lf]("0.9")
SetElementCanBeChoosenTwice[ab](True)
}
}
}
"#;
    let value = read_sound_definitions(text, "test.csb").unwrap();
    value.validate().unwrap();
    let SoundOperation::Play {
        resource: Some(target),
        pitch_variation,
        stoppable,
        dynamic,
    } = &value.events[0].operation
    else {
        panic!()
    };
    assert_eq!(target.id, 9);
    assert_eq!(*pitch_variation, 0.125);
    assert!(*stoppable && !dynamic);
    let SoundOperation::Stop {
        event: Some(target),
    } = &value.events[1].operation
    else {
        panic!()
    };
    assert_eq!(
        target.id, 0,
        "stop identifies the playing event, not its resource"
    );
    let SoundResourceData::Random {
        choices,
        silence_probability,
        ..
    } = &value.resources[0].data
    else {
        panic!()
    };
    assert_eq!(choices[0].resource.id, 10);
    assert_eq!(*silence_probability, 0.1);
    assert!(choices[0].repeat);
    let restored: SoundDefinitions =
        astrolabe::rete::decode(&astrolabe::rete::encode(&value).unwrap()).unwrap();
    assert_eq!(restored, value);
    assert!(
        read_sound_definitions(
            &text.replace("SndEventM:0)", "SndResourceM:0)"),
            "wrong.csb"
        )
        .is_err()
    );
}
#[test]
#[ignore = "requires local disc; run under the overall suite deadline"]
fn local_audio_full_corpus() {
    use astrolabe::openspace::audio::{read_sound_definitions, write_sound_definitions};
    let disc = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("disc"));
    let root = disc.join("Gamedata/World/Sound");
    let mut counts = (0, 0, 0, 0);
    for dir in [root.clone(), root.join("English")] {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.eq_ignore_ascii_case("bnm"))
            {
                let bank = read_bnm(
                    &std::fs::read(&path).unwrap(),
                    path.file_name().unwrap().to_string_lossy(),
                )
                .unwrap_or_else(|e| panic!("{}: {e:#}", path.display()));
                bank.validate().unwrap();
                counts.0 += 1;
                counts.1 += bank.sounds.len();
                counts.2 += bank.external_sounds.len();
                counts.3 += bank.sounds.iter().filter(|s| s.surround.is_some()).count();
                assert!(bank.sounds.iter().all(|s| !s.samples.is_empty()));
            }
        }
    }
    assert_eq!(counts.1, 2573);
    assert_eq!(counts.2, 224);
    eprintln!(
        "BNM corpus: {} banks, {} embedded, {} external, {} surround",
        counts.0, counts.1, counts.2, counts.3
    );
    let mut scripts = 0;
    for dir in [disc.join("Sound"), disc.join("Sound/English")] {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|s| ["csb", "lcb", "sif"].contains(&s.to_ascii_lowercase().as_str()))
            {
                let bytes = std::fs::read(&path).unwrap();
                let text: String = bytes.iter().map(|&b| char::from(b)).collect();
                let name = path.file_name().unwrap().to_string_lossy();
                let definitions = read_sound_definitions(&text, name.as_ref())
                    .unwrap_or_else(|e| panic!("{}: {e:#}", path.display()));
                let stored = astrolabe::rete::encode(&definitions)
                    .unwrap_or_else(|e| panic!("{} persistence: {e:#}", path.display()));
                let restored: astrolabe::model::audio::SoundDefinitions =
                    astrolabe::rete::decode(&stored).unwrap();
                assert_eq!(restored, definitions);
                if !matches!(
                    definitions.kind,
                    astrolabe::model::audio::SoundDefinitionKind::Events
                ) {
                    let rebuilt = write_sound_definitions(&definitions)
                        .unwrap_or_else(|e| panic!("{}: {e:#}", path.display()));
                    assert_eq!(
                        read_sound_definitions(&rebuilt, name.as_ref()).unwrap(),
                        definitions
                    );
                }
                if name == "M_Astrolabe.csb" {
                    assert_eq!(definitions.events.len(), 8);
                    assert_eq!(definitions.events[0].name, "Play_Rajintro");
                    assert!(!definitions.resources.is_empty());
                }
                scripts += 1;
            }
        }
    }
    eprintln!("Sound script corpus: {scripts} definitions files");
}

#[test]
fn apm_loop_state_is_generated_for_an_edited_odd_frame_loop() {
    let mut sound = fixture();
    sound.loop_region = Some(astrolabe::model::audio::SampleLoop {
        start_frame: 1,
        end_frame: 7,
    });
    let bytes = write_apm(&sound).unwrap();
    let restored = read_apm(&bytes, "loop").unwrap();
    assert_eq!(restored.loop_region, sound.loop_region);
    assert_eq!(
        u32::from_le_bytes(bytes[68..72].try_into().unwrap()),
        1,
        "loop resumes at the low nibble"
    );
    let samples = restored.interleaved_i16();
    // Header loop state stores right then left. At frame one each predictor
    // must be the first decoded sample of its own channel, not the initial state.
    assert_eq!(
        i32::from_le_bytes(bytes[72..76].try_into().unwrap()),
        i32::from(samples[1])
    );
    assert_eq!(
        i32::from_le_bytes(bytes[84..88].try_into().unwrap()),
        i32::from(samples[0])
    );
    let saved = astrolabe::rete::encode(&restored).unwrap();
    let loaded: Sound = astrolabe::rete::decode(&saved).unwrap();
    assert_eq!(loaded.loop_region, sound.loop_region);
}
