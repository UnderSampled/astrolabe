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
        let mut targets = Vec::new();
        let mut surfaces = Vec::new();
        for (i, surface) in boundary.surfaces.iter().enumerate() {
            for triangle in &surface.triangles {
                vertices.extend(
                    triangle
                        .iter()
                        .map(|&v| vec3(vector(boundary.positions[usize::from(v)]))),
                );
                targets.push(quoted(surface.sector.as_deref().unwrap_or("")));
                surfaces.push(i.to_string());
            }
        }
        let name = format!("{parent}_boundary");
        writeln!(
            self.builder,
            "    var {name} = MeshInstance3D.new()\n    {name}.name = \"SectorBoundary\"\n    {name}.set_script(load({}))\n    {name}.vertices = PackedVector3Array([{}])\n    {name}.neighbors = PackedStringArray([{}])\n    {name}.surfaces = PackedInt32Array([{}])\n    {name}.build_wireframe()\n    {name}.visible = false\n    {parent}.add_child({name})\n    {name}.owner = {owner}",
            quoted(&format!("res://{SCRIPT}")),
            vertices.join(","),
            targets.join(","),
            surfaces.join(",")
        )?;
        Ok(())
    }
}
