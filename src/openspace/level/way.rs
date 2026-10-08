//! Bounded source observations for Hype's actor-local routes. These are research
//! projections, not persisted assets or a Godot route interpreter.
use super::{container::wire, records::*, session::SourceSession};
use crate::model::{
    AssetPath, AssetRef, Selection, ai::SourceObjectIdentity, io::Content, route::*,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorWayInspection {
    pub actor: AssetPath,
    pub object_types: [u32; 3],
    /// Script category 23 selects this actor-local slot, including empty slots.
    pub slots: Vec<Option<WayInspection>>,
    pub progress: Vec<[u8; 4]>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WayInspection {
    pub source: SourceObjectIdentity,
    pub points: Vec<WaypointInspection>,
    /// There are N-1 links for N points, rather than one link per point.
    pub links: Vec<Option<WayLinkInspection>>,
    pub current: u8,
    pub previous: u8,
    pub unknown: u8,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaypointInspection {
    pub source: SourceObjectIdentity,
    pub position: [f32; 3],
    pub radius: f32,
    /// Non-null points are relative to the referenced actor's world transform.
    pub actor: Option<SourceObjectIdentity>,
    pub unknown: [u8; 4],
    pub tail: [u8; 4],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WayLinkInspection {
    pub source: SourceObjectIdentity,
    /// Geometry remains an explicit unpromoted reference; do not traverse it.
    pub geometry: Option<SourceObjectIdentity>,
    pub static_endpoint: Option<WaypointInspection>,
    pub connection: u32,
    pub dynamic_kind: u32,
    pub sample: u8,
    pub used: u8,
    pub length: f32,
    pub unknown: [u8; 2],
    /// The 52-byte DynamicObject value is retained while its semantics are mapped.
    pub dynamics: Vec<u8>,
}

/// Inspect one rooted actor without interpreting its AI or following opaque geometry.
/// Source addresses/relocations stay private to the OpenSpace boundary.
pub fn inspect_actor_ways(
    game: &Path,
    selection: &Selection,
    actor: &AssetPath,
) -> Result<ActorWayInspection> {
    let node_path = actor
        .as_str()
        .strip_prefix("level:/persos/")
        .context("route selector must be a rooted level actor")?;
    let session = SourceSession::open(game, selection)?;
    let node = session
        .tree()?
        .into_iter()
        .find(|n| n.path == node_path)
        .context("route actor is absent from the rooted scene")?;
    let node_wire: SuperObjectWire = wire(session.bytes(node.address, 64)?)?;
    ensure!(node_wire.kind == 4, "route selector is not a Perso");
    let perso = session.required(node.address, 4)?;
    let parts =
        super::source_view::perso_parts(&mut super::source_view::FileView(&session), perso)?;
    read_actor_ways(&session, perso, actor.clone(), parts.object_types)
}

pub(super) fn read_actor_ways(
    s: &SourceSession,
    perso: Address,
    actor: AssetPath,
    object_types: [u32; 3],
) -> Result<ActorWayInspection> {
    let _: PersoWaysWire = wire(s.bytes(perso, 36)?)?;
    let mut value = ActorWayInspection {
        actor,
        object_types,
        slots: Vec::new(),
        progress: Vec::new(),
    };
    let Some(component) = s.pointer(perso, 32)? else {
        return Ok(value);
    };
    // Only this documented first word of the component is inspected.
    let _: u32 = wire(s.bytes(component, 4)?)?;
    let Some(header) = s.pointer(component, 0)? else {
        return Ok(value);
    };
    let list: WayListWire = wire(s.bytes(header, 12)?)?;
    ensure!(list.count <= 256, "actor route count exceeds bound");
    let mut entry = s.pointer(header, 0)?;
    let mut seen = BTreeSet::new();
    let mut previous = None;
    for _ in 0..list.count {
        let binding = entry.context("short actor route list")?;
        ensure!(seen.insert(binding), "actor route list cycles");
        let b: WayBindingWire = wire(s.bytes(binding, 20)?)?;
        ensure!(
            s.pointer(binding, 8)? == Some(header) && s.pointer(binding, 4)? == previous,
            "actor route list ownership or previous link differs"
        );
        value.progress.push(b.progress);
        value.slots.push(
            s.pointer(binding, 12)?
                .map(|route| read_route(s, route))
                .transpose()?,
        );
        previous = Some(binding);
        entry = s.pointer(binding, 0)?;
    }
    ensure!(
        entry.is_none() && s.pointer(header, 4)? == previous,
        "actor route count or tail differs"
    );
    Ok(value)
}

fn read_route(s: &SourceSession, route: Address) -> Result<WayInspection> {
    let h: WayWire = wire(s.bytes(route, 12)?)?;
    let mut value = WayInspection {
        source: s.source_identity(route)?,
        points: Vec::new(),
        links: Vec::new(),
        current: h.current,
        previous: h.previous,
        unknown: h.unknown,
    };
    if h.count > 0 {
        let pp = s.required(route, 0)?;
        s.bytes(pp, h.count as usize * 4)?;
        for index in 0..h.count as usize {
            let p = s.required(pp, index * 4)?;
            value.points.push(read_point(s, p)?);
        }
    }
    if h.count > 1 {
        let lp = s.required(route, 4)?;
        s.bytes(lp, (h.count as usize - 1) * 4)?;
        for index in 0..h.count as usize - 1 {
            value.links.push(
                s.pointer(lp, index * 4)?
                    .map(|a| -> Result<_> {
                        let raw: WayLinkWire = wire(s.bytes(a, 72)?)?;
                        ensure!(
                            raw.length.is_finite() && raw.length >= 0.0,
                            "invalid route link length"
                        );
                        Ok(WayLinkInspection {
                            source: s.source_identity(a)?,
                            geometry: s.pointer(a, 0)?.map(|a| s.source_identity(a)).transpose()?,
                            static_endpoint: if raw.connection == 0 {
                                s.pointer(a, 0)?.map(|geometry| -> Result<_> {
                                    let _: StaticLineWire = wire(s.bytes(geometry, 8)?)?;
                                    let point = read_point(s, s.required(geometry, 0)?)?;
                                    // A derived world-position cache is read only after
                                    // promoting the object. It is not persisted as geometry.
                                    if let Some(position) = s.pointer(geometry, 4)? {
                                        let cached: [f32; 3] = wire(s.bytes(position, 12)?)?;
                                        ensure!(cached.iter().all(|x| x.is_finite()), "invalid static endpoint cache");
                                        if point.actor.is_none() {
                                            ensure!(cached == point.position, "static endpoint cache differs from fixed waypoint");
                                        }
                                    }
                                    Ok(point)
                                }).transpose()?
                            } else { None },
                            connection: raw.connection,
                            dynamic_kind: raw.dynamic_kind,
                            sample: raw.sample,
                            used: raw.used,
                            length: raw.length,
                            unknown: raw.unknown,
                            dynamics: raw.dynamics.to_vec(),
                        })
                    })
                    .transpose()?,
            );
        }
    }
    Ok(value)
}

fn read_point(s: &SourceSession, p: Address) -> Result<WaypointInspection> {
    let w: WaypointWire = wire(s.bytes(p, 28)?)?;
    ensure!(
        w.position.iter().all(|v| v.is_finite()) && w.radius.is_finite() && w.radius >= 0.0,
        "invalid route point geometry"
    );
    Ok(WaypointInspection {
        source: s.source_identity(p)?,
        position: w.position,
        radius: w.radius,
        actor: s
            .pointer(p, 20)?
            .map(|a| s.source_identity(a))
            .transpose()?,
        unknown: w.unknown,
        tail: w.tail,
    })
}

fn path(source: &SourceObjectIdentity, kind: &str) -> Result<AssetPath> {
    let role = match source.role {
        crate::model::ai::SourceRole::Fix => "fix",
        crate::model::ai::SourceRole::Level => "level",
    };
    AssetPath::new(format!(
        "{role}:/{kind}/{}-{}/{}",
        source.block[0], source.block[1], source.allocation
    ))
}
fn point(
    p: WaypointInspection,
    result: &mut super::scene::SceneImport,
) -> Result<AssetRef<Waypoint>> {
    let value = Waypoint {
        path: path(&p.source, "waypoints")?,
        source: p.source,
        position: p.position,
        radius: p.radius,
        relative_actor: p.actor,
        relative_actor_ref: None,
        unknown: p.unknown,
        tail: p.tail,
    };
    value.validate()?;
    let reference = AssetRef::from_path(value.path.clone());
    if let Some(existing) = result.waypoints.iter().find(|p| p.path == value.path) {
        ensure!(
            *existing == value,
            "shared waypoint identity has conflicting values"
        );
    } else {
        result.waypoints.push(value);
    }
    Ok(reference)
}

pub(super) fn import_waypoint(
    s: &SourceSession,
    address: Address,
    result: &mut super::scene::SceneImport,
) -> Result<AssetRef<Waypoint>> {
    point(read_point(s, address)?, result)
}

/// Promote the same declarations used by inspection into the useful asset graph.
pub(super) fn import_bindings(
    s: &SourceSession,
    perso: Address,
    actor: &AssetPath,
    types: [u32; 3],
    result: &mut super::scene::SceneImport,
) -> Result<Vec<crate::model::route::RouteBinding>> {
    let observed = read_actor_ways(s, perso, actor.clone(), types)?;
    let mut bindings = Vec::new();
    for (slot, progress) in observed.slots.into_iter().zip(observed.progress) {
        let route = slot
            .map(|r| -> Result<_> {
                let value = Route {
                    path: path(&r.source, "routes")?,
                    source: r.source,
                    points: r
                        .points
                        .into_iter()
                        .map(|p| point(p, result))
                        .collect::<Result<_>>()?,
                    links: r
                        .links
                        .into_iter()
                        .map(|link| {
                            link.map(|l| -> Result<_> {
                                Ok(RouteLink {
                                    source: l.source,
                                    geometry: l.geometry,
                                    static_endpoint: l
                                        .static_endpoint
                                        .map(|p| point(p, result))
                                        .transpose()?,
                                    connection: l.connection,
                                    dynamic_kind: l.dynamic_kind,
                                    sample: l.sample,
                                    used: l.used,
                                    length: l.length,
                                    unknown: l.unknown,
                                    dynamics: l.dynamics,
                                })
                            })
                            .transpose()
                        })
                        .collect::<Result<_>>()?,
                    current: r.current,
                    previous: r.previous,
                    unknown: r.unknown,
                };
                value.validate()?;
                let reference = AssetRef::from_path(value.path.clone());
                if let Some(existing) = result.routes.iter().find(|r| r.path == value.path) {
                    ensure!(
                        *existing == value,
                        "shared route identity has conflicting values"
                    );
                } else {
                    result.routes.push(value);
                }
                Ok(reference)
            })
            .transpose()?;
        bindings.push(RouteBinding { route, progress });
    }
    Ok(bindings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use binrw::{BinRead, BinWrite};
    fn reconstruct<T: BinRead + BinWrite>(s: &SourceSession, a: Address, size: usize) -> Result<()>
    where
        for<'a> <T as BinRead>::Args<'a>: Default,
        for<'a> <T as BinWrite>::Args<'a>: Default,
    {
        let bytes = s.bytes(a, size)?;
        let value: T = wire(bytes)?;
        assert_eq!(super::super::container::encoded(&value)?, bytes);
        Ok(())
    }
    #[test]
    #[ignore = "requires local manor source; included by tools/test.py --local"]
    fn source_gogoud_opening_route_and_records() -> Result<()> {
        let game = std::env::var_os("ASTROLABE_GAME_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "disc".into());
        let selection = Selection::hype("manoir", "en")?;
        let actor = AssetPath::new("level:/persos/world/0/5")?;
        let observed = inspect_actor_ways(&game, &selection, &actor)?;
        assert_eq!(observed.slots.len(), 7);
        for slot in 1..4 {
            assert!(observed.slots[slot].is_none());
        }
        assert_eq!(observed.progress, vec![[0, 0, 255, 255]; 7]);
        let loop_route = observed.slots[0].as_ref().unwrap();
        assert_eq!(loop_route.points.first(), loop_route.points.last());
        let opening = observed.slots[6].as_ref().unwrap();
        assert_eq!(opening.points.len(), 2);
        assert_eq!(opening.links.len(), 1);
        let expected = [
            [-28.268343, 148.91852, 33.178833],
            [-18.166048, 149.11961, 32.88652],
        ];
        for (point, expected) in opening.points.iter().zip(expected) {
            assert!(point.actor.is_none());
            assert_eq!(point.radius, 1.0);
            for (actual, expected) in point.position.into_iter().zip(expected) {
                assert!((actual - expected).abs() < 0.00001);
            }
        }
        let link = opening.links[0].as_ref().unwrap();
        assert_eq!(link.static_endpoint.as_ref(), opening.points.get(1));
        assert_eq!((link.connection, link.dynamic_kind), (0, 0));
        assert!((link.length - 10.108523).abs() < 0.00001);
        let s = SourceSession::open(&game, &selection)?;
        let node = s
            .tree()?
            .into_iter()
            .find(|n| n.path == "world/0/5")
            .unwrap();
        let p = s.required(node.address, 4)?;
        reconstruct::<PersoWaysWire>(&s, p, 36)?;
        let h = s.required(s.required(p, 32)?, 0)?;
        reconstruct::<WayListWire>(&s, h, 12)?;
        let mut next = s.pointer(h, 0)?;
        while let Some(b) = next {
            reconstruct::<WayBindingWire>(&s, b, 20)?;
            if let Some(r) = s.pointer(b, 12)? {
                reconstruct::<WayWire>(&s, r, 12)?;
                let w: WayWire = wire(s.bytes(r, 12)?)?;
                for i in 0..w.count as usize {
                    reconstruct::<WaypointWire>(&s, s.required(s.required(r, 0)?, i * 4)?, 28)?;
                }
                for i in 0..(w.count as usize).saturating_sub(1) {
                    if let Some(l) = s.pointer(s.required(r, 4)?, i * 4)? {
                        reconstruct::<WayLinkWire>(&s, l, 72)?;
                        let link: WayLinkWire = wire(s.bytes(l, 72)?)?;
                        if link.connection == 0
                            && let Some(g) = s.pointer(l, 0)?
                        {
                            reconstruct::<StaticLineWire>(&s, g, 8)?;
                        }
                    }
                }
            }
            next = s.pointer(b, 0)?;
        }
        // Deliberately edit a useful field and reconstruct its bytes while
        // preserving the rest of the original point, including unknowns.
        let mut binding = s.required(h, 0)?;
        for _ in 0..6 {
            binding = s.required(binding, 0)?;
        }
        let route = s.required(binding, 12)?;
        let point = s.required(s.required(route, 0)?, 0)?;
        let original = s.bytes(point, 28)?;
        let mut edited: WaypointWire = wire(original)?;
        edited.position[0] += 1.0;
        let bytes = super::super::container::encoded(&edited)?;
        assert_eq!(&bytes[..4], &original[..4]);
        assert_eq!(&bytes[8..], &original[8..]);
        let reread: WaypointWire = wire(&bytes)?;
        assert_eq!(reread.position[0], opening.points[0].position[0] + 1.0);
        Ok(())
    }
}
