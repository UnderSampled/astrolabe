//! Persisted native characters and family/configuration-specialized animation libraries.
use super::{
    Project,
    animation::{self, NativeAnimation, Pose},
    disk_path, quoted, vec3, vector,
};
use crate::model::{
    AssetPath, AssetRef, animation::*, character::*, content::ContentSource,
    geometry::SourceTransform, io::Content, mechanics::*,
};
use anyhow::{Context, Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    sync::Arc,
};

fn label(name: &str) -> String {
    let text: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if text.is_empty() {
        "unnamed".into()
    } else {
        text
    }
}
pub(super) fn perso_path(project: &Project, perso: &Perso) -> String {
    project.level_path(&format!(
        "persos/{}__{}.tscn",
        label(&perso.name),
        &super::digest(perso.path.as_str().as_bytes())[..10]
    ))
}
struct Part {
    channel: u16,
    slot: u16,
    restored: bool,
    instance: Option<String>,
    physical_contact: Option<(crate::model::collision::CollisionMesh, Vec<SourceTransform>)>,
    action: Option<String>,
    scale: Option<[f32; 3]>,
    configuration: String,
    selected: bool,
}
impl Part {
    fn node(&self) -> String {
        format!(
            "Channels/channel_{}/{}part_{}",
            self.channel,
            if self.restored { "reset_" } else { "" },
            self.slot
        )
    }
}
pub(super) fn export_configuration(project: &mut Project, config: &ObjectList) -> Result<String> {
    config.validate()?;
    let path = disk_path(
        &project.selection,
        &config.path,
        "native-configuration.tres",
    );
    if !project.generated.contains(&path) {
        let order = (0..config.parts.len())
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(",");
        project.write(path.clone(), format!("[gd_resource type=\"Resource\" load_steps=2 format=3]\n[ext_resource type=\"Script\" path=\"res://addons/astrolabe/character/configuration.gd\" id=\"script\"]\n[resource]\nresource_local_to_scene = true\nscript = ExtResource(\"script\")\nsource_id = {}\npayload_order = PackedInt32Array({order})\nhas_reset_payloads = {}\nrestored = false\n", quoted(config.path.as_str()), config.reset_parts.is_some()).as_bytes())?;
    }
    Ok(format!("res://{path}"))
}
struct State {
    state: CharacterState,
    clip: Option<Arc<AnimationClip>>,
    frames: Vec<BTreeMap<u16, Pose>>,
}

fn movement_collision(
    project: &mut Project,
    source: &mut impl ContentSource,
    perso: &Perso,
    actor: &str,
    base: &str,
) -> Result<()> {
    let Some(collisions) = &perso.collisions else {
        return Ok(());
    };
    collisions.validate()?;
    let group = &collisions.groups[2];
    let Some(reference) = &group.profile else {
        return Ok(());
    };
    let profile = source.load(reference)?;
    profile.validate_reference(&reference.path)?;
    profile.validate()?;
    if let Some(id) = group.active {
        profile.selection(id)?;
    }
    let Some(geometry) = &profile.geometry else {
        return Ok(());
    };
    let selected = |id| -> BTreeSet<_> {
        profile
            .selections
            .iter()
            .find(|s| Some(s.id) == id)
            .into_iter()
            .flat_map(|s| &s.zones)
            .map(|z| usize::from(z.index))
            .collect()
    };
    let active = selected(group.active);
    let mut shapes = Vec::new();
    for (slot, reference) in geometry.slots.iter().enumerate() {
        let Some(reference) = reference else {
            continue;
        };
        let collision = source.load(reference)?;
        collision.validate_reference(&reference.path)?;
        let mesh = collision.resolve(source)?;
        for name in project.collision_shapes(
            &mesh,
            source,
            actor,
            actor,
            &format!("{actor}_movement_{slot}"),
            true,
        )? {
            let node = name.split_once("_movement_").unwrap().1;
            writeln!(
                project.builder,
                "    {name}.name = \"Movement_{node}\"\n    {name}.set_meta(\"source_zone\", {slot})\n    {name}.set_meta(\"source_collision\", {})\n    {actor}._set_movement_shape_active({name}, {})",
                quoted(reference.path.as_str()),
                active.contains(&slot)
            )?;
            shapes.push((slot, format!("Movement_{node}")));
        }
    }
    if shapes.is_empty() {
        return Ok(());
    }
    writeln!(
        project.builder,
        "    {actor}.collision_layer = 2\n    {actor}.collision_mask = 1"
    )?;
    let Some(activations) = &profile.activations else {
        return Ok(());
    };
    let mut cases: BTreeMap<Option<u32>, Vec<String>> = BTreeMap::new();
    for activation in activations {
        cases
            .entry(activation.selection)
            .or_default()
            .push(activation.state.to_string());
    }
    let mut script = format!(
        "extends {}\n## Compiled state activation of persisted native movement shapes.\nfunc _state_movement_collision(id: int) -> void:\n    super._state_movement_collision(id)\n    match id:\n",
        quoted(base)
    );
    for (selection, states) in cases {
        writeln!(script, "        {}:", states.join(", "))?;
        let active = selected(selection);
        for (slot, node) in &shapes {
            writeln!(
                script,
                "            _set_movement_shape_active(${node}, {})",
                active.contains(slot)
            )?;
        }
    }
    script.push_str("        _: pass\n");
    let path = perso_path(project, perso).replace(".tscn", "_collisions.gd");
    project.write(path.clone(), script.as_bytes())?;
    writeln!(
        project.builder,
        "    {actor}.set_script(native_resource({}))",
        quoted(&format!("res://{path}"))
    )?;
    Ok(())
}

