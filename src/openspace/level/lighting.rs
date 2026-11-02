//! Resolve shared lights from Hype scene nodes and the sector's ordered lists.
use super::{
    container::wire,
    records::*,
    scene::SceneImport,
    session::{RootedNode, SourceSession},
};
use crate::model::{
    AssetPath, AssetRef,
    io::{Diagnostic, Severity},
    light::Light,
    scene::Sector,
};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

fn light(
    s: &SourceSession,
    a: Address,
    values: &mut BTreeMap<Address, Light>,
) -> Result<AssetRef<Light>> {
    if let std::collections::btree_map::Entry::Vacant(entry) = values.entry(a) {
        let role = if a.owner == Owner::Fix {
            "fix"
        } else {
            "level"
        };
        let path = AssetPath::new(format!("{role}:/lights/{}", s.allocation_index(a)?))?;
        let value = super::read_light(s.bytes(a, 200)?, path)?;
        entry.insert(value);
    }
    Ok(AssetRef::from_path(values[&a].path.clone()))
}
pub(super) fn import(
    s: &SourceSession,
    tree: &[RootedNode],
    out: &mut SceneImport,
    sounds: &super::sound::SoundSites,
) -> Result<()> {
    let mut lights = BTreeMap::new();
    let mut environments = super::environment::Import::default();
    let nodes: BTreeMap<_, _> = tree.iter().map(|n| (n.address, n.path.clone())).collect();
    for (source, node) in tree.iter().zip(&mut out.scene.nodes) {
        if let Some(a) = s.pointer(source.address, 48)? {
            node.light = Some(light(s, a, &mut lights)?);
        }
        if node.kind != crate::model::scene::NodeKind::Sector {
            continue;
        }
        let a = s.required(source.address, 4)?;
        let h: SectorWire = wire(s.bytes(a, 160)?)?;
        ensure!(
            (h.border != 0) == s.pointer(a, 0)?.is_some(),
            "sector border relocation mismatch"
        );
        let mut lists = Vec::new();
        for i in [3usize, 4] {
            let mut list = Vec::new();
            for entry in super::sector::entries(s, a, 4 + i * 12, 4)? {
                let target = s.required(entry, 0)?;
                list.push(light(s, target, &mut lights)?);
            }
            lists.push(list);
        }
        let name = if h.name == 0 {
            node.id.clone()
        } else {
            let text = s.required(a, 156)?;
            let bytes = &s.block(text)?.data[text.offset..];
            let end = bytes
                .iter()
                .take(4096)
                .position(|&b| b == 0)
                .context("unterminated sector name")?;
            bytes[..end].iter().map(|&b| char::from(b)).collect()
        };
        node.sector = Some(Sector {
            name,
            environments: super::sector::entries(s, a, 4, 4)?
                .into_iter()
                .map(|entry| environments.environment(s, s.required(entry, 0)?))
                .collect::<Result<_>>()?,
            virtual_sector: h.virtual_sector,
            activation: h.activation,
            static_lights: lists.remove(0),
            dynamic_lights: lists.remove(0),
            boundary: s
                .pointer(a, 0)?
                .map(|a| super::sector::boundary(s, a, &nodes))
                .transpose()?,
            persos: super::sector::node_list(s, a, 28, &nodes)?,
            graphics: super::sector::graphics(s, a, 76, &nodes)?,
            optional_graphics: super::sector::graphics(s, a, 88, &nodes)?,
            collision: super::sector::node_list(s, a, 100, &nodes)?,
            activity: super::sector::node_list(s, a, 112, &nodes)?,
            sound: super::sector::sound(s, a, 124, &nodes)?,
            sound_events: super::sector::sound_events(s, a, 136, &nodes, sounds)?,
            unknown: h.unknown,
        });
        let unsupported: Vec<_> = [(1, "surface"), (5, "stream")]
            .into_iter()
            .filter(|&(i, _)| h.lists[i].count > 0)
            .map(|(i, label)| format!("{label}: {}", h.lists[i].count))
            .collect();
        if !unsupported.is_empty() {
            out.diagnostics.push(Diagnostic {
                code: "openspace.sector.coverage".into(),
                asset: Some(out.scene.path.clone()),
                severity: Severity::Warning,
                message: format!(
                    "{}: sector domains requiring codecs: {}",
                    node.id,
                    unsupported.join(", ")
                ),
            });
        }
    }
    for value in lights.values() {
        if value.unresolved_dimmer {
            out.diagnostics.push(Diagnostic {
                code: "openspace.light.dimmer".into(),
                asset: Some(value.path.clone()),
                severity: Severity::Warning,
                message: "Referenced light dimmer remains undecoded; reconstruction rejects it"
                    .into(),
            });
        }
    }
    environments.finish(out);
    out.lights = lights.into_values().collect();
    Ok(())
}
