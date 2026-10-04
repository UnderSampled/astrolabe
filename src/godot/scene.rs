//! Scene hierarchy projection. Source-local transforms are converted only here.
use super::{Project, disk_path, quoted, vec3, vector};
use crate::model::{geometry::SourceTransform, io::Content, scene::Scene};
use anyhow::Result;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
};

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
        writeln!(
            self.builder,
            "    var {scene_id}_ui = load(\"res://addons/astrolabe/ui/screen.tscn\").instantiate()\n    {scene_id}.add_child({scene_id}_ui)\n    {scene_id}_ui.owner = {scene_id}"
        )?;
        if !self.dialogue_resources.is_empty() {
            let resources = self
                .dialogue_resources
                .iter()
                .map(|p| format!("load({})", quoted(&format!("res://{p}"))))
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(
                self.builder,
                "    {scene_id}_ui.dialogue_items.assign([{resources}])"
            )?;
        }
        self.scene_environment(scene, source, &scene_id)?;
        let physical_environment = self.scene_physical_environment(scene, source, &scene_id)?;
        let mut names: BTreeMap<String, String> = BTreeMap::new();
        // Source contact visits the actor's physical sector and its explicit
        // collision neighbors (00464570). Virtual display sectors that no
        // physical sector references must not become global Godot blockers.
        // Keep their native shapes for inspection, with no collision layer.
        let collision_sectors: BTreeSet<_> = scene
            .nodes
            .iter()
            .filter_map(|node| node.sector.as_ref().map(|sector| (node, sector)))
            .filter(|(_, sector)| sector.virtual_sector == 0)
            .flat_map(|(node, sector)| std::iter::once(&node.id).chain(&sector.collision))
            .collect();
        let mut contact_enabled: BTreeMap<String, bool> = BTreeMap::new();
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
            let contact = if node.sector.is_some() {
                collision_sectors.contains(&node.id)
            } else {
                node.parent
                    .as_ref()
                    .map(|parent| contact_enabled[parent])
                    .unwrap_or(true)
            };
            contact_enabled.insert(node.id.clone(), contact);
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
                self.collision_nodes(&collision, source, &name, &scene_id, contact)?;
            }
            names.insert(node.id.clone(), name);
        }
        self.native_environment(&scene_id)?;
        // Fix-owned actors with no rooted SuperObject still carry useful Brain
        // state. Instance them once as native, nonvisual scene nodes so ordinary
        // compiled actor references can reach their saved designer properties.
        let mut global_names = BTreeMap::new();
        if !scene.global_actors.is_empty() {
            let globals = format!("{scene_id}_globals");
            writeln!(
                self.builder,
                "    var {globals} = Node3D.new()\n    {globals}.name = \"Globals\"\n    {scene_id}.add_child({globals})\n    {globals}.owner = {scene_id}"
            )?;
            for (i, global) in scene.global_actors.iter().enumerate() {
                let actor = source.load(&global.perso)?;
                let name = format!("{scene_id}_global_{i}");
                writeln!(
                    self.builder,
                    "    var {name} = Node3D.new()\n    {name}.name = {}\n    {globals}.add_child({name})\n    {name}.owner = {scene_id}\n    {name}.set_meta(\"source_perso\", {})\n    {name}.set_meta(\"source_stock_bound\", true)",
                    quoted(&actor.name),
                    quoted(actor.path.as_str())
                )?;
                let mut written = std::collections::BTreeSet::new();
                super::ai::component(self, source, &actor, &name, &mut written)?;
                if let Some(stock) = super::character::export_stock(self, &actor)? {
                    writeln!(
                        self.builder,
                        "    {name}.set_meta(\"source_stock\", load({}))",
                        quoted(&stock)
                    )?;
                }
                if actor.brain.is_some() {
                    writeln!(
                        self.builder,
                        "    {name}.get_node(\"Brain\").owner = {scene_id}\n    {name}.get_node(\"Brain\").set(\"automatic\", false)"
                    )?;
                }
                global_names.insert(global.perso.path.clone(), name);
            }
        }
        // Expose one stable source-identity index for generated Brains whose
        // external actor slots are compiled before the level is selected.
        // Values are NodePaths relative to the
        // level root, so this remains valid for both rooted and global actors.
        writeln!(
            self.builder,
            "    var {scene_id}_source_actor_brains = {{}}\n    var {scene_id}_source_actor_nodes = {{}}"
        )?;
        let mut source_actor_keys = BTreeSet::new();
        for node in &scene.nodes {
            let Some(reference) = &node.perso else {
                continue;
            };
            anyhow::ensure!(
                source_actor_keys.insert(reference.path.clone()),
                "duplicate source actor key {} in scene {}",
                reference.path,
                scene.path
            );
            let perso = source.load(reference)?;
            let character = format!("{}_character", names[&node.id]);
            writeln!(
                self.builder,
                "    {scene_id}_source_actor_nodes[{}] = {scene_id}.get_path_to({character})",
                quoted(reference.path.as_str())
            )?;
            if perso.brain.is_some() {
                writeln!(
                    self.builder,
                    "    {scene_id}_source_actor_brains[{}] = {scene_id}.get_path_to({character}.get_node(\"Brain\"))",
                    quoted(reference.path.as_str())
                )?;
            }
        }
        for global in &scene.global_actors {
            anyhow::ensure!(
                source_actor_keys.insert(global.perso.path.clone()),
                "duplicate source actor key {} in rooted/global actors for scene {}",
                global.perso.path,
                scene.path
            );
            let perso = source.load(&global.perso)?;
            let name = &global_names[&global.perso.path];
            writeln!(
                self.builder,
                "    {scene_id}_source_actor_nodes[{}] = {scene_id}.get_path_to({name})",
                quoted(global.perso.path.as_str())
            )?;
            if perso.brain.is_some() {
                writeln!(
                    self.builder,
                    "    {scene_id}_source_actor_brains[{}] = {scene_id}.get_path_to({name}.get_node(\"Brain\"))",
                    quoted(global.perso.path.as_str())
                )?;
            }
        }
        writeln!(
            self.builder,
            "    {scene_id}.set_meta(\"source_actor_brains\", {scene_id}_source_actor_brains)\n    {scene_id}.set_meta(\"source_actor_nodes\", {scene_id}_source_actor_nodes)"
        )?;
        // One native identity map serves rooted and nonvisual Global actors.
        // Both kinds can own designer references and compiled behavior fields.
        let mut actor_names = global_names.clone();
        let mut rooted_actors = BTreeSet::new();
        for node in &scene.nodes {
            if let Some(reference) = &node.perso {
                actor_names.insert(
                    reference.path.clone(),
                    format!("{}_character", names[&node.id]),
                );
                rooted_actors.insert(reference.path.clone());
            }
        }
        if let Some(reference) = &scene.armor {
            let resource = disk_path(&self.selection, &reference.path, "tres");
            // One owned catalog per level instance, shared by all its actors.
            // Nested definitions/pieces are local resources, so new sessions
            // cannot inherit the previous session's armor mutations.
            writeln!(
                self.builder,
                "    var {scene_id}_armor: Resource = load({}).call(\"clone_for_scene\")\n    {scene_id}_armor.resource_local_to_scene = true\n    {scene_id}.set_meta(\"source_armor\", {scene_id}_armor)",
                quoted(&format!("res://{resource}"))
            )?;
            for identity in &rooted_actors {
                let character = &actor_names[identity];
                writeln!(
                    self.builder,
                    "    {scene_id}.set_editable_instance({character}, true)\n    {character}.source_armor = {scene_id}_armor"
                )?;
            }
        }
        if let Some(reference) = &scene.inventory {
            let resource = disk_path(&self.selection, &reference.path, "tres");
            writeln!(
                self.builder,
                "    var {scene_id}_inventory: Resource = load({}).call(\"clone_for_scene\")\n    {scene_id}_inventory.resource_local_to_scene = true\n    {scene_id}.set_meta(\"source_inventory\", {scene_id}_inventory)",
                quoted(&format!("res://{resource}"))
            )?;
            for identity in &rooted_actors {
                let character = &actor_names[identity];
                writeln!(
                    self.builder,
                    "    {scene_id}.set_editable_instance({character}, true)\n    {character}.source_inventory = {scene_id}_inventory"
                )?;
            }
        }
        // Configurations are shared assets in the source. A native swap mutates
        // one scene-owned Resource and notifies every actor using that asset.
        writeln!(
            self.builder,
            "    var {scene_id}_configurations: Dictionary = {{}}"
        )?;
        for (i, identity) in rooted_actors.iter().enumerate() {
            let character = &actor_names[identity];
            writeln!(
                self.builder,
                r#"    var owned_choices_{i}: Array[Resource] = []
    for choice_{i}: Resource in {character}.source_configuration_choices:
        var configuration_id_{i}: String = str(choice_{i}.get("source_id"))
        if not {scene_id}_configurations.has(configuration_id_{i}):
            var owned_configuration_{i}: Resource = choice_{i}.duplicate(true)
            owned_configuration_{i}.resource_local_to_scene = true
            {scene_id}_configurations[configuration_id_{i}] = owned_configuration_{i}
        owned_choices_{i}.append({scene_id}_configurations[configuration_id_{i}])
    {scene_id}.set_editable_instance({character}, true)
    {character}.source_configuration_choices = owned_choices_{i}
    for configuration_field_{i} in ["source_configuration", "source_initial_configuration"]:
        var selected_configuration_{i}: Resource = {character}.get(configuration_field_{i})
        if selected_configuration_{i} != null:
            var selected_id_{i}: String = str(selected_configuration_{i}.get("source_id"))
            if not {scene_id}_configurations.has(selected_id_{i}):
                push_error("Character configuration is outside its saved choices")
                quit(1)
                return
            {character}.set(configuration_field_{i}, {scene_id}_configurations[selected_id_{i}])"#
            )?;
        }
        writeln!(
            self.builder,
            "    {scene_id}.set_meta(\"source_configurations\", {scene_id}_configurations)"
        )?;
        for (identity, character) in &actor_names {
            let perso = source.load(
                &crate::model::AssetRef::<crate::model::character::Perso>::from_path(
                    identity.clone(),
                ),
            )?;
            let Some(brain) = &perso.brain else { continue };
            if rooted_actors.contains(identity) {
                writeln!(
                    self.builder,
                    "    {scene_id}.set_editable_instance({character}, true)"
                )?;
                if !perso.routes.is_empty()
                    && let Some(physical) = &physical_environment
                {
                    writeln!(
                        self.builder,
                        "    {character}.physical_environment = {physical}"
                    )?;
                }
            }
            let model = source.load(&brain.model)?;
            let field_writes = model.actor_field_writes();
            for (i, target) in model.actor_field_targets().iter().enumerate() {
                let required_write = field_writes.iter().any(|(written, _)| written == target);
                let target_perso = source.load(target)?;
                let target_name = actor_names.get(&target.path);
                if target_perso.brain.is_none() || target_name.is_none() {
                    anyhow::ensure!(
                        !required_write,
                        "{} actor field write target {} lacks a saved native Brain",
                        perso.path,
                        target.path
                    );
                    self.diagnostic(format!(
                        "{} actor field read target {} lacks a saved native Brain; a reached read will fail explicitly",
                        perso.path, target.path
                    ));
                    continue;
                }
                let target_name = target_name.unwrap();
                writeln!(
                    self.builder,
                    "    {character}.get_node(\"Brain\").set(\"actor_brain_{i}_path\", {character}.get_node(\"Brain\").get_path_to({target_name}.get_node(\"Brain\")))"
                )?;
            }
            for (i, value) in brain.variables.iter().enumerate() {
                let crate::model::ai::DesignerValue::Actor(Some(target)) = value else {
                    continue;
                };
                let target_name = actor_names.get(&target.path).ok_or_else(|| {
                    anyhow::anyhow!(
                        "{} designer {i} refers to actor {} outside the saved scene",
                        perso.path,
                        target.path
                    )
                })?;
                writeln!(
                    self.builder,
                    "    {character}.get_node(\"Brain\").dsg_{i} = {target_name}"
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
            // The normally saved level, not a test wrapper, owns gameplay
            // motion for the selected player. Other persos retain their
            // inspection pose until their own behavior path is supported.
            writeln!(
                self.builder,
                "    {scene_id}.set_editable_instance({main}, true)\n    {main}.motion_enabled = true"
            )?;
            writeln!(
                self.builder,
                "    {scene_id}_ui.actor_path = {scene_id}_ui.get_path_to({main})"
            )?;
            if let Some(physical) = &physical_environment {
                writeln!(self.builder, "    {main}.physical_environment = {physical}")?;
            }
            for (identity, character) in &actor_names {
                let perso = source.load(
                    &crate::model::AssetRef::<crate::model::character::Perso>::from_path(
                        identity.clone(),
                    ),
                )?;
                if perso.brain.is_some() {
                    writeln!(
                        self.builder,
                        "    {character}.get_node(\"Brain\").main_actor_path = {character}.get_node(\"Brain\").get_path_to({main})"
                    )?;
                }
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
