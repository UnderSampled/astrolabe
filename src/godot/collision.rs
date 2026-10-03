//! Static collision projection; gameplay responses remain explicit source metadata.
use super::{Project, quoted, vec3, vector};
use crate::model::{AssetRef, collision::*, content::ContentSource, io::Content};
use anyhow::Result;
use std::fmt::Write as _;

impl Project {
    pub(super) fn collision_nodes(
        &mut self,
        collision: &Collision,
        source: &mut impl ContentSource,
        parent: &str,
        owner: &str,
        contact: bool,
    ) -> Result<()> {
        collision.validate()?;
        let collision = collision.resolve(source)?;
        let body = format!("{parent}_collision");
        writeln!(
            self.builder,
            "    var {body} = StaticBody3D.new()\n    {body}.name = \"Collision\"\n    {parent}.add_child({body})\n    {body}.owner = {owner}"
        )?;
        self.collision_shapes(&collision, source, &body, owner, &body, false)?;
        writeln!(
            self.builder,
            "    {body}.set_meta(\"source_collision_reachable\", {contact})"
        )?;
        if !contact {
            writeln!(
                self.builder,
                "    {body}.collision_layer = 0\n    {body}.collision_mask = 0"
            )?;
        }
        Ok(())
    }
    pub(super) fn collision_shapes(
        &mut self,
        collision: &CollisionMesh,
        source: &mut impl ContentSource,
        body: &str,
        owner: &str,
        prefix: &str,
        moving: bool,
    ) -> Result<Vec<String>> {
        let mut shapes = Vec::new();
        for (i, element) in collision.elements.iter().enumerate() {
            let name = format!("{prefix}_{i}");
            match element {
                CollisionElement::Points { .. } => {
                    self.diagnostic(format!("{prefix}: point collision retained; native contact projection is unsupported"));
                }
                CollisionElement::Triangles {
                    material, indices, ..
                } => {
                    if moving {
                        self.diagnostic(format!("{prefix}: moving triangle collision retained; native contact projection is unsupported"));
                        continue;
                    }
                    let vertices: Vec<_> = indices
                        .iter()
                        .flat_map(|t| [t[0], t[2], t[1]])
                        .map(|v| vec3(vector(collision.positions[v as usize])))
                        .collect();
                    writeln!(
                        self.builder,
                        "    var {name}_shape = ConcavePolygonShape3D.new()\n    {name}_shape.set_faces(PackedVector3Array([{}]))",
                        vertices.join(",")
                    )?;
                    self.collision_shape_node(source, material, &name, body, owner, [0.; 3])?;
                    shapes.push(name);
                }
                CollisionElement::Spheres(spheres) => {
                    for (j, sphere) in spheres.iter().enumerate() {
                        if sphere.radius == 0. {
                            self.diagnostic(format!(
                                "{prefix}: empty source sphere omitted from physics"
                            ));
                            continue;
                        }
                        let name = format!("{name}_{j}");
                        writeln!(
                            self.builder,
                            "    var {name}_shape = SphereShape3D.new()\n    {name}_shape.radius = {}",
                            sphere.radius
                        )?;
                        self.collision_shape_node(
                            source,
                            &sphere.material,
                            &name,
                            body,
                            owner,
                            vector(collision.positions[sphere.center as usize]),
                        )?;
                        shapes.push(name);
                    }
                }
                CollisionElement::Boxes(boxes) => {
                    for (j, b) in boxes.iter().enumerate() {
                        let name = format!("{name}_{j}");
                        let min = vector(collision.positions[b.min as usize]);
                        let max = vector(collision.positions[b.max as usize]);
                        let size = std::array::from_fn(|i| (max[i] - min[i]).abs());
                        let center = std::array::from_fn(|i| (max[i] + min[i]) * 0.5);
                        writeln!(
                            self.builder,
                            "    var {name}_shape = BoxShape3D.new()\n    {name}_shape.size = {}",
                            vec3(size)
                        )?;
                        self.collision_shape_node(source, &b.material, &name, body, owner, center)?;
                        shapes.push(name);
                    }
                }
            }
        }
        Ok(shapes)
    }
    fn collision_shape_node(
        &mut self,
        source: &mut impl ContentSource,
        material: &Option<AssetRef<crate::model::geometry::Material>>,
        name: &str,
        body: &str,
        owner: &str,
        center: [f32; 3],
    ) -> Result<()> {
        writeln!(
            self.builder,
            "    var {name} = CollisionShape3D.new()\n    {name}.name = {}\n    {name}.shape = {name}_shape\n    {name}.position = {}\n    {body}.add_child({name})\n    {name}.owner = {owner}",
            quoted(name.rsplit("_collision_").next().unwrap()),
            vec3(center)
        )?;
        if let Some(reference) = material {
            let value = source.load(reference)?;
            value.validate_reference(&reference.path)?;
            writeln!(
                self.builder,
                "    {name}.set_meta(\"source_material\", {})\n    {name}.set_meta(\"source_collision_identifier\", {})\n    {name}.set_meta(\"source_sound_material\", {})",
                quoted(reference.path.as_str()),
                value.collision.identifier(),
                value.sound_material
            )?;
            if value.collision.identifier() & 0x8000 != 0 {
                writeln!(self.builder, "    {name}.disabled = true")?;
            }
        }
        Ok(())
    }
}
