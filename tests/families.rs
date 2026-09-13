use anyhow::Result;
use astrolabe::{
    model::{
        AssetPath, AssetRef, Selection,
        character::*,
        io::{Content, Load, Save},
    },
    openspace::level::{FamilyDependencies, ReferenceTarget, write_families},
    rete,
};
use std::collections::BTreeMap;

#[test]
fn family_graph_rete_edits_identity_sharing_and_intrusive_links() -> Result<()> {
    let config = ObjectList {
        path: AssetPath::new("fix:/configurations/shared")?,
        parts: vec![],
        reset_parts: None,
        unknown: [7, 8],
    };
    let mut a = Family {
        path: AssetPath::new("fix:/families/renamed")?,
        name: "Renamed".into(),
        object_type: 37,
        bounding_geometry: Some(AssetRef::new("fix:/collision/bounds")?),
        states: vec![
            CharacterState {
                id: 7,
                animation: Some(AssetRef::new("fix:/animations/shared")?),
                mechanics: Some(AssetRef::new("fix:/mechanics/shared")?),
                speed: 30,
                next: Some(42),
                transitions: vec![StateTransition {
                    target: 42,
                    via: 7,
                    linking_type: 2,
                    unknown: [11, 12, 13],
                }],
                prohibited: vec![42],
                unknown: [1, 2, 3, 4, 5, 6, 7],
            },
            CharacterState {
                id: 42,
                animation: Some(AssetRef::new("fix:/animations/shared")?),
                mechanics: None,
                speed: 10,
                next: None,
                transitions: vec![],
                prohibited: vec![],
                unknown: [0; 7],
            },
        ],
        configurations: vec![AssetRef::from_path(config.path.clone())],
        default_configuration: Some(AssetRef::from_path(config.path.clone())),
        animation_bank: 0,
        unknown: [8, 9, 10],
    };
    let b = Family {
        path: AssetPath::new("fix:/families/second")?,
        name: "Second".into(),
        object_type: 99,
        states: vec![],
        configurations: vec![],
        ..a.clone()
    };
    let map = |path: &str, address, block| -> Result<_> {
        Ok(BTreeMap::from([(
            AssetPath::new(path)?,
            ReferenceTarget { address, block },
        )]))
    };
    let clips = map("fix:/animations/shared", 0x20000000, (5, 0))?;
    let bounds = map("fix:/collision/bounds", 0x31000000, (6, 1))?;
    let mechanics = map("fix:/mechanics/shared", 0x41000000, (6, 1))?;
    let empty = BTreeMap::new();
    let deps = FamilyDependencies {
        animations: &clips,
        physical_objects: &empty,
        actions: &empty,
        bounds: &bounds,
        mechanics: &mechanics,
        configurations: &empty,
    };
    let initial = write_families(
        &[a.clone(), b.clone()],
        std::slice::from_ref(&config),
        &deps,
        0x70000000,
    )?;
    a.states[0].speed = 15;
    a.states[0].transitions.push(StateTransition {
        target: 7,
        via: 42,
        linking_type: 4,
        unknown: [14; 3],
    });
    let temp = tempfile::tempdir()?;
    let mut store = rete::Store::new(temp.path(), Selection::hype("families", "en")?)?;
    let ar = AssetRef::from_path(a.path.clone());
    store.save(&ar, &a)?;
    store.save(&AssetRef::from_path(config.path.clone()), &config)?;
    drop(store);
    let mut store = rete::Store::new(temp.path(), Selection::hype("families", "en")?)?;
    let a = store.load(&ar)?;
    let loaded = store.load(a.default_configuration.as_ref().unwrap())?;
    assert!(std::sync::Arc::ptr_eq(
        &loaded,
        &store.load(&a.configurations[0])?
    ));
    let out = write_families(
        &[a.as_ref().clone(), b.clone()],
        std::slice::from_ref(loaded.as_ref()),
        &deps,
        0x70000000,
    )?;
    assert!(out.fragment.bytes.len() > initial.fragment.bytes.len());
    assert_eq!(out.configurations.len(), 1);
    let byte_offset = |target: ReferenceTarget| (target.address - 0x70000000) as usize;
    let word =
        |offset| u32::from_le_bytes(out.fragment.bytes[offset..offset + 4].try_into().unwrap());
    let first = byte_offset(out.families[&a.path]);
    let second = byte_offset(out.families[&b.path]);
    let state = byte_offset(out.states[&(a.path.clone(), 7)]);
    let next = out.states[&(a.path.clone(), 42)].address;
    let config = out.configurations[&loaded.path].address;
    assert_eq!(
        word(first + 12),
        37,
        "rename must not replace source object-type identity"
    );
    assert_eq!(word(first), 0x70000000 + second as u32);
    assert_eq!(word(second + 4), 0x70000000 + first as u32);
    assert_eq!(word(first + 28), config);
    assert_eq!(word(second + 28), config);
    assert_eq!(
        word(byte_offset(out.configurations[&loaded.path]) + 8),
        0x70000000 + first as u32 + 32
    );
    assert_eq!(word(state + 40), next);
    assert_eq!(word(state + 48), 0x41000000);
    assert_eq!(word(state + 12), 0x20000000);
    assert_eq!(out.fragment.bytes[state + 59], 15);
    assert_eq!(&out.fragment.bytes[state + 60..state + 64], &[4, 5, 6, 7]);
    assert_eq!(word(state + 24), 2);
    let transition = (word(state + 16) - 0x70000000) as usize;
    assert_eq!(word(transition + 12), next);
    assert_eq!(word(transition + 16), 0x70000000 + state as u32);
    let transition2 = (word(transition) - 0x70000000) as usize;
    assert_eq!(word(transition2 + 4), 0x70000000 + transition as u32);
    assert_eq!(
        &out.fragment.bytes[transition2 + 20..transition2 + 24],
        &[4, 14, 14, 14]
    );
    let prohibited = (word(state + 28) - 0x70000000) as usize;
    assert_eq!(word(prohibited + 12), next);
    let mut bad = b;
    bad.configurations = a.configurations.clone();
    assert!(
        write_families(
            &[a.as_ref().clone(), bad],
            std::slice::from_ref(loaded.as_ref()),
            &deps,
            0x70000000
        )
        .is_err()
    );
    let mut bad = a.as_ref().clone();
    bad.states[0].next = Some(999);
    assert!(bad.validate().is_err());
    Ok(())
}
