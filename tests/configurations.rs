use anyhow::Result;
use astrolabe::{
    model::{
        AssetPath, AssetRef, Selection,
        character::*,
        io::{Content, Load, Save},
    },
    openspace::level::{ReferenceTarget, write_configuration},
    rete,
};
use std::collections::BTreeMap;

#[test]
fn configuration_reset_snapshot_edits_and_reconstructed_targets() -> Result<()> {
    let model = AssetRef::new("fix:/objects/body")?;
    let action = AssetRef::new("level:/animation-actions/sound")?;
    let part = ObjectPart {
        instance: Some(model.clone()),
        action: None,
        kind: ObjectPartKind::Model,
        scale: Some([1., 2., 3.]),
        unknown: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    };
    let mut config = ObjectList {
        path: AssetPath::new("level:/configurations/test")?,
        parts: vec![
            part.clone(),
            ObjectPart {
                instance: None,
                action: Some(action.clone()),
                kind: ObjectPartKind::Action,
                scale: None,
                unknown: [19; 10],
            },
        ],
        reset_parts: Some(vec![part.clone(), part]),
        unknown: [0xab, 0xcd],
    };
    let models = BTreeMap::from([(
        model.path.clone(),
        ReferenceTarget {
            address: 0x11000004,
            block: (6, 1),
        },
    )]);
    let actions = BTreeMap::from([(
        action.path,
        ReferenceTarget {
            address: 0x6100000c,
            block: (5, 1),
        },
    )]);
    let first = write_configuration(&config, &models, &actions, 0x70000000)?;
    let absent = ObjectPart {
        instance: None,
        action: None,
        kind: ObjectPartKind::Absent,
        scale: None,
        unknown: [0; 10],
    };
    config.parts.push(absent.clone());
    assert!(config.validate().is_err());
    config.reset_parts.as_mut().unwrap().push(absent);
    config.reset_parts.as_mut().unwrap()[0].scale = Some([4., 5., 6.]);
    let temp = tempfile::tempdir()?;
    let mut store = rete::Store::new(temp.path(), Selection::hype("configs", "en")?)?;
    let reference = AssetRef::from_path(config.path.clone());
    store.save(&reference, &config)?;
    drop(store);
    let mut store = rete::Store::new(temp.path(), Selection::hype("configs", "en")?)?;
    let loaded = store.load(&reference)?;
    assert_eq!(*loaded, config);
    let out = write_configuration(&loaded, &models, &actions, 0x70000000)?;
    assert!(out.bytes.len() > first.bytes.len());
    assert_eq!(&out.bytes[..12], &[0; 12]);
    assert_eq!(&out.bytes[20..24], &[3, 0, 0xab, 0xcd]);
    let word = |offset| u32::from_le_bytes(out.bytes[offset..offset + 4].try_into().unwrap());
    let active = (word(12) - 0x70000000) as usize;
    let reset = (word(16) - 0x70000000) as usize;
    assert_ne!(active, reset);
    assert_eq!(word(active + 4), 0x11000004);
    assert_eq!(word(active + 24), 0x6100000c);
    assert_eq!(word(reset + 24), 0x11000004);
    assert_eq!(
        &out.bytes[active + 8..active + 20],
        &[1, 2, 3, 4, 0, 0, 5, 6, 7, 8, 9, 10]
    );
    assert_eq!(&out.bytes[active + 52..active + 54], &[255, 255]);
    let scale = (word(reset) - 0x70000000) as usize;
    assert_eq!(
        &out.bytes[scale..scale + 12],
        &[4f32, 5., 6.]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>()
    );
    assert_ne!(
        word(16),
        u32::from_le_bytes(first.bytes[16..20].try_into()?)
    );
    assert!(write_configuration(&loaded, &BTreeMap::new(), &actions, 0x70000000).is_err());
    assert!(write_configuration(&loaded, &models, &actions, u32::MAX - 3).is_err());
    Ok(())
}
