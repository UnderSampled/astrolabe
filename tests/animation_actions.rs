use anyhow::Result;
use astrolabe::{
    model::{
        AssetPath, AssetRef, Selection,
        animation::*,
        audio::SoundEventLink,
        io::{Content, Load, Save},
    },
    openspace::level::{write_animation, write_animation_actions},
    rete,
};
use std::collections::BTreeMap;

#[test]
fn shared_animation_cues_rete_edits_and_source_sound_sites() -> Result<()> {
    let mut action = AnimationAction {
        path: AssetPath::new("fix:/animation-actions/call")?,
        operation: AnimationOperation::Sound {
            event: Some(SoundEventLink {
                library: AssetRef::new("sound:/Sound/library.lcb")?,
                bank: 7,
                member: 9,
            }),
        },
        unknown: [1, 2, 255, 4, 5, 6, 7],
    };
    let reference = AssetRef::from_path(action.path.clone());
    let cue = AnimationCue {
        action: reference.clone(),
        frame: 0,
        channel: 0,
        configuration_slot: 300,
    };
    let mut clip = AnimationClip {
        path: AssetPath::new("level:/animations/call")?,
        channel_count: 1,
        frame_rate: 30,
        speed: Default::default(),
        samples: vec![ChannelSample {
            transform: None,
            object: None,
            channel: Some(3),
            visibility: 1,
            opacity: 255,
            kind: ChannelKind::Object,
            unknown: [0; 6],
        }],
        frames: vec![
            AnimationFrame {
                samples: vec![0],
                hierarchy: vec![],
                motion_rotation: None,
                motion_translation: None
            };
            4
        ],
        cues: vec![cue.clone(), AnimationCue { frame: 2, ..cue }],
        morph_count: 0,
        unknown: vec![10, 11, 12, 13, 14, 15],
    };
    let temp = tempfile::tempdir()?;
    let selection = Selection::hype("cues", "en")?;
    let clip_reference = AssetRef::from_path(clip.path.clone());
    let mut store = rete::Store::new(temp.path(), selection.clone())?;
    store.save(&reference, &action)?;
    store.save(&clip_reference, &clip)?;
    drop(store);
    let mut store = rete::Store::new(temp.path(), selection)?;
    let loaded: std::sync::Arc<AnimationClip> = store.load(&clip_reference)?;
    assert_eq!(*loaded, clip);
    let first = store.load(&loaded.cues[0].action)?;
    let second = store.load(&loaded.cues[1].action)?;
    assert!(std::sync::Arc::ptr_eq(&first, &second));
    assert_eq!(*first, action);
    let AnimationOperation::Sound { event: Some(event) } = &mut action.operation else {
        panic!()
    };
    event.member = 19;
    clip.cues[1].frame = 3;
    let actions = write_animation_actions(&[action.clone()], 0x41000000)?;
    assert_eq!(
        &actions.bytes[..5],
        &[0, 0, 0, 0, 0],
        "must derive a placeholder instead of replaying a runtime handle"
    );
    assert_eq!(&actions.bytes[5..12], &action.unknown);
    assert_eq!(
        u32::from_le_bytes(actions.snd[..4].try_into()?),
        (7 << 16) | 19
    );
    assert_eq!(
        u32::from_le_bytes(actions.snd[4..8].try_into()?),
        actions.targets[&action.path].address
    );
    let out = write_animation(&clip, &BTreeMap::new(), &actions.targets, 0x20000000)?;
    assert_eq!(
        &out.bytes[16..20],
        &[2, 15, 0, 0],
        "event count must not occupy the morph fields"
    );
    let range = out.ranges.iter().find(|r| r.name == "cues").unwrap();
    for (i, frame) in [0, 3].into_iter().enumerate() {
        let entry = &out.bytes[range.offset + i * 8..range.offset + (i + 1) * 8];
        assert_eq!(
            u32::from_le_bytes(entry[..4].try_into()?),
            actions.targets[&action.path].address
        );
        assert_eq!(&entry[4..], &[44, 1, frame, 0]);
    }
    assert!(write_animation(&clip, &BTreeMap::new(), &BTreeMap::new(), 0x20000000).is_err());
    clip.cues[1].frame = 4;
    assert!(clip.validate().is_err());
    clip.cues[1].frame = 3;
    clip.cues[1].channel = 1;
    assert!(clip.validate().is_err());
    clip.cues[1].channel = 0;
    clip.cues.swap(0, 1);
    assert!(clip.validate().is_err());
    clip.cues.swap(0, 1);
    clip.morph_count = 1;
    assert!(write_animation(&clip, &BTreeMap::new(), &actions.targets, 0x20000000).is_err());
    Ok(())
}