fn auxiliary_collision(
    project: &mut Project,
    source: &mut impl ContentSource,
    perso: &Perso,
    family: &Family,
    actor: &str,
    group_index: usize,
) -> Result<()> {
    let Some(collisions) = &perso.collisions else {
        return Ok(());
    };
    collisions.validate()?;
    let group = &collisions.groups[group_index];
    let (label, native_type, layer) = match group_index {
        1 => ("ZDE", "Area3D", 4),
        3 => ("ZDR", "StaticBody3D", 8),
        _ => unreachable!("auxiliary collision group"),
    };
    let Some(reference) = &group.profile else {
        return Ok(());
    };
    let profile = source.load(reference)?;
    profile.validate_reference(&reference.path)?;
    profile.validate()?;
    if let Some(id) = group.active {
        profile.selection(id)?;
    }
    let Some(geometry) = &profile.geometry else {
        return Ok(());
    };
    let selected = |id| -> BTreeSet<_> {
        profile
            .selections
            .iter()
            .find(|selection| Some(selection.id) == id)
            .into_iter()
            .flat_map(|selection| &selection.zones)
            .map(|zone| usize::from(zone.index))
            .collect()
    };
    let fallback_active = selected(group.active);
    let active_states = |slot: usize| -> Vec<i32> {
        if let Some(activations) = &profile.activations {
            activations
                .iter()
                .filter(|activation| selected(activation.selection).contains(&slot))
                .map(|activation| i32::from(activation.state))
                .collect()
        } else if fallback_active.contains(&slot) {
            family.states.iter().map(|state| state.id as i32).collect()
        } else {
            Vec::new()
        }
    };
    let initial_state = perso.current_state.or(perso.initial_state);
    let area = format!("{actor}_{}", label.to_lowercase());
    writeln!(
        project.builder,
        "    var {area} = {native_type}.new()\n    {area}.name = \"{label}\"\n    {area}.collision_layer = {layer}\n    {area}.collision_mask = 0\n    {area}.set_meta(\"source_collision_perso\", {})\n    {actor}.add_child({area})\n    {area}.owner = {actor}",
        quoted(perso.path.as_str())
    )?;
    if group_index == 1 {
        writeln!(
            project.builder,
            "    {area}.monitoring = false\n    {area}.monitorable = true"
        )?;
    }
    for (slot, reference) in geometry.slots.iter().enumerate() {
        let Some(reference) = reference else {
            continue;
        };
        let collision = source.load(reference)?;
        collision.validate_reference(&reference.path)?;
        let mesh = collision.resolve(source)?;
        let names = project.collision_shapes(
            &mesh,
            source,
            &area,
            actor,
            &format!("{actor}_{}_{slot}", label.to_lowercase()),
            true,
        )?;
        let states = active_states(slot);
        let state_array = format!(
            "PackedInt32Array([{}])",
            states
                .iter()
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        );
        for name in names {
            writeln!(
                project.builder,
                "    {name}.name = {}\n    {name}.set_meta(\"source_zone\", {slot})\n    {name}.set_meta(\"source_active_states\", {state_array})\n    {actor}._set_zde_shape_active({name}, {})",
                quoted(&format!(
                    "{label}_{slot}_{}",
                    name.rsplit('_').next().unwrap_or("shape")
                )),
                initial_state.is_some_and(|state| states.contains(&(state as i32)))
            )?;
        }
    }
    Ok(())
}

/// Save typed pickup properties on the actor. Identity strings only join
/// catalogue Resources; runtime code never decodes source records or JSON.
pub(super) fn export_stock(project: &mut Project, perso: &Perso, name: &str) -> Result<()> {
    writeln!(
        project.builder,
        "    {name}.set_meta(\"source_stock_bound\", true)"
    )?;
    if let Some(stock) = &perso.inventory_stock {
        let key = |identity: &Option<crate::model::ai::SourceObjectIdentity>| -> Result<String> {
            Ok(identity
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?
                .unwrap_or_default())
        };
        writeln!(
            project.builder,
            "    var {name}_stock: Resource = load(\"res://addons/astrolabe/character/inventory_stock.gd\").new()\n    {name}_stock.resource_local_to_scene = true\n    {name}_stock.category_key = {}\n    {name}_stock.item_key = {}\n    {name}_stock.count = {}\n    {name}.set_meta(\"source_stock\", {name}_stock)",
            quoted(&key(&stock.category)?),
            quoted(&key(&stock.item)?),
            stock.quantity
        )?;
    }
    Ok(())
}

