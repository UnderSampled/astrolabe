//! Inspectable portal geometry and segment queries; routing policies remain downstream.
use super::{Project, quoted, vec3, vector};
use crate::model::scene::SectorBoundary;
use anyhow::Result;
use std::fmt::Write as _;
impl Project {
    pub(super) fn sector_boundary(
        &mut self,
        boundary: &SectorBoundary,
        parent: &str,
        owner: &str,
    ) -> Result<()> {
        const SCRIPT: &str = "addons/astrolabe/scene/sector_boundary.gd";
        self.diagnostic("Sector boundary segment queries and optional wireframes are available; graphics LOD/culling, activity and sound-propagation policies remain source metadata.");
        let mut vertices = Vec::new();
        let mut face_normals = Vec::new();
        let mut targets = Vec::new();
        let mut surfaces = Vec::new();
        for (i, surface) in boundary.surfaces.iter().enumerate() {
            for (triangle_index, triangle) in surface.triangles.iter().enumerate() {
                let a = boundary.positions[usize::from(triangle[0])];
                let b = boundary.positions[usize::from(triangle[1])];
                let c = boundary.positions[usize::from(triangle[2])];
                let edge_a = std::array::from_fn::<_, 3, _>(|axis| b[axis] - a[axis]);
                let edge_b = std::array::from_fn::<_, 3, _>(|axis| c[axis] - a[axis]);
                let computed_normal = [
                    edge_a[1] * edge_b[2] - edge_a[2] * edge_b[1],
                    edge_a[2] * edge_b[0] - edge_a[0] * edge_b[2],
                    edge_a[0] * edge_b[1] - edge_a[1] * edge_b[0],
                ];
                let normal = surface
                    .normals
                    .get(triangle_index)
                    .copied()
                    .filter(|normal| normal.iter().any(|component| *component != 0.))
                    .unwrap_or(computed_normal);
                vertices.extend(
                    triangle
                        .iter()
                        .map(|&v| vec3(vector(boundary.positions[usize::from(v)]))),
                );
                face_normals.push(vec3(vector(normal)));
                targets.push(quoted(surface.sector.as_deref().unwrap_or("")));
                surfaces.push(i.to_string());
            }
        }
        let name = format!("{parent}_boundary");
        writeln!(
            self.builder,
            "    var {name} = MeshInstance3D.new()\n    {name}.name = \"SectorBoundary\"\n    {name}.set_script(load({}))\n    {name}.vertices = PackedVector3Array([{}])\n    {name}.neighbors = PackedStringArray([{}])\n    {name}.surfaces = PackedInt32Array([{}])\n    {name}.face_normals = PackedVector3Array([{}])\n    {name}.build_wireframe()\n    {name}.visible = false\n    {parent}.add_child({name})\n    {name}.owner = {owner}",
            quoted(&format!("res://{SCRIPT}")),
            vertices.join(","),
            targets.join(","),
            surfaces.join(","),
            face_normals.join(",")
        )?;
        Ok(())
    }
}
