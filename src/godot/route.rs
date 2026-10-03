//! Resolve shared route/waypoint assets into native Curve3D resources at export.
use super::{Project, disk_path, quoted, vec3, vector};
use crate::model::{content::ContentSource, io::Content, route::Route};
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
    let supported = points.iter().all(|p| p.relative_actor.is_none())
        && route.links.iter().enumerate().all(|(i, link)| {
            link.as_ref().is_some_and(|l| {
                l.connection == 0
                    && l.dynamic_kind == 0
                    && l.static_endpoint.as_ref() == Some(&route.points[i + 1])
            })
        });
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