pub(super) fn export_perso(
    project: &mut Project,
    source: &mut impl ContentSource,
    perso: &Perso,
    written: &mut BTreeSet<AssetPath>,
) -> Result<()> {
    if !written.insert(perso.path.clone()) {
        return Ok(());
    }
    let family = source.load(&perso.family)?;
    family.validate_reference(&perso.family.path)?;
    family.validate()?;
    for id in [
        perso.initial_state,
        perso.current_state,
        perso.secondary_state,
    ]
    .into_iter()
    .flatten()
    {
        ensure!(
            family.states.iter().any(|s| s.id == id),
            "perso state missing from family"
        );
    }
    let configuration = perso.configuration.as_ref();
    let mut references: BTreeSet<AssetPath> = perso
        .configuration
        .iter()
        .chain(perso.initial_configuration.iter())
        .map(|r| r.path.clone())
        .collect();
    if let Some(brain) = &perso.brain {
        let model = source.load(&brain.model)?;
        model.validate_reference(&brain.model.path)?;
        references.extend(
            model
                .configuration_references()
                .into_iter()
                .map(|r| r.path.clone()),
        );
    }
    let mut configurations = Vec::new();
    for path in &references {
        let reference = AssetRef::<ObjectList>::from_path(path.clone());
        let value = source.load(&reference)?;
        value.validate_reference(path)?;
        value.validate()?;
        configurations.push(value);
    }
    // The target library is specialized in this active level; no Fix binding is cached globally.
    let key = format!(
        "{}|{}|{}",
        family.path,
        configuration.map_or("none", |r| r.path.as_str()),
        references
            .iter()
            .map(AssetPath::as_str)
            .collect::<Vec<_>>()
            .join("|")
    );
    let rig_id = format!("rig_{}", &super::digest(key.as_bytes())[..12]);
    if !project.character_rigs.contains(&rig_id) {
        export_rig(
            project,
            source,
            &family,
            &configurations,
            configuration.map(|r| &r.path),
            &rig_id,
            written,
        )?;
        project.character_rigs.insert(rig_id.clone());
    }
    let path = perso_path(project, perso);
    project.reserve(&path)?;
    let name = format!(
        "perso_{}",
        &super::digest(perso.path.as_str().as_bytes())[..12]
    );
    let state = perso
        .current_state
        .or(perso.initial_state)
        .map_or(-1, i64::from);
    writeln!(
        project.builder,
        "    var {name} = {rig_id}()\n    {name}.name = {}\n    {name}.set_meta(\"source_perso_name\", {})\n    {name}.set_meta(\"source_perso\", {})",
        quoted(&label(&perso.name)),
        quoted(&perso.name),
        quoted(perso.path.as_str())
    )?;
    writeln!(
        project.builder,
        "    {name}.set_meta(\"source_object_types\", PackedInt32Array([{}, {}, {}]))",
        perso.object_types[0], perso.object_types[1], perso.object_types[2]
    )?;
    export_stock(project, perso, &name)?;
    writeln!(
        project.builder,
        "    {name}.source_custom_bits = {}",
        perso.custom_bits
    )?;
    if let Some(activation) = perso.activation {
        writeln!(
            project.builder,
            "    {name}.source_activation_known = true\n    {name}.source_object_active = {}\n    {name}.source_return_suppressed = {}\n    {name}.source_kill_resets_components = {}",
            activation.active, activation.return_suppressed, activation.reset_on_kill
        )?;
    }
    if let Some(reference) = &perso.picking_bounds {
        let bounds = source.load(reference)?;
        bounds.validate_reference(&reference.path)?;
        let mesh = bounds.resolve(source)?;
        let Some(crate::model::collision::CollisionElement::Spheres(spheres)) =
            mesh.elements.first()
        else {
            anyhow::bail!("{} picking bounds need first sphere element", perso.path);
        };
        let center = spheres
            .first()
            .context("picking bounds have no first sphere")?
            .center;
        let position = mesh.positions[usize::from(center)];
        writeln!(
            project.builder,
            "    var {name}_picking = Marker3D.new()\n    {name}_picking.name = \"PickingCenter\"\n    {name}_picking.position = {}\n    {name}.add_child({name}_picking)\n    {name}_picking.owner = {name}\n    {name}.source_picking_center_path = ^\"PickingCenter\"",
            vec3(super::vector(position))
        )?;
    }
    writeln!(
        project.builder,
        "    {name}.source_motion_binding_present = {}",
        perso.motion_binding_present
    )?;
    if let Some(microphone) = &perso.microphone {
        let rotation = crate::model::geometry::SourceTransform {
            basis: microphone.local_transform.rotation,
            ..microphone.local_transform.clone()
        };
        writeln!(
            project.builder,
            "    {name}.get_node(\"SoundEmitter\").microphone_basis = ({}).basis\n    {name}.get_node(\"SoundEmitter\").microphone_basis_known = true",
            super::scene::transform(&rotation)
        )?;
    }
    let controller = format!(
        "res://{}/actions.gd",
        rig_directory(project, &family, &rig_id)
    );
    if let Some(component) = &perso.particles {
        if component.selected.is_some() || component.enabled {
            project.diagnostic(format!(
                "{} has an initial selected/active particle emission; the actor is exported without that emission",
                perso.path
            ));
        }
        writeln!(
            project.builder,
            "    var {name}_particles = Node3D.new()\n    {name}_particles.name = \"ParticleSource\"\n    {name}_particles.set_script(load(\"res://addons/astrolabe/character/particle_source.gd\"))\n    {name}.add_child({name}_particles)\n    {name}_particles.owner = {name}"
        )?;
    }
    auxiliary_collision(project, source, perso, &family, &name, 1)?;
    auxiliary_collision(project, source, perso, &family, &name, 3)?;
    movement_collision(project, source, perso, &name, &controller)?;
    let choices = configurations
        .iter()
        .map(|config| {
            export_configuration(project, config)
                .map(|path| format!("native_resource({})", quoted(&path)))
        })
        .collect::<Result<Vec<_>>>()?
        .join(",");
    writeln!(
        project.builder,
        "    var {name}_configurations: Array[Resource] = [{choices}]\n    {name}.source_configuration_choices = {name}_configurations"
    )?;
    if let Some(config) = configuration {
        writeln!(
            project.builder,
            "    {name}.source_configuration = native_resource({})",
            quoted(&format!(
                "res://{}",
                disk_path(
                    &project.selection,
                    &config.path,
                    "native-configuration.tres"
                )
            ))
        )?;
    }
    if let Some(initial) = &perso.initial_configuration {
        writeln!(
            project.builder,
            "    {name}.source_initial_configuration = native_resource({})",
            quoted(&format!(
                "res://{}",
                disk_path(
                    &project.selection,
                    &initial.path,
                    "native-configuration.tres"
                )
            ))
        )?;
    }
    if let Some(points) = perso.hit_points {
        writeln!(
            project.builder,
            "    var {name}_hit_points = load(\"res://addons/astrolabe/character/hit_points.gd\").new()\n    {name}_hit_points.resource_local_to_scene = true\n    {name}_hit_points.current = {}\n    {name}_hit_points.maximum = {}\n    {name}_hit_points.limit = {}\n    {name}.source_hit_points = {name}_hit_points",
            points.current, points.maximum, points.limit
        )?;
    }
    writeln!(
        project.builder,
        "    {name}.initial_state = {state}\n    {name}.current_state = {state}\n    if {state} >= 0:\n        apply_initial_pose({name}, {name}.get_node(\"AnimationPlayer\").get_animation(\"state_{state}\"))"
    )?;
    if let Some(camera) = &perso.camera {
        let framing = &camera.parameters[1];
        let initial = &camera.parameters[0];
        let initially_current = framing.registered_viewport() == Some(1);
        writeln!(
            project.builder,
            "    var {name}_camera = Camera3D.new()\n    {name}_camera.name = \"SourceCamera\"\n    {name}_camera.set_script(load(\"res://addons/astrolabe/camera/source_camera.gd\"))\n    {name}_camera.rotation.y = PI\n    {name}_camera.keep_aspect = Camera3D.KEEP_WIDTH\n    {name}_camera.fov = {}\n    {name}_camera.near = {}\n    {name}_camera.far = {}\n    {name}_camera.motion_mode = {}\n    {name}_camera.initial_motion_mode = {}\n    {name}_camera.actor_offset = {}\n    {name}_camera.look_offset = {}\n    {name}_camera.mode_zero_target_offset = {}\n    {name}_camera.mode_zero_actor_offset = {}\n    {name}_camera.mode_zero_look_offset = {}\n    {name}_camera.mode_zero_distance = {}\n    {name}_camera.mode_zero_yaw_limits = Vector2({}, {})\n    {name}_camera.source_shift_alpha = {}\n    {name}_camera.mode_zero_pitch_limits = Vector2({}, {})\n    {name}_camera.mode_zero_parameter_flags = {}\n    {name}_camera.source_mode_zero_action_flags = {}\n    {name}_camera.mode_zero_target_child_index = {}\n    {name}_camera.initially_current = {}\n    {name}.add_child({name}_camera)\n    {name}_camera.owner = {name}",
            framing.horizontal_fov.to_degrees(),
            camera.clip_planes[0],
            camera.clip_planes[1],
            framing.mode,
            initial.mode,
            vec3(vector(framing.actor_offset)),
            vec3(vector(framing.look_offset)),
            vec3(vector(framing.target_offset)),
            vec3(vector(framing.actor_offset)),
            vec3(vector(framing.look_offset)),
            format_args!("Vector2({},{})", framing.distance[0], framing.distance[1]),
            framing.yaw,
            framing.shift_alpha(),
            framing.shift_alpha(),
            framing.pitch,
            framing.shift_teta(),
            framing.angle_flags(),
            framing.controller_flags(),
            framing.target_child_index(),
            initially_current
        )?;
        writeln!(
            project.builder,
            "    {name}_camera.owner_motion_binding_present = {}\n    {name}_camera.source_initial_horizontal_fov = {}\n    {name}_camera.initially_registered = {}\n    {name}_camera.initial_viewport_context = {}\n    {name}_camera.initial_secondary_registered = {}\n    {name}_camera.initial_secondary_viewport_context = {}\n    {name}_camera.initial_auxiliary_viewport_context = {}",
            camera.owner_motion_binding_present,
            initial.horizontal_fov,
            framing.registration_active(),
            framing.viewport_context(),
            camera.parameters[3].registration_active(),
            camera.parameters[3].viewport_context(),
            camera.parameters[2].viewport_context()
        )?;
        writeln!(
            project.builder,
            "    var {name}_camera_initial = load(\"res://addons/astrolabe/camera/framing.gd\").new()\n    {name}_camera_initial.motion_mode = {}\n    {name}_camera_initial.target_offset = {}\n    {name}_camera_initial.actor_offset = {}\n    {name}_camera_initial.look_offset = {}\n    {name}_camera_initial.distance = Vector2({}, {})\n    {name}_camera_initial.yaw_limits = Vector2({}, {})\n    {name}_camera_initial.pitch_limits = Vector2({}, {})\n    {name}_camera_initial.shift_alpha = {}\n    {name}_camera_initial.parameter_flags = {}\n    {name}_camera_initial.action_flags = {}\n    {name}_camera_initial.target_child_index = {}\n    {name}_camera_initial.horizontal_fov = {}\n    {name}_camera.initial_framing = {name}_camera_initial",
            initial.mode,
            vec3(vector(initial.target_offset)),
            vec3(vector(initial.actor_offset)),
            vec3(vector(initial.look_offset)),
            initial.distance[0],
            initial.distance[1],
            initial.yaw,
            initial.shift_alpha(),
            initial.pitch,
            initial.shift_teta(),
            initial.shift_alpha(),
            initial.angle_flags(),
            initial.controller_flags(),
            initial.target_child_index(),
            initial.horizontal_fov,
        )?;
        writeln!(
            project.builder,
            "    {name}_camera_initial.viewport_registered = {}\n    {name}_camera_initial.viewport_context = {}",
            initial.registration_active(),
            initial.viewport_context()
        )?;
        for (owner, parameters) in [
            (format!("{name}_camera"), framing),
            (format!("{name}_camera_initial"), initial),
        ] {
            let limits = parameters.follow_linear_speed_limits();
            writeln!(
                project.builder,
                "    {owner}.follow_linear_speed_limits = Vector2({}, {})\n    {owner}.follow_linear_response_distance = {}\n    {owner}.follow_linear_acceleration_divisor = {}\n    {owner}.follow_angular_speed = {}\n    {owner}.follow_angular_response_scale = {}\n    {owner}.follow_angular_acceleration_divisor = {}\n    {owner}.follow_view_angles = {}\n    {owner}.follow_view_angle_speed = {}",
                limits[0],
                limits[1],
                parameters.follow_linear_response_distance(),
                initial.follow_linear_acceleration_divisor(),
                parameters.follow_angular_speed(),
                parameters.follow_angular_response_scale(),
                parameters.follow_angular_acceleration_divisor(),
                vec3(parameters.follow_view_angles()),
                parameters.follow_view_angle_speed(),
            )?;
            writeln!(
                project.builder,
                "    {owner}.follow_controls_present = true"
            )?;
        }
        writeln!(
            project.builder,
            "    {name}_camera.static_obstruction_frames = {}",
            camera.static_retry_frames()
        )?;
        // Source camera local -Y becomes the converted actor's +Z. Godot's
        // camera looks along local -Z, hence the child half-turn above.
        project.diagnostic(format!(
            "{}: source parameter set 1 supplies active camera framing/registration and set 0 supplies the reset preset and linear acceleration divisor; the scene binds the exact MainActor target; native mode-0 follow applies source motion controls to the owner pose; Type3 multi-contact sliding, target-roll publication and 0051ec70 recovery remain explicit conditional gaps",
            perso.path
        ));
    }
    let mut routes = Vec::new();
    let mut progress = Vec::new();
    let mut samples = Vec::new();
    let mut previous_points = Vec::new();
    let mut previous_samples = Vec::new();
    for binding in &perso.routes {
        routes.push(if let Some(reference) = &binding.route {
            let route = source.load(reference)?;
            route.validate_reference(&reference.path)?;
            let path = super::route::export_route(project, source, &route)?;
            format!("load({})", quoted(&format!("res://{path}")))
        } else {
            "null".into()
        });
        progress.push(binding.progress[0].to_string());
        samples.push(binding.progress[1].to_string());
        let previous = |v: u8| if v == 255 { "-1".into() } else { v.to_string() };
        previous_points.push(previous(binding.progress[2]));
        previous_samples.push(previous(binding.progress[3]));
    }
    if !routes.is_empty() {
        writeln!(
            project.builder,
            "    {name}.source_routes.assign([{}])\n    {name}.source_route_points = PackedInt32Array([{}])\n    {name}.source_route_samples = PackedInt32Array([{}])\n    {name}.source_route_previous_points = PackedInt32Array([{}])\n    {name}.source_route_previous_samples = PackedInt32Array([{}])",
            routes.join(", "),
            progress.join(", "),
            samples.join(", "),
            previous_points.join(", "),
            previous_samples.join(", ")
        )?;
    }
    super::ai::component(project, source, perso, &name, written)?;
    writeln!(
        project.builder,
        "    {name}.source_set_object_active_state({name}.source_object_active and not {name}.source_return_suppressed)\n    {name}.refresh_configured_participation()\n    save_scene({name}, {})\n    {name}.free()",
        quoted(&format!("res://{path}"))
    )?;
    if state < 0 {
        project.diagnostic(format!(
            "{}: no initial state; persisted alternatives remain hidden",
            perso.path
        ));
    }
    project.diagnostic(format!("{}: native sampled poses; native translation tracks available; gameplay visibility overrides, root rotation, lip sync and complete AI control remain unsupported",perso.path));
    Ok(())
}

