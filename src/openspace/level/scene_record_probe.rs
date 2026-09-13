//! Source-only audit before extending scene and perso reconstruction.
use super::{container::u32_at, records::*, session::SourceSession};
use anyhow::Result;
use std::collections::BTreeMap;

pub(super) fn check(session: &SourceSession, value: &super::SceneImport) -> Result<()> {
    reconstruction(session, value)?;
    let mut sizes = BTreeMap::<(&str, usize), usize>::new();
    let mut references = BTreeMap::<(&str, u32, BlockKey), usize>::new();
    let mut scene_scalars = BTreeMap::<(usize, u32), usize>::new();
    let mut aliases = BTreeMap::<(&str, bool), usize>::new();
    let mut persos = std::collections::BTreeSet::new();
    let tree = session.tree()?;
    let addresses = tree
        .iter()
        .map(|n| n.address)
        .collect::<std::collections::BTreeSet<_>>();
    for node in tree {
        let size = allocation_size(session, node.address)?;
        // HIE_fn_hCreateSuperObject (004fc660) allocates and initializes 64 bytes.
        assert_eq!(size, 64);
        audit(session, node.address, "SO", &mut sizes, &mut references)?;
        for field in [40, 44, 60] {
            let value = u32_at(session.bytes(node.address, 64)?, field)?;
            *scene_scalars.entry((field, value)).or_default() += 1;
        }
        if u32_at(session.bytes(node.address, 4)?, 0)? != 4 {
            continue;
        }
        let perso = session.required(node.address, 4)?;
        persos.insert(perso);
        let graphics = session.required(perso, 0)?;
        let standard = session.required(perso, 4)?;
        // FUN_00441da0 and FUN_00438800; historical 32/66-byte records do not apply.
        assert_eq!(allocation_size(session, graphics)?, 444);
        assert_eq!(allocation_size(session, standard)?, 56);
        let backlink = session.required(standard, 12)?;
        // Embedded local matrix at +24; +112 refers to the SO's computed matrix.
        // FUN_004425d0 / FUN_00442590 and HIE matrix accessors corroborate these aliases.
        let embedded = Address {
            offset: graphics.offset + 24,
            ..graphics
        };
        for (name, same) in [
            ("backlink_is_node", backlink == node.address),
            ("backlink_is_rooted", addresses.contains(&backlink)),
            (
                "local_is_embedded",
                session.required(node.address, 32)? == embedded,
            ),
            (
                "global_is_node",
                session.required(graphics, 112)? == session.required(node.address, 36)?,
            ),
        ] {
            *aliases.entry((name, same)).or_default() += 1;
        }
        for (a, name) in [
            (perso, "Perso"),
            (graphics, "Graphics"),
            (standard, "Standard"),
        ] {
            audit(session, a, name, &mut sizes, &mut references)?;
        }
    }
    eprintln!("Scene/Perso allocation inventory: {sizes:?}");
    assert_eq!(persos.len(), 108);
    assert_eq!(session.fix_bindings.transforms.len(), 53);
    assert_eq!(session.fix_bindings.pointers.len(), 106);
    assert_eq!(aliases.len(), 4);
    for (name, expected) in [
        ("backlink_is_node", 108),
        ("backlink_is_rooted", 108),
        ("local_is_embedded", 108),
        ("global_is_node", 108),
    ] {
        assert_eq!(aliases.get(&(name, true)), Some(&expected), "{name}");
    }
    eprintln!(
        "Scene/Perso alias inventory: {} distinct persos, {aliases:?}",
        persos.len()
    );
    eprintln!("Scene/Perso relocation-site inventory: {references:?}");
    eprintln!("SO last-compute-frame/draw-mask/transparency words: {scene_scalars:?}");
    Ok(())
}

