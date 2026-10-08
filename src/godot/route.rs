//! Resolve shared route/waypoint assets into native Curve3D resources at export.
use super::{Project, disk_path, quoted, vec3, vector};
use crate::model::{
    content::ContentSource,
    io::Content,
    route::{Route, Waypoint},
};
use anyhow::Result;
use std::fmt::Write as _;

pub(super) fn export_route(
    project: &mut Project,
    source: &mut impl ContentSource,
    route: &Route,
) -> Result<String> {
    route.validate()?;
    let path = disk_path(&project.selection, &route.path, "res");
    if project.generated.contains(&path) {
        return Ok(path);
    }
    project.reserve(&path)?;
    let points = route
        .points
        .iter()
        .map(|r| {
            let p = source.load(r)?;
            p.validate_reference(&r.path)?;
            p.validate()?;
            Ok(p)
        })
        .collect::<Result<Vec<_>>>()?;
    for point in &points {
        export_waypoint(project, point)?;
    }
    let supported = points.iter().all(|p| p.relative_actor.is_none())
        && route.links.iter().enumerate().all(|(i, link)| {
            link.as_ref().is_some_and(|l| {
                l.connection == 0
                    && l.dynamic_kind == 0
                    && l.static_endpoint.as_ref() == Some(&route.points[i + 1])
            })
        });
    let point_supported = points
        .iter()
        .map(|p| if p.relative_actor.is_none() { "1" } else { "0" })
        .collect::<Vec<_>>();
    let link_supported = route
        .links
        .iter()
        .enumerate()
        .map(|(i, link)| {
            if link.as_ref().is_some_and(|l| {
                l.connection == 0
                    && l.dynamic_kind == 0
                    && l.static_endpoint.as_ref() == Some(&route.points[i + 1])
            }) {
                "1"
            } else {
                "0"
            }
        })
        .collect::<Vec<_>>();
    let id = format!(
        "route_{}",
        &super::digest(route.path.as_str().as_bytes())[..12]
    );
    writeln!(
        project.builder,
        "    var {id} = load(\"res://addons/astrolabe/character/route.gd\").new()\n    {id}.curve = Curve3D.new()\n    {id}.supported = {supported}\n    {id}.closed = {}\n    {id}.source_identity = {}",
        route.points.len() > 1 && route.points.first() == route.points.last(),
        quoted(route.path.as_str())
    )?;
    writeln!(
        project.builder,
        "    {id}.point_supported = PackedByteArray([{}])\n    {id}.link_supported = PackedByteArray([{}])",
        point_supported.join(", "),
        link_supported.join(", ")
    )?;
    for point in &points {
        writeln!(
            project.builder,
            "    {id}.curve.add_point({})\n    {id}.radii.append({})",
            vec3(vector(point.position)),
            point.radius
        )?;
    }
    writeln!(
        project.builder,
        "    if not save_binary_resource({id}, {}, {}):\n        quit(1)\n        return",
        quoted(&format!("res://{path}")),
        quoted(&super::engine::resource_uid(&path))
    )?;
    if !supported {
        project.diagnostic(format!("{}: fixed StaticLine routes with dynamic kind zero are supported; a reached other route fails explicitly", route.path));
    }
    Ok(path)
}

pub(super) fn export_waypoint(project: &mut Project, waypoint: &Waypoint) -> Result<String> {
    waypoint.validate()?;
    let path = disk_path(&project.selection, &waypoint.path, "res");
    if project.generated.contains(&path) {
        return Ok(path);
    }
    project.reserve(&path)?;
    let id = format!(
        "waypoint_asset_{}",
        super::digest(waypoint.path.as_str().as_bytes())[..12].to_owned()
    );
    let relative_actor_path = waypoint
        .relative_actor_ref
        .as_ref()
        .map_or("", |actor| actor.path.as_str());
    let relative_actor_identity =
        waypoint
            .relative_actor
            .as_ref()
            .map_or_else(String::new, |identity| {
                format!(
                    "{}:{:02x}/{:02x}/{}",
                    match identity.role {
                        crate::model::ai::SourceRole::Fix => "fix",
                        crate::model::ai::SourceRole::Level => "level",
                    },
                    identity.block[0],
                    identity.block[1],
                    identity.allocation
                )
            });
    let supported = waypoint.relative_actor.is_none() || waypoint.relative_actor_ref.is_some();
    writeln!(
        project.builder,
        "    var {id} = load(\"res://addons/astrolabe/character/waypoint.gd\").new()\n    {id}.source_position = {}\n    {id}.radius = {}\n    {id}.relative_actor_path = {}\n    {id}.relative_actor_identity = {}\n    {id}.source_identity = {}\n    {id}.supported = {supported}",
        vec3(waypoint.position),
        waypoint.radius,
        quoted(relative_actor_path),
        quoted(&relative_actor_identity),
        quoted(waypoint.path.as_str()),
    )?;
    writeln!(
        project.builder,
        "    if not save_binary_resource({id}, {}, {}):\n        quit(1)\n        return",
        quoted(&format!("res://{path}")),
        quoted(&super::engine::resource_uid(&path))
    )?;
    if !supported {
        project.diagnostic(format!(
            "{}: actor-relative waypoint target {} is outside the imported actor graph; a reached location query fails explicitly",
            waypoint.path,
            waypoint.relative_actor.as_ref().map_or("unknown", |_| "unresolved")
        ));
    }
    Ok(path)
}