fn rig_directory(project: &Project, family: &Family, rig_id: &str) -> String {
    project.level_path(&format!(
        "characters/{}__{}/{}",
        label(&family.name),
        &super::digest(family.path.as_str().as_bytes())[..8],
        &rig_id[4..]
    ))
}

fn export_rig(
    project: &mut Project,
    source: &mut impl ContentSource,
    family: &Family,
    configurations: &[Arc<ObjectList>],
    selected: Option<&AssetPath>,
    rig_id: &str,
    written: &mut BTreeSet<AssetPath>,
) -> Result<()> {
    let directory = rig_directory(project, family, rig_id);
    let mut states = Vec::new();
    let mut channels = BTreeSet::new();
    let mut sampled_slots: BTreeMap<u16, BTreeSet<u16>> = BTreeMap::new();
    for state in &family.states {
        let clip = state
            .animation
            .as_ref()
            .map(|r| {
                let value = source.load(r)?;
                value.validate_reference(&r.path)?;
                Ok::<_, anyhow::Error>(value)
            })
            .transpose()?;
        let frames = if let Some(clip) = &clip {
            if clip.morph_count > 0 {
                project.diagnostic(format!(
                    "{}: {} morph payloads remain unprojected",
                    clip.path, clip.morph_count
                ));
            }
            if clip
                .samples
                .iter()
                .any(|s| matches!(s.kind, ChannelKind::Light | ChannelKind::Unknown(_)))
            {
                project.diagnostic(format!(
                    "{}: light/reserved channel transforms retained; payloads not rendered",
                    clip.path
                ));
            }
            animation::bake(clip, source)?
        } else {
            vec![BTreeMap::new()]
        };
        for pose in &frames {
            for (&channel, sample) in pose {
                channels.insert(channel);
                if let Some(slot) = sample.object {
                    sampled_slots.entry(channel).or_default().insert(slot);
                }
            }
        }
        if let Some(reference) = &state.mechanics {
            export_mechanics(project, source, reference, written)?;
        }
        states.push(State {
            state: state.clone(),
            clip,
            frames,
        });
    }
    let mut parts = Vec::new();
    // Only channels that sample objects need saved payload banks. Transform and
    // cue targets remain native channels even when they never display content.
    // Keep every nonempty original slot: an arbitrary Proc82 swap can move it
    // into a sampled slot. Empty entries are explicit bank metadata, not nodes.
    const MAX_CONFIGURED_PAYLOADS: usize = 65_536;
    let payload_count = configurations.iter().try_fold(0usize, |total, config| {
        let nonempty = config
            .all_parts()
            .filter(|part| part.kind != ObjectPartKind::Absent)
            .count();
        nonempty
            .checked_mul(sampled_slots.len())
            .and_then(|n| total.checked_add(n))
    });
    let payload_count = payload_count
        .ok_or_else(|| anyhow::anyhow!("{} configured payload count overflow", family.path))?;
    ensure!(
        payload_count <= MAX_CONFIGURED_PAYLOADS,
        "{} needs {payload_count} native configured payloads across {} object-bearing channels; supported rig budget is {MAX_CONFIGURED_PAYLOADS}",
        family.path,
        sampled_slots.len()
    );
    for config in configurations {
        export_configuration(project, config)?;
        for (restored, table) in [
            (false, Some(&config.parts)),
            (true, config.reset_parts.as_ref()),
        ] {
            let Some(table) = table else {
                continue;
            };
            for &channel in sampled_slots.keys() {
                for (slot, part) in table.iter().enumerate() {
                    if part.kind == ObjectPartKind::Absent {
                        continue;
                    }
                    let physical_contact = if let Some(reference) = &part.instance {
                        let instance = source.load(reference)?;
                        let collision = crate::model::collision::Collision {
                            path: instance.model.path.clone(),
                            geometry: crate::model::collision::CollisionGeometry::Model(
                                instance.model.clone(),
                            ),
                        };
                        Some((collision.resolve(source)?, instance.transforms.clone()))
                    } else {
                        None
                    };
                    let instance = if let Some(reference) = &part.instance {
                        let instance = source.load(reference)?;
                        instance.validate_reference(&reference.path)?;
                        let model = source.load(&instance.model)?;
                        if written.insert(instance.path.clone()) {
                            super::export::export_instance(
                                project, source, &model, &instance, written,
                            )?;
                        }
                        Some(format!(
                            "res://{}",
                            disk_path(&project.selection, &instance.path, "tscn")
                        ))
                    } else {
                        None
                    };
                    let action = if let Some(reference) = &part.action {
                        export_action(project, source, reference, written)?;
                        Some(format!(
                            "res://{}",
                            disk_path(&project.selection, &reference.path, "tres")
                        ))
                    } else {
                        None
                    };
                    parts.push(Part {
                        channel,
                        slot: slot.try_into()?,
                        restored,
                        instance,
                        physical_contact,
                        action,
                        scale: part.scale,
                        configuration: config.path.to_string(),
                        selected: selected == Some(&config.path),
                    });
                }
            }
        }
    }
    ensure!(
        parts.len() == payload_count,
        "{} generated configured payload count differs from its checked budget",
        family.path
    );
    let mut libraries = Vec::new();
    for restored in [false, true] {
        if restored && !configurations.iter().any(|c| c.reset_parts.is_some()) {
            continue;
        }
        let library_id = format!(
            "library_{rig_id}_{}",
            if restored { "restored" } else { "normal" }
        );
        writeln!(
            project.builder,
            "    var {library_id} = AnimationLibrary.new()"
        )?;
        for state in &states {
            let name = format!("state_{}", state.state.id);
            let input = format!(
                "build_inputs/{directory}/{}{name}.tres",
                if restored { "restored/" } else { "" }
            );
            export_animation(
                project,
                source,
                state,
                &channels,
                &sampled_slots,
                configurations.iter().any(|c| c.reset_parts.is_some()),
                restored,
                &input,
                written,
            )?;
            writeln!(
                project.builder,
                "    if not add_embedded_animation({library_id}, {}, {}):\n        quit(1)\n        return",
                quoted(&name),
                quoted(&format!("res://{input}")),
            )?;
        }
        let path = format!(
            "{directory}/{}animations.res",
            if restored { "restored/" } else { "" }
        );
        project.reserve(&path)?;
        writeln!(
            project.builder,
            "    if not save_binary_resource({library_id}, {}, {}):\n        quit(1)\n        return",
            quoted(&format!("res://{path}")),
            quoted(&super::engine::resource_uid(&path)),
        )?;
        libraries.push((restored, path));
    }
    let controller = format!("{directory}/actions.gd");
    project.write(
        controller.clone(),
        super::action::compile(family)?.as_bytes(),
    )?;
    let mut function = format!(
        "\nfunc {rig_id}() -> Node3D:\n    var actor = CharacterBody3D.new()\n    actor.set_script(native_resource({}))\n    actor.set_meta(\"source_family\", {})\n    actor.collision_layer = 0\n    actor.collision_mask = 1\n    var motion = Node3D.new()\n    motion.name = \"RootMotion\"\n    actor.add_child(motion)\n    motion.owner = actor\n    var channels = Node3D.new()\n    channels.name = \"Channels\"\n    actor.add_child(channels)\n    channels.owner = actor\n",
        quoted(&format!("res://{controller}")),
        quoted(family.path.as_str())
    );
    for channel in &channels {
        writeln!(
            function,
            "    var c_{channel} = Node3D.new()\n    c_{channel}.set_script(native_resource(\"res://addons/astrolabe/character/channel.gd\"))\n    c_{channel}.name = \"channel_{channel}\"\n    channels.add_child(c_{channel})\n    c_{channel}.owner = actor\n    c_{channel}.set_meta(\"source_channel\", {})\n    c_{channel}.set_meta(\"source_channel_occurrence\", {})",
            channel % 256,
            channel / 256
        )?;
    }
    // Parking banks remain saved native content. They inherit neither physics
    // processing nor animated transforms; moving a payload to a selected slot
    // restores its ordinary local transform and native contact participation.
    for config in configurations {
        let identity = &super::digest(config.path.as_str().as_bytes())[..12];
        for &channel in sampled_slots.keys() {
            for (restored, table) in [
                (false, Some(&config.parts)),
                (true, config.reset_parts.as_ref()),
            ] {
                let Some(table) = table else { continue };
                let empty_slots = table
                    .iter()
                    .enumerate()
                    .filter(|(_, part)| part.kind == ObjectPartKind::Absent)
                    .map(|(slot, _)| slot.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                let bank = format!(
                    "bank_{identity}_{}",
                    if restored { "reset" } else { "normal" }
                );
                writeln!(
                    function,
                    "    var {bank}_{channel} = Node3D.new()\n    {bank}_{channel}.name = {}\n    {bank}_{channel}.visible = false\n    {bank}_{channel}.process_mode = Node.PROCESS_MODE_DISABLED\n    {bank}_{channel}.top_level = true\n    {bank}_{channel}.set_meta(\"source_configuration\", {})\n    {bank}_{channel}.set_meta(\"source_restored\", {})\n    {bank}_{channel}.set_meta(\"empty_payload_slots\", PackedInt32Array([{empty_slots}]))\n    c_{channel}.add_child({bank}_{channel})\n    {bank}_{channel}.owner = actor",
                    quoted(&bank),
                    quoted(config.path.as_str()),
                    restored
                )?;
            }
        }
    }
    let mut anchors = BTreeMap::new();
    let has_restored = configurations.iter().any(|c| c.reset_parts.is_some());
    for (&channel, slots) in &sampled_slots {
        for restored in [false, true] {
            if restored && !has_restored {
                continue;
            }
            for &slot in slots {
                let name = format!("{}part_{slot}", if restored { "reset_" } else { "" });
                let key = format!("Channels/channel_{channel}/{name}");
                let anchor = anchors.len();
                anchors.insert(key, anchor);
                writeln!(
                    function,
                    "    var part_{anchor} = Node3D.new()\n    part_{anchor}.set_script(native_resource(\"res://addons/astrolabe/character/configured_slot.gd\"))\n    part_{anchor}.name = {}\n    c_{channel}.add_child(part_{anchor})\n    part_{anchor}.owner = actor\n    part_{anchor}.visible = false\n    part_{anchor}.process_mode = Node.PROCESS_MODE_DISABLED\n    part_{anchor}.top_level = true\n    part_{anchor}.set_meta(\"configuration_slot\", {slot})",
                    quoted(&name)
                )?;
            }
        }
    }
    for (i, part) in parts.iter().enumerate() {
        let identity = &super::digest(part.configuration.as_bytes())[..12];
        let bank = format!(
            "bank_{identity}_{}",
            if part.restored { "reset" } else { "normal" }
        );
        let parent = if part.selected
            && !part.restored
            && let Some(anchor) = anchors.get(&part.node())
        {
            format!("part_{anchor}")
        } else {
            format!("{bank}_{}", part.channel)
        };
        writeln!(
            function,
            "    var payload_{i} = Node3D.new()\n    payload_{i}.set_script(native_resource(\"res://addons/astrolabe/character/configured_payload.gd\"))\n    payload_{i}.name = \"payload_{}\"\n    payload_{i}.original_slot = {}\n    payload_{i}.set_meta(\"source_configuration_bank\", NodePath({}))\n    {parent}.add_child(payload_{i})\n    payload_{i}.owner = actor",
            part.slot,
            part.slot,
            quoted(&format!("Channels/channel_{}/{bank}", part.channel))
        )?;
        if let Some(action) = &part.action {
            writeln!(
                function,
                "    payload_{i}.action = native_resource({})",
                quoted(action)
            )?;
        }
        if let Some(instance) = &part.instance {
            writeln!(
                function,
                "    var visual_{i} = native_resource({}).instantiate()\n    visual_{i}.name = \"Visual\"\n    payload_{i}.add_child(visual_{i})\n    visual_{i}.owner = actor",
                quoted(instance)
            )?;
        }
        if let Some((mesh, transforms)) = &part.physical_contact {
            let transform = transforms
                .iter()
                .fold(String::from("Transform3D.IDENTITY"), |outer, value| {
                    format!("({outer} * {})", super::scene::transform(value))
                });
            let area = format!("physical_contact_{i}");
            writeln!(
                function,
                "    var {area} = Area3D.new()\n    {area}.name = \"PhysicalContact\"\n    {area}.disable_mode = CollisionObject3D.DISABLE_MODE_REMOVE\n    {area}.collision_layer = 16\n    {area}.collision_mask = 0\n    {area}.monitoring = false\n    {area}.monitorable = true\n    {area}.set_meta(\"physical_object_contact\", true)\n    {area}.transform = {}\n    payload_{i}.add_child({area})\n    {area}.owner = actor",
                transform
            )?;
            let outer = std::mem::take(&mut project.builder);
            project.collision_shapes(mesh, source, &area, "actor", &area, false)?;
            function.push_str(&project.builder);
            project.builder = outer;
        }
        if let Some([x, y, z]) = part.scale {
            writeln!(function, "    payload_{i}.scale = {}", vec3([x, z, y]))?;
        }
    }
    writeln!(
        function,
        "    var player = AnimationPlayer.new()\n    player.name = \"AnimationPlayer\"\n    player.root_motion_track = ^\"RootMotion\"\n    player.callback_mode_process = AnimationMixer.ANIMATION_CALLBACK_MODE_PROCESS_MANUAL\n    player.callback_mode_method = AnimationMixer.ANIMATION_CALLBACK_MODE_METHOD_IMMEDIATE\n    actor.add_child(player)\n    player.owner = actor"
    )?;
    for (restored, path) in libraries {
        writeln!(
            function,
            "    player.add_animation_library({}, native_resource({}))",
            quoted(if restored { "restored" } else { "" }),
            quoted(&format!("res://{path}"))
        )?;
    }
    // The emitter operates on native waveform resources; no audio voices exist before cues.
    writeln!(
        function,
        "    var sound = Node3D.new()\n    sound.name = \"SoundEmitter\"\n    sound.set_script(native_resource(\"res://addons/astrolabe/audio/emitter.gd\"))\n    actor.add_child(sound)\n    sound.owner = actor"
    )?;
    bounds(project, source, family, &mut function)?;
    function.push_str("    return actor\n");
    project.builder_functions.push_str(&function);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn export_animation(
    project: &mut Project,
    source: &mut impl ContentSource,
    state: &State,
    channels: &BTreeSet<u16>,
    sampled_slots: &BTreeMap<u16, BTreeSet<u16>>,
    has_restored: bool,
    restored: bool,
    input: &str,
    written: &mut BTreeSet<AssetPath>,
) -> Result<()> {
    let fps = state.state.speed;
    let frames = &state.frames;
    let mut anim = NativeAnimation::new(
        &format!("state_{}", state.state.id),
        frames.len() as f64 / f64::from(fps.max(1)),
        fps,
        frames.len(),
        fps > 0,
    );
    let mut links = String::new();
    anim.values(".:current_state", [state.state.id.to_string()], fps)?;
    let mechanics = if let Some(reference) = &state.state.mechanics {
        writeln!(
            links,
            "[ext_resource type=\"Resource\" path={} id=\"mechanics\"]",
            quoted(&format!(
                "res://{}",
                disk_path(&project.selection, &reference.path, "tres")
            ))
        )?;
        "ExtResource(\"mechanics\")"
    } else {
        "null"
    };
    anim.values(".:current_mechanics", [mechanics.into()], fps)?;
    // An explicitly absent state identity card schedules no position update.
    // The source camera still consumes the actor's existing position history.
    anim.values(
        ".:source_camera_retains_position_history",
        [state.state.mechanics.is_none().to_string()],
        fps,
    )?;
    for channel in channels {
        anim.continuous_values(
            &format!("Channels/channel_{channel}:sample_visibility"),
            frames.iter().map(|f| {
                f.get(channel)
                    .and_then(|sample| sample.default_visibility)
                    .map_or(-1, i32::from)
                    .to_string()
            }),
            fps,
        )?;
        anim.values(
            &format!("Channels/channel_{channel}:transform"),
            frames.iter().map(|f| {
                animation::affine(
                    &f.get(channel)
                        .map_or_else(Default::default, |s| s.transform.clone()),
                )
            }),
            fps,
        )?;
    }
    // Stable sampled anchors must exist even if every initial payload at a
    // slot is absent: a later swap can bring saved content into that slot.
    for (&channel, slots) in sampled_slots {
        for anchor_restored in [false, true] {
            // Normal libraries hide reset anchors and restored libraries hide
            // normal anchors, without rebuilding content on selection changes.
            if anchor_restored && !has_restored {
                continue;
            }
            for &slot in slots {
                let node = format!(
                    "Channels/channel_{channel}/{}part_{slot}",
                    if anchor_restored { "reset_" } else { "" }
                );
                let active = |f: &BTreeMap<u16, Pose>| {
                    f.get(&channel)
                        .filter(|s| restored == anchor_restored && s.object == Some(slot))
                        // Channel visibility owns the sampled default and AI
                        // override. The anchor only selects the current part.
                        .map(|s| (s.opacity > 0, s.opacity))
                };
                anim.values(
                    &format!("{node}:visible"),
                    frames
                        .iter()
                        .map(|f| active(f).is_some_and(|s| s.0).to_string()),
                    fps,
                )?;
                anim.values(
                    &format!("{node}:transparency"),
                    frames.iter().map(|f| {
                        active(f)
                            .map_or(0., |s| 1. - f32::from(s.1) / 255.)
                            .to_string()
                    }),
                    fps,
                )?;
            }
        }
    }
    if let Some(clip) = &state.clip {
        anim.root_translation(clip, fps)?;
        writeln!(
            anim.text,
            "metadata/source_animation = {}\nmetadata/source_state_next = {}",
            quoted(clip.path.as_str()),
            state.state.next.map_or(-1, i64::from)
        )?;
        let mut groups: BTreeMap<u8, Vec<(usize, &AnimationCue)>> = BTreeMap::new();
        for (i, cue) in clip.cues.iter().enumerate() {
            export_action(project, source, &cue.action, written)?;
            writeln!(
                links,
                "[ext_resource type=\"Resource\" path={} id=\"cue_{i}\"]",
                quoted(&format!(
                    "res://{}",
                    disk_path(&project.selection, &cue.action.path, "tres")
                ))
            )?;
            groups.entry(cue.frame).or_default().push((i, cue));
        }
        let mut keys = Vec::new();
        for (frame, cues) in groups {
            let channel_paths = cues
                .iter()
                .map(|(_, cue)| {
                    let samples = &clip.frames[usize::from(frame)].samples;
                    let column = usize::from(cue.channel);
                    let id = clip.samples[samples[column] as usize].channel;
                    id.map_or_else(
                        || quoted(""),
                        |id| {
                            let occurrence = samples[..column]
                                .iter()
                                .filter(|&&index| clip.samples[index as usize].channel == Some(id))
                                .count();
                            quoted(&format!(
                                "Channels/channel_{}",
                                usize::from(id) + 256 * occurrence
                            ))
                        },
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            keys.push((f64::from(frame)/f64::from(fps.max(1)),format!("{{\"method\": &\"_cues\", \"args\": [{frame}, PackedInt32Array({}), PackedInt32Array({}), [{}], PackedStringArray({channel_paths})]}}",cues.iter().map(|(_,c)|c.channel.to_string()).collect::<Vec<_>>().join(","),cues.iter().map(|(_,c)|c.configuration_slot.to_string()).collect::<Vec<_>>().join(","),cues.iter().map(|(i,_)|format!("ExtResource(\"cue_{i}\")")).collect::<Vec<_>>().join(","))));
        }
        let releases_object = clip
            .cues
            .iter()
            .map(|cue| source.load(&cue.action))
            .collect::<Result<Vec<_>>>()?
            .iter()
            .any(|action| matches!(action.operation, AnimationOperation::Attachment { .. }));
        writeln!(
            anim.text,
            "metadata/releases_generated_object = {releases_object}"
        )?;
        anim.methods(keys)?;
    }
    project.write_build_input(
        input,
        format!(
            "[gd_resource type=\"Animation\" load_steps={} format=3]\n\n{links}\n{}",
            links.lines().count() + 1,
            anim.text
        )
        .as_bytes(),
    )?;
    Ok(())
}

fn export_action(
    project: &mut Project,
    source: &mut impl ContentSource,
    reference: &AssetRef<AnimationAction>,
    written: &mut BTreeSet<AssetPath>,
) -> Result<()> {
    if !written.insert(reference.path.clone()) {
        return Ok(());
    }
    let action = source.load(reference)?;
    action.validate_reference(&reference.path)?;
    action.validate()?;
    let mut links = String::new();
    let mut sound = "null";
    let (kind, command) = match &action.operation {
        AnimationOperation::Sound { event } => {
            if let Some(event) = event {
                let path = super::sound::export_event(project, source, event, written)?;
                writeln!(
                    links,
                    "[ext_resource type=\"Resource\" path={} id=\"sound\"]",
                    quoted(&path)
                )?;
                sound = "ExtResource(\"sound\")";
            }
            ("sound", 0)
        }
        AnimationOperation::Mechanics { command } => ("mechanics", *command),
        AnimationOperation::Attachment { value } => ("attachment", *value),
        AnimationOperation::AttachmentControl { command } => ("attachment_control", *command),
        AnimationOperation::Camera { command } => ("camera", *command),
        AnimationOperation::Unknown { kind, .. } => ("unknown", u32::from(*kind)),
    };
    if kind != "sound" && kind != "attachment" {
        project.diagnostic(format!(
            "{}: native timed cue signal; {kind} operation is not executed",
            action.path
        ));
    }
    project.write(disk_path(&project.selection,&action.path,"tres"),format!("[gd_resource type=\"Resource\" load_steps={} format=3]\n[ext_resource type=\"Script\" path=\"res://addons/astrolabe/character/cue.gd\" id=\"script\"]\n{links}\n[resource]\nscript = ExtResource(\"script\")\nsource_action = {}\nkind = &{}\ncommand = {command}\nsound = {sound}\n",links.lines().count()+2,quoted(action.path.as_str()),quoted(kind)).as_bytes())
}

fn export_mechanics(
    project: &mut Project,
    source: &mut impl ContentSource,
    reference: &AssetRef<Mechanics>,
    written: &mut BTreeSet<AssetPath>,
) -> Result<()> {
    if !written.insert(reference.path.clone()) {
        return Ok(());
    }
    let value = source.load(reference)?;
    value.validate_reference(&reference.path)?;
    value.validate()?;
    let mut text = String::from(
        "[gd_resource type=\"Resource\" load_steps=2 format=3]\n[ext_resource type=\"Script\" path=\"res://addons/astrolabe/character/mechanics.gd\" id=\"script\"]\n\n[resource]\nscript = ExtResource(\"script\")\n",
    );
    // The source table has an update/control pair per identity-card kind.
    // Walk 0/4 and Mass9 publish post-contact velocity and position. Type3
    // publishes its committed frame displacement. These native bindings do
    // not infer unknown card fields or reconstruct the source dynamics block.
    let (camera_position_source, camera_velocity_source) = match &value.data {
        MechanicsData::Character { mode: 0 | 4, .. } | MechanicsData::Mass { .. } => {
            ("post_contact_position", "post_contact_velocity")
        }
        MechanicsData::Unresolved { kind: 3, .. } => {
            ("camera_frame_position", "camera_frame_displacement")
        }
        _ => ("", ""),
    };
    writeln!(
        text,
        "camera_position_source = &{}\ncamera_velocity_source = &{}",
        quoted(camera_position_source),
        quoted(camera_velocity_source)
    )?;
    match &value.data {
        MechanicsData::Mass { mass } => writeln!(text, "mode = 9\nmass = {mass}")?,
        MechanicsData::Character { mode, settings } => {
            writeln!(text, "mode = {mode}")?;
            // Named numeric target properties, not a runtime source-card decoder.
            let values = serde_json::to_value(settings)?;
            for (key, value) in values.as_object().unwrap() {
                if value.is_number() {
                    if [
                        "fsl_available",
                        "orientation_free",
                        "dt_forced",
                        "against_wall",
                    ]
                    .contains(&key.as_str())
                    {
                        writeln!(text, "{key} = {}", value.as_u64().unwrap() != 0)?;
                    } else {
                        writeln!(text, "{key} = {value}")?;
                    }
                }
            }
        }
        MechanicsData::Unresolved { kind, .. } => {
            writeln!(text, "mode = {kind}")?;
            project.diagnostic(format!(
                "{}: unresolved mechanics card {kind}; values remain in Rete",
                value.path
            ));
        }
    }
    project.diagnostic(format!(
        "{}: typed mechanics parameters; original dynamics simulation remains unsupported",
        value.path
    ));
    project.write(
        disk_path(&project.selection, &value.path, "tres"),
        text.as_bytes(),
    )
}

fn bounds(
    project: &mut Project,
    source: &mut impl ContentSource,
    family: &Family,
    code: &mut String,
) -> Result<()> {
    let Some(reference) = &family.bounding_geometry else {
        return Ok(());
    };
    let geometry = source.load(reference)?;
    geometry.validate_reference(&reference.path)?;
    let geometry = geometry.resolve(source)?;
    code.push_str("    var bounds = Node3D.new()\n    bounds.name = \"FamilyBounds\"\n    bounds.visible = false\n    actor.add_child(bounds)\n    bounds.owner = actor\n    var bounds_material = StandardMaterial3D.new()\n    bounds_material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED\n    bounds_material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA\n    bounds_material.albedo_color = Color(0.1, 1.0, 0.4, 0.18)\n    bounds_material.cull_mode = BaseMaterial3D.CULL_DISABLED\n");
    let mut i = 0;
    for element in &geometry.elements {
        let mut shapes = Vec::new();
        match element {
            crate::model::collision::CollisionElement::Spheres(spheres) => {
                for s in spheres {
                    if s.radius > 0. {
                        shapes.push((
                            "SphereMesh.new()".to_string(),
                            format!(
                                "    bound_{i}_mesh.radius = {}\n    bound_{i}_mesh.height = {}\n",
                                s.radius,
                                2. * s.radius
                            ),
                            vector(geometry.positions[s.center as usize]),
                        ));
                        i += 1;
                    }
                }
            }
            crate::model::collision::CollisionElement::Boxes(boxes) => {
                for b in boxes {
                    let a = vector(geometry.positions[b.min as usize]);
                    let b = vector(geometry.positions[b.max as usize]);
                    shapes.push((
                        "BoxMesh.new()".into(),
                        format!(
                            "    bound_{i}_mesh.size = {}\n",
                            vec3(std::array::from_fn(|c| (b[c] - a[c]).abs()))
                        ),
                        std::array::from_fn(|c| (a[c] + b[c]) * 0.5),
                    ));
                    i += 1;
                }
            }
            _ => project.diagnostic(format!(
                "{}: triangle family bounds have no debug primitive projection",
                reference.path
            )),
        }
        let start = i - shapes.len();
        for (i, (mesh, properties, position)) in (start..).zip(shapes) {
            writeln!(
                code,
                "    var bound_{i} = MeshInstance3D.new()\n    bound_{i}.name = \"Bound_{i}\"\n    var bound_{i}_mesh = {mesh}\n{properties}    bound_{i}.mesh = bound_{i}_mesh\n    bound_{i}.material_override = bounds_material\n    bound_{i}.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF\n    bound_{i}.position = {}\n    bound_{i}.set_meta(\"family_bounds_debug\", true)\n    bounds.add_child(bound_{i})\n    bound_{i}.owner = actor",
                vec3(position)
            )?;
        }
    }
    Ok(())
}
impl Project {
    pub(super) fn character_node(
        &mut self,
        source: &mut impl ContentSource,
        reference: &AssetRef<Perso>,
        parent: &str,
        owner: &str,
    ) -> Result<()> {
        let perso = source.load(reference)?;
        perso.validate_reference(&reference.path)?;
        let path = perso_path(self, &perso);
        ensure!(self.generated.contains(&path), "missing character scene");
        writeln!(
            self.builder,
            "    var {parent}_character = load({}).instantiate()\n    {parent}.add_child({parent}_character)\n    {parent}_character.owner = {owner}",
            quoted(&format!("res://{path}"))
        )?;
        Ok(())
    }
}
