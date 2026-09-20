use super::{
    container::{u32_at, wire},
    session::SourceSession,
    *,
};
use crate::model::{Selection, character::Perso};
use anyhow::Result;
use std::collections::BTreeMap;

fn list(session: &SourceSession, header: Address) -> Result<Vec<Address>> {
    let h: ListWire = wire(session.bytes(header, 12)?)?;
    let mut next = session.pointer(header, 0)?;
    let mut entries = Vec::new();
    for _ in 0..h.count {
        let entry = next.unwrap();
        entries.push(entry);
        next = session.pointer(entry, 0)?;
    }
    assert!(next.is_none());
    Ok(entries)
}

fn bind(map: &mut BTreeMap<String, Address>, name: String, address: Address) {
    if let Some(old) = map.insert(name.clone(), address) {
        assert_eq!(old, address, "{name}");
    }
}
fn compare(
    session: &SourceSession,
    source: Address,
    bytes: &[u8],
    placements: &BTreeMap<Address, u32>,
) -> Result<()> {
    assert_eq!(
        u32_at(&session.block(source)?.data, source.offset - 4)? as usize * 4 - 4,
        bytes.len()
    );
    let mut expected = session.bytes(source, bytes.len())?.to_vec();
    for field in (0..bytes.len()).step_by(4) {
        let site = session.block(source)?.base + (source.offset + field) as u32;
        if !session
            .pointers
            .contains_key(&(source.owner, source.key, site))
        {
            continue;
        }
        if let Some(target) = session.pointer(source, field)?
            && let Some(relocated) = placements.get(&target)
        {
            expected[field..field + 4].copy_from_slice(&relocated.to_le_bytes());
        }
    }
    assert_eq!(expected, bytes, "{source:?}");
    Ok(())
}

