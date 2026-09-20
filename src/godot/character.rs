//! Persisted native characters and family/configuration-specialized animation libraries.
use super::{
    Project,
    animation::{self, NativeAnimation, Pose},
    disk_path, quoted, vec3, vector,
};
use crate::model::{
    AssetPath, AssetRef, animation::*, character::*, content::ContentSource, io::Content,
    mechanics::*,
};
use anyhow::{Result, ensure};
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
    instance: String,
    scale: Option<[f32; 3]>,
    geometries: Vec<String>,
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
        "extends {}\n## Compiled state activation of persisted native movement shapes.\nfunc _state_movement_collision(id: int) -> void:\n    match id:\n",
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
    let configuration = perso
        .configuration
        .as_ref()
        .or(family.default_configuration.as_ref())
        .or(perso.initial_configuration.as_ref());
    let config = configuration.map(|r| source.load(r)).transpose()?;
    if let (Some(reference), Some(config)) = (configuration, &config) {
        config.validate_reference(&reference.path)?;
        config.validate()?;
    }
    // The target library is specialized in this active level; no Fix binding is cached globally.
    let key = format!(
        "{}|{}",
        family.path,
        configuration.map_or("none", |r| r.path.as_str())
    );
    let rig_id = format!("rig_{}", &super::digest(key.as_bytes())[..12]);
    if !project.character_rigs.contains(&rig_id) {
        export_rig(
            project,
            source,
            &family,
            config.as_deref(),
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
    let controller = format!(
        "res://{}/actions.gd",
        rig_directory(project, &family, &rig_id)
    );
    movement_collision(project, source, perso, &name, &controller)?;
    writeln!(
        project.builder,
        "    {name}.initial_state = {state}\n    {name}.current_state = {state}\n    if {state} >= 0:\n        apply_initial_pose({name}, {name}.get_node(\"AnimationPlayer\").get_animation(\"state_{state}\"))"
    )?;
    super::ai::component(project, source, perso, &name, written)?;
    writeln!(
        project.builder,
        "    save_scene({name}, {})\n    {name}.free()",
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
    config: Option<&ObjectList>,
    rig_id: &str,
    written: &mut BTreeSet<AssetPath>,
) -> Result<()> {
    let directory = rig_directory(project, family, rig_id);
    let mut states = Vec::new();
    let mut channels = BTreeSet::new();
    let mut slots: BTreeMap<u16, BTreeSet<u16>> = BTreeMap::new();
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
                    slots.entry(channel).or_default().insert(slot);
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
    if let Some(config) = config {
        for (restored, table) in [
            (false, Some(&config.parts)),
            (true, config.reset_parts.as_ref()),
        ] {
            let Some(table) = table else {
                continue;
            };
            for (&channel, slots) in &slots {
                for &slot in slots {
                    let Some(part) = table.get(usize::from(slot)) else {
                        continue;
                    };
                    let Some(reference) = &part.instance else {
                        continue;
                    };
                    let instance = source.load(reference)?;
                    instance.validate_reference(&reference.path)?;
                    let model = source.load(&instance.model)?;
                    if written.insert(instance.path.clone()) {
                        super::export::export_instance(
                            project, source, &model, &instance, written,
                        )?;
                    }
                    let mut geometries = vec![String::new()];
                    for (e, element) in model.sprite_elements.iter().enumerate() {
                        for (i, placement) in element.sprites.iter().enumerate() {
                            let sprite = source.load(&placement.sprite)?;
                            for (v, view) in sprite.views.iter().enumerate() {
                                if view.material.is_some() {
                                    geometries.push(format!("/Sprite_{e}_{i}/View_{v}"));
                                }
                            }
                        }
                    }
                    parts.push(Part {
                        channel,
                        slot,
                        restored,
                        instance: format!(
                            "res://{}",
                            disk_path(&project.selection, &instance.path, "tscn")
                        ),
                        scale: part.scale,
                        geometries,
                    });
                }
            }
        }
    }
    let mut libraries = Vec::new();
    for restored in [false, true] {
        if restored && config.is_none_or(|c| c.reset_parts.is_none()) {
            continue;
        }
        let mut links = String::new();
        let mut entries = Vec::new();
        for state in &states {
            let name = format!("state_{}", state.state.id);
            let path = format!(
                "{directory}/{}{name}.tres",
                if restored { "restored/" } else { "" }
            );
            export_animation(
                project, source, state, &channels, &parts, restored, &path, written,
            )?;
            let ext = format!("s_{}", state.state.id);
            writeln!(
                links,
                "[ext_resource type=\"Animation\" path={} id=\"{ext}\"]",
                quoted(&format!("res://{path}"))
            )?;
            entries.push(format!("&{}: ExtResource(\"{ext}\")", quoted(&name)));
        }
        let path = format!(
            "{directory}/{}animations.tres",
            if restored { "restored/" } else { "" }
        );
        project.write(path.clone(),format!("[gd_resource type=\"AnimationLibrary\" load_steps={} format=3]\n\n{links}\n[resource]\n_data = {{ {} }}\n",states.len()+1,entries.join(",\n")).as_bytes())?;
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
    for channel in channels {
        writeln!(
            function,
            "    var c_{channel} = Node3D.new()\n    c_{channel}.name = \"channel_{channel}\"\n    channels.add_child(c_{channel})\n    c_{channel}.owner = actor\n    c_{channel}.set_meta(\"source_channel\", {})\n    c_{channel}.set_meta(\"source_channel_occurrence\", {})",
            channel % 256,
            channel / 256
        )?;
    }
    for (i, part) in parts.iter().enumerate() {
        writeln!(
            function,
            "    var part_{i} = native_resource({}).instantiate()\n    part_{i}.name = {}\n    c_{}.add_child(part_{i})\n    part_{i}.owner = actor\n    part_{i}.visible = false\n    part_{i}.set_meta(\"configuration_slot\", {})",
            quoted(&part.instance),
            quoted(part.node().rsplit('/').next().unwrap()),
            part.channel,
            part.slot
        )?;
        if let Some([x, y, z]) = part.scale {
            writeln!(function, "    part_{i}.scale = {}", vec3([x, z, y]))?;
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
    parts: &[Part],
    restored: bool,
    path: &str,
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
    for channel in channels {
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
    for part in parts {
        let active = |f: &BTreeMap<u16, Pose>| {
            f.get(&part.channel)
                .filter(|s| restored == part.restored && s.object == Some(part.slot))
                .map(|s| (s.visible, s.opacity))
        };
        anim.values(
            &format!("{}:visible", part.node()),
            frames
                .iter()
                .map(|f| active(f).is_some_and(|s| s.0).to_string()),
            fps,
        )?;
        for geometry in &part.geometries {
            anim.values(
                &format!("{}{geometry}:transparency", part.node()),
                frames.iter().map(|f| {
                    active(f)
                        .map_or(0., |s| 1. - f32::from(s.1) / 255.)
                        .to_string()
                }),
                fps,
            )?;
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
            keys.push((f64::from(frame)/f64::from(fps.max(1)),format!("{{\"method\": &\"_cues\", \"args\": [{frame}, PackedInt32Array({}), PackedInt32Array({}), [{}]]}}",cues.iter().map(|(_,c)|c.channel.to_string()).collect::<Vec<_>>().join(","),cues.iter().map(|(_,c)|c.configuration_slot.to_string()).collect::<Vec<_>>().join(","),cues.iter().map(|(i,_)|format!("ExtResource(\"cue_{i}\")")).collect::<Vec<_>>().join(","))));
        }
        anim.methods(keys)?;
    }
    project.write(
        path.into(),
        format!(
            "[gd_resource type=\"Animation\" load_steps={} format=3]\n\n{links}\n{}",
            links.lines().count() + 1,
            anim.text
        )
        .as_bytes(),
    )
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
    if kind != "sound" {
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
    match &value.data {
        MechanicsData::Mass { mass } => writeln!(text, "mass = {mass}")?,
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
