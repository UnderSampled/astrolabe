//! Resolve shared route/waypoint assets into native Curve3D resources at export.
use super::{Project, disk_path, quoted, vec3, vector};
use crate::model::{
    content::ContentSource,
    io::Content,
    route::{Route, RouteCurveShape, Waypoint},
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
    // StaticLine targets its endpoint; Bezier and circle-arc links target
    // their baked samples. Dynamic speed profiles (kind > 0) are not ported.
    let link_ok = |i: usize, link: &Option<crate::model::route::RouteLink>| {
        link.as_ref().is_some_and(|l| {
            l.dynamic_kind == 0
                && match l.connection {
                    0 => l.static_endpoint.as_ref() == Some(&route.points[i + 1]),
                    2 | 3 => l.curve.is_some(),
                    _ => false,
                }
        })
    };
    let supported = points.iter().all(|p| p.relative_actor.is_none())
        && route.links.iter().enumerate().all(|(i, link)| link_ok(i, link));
    let point_supported = points
        .iter()
        .map(|p| if p.relative_actor.is_none() { "1" } else { "0" })
        .collect::<Vec<_>>();
    let link_supported = route
        .links
        .iter()
        .enumerate()
        .map(|(i, link)| if link_ok(i, link) { "1" } else { "0" })
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
    // A link's last sample index: its sampling rate, or 0 for StaticLine
    // (00526c90); ReInitWayBack and InitWayWithWp seed progress with it.
    let rates = route
        .links
        .iter()
        .map(|l| l.as_ref().and_then(|l| l.curve.as_ref()).map_or(0, |c| c.rate).to_string())
        .collect::<Vec<_>>();
    writeln!(project.builder, "    {id}.link_rates = PackedByteArray([{}])", rates.join(", "))?;
    for (index, link) in route.links.iter().enumerate() {
        let curve = format!("{id}_link_{index}");
        writeln!(project.builder, "    var {curve} := Curve3D.new()")?;
        if let Some(shape) = link.as_ref().and_then(|l| l.curve.as_ref()).map(|c| &c.shape) {
            for (point, handle_in, handle_out) in link_curve_points(shape) {
                writeln!(
                    project.builder,
                    "    {curve}.add_point({}, {}, {})",
                    vec3(vector(point)),
                    vec3(vector(handle_in)),
                    vec3(vector(handle_out))
                )?;
            }
        }
        writeln!(project.builder, "    {id}.link_curves.append({curve})")?;
    }
    writeln!(
        project.builder,
        "    if not save_binary_resource({id}, {}, {}):\n        quit(1)\n        return",
        quoted(&format!("res://{path}")),
        quoted(&super::engine::resource_uid(&path))
    )?;
    if !supported {
        project.diagnostic(format!("{}: StaticLine, Bezier and circle-arc routes with dynamic kind zero are supported; a reached other route fails explicitly", route.path));
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

/// A curved link as native cubic Bezier points (position, in handle, out
/// handle) in source space. Bezier links are exact: 00531720 evaluates the
/// cubic through start + start_vector and end + end_vector. A circle arc
/// (00532810: the short arc about the centre at the start radius, ending at
/// the main angle) is split into pieces of at most a quarter turn, each the
/// standard cubic with handles (4/3) tan(theta/4) r along the tangents.
fn link_curve_points(shape: &RouteCurveShape) -> Vec<([f32; 3], [f32; 3], [f32; 3])> {
    match *shape {
        RouteCurveShape::Bezier {
            start,
            end,
            start_vector,
            end_vector,
        } => vec![
            (start, [0.0; 3], start_vector),
            (end, end_vector, [0.0; 3]),
        ],
        RouteCurveShape::CircleArc { start, end, centre } => {
            let f = |v: [f32; 3]| v.map(f64::from);
            let (start, end, centre) = (f(start), f(end), f(centre));
            let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
            let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
            let cross = |a: [f64; 3], b: [f64; 3]| {
                [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
            };
            let norm = |a: [f64; 3]| {
                let l = dot(a, a).sqrt();
                if l > 0.0 { a.map(|x| x / l) } else { [0.0; 3] }
            };
            let radius = dot(sub(start, centre), sub(start, centre)).sqrt();
            let u = norm(sub(start, centre));
            let w = norm(sub(end, centre));
            let normal = norm(cross(u, w));
            let v = norm(cross(normal, u));
            let angle = dot(u, w).clamp(-1.0, 1.0).acos();
            let pieces = (angle / std::f64::consts::FRAC_PI_2).ceil().max(1.0) as usize;
            let step = angle / pieces as f64;
            let handle = 4.0 / 3.0 * (step / 4.0).tan() * radius;
            let at = |a: f64, scale: f64| {
                let (sin, cos) = a.sin_cos();
                let point = [0, 1, 2].map(|i| centre[i] + radius * (cos * u[i] + sin * v[i]));
                let tangent = [0, 1, 2].map(|i| scale * (-sin * u[i] + cos * v[i]));
                (point.map(|x| x as f32), tangent.map(|x| x as f32))
            };
            (0..=pieces)
                .map(|k| {
                    let (point, tangent) = at(step * k as f64, handle);
                    let back = tangent.map(|x| -x);
                    (
                        point,
                        if k == 0 { [0.0; 3] } else { back },
                        if k == pieces { [0.0; 3] } else { tangent },
                    )
                })
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circle_arc_pieces_stay_on_the_circle_and_bezier_keeps_its_handles() {
        // A half turn of radius 2 about the origin: two quarter-turn pieces.
        let arc = link_curve_points(&RouteCurveShape::CircleArc {
            start: [2.0, 0.0, 0.0],
            end: [-2.0, 0.0001, 0.0],
            centre: [0.0; 3],
        });
        assert_eq!(arc.len(), 3);
        for (p0, p1) in arc.iter().zip(&arc[1..]) {
            let control = |p: [f32; 3], h: [f32; 3]| [0, 1, 2].map(|i| p[i] + h[i]);
            let (c1, c2) = (control(p0.0, p0.2), control(p1.0, p1.1));
            for t in [0.25f32, 0.5, 0.75] {
                let u = 1.0 - t;
                let point = [0, 1, 2].map(|i| {
                    u * u * u * p0.0[i] + 3.0 * u * u * t * c1[i] + 3.0 * u * t * t * c2[i] + t * t * t * p1.0[i]
                });
                let radius = point.iter().map(|x| x * x).sum::<f32>().sqrt();
                assert!((radius - 2.0).abs() < 0.002, "{radius}");
            }
        }
        let bezier = link_curve_points(&RouteCurveShape::Bezier {
            start: [0.0; 3],
            end: [4.0, 0.0, 0.0],
            start_vector: [1.0, 1.0, 0.0],
            end_vector: [-1.0, 1.0, 0.0],
        });
        assert_eq!(bezier[0].2, [1.0, 1.0, 0.0]);
        assert_eq!(bezier[1].1, [-1.0, 1.0, 0.0]);
    }
}
