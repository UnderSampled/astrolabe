//! Static collision projection; gameplay responses remain explicit source metadata.
use super::{Project, quoted, vec3, vector};
use crate::model::{AssetRef, collision::*, content::ContentSource, io::Content};
use anyhow::{Result, ensure};
use std::fmt::Write as _;

impl Project {
    /// Geometry is immutable in the support package. Placement, disabled state
    /// and source contact metadata belong to CollisionShape3D nodes, so repeated
    /// configured parts can share a native Shape3D without sharing actor state.
    fn shared_collision_shape(&mut self, kind: &str, properties: &str) -> Result<String> {
        let text = format!("[gd_resource type=\"{kind}\" format=3]\n\n[resource]\n{properties}\n");
        let path = self.level_path(&format!(
            "collision/shapes/{}.tres",
            super::digest(text.as_bytes())
        ));
        if self.generated.contains(&path) {
            ensure!(
                std::fs::read(self.stage.path().join(&path))? == text.as_bytes(),
                "shared native collision shape identity conflict at {path}"
            );
        } else {
            self.write(path.clone(), text.as_bytes())?;
        }
        Ok(format!(
            "native_resource({})",
            quoted(&format!("res://{path}"))
        ))
    }

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
                        .flat_map(|v| vector(collision.positions[v as usize]))
                        .map(|v| v.to_string())
                        .collect();
                    let shape = self.shared_collision_shape(
                        "ConcavePolygonShape3D",
                        &format!("data = PackedVector3Array({})", vertices.join(",")),
                    )?;
                    writeln!(self.builder, "    var {name}_shape = {shape}")?;
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
                        let shape = self.shared_collision_shape(
                            "SphereShape3D",
                            &format!("radius = {}", sphere.radius),
                        )?;
                        writeln!(self.builder, "    var {name}_shape = {shape}")?;
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
                        let shape = self.shared_collision_shape(
                            "BoxShape3D",
                            &format!("size = {}", vec3(size)),
                        )?;
                        writeln!(self.builder, "    var {name}_shape = {shape}")?;
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
            writeln!(
                self.builder,
                "    {name}.set_meta(\"source_contact_material_present\", {})",
                matches!(value.collision, CollisionProperties::Surface { .. })
            )?;
            if value.collision.identifier() & 0x8000 != 0 {
                writeln!(self.builder, "    {name}.disabled = true")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AssetPath, Selection, content::Collection, geometry::Material};

    #[test]
    fn repeated_collision_geometry_shares_resources_without_sharing_contact_state() -> Result<()> {
        let output = tempfile::tempdir()?;
        let mut project = Project::new(output.path(), Selection::hype("shapes", "en")?)?;
        let mut source = Collection::default();
        let mut material = Material {
            path: AssetPath::new("level:/materials/contact")?,
            visual: None,
            collision: CollisionProperties::Surface {
                kind: 0,
                identifier: 0x8002,
            },
            sound_material: 7,
        };
        let material_ref = AssetRef::from_path(material.path.clone());
        source.insert(&material_ref, material.clone())?;
        let mut mesh = CollisionMesh {
            positions: vec![[1., 2., 3.], [4., 5., 6.], [7., 8., 9.]],
            normals: Vec::new(),
            vertex_materials: Vec::new(),
            elements: vec![
                CollisionElement::Triangles {
                    material: Some(material_ref.clone()),
                    indices: vec![[0, 1, 2]],
                    normals: Vec::new(),
                    texcoords: Vec::new(),
                    uv_indices: Vec::new(),
                    vertex_indices: Vec::new(),
                },
                CollisionElement::Spheres(vec![CollisionSphere {
                    center: 0,
                    radius: 2.,
                    material: Some(material_ref.clone()),
                }]),
                CollisionElement::Boxes(vec![CollisionBox {
                    min: 0,
                    max: 1,
                    material: Some(material_ref),
                }]),
            ],
        };
        project.collision_shapes(&mesh, &mut source, "first", "actor", "first", false)?;
        let first_resources = project.generated.clone();
        assert_eq!(first_resources.len(), 3);
        // Contact flags, sound response and placement can differ without changing
        // immutable geometry, even for independently identified source materials.
        material.path = AssetPath::new("level:/materials/other")?;
        material.collision = CollisionProperties::Surface {
            kind: 0,
            identifier: 4,
        };
        material.sound_material = 8;
        let reference = AssetRef::from_path(material.path.clone());
        source.insert(&reference, material)?;
        if let CollisionElement::Spheres(spheres) = &mut mesh.elements[1] {
            spheres[0].center = 1;
            spheres[0].material = Some(reference);
        }
        project.collision_shapes(&mesh, &mut source, "second", "actor", "second", false)?;
        assert_eq!(project.generated, first_resources);
        assert!(
            project
                .builder
                .contains("first_1_0.position = Vector3(1,3,-2)")
        );
        assert!(
            project
                .builder
                .contains("second_1_0.position = Vector3(4,6,-5)")
        );
        assert!(project.builder.contains("first_1_0.disabled = true"));
        assert!(!project.builder.contains("second_1_0.disabled = true"));
        assert!(
            project
                .builder
                .contains("second_1_0.set_meta(\"source_sound_material\", 8)")
        );
        let resources = first_resources
            .iter()
            .map(|path| std::fs::read_to_string(project.stage.path().join(path)))
            .collect::<std::io::Result<Vec<_>>>()?;
        assert!(
            resources
                .iter()
                .any(|text| text.contains("data = PackedVector3Array(1,3,-2,7,9,-8,4,6,-5)"))
        );
        assert!(
            resources
                .iter()
                .any(|text| text.contains("size = Vector3(3,3,3)"))
        );
        if let CollisionElement::Spheres(spheres) = &mut mesh.elements[1] {
            spheres[0].radius = 3.;
        }
        project.collision_shapes(&mesh, &mut source, "edited", "actor", "edited", false)?;
        assert_eq!(
            project.generated.len(),
            4,
            "an edited radius must not reuse the previous shape"
        );
        Ok(())
    }
}
