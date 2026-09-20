//! Scene hierarchy projection. Source-local transforms are converted only here.
use super::{Project, disk_path, quoted, vec3, vector};
use crate::model::{geometry::SourceTransform, io::Content, scene::Scene};
use anyhow::Result;
use std::{collections::BTreeMap, fmt::Write as _};

pub(super) fn transform(t: &SourceTransform) -> String {
    let basis = [
        vector(t.basis[0]),
        vector(t.basis[2]),
        vector(t.basis[1]).map(|v| -v),
    ];
    format!(
        "Transform3D(Basis({}, {}, {}), {})",
        vec3(basis[0]),
        vec3(basis[1]),
        vec3(basis[2]),
        vec3(vector(t.translation))
    )
}

impl Project {
    pub fn write_scene(
        &mut self,
        scene: &Scene,
        source: &mut impl crate::model::content::ContentSource,
    ) -> Result<()> {
        scene.validate()?;
        let path = self.level_path("level.tscn");
        self.has_level_scene = true;
        self.reserve(&path)?;
        let scene_id = format!("scene_{}", self.preview_instances.len());
        writeln!(
            self.builder,
            "    var {scene_id} = Node3D.new()\n    {scene_id}.name = {}",
            quoted(&scene.name)
        )?;
        self.scene_environment(scene, source, &scene_id)?;
        let mut names: BTreeMap<String, String> = BTreeMap::new();
        let mut lights = std::collections::BTreeSet::new();
        for node in &scene.nodes {
            for reference in node.light.iter().chain(
                node.sector
                    .iter()
                    .flat_map(|s| s.static_lights.iter().chain(&s.dynamic_lights)),
            ) {
                if lights.insert(reference.path.clone()) {
                    let light = source.load(reference)?;
                    light.validate_reference(&reference.path)?;
                    self.light_nodes(&light, &scene_id, lights.len() - 1)?;
                }
            }
        }
        for (i, node) in scene.nodes.iter().enumerate() {
            let name = format!("{scene_id}_node_{i}");
            let parent = node
                .parent
                .as_ref()
                .map(|p| names[p].clone())
                .unwrap_or_else(|| scene_id.clone());
            writeln!(
                self.builder,
                "    var {name} = Node3D.new()\n    {name}.name = {}\n    {parent}.add_child({name})\n    {name}.owner = {scene_id}",
                quoted(node.id.rsplit('/').next().unwrap())
            )?;
            if node.properties.transform_space == crate::model::scene::TransformSpace::World {
                writeln!(self.builder, "    {name}.top_level = true")?;
            }
            if let Some(t) = &node.transform {
                writeln!(self.builder, "    {name}.transform = {}", transform(t))?;
            }
            writeln!(
                self.builder,
                "    {name}.set_meta(\"source_node_id\", {})\n    {name}.set_meta(\"source_draw_mask\", {})",
                quoted(&node.id),
                node.draw_mask
            )?;
            writeln!(
                self.builder,
                "    {name}.set_meta(\"source_node_properties\", {})",
                quoted(&serde_json::to_string(&node.properties)?)
            )?;
            if let Some(reference) = &node.properties.bounding_geometry {
                let bounds = source.load(reference)?;
                bounds.validate_reference(&reference.path)?;
                bounds.validate()?;
                writeln!(
                    self.builder,
                    "    {name}.set_meta(\"source_picking_bounds\", {})",
                    quoted(&serde_json::to_string(bounds.as_ref())?)
                )?;
            }
            if node.properties.opacity != 255. {
                self.diagnostic(format!("{} {}: source node opacity {} retained for inspection; whole-node fading is not projected by the compatibility renderer",scene.path,node.id,node.properties.opacity));
            }
            if let Some(sector) = &node.sector {
                if let Some(boundary) = &sector.boundary {
                    self.sector_boundary(boundary, &name, &scene_id)?;
                }
                writeln!(
                    self.builder,
                    "    {name}.set_meta(\"source_sector\", {})",
                    quoted(&serde_json::to_string(sector)?)
                )?;
            }
            if let Some(instance) = &node.instance {
                let resource = disk_path(&self.selection, &instance.path, "tscn");
                writeln!(
                    self.builder,
                    "    var {name}_model = load({}).instantiate()\n    {name}.add_child({name}_model)\n    {name}_model.owner = {scene_id}",
                    quoted(&format!("res://{resource}"))
                )?;
            }
            if let Some(reference) = &node.perso {
                self.character_node(source, reference, &name, &scene_id)?;
            }
            if let Some(reference) = &node.collision {
                let collision = source.load(reference)?;
                collision.validate_reference(&reference.path)?;
                self.collision_nodes(&collision, source, &name, &scene_id)?;
            }
            names.insert(node.id.clone(), name);
        }
        self.native_environment(&scene_id)?;
        for node in &scene.nodes {
            let Some(reference) = &node.perso else {
                continue;
            };
            let perso = source.load(reference)?;
            let Some(brain) = &perso.brain else { continue };
            for (i, value) in brain.variables.iter().enumerate() {
                let crate::model::ai::DesignerValue::Actor(Some(target)) = value else {
                    continue;
                };
                let target_node = scene
                    .nodes
                    .iter()
                    .find(|n| n.perso.as_ref() == Some(target))
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "{} designer {i} refers to actor {} outside the saved scene",
                            perso.path,
                            target.path
                        )
                    })?;
                let character = format!("{}_character", names[&node.id]);
                let target = format!("{}_character", names[&target_node.id]);
                writeln!(
                    self.builder,
                    "    {scene_id}.set_editable_instance({character}, true)\n    {character}.get_node(\"Brain\").dsg_{i} = {target}"
                )?;
            }
        }
        if let Some(actor) = &scene.main_actor {
            let actor_node = scene
                .nodes
                .iter()
                .find(|n| n.perso.as_ref() == Some(actor))
                .unwrap();
            let main = format!("{}_character", names[&actor_node.id]);
            for node in &scene.nodes {
                let Some(reference) = &node.perso else {
                    continue;
                };
                if source.load(reference)?.brain.is_none() {
                    continue;
                }
                let character = format!("{}_character", names[&node.id]);
                writeln!(
                    self.builder,
                    "    {scene_id}.set_editable_instance({character}, true)\n    {character}.get_node(\"Brain\").main_actor_path = {character}.get_node(\"Brain\").get_path_to({main})"
                )?;
            }
        }
        writeln!(
            self.builder,
            "    var {scene_id}_packed = PackedScene.new()\n    if {scene_id}_packed.pack({scene_id}) != OK or ResourceSaver.save({scene_id}_packed, {}) != OK:\n        push_error(\"cannot save scene\")\n        quit(1)\n        return\n    {scene_id}.free()",
            quoted(&format!("res://{path}"))
        )?;
        writeln!(
            self.scene,
            "[ext_resource type=\"PackedScene\" path={} id=\"{scene_id}\"]",
            quoted(&format!("res://{path}"))
        )?;
        self.preview_instances.push(scene_id);
        Ok(())
    }
}
