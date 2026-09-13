use anyhow::Result;
use astrolabe::{
    model::{AssetPath, mechanics::MechanicsData},
    openspace::level::{read_mechanics, write_mechanics},
    rete,
};

#[test]
fn mechanics_fields_survive_rete_edits_and_unknown_variants_remain_explicit() -> Result<()> {
    let path = AssetPath::new("fix:/mechanics/character")?;
    let mut bytes = vec![0; 92];
    for (offset, value) in [(4, 120f32), (8, 0.41), (20, 50.), (24, 17453.293), (88, 2.)] {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    for offset in [28, 56, 64, 72] {
        bytes[offset..offset + 4].copy_from_slice(&[1, 0xab, 0xcd, 0xef]);
    }
    let value = read_mechanics(&bytes, path.clone())?;
    assert_eq!(write_mechanics(&value)?, bytes);
    let mut loaded =
        rete::decode::<astrolabe::model::mechanics::Mechanics>(&rete::encode(&value)?)?;
    let MechanicsData::Character { mode, settings } = &mut loaded.data else {
        panic!("character mechanics must have named parameters");
    };
    assert_eq!(*mode, 0);
    assert_eq!(settings.mass, 120.);
    assert_eq!(settings.stairs_step_height, 0.41);
    assert_eq!(settings.max_linear_speed, 50.);
    assert_eq!(settings.fsl_unknown, [0xab, 0xcd, 0xef]);
    settings.mass = 80.;
    settings.max_linear_speed = 25.;
    let rebuilt = write_mechanics(&loaded)?;
    bytes[4..8].copy_from_slice(&80f32.to_le_bytes());
    bytes[20..24].copy_from_slice(&25f32.to_le_bytes());
    assert_eq!(rebuilt, bytes);
    assert_eq!(read_mechanics(&rebuilt, path.clone())?, loaded);
    let MechanicsData::Character { settings, .. } = &mut loaded.data else {
        unreachable!()
    };
    settings.mass = f32::NAN;
    assert!(write_mechanics(&loaded).is_err());
    for (kind, length) in [(3u32, 16), (6, 92), (7, 52), (9, 8), (19, 8)] {
        let mut record = vec![0; length];
        record[..4].copy_from_slice(&kind.to_le_bytes());
        record[4..8].copy_from_slice(&17f32.to_le_bytes());
        let value = read_mechanics(&record, path.clone())?;
        if kind == 9 {
            assert_eq!(value.data, MechanicsData::Mass { mass: 17. });
        } else {
            assert!(
                matches!(&value.data, MechanicsData::Unresolved { kind: k, body } if *k == kind && body == &record[4..])
            );
        }
        let loaded =
            rete::decode::<astrolabe::model::mechanics::Mechanics>(&rete::encode(&value)?)?;
        assert_eq!(write_mechanics(&loaded)?, record);
        assert!(read_mechanics(&record[..length - 1], path.clone()).is_err());
        record.push(0);
        assert!(read_mechanics(&record, path.clone()).is_err());
    }
    assert!(read_mechanics(&2u32.to_le_bytes(), path).is_err());
    Ok(())
}