fn reconstruction(session: &SourceSession, value: &super::SceneImport) -> Result<()> {
    use super::{ReferenceTarget, SceneDependencies, read_transform, write_scene_graph};
    use crate::model::{io::Content, scene::Scene};
    // A persisted scene drives a fresh layout. Original component addresses here
    // are test-only external placements, not part of the scene model or writer.
    let scene = crate::rete::decode::<Scene>(&crate::rete::encode(&value.scene)?)?;
    scene.validate()?;
    let tree = session.tree()?;
    let mut dependencies = SceneDependencies::default();
    let target = |a: Address| -> anyhow::Result<ReferenceTarget> {
        Ok(ReferenceTarget {
            address: session.block(a)?.base + a.offset as u32,
            block: (a.key.0, a.key.1),
        })
    };
    for (source, node) in tree.iter().zip(&scene.nodes) {
        assert_eq!(source.path, node.id);
        if let Some(a) = session.pointer(source.address, 4)? {
            dependencies.data.insert(node.id.clone(), target(a)?);
        }
        if let Some(reference) = &node.light {
            dependencies.lights.insert(
                reference.path.clone(),
                target(session.required(source.address, 48)?)?,
            );
        }
        if let Some(reference) = &node.properties.bounding_geometry {
            dependencies.bounds.insert(
                reference.path.clone(),
                target(session.required(source.address, 52)?)?,
            );
            assert!(value.collisions.iter().any(|c| c.path == reference.path));
        }
        if node.perso.is_some() {
            dependencies.local_transforms.insert(
                node.id.clone(),
                target(session.required(source.address, 32)?)?,
            );
        }
    }
    let base = 0x70000000;
    let out = write_scene_graph(&scene, &dependencies, base)?;
    let mut bounds = 0;
    for (source, node) in tree.iter().zip(&scene.nodes) {
        let p = (out.nodes[&node.id].address - base) as usize;
        let written = &out.fragment.bytes[p..p + 64];
        let original = session.bytes(source.address, 64)?;
        // Compare links by node identity under the changed layout, independently
        // of the writer's child list. This includes sibling order and null links.
        for field in [8, 12, 20, 24, 28] {
            let expected = session
                .pointer(source.address, field)?
                .map(|a| {
                    let n = tree.iter().find(|n| n.address == a).unwrap();
                    out.nodes[&n.path].address
                })
                .unwrap_or(0);
            assert_eq!(
                u32_at(written, field)?,
                expected,
                "{} link {field}",
                node.id
            );
        }
        // Remaining scalar and component-pointer words retain their values.
        for field in [0, 4, 16, 44, 48, 52, 60] {
            assert_eq!(
                u32_at(written, field)?,
                u32_at(original, field)?,
                "{} field {field}",
                node.id
            );
        }
        assert_eq!(u32_at(written, 40)?, u32::MAX - 1);
        for field in [32, 56] {
            let expected = session.transform(session.required(source.address, field)?)?;
            if field == 32 && node.perso.is_some() {
                assert_eq!(
                    u32_at(written, 32)?,
                    target(session.required(source.address, 32)?)?.address
                );
            } else {
                let p = (u32_at(written, field)? - base) as usize;
                assert_eq!(
                    read_transform(&out.fragment.bytes[p..])?.affine(),
                    expected,
                    "{} matrix {field}",
                    node.id
                );
            }
        }
        bounds += usize::from(node.properties.bounding_geometry.is_some());
    }
    assert_eq!(bounds, 145);
    eprintln!(
        "Scene reconstruction: {} full SO records; {} picking-bound references/{} shared assets; hierarchy order, component links, opacity, source kinds and initial matrices verified under a new layout",
        scene.nodes.len(),
        bounds,
        dependencies.bounds.len()
    );
    Ok(())
}

fn allocation_size(session: &SourceSession, a: Address) -> Result<usize> {
    session.allocation_index(a)?;
    Ok(u32_at(&session.block(a)?.data, a.offset - 4)? as usize * 4 - 4)
}

fn audit(
    session: &SourceSession,
    a: Address,
    name: &'static str,
    sizes: &mut BTreeMap<(&'static str, usize), usize>,
    references: &mut BTreeMap<(&'static str, u32, BlockKey), usize>,
) -> Result<()> {
    let size = allocation_size(session, a)?;
    *sizes.entry((name, size)).or_default() += 1;
    let start = session.block(a)?.base + a.offset as u32;
    for ((_, _, site), target) in session
        .pointers
        .range((a.owner, a.key, start)..(a.owner, a.key, start + size as u32))
    {
        *references.entry((name, site - start, *target)).or_default() += 1;
    }
    Ok(())
}