#[test]
#[ignore = "requires local game data; included by tools/test.py --local"]
fn source_character_collisions_reconstruct_and_hype_movement_matches_live_shapes() -> Result<()> {
    use crate::model::{AssetPath, character::CollisionProfile};
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let selection = Selection::hype("manoir", "en")?;
    let session = SourceSession::open(&game, &selection)?;
    let imported = import_scene(&game, &selection)?;
    let profiles = imported
        .collision_profiles
        .iter()
        .map(|p| {
            let restored: CollisionProfile = crate::rete::decode(&crate::rete::encode(p)?)?;
            assert_eq!(&restored, p);
            Ok((p.path.clone(), restored))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut sources = BTreeMap::<AssetPath, BTreeMap<String, Address>>::new();
    let mut geometry_targets = BTreeMap::new();
    let mut sets = BTreeMap::new();
    let mut uses = 0;
    for node in session.tree()? {
        if u32_at(session.bytes(node.address, 4)?, 0)? != 4 {
            continue;
        }
        let source_perso = session.required(node.address, 4)?;
        let Some(address) = session.pointer(source_perso, 24)? else {
            continue;
        };
        let perso = imported
            .persos
            .iter()
            .find(|p| p.path.file() == format!("persos/{}", node.path))
            .unwrap();
        let restored: Perso = crate::rete::decode(&crate::rete::encode(perso)?)?;
        assert_eq!(&restored, perso);
        assert!(
            sets.insert(address, restored.clone()).is_none(),
            "instance collision set unexpectedly shared"
        );
        for (i, group) in restored
            .collisions
            .as_ref()
            .unwrap()
            .groups
            .iter()
            .enumerate()
        {
            let Some(reference) = &group.profile else {
                continue;
            };
            uses += 1;
            let profile = &profiles[&reference.path];
            let map = sources.entry(reference.path.clone()).or_default();
            let mut selections = BTreeMap::new();
            if let Some(header) = session.pointer(address, i * 4)? {
                bind(map, "geometry".into(), header);
                for (j, entry) in list(&session, header)?.into_iter().enumerate() {
                    bind(map, format!("slots/{j}"), entry);
                    if let Some(geometry) = session.pointer(entry, 12)? {
                        let path = profile.geometry.as_ref().unwrap().slots[j]
                            .as_ref()
                            .unwrap()
                            .path
                            .clone();
                        geometry_targets.insert(
                            path,
                            ReferenceTarget {
                                address: session.block(geometry)?.base + geometry.offset as u32,
                                block: (geometry.key.0, geometry.key.1),
                            },
                        );
                    }
                }
            }
            if let Some(header) = session.pointer(address, 16 + i * 4)? {
                bind(map, "activations".into(), header);
                for (j, entry) in list(&session, header)?.into_iter().enumerate() {
                    bind(map, format!("activations/{j}"), entry);
                    if let Some(zones) = session.pointer(entry, 12)? {
                        let id = profile.activations.as_ref().unwrap()[j].selection.unwrap();
                        if let Some(old) = selections.insert(id, zones) {
                            assert_eq!(old, zones);
                        }
                    }
                }
            }
            if let Some(zones) = session.pointer(address, [32, 36, 44, 40][i])?
                && let Some(old) = selections.insert(group.active.unwrap(), zones)
            {
                assert_eq!(old, zones);
            }
            for (id, zones) in selections {
                bind(map, format!("selections/{id}"), zones);
                for (j, entry) in list(&session, zones)?.into_iter().enumerate() {
                    bind(map, format!("selections/{id}/zones/{j}"), entry);
                }
            }
        }
    }
    let count = sets.len();
    assert!(
        count > 10 && uses > profiles.len(),
        "fixture must exercise shared profiles"
    );
    let mut owners = BTreeMap::new();
    let mut placements = BTreeMap::new();
    let mut targets = BTreeMap::new();
    let mut outputs = BTreeMap::new();
    let mut base = 0x60000000;
    for (path, profile) in &profiles {
        let output = write_collision_profile(profile, &geometry_targets, base)?;
        let map = &sources[path];
        assert_eq!(output.fragment.ranges.len(), map.len());
        for range in &output.fragment.ranges {
            let source = map[&range.name];
            assert!(
                owners
                    .insert(source, (path.clone(), range.name.clone()))
                    .is_none(),
                "shared source collision definition duplicated across profiles"
            );
            assert!(
                placements
                    .insert(source, base + range.offset as u32)
                    .is_none()
            );
        }
        base += (output.fragment.bytes.len() as u32 + 3) & !3;
        targets.insert(path.clone(), output.targets);
        outputs.insert(path.clone(), output.fragment);
    }
    for (path, output) in outputs {
        for range in output.ranges {
            compare(
                &session,
                sources[&path][&range.name],
                &output.bytes[range.offset..range.offset + range.length],
                &placements,
            )?;
        }
    }
    for (address, perso) in &sets {
        let output = write_character_collisions(
            perso.collisions.as_ref().unwrap(),
            &targets,
            perso.path.role(),
            0x70000000,
        )?;
        assert_eq!(output.bytes.len(), 52);
        compare(&session, *address, &output.bytes, &placements)?;
    }
    let hype = imported.persos.iter().find(|p| p.name == "Hype").unwrap();
    let reference = hype.collisions.as_ref().unwrap().groups[2]
        .profile
        .as_ref()
        .unwrap();
    let movement = &profiles[&reference.path];
    let mut reordered = movement.clone();
    reordered.activations.as_mut().unwrap().swap(0, 1);
    assert!(
        write_collision_profile(&reordered, &BTreeMap::new(), 0x60000000)
            .err()
            .unwrap()
            .to_string()
            .contains("family-state order")
    );
    assert_eq!(movement.geometry.as_ref().unwrap().slots.len(), 12);
    assert_eq!(movement.activations.as_ref().unwrap().len(), 389);
    // Independent MCP capture: 250 selections in ascending order, 137 reversed;
    // states 43/51 select an empty list. No live addresses enter this fixture.
    let reversed = [
        27, 28, 29, 30, 31, 39, 45, 48, 57, 58, 59, 62, 63, 64, 71, 72, 73, 74, 75, 83, 86, 89, 91,
        92, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 116, 120, 121, 122, 125,
        126, 127, 128, 134, 135, 136, 137, 140, 141, 144, 145, 146, 147, 148, 149, 151, 152, 153,
        154, 156, 157, 158, 160, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 176,
        177, 178, 179, 180, 185, 186, 189, 190, 193, 194, 195, 198, 201, 202, 204, 207, 211, 213,
        219, 220, 221, 222, 225, 226, 228, 230, 231, 232, 233, 236, 237, 240, 242, 243, 244, 245,
        246, 247, 248, 249, 250, 251, 252, 253, 254, 255, 256, 257, 258, 259, 260, 261, 262, 263,
        264, 265, 266, 267, 268, 269,
    ];
    for activation in movement.activations.as_ref().unwrap() {
        let selected = movement
            .selections
            .iter()
            .find(|s| Some(s.id) == activation.selection)
            .unwrap();
        let expected = if [43, 51].contains(&activation.state) {
            vec![]
        } else if reversed.contains(&activation.state) {
            vec![2, 1, 0]
        } else {
            vec![0, 1, 2]
        };
        assert_eq!(
            selected.zones.iter().map(|z| z.index).collect::<Vec<_>>(),
            expected,
            "live state {}",
            activation.state
        );
    }
    for (i, expected_height) in [0.53f32, 1.1, 1.65].into_iter().enumerate() {
        let reference = movement.geometry.as_ref().unwrap().slots[i]
            .as_ref()
            .unwrap();
        let geometry = imported
            .collisions
            .iter()
            .find(|c| c.path == reference.path)
            .unwrap();
        let crate::model::collision::CollisionGeometry::Mesh(mesh) = &geometry.geometry else {
            panic!()
        };
        let crate::model::collision::CollisionElement::Spheres(spheres) = &mesh.elements[0] else {
            panic!()
        };
        assert_eq!(spheres.len(), 1);
        assert_eq!(spheres[0].radius, 0.55);
        assert_eq!(
            mesh.positions[usize::from(spheres[0].center)],
            [0., 0., expected_height]
        );
    }
    eprintln!(
        "CHARACTER_COLLISION_OK: {count} per-perso sets and {} shared profiles reconstructed through Rete; Hype's 389 movement selections and three 0.55-radius spheres match the live reference",
        imported.collision_profiles.len()
    );
    Ok(())
}
