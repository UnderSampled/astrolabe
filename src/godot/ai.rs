//! Typed AI trees become ordinary GDScript control flow and native resource calls.
use super::{Project, quoted};
use crate::model::{
    AssetPath,
    ai::{walk::AstNode, *},
    character::{ObjectList, Perso},
    content::ContentSource,
    io::Content,
};
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeSet, fmt::Write as _};

fn waypoint_constant(reference: &crate::model::AssetRef<crate::model::route::Waypoint>) -> String {
    format!(
        "waypoint_{}",
        super::digest(reference.path.as_str().as_bytes())
    )
}
fn configuration_constant(reference: &crate::model::AssetRef<ObjectList>) -> String {
    format!(
        "configuration_{}",
        super::digest(reference.path.as_str().as_bytes())
    )
}

fn waypoint_resource_constant(
    reference: &crate::model::AssetRef<crate::model::route::Waypoint>,
) -> String {
    format!(
        "waypoint_asset_{}",
        super::digest(reference.path.as_str().as_bytes())
    )
}

type WaypointBinding = (
    crate::model::AssetRef<crate::model::route::Waypoint>,
    Option<[f32; 3]>,
    Option<String>,
);

fn model_script_file(path: &AssetPath) -> String {
    let file = path.file();
    if path.role() == crate::model::PackageRole::Fix {
        let (directory, name) = file.rsplit_once('/').unwrap_or(("", file));
        format!("{directory}/fix/{name}.gd")
    } else {
        format!("{file}.gd")
    }
}

fn actor_target_label(target: &ActorTarget) -> String {
    match target {
        ActorTarget::Keyword(ActorKeyword::SelfActor) => "Me".into(),
        ActorTarget::Keyword(ActorKeyword::MainActor) => "MainActor".into(),
        ActorTarget::Reference(actor) => actor.path.as_str().into(),
        ActorTarget::Variable(index) => format!("dsg_{index}"),
    }
}

pub(super) fn export_program(
    project: &mut Project,
    source: &mut impl ContentSource,
    model: &AiModel,
    written: &mut BTreeSet<AssetPath>,
) -> Result<String> {
    model.validate()?;
    for variable in &model.variables {
        if let DesignerValue::ActorList(list) = &variable.default {
            ensure!(
                list.active_len == 0,
                "populated source actor-list defaults need scene actor binding"
            );
        }
    }
    for (actor, field) in model.actor_field_writes() {
        let target = source.load(&actor)?;
        let Some(brain) = &target.brain else {
            project.diagnostic(format!(
                "{} actor field write {}.{} has no saved Brain, so its field cannot be validated; a reached write will fail explicitly",
                model.path, actor.path, field
            ));
            continue;
        };
        let target_model = source.load(&brain.model)?;
        brain.validate_model(&target_model)?;
        ensure!(
            usize::from(field) < target_model.variables.len(),
            "{} actor field {field} is absent from {}",
            model.path,
            actor.path
        );
    }
    for (actor, field) in model.actor_field_reads() {
        let target = source.load(&actor)?;
        let Some(brain) = &target.brain else {
            project.diagnostic(format!(
                "{} actor field read {}.{} has no saved Brain; a reached read will fail explicitly",
                model.path, actor.path, field
            ));
            continue;
        };
        let target_model = source.load(&brain.model)?;
        brain.validate_model(&target_model)?;
        if usize::from(field) >= target_model.variables.len() {
            project.diagnostic(format!(
                "{} actor field read {}.{} is absent from the target Brain; a reached read will fail explicitly",
                model.path, actor.path, field
            ));
        }
    }
    for reference in model.behavior_references() {
        if reference.model.path != model.path {
            let target = source.load(&reference.model)?;
            reference.validate_model(&target)?;
        }
    }
    // Fix scripts may carry level-bound actor references. Compile their
    // level-specific projection beside that level's Perso scenes rather than
    // publishing one shared GDScript whose bindings a later level overwrites.
    // Numeric model filenames are only unique inside a package role. Give Fix
    // scripts their own subdirectory so they cannot alias a level model with
    // the same source number in one exported map.
    let path = project.level_path(&model_script_file(&model.path));
    if project.generated.contains(&path) || !written.insert(model.path.clone()) {
        return Ok(path);
    }
    let mut templates = Vec::new();
    for reference in model.object_templates() {
        let perso = source.load(&reference)?;
        perso.validate_reference(&reference.path)?;
        super::character::export_perso(project, source, &perso, written)?;
        templates.push((
            reference,
            format!("res://{}", super::character::perso_path(project, &perso)),
        ));
    }
    let mut events = Vec::new();
    for event in model.sound_events() {
        if events.iter().any(|(e, _)| e == event) {
            continue;
        }
        let resource = super::sound::export_event(project, source, event, written)?;
        events.push((event.clone(), resource));
    }
    let family = model
        .state_references()
        .first()
        .map(|r| source.load(&r.family))
        .transpose()?;
    let mut waypoints = Vec::new();
    for reference in model.waypoint_references() {
        if waypoints.iter().any(|(r, _, _)| r == reference) {
            continue;
        }
        let point = source.load(reference)?;
        point.validate_reference(&reference.path)?;
        point.validate()?;
        // Fixed waypoints lower to native vector constants at conversion,
        // just as route points lower to Curve3D control points. Relative
        // bindings remain explicit failures when the source reaches them.
        let resource = super::route::export_waypoint(project, &point)?;
        waypoints.push((
            reference.clone(),
            point.relative_actor.is_none().then_some(point.position),
            Some(format!("res://{resource}")),
        ));
    }
    let mut particles = Vec::new();
    for reference in model.particle_generators() {
        let generator = source.load(&reference)?;
        generator.validate_reference(&reference.path)?;
        let path = super::particle::export(project, source, &generator, written)?;
        particles.push((reference, format!("res://{path}")));
    }
    let mut configurations = Vec::new();
    for reference in model.configuration_references() {
        let configuration = source.load(reference)?;
        configuration.validate_reference(&reference.path)?;
        let resource = super::character::export_configuration(project, &configuration)?;
        configurations.push((reference.clone(), resource));
    }
    let script = compile_with_bindings(
        model,
        &events,
        family.as_deref(),
        &templates,
        &waypoints,
        &particles,
        &configurations,
    )?;
    project.write(path.clone(), script.as_bytes())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AssetRef, Selection,
        ai::{
            Behavior, BehaviorSet, DesignerValue, DesignerVariable, Expression, Script, Statement,
        },
        character::Perso,
        content::Collection,
    };
    use anyhow::Result;
    use std::fs;

    #[test]
    fn field_write_to_brainless_source_target_keeps_reached_failure() -> Result<()> {
        let target = AssetRef::<Perso>::new("level:/persos/world/0/88")?;
        let mut collection = Collection::default();
        collection.insert(
            &target,
            Perso {
                path: target.path.clone(),
                name: "iThievesCommun_Brigand_01".into(),
                family: AssetRef::new("level:/families/0")?,
                initial_state: None,
                current_state: None,
                secondary_state: None,
                configuration: None,
                initial_configuration: None,
                object_types: [0; 3],
                camera: None,
                hit_points: None,
                inventory_stock: None,
                motion_binding_present: false,
                picking_bounds: None,
                contact_eligible: true,
                microphone: None,
                routes: Vec::new(),
                brain: None,
                collisions: None,
                particles: None,
            },
        )?;
        let model = AiModel {
            path: AssetPath::new("level:/ai/models/brainless_target_writer")?,
            name: "Brainless Target Writer".into(),
            normal: Some(BehaviorSet {
                behaviors: vec![Behavior {
                    id: 0,
                    rules: Vec::new(),
                    schedule: Some(Script {
                        statements: vec![Statement::ActorFieldAssign {
                            actor: target,
                            variable: 36,
                            value: Expression::Integer(1),
                        }],
                    }),
                    unknown: [0; 3],
                }],
                default_behavior: Some(0),
                action_capacity: 1,
                unknown: [0; 3],
            }),
            reflex: None,
            variables: vec![DesignerVariable {
                default: DesignerValue::Integer(0),
                save: 0,
                initialize: 0,
            }],
            designer_unknown: [0; 3],
            unknown: [0; 4],
        };
        let output = tempfile::tempdir()?;
        let mut project = Project::new(output.path(), Selection::hype("brigand", "en")?)?;
        let mut written = BTreeSet::new();
        let script_path = export_program(&mut project, &mut collection, &model, &mut written)?;
        let script = fs::read_to_string(project.stage.path().join(&script_path))?;

        assert!(
            project.diagnostics.iter().any(|diagnostic| {
                diagnostic.contains("level:/persos/world/0/88.36")
                    && diagnostic.contains("a reached write will fail explicitly")
            }),
            "diagnostics: {:?}",
            project.diagnostics
        );
        assert!(
            script.contains("if not is_instance_valid(actor_brain_0_target_0)"),
            "{script}"
        );
        assert!(
            script.contains("requires actor level:/persos/world/0/88 Brain field 36"),
            "{script}"
        );
        assert!(script_path.ends_with(".gd"));
        Ok(())
    }
}
/// Exposed for authored tooling and focused tests; dependencies have already been
/// resolved to native resource paths by this adapter, never by generated code.
pub fn compile(
    model: &AiModel,
    events: &[(crate::model::audio::SoundEventLink, String)],
) -> Result<String> {
    compile_for_family(model, events, None)
}
pub fn compile_for_family(
    model: &AiModel,
    events: &[(crate::model::audio::SoundEventLink, String)],
    family: Option<&crate::model::character::Family>,
) -> Result<String> {
    compile_with_templates(model, events, family, &[], &[], &[])
}
fn compile_with_templates(
    model: &AiModel,
    events: &[(crate::model::audio::SoundEventLink, String)],
    family: Option<&crate::model::character::Family>,
    templates: &[(crate::model::AssetRef<Perso>, String)],
    waypoints: &[WaypointBinding],
    particles: &[(
        crate::model::AssetRef<crate::model::particle::ParticleGenerator>,
        String,
    )],
) -> Result<String> {
    compile_with_bindings(model, events, family, templates, waypoints, particles, &[])
}
fn compile_with_bindings(
    model: &AiModel,
    events: &[(crate::model::audio::SoundEventLink, String)],
    family: Option<&crate::model::character::Family>,
    templates: &[(crate::model::AssetRef<Perso>, String)],
    waypoints: &[WaypointBinding],
    particles: &[(
        crate::model::AssetRef<crate::model::particle::ParticleGenerator>,
        String,
    )],
    configurations: &[(crate::model::AssetRef<ObjectList>, String)],
) -> Result<String> {
    model.validate()?;
    for action in model.state_references() {
        action.validate_family(
            family.ok_or_else(|| anyhow::anyhow!("AI action requires its receiving family"))?,
        )?;
    }
    let mut out = format!(
        "extends \"res://addons/astrolabe/behavior/behavior.gd\"\n## Compiled from {}. Edit the useful AI asset and regenerate.\n",
        model.path.as_str().replace(['\n', '\r'], " ")
    );
    writeln!(
        out,
        "@export var source_model_path: String = {}",
        quoted(model.path.as_str())
    )?;
    for reference in model.waypoint_references() {
        ensure!(
            waypoints.iter().any(|(r, _, _)| r == reference),
            "AI waypoint requires conversion binding: {}",
            reference.path
        );
    }
    let mut waypoint_resource_references = Vec::new();
    for expression in model.expressions() {
        if let Expression::WaypointReference(reference) = expression {
            waypoint_resource_references.push(reference);
        }
    }
    for variable in &model.variables {
        match &variable.default {
            DesignerValue::Waypoint(Some(reference)) => {
                waypoint_resource_references.push(reference)
            }
            DesignerValue::WaypointArray(array) => {
                waypoint_resource_references.extend(array.values.iter().flatten());
            }
            _ => {}
        }
    }
    for reference in waypoint_resource_references {
        ensure!(
            waypoints
                .iter()
                .any(|(candidate, _, resource)| candidate == reference && resource.is_some()),
            "AI waypoint requires native resource binding: {}",
            reference.path
        );
    }
    for reference in model.configuration_references() {
        ensure!(
            configurations.iter().any(|(target, _)| target == reference),
            "AI configuration requires conversion binding: {}",
            reference.path
        );
    }
    for (reference, resource) in configurations {
        ensure!(
            resource.starts_with("res://"),
            "AI configuration resource must be project-local"
        );
        writeln!(
            out,
            "const {} = preload({})",
            configuration_constant(reference),
            quoted(resource)
        )?;
    }
    let mut waypoint_accessors = String::new();
    for (reference, position, resource) in waypoints {
        let value = position.map(super::vec3).unwrap_or_else(|| "null".into());
        writeln!(out, "const {} = {value}", waypoint_constant(reference))?;
        if let Some(resource) = resource {
            ensure!(
                resource.starts_with("res://"),
                "AI waypoint resource must be project-local"
            );
            writeln!(
                waypoint_accessors,
                "func {}() -> Resource:\n    return load({}) as Resource",
                waypoint_resource_constant(reference),
                quoted(resource)
            )?;
        }
    }
    for reference in model.particle_generators() {
        let (_, resource) = particles
            .iter()
            .find(|(r, _)| r == &reference)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "particle generator requires native conversion binding: {}",
                    reference.path
                )
            })?;
        writeln!(
            out,
            "const {} = preload({})",
            super::particle::constant(&reference),
            quoted(resource)
        )?;
    }
    out.push_str("const NativeContactPickup = preload(\"res://addons/astrolabe/character/contact_pickup.gd\")\n");
    let actor_targets = model.actor_field_targets();
    for (i, _) in actor_targets.iter().enumerate() {
        writeln!(out, "@export var actor_brain_{i}_path: NodePath")?;
        writeln!(out, "var actor_brain_{i}: Node")?;
    }
    writeln!(
        out,
        "func source_external_actor_brains() -> PackedStringArray:"
    )?;
    writeln!(
        out,
        "    return PackedStringArray([{}])",
        actor_targets
            .iter()
            .map(|actor| quoted(actor.path.as_str()))
            .collect::<Vec<_>>()
            .join(", ")
    )?;
    for (i, reference) in model.object_templates().iter().enumerate() {
        let path = templates
            .iter()
            .find(|(target, _)| target == reference)
            .map_or("", |(_, path)| path.as_str());
        writeln!(out, "const object_template_{i}_path = {}", quoted(path))?;
    }
    for (i, (_, path)) in events.iter().enumerate() {
        ensure!(
            path.starts_with("res://"),
            "AI sound resource must be project-local"
        );
        writeln!(out, "const sound_{i} = preload({})", quoted(path))?;
    }
    let mut waypoint_default_initializers = Vec::new();
    for (i, v) in model.variables.iter().enumerate() {
        let default = match &v.default {
            DesignerValue::Waypoint(Some(_)) => {
                waypoint_default_initializers.push((
                    i,
                    literal(&v.default, waypoints, false)?,
                    false,
                ));
                "null".to_owned()
            }
            DesignerValue::WaypointArray(_) => {
                waypoint_default_initializers.push((
                    i,
                    literal(&v.default, waypoints, false)?,
                    true,
                ));
                "[]".to_owned()
            }
            DesignerValue::SourceWaypointArray(_) => {
                waypoint_default_initializers.push((
                    i,
                    literal(&v.default, waypoints, true)?,
                    true,
                ));
                "[]".to_owned()
            }
            _ => literal(&v.default, waypoints, true)?,
        };
        writeln!(
            out,
            "@export var dsg_{i}: {} = {}",
            value_type(&v.default),
            default
        )?;
    }
    if !waypoint_default_initializers.is_empty() {
        out.push_str("\nfunc _init() -> void:\n    super()\n");
        for (i, value, is_array) in waypoint_default_initializers {
            if is_array {
                writeln!(out, "    dsg_{i}.assign({value})")?;
            } else {
                writeln!(out, "    dsg_{i} = {value}")?;
            }
        }
    }
    if !waypoint_accessors.is_empty() {
        out.push('\n');
        out.push_str(&waypoint_accessors);
    }
    let inputs: BTreeSet<_> = model
        .expressions()
        .into_iter()
        .filter_map(|e| match e {
            Expression::Button { action, .. } => Some(action.as_str()),
            _ => None,
        })
        .collect();
    if !inputs.is_empty() {
        out.push_str("\nfunc _bind_inputs() -> void:\n");
        for action in inputs {
            // Physical defaults for the selected Hype control path. Authored
            // InputMap entries always win; other logical controls need bindings.
            let key = match action {
                "Button_Up" => "KEY_UP",
                "Button_Down" => "KEY_DOWN",
                "Button_Left" => "KEY_LEFT",
                "Button_Right" => "KEY_RIGHT",
                "Button_SpeedUp" => "KEY_SHIFT",
                "Button_Jump" => "KEY_CTRL",
                "Button_Hit" => "KEY_SPACE",
                "Button_GetSword" => "KEY_X",
                "Button_GetCrossbow" => "KEY_Z",
                "Button_Magic" => "KEY_ENTER",
                _ => "KEY_NONE",
            };
            writeln!(out, "    adopt_input({}, {key})", quoted(action))?;
        }
    }
    for (group, set) in [("normal", &model.normal), ("reflex", &model.reflex)] {
        writeln!(
            out,
            "\nfunc _enter_{group}(token: int) -> void:\n    match current_{group}:"
        )?;
        for behavior in set.iter().flat_map(|s| &s.behaviors) {
            writeln!(
                out,
                "        {}: {group}_{}(token)",
                behavior.id, behavior.id
            )?;
        }
        writeln!(
            out,
            "        -1: pass\n        _:\n            push_error(\"Unknown compiled {group} behavior\")\n            stop()"
        )?;
        for behavior in set.iter().flat_map(|s| &s.behaviors) {
            let id = behavior.id;
            writeln!(
                out,
                "\nfunc {group}_{id}(token: int) -> void:\n    var schedule_started := false\n    while active_{group}(token):\n        var continue_rules := true"
            )?;
            for (rule, _) in behavior
                .rules
                .iter()
                .enumerate()
                .filter(|(_, r)| r.is_some())
            {
                writeln!(
                    out,
                    "        if continue_rules:\n            continue_rules = await {group}_{id}_rule_{rule}(token)\n            if not active_{group}(token): return"
                )?;
            }
            if behavior.schedule.is_some() {
                writeln!(
                    out,
                    "        if not {group}_rule_claimed:\n            if not schedule_started:\n                schedule_started = true\n                {group}_{id}_schedule(token)\n            else:\n                {group}_schedule_tick.emit()"
                )?;
            }
            writeln!(out, "        await {group}_tick")?;
            for (rule, script) in behavior
                .rules
                .iter()
                .enumerate()
                .filter_map(|(i, r)| r.as_ref().map(|r| (i, r)))
            {
                writeln!(out, "\nfunc {group}_{id}_rule_{rule}(token: int) -> bool:")?;
                statements(
                    &mut out,
                    &script.statements,
                    model,
                    events,
                    1,
                    &mut Flow {
                        group,
                        rule: true,
                        local: 0,
                    },
                )?;
                out.push_str("    return true\n");
            }
            if let Some(script) = &behavior.schedule {
                writeln!(
                    out,
                    "\nfunc {group}_{id}_schedule(token: int) -> void:\n    while active_{group}(token):"
                )?;
                statements(
                    &mut out,
                    &script.statements,
                    model,
                    events,
                    2,
                    &mut Flow {
                        group,
                        rule: false,
                        local: 0,
                    },
                )?;
                writeln!(out, "        await {group}_schedule_tick")?;
            }
        }
    }
    Ok(out)
}
fn value_type(v: &DesignerValue) -> &'static str {
    match v {
        DesignerValue::Boolean(_) => "bool",
        DesignerValue::Integer(_)
        | DesignerValue::Byte(_)
        | DesignerValue::UnsignedByte(_)
        | DesignerValue::Short(_)
        | DesignerValue::UnsignedShort(_)
        | DesignerValue::UnsignedInteger(_)
        | DesignerValue::Action(_) => "int",
        DesignerValue::Real(_) => "float",
        DesignerValue::Vector(_) => "Vector3",
        DesignerValue::IntegerArray(_) | DesignerValue::ActionArray(_) => "PackedInt32Array",
        DesignerValue::RealArray(_) => "PackedFloat32Array",
        DesignerValue::VectorArray(_) => "PackedVector3Array",
        DesignerValue::ActorList(_) => "Array[Node3D]",
        DesignerValue::Actor(_) | DesignerValue::SourceActor(_) => "Node3D",
        DesignerValue::SourceWaypoint(_) | DesignerValue::SourceWaypointArray(_) => "Variant",
        DesignerValue::Waypoint(_) => "Resource",
        DesignerValue::WaypointArray(_) => "Array[Resource]",
        DesignerValue::Opaque { .. } => "PackedByteArray",
    }
}
fn literal(
    v: &DesignerValue,
    waypoints: &[WaypointBinding],
    use_preloaded_constants: bool,
) -> Result<String> {
    let array = |kind: &str, values: Vec<String>| format!("{kind}([{}])", values.join(", "));
    let waypoint = |reference: &crate::model::AssetRef<crate::model::route::Waypoint>| {
        waypoints
            .iter()
            .find(|(candidate, _, _)| candidate == reference)
            .and_then(|(_, _, resource)| resource.as_ref())
            .map(|resource| {
                if use_preloaded_constants {
                    format!("{}()", waypoint_resource_constant(reference))
                } else {
                    format!("load({})", quoted(resource))
                }
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "designer waypoint requires native resource binding: {}",
                    reference.path
                )
            })
    };
    Ok(match v {
        DesignerValue::Boolean(v) => v.to_string(),
        DesignerValue::Integer(v) => v.to_string(),
        DesignerValue::Byte(v) => v.to_string(),
        DesignerValue::UnsignedByte(v) => v.to_string(),
        DesignerValue::Short(v) => v.to_string(),
        DesignerValue::UnsignedShort(v) => v.to_string(),
        DesignerValue::UnsignedInteger(v) => v.to_string(),
        DesignerValue::Real(v) => format!("{v:?}"),
        // Keep AI vector components in source coordinates. The native API owns
        // conversion when a vector crosses into Godot position or motion.
        DesignerValue::Vector(v) => super::vec3(*v),
        DesignerValue::IntegerArray(v) => array(
            "PackedInt32Array",
            v.values.iter().map(ToString::to_string).collect(),
        ),
        DesignerValue::RealArray(v) => array(
            "PackedFloat32Array",
            v.values.iter().map(|v| format!("{v:?}")).collect(),
        ),
        DesignerValue::VectorArray(v) => array(
            "PackedVector3Array",
            v.values.iter().map(|v| super::vec3(*v)).collect(),
        ),
        DesignerValue::Action(v) => v.as_ref().map_or(-1, |v| i64::from(v.state)).to_string(),
        DesignerValue::ActionArray(v) => array(
            "PackedInt32Array",
            v.values
                .iter()
                .map(|v| v.as_ref().map_or(-1, |v| i64::from(v.state)).to_string())
                .collect(),
        ),
        // Node references are assigned in the saved scene, after its actors exist.
        DesignerValue::Actor(_) | DesignerValue::SourceActor(_) => "null".into(),
        DesignerValue::SourceWaypoint(_) => "null".into(),
        DesignerValue::SourceWaypointArray(v) => {
            format!("[{}]", vec!["null"; v.values.len()].join(", "))
        }
        DesignerValue::Waypoint(value) => value
            .as_ref()
            .map(waypoint)
            .transpose()?
            .unwrap_or_else(|| "null".into()),
        DesignerValue::WaypointArray(value) => format!(
            "[{}]",
            value
                .values
                .iter()
                .map(|value| value.as_ref().map(waypoint).transpose())
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .map(|value| value.unwrap_or_else(|| "null".into()))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        DesignerValue::ActorList(_) => "[]".into(),
        DesignerValue::Opaque { bytes, .. } => format!(
            "PackedByteArray([{}])",
            bytes
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    })
}
fn opaque_accessed_variable(statement: &Statement, model: &AiModel) -> Option<u16> {
    // The outer If owns its condition; nested statements are compiled by the
    // recursive call and must not fail before their branch is reached.
    let statement = match statement {
        Statement::If { condition, .. } => Statement::If {
            condition: condition.clone(),
            then: Vec::new(),
            otherwise: None,
        },
        _ => statement.clone(),
    };
    let scan = Script {
        statements: vec![statement],
    };
    for (node, _) in scan.walk() {
        let index = match node {
            AstNode::Expression(Expression::Variable(index) | Expression::SelfFieldRead(index)) => {
                Some(*index)
            }
            AstNode::Expression(Expression::ActorListContains { list, .. }) => Some(*list),
            AstNode::Statement(
                Statement::Assign { variable, .. }
                | Statement::MainActorFieldAssign { variable, .. }
                | Statement::SubtractAssign { variable, .. }
                | Statement::AddAssign { variable, .. }
                | Statement::MultiplyAssign { variable, .. },
            ) => Some(*variable),
            AstNode::Statement(Statement::VectorComponentAssign { vector, .. }) => Some(*vector),
            AstNode::Statement(Statement::ArrayAssign {
                array: Expression::Variable(index),
                ..
            }) => Some(*index),
            AstNode::Statement(
                Statement::Increment {
                    target: FieldTarget::Variable(index),
                }
                | Statement::Decrement {
                    target: FieldTarget::Variable(index),
                },
            ) => Some(*index),
            AstNode::Statement(Statement::ActorListAdd { list, .. }) => Some(*list),
            _ => None,
        };
        if index.is_some_and(|index| {
            model
                .variables
                .get(usize::from(index))
                .is_some_and(|variable| matches!(variable.default, DesignerValue::Opaque { .. }))
        }) {
            return index;
        }
    }
    None
}
fn unresolved_variable(model: &AiModel, index: u16) -> bool {
    model
        .variables
        .get(usize::from(index))
        .is_some_and(|variable| {
            matches!(
                variable.default,
                DesignerValue::SourceWaypoint(_)
                    | DesignerValue::SourceWaypointArray(_)
                    | DesignerValue::SourceActor(_)
            )
        })
}
fn actor_expression(e: &Expression, model: &AiModel) -> bool {
    match e {
        Expression::Actor(_) => true,
        Expression::Variable(index) => {
            model
                .variables
                .get(usize::from(*index))
                .is_some_and(|variable| {
                    matches!(
                        variable.default,
                        DesignerValue::Actor(_) | DesignerValue::SourceActor(_)
                    )
                })
        }
        _ => false,
    }
}
fn actor_target_code(actor: &ActorTarget) -> String {
    match actor {
        ActorTarget::Keyword(ActorKeyword::SelfActor) => "character()".into(),
        ActorTarget::Keyword(ActorKeyword::MainActor) => "main_actor".into(),
        ActorTarget::Variable(index) => format!("dsg_{index}"),
        ActorTarget::Reference(reference) => format!(
            "character().source_actor_node({})",
            quoted(reference.path.as_str())
        ),
    }
}
fn armor_receiver(
    out: &mut String,
    model: &AiModel,
    depth: usize,
    flow: &mut Flow,
    selected: bool,
) -> Result<String> {
    let local = format!("armor_state_{}", flow.local);
    flow.local += 1;
    let pad = "    ".repeat(depth);
    let exit = flow.exit();
    let failure = quoted(&format!(
        "{} requires native shared armor{}",
        model.path,
        if selected {
            " with a selected definition"
        } else {
            ""
        }
    ));
    writeln!(
        out,
        "{pad}var {local}: Resource = source_armor_catalog()\n{pad}if {local} == null or not {local}.has_method(\"active\"):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}"
    )?;
    if selected {
        writeln!(
            out,
            "{pad}if {local}.call(\"active\") == null:\n{pad}    fail_unsupported({failure})\n{pad}    {exit}"
        )?;
    }
    Ok(local)
}
fn actor_list_capacity(model: &AiModel, index: u16) -> Result<usize> {
    match model.variables.get(usize::from(index)).map(|v| &v.default) {
        Some(DesignerValue::ActorList(list)) => Ok(list.slots.len()),
        _ => anyhow::bail!("AI field {index} is not a source actor list"),
    }
}
fn vector_expression(e: &Expression, model: &AiModel) -> bool {
    match e {
        Expression::Vector(_)
        | Expression::SelfPosition
        | Expression::ActorPosition(_)
        | Expression::SelfPositionField
        | Expression::WaypointPosition(_)
        | Expression::SelfSighting
        | Expression::LaserDirection
        | Expression::VectorPlus(_, _) => true,
        Expression::Variable(index) | Expression::SelfFieldRead(index) => matches!(
            model
                .variables
                .get(usize::from(*index))
                .map(|variable| &variable.default),
            Some(DesignerValue::Vector(_))
        ),
        Expression::ArrayElement { array, .. } => match array.as_ref() {
            Expression::Variable(index) | Expression::SelfFieldRead(index) => matches!(
                model
                    .variables
                    .get(usize::from(*index))
                    .map(|variable| &variable.default),
                Some(DesignerValue::VectorArray(_))
            ),
            _ => false,
        },
        _ => false,
    }
}
fn comparison_code(
    operation: Comparison,
    left: &Expression,
    right: &Expression,
    left_code: String,
    right_code: String,
    model: &AiModel,
) -> String {
    let op = match operation {
        Comparison::Equal => "==",
        Comparison::Different => "!=",
        Comparison::Less => "<",
        Comparison::Greater => ">",
        Comparison::LessOrEqual => "<=",
        Comparison::GreaterOrEqual => ">=",
    };
    if matches!(operation, Comparison::Equal | Comparison::Different)
        && (actor_expression(left, model)
            || actor_expression(right, model)
            || reference_expression(left, model)
            || reference_expression(right, model)
            || matches!(left, Expression::Nobody | Expression::Nowhere)
            || matches!(right, Expression::Nobody | Expression::Nowhere))
    {
        let left_code = if matches!(left, Expression::Integer(0)) {
            "null".into()
        } else {
            left_code
        };
        let right_code = if matches!(right, Expression::Integer(0)) {
            "null".into()
        } else {
            right_code
        };
        format!("({left_code} {op} {right_code})")
    } else if vector_expression(left, model) || vector_expression(right, model) {
        // GDScript's float() accepts scalar numeric values; applying it to a
        // Vector3 is a parse error. Vector equality is native Godot behavior.
        format!("({left_code} {op} {right_code})")
    } else {
        format!("(float({left_code}) {op} float({right_code}))")
    }
}

fn reference_expression(expression: &Expression, model: &AiModel) -> bool {
    match expression {
        Expression::WaypointReference(_) | Expression::Actor(_) => true,
        Expression::Variable(index) | Expression::SelfFieldRead(index) => matches!(
            model
                .variables
                .get(usize::from(*index))
                .map(|variable| &variable.default),
            Some(
                DesignerValue::Actor(_)
                    | DesignerValue::SourceActor(_)
                    | DesignerValue::Waypoint(_)
                    | DesignerValue::SourceWaypoint(_)
                    | DesignerValue::ActorList(_)
                    | DesignerValue::WaypointArray(_)
                    | DesignerValue::SourceWaypointArray(_)
            )
        ),
        Expression::ArrayElement { array, .. } => match array.as_ref() {
            Expression::Variable(index) | Expression::SelfFieldRead(index) => matches!(
                model
                    .variables
                    .get(usize::from(*index))
                    .map(|variable| &variable.default),
                Some(
                    DesignerValue::ActorList(_)
                        | DesignerValue::WaypointArray(_)
                        | DesignerValue::SourceWaypointArray(_)
                )
            ),
            _ => false,
        },
        _ => false,
    }
}
fn needs_guard(e: &Expression, model: &AiModel) -> bool {
    match e {
        Expression::Unsupported(_)
        | Expression::InventoryAddObject(_)
        | Expression::MagicAddPoints(_)
        | Expression::MagicPointsMaximum
        | Expression::WaypointPosition(_)
        | Expression::DistanceToWaypoint { .. }
        | Expression::WaypointSphereContains { .. }
        | Expression::CollideWithNothing
        | Expression::LaserDirection
        | Expression::MagicCurrentSpell
        | Expression::DialogIsOver
        | Expression::DialogIsOverAndChangeBehavior
        | Expression::MagicPoints
        | Expression::SelfHitPoints
        | Expression::ActorHitPointsMaximum(_)
        | Expression::ActorPosition(_)
        | Expression::CurrentAnimationFrame
        | Expression::MagicRemovePoints(_)
        | Expression::Square(_)
        | Expression::AbsoluteValue(_)
        | Expression::ActorDistance(_)
        | Expression::ActorDistanceFrom { .. }
        | Expression::ActorFacingAngle { .. }
        | Expression::ActorLocalSourceXYDistance(_)
        | Expression::ActorLocalDistance { .. }
        | Expression::IsAbovePerso { .. }
        | Expression::ActorPositionField(_)
        | Expression::ActorHitPoints(_)
        | Expression::GameState
        | Expression::InventoryGold
        | Expression::InventoryMaximum(_)
        | Expression::Sine(_)
        | Expression::Cosine(_)
        | Expression::IsValidActor(_)
        | Expression::ApplyArmoredDamage(_)
        | Expression::ArmorResistance
        | Expression::ArmorCount
        | Expression::CameraShiftAlpha { .. }
        | Expression::CameraShiftTeta { .. } => true,
        Expression::IsInBehavior { .. } => true,
        Expression::Variable(index) => unresolved_variable(model, *index),
        Expression::ActorListContains { actor, .. } => needs_guard(actor, model),
        Expression::ActorFieldRead { .. } | Expression::MainActorFieldRead(_) => true,
        Expression::Vector(values) => values.iter().any(|value| needs_guard(value, model)),
        Expression::Multiply(a, b)
        | Expression::Plus(a, b)
        | Expression::VectorPlus(a, b)
        | Expression::Minus(a, b)
        | Expression::Divide(a, b)
        | Expression::TimeElapsed {
            start: a,
            duration: b,
        }
        | Expression::RandomInt { lower: a, upper: b }
        | Expression::InventoryRemove { item: a, count: b }
        | Expression::VectorCos(a, b)
        | Expression::ArrayElement { array: a, index: b }
        | Expression::And(a, b)
        | Expression::Or(a, b) => needs_guard(a, model) || needs_guard(b, model),
        Expression::Compare { left, right, .. } => {
            needs_guard(left, model) || needs_guard(right, model)
        }
        Expression::Negate(value)
        | Expression::IsInAction(value)
        | Expression::Not(value)
        | Expression::InventoryQuantity(value)
        | Expression::IntegerCast(value) => needs_guard(value, model),
        Expression::VectorComponent { vector, .. } => needs_guard(vector, model),
        _ => false,
    }
}
/// A fragment emits a failure and exits the enclosing coroutine before a
/// branch, assignment, or action request can use a fabricated fallback value.
fn guarded_expression(
    out: &mut String,
    e: &Expression,
    model: &AiModel,
    depth: usize,
    flow: &mut Flow,
) -> Result<String> {
    if !needs_guard(e, model) {
        return expression(e, model);
    }
    let pad = "    ".repeat(depth);
    let exit = flow.exit();
    Ok(match e {
        Expression::IsValidActor(actor) => {
            let value = guarded_expression(out, actor, model, depth, flow)?;
            let local = format!("actor_value_{}", flow.local);
            flow.local += 1;
            writeln!(out, "{pad}var {local} = {value}")?;
            format!("(typeof({local}) == TYPE_OBJECT and is_instance_valid({local}))")
        }
        Expression::ActorDistance(actor) => {
            let value = guarded_expression(out, actor, model, depth, flow)?;
            let id = flow.local;
            flow.local += 1;
            writeln!(
                out,
                "{pad}var distance_actor_{id} = {value}\n{pad}if typeof(distance_actor_{id}) != TYPE_OBJECT or not is_instance_valid(distance_actor_{id}) or not (distance_actor_{id} is Node3D) or not distance_actor_{id}.is_inside_tree() or not character().is_inside_tree():\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                quoted(&format!(
                    "{} distance requires positioned actors",
                    model.path
                ))
            )?;
            format!(
                "character().global_position.distance_to((distance_actor_{id} as Node3D).global_position)"
            )
        }
        Expression::ActorPosition(actor) | Expression::ActorPositionField(actor) => {
            let local = format!("position_actor_{}", flow.local);
            flow.local += 1;
            writeln!(
                out,
                "{pad}var {local}: Node3D = {}\n{pad}if not is_instance_valid({local}) or not {local}.is_inside_tree() or not {local}.has_method(\"source_position\"):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                actor_target_code(actor),
                quoted(&format!(
                    "{} requires positioned actor {}",
                    model.path,
                    actor_target_label(actor)
                ))
            )?;
            format!("{local}.source_position()")
        }
        Expression::ActorFacingAngle { receiver, target } => {
            let id = flow.local;
            flow.local += 1;
            writeln!(
                out,
                "{pad}var facing_receiver_{id}: Node3D = {}\n{pad}if not is_instance_valid(facing_receiver_{id}) or not facing_receiver_{id}.has_method(\"source_turn_angles\"):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                actor_target_code(receiver),
                quoted(&format!("{} requires a native facing receiver", model.path))
            )?;
            let target = guarded_expression(out, target, model, depth, flow)?;
            writeln!(
                out,
                "{pad}var facing_target_{id} = {target}\n{pad}if typeof(facing_target_{id}) != TYPE_OBJECT or not is_instance_valid(facing_target_{id}) or not (facing_target_{id} is Node3D):\n{pad}    fail_unsupported({})\n{pad}    {exit}\n{pad}var facing_angles_{id}: Vector2 = facing_receiver_{id}.source_turn_angles(facing_target_{id})",
                quoted(&format!(
                    "{} requires a positioned facing target",
                    model.path
                ))
            )?;
            format!("(facing_angles_{id}.y * 57.295780181884766)")
        }
        Expression::ActorHitPointsMaximum(actor) | Expression::ActorHitPoints(actor) => {
            let value = guarded_expression(out, actor, model, depth, flow)?;
            let local = format!("maximum_life_actor_{}", flow.local);
            flow.local += 1;
            writeln!(
                out,
                "{pad}var {local} = {value}\n{pad}if typeof({local}) != TYPE_OBJECT or not is_instance_valid({local}) or not {local}.has_method(\"source_hit_points_ready\") or not {local}.source_hit_points_ready():\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                quoted(&format!(
                    "{} requires converted actor maximum hit points",
                    model.path
                ))
            )?;
            format!(
                "int({local}.source_hit_points.{})",
                if matches!(e, Expression::ActorHitPoints(_)) {
                    "current"
                } else {
                    "maximum"
                }
            )
        }
        Expression::ActorDistanceFrom { receiver, target } => {
            let id = flow.local;
            flow.local += 1;
            let receiver_local = format!("distance_receiver_{id}");
            let target_local = format!("distance_target_{id}");
            let receiver_expr = actor_target_code(receiver);
            writeln!(
                out,
                "{pad}var {receiver_local}: Node3D = {receiver_expr}\n{pad}if not is_instance_valid({receiver_local}) or not {receiver_local}.is_inside_tree():\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                quoted(&format!(
                    "{} distance requires receiver {}",
                    model.path,
                    actor_target_label(receiver)
                ))
            )?;
            let target_expr = guarded_expression(out, target, model, depth, flow)?;
            writeln!(
                out,
                "{pad}var {target_local} = {target_expr}\n{pad}if typeof({target_local}) != TYPE_OBJECT or not is_instance_valid({target_local}) or not ({target_local} is Node3D) or not {target_local}.is_inside_tree():\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                quoted(&format!(
                    "{} distance requires a positioned target actor",
                    model.path
                ))
            )?;
            format!(
                "{receiver_local}.global_position.distance_to(({target_local} as Node3D).global_position)"
            )
        }
        Expression::ActorLocalSourceXYDistance(target)
        | Expression::ActorLocalDistance { target, .. } => {
            let receiver = if let Expression::ActorLocalDistance { receiver, .. } = e {
                actor_target_code(receiver)
            } else {
                "character()".into()
            };
            let id = flow.local;
            flow.local += 1;
            writeln!(out, "{pad}var spatial_receiver_{id}: Node3D = {receiver}")?;
            let target = guarded_expression(out, target, model, depth, flow)?;
            writeln!(
                out,
                "{pad}var spatial_target_{id}: Variant = {target}\n{pad}if not is_instance_valid(spatial_receiver_{id}) or not spatial_receiver_{id}.is_inside_tree() or not (spatial_target_{id} is Node3D) or not is_instance_valid(spatial_target_{id}) or not spatial_target_{id}.is_inside_tree():\n{pad}    fail_unsupported(\"Local distance needs positioned actors\")\n{pad}    {exit}"
            )?;
            let method = if matches!(e, Expression::ActorLocalDistance { vertical: true, .. }) {
                "distance_z"
            } else {
                "distance_xy"
            };
            format!("NativeContactPickup.{method}(spatial_receiver_{id}, spatial_target_{id})")
        }
        Expression::IsAbovePerso {
            receiver,
            point,
            threshold,
        } => {
            let id = flow.local;
            flow.local += 1;
            writeln!(
                out,
                "{pad}var above_receiver_{id}: Node3D = {}\n{pad}if not is_instance_valid(above_receiver_{id}) or not above_receiver_{id}.has_method(\"source_picking_center\") or not above_receiver_{id}.is_inside_tree():\n{pad}    fail_unsupported(\"IsAbove needs a bound picking center\")\n{pad}    {exit}",
                actor_target_code(receiver)
            )?;
            let point = guarded_expression(out, point, model, depth, flow)?;
            writeln!(out, "{pad}var above_point_{id}: Vector3 = {point}")?;
            let threshold = guarded_expression(out, threshold, model, depth, flow)?;
            format!(
                "NativeContactPickup.is_above(above_receiver_{id}, Vector3(above_point_{id}.x, above_point_{id}.z, -above_point_{id}.y), float({threshold}), above_receiver_{id}.source_picking_center())"
            )
        }
        Expression::Sine(value) | Expression::Cosine(value) => {
            let value = guarded_expression(out, value, model, depth, flow)?;
            let function = if matches!(e, Expression::Sine(_)) {
                "sin"
            } else {
                "cos"
            };
            format!("PackedFloat32Array([{function}(float({value}) * 0.01745329238474369)])[0]")
        }
        Expression::GameState => {
            let id = flow.local;
            flow.local += 1;
            writeln!(
                out,
                "{pad}var state_session_{id} := source_game_session()\n{pad}if not is_instance_valid(state_session_{id}) or not state_session_{id}.has_method(\"source_get_game_state\"):\n{pad}    fail_unsupported(\"GameState needs the game session\")\n{pad}    {exit}\n{pad}var game_state_{id}: Variant = state_session_{id}.source_get_game_state()\n{pad}if not game_state_{id} is int:\n{pad}    fail_unsupported(\"Unrecognized GameState mask\")\n{pad}    {exit}"
            )?;
            format!("game_state_{id}")
        }
        Expression::InventoryGold | Expression::InventoryMaximum(_) => {
            let id = flow.local;
            flow.local += 1;
            writeln!(
                out,
                "{pad}if character().source_inventory == null:\n{pad}    fail_unsupported(\"Inventory query needs shared inventory\")\n{pad}    {exit}"
            )?;
            let query = if let Expression::InventoryMaximum(item) = e {
                let item = guarded_expression(out, item, model, depth, flow)?;
                format!("maximum_capacity(int({item}))")
            } else {
                "gold_value()".into()
            };
            writeln!(
                out,
                "{pad}var inventory_query_{id}: int = character().source_inventory.{query}\n{pad}if not character().source_inventory.last_failure.is_empty():\n{pad}    fail_unsupported(character().source_inventory.last_failure)\n{pad}    {exit}"
            )?;
            format!("inventory_query_{id}")
        }
        Expression::Square(value) | Expression::AbsoluteValue(value) => {
            let value = guarded_expression(out, value, model, depth, flow)?;
            let local = format!("real_operand_{}", flow.local);
            flow.local += 1;
            writeln!(out, "{pad}var {local} := float({value})")?;
            if matches!(e, Expression::Square(_)) {
                format!("PackedFloat32Array([{local} * {local}])[0]")
            } else {
                format!("PackedFloat32Array([-{local} if {local} <= 0.0 else {local}])[0]")
            }
        }
        Expression::ApplyArmoredDamage(value) => {
            let value = guarded_expression(out, value, model, depth, flow)?;
            let id = flow.local;
            flow.local += 1;
            let armor = armor_receiver(out, model, depth, flow, false)?;
            let failure = quoted(&format!(
                "{} requires native shared armor and main actor hit points for damage",
                model.path
            ));
            writeln!(
                out,
                "{pad}var armor_damage_input_{id}: int = int({value})\n{pad}if not {armor}.has_method(\"damage\") or not is_instance_valid(main_actor):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}var armor_hit_points_{id}: Resource = main_actor.get(\"source_hit_points\") as Resource\n{pad}if armor_hit_points_{id} == null or not armor_hit_points_{id}.has_method(\"subtract_damage\") or not armor_hit_points_{id}.has_method(\"can_receive_damage\") or not bool(armor_hit_points_{id}.call(\"can_receive_damage\")):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}var effective_damage_{id}: int = int({armor}.call(\"damage\", armor_damage_input_{id}))\n{pad}if effective_damage_{id} < 0:\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}var remaining_hit_points_{id}: int = int(armor_hit_points_{id}.call(\"subtract_damage\", effective_damage_{id}))"
            )?;
            format!("remaining_hit_points_{id}")
        }
        Expression::ArmorResistance | Expression::ArmorCount => {
            let armor = armor_receiver(
                out,
                model,
                depth,
                flow,
                matches!(e, Expression::ArmorResistance),
            )?;
            if matches!(e, Expression::ArmorResistance) {
                format!("int({armor}.call(\"resistance\"))")
            } else {
                format!("{armor}.call(\"definition_count\")")
            }
        }
        Expression::WaypointPosition(reference) => {
            let value = waypoint_constant(reference);
            let local = format!("waypoint_position_{}", flow.local);
            flow.local += 1;
            writeln!(
                out,
                "{pad}var {local} = {value}\n{pad}if typeof({local}) != TYPE_VECTOR3:\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                quoted(&format!(
                    "{} requires resolved absolute waypoint {}",
                    model.path, reference.path
                ))
            )?;
            local
        }
        Expression::DistanceToWaypoint { actor, waypoint } => {
            let receiver = match actor {
                ActorTarget::Keyword(ActorKeyword::SelfActor) => "character()".to_owned(),
                ActorTarget::Keyword(ActorKeyword::MainActor) => "main_actor".to_owned(),
                ActorTarget::Variable(index) => format!("dsg_{index}"),
                ActorTarget::Reference(actor) => format!(
                    "character().source_actor_node({})",
                    quoted(actor.path.as_str())
                ),
            };
            let id = flow.local;
            flow.local += 1;
            let receiver_local = format!("distance_receiver_{id}");
            let waypoint_local = format!("distance_waypoint_{id}");
            let value_local = format!("distance_value_{id}");
            writeln!(
                out,
                "{pad}var {receiver_local}: Node3D = {receiver}\n{pad}if not is_instance_valid({receiver_local}):\n{pad}    fail_unsupported({})\n{pad}    {exit}\n{pad}var {waypoint_local} = {}\n{pad}if typeof({waypoint_local}) != TYPE_VECTOR3:\n{pad}    fail_unsupported({})\n{pad}    {exit}\n{pad}var {value_local} = character().source_distance_to_waypoint({receiver_local}, {waypoint_local})",
                quoted(&format!(
                    "{} requires distance receiver {}",
                    model.path,
                    match actor {
                        ActorTarget::Keyword(ActorKeyword::SelfActor) => "self",
                        ActorTarget::Keyword(ActorKeyword::MainActor) => "main actor",
                        ActorTarget::Reference(actor) => actor.path.as_str(),
                        ActorTarget::Variable(_) => "actor designer field",
                    }
                )),
                waypoint_constant(waypoint),
                quoted(&format!(
                    "{} requires resolved absolute waypoint {}",
                    model.path, waypoint.path
                ))
            )?;
            value_local
        }
        Expression::CameraShiftAlpha { selector, receiver } => {
            let selector = guarded_expression(out, selector, model, depth, flow)?;
            let receiver = match receiver {
                ActorTarget::Keyword(ActorKeyword::SelfActor) => "character()".to_owned(),
                ActorTarget::Keyword(ActorKeyword::MainActor) => "main_actor".to_owned(),
                ActorTarget::Variable(index) => format!("dsg_{index}"),
                ActorTarget::Reference(actor) => format!(
                    "character().source_actor_node({})",
                    quoted(actor.path.as_str())
                ),
            };
            let id = flow.local;
            flow.local += 1;
            let actor_local = format!("camera_shift_actor_{id}");
            let value_local = format!("camera_shift_alpha_{id}");
            let failure = quoted(&format!(
                "{} Function_Camera_GetShiftAlpha requires a mapped selector on a bound Perso with SourceCamera",
                model.path
            ));
            writeln!(
                out,
                "{pad}var {actor_local} = {receiver}\n{pad}if typeof({actor_local}) != TYPE_OBJECT or not is_instance_valid({actor_local}) or not ({actor_local} is Node3D) or not {actor_local}.has_method(\"source_camera_get_shift_alpha\"):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}var {value_local} = {actor_local}.call(\"source_camera_get_shift_alpha\", int({selector}))\n{pad}if typeof({value_local}) != TYPE_FLOAT:\n{pad}    fail_unsupported({failure})\n{pad}    {exit}"
            )?;
            value_local
        }
        Expression::CameraShiftTeta { selector, receiver } => {
            let selector = guarded_expression(out, selector, model, depth, flow)?;
            let receiver = match receiver {
                ActorTarget::Keyword(ActorKeyword::SelfActor) => "character()".to_owned(),
                ActorTarget::Keyword(ActorKeyword::MainActor) => "main_actor".to_owned(),
                ActorTarget::Variable(index) => format!("dsg_{index}"),
                ActorTarget::Reference(actor) => format!(
                    "character().source_actor_node({})",
                    quoted(actor.path.as_str())
                ),
            };
            let id = flow.local;
            flow.local += 1;
            let actor_local = format!("camera_shift_teta_actor_{id}");
            let value_local = format!("camera_shift_teta_{id}");
            let failure = quoted(&format!(
                "{} Function_Camera_GetShiftTeta requires a mapped selector on a bound Perso with SourceCamera",
                model.path
            ));
            writeln!(
                out,
                "{pad}var {actor_local} = {receiver}\n{pad}if typeof({actor_local}) != TYPE_OBJECT or not is_instance_valid({actor_local}) or not ({actor_local} is Node3D) or not {actor_local}.has_method(\"source_camera_get_shift_teta\"):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}var {value_local} = {actor_local}.call(\"source_camera_get_shift_teta\", int({selector}))\n{pad}if typeof({value_local}) != TYPE_FLOAT:\n{pad}    fail_unsupported({failure})\n{pad}    {exit}"
            )?;
            value_local
        }
        Expression::WaypointSphereContains { actor, waypoint } => {
            let actor_value = guarded_expression(out, actor, model, depth, flow)?;
            let actor_id = flow.local;
            flow.local += 1;
            let actor_local = format!("waypoint_actor_{actor_id}");
            let failure = quoted(&format!(
                "{} Cond_CollisionWP requires a live actor and resolved waypoint",
                model.path
            ));
            writeln!(
                out,
                "{pad}var {actor_local}: Variant = {actor_value}\n{pad}if typeof({actor_local}) != TYPE_OBJECT or not is_instance_valid({actor_local}) or not ({actor_local} is Node3D) or not {actor_local}.is_inside_tree():\n{pad}    fail_unsupported({failure})\n{pad}    {exit}"
            )?;
            let waypoint_local = format!("waypoint_operand_{actor_id}");
            if let Expression::ArrayElement { array, index } = waypoint.as_ref() {
                let array_value = guarded_expression(out, array, model, depth, flow)?;
                let index_value = guarded_expression(out, index, model, depth, flow)?;
                let array_local = format!("waypoint_array_{actor_id}");
                let index_local = format!("waypoint_index_{actor_id}");
                writeln!(
                    out,
                    "{pad}var {array_local}: Variant = {array_value}\n{pad}var {index_local}: int = int({index_value})\n{pad}if typeof({array_local}) != TYPE_ARRAY or {index_local} < 0 or {index_local} >= {array_local}.size():\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}var {waypoint_local}: Variant = {array_local}[{index_local}]"
                )?;
            } else {
                let waypoint_value = guarded_expression(out, waypoint, model, depth, flow)?;
                writeln!(out, "{pad}var {waypoint_local}: Variant = {waypoint_value}")?;
            }
            writeln!(
                out,
                "{pad}if typeof({waypoint_local}) != TYPE_OBJECT or not is_instance_valid({waypoint_local}) or not {waypoint_local}.has_method(\"source_world_position\"):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}"
            )?;
            let result_local = format!("waypoint_contains_{actor_id}");
            writeln!(
                out,
                "{pad}var {result_local}: Variant = character().source_actor_in_waypoint_sphere({actor_local} as Node3D, {waypoint_local} as Resource)\n{pad}if typeof({result_local}) != TYPE_BOOL:\n{pad}    fail_unsupported({failure})\n{pad}    {exit}"
            )?;
            result_local
        }
        Expression::DialogIsOver | Expression::DialogIsOverAndChangeBehavior => {
            writeln!(
                out,
                "{pad}if source_ui() == null:\n{pad}    fail_unsupported(\"Dialogue query requires scene UI\")\n{pad}    {exit}"
            )?;
            expression(e, model)?
        }
        Expression::CurrentAnimationFrame => {
            let id = flow.local;
            flow.local += 1;
            let local = format!("current_animation_frame_{id}");
            writeln!(
                out,
                "{pad}if character().current_clip == null or character().sample_rate <= 0.0:\n{pad}    fail_unsupported({})\n{pad}    {exit}\n{pad}var {local}: int = character().frame_index",
                quoted(&format!(
                    "{} Func_GetCurrectAnimFrame requires an active native animation clip with a positive source frame rate",
                    model.path
                ))
            )?;
            local
        }
        Expression::Unsupported(fragment) => {
            let id = flow.local;
            flow.local += 1;
            let local = format!("partial_expr_{id}");
            let receiver = fragment
                .receiver
                .as_ref()
                .map(|target| format!(" (receiver {})", actor_target_label(target)))
                .unwrap_or_default();
            writeln!(
                out,
                "{pad}var {local}\n{pad}fail_unsupported({})\n{pad}{exit}",
                quoted(&format!(
                    "{} source AI node {}{}: {}",
                    model.path, fragment.start, receiver, fragment.reason
                ))
            )?;
            local
        }
        Expression::Variable(index) if unresolved_variable(model, *index) => {
            let id = flow.local;
            flow.local += 1;
            let local = format!("partial_expr_{id}");
            writeln!(
                out,
                "{pad}var {local}\n{pad}fail_unsupported({})\n{pad}{exit}",
                quoted(&format!(
                    "{} unresolved source designer field {index}",
                    model.path
                ))
            )?;
            local
        }
        Expression::ActorFieldRead { actor, variable } => {
            let index = model
                .actor_field_targets()
                .iter()
                .position(|target| target == actor)
                .ok_or_else(|| anyhow::anyhow!("actor field read target was not declared"))?;
            let id = flow.local;
            flow.local += 1;
            let node = format!("actor_brain_{index}_read_{id}");
            let local = format!("partial_expr_{id}");
            let failure = quoted(&format!(
                "{} requires actor {} Brain field {variable}",
                model.path, actor.path
            ));
            writeln!(
                out,
                "{pad}var {node} := actor_brain_{index}\n{pad}if not is_instance_valid({node}) and not actor_brain_{index}_path.is_empty():\n{pad}    {node} = get_node_or_null(actor_brain_{index}_path)\n{pad}if not is_instance_valid({node}):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}var {local} = {node}.get(\"dsg_{variable}\")"
            )?;
            local
        }
        Expression::SeePerso { receiver, target }
            if *receiver == ActorKeyword::MainActor || *target == ActorKeyword::MainActor =>
        {
            writeln!(
                out,
                "{pad}if not is_instance_valid(main_actor):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                quoted(&format!("{} SeePerso requires the main actor", model.path))
            )?;
            expression(e, model)?
        }
        Expression::MainActorFieldRead(variable) => {
            let id = flow.local;
            flow.local += 1;
            let node = format!("main_actor_brain_{id}");
            let local = format!("partial_expr_{id}");
            let failure = quoted(&format!(
                "{} requires main actor Brain field {variable}",
                model.path
            ));
            writeln!(
                out,
                "{pad}if not is_instance_valid(main_actor):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}var {node} := main_actor.get_node_or_null(\"Brain\")\n{pad}if not is_instance_valid({node}):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}var {local} = {node}.get(\"dsg_{variable}\")"
            )?;
            local
        }
        Expression::And(left, right) | Expression::Or(left, right) => {
            let left = guarded_expression(out, left, model, depth, flow)?;
            let id = flow.local;
            flow.local += 1;
            let local = format!("partial_expr_{id}");
            writeln!(out, "{pad}var {local} := bool({left})")?;
            writeln!(
                out,
                "{pad}if {}{local}:",
                if matches!(e, Expression::Or(..)) {
                    "not "
                } else {
                    ""
                }
            )?;
            let right = guarded_expression(out, right, model, depth + 1, flow)?;
            writeln!(out, "{pad}    {local} = bool({right})")?;
            local
        }
        Expression::Multiply(left, right) => format!(
            "(float({}) * float({}))",
            guarded_expression(out, left, model, depth, flow)?,
            guarded_expression(out, right, model, depth, flow)?
        ),
        Expression::TimeElapsed { start, duration } => format!(
            "source_time_elapsed(int({}), int({}))",
            guarded_expression(out, start, model, depth, flow)?,
            guarded_expression(out, duration, model, depth, flow)?
        ),
        Expression::Plus(left, right)
        | Expression::Minus(left, right)
        | Expression::Divide(left, right) => {
            let operator = match e {
                Expression::Plus(..) => "+",
                Expression::Minus(..) => "-",
                _ => "/",
            };
            format!(
                "(float({}) {operator} float({}))",
                guarded_expression(out, left, model, depth, flow)?,
                guarded_expression(out, right, model, depth, flow)?
            )
        }
        Expression::VectorPlus(left, right) => format!(
            "({} + {})",
            guarded_expression(out, left, model, depth, flow)?,
            guarded_expression(out, right, model, depth, flow)?
        ),
        Expression::Vector(values) => format!(
            "Vector3(float({}), float({}), float({}))",
            guarded_expression(out, &values[0], model, depth, flow)?,
            guarded_expression(out, &values[1], model, depth, flow)?,
            guarded_expression(out, &values[2], model, depth, flow)?
        ),
        Expression::IntegerCast(value) => {
            format!(
                "int({})",
                guarded_expression(out, value, model, depth, flow)?
            )
        }
        Expression::RandomInt { lower, upper } => format!(
            "random_int(int({}), int({}))",
            guarded_expression(out, lower, model, depth, flow)?,
            guarded_expression(out, upper, model, depth, flow)?
        ),
        Expression::InventoryQuantity(item) => format!(
            "character().source_inventory_quantity(int({}))",
            guarded_expression(out, item, model, depth, flow)?
        ),
        Expression::InventoryRemove { item, count } => format!(
            "character().source_inventory_remove(int({}), int({}))",
            guarded_expression(out, item, model, depth, flow)?,
            guarded_expression(out, count, model, depth, flow)?
        ),
        Expression::SelfHitPoints => {
            writeln!(
                out,
                "{pad}if not character().source_hit_points_ready():\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                quoted(&format!(
                    "{} requires converted actor hit points",
                    model.path
                ))
            )?;
            expression(e, model)?
        }
        Expression::InventoryAddObject(actor) => {
            let actor = guarded_expression(out, actor, model, depth, flow)?;
            let local = format!("pickup_result_{}", flow.local);
            flow.local += 1;
            writeln!(
                out,
                "{pad}var {local}: int = character().source_inventory_add_object({actor})\n{pad}if not character().source_inventory.last_failure.is_empty():\n{pad}    fail_unsupported(character().source_inventory.last_failure)\n{pad}    {exit}"
            )?;
            local
        }
        Expression::MagicCurrentSpell
        | Expression::MagicPoints
        | Expression::MagicPointsMaximum => {
            let ready = if matches!(e, Expression::MagicCurrentSpell) {
                "source_magic_ready"
            } else {
                "source_magic_points_ready"
            };
            writeln!(
                out,
                "{pad}if not character().{ready}():\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                quoted(&format!(
                    "{} requires restored source magick state",
                    model.path
                ))
            )?;
            expression(e, model)?
        }
        Expression::MagicRemovePoints(amount) => {
            let amount = guarded_expression(out, amount, model, depth, flow)?;
            writeln!(
                out,
                "{pad}if not character().source_magic_ready():\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                quoted(&format!(
                    "{} requires restored source magick state",
                    model.path
                )),
            )?;
            format!("character().source_magic_remove_points(int({amount}))")
        }
        Expression::MagicAddPoints(amount) => {
            let amount = guarded_expression(out, amount, model, depth, flow)?;
            let local = flow.local;
            flow.local += 1;
            writeln!(
                out,
                "{pad}if not character().source_magic_points_ready():\n{pad}    fail_unsupported(\"Magick point state is unavailable\")\n{pad}    {exit}\n{pad}var magic_sum_{local}: int = character().source_magick.add_points(int({amount}))"
            )?;
            format!("magic_sum_{local}")
        }
        Expression::IsInBehavior { actor, target } => {
            let id = flow.local;
            flow.local += 1;
            let actor_node = match actor {
                ActorTarget::Keyword(ActorKeyword::SelfActor) => "character()".to_owned(),
                ActorTarget::Keyword(ActorKeyword::MainActor) => "main_actor".to_owned(),
                ActorTarget::Variable(index) => format!("dsg_{index}"),
                ActorTarget::Reference(reference) => format!(
                    "character().source_actor_node({})",
                    quoted(reference.path.as_str())
                ),
            };
            let query_actor = if matches!(actor, ActorTarget::Keyword(ActorKeyword::SelfActor)) {
                actor_node
            } else {
                let local = format!("behavior_actor_{id}");
                let description = match actor {
                    ActorTarget::Keyword(ActorKeyword::MainActor) => "MainActor".to_owned(),
                    ActorTarget::Reference(reference) => reference.path.to_string(),
                    ActorTarget::Variable(index) => format!("dsg_{index}"),
                    ActorTarget::Keyword(ActorKeyword::SelfActor) => unreachable!(),
                };
                let failure = quoted(&format!(
                    "{} requires resolved actor {} for behavior query",
                    model.path, description
                ));
                writeln!(
                    out,
                    "{pad}var {local} = {actor_node}\n{pad}if not is_instance_valid({local}):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}"
                )?;
                local
            };
            format!(
                "character().source_actor_is_in_behavior({query_actor}, &\"{}\", {})",
                group_name(target.group),
                target.behavior
            )
        }
        Expression::CollideWithNothing => {
            let id = flow.local;
            flow.local += 1;
            let local = format!("contact_result_{id}");
            writeln!(
                out,
                "{pad}var {local} = character().source_collide_with_nothing()\n{pad}if {local} == null:\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                quoted(&format!(
                    "{} requires a verified source contact result",
                    model.path
                ))
            )?;
            local
        }
        Expression::LaserDirection => {
            writeln!(
                out,
                "{pad}if not character().source_laser_pointer_ready():\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                quoted(&format!("{} requires a native laser target", model.path))
            )?;
            "character().source_laser_pointer_direction()".into()
        }
        Expression::VectorCos(a, b) => format!(
            "character().source_vector_cos({}, {})",
            guarded_expression(out, a, model, depth, flow)?,
            guarded_expression(out, b, model, depth, flow)?
        ),
        Expression::Negate(value) => format!(
            "(-float({}))",
            guarded_expression(out, value, model, depth, flow)?
        ),
        Expression::IsInAction(action) => format!(
            "character().is_in_action(int({}))",
            guarded_expression(out, action, model, depth, flow)?
        ),
        Expression::ArrayElement { array, index } => format!(
            "{}[int({})]",
            guarded_expression(out, array, model, depth, flow)?,
            guarded_expression(out, index, model, depth, flow)?
        ),
        Expression::ActorListContains { list, actor } => {
            actor_list_capacity(model, *list)?;
            format!(
                "dsg_{list}.has({})",
                guarded_expression(out, actor, model, depth, flow)?
            )
        }
        Expression::Compare {
            operation,
            left,
            right,
        } => comparison_code(
            *operation,
            left,
            right,
            guarded_expression(out, left, model, depth, flow)?,
            guarded_expression(out, right, model, depth, flow)?,
            model,
        ),
        Expression::Not(value) => format!(
            "(not bool({}))",
            guarded_expression(out, value, model, depth, flow)?
        ),
        Expression::VectorComponent { vector, axis } => format!(
            "({}).{}",
            guarded_expression(out, vector, model, depth, flow)?,
            match axis {
                VectorAxis::X => "x",
                VectorAxis::Y => "y",
                VectorAxis::Z => "z",
            }
        ),
        _ => expression(e, model)?,
    })
}
fn expression(e: &Expression, model: &AiModel) -> Result<String> {
    Ok(match e {
        Expression::Nobody | Expression::Nowhere => "null".into(),
        Expression::Actor(actor) => actor_target_code(actor),
        Expression::Configuration(None) => "null".into(),
        Expression::Configuration(Some(reference)) => configuration_constant(reference),
        Expression::Integer(v) => v.to_string(),
        Expression::Real(v) => format!("{v:?}"),
        Expression::IntegerCast(value) => format!("int({})", expression(value, model)?),
        Expression::SelfPosition => "character().source_position()".into(),
        Expression::SelfPositionField => "character().source_position()".into(),
        Expression::WaypointReference(reference) => {
            format!("{}()", waypoint_resource_constant(reference))
        }
        Expression::WaypointPosition(reference) => waypoint_constant(reference),
        Expression::GameTime => "character().source_game_time()".into(),
        Expression::TimeElapsed { start, duration } => format!(
            "source_time_elapsed(int({}), int({}))",
            expression(start, model)?,
            expression(duration, model)?
        ),
        Expression::Vector(values) => format!(
            "Vector3(float({}), float({}), float({}))",
            expression(&values[0], model)?,
            expression(&values[1], model)?,
            expression(&values[2], model)?
        ),
        Expression::Variable(v) => {
            ensure!(
                usize::from(*v) < model.variables.len(),
                "AI variable missing"
            );
            ensure!(
                !unresolved_variable(model, *v),
                "compiled AI reads unresolved source waypoint designer field {v}"
            );
            format!("dsg_{v}")
        }
        Expression::ActorListContains { list, actor } => {
            actor_list_capacity(model, *list)?;
            format!("dsg_{list}.has({})", expression(actor, model)?)
        }
        Expression::SelfFieldRead(v) => {
            ensure!(
                usize::from(*v) < model.variables.len(),
                "AI own field missing"
            );
            format!("dsg_{v}")
        }
        Expression::Action(v) => v.as_ref().map_or(-1, |r| i64::from(r.state)).to_string(),
        Expression::IsInAction(action) => format!(
            "character().is_in_action(int({}))",
            expression(action, model)?
        ),
        Expression::DialogIsOver => "source_ui().source_dialog_is_over(main_actor)".into(),
        Expression::DialogIsOverAndChangeBehavior => {
            "source_ui().source_dialog_is_over_and_change_behavior(main_actor)".into()
        }
        Expression::ActionFinished => "character().action_complete".into(),
        Expression::CurrentAnimationFrame => {
            anyhow::bail!("current animation frame requires guarded lowering")
        }
        Expression::CollideWithGround => "character().source_collide_with_ground()".into(),
        Expression::CollideWithNothing => "character().source_collide_with_nothing()".into(),
        Expression::CollidePersoZdeNoWithTypeZde {
            zone,
            material_mask,
        } => {
            format!(
                "character().source_collide_perso_zde_no_with_type_zde({zone}, {material_mask})"
            )
        }
        Expression::CollideWithSlope => "character().source_collide_with_slope()".into(),
        Expression::IsTypeOfGmtCollide(kind) => {
            format!("character().source_is_type_of_gmt_collide({kind})")
        }
        Expression::InTopOfJump => "character().source_in_top_of_jump()".into(),
        Expression::IsInBehavior { actor, target } => {
            format!(
                "character().source_actor_is_in_behavior({}, &\"{}\", {})",
                actor_target_code(actor),
                group_name(target.group),
                target.behavior
            )
        }
        Expression::ArrayElement { array, index } => format!(
            "{}[int({})]",
            expression(array, model)?,
            expression(index, model)?
        ),
        Expression::AbsentButton(query) => if *query == ButtonQuery::Released {
            "true"
        } else {
            "false"
        }
        .into(),
        Expression::Button { action, query } => {
            let name = quoted(action);
            match query {
                ButtonQuery::Pressed => format!("Input.is_action_pressed({name})"),
                ButtonQuery::Released => format!("not Input.is_action_pressed({name})"),
                ButtonQuery::JustPressed => format!("Input.is_action_just_pressed({name})"),
                ButtonQuery::JustReleased => format!("Input.is_action_just_released({name})"),
            }
        }
        Expression::MainActorDistance => "distance_to_main_actor()".into(),
        Expression::Square(_)
        | Expression::AbsoluteValue(_)
        | Expression::ActorDistance(_)
        | Expression::ActorHitPointsMaximum(_)
        | Expression::ActorPosition(_)
        | Expression::ActorDistanceFrom { .. }
        | Expression::ActorFacingAngle { .. }
        | Expression::ActorLocalSourceXYDistance(_)
        | Expression::ActorLocalDistance { .. }
        | Expression::IsAbovePerso { .. }
        | Expression::ActorPositionField(_)
        | Expression::ActorHitPoints(_)
        | Expression::GameState
        | Expression::InventoryGold
        | Expression::InventoryMaximum(_)
        | Expression::Sine(_)
        | Expression::Cosine(_)
        | Expression::IsValidActor(_)
        | Expression::ApplyArmoredDamage(_)
        | Expression::DistanceToWaypoint { .. }
        | Expression::CameraShiftAlpha { .. }
        | Expression::CameraShiftTeta { .. }
        | Expression::WaypointSphereContains { .. } => {
            anyhow::bail!("actor or numeric expression requires guarded compilation")
        }
        Expression::ArmorResistance | Expression::ArmorCount => {
            anyhow::bail!("armor query requires guarded compilation")
        }
        Expression::SelfHitPoints => "character().source_get_hit_points()".into(),
        Expression::InventoryWeapon => "character().source_inventory_weapon()".into(),
        Expression::InventoryAddObject(actor) => format!(
            "character().source_inventory_add_object({})",
            expression(actor, model)?
        ),
        Expression::InventoryQuantity(item) => format!(
            "character().source_inventory_quantity(int({}))",
            expression(item, model)?
        ),
        Expression::InventoryRemove { item, count } => format!(
            "character().source_inventory_remove(int({}), int({}))",
            expression(item, model)?,
            expression(count, model)?
        ),
        Expression::MagicCurrentSpell => "character().source_magic_current_spell()".into(),
        Expression::MagicPoints => "character().source_magic_points()".into(),
        Expression::MagicPointsMaximum => "int(character().source_magick.maximum_points)".into(),
        Expression::MagicAddPoints(amount) => format!(
            "character().source_magick.add_points(int({}))",
            expression(amount, model)?
        ),
        Expression::MagicRemovePoints(amount) => format!(
            "character().source_magic_remove_points(int({}))",
            expression(amount, model)?
        ),
        Expression::LaserDirection => {
            anyhow::bail!("laser direction requires guarded compilation")
        }
        Expression::VectorCos(a, b) => format!(
            "character().source_vector_cos({}, {})",
            expression(a, model)?,
            expression(b, model)?
        ),
        Expression::SeePerso { receiver, target } => {
            let actor = |keyword| match keyword {
                ActorKeyword::SelfActor => "character()",
                ActorKeyword::MainActor => "main_actor",
            };
            format!(
                "character().source_see_perso({}, {})",
                actor(*receiver),
                actor(*target)
            )
        }
        Expression::SelfSighting => "character().source_sighting()".into(),
        Expression::RandomInt { lower, upper } => format!(
            "random_int(int({}), int({}))",
            expression(lower, model)?,
            expression(upper, model)?
        ),
        Expression::Multiply(a, b) => format!(
            "(float({}) * float({}))",
            expression(a, model)?,
            expression(b, model)?
        ),
        Expression::ActorFieldRead { .. } | Expression::MainActorFieldRead(_) => {
            anyhow::bail!("actor field reads require guarded compilation")
        }
        Expression::Plus(a, b) | Expression::Minus(a, b) | Expression::Divide(a, b) => {
            let operator = match e {
                Expression::Plus(..) => "+",
                Expression::Minus(..) => "-",
                _ => "/",
            };
            format!(
                "(float({}) {operator} float({}))",
                expression(a, model)?,
                expression(b, model)?
            )
        }
        Expression::VectorPlus(a, b) => {
            format!("({} + {})", expression(a, model)?, expression(b, model)?)
        }
        Expression::Negate(v) => format!("(-float({}))", expression(v, model)?),
        Expression::Compare {
            operation,
            left,
            right,
        } => comparison_code(
            *operation,
            left,
            right,
            expression(left, model)?,
            expression(right, model)?,
            model,
        ),
        Expression::And(a, b) => format!(
            "(bool({}) and bool({}))",
            expression(a, model)?,
            expression(b, model)?
        ),
        Expression::Or(a, b) => format!(
            "(bool({}) or bool({}))",
            expression(a, model)?,
            expression(b, model)?
        ),
        Expression::Not(v) => format!("(not bool({}))", expression(v, model)?),
        Expression::VectorComponent { vector, axis } => format!(
            "({}).{}",
            expression(vector, model)?,
            match axis {
                VectorAxis::X => "x",
                VectorAxis::Y => "y",
                VectorAxis::Z => "z",
            }
        ),
        Expression::Unsupported(_) => {
            anyhow::bail!("unsupported AI expression requires guarded lowering")
        }
    })
}
struct Flow {
    group: &'static str,
    rule: bool,
    local: usize,
}
fn group_name(group: BehaviorGroup) -> &'static str {
    match group {
        BehaviorGroup::Normal => "normal",
        BehaviorGroup::Reflex => "reflex",
    }
}
/// A blocking request is an ordinary coroutine over captured native values.
/// Its enclosing behavior owns cancellation; a rule can invalidate only the
/// schedule's current request without re-running earlier branches or effects.
fn blocking(
    out: &mut String,
    statement: &Statement,
    model: &AiModel,
    depth: usize,
    flow: &mut Flow,
) -> Result<()> {
    let pad = "    ".repeat(depth);
    let group = flow.group;
    let exit = flow.exit();
    let id = flow.local;
    flow.local += 1;
    let captured_point = if let Statement::IncrementalTurnPosition { point, .. }
    | Statement::GoToPosition { point, .. } = statement
    {
        let value = guarded_expression(out, point, model, depth, flow)?;
        writeln!(out, "{pad}var request_point_{id}: Vector3 = {value}")?;
        Some(format!("request_point_{id}"))
    } else {
        None
    };
    let operand = match statement {
        Statement::Wait { milliseconds } => {
            Some(guarded_expression(out, milliseconds, model, depth, flow)?)
        }
        Statement::WaitFrames(frames)
        | Statement::ExecuteAction(frames)
        | Statement::IncrementalTurnToward { frames, .. } => {
            Some(guarded_expression(out, frames, model, depth, flow)?)
        }
        Statement::IncrementalTurnPosition { frames, .. } => {
            Some(guarded_expression(out, frames, model, depth, flow)?)
        }
        Statement::GoToPosition { radius, .. } => {
            let value = guarded_expression(out, radius, model, depth, flow)?;
            writeln!(out, "{pad}var request_radius_{id}: float = float({value})")?;
            Some(format!("request_radius_{id}"))
        }
        Statement::ExecuteActionAndGenerateObject { action, .. } => {
            Some(guarded_expression(out, action, model, depth, flow)?)
        }
        _ => None,
    };
    let vector = if let Statement::ExecuteActionAndGenerateObject { vector, .. } = statement {
        Some(guarded_expression(out, vector, model, depth, flow)?)
    } else {
        None
    };
    if flow.rule {
        writeln!(
            out,
            "{pad}{group}_rule_claimed = true\n{pad}{group}_action_revision += 1"
        )?;
    }
    writeln!(out, "{pad}while active_{group}(token):")?;
    if !flow.rule {
        writeln!(
            out,
            "{pad}    var action_revision_{id} := {group}_action_revision"
        )?;
    }
    let complete = match statement {
        Statement::Wait { .. } => {
            writeln!(
                out,
                "{pad}    var duration_{id} := float({})\n{pad}    assert(is_finite(duration_{id}) and duration_{id} >= 0.0)\n{pad}    var deadline_{id} := clock_milliseconds + int(duration_{id})",
                operand.as_deref().expect("wait operand prepared")
            )?;
            format!("clock_milliseconds + 0.0000001 >= deadline_{id}")
        }
        Statement::WaitFrames(_) => {
            writeln!(
                out,
                "{pad}    var remaining_{id} := int({})",
                operand.as_deref().expect("frame operand prepared")
            )?;
            format!("remaining_{id} <= 0")
        }
        Statement::IncrementalTurnToward { target, .. } => {
            let target = match target {
                ActorTarget::Keyword(ActorKeyword::SelfActor) => "character()".to_owned(),
                ActorTarget::Keyword(ActorKeyword::MainActor) => "main_actor".to_owned(),
                ActorTarget::Variable(index) => format!("dsg_{index}"),
                ActorTarget::Reference(actor) => format!(
                    "character().source_actor_node({})",
                    quoted(actor.path.as_str())
                ),
            };
            writeln!(
                out,
                "{pad}    var remaining_{id} := int({})\n{pad}    if remaining_{id} < 0: remaining_{id} = 1\n{pad}    var turn_angles_{id}: Vector2 = character().source_turn_angles({target})\n{pad}    if not is_finite(turn_angles_{id}.x) or not is_finite(turn_angles_{id}.y):\n{pad}        fail_unsupported({})\n{pad}        {exit}\n{pad}    var turn_heading_{id} := turn_angles_{id}.x\n{pad}    var turn_step_{id} := 0.0 if remaining_{id} == 0 else turn_angles_{id}.y / float(remaining_{id})",
                operand.as_deref().expect("turn frame operand prepared"),
                quoted(&format!(
                    "{} requires a valid native incremental turn target",
                    model.path
                ))
            )?;
            format!("remaining_{id} <= 0")
        }
        Statement::IncrementalTurnPosition { .. } => {
            writeln!(
                out,
                "{pad}    var remaining_{id} := int({})\n{pad}    var turn_angles_{id}: Vector2 = character().source_turn_angles_to_position({})\n{pad}    if not is_finite(turn_angles_{id}.x) or not is_finite(turn_angles_{id}.y):\n{pad}        fail_unsupported({})\n{pad}        {exit}\n{pad}    var turn_heading_{id} := turn_angles_{id}.x\n{pad}    var turn_step_{id} := turn_angles_{id}.y / (0.000001 if remaining_{id} == 0 else float(abs(remaining_{id})))",
                operand.as_deref().expect("turn frame operand prepared"),
                captured_point.as_deref().expect("turn point prepared"),
                quoted(&format!(
                    "{} requires finite native point-turn geometry",
                    model.path
                ))
            )?;
            format!("remaining_{id} <= 0 or not character().source_motion_binding_present")
        }
        Statement::GoToPosition { horizontal, .. } => {
            writeln!(
                out,
                "{pad}    var motion_ticket_{id}: int = character().source_begin_point_motion({}, {}, {horizontal})",
                captured_point.as_deref().expect("motion point prepared"),
                operand.as_deref().expect("motion radius prepared"),
            )?;
            point_motion_guard(out, model, &pad, id, exit)?;
            format!("character().source_point_motion_finished(motion_ticket_{id})")
        }
        Statement::GoToNextWaypoint(slot) => {
            writeln!(
                out,
                "{pad}    var motion_ticket_{id}: int = character().source_begin_next_waypoint({slot})",
            )?;
            point_motion_guard(out, model, &pad, id, exit)?;
            format!("character().source_point_motion_finished(motion_ticket_{id})")
        }
        Statement::ExecuteAction(_) => {
            writeln!(
                out,
                "{pad}    var requested_{id} := int({})\n{pad}    var accepted_{id} := character().request_action(requested_{id}, true)",
                operand.as_deref().expect("action operand prepared")
            )?;
            format!(
                "not accepted_{id} or not character().is_in_action(requested_{id}) or character().action_complete"
            )
        }
        Statement::ExecuteActionAndGenerateObject { template, .. } => {
            let index = model
                .object_templates()
                .iter()
                .position(|reference| reference == template)
                .ok_or_else(|| anyhow::anyhow!("AI object template was not declared"))?;
            writeln!(
                out,
                "{pad}    var requested_{id} := int({})\n{pad}    var spawn_result_{id} := character().source_execute_action_and_generate_object(object_template_{index}_path, requested_{id}, {})\n{pad}    if spawn_result_{id} < 0:\n{pad}        fail_unsupported({})\n{pad}        {exit}",
                operand.as_deref().expect("action operand prepared"),
                vector.as_deref().expect("vector operand prepared"),
                quoted(&format!(
                    "{} requires native action and object generation for {}",
                    model.path, template.path
                ))
            )?;
            format!(
                "spawn_result_{id} == 0 or not character().is_in_action(requested_{id}) or character().action_complete"
            )
        }
        Statement::WaitEndOfAction => "character().action_complete".into(),
        Statement::WaitEndOfAnimation => "character().animation_complete".into(),
        _ => unreachable!("nonblocking statement"),
    };
    writeln!(out, "{pad}    while active_{group}(token):")?;
    if !flow.rule {
        writeln!(
            out,
            "{pad}        if action_revision_{id} != {group}_action_revision: break"
        )?;
    } else {
        writeln!(out, "{pad}        {group}_rule_claimed = true")?;
    }
    if group == "reflex" {
        writeln!(out, "{pad}        reflex_controls_tick = true")?;
    }
    writeln!(out, "{pad}        if {complete}: break")?;
    if matches!(statement, Statement::WaitFrames(_)) {
        writeln!(out, "{pad}        remaining_{id} -= 1")?;
    }
    if matches!(
        statement,
        Statement::IncrementalTurnToward { .. } | Statement::IncrementalTurnPosition { .. }
    ) {
        writeln!(
            out,
            "{pad}        turn_heading_{id} += turn_step_{id}\n{pad}        character().source_turn_to_heading(turn_heading_{id})\n{pad}        remaining_{id} -= 1"
        )?;
    }
    writeln!(out, "{pad}        await {}", flow.tick())?;
    if matches!(
        statement,
        Statement::GoToPosition { .. } | Statement::GoToNextWaypoint(_)
    ) {
        // Query the completed request before cancellation clears its state.
        // A later reached unsupported link must stop the compiled caller.
        writeln!(
            out,
            "{pad}    var motion_failure_{id}: String = character().source_point_motion_failure(motion_ticket_{id})\n{pad}    character().source_cancel_point_motion(motion_ticket_{id})\n{pad}    if not active_{group}(token): {exit}\n{pad}    if not motion_failure_{id}.is_empty():\n{pad}        fail_unsupported(motion_failure_{id})\n{pad}        {exit}"
        )?;
    } else {
        writeln!(out, "{pad}    if not active_{group}(token): {exit}")?;
    }
    if !flow.rule {
        writeln!(
            out,
            "{pad}    if action_revision_{id} == {group}_action_revision: break"
        )?;
    } else {
        writeln!(out, "{pad}    break")?;
    }
    writeln!(out, "{pad}if not active_{group}(token): {exit}")?;
    Ok(())
}
fn point_motion_guard(
    out: &mut String,
    model: &AiModel,
    pad: &str,
    id: usize,
    exit: &str,
) -> Result<()> {
    writeln!(
        out,
        "{pad}    if motion_ticket_{id} < 0:\n{pad}        fail_unsupported({} + character().source_point_motion_failure(motion_ticket_{id}))\n{pad}        {exit}",
        quoted(&format!(
            "{} requires supported native point motion: ",
            model.path
        ))
    )?;
    Ok(())
}
impl Flow {
    fn exit(&self) -> &'static str {
        if self.rule { "return false" } else { "return" }
    }
    fn tick(&self) -> String {
        format!(
            "{}_{}tick",
            self.group,
            if self.rule { "" } else { "schedule_" }
        )
    }
}
fn statements(
    out: &mut String,
    values: &[Statement],
    model: &AiModel,
    events: &[(crate::model::audio::SoundEventLink, String)],
    depth: usize,
    flow: &mut Flow,
) -> Result<()> {
    let group = flow.group;
    let exit = flow.exit();
    let pad = "    ".repeat(depth);
    if values.is_empty() {
        writeln!(out, "{pad}pass")?;
    }
    for statement in values {
        if let Some(index) = opaque_accessed_variable(statement, model) {
            writeln!(
                out,
                "{pad}fail_unsupported({})\n{pad}{exit}",
                quoted(&format!(
                    "{} accesses opaque source designer field {index}",
                    model.path
                ))
            )?;
            continue;
        }
        match statement {
            Statement::Evaluate(value) => {
                let value = guarded_expression(out, value, model, depth, flow)?;
                let local = flow.local;
                flow.local += 1;
                writeln!(out, "{pad}var _discarded_{local} = {value}")?;
            }
            Statement::SetHitPointsMaximum { actor, value }
            | Statement::AddHitPoints { actor, value } => {
                let actor = guarded_expression(out, actor, model, depth, flow)?;
                let local = flow.local;
                flow.local += 1;
                writeln!(out, "{pad}var life_actor_{local}: Node = {actor}")?;
                let value = guarded_expression(out, value, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}if not is_instance_valid(life_actor_{local}) or life_actor_{local}.get(\"source_hit_points\") == null:\n{pad}    fail_unsupported(\"Life operation requires converted actor hit points\")\n{pad}    {exit}"
                )?;
                let method = if matches!(statement, Statement::AddHitPoints { .. }) {
                    "add_points"
                } else {
                    "set_maximum"
                };
                writeln!(
                    out,
                    "{pad}life_actor_{local}.get(\"source_hit_points\").{method}(int({value}))"
                )?;
            }
            Statement::SetMagicPointsMaximum(value) => {
                let value = guarded_expression(out, value, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}if not character().source_magic_points_ready():\n{pad}    fail_unsupported(\"Magick point state is unavailable\")\n{pad}    {exit}\n{pad}character().source_magick.change_maximum(int({value}))"
                )?;
            }
            Statement::SetInventoryMaximum { item, value } => {
                let item = guarded_expression(out, item, model, depth, flow)?;
                let local = flow.local;
                flow.local += 1;
                writeln!(out, "{pad}var inventory_item_{local}: int = int({item})")?;
                let value = guarded_expression(out, value, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}if character().source_inventory == null or not character().source_inventory.change_maximum_capacity(inventory_item_{local}, int({value})):\n{pad}    fail_unsupported(\"Inventory capacity state is unavailable\")\n{pad}    {exit}"
                )?;
            }
            Statement::ActivateMapName { name, active } => {
                let name = guarded_expression(out, name, model, depth, flow)?;
                let local = flow.local;
                flow.local += 1;
                writeln!(out, "{pad}var map_name_{local}: int = int({name})")?;
                let active = guarded_expression(out, active, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}if not character().source_activate_map_name(map_name_{local}, int({active})):\n{pad}    fail_unsupported(\"Localized map-name state is unavailable\")\n{pad}    {exit}"
                )?;
            }
            Statement::If {
                condition,
                then,
                otherwise,
            } => {
                let condition = guarded_expression(out, condition, model, depth, flow)?;
                writeln!(out, "{pad}if bool({condition}):")?;
                statements(out, then, model, events, depth + 1, flow)?;
                if let Some(otherwise) = otherwise {
                    writeln!(out, "{pad}else:")?;
                    statements(out, otherwise, model, events, depth + 1, flow)?;
                }
            }
            Statement::Assign { variable, value } => {
                let kind = &model.variables[usize::from(*variable)].default;
                let clears_source_reference = matches!(
                    (kind, value),
                    (DesignerValue::SourceWaypoint(_), Expression::Nowhere)
                        | (DesignerValue::SourceActor(_), Expression::Nobody)
                );
                if unresolved_variable(model, *variable) && !clears_source_reference {
                    writeln!(
                        out,
                        "{pad}fail_unsupported({})\n{pad}{exit}",
                        quoted(&format!(
                            "{} writes unresolved source designer field {variable}",
                            model.path
                        ))
                    )?;
                    continue;
                }
                if clears_source_reference {
                    writeln!(out, "{pad}dsg_{variable} = null")?;
                    continue;
                }
                let value_is_native_vector = matches!(
                    (kind, value),
                    (DesignerValue::Vector(_), Expression::SelfPositionField)
                );
                let value = guarded_expression(out, value, model, depth, flow)?;
                let value = if matches!(
                    kind,
                    DesignerValue::Actor(_)
                        | DesignerValue::Waypoint(_)
                        | DesignerValue::WaypointArray(_)
                        | DesignerValue::ActorList(_)
                ) || value_is_native_vector
                {
                    value
                } else {
                    format!("{}({value})", value_type(kind))
                };
                writeln!(out, "{pad}dsg_{variable} = {value}")?;
            }
            Statement::ActorFieldAssign {
                actor,
                variable,
                value,
            } => {
                let index = model
                    .actor_field_targets()
                    .iter()
                    .position(|target| target == actor)
                    .ok_or_else(|| anyhow::anyhow!("actor field target was not declared"))?;
                let local = flow.local;
                flow.local += 1;
                let node = format!("actor_brain_{index}_target_{local}");
                writeln!(
                    out,
                    "{pad}var {node} := actor_brain_{index}\n{pad}if not is_instance_valid({node}) and not actor_brain_{index}_path.is_empty():\n{pad}    {node} = get_node_or_null(actor_brain_{index}_path)\n{pad}if not is_instance_valid({node}):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    quoted(&format!(
                        "{} requires actor {} Brain field {variable}",
                        model.path, actor.path
                    ))
                )?;
                let value = guarded_expression(out, value, model, depth, flow)?;
                writeln!(out, "{pad}{node}.set(\"dsg_{variable}\", {value})")?;
            }
            Statement::MainActorFieldAssign { variable, value } => {
                let local = flow.local;
                flow.local += 1;
                let node = format!("main_actor_brain_target_{local}");
                let failure = quoted(&format!(
                    "{} requires main actor Brain field {variable}",
                    model.path
                ));
                writeln!(
                    out,
                    "{pad}if not is_instance_valid(main_actor):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}var {node} := main_actor.get_node_or_null(\"Brain\")\n{pad}if not is_instance_valid({node}):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}"
                )?;
                let value = guarded_expression(out, value, model, depth, flow)?;
                writeln!(out, "{pad}{node}.set(\"dsg_{variable}\", {value})")?;
            }
            Statement::SubtractAssign { variable, value } => {
                let kind = &model.variables[usize::from(*variable)].default;
                ensure!(
                    matches!(
                        kind,
                        DesignerValue::Byte(_)
                            | DesignerValue::UnsignedByte(_)
                            | DesignerValue::Short(_)
                            | DesignerValue::UnsignedShort(_)
                            | DesignerValue::Integer(_)
                            | DesignerValue::UnsignedInteger(_)
                            | DesignerValue::Real(_)
                    ),
                    "subtract assignment requires a numeric property"
                );
                let value = guarded_expression(out, value, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}dsg_{variable} = {}(dsg_{variable} - float({}))",
                    value_type(kind),
                    value
                )?;
            }
            Statement::MultiplyAssign { variable, value } => {
                let value = guarded_expression(out, value, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}dsg_{variable} = {}(dsg_{variable} * float({value}))",
                    value_type(&model.variables[usize::from(*variable)].default)
                )?;
            }
            Statement::SetStockCount { actor, count } => {
                let id = flow.local;
                flow.local += 1;
                let actor = guarded_expression(out, actor, model, depth, flow)?;
                writeln!(out, "{pad}var stock_actor_{id}: Variant = {actor}")?;
                let count = guarded_expression(out, count, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}if not character().source_set_stock_count(stock_actor_{id}, int({count})):\n{pad}    fail_unsupported(\"Stock count needs a converted stock binding\")\n{pad}    {exit}"
                )?;
            }
            Statement::DisplayPickup {
                item,
                milliseconds,
                from,
                to,
                x,
                y,
                width,
                height,
            } => {
                let mut values = Vec::new();
                for value in [item, milliseconds, from, to, x, y, width, height] {
                    let id = flow.local;
                    flow.local += 1;
                    let value = guarded_expression(out, value, model, depth, flow)?;
                    writeln!(out, "{pad}var pickup_display_{id} = {value}")?;
                    values.push(format!("pickup_display_{id}"));
                }
                writeln!(
                    out,
                    "{pad}if not character().source_display_pickup({}):\n{pad}    fail_unsupported(\"Pickup HUD requires scene presentation\")\n{pad}    {exit}",
                    values.join(", ")
                )?;
            }
            Statement::WaitBoolean {
                field, expected, ..
            }
            | Statement::WaitInteger {
                left: field,
                right: expected,
                ..
            } => {
                let id = flow.local;
                flow.local += 1;
                let (node, variable) = match field {
                    Expression::Variable(variable) | Expression::SelfFieldRead(variable) => ("self".into(), *variable),
                    Expression::MainActorFieldRead(variable) => ("(main_actor.get_node_or_null(\"Brain\") if is_instance_valid(main_actor) else null)".into(), *variable),
                    Expression::ActorFieldRead { actor, variable } => {
                        let index = model.actor_field_targets().iter().position(|target| target == actor).context("wait field actor missing")?;
                        (format!("(actor_brain_{index} if is_instance_valid(actor_brain_{index}) else get_node_or_null(actor_brain_{index}_path))"), *variable)
                    }
                    _ => {
                        let _ = guarded_expression(out, field, model, depth, flow)?;
                        let _ = guarded_expression(out, expected, model, depth, flow)?;
                        writeln!(out, "{pad}fail_unsupported(\"Wait requires an ordinary Brain property\")\n{pad}{exit}")?;
                        continue;
                    },
                };
                writeln!(
                    out,
                    "{pad}var wait_brain_{id}: Node = {node}\n{pad}if not is_instance_valid(wait_brain_{id}) or not wait_brain_{id}.get_property_list().any(func(property): return str(property.name) == \"dsg_{variable}\"):\n{pad}    fail_unsupported(\"Wait needs an existing Brain field\")\n{pad}    {exit}"
                )?;
                let selector = if let Statement::WaitInteger { selector, .. } = statement {
                    guarded_expression(out, selector, model, depth, flow)?
                } else {
                    "0".into()
                };
                writeln!(
                    out,
                    "{pad}var wait_selector_{id}: int = int({selector}) & 0xffffffff"
                )?;
                let value = guarded_expression(out, expected, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}var wait_expected_{id}: int = int({value}) & 0xffffffff\n{pad}if wait_expected_{id} >= 0x80000000: wait_expected_{id} -= 0x100000000"
                )?;
                if flow.rule {
                    writeln!(
                        out,
                        "{pad}{group}_rule_claimed = true\n{pad}{group}_action_revision += 1"
                    )?;
                }
                writeln!(
                    out,
                    "{pad}while active_{group}(token):\n{pad}    if not is_instance_valid(wait_brain_{id}):\n{pad}        fail_unsupported(\"Wait owner was removed\")\n{pad}        {exit}\n{pad}    var wait_observed_{id}: int = int(wait_brain_{id}.get(\"dsg_{variable}\")) & 0xffffffff\n{pad}    if wait_observed_{id} >= 0x80000000: wait_observed_{id} -= 0x100000000\n{pad}    var wait_complete_{id} := true\n{pad}    match wait_selector_{id}:\n{pad}        0: wait_complete_{id} = wait_observed_{id} == wait_expected_{id}\n{pad}        1: wait_complete_{id} = wait_observed_{id} != wait_expected_{id}\n{pad}        2: wait_complete_{id} = wait_observed_{id} < wait_expected_{id}\n{pad}        3: wait_complete_{id} = wait_observed_{id} > wait_expected_{id}\n{pad}        4: wait_complete_{id} = wait_observed_{id} <= wait_expected_{id}\n{pad}        5: wait_complete_{id} = wait_observed_{id} >= wait_expected_{id}\n{pad}    if wait_complete_{id}: break"
                )?;
                if flow.rule {
                    writeln!(out, "{pad}    {group}_rule_claimed = true")?;
                }
                if group == "reflex" {
                    writeln!(out, "{pad}    reflex_controls_tick = true")?;
                }
                writeln!(
                    out,
                    "{pad}    await {}\n{pad}if not active_{group}(token): {exit}",
                    flow.tick()
                )?;
            }
            Statement::Jump => {
                writeln!(out, "{pad}character().source_jump()")?;
            }
            Statement::AddAssign { variable, value } => {
                let value = guarded_expression(out, value, model, depth, flow)?;
                writeln!(out, "{pad}dsg_{variable} = dsg_{variable} + ({value})")?;
            }
            Statement::Increment { target } | Statement::Decrement { target } => {
                let delta = if matches!(statement, Statement::Increment { .. }) {
                    1
                } else {
                    -1
                };
                match target {
                    FieldTarget::Variable(variable) => {
                        let kind = &model.variables[usize::from(*variable)].default;
                        ensure!(
                            matches!(
                                kind,
                                DesignerValue::Byte(_)
                                    | DesignerValue::UnsignedByte(_)
                                    | DesignerValue::Short(_)
                                    | DesignerValue::UnsignedShort(_)
                                    | DesignerValue::Integer(_)
                                    | DesignerValue::UnsignedInteger(_)
                            ),
                            "field adjustment requires an integer designer property"
                        );
                        writeln!(out, "{pad}dsg_{variable} += {delta}")?;
                    }
                    FieldTarget::ActorField { actor, variable } => {
                        let index = model
                            .actor_field_targets()
                            .iter()
                            .position(|target| target == actor)
                            .ok_or_else(|| {
                                anyhow::anyhow!("actor field target was not declared")
                            })?;
                        let id = flow.local;
                        flow.local += 1;
                        let node = format!("actor_brain_{index}_adjust_{id}");
                        let failure = quoted(&format!(
                            "{} requires actor {} Brain field {variable}",
                            model.path, actor.path
                        ));
                        writeln!(
                            out,
                            "{pad}var {node} := actor_brain_{index}\n{pad}if not is_instance_valid({node}) and not actor_brain_{index}_path.is_empty():\n{pad}    {node} = get_node_or_null(actor_brain_{index}_path)\n{pad}if not is_instance_valid({node}):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}{node}.set(\"dsg_{variable}\", int({node}.get(\"dsg_{variable}\")) + ({delta}))"
                        )?;
                    }
                    FieldTarget::MainActorField(variable) => {
                        let id = flow.local;
                        flow.local += 1;
                        let node = format!("main_actor_brain_adjust_{id}");
                        let failure = quoted(&format!(
                            "{} requires main actor Brain field {variable}",
                            model.path
                        ));
                        writeln!(
                            out,
                            "{pad}if not is_instance_valid(main_actor):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}var {node} := main_actor.get_node_or_null(\"Brain\")\n{pad}if not is_instance_valid({node}):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}{node}.set(\"dsg_{variable}\", int({node}.get(\"dsg_{variable}\")) + ({delta}))"
                        )?;
                    }
                }
            }
            Statement::Turn { degrees_per_second } => {
                let value = guarded_expression(out, degrees_per_second, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}character().turn(float({}), frame_seconds)",
                    value
                )?;
            }
            Statement::SetVectorSpeed(value)
            | Statement::SetImpulse(value)
            | Statement::AddWind(value) => {
                let value = guarded_expression(out, value, model, depth, flow)?;
                let method = match statement {
                    Statement::SetVectorSpeed(_) => "source_set_vector_speed",
                    Statement::AddWind(_) => "source_add_wind",
                    _ => "source_set_impulse",
                };
                writeln!(out, "{pad}character().{method}({value})")?;
            }
            Statement::GoRelative => {
                writeln!(out, "{pad}character().source_go_relative()")?;
            }
            Statement::ResetRoute(slot) => {
                writeln!(
                    out,
                    "{pad}if not character().source_reset_route({slot}):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    quoted(&format!(
                        "{} requires actor-local route slot {slot}",
                        model.path
                    ))
                )?;
            }
            Statement::GoOverRoute(slot) => {
                let id = flow.local;
                flow.local += 1;
                if flow.rule {
                    writeln!(
                        out,
                        "{pad}{group}_rule_claimed = true\n{pad}{group}_action_revision += 1"
                    )?;
                }
                writeln!(
                    out,
                    "{pad}while active_{group}(token):\n{pad}    var route_revision_{id} := {group}_action_revision\n{pad}    var route_ticket_{id} := character().source_begin_route({slot})\n{pad}    if route_ticket_{id} < 0:\n{pad}        fail_unsupported({})\n{pad}        {exit}\n{pad}    while active_{group}(token) and route_revision_{id} == {group}_action_revision and not character().source_route_finished(route_ticket_{id}):",
                    quoted(&format!(
                        "{} requires supported native route slot {slot}",
                        model.path
                    ))
                )?;
                if flow.rule {
                    writeln!(out, "{pad}        {group}_rule_claimed = true")?;
                }
                if group == "reflex" {
                    writeln!(out, "{pad}        reflex_controls_tick = true")?;
                }
                writeln!(
                    out,
                    "{pad}        await {}\n{pad}    character().source_cancel_route(route_ticket_{id})\n{pad}    if not active_{group}(token): {exit}\n{pad}    if route_revision_{id} == {group}_action_revision: break",
                    flow.tick()
                )?;
            }
            Statement::KillSelf => {
                writeln!(out, "{pad}character().source_kill()\n{pad}{exit}")?;
            }
            Statement::KillPerso { target } => {
                // The pinned Hype PC dispatcher at 0x0053fc30 routes operation
                // 226 to its default return. Preserve the true source target in
                // the IR/comment, but do not invent a target-side deletion.
                let target = match target {
                    KillPersoTarget::SelfActor => "self".to_owned(),
                    KillPersoTarget::MainActor => "main actor".to_owned(),
                    KillPersoTarget::Reference(actor) => actor.path.to_string(),
                    KillPersoTarget::Source(identity) => format!(
                        "source {:?} block {:02x}:{:02x} allocation {}",
                        identity.role, identity.block[0], identity.block[1], identity.allocation
                    ),
                };
                writeln!(out, "{pad}pass # source Proc_KillPerso target: {target}")?;
            }
            Statement::ExecuteFade {
                from,
                to,
                duration_seconds,
            } => {
                let from = guarded_expression(out, from, model, depth, flow)?;
                let to = guarded_expression(out, to, model, depth, flow)?;
                let duration = guarded_expression(out, duration_seconds, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}if not source_execute_fade(int({from}), int({to}), float({duration})):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    quoted(&format!(
                        "{} requires the native scene fade API",
                        model.path
                    ))
                )?;
            }
            Statement::SetGameState(state) => {
                let state = guarded_expression(out, state, model, depth, flow)?;
                let id = flow.local;
                flow.local += 1;
                writeln!(
                    out,
                    "{pad}var source_session_{id} := source_game_session()\n{pad}if not is_instance_valid(source_session_{id}) or not source_session_{id}.has_method(\"source_set_game_state\"):\n{pad}    fail_unsupported({})\n{pad}    {exit}\n{pad}source_session_{id}.call(\"source_set_game_state\", int({state}))",
                    quoted(&format!(
                        "{} requires the active game session state API",
                        model.path
                    ))
                )?;
            }
            Statement::SetFadeTexture(texture) => {
                let value = guarded_expression(out, texture, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}if source_ui() == null or not source_ui().has_method(\"source_set_fade_texture\") or not source_ui().call(\"source_set_fade_texture\", int({value})):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    quoted(&format!(
                        "{} requires a source-bound STE_FADE texture",
                        model.path
                    ))
                )?;
            }
            Statement::SetFadeValue(value) => {
                let value = guarded_expression(out, value, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}if source_ui() == null or not source_ui().has_method(\"source_set_fade_value\") or not source_ui().call(\"source_set_fade_value\", int({value})):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    quoted(&format!(
                        "{} requires native source fade-value state",
                        model.path
                    ))
                )?;
            }
            Statement::ActivateFade | Statement::DeactivateFade => {
                let method = if matches!(statement, Statement::ActivateFade) {
                    "source_activate_fade"
                } else {
                    "source_deactivate_fade"
                };
                writeln!(
                    out,
                    "{pad}if source_ui() == null or not source_ui().has_method(\"{method}\"):\n{pad}    fail_unsupported({})\n{pad}    {exit}\n{pad}source_ui().call(\"{method}\")",
                    quoted(&format!(
                        "{} requires native source fade drawing",
                        model.path
                    ))
                )?;
            }
            Statement::JumpAbsolute => {
                writeln!(out, "{pad}character().source_jump_absolute()")?;
            }
            Statement::JumpWithoutAddingSpeed => {
                writeln!(out, "{pad}character().source_jump_without_adding_speed()")?;
            }
            Statement::SetParticleGeneratorEnabled(enabled) => {
                writeln!(
                    out,
                    "{pad}if not character().source_set_particle_generator_enabled({enabled}):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    quoted(&format!(
                        "{} requires supported native particle emission/selection state",
                        model.path
                    ))
                )?;
            }
            Statement::SetParticleGenerator(reference) => {
                writeln!(
                    out,
                    "{pad}if not character().source_set_particle_generator({}):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    super::particle::constant(reference),
                    quoted(&format!(
                        "{} could not select converted particle generator {}",
                        model.path, reference.path
                    ))
                )?;
            }
            Statement::SetLaserPointerActive(active) => {
                writeln!(
                    out,
                    "{pad}character().source_set_laser_pointer_active({active})"
                )?;
            }
            Statement::SetLaserPointerSniperMode(enabled) => {
                let value = guarded_expression(out, enabled, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}if not character().source_set_laser_pointer_sniper_mode(bool({value})):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    quoted(&format!("{} has no native laser-pointer state", model.path))
                )?;
            }
            Statement::SetLaserPointerFlyMode(enabled) => {
                let value = guarded_expression(out, enabled, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}if not character().source_set_laser_pointer_fly_mode(bool({value})):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    quoted(&format!("{} has no native laser-pointer state", model.path))
                )?;
            }
            Statement::SetDisplayVisible(visible) => {
                writeln!(
                    out,
                    "{pad}character().source_set_display_visible({visible})"
                )?;
            }
            Statement::SetActorDisplayVisible { actor, visible } => match actor {
                ActorTarget::Variable(_) => {
                    anyhow::bail!("dynamic display receiver is outside the supported slice")
                }
                ActorTarget::Keyword(ActorKeyword::SelfActor) => {
                    writeln!(
                        out,
                        "{pad}character().source_set_display_visible({visible})"
                    )?;
                }
                ActorTarget::Keyword(ActorKeyword::MainActor) => {
                    writeln!(
                        out,
                        "{pad}if not is_instance_valid(main_actor) or not character().source_actor_set_display_visible_from_receiver(main_actor, {visible}):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                        quoted(&format!(
                            "{} requires MainActor display visibility",
                            model.path
                        ))
                    )?;
                }
                ActorTarget::Reference(actor) => {
                    writeln!(
                        out,
                        "{pad}if not character().source_actor_set_display_visible({}, {visible}):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                        quoted(actor.path.as_str()),
                        quoted(&format!(
                            "{} requires display receiver {}",
                            model.path, actor.path
                        ))
                    )?;
                }
            },
            Statement::SetActorChannelActive {
                actor,
                channel,
                active,
            } => match actor {
                ActorTarget::Variable(_) => {
                    anyhow::bail!("dynamic channel receiver is outside the supported slice")
                }
                ActorTarget::Keyword(ActorKeyword::SelfActor) => {
                    writeln!(
                        out,
                        "{pad}if not character().source_set_channel_active({channel}, {active}):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                        quoted(&format!(
                            "{} requires character channel {channel}",
                            model.path
                        ))
                    )?;
                }
                ActorTarget::Keyword(ActorKeyword::MainActor) => {
                    writeln!(
                        out,
                        "{pad}if not is_instance_valid(main_actor) or not character().source_actor_set_channel_active_from_receiver(main_actor, {channel}, {active}):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                        quoted(&format!(
                            "{} requires MainActor channel {channel}",
                            model.path
                        ))
                    )?;
                }
                ActorTarget::Reference(actor) => {
                    writeln!(
                        out,
                        "{pad}if not character().source_actor_set_channel_active({}, {channel}, {active}):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                        quoted(actor.path.as_str()),
                        quoted(&format!(
                            "{} requires channel receiver {} channel {channel}",
                            model.path, actor.path
                        ))
                    )?;
                }
            },
            Statement::SetTransparency(value) => {
                let value = guarded_expression(out, value, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}if not character().source_set_transparency(int({value})):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    quoted(&format!(
                        "{} requires supported native opacity materials",
                        model.path
                    ))
                )?;
            }
            Statement::SetTransparentDisplay(enabled) => {
                writeln!(
                    out,
                    "{pad}if not character().source_set_transparent_display({enabled}):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    quoted(&format!(
                        "{} requires supported native transparent display materials",
                        model.path
                    ))
                )?;
            }
            Statement::ChangeSighting(direction) => {
                let value = guarded_expression(out, direction, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}if not character().source_change_sighting({value}):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    quoted(&format!(
                        "{} received invalid sighting direction",
                        model.path
                    ))
                )?;
            }
            Statement::CameraChangeTargetedPerso { camera, target } => {
                let failure = quoted(&format!(
                    "{} requires camera target state for {}",
                    model.path, camera.path
                ));
                writeln!(
                    out,
                    "{pad}if not character().source_camera_prepare_target_change({}):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}",
                    quoted(camera.path.as_str())
                )?;
                let target = match target {
                    ActorTarget::Variable(index) => format!("dsg_{index}"),
                    ActorTarget::Keyword(ActorKeyword::MainActor) => "main_actor".to_string(),
                    ActorTarget::Keyword(ActorKeyword::SelfActor) => "character()".to_string(),
                    ActorTarget::Reference(reference) => {
                        let local = format!("camera_target_{}", flow.local);
                        flow.local += 1;
                        writeln!(
                            out,
                            "{pad}var {local} := character().source_actor_node({})",
                            quoted(reference.path.as_str())
                        )?;
                        local
                    }
                };
                let failure = quoted(&format!(
                    "{} requires camera target state for {}",
                    model.path, camera.path
                ));
                writeln!(
                    out,
                    "{pad}if not is_instance_valid({target}):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}if not character().source_camera_change_targeted_perso({}, {target}):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}",
                    quoted(&camera.path.to_string())
                )?;
            }
            Statement::CameraResetTargetedPerso { camera } => {
                let failure = quoted(&format!(
                    "{} requires camera reset target state for {}",
                    model.path, camera.path
                ));
                writeln!(
                    out,
                    "{pad}if not is_instance_valid(main_actor):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}if not character().source_camera_reset_targeted_perso({}, main_actor):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}",
                    quoted(camera.path.as_str())
                )?;
            }
            Statement::CameraChangeState { camera, state } => {
                let value = guarded_expression(out, state, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}if not character().source_camera_change_state({}, int({value})):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    quoted(camera.path.as_str()),
                    quoted(&format!(
                        "{} requires supported camera state for {}",
                        model.path, camera.path
                    ))
                )?;
            }
            Statement::SetActorAbsolutePosition { actor, position } => {
                let value = guarded_expression(out, position, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}if not character().source_actor_set_absolute_position({}, {value}):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    quoted(actor.path.as_str()),
                    quoted(&format!(
                        "{} requires absolute position receiver {}",
                        model.path, actor.path
                    ))
                )?;
            }
            Statement::SetKeywordAbsolutePosition { actor, position } => {
                let value = guarded_expression(out, position, model, depth, flow)?;
                if *actor == ActorKeyword::MainActor {
                    writeln!(
                        out,
                        "{pad}if not is_instance_valid(main_actor) or not character().source_actor_set_absolute_position_from_receiver(main_actor, {value}):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                        quoted(&format!(
                            "{} requires MainActor absolute position",
                            model.path
                        ))
                    )?;
                } else {
                    writeln!(
                        out,
                        "{pad}if not character().source_set_absolute_position({value}):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                        quoted(&format!("{} requires self absolute position", model.path))
                    )?;
                }
            }
            Statement::SetObjectActive { actor, active } => {
                let local = format!("active_actor_{}", flow.local);
                flow.local += 1;
                writeln!(
                    out,
                    "{pad}var {local}: Node3D = {}\n{pad}if not is_instance_valid({local}) or not {local}.has_method(\"source_set_object_active_state\") or not {local}.source_set_object_active_state({active}):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    actor_target_code(actor),
                    quoted(&format!(
                        "{} requires source object active state for {}",
                        model.path,
                        actor_target_label(actor)
                    ))
                )?;
            }
            Statement::SetSelfAbsolutePosition(position) => {
                let value = guarded_expression(out, position, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}if not character().source_set_absolute_position({value}):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    quoted(&format!(
                        "{} requires a finite absolute position",
                        model.path
                    ))
                )?;
            }
            Statement::CameraUpdatePosition { .. } => {
                writeln!(
                    out,
                    "{pad}if not character().source_camera_update_position():\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    quoted(&format!(
                        "{} requires native camera position update",
                        model.path
                    ))
                )?;
            }
            Statement::ActorListAdd { list, actor } => {
                let capacity = actor_list_capacity(model, *list)?;
                let id = flow.local;
                flow.local += 1;
                let local = format!("list_actor_{id}");
                let failure = quoted(&format!(
                    "{} requires actor {} for designer list {list}",
                    model.path, actor.path
                ));
                writeln!(
                    out,
                    "{pad}var {local} := character().source_actor_node({})\n{pad}if not is_instance_valid({local}):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}if not character().source_list_add(dsg_{list}, {capacity}, {local}):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    quoted(actor.path.as_str()),
                    quoted(&format!(
                        "{} designer list {list} cannot append actor {}",
                        model.path, actor.path
                    ))
                )?;
            }
            Statement::SetChannelActive { channel, active } => {
                writeln!(
                    out,
                    "{pad}if not character().source_set_channel_active({channel}, {active}):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    quoted(&format!(
                        "{} requires converted character channel {channel}",
                        model.path
                    ))
                )?;
            }
            Statement::ChangeMap { level } => {
                writeln!(
                    out,
                    "{pad}if not LevelTransition.request_level_change({}, \"\", \"\"):\n{pad}    fail_unsupported({})\n{pad}    {exit}\n{pad}{exit}",
                    quoted(level),
                    quoted(&format!("{} could not request map {level:?}", model.path))
                )?;
            }
            Statement::ChangeMapAtPosition {
                level,
                entry,
                camera_entry,
            } => {
                writeln!(
                    out,
                    "{pad}if not LevelTransition.request_level_change({}, {}, {}):\n{pad}    fail_unsupported({})\n{pad}    {exit}\n{pad}{exit}",
                    quoted(level),
                    quoted(entry),
                    quoted(camera_entry),
                    quoted(&format!(
                        "{} could not request positioned map {level:?}",
                        model.path
                    ))
                )?;
            }
            Statement::ToggleWideScreen
            | Statement::PositionDialog { .. }
            | Statement::RunDialog { .. }
            | Statement::ScrollDialog(_)
            | Statement::SelectDialog
            | Statement::EndDialog
            | Statement::ChangeDialogBehavior => {
                writeln!(
                    out,
                    "{pad}if source_ui() == null:\n{pad}    fail_unsupported(\"Presentation call requires scene UI\")\n{pad}    {exit}"
                )?;
                match statement {
                    Statement::ToggleWideScreen => {
                        writeln!(out, "{pad}source_ui().source_toggle_widescreen()")?
                    }
                    Statement::PositionDialog { x, y } => {
                        let x = guarded_expression(out, x, model, depth, flow)?;
                        let y = guarded_expression(out, y, model, depth, flow)?;
                        writeln!(
                            out,
                            "{pad}source_ui().source_position_dialog(float({x}), float({y}))"
                        )?;
                    }
                    Statement::RunDialog { name, mode } => {
                        let mode = guarded_expression(out, mode, model, depth, flow)?;
                        writeln!(
                            out,
                            "{pad}if not source_ui().source_run_dialog({}, int({mode}), self):\n{pad}    fail_unsupported(\"Dialogue resource/binding or mode unavailable\")\n{pad}    {exit}",
                            quoted(name)
                        )?;
                    }
                    Statement::ScrollDialog(direction) => {
                        let direction = guarded_expression(out, direction, model, depth, flow)?;
                        writeln!(
                            out,
                            "{pad}source_ui().source_scroll_dialog(int({direction}))"
                        )?;
                    }
                    Statement::EndDialog => writeln!(out, "{pad}source_ui().source_end_dialog()")?,
                    Statement::ChangeDialogBehavior => writeln!(
                        out,
                        "{pad}if not source_ui().source_change_dialog_behavior():\n{pad}    fail_unsupported(\"Dialogue callback binding unavailable\")\n{pad}    {exit}"
                    )?,
                    Statement::SelectDialog => {
                        writeln!(out, "{pad}source_ui().source_select_dialog()")?
                    }
                    _ => unreachable!(),
                }
            }
            Statement::SetHudVisible { indicator, visible } => {
                let visible = guarded_expression(out, visible, model, depth, flow)?;
                let indicator = match indicator {
                    crate::model::ai::HudIndicator::Life => "life",
                    crate::model::ai::HudIndicator::Armor => "armor",
                    crate::model::ai::HudIndicator::Magic => "magic",
                    crate::model::ai::HudIndicator::SelectedObject => "selected_object",
                    crate::model::ai::HudIndicator::Weapon => "weapon",
                    crate::model::ai::HudIndicator::Cash => "cash",
                };
                writeln!(
                    out,
                    "{pad}character().source_set_hud_indicator_visible({}, bool({visible}))",
                    quoted(indicator)
                )?;
            }
            Statement::SelectArmor(index) => {
                let index = guarded_expression(out, index, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}if not source_select_armor(int({index})):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    quoted(&format!(
                        "{} requires native armor-selection state",
                        model.path
                    ))
                )?;
            }
            Statement::SetArmorResistance(rank) => {
                let rank = guarded_expression(out, rank, model, depth, flow)?;
                let armor = armor_receiver(out, model, depth, flow, true)?;
                writeln!(out, "{pad}{armor}.call(\"set_resistance\", int({rank}))")?;
            }
            Statement::RepairArmor => {
                let armor = armor_receiver(out, model, depth, flow, true)?;
                writeln!(out, "{pad}{armor}.call(\"repair\")")?;
            }
            Statement::ChangeConfiguration(value) => {
                let value = guarded_expression(out, value, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}if not character().source_change_configuration({value}):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    quoted(&format!(
                        "{} requires a bound native configuration",
                        model.path
                    ))
                )?;
            }
            Statement::SwapConfiguredParts { first, second } => {
                let first = guarded_expression(out, first, model, depth, flow)?;
                let second = guarded_expression(out, second, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}if not character().source_swap_configured_parts(int({first}), int({second})):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    quoted(&format!(
                        "{} requires a selected native configuration",
                        model.path
                    ))
                )?;
            }
            Statement::ArrayAssign {
                array,
                index,
                value,
            } => {
                let variable = match array {
                    Expression::Variable(variable) if !unresolved_variable(model, *variable) => {
                        *variable
                    }
                    _ if needs_guard(array, model) => {
                        guarded_expression(out, array, model, depth, flow)?;
                        continue;
                    }
                    _ => {
                        anyhow::bail!("array assignment requires a native designer array property")
                    }
                };
                let kind = match &model.variables[usize::from(variable)].default {
                    DesignerValue::IntegerArray(_) | DesignerValue::ActionArray(_) => "int",
                    DesignerValue::RealArray(_) => "float",
                    DesignerValue::VectorArray(_) => "Vector3",
                    _ => anyhow::bail!("AI assignment target dsg_{variable} is not an array"),
                };
                let index = guarded_expression(out, index, model, depth, flow)?;
                let value = guarded_expression(out, value, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}dsg_{variable}[int({})] = {kind}({})",
                    index, value
                )?;
            }
            Statement::VectorComponentAssign {
                vector,
                axis,
                value,
            } => {
                ensure!(
                    matches!(
                        &model.variables[usize::from(*vector)].default,
                        DesignerValue::Vector(_)
                    ),
                    "vector component assignment requires a vector designer property"
                );
                let value = guarded_expression(out, value, model, depth, flow)?;
                let component = match axis {
                    VectorAxis::X => "x",
                    VectorAxis::Y => "y",
                    VectorAxis::Z => "z",
                };
                let id = flow.local;
                flow.local += 1;
                writeln!(
                    out,
                    "{pad}var vector_component_{id}: Vector3 = dsg_{vector}\n{pad}vector_component_{id}.{component} = float({value})\n{pad}dsg_{vector} = vector_component_{id}"
                )?;
            }
            Statement::Wait { .. }
            | Statement::WaitFrames(_)
            | Statement::IncrementalTurnToward { .. }
            | Statement::IncrementalTurnPosition { .. }
            | Statement::GoToPosition { .. }
            | Statement::GoToNextWaypoint(_)
            | Statement::ExecuteAction(_)
            | Statement::ExecuteActionAndGenerateObject { .. }
            | Statement::WaitEndOfAction
            | Statement::WaitEndOfAnimation => {
                blocking(out, statement, model, depth, flow)?;
            }
            Statement::ChangeBehavior {
                target,
                pause_after,
            } => {
                ensure!(
                    target.model.path == model.path,
                    "change behavior target belongs to another AI model"
                );
                let target_group = group_name(target.group);
                let same_group = target_group == group;
                writeln!(
                    out,
                    "{pad}if change_{target_group}({}, {}):",
                    target.behavior,
                    if same_group && *pause_after {
                        "true"
                    } else {
                        "false"
                    }
                )?;
                writeln!(out, "{pad}    {}", if same_group { exit } else { "pass" })?;
                if *pause_after {
                    if flow.rule {
                        writeln!(
                            out,
                            "{pad}{group}_rule_claimed = true\n{pad}{group}_action_revision += 1"
                        )?;
                    }
                    writeln!(
                        out,
                        "{pad}await {}\n{pad}if not active_{group}(token): {exit}",
                        flow.tick()
                    )?;
                }
            }
            Statement::ChangeActorBehavior { actor, target } => {
                ensure!(
                    target.group == BehaviorGroup::Normal,
                    "Proc_ChangeComport requires a Normal behavior"
                );
                let id = flow.local;
                flow.local += 1;
                let actor_expression = actor_target_code(actor);
                let failure = quoted(&format!(
                    "{} Proc_ChangeComport requires a different target actor with a Brain using {}",
                    model.path, target.model.path
                ));
                let target_model_path = quoted(target.model.path.as_str());
                writeln!(
                    out,
                    "{pad}var change_actor_{id}: Node3D = {actor_expression}\n{pad}if not is_instance_valid(change_actor_{id}):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}if change_actor_{id} == character():\n{pad}    fail_unsupported({})\n{pad}    {exit}\n{pad}var change_brain_{id}: Node = change_actor_{id}.get_node_or_null(\"Brain\")\n{pad}if not is_instance_valid(change_brain_{id}) or str(change_brain_{id}.get(\"source_model_path\")) != {target_model_path} or not change_brain_{id}.has_method(\"change_normal\"):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}change_brain_{id}.call(\"change_normal\", {}, false)",
                    quoted(&format!(
                        "{} Proc_ChangeComport cannot switch the currently executing actor because the source callback's same-actor status semantics are unresolved",
                        model.path
                    )),
                    target.behavior
                )?;
            }
            Statement::ChangeAction(action) => {
                let value = guarded_expression(out, action, model, depth, flow)?;
                if flow.rule {
                    // A refused procedure stops the rest of this rule pass, but
                    // permits the schedule. It does not acquire a blocking action.
                    writeln!(
                        out,
                        "{pad}if not character().request_action(int({value})): return false"
                    )?;
                } else {
                    writeln!(
                        out,
                        "{pad}while not character().request_action(int({value})):\n{pad}    await {}\n{pad}    if not active_{group}(token): {exit}",
                        flow.tick()
                    )?;
                }
            }
            Statement::Sound { event: Some(event) } => {
                let i = events
                    .iter()
                    .position(|(e, _)| e == event)
                    .ok_or_else(|| anyhow::anyhow!("AI sound resource has not been exported"))?;
                writeln!(
                    out,
                    "{pad}send_sound(sound_{i})\n{pad}if not active_{group}(token): {exit}"
                )?;
            }
            Statement::SoundOnActor { actor, event } => {
                let id = flow.local;
                flow.local += 1;
                let receiver = format!("sound_receiver_{id}");
                writeln!(
                    out,
                    "{pad}var {receiver}: Node3D = {}",
                    actor_target_code(actor)
                )?;
                let event = if let Some(event) = event {
                    let i = events
                        .iter()
                        .position(|(e, _)| e == event)
                        .context("AI actor sound resource has not been exported")?;
                    format!("sound_{i}")
                } else {
                    "null".into()
                };
                writeln!(
                    out,
                    "{pad}if not send_sound_to_actor({receiver}, {event}):\n{pad}    fail_unsupported({})\n{pad}    {exit}\n{pad}if not active_{group}(token): {exit}",
                    quoted(&format!(
                        "{} requires native actor sound emitter",
                        model.path
                    ))
                )?;
            }
            Statement::SwitchMicrophone {
                actor,
                evaluated_second,
            } => {
                let id = flow.local;
                flow.local += 1;
                let actor = guarded_expression(out, actor, model, depth, flow)?;
                writeln!(out, "{pad}var microphone_actor_{id} = {actor}")?;
                if let Some(second) = evaluated_second {
                    let value = guarded_expression(out, second, model, depth, flow)?;
                    writeln!(out, "{pad}var _microphone_discarded_actor_{id} = {value}")?;
                }
                writeln!(
                    out,
                    "{pad}if not source_switch_microphone(microphone_actor_{id}):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    quoted(&format!(
                        "{} requires a bound native microphone actor",
                        model.path
                    ))
                )?;
            }
            Statement::MusicRequest { event: Some(event) } => {
                let i = events
                    .iter()
                    .position(|(e, _)| e == event)
                    .context("AI music resource has not been exported")?;
                writeln!(
                    out,
                    "{pad}send_music(sound_{i})\n{pad}if not active_{group}(token): {exit}"
                )?;
            }
            Statement::MusicRequest { event: None } => writeln!(
                out,
                "{pad}send_music(null)\n{pad}if not active_{group}(token): {exit}"
            )?,
            Statement::SoundFadeInObject {
                actor,
                target,
                duration_ms,
                event,
            } => {
                let id = flow.local;
                flow.local += 1;
                let receiver = format!("sound_receiver_{id}");
                let level = format!("sound_level_{id}");
                let duration = format!("sound_duration_{id}");
                writeln!(
                    out,
                    "{pad}var {receiver}: Node3D = {}\n{pad}if not is_instance_valid({receiver}):\n{pad}    fail_unsupported({})\n{pad}    {exit}",
                    actor_target_code(actor),
                    quoted(&format!(
                        "{} requires sound fade receiver {}",
                        model.path,
                        actor_target_label(actor)
                    ))
                )?;
                let target = guarded_expression(out, target, model, depth, flow)?;
                writeln!(out, "{pad}var {level}: int = int({target})")?;
                let duration_ms = guarded_expression(out, duration_ms, model, depth, flow)?;
                writeln!(out, "{pad}var {duration}: int = int({duration_ms})")?;
                let event = if let Some(event) = event {
                    let i = events
                        .iter()
                        .position(|(e, _)| e == event)
                        .context("AI object fade sound resource has not been exported")?;
                    format!("sound_{i}")
                } else {
                    "null".into()
                };
                writeln!(
                    out,
                    "{pad}if not sound_fade_in_object({receiver}, {level}, {duration}, {event}):\n{pad}    fail_unsupported({})\n{pad}    {exit}\n{pad}if not active_{group}(token): {exit}",
                    quoted(&format!(
                        "{} requires native object sound emitter",
                        model.path
                    ))
                )?;
            }
            Statement::SoundFadeMusic { arg0, arg1 } => {
                let arg0 = guarded_expression(out, arg0, model, depth, flow)?;
                let arg1 = guarded_expression(out, arg1, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}sound_fade_music(int({arg0}), int({arg1}))\n{pad}if not active_{group}(token): {exit}"
                )?;
            }
            Statement::SoundFadeVoice {
                target,
                duration_ms,
            } => {
                let target = guarded_expression(out, target, model, depth, flow)?;
                let duration = guarded_expression(out, duration_ms, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}sound_fade_voice(int({target}), int({duration}))\n{pad}if not active_{group}(token): {exit}"
                )?;
            }
            Statement::SoundFadeSound {
                target,
                duration_ms,
            } => {
                let target = guarded_expression(out, target, model, depth, flow)?;
                let duration = guarded_expression(out, duration_ms, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}sound_fade_sound(int({target}), int({duration}))\n{pad}if not active_{group}(token): {exit}"
                )?;
            }
            Statement::SoundFadeAll {
                target,
                duration_ms,
            } => {
                let target = guarded_expression(out, target, model, depth, flow)?;
                let duration = guarded_expression(out, duration_ms, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}sound_fade_all(int({target}), int({duration}))\n{pad}if not active_{group}(token): {exit}"
                )?;
            }
            Statement::SoundKillMusic => {
                writeln!(
                    out,
                    "{pad}sound_kill_music()\n{pad}if not active_{group}(token): {exit}"
                )?;
            }
            Statement::SetVisualGMTDiffuseCoef { material, value } => {
                let Expression::Variable(variable) = value else {
                    anyhow::bail!(
                        "Proc_SetVisualGMTDiffuseCoef requires a direct designer-variable operand"
                    );
                };
                let designer_variable = model
                    .variables
                    .get(usize::from(*variable))
                    .context("Proc_SetVisualGMTDiffuseCoef designer field is missing")?;
                ensure!(
                    matches!(
                        &designer_variable.default,
                        DesignerValue::Integer(_) | DesignerValue::UnsignedInteger(_)
                    ),
                    "Proc_SetVisualGMTDiffuseCoef requires a 32-bit integer designer field"
                );
                let failure = quoted(&format!(
                    "{} Proc_SetVisualGMTDiffuseCoef requires native shared visual-material state for {}",
                    model.path, material.path
                ));
                writeln!(
                    out,
                    "{pad}if not character().has_method(\"source_visual_material_target_ready\") or not character().has_method(\"source_set_visual_gmt_diffuse_coef\"):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}if not bool(character().call(\"source_visual_material_target_ready\", {})):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}",
                    quoted(material.path.as_str())
                )?;
                // OpenSpace resolves the GameMaterial and VisualMaterial before
                // evaluating this operand. Keep the live designer read after
                // the native target preflight so a missing target cannot consume
                // an unsupported or side-effecting source expression.
                let value = guarded_expression(out, value, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}if not bool(character().call(\"source_set_visual_gmt_diffuse_coef\", {}, int({value}))):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}if not active_{group}(token): {exit}",
                    quoted(material.path.as_str())
                )?;
            }
            Statement::AdjustPersoOnObject { .. } => {
                writeln!(
                    out,
                    "{pad}fail_unsupported({})\n{pad}{exit}",
                    quoted(&format!(
                        "{} reaches Action_AdjustPersoOnObject (0054ac30/0054ad00), whose native movement and completion semantics are not implemented",
                        model.path
                    ))
                )?;
            }
            Statement::Sound { event: None } | Statement::Noop => writeln!(out, "{pad}pass")?,
            Statement::Unsupported(fragment) => {
                let receiver = fragment
                    .receiver
                    .as_ref()
                    .map(|target| format!(" (receiver {})", actor_target_label(target)))
                    .unwrap_or_default();
                writeln!(
                    out,
                    "{pad}fail_unsupported({})\n{pad}{exit}",
                    quoted(&format!(
                        "{} source AI node {}{}: {}",
                        model.path, fragment.start, receiver, fragment.reason
                    ))
                )?;
            }
        }
    }
    Ok(())
}
fn brain_variable_initialization(
    parent: &str,
    index: usize,
    value: &DesignerValue,
    waypoints: &[WaypointBinding],
) -> Result<String> {
    if matches!(value, DesignerValue::ActorList(_)) {
        return Ok(format!(
            "var {parent}_brain_list_{index}: Array[Node3D] = []\n    {parent}_brain.dsg_{index} = {parent}_brain_list_{index}"
        ));
    }
    if matches!(
        value,
        DesignerValue::WaypointArray(_) | DesignerValue::SourceWaypointArray(_)
    ) {
        return Ok(format!(
            "{parent}_brain.dsg_{index}.assign({})",
            literal(value, waypoints, false)?
        ));
    }
    Ok(format!(
        "{parent}_brain.dsg_{index} = {}",
        literal(value, waypoints, false)?
    ))
}

pub(super) fn component(
    project: &mut Project,
    source: &mut impl ContentSource,
    perso: &Perso,
    parent: &str,
    written: &mut BTreeSet<AssetPath>,
) -> Result<()> {
    let Some(brain) = &perso.brain else {
        return Ok(());
    };
    let model = source.load(&brain.model)?;
    brain.validate_model(&model)?;
    let family = source.load(&perso.family)?;
    for value in brain
        .variables
        .iter()
        .chain(brain.initial_variables.iter().flatten())
    {
        if let DesignerValue::ActorList(list) = value {
            ensure!(
                list.active_len == 0,
                "populated source actor-list memory needs scene actor binding"
            );
        }
        for action in value.state_references() {
            action.validate_family(&family)?;
        }
    }
    for action in model.state_references() {
        action.validate_family(&family)?;
    }
    let mut waypoints = Vec::new();
    let mut waypoint_references = model.waypoint_references();
    // A Brain's current and initial memory can hold semantic waypoint assets
    // that are not the model's descriptor defaults. Export those instance
    // references at the actor-component seam as well, so the native Resource
    // constants used to initialize DSG fields always have a project-local file.
    for value in brain
        .variables
        .iter()
        .chain(brain.initial_variables.iter().flatten())
    {
        match value {
            DesignerValue::Waypoint(Some(reference)) => waypoint_references.push(reference),
            DesignerValue::WaypointArray(array) => {
                waypoint_references.extend(array.values.iter().flatten());
            }
            _ => {}
        }
    }
    for reference in waypoint_references {
        if waypoints
            .iter()
            .any(|(candidate, _, _)| candidate == reference)
        {
            continue;
        }
        let waypoint = source.load(reference)?;
        waypoint.validate_reference(&reference.path)?;
        waypoint.validate()?;
        let resource = super::route::export_waypoint(project, &waypoint)?;
        waypoints.push((
            (*reference).clone(),
            waypoint
                .relative_actor
                .is_none()
                .then_some(waypoint.position),
            Some(format!("res://{resource}")),
        ));
    }
    let script = export_program(project, source, &model, written)?;
    writeln!(
        project.builder,
        "    var {parent}_brain = Node3D.new()\n    {parent}_brain.name = \"Brain\"\n    {parent}_brain.set_script(native_resource({}))\n    {parent}.add_child({parent}_brain)\n    {parent}_brain.owner = {parent}",
        quoted(&format!("res://{script}"))
    )?;
    for (i, v) in brain.variables.iter().enumerate() {
        let initialization = brain_variable_initialization(parent, i, v, &waypoints)
            .with_context(|| format!("{} Brain designer field {i}", brain.model.path))?;
        writeln!(project.builder, "    {initialization}")?;
    }
    for (name, selection) in [("normal", &brain.normal), ("reflex", &brain.reflex)] {
        writeln!(
            project.builder,
            "    {parent}_brain.current_{name} = {}",
            selection
                .as_ref()
                .and_then(|s| s.current.or(s.initial))
                .map_or(-1, i64::from)
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod source_position_tests {
    use super::{compile, compile_with_templates, model_script_file};
    use crate::model::{
        AssetPath, AssetRef,
        ai::{
            ActorKeyword, ActorTarget, AiModel, Behavior, BehaviorGroup, BehaviorRef, BehaviorSet,
            Comparison, DesignerArray, DesignerValue, DesignerVariable, Expression,
            KillPersoTarget, PreservedAiNode, PreservedAiOperand, Script, Statement,
            UnsupportedFragment,
        },
        character::Perso,
    };

    #[test]
    fn compiles_saved_gate_branch_to_self_position_and_next_behavior() -> anyhow::Result<()> {
        let path = AssetPath::new("level:/ai/models/272")?;
        let global = AssetRef::<Perso>::new("fix:/persos/15")?;
        let next = BehaviorRef {
            model: AssetRef::<AiModel>::from_path(path.clone()),
            group: BehaviorGroup::Normal,
            behavior: 3,
        };
        let change_to_next = || Statement::ChangeBehavior {
            target: next.clone(),
            pause_after: false,
        };
        let script = Script {
            statements: vec![Statement::If {
                condition: Expression::Compare {
                    operation: Comparison::Equal,
                    left: Box::new(Expression::ActorFieldRead {
                        actor: global,
                        variable: 149,
                    }),
                    right: Box::new(Expression::Integer(1)),
                },
                then: vec![
                    Statement::SetSelfAbsolutePosition(Expression::VectorPlus(
                        Box::new(Expression::SelfPosition),
                        Box::new(Expression::Vector(Box::new([
                            Expression::Real(2.6),
                            Expression::Real(0.0),
                            Expression::Real(0.0),
                        ]))),
                    )),
                    change_to_next(),
                ],
                otherwise: Some(vec![change_to_next()]),
            }],
        };
        let model = AiModel {
            path,
            name: "iMGrille_Manoir_Pont".into(),
            normal: Some(BehaviorSet {
                behaviors: vec![
                    Behavior {
                        id: 0,
                        rules: Vec::new(),
                        schedule: Some(script),
                        unknown: [0; 3],
                    },
                    Behavior {
                        id: 3,
                        rules: Vec::new(),
                        schedule: None,
                        unknown: [0; 3],
                    },
                ],
                default_behavior: Some(0),
                action_capacity: 0,
                unknown: [0; 3],
            }),
            reflex: None,
            variables: Vec::new(),
            designer_unknown: [0; 3],
            unknown: [0; 4],
        };
        let generated = compile(&model, &[])?;
        assert!(generated.contains("character().source_set_absolute_position("));
        assert!(generated.contains(
            "character().source_position() + Vector3(float(2.6), float(0.0), float(0.0))"
        ));
        assert!(generated.contains("if change_normal(3, false):"));
        assert!(!generated.contains("unsupported AI statement category 4 operation 253"));
        Ok(())
    }

    #[test]
    fn compiles_ultra_display_receiver_to_its_named_perso() -> anyhow::Result<()> {
        let actor = crate::model::AssetRef::<Perso>::new("level:/persos/world/0/174")?;
        let model = AiModel {
            path: AssetPath::new("level:/ai/models/ultra_display_fixture")?,
            name: "Ultra display receiver fixture".into(),
            normal: Some(BehaviorSet {
                behaviors: vec![Behavior {
                    id: 0,
                    rules: Vec::new(),
                    schedule: Some(Script {
                        statements: vec![
                            Statement::SetActorDisplayVisible {
                                actor: ActorTarget::Reference(actor.clone()),
                                visible: false,
                            },
                            Statement::SetActorChannelActive {
                                actor: ActorTarget::Reference(actor.clone()),
                                channel: 7,
                                active: false,
                            },
                        ],
                    }),
                    unknown: [0; 3],
                }],
                default_behavior: Some(0),
                action_capacity: 0,
                unknown: [0; 3],
            }),
            reflex: None,
            variables: Vec::new(),
            designer_unknown: [0; 3],
            unknown: [0; 4],
        };

        let generated = compile(&model, &[])?;
        assert!(generated.contains(
            "character().source_actor_set_display_visible(\"level:/persos/world/0/174\", false)"
        ));
        assert!(generated.contains(
            "character().source_actor_set_channel_active(\"level:/persos/world/0/174\", 7, false)"
        ));
        assert!(!generated.contains("Ultra receiver/category"));
        Ok(())
    }

    #[test]
    fn compiles_ultra_distance_receiver_and_target_as_named_persos() -> anyhow::Result<()> {
        let target = crate::model::AssetRef::<Perso>::new("level:/persos/world/0/174")?;
        let model = AiModel {
            path: AssetPath::new("level:/ai/models/ultra_distance_fixture")?,
            name: "Ultra distance receiver fixture".into(),
            normal: Some(BehaviorSet {
                behaviors: vec![Behavior {
                    id: 0,
                    rules: Vec::new(),
                    schedule: Some(Script {
                        statements: vec![Statement::If {
                            condition: Expression::Compare {
                                operation: Comparison::Less,
                                left: Box::new(Expression::ActorDistanceFrom {
                                    receiver: ActorTarget::Keyword(ActorKeyword::MainActor),
                                    target: Box::new(Expression::Actor(ActorTarget::Reference(
                                        target,
                                    ))),
                                }),
                                right: Box::new(Expression::Integer(5)),
                            },
                            then: Vec::new(),
                            otherwise: None,
                        }],
                    }),
                    unknown: [0; 3],
                }],
                default_behavior: Some(0),
                action_capacity: 0,
                unknown: [0; 3],
            }),
            reflex: None,
            variables: Vec::new(),
            designer_unknown: [0; 3],
            unknown: [0; 4],
        };

        let generated = compile(&model, &[])?;
        assert!(generated.contains("var distance_receiver_0: Node3D = main_actor"));
        assert!(generated.contains("character().source_actor_node(\"level:/persos/world/0/174\")"));
        assert!(generated.contains("distance_receiver_0.global_position.distance_to("));
        assert!(generated.contains("distance requires receiver MainActor"));
        Ok(())
    }

    #[test]
    fn unsupported_ultra_calls_keep_the_resolved_receiver_in_failure_output() -> anyhow::Result<()>
    {
        let actor = crate::model::AssetRef::<Perso>::new("level:/persos/world/0/174")?;
        let model = AiModel {
            path: AssetPath::new("level:/ai/models/ultra_unsupported_fixture")?,
            name: "Ultra unsupported receiver fixture".into(),
            normal: Some(BehaviorSet {
                behaviors: vec![Behavior {
                    id: 0,
                    rules: Vec::new(),
                    schedule: Some(Script {
                        statements: vec![Statement::Unsupported(UnsupportedFragment {
                            reason: "Ultra receiver/category 24 does not support category 4 operation 413".into(),
                            start: 0,
                            nodes: vec![PreservedAiNode {
                                category: 2,
                                indent: 1,
                                reserved: [0; 2],
                                operand: PreservedAiOperand::NonRelocated([0; 4]),
                            }],
                            receiver: Some(ActorTarget::Reference(actor)),
                        })],
                    }),
                    unknown: [0; 3],
                }],
                default_behavior: Some(0),
                action_capacity: 0,
                unknown: [0; 3],
            }),
            reflex: None,
            variables: Vec::new(),
            designer_unknown: [0; 3],
            unknown: [0; 4],
        };

        let generated = compile(&model, &[])?;
        assert!(generated.contains("(receiver level:/persos/world/0/174)"));
        assert!(generated.contains("category 4 operation 413"));
        Ok(())
    }

    #[test]
    fn adjust_perso_on_object_fails_closed_until_native_update_is_supported() -> anyhow::Result<()>
    {
        let model = AiModel {
            path: AssetPath::new("level:/ai/models/adjust_perso_fixture")?,
            name: "Adjust Perso fixture".into(),
            normal: Some(BehaviorSet {
                behaviors: vec![Behavior {
                    id: 0,
                    rules: Vec::new(),
                    schedule: Some(Script {
                        statements: vec![Statement::AdjustPersoOnObject {
                            target: Expression::Variable(0),
                            near_threshold: Expression::Real(1.3),
                            far_threshold: Expression::Real(1.4),
                            far_action: Expression::Action(None),
                            near_action: Expression::Action(None),
                        }],
                    }),
                    unknown: [0; 3],
                }],
                default_behavior: Some(0),
                action_capacity: 0,
                unknown: [0; 3],
            }),
            reflex: None,
            variables: vec![DesignerVariable {
                default: DesignerValue::Actor(None),
                save: 0,
                initialize: 0,
            }],
            designer_unknown: [0; 3],
            unknown: [0; 4],
        };

        let generated = compile(&model, &[])?;
        assert!(generated.contains("reaches Action_AdjustPersoOnObject (0054ac30/0054ad00)"));
        assert!(generated.contains("fail_unsupported("));
        assert!(!generated.contains("source_adjust_perso_on_object"));
        assert!(!generated.contains("request_action(int(dsg_0)"));
        Ok(())
    }

    #[test]
    fn compiles_both_source_map_transition_forms_to_native_level_api() -> anyhow::Result<()> {
        let path = AssetPath::new("level:/ai/models/334")?;
        let model = AiModel {
            path,
            name: "iDummy_Load_Manoir_MAD".into(),
            normal: Some(BehaviorSet {
                behaviors: vec![Behavior {
                    id: 0,
                    rules: vec![Some(Script {
                        statements: vec![
                            Statement::ChangeMap {
                                level: "ciel1".into(),
                            },
                            Statement::ChangeMapAtPosition {
                                level: "brigand".into(),
                                entry: "i_brigand_dummy_gogood_hype".into(),
                                camera_entry: "i_brigand_dummy_gogood_camera".into(),
                            },
                        ],
                    })],
                    schedule: None,
                    unknown: [0; 3],
                }],
                default_behavior: Some(0),
                action_capacity: 0,
                unknown: [0; 3],
            }),
            reflex: None,
            variables: Vec::new(),
            designer_unknown: [0; 3],
            unknown: [0; 4],
        };
        let generated = compile(&model, &[])?;
        assert!(generated.contains("LevelTransition.request_level_change(\"ciel1\", \"\", \"\")"));
        assert!(generated.contains(
            "LevelTransition.request_level_change(\"brigand\", \"i_brigand_dummy_gogood_hype\", \"i_brigand_dummy_gogood_camera\")"
        ));
        Ok(())
    }

    #[test]
    fn compares_vector_designer_fields_without_scalar_casts() -> anyhow::Result<()> {
        let path = AssetPath::new("level:/ai/models/vector_comparison")?;
        let vector = |variable| DesignerVariable {
            default: variable,
            save: 0,
            initialize: 0,
        };
        let model = AiModel {
            path,
            name: "Vector comparison".into(),
            normal: Some(BehaviorSet {
                behaviors: vec![Behavior {
                    id: 0,
                    rules: Vec::new(),
                    schedule: Some(Script {
                        statements: vec![
                            Statement::If {
                                condition: Expression::Compare {
                                    operation: Comparison::Equal,
                                    left: Box::new(Expression::Variable(0)),
                                    right: Box::new(Expression::Vector(Box::new([
                                        Expression::Real(0.0),
                                        Expression::Real(0.0),
                                        Expression::Real(0.0),
                                    ]))),
                                },
                                then: vec![Statement::Noop],
                                otherwise: None,
                            },
                            Statement::If {
                                condition: Expression::Compare {
                                    operation: Comparison::Different,
                                    left: Box::new(Expression::ArrayElement {
                                        array: Box::new(Expression::Variable(1)),
                                        index: Box::new(Expression::Integer(0)),
                                    }),
                                    right: Box::new(Expression::Variable(0)),
                                },
                                then: vec![Statement::Noop],
                                otherwise: None,
                            },
                        ],
                    }),
                    unknown: [0; 3],
                }],
                default_behavior: Some(0),
                action_capacity: 0,
                unknown: [0; 3],
            }),
            reflex: None,
            variables: vec![
                vector(DesignerValue::Vector([0.0; 3])),
                vector(DesignerValue::VectorArray(DesignerArray::new(vec![[
                    1.0, 2.0, 3.0,
                ]]))),
            ],
            designer_unknown: [0; 3],
            unknown: [0; 4],
        };
        let generated = compile(&model, &[])?;
        assert!(generated.contains("(dsg_0 == Vector3("), "{generated}");
        assert!(
            generated.contains("(dsg_1[int(0)] != dsg_0)"),
            "{generated}"
        );
        assert!(
            !generated.contains("float(dsg_0) == float(Vector3("),
            "{generated}"
        );
        assert!(!generated.contains("float(dsg_1[int(0)])"), "{generated}");
        Ok(())
    }

    #[test]
    fn fix_and_level_ai_models_with_same_number_keep_distinct_scene_scripts() -> anyhow::Result<()>
    {
        let fix = AssetPath::new("fix:/ai/models/535")?;
        let level = AssetPath::new("level:/ai/models/535")?;
        assert_eq!(model_script_file(&fix), "ai/models/fix/535.gd");
        assert_eq!(model_script_file(&level), "ai/models/535.gd");
        assert_ne!(model_script_file(&fix), model_script_file(&level));
        Ok(())
    }

    #[test]
    fn compiles_the_tunnel_fade_audio_and_positioned_transition_in_source_order()
    -> anyhow::Result<()> {
        let path = AssetPath::new("level:/ai/models/334")?;
        let waypoint = crate::model::AssetRef::<crate::model::route::Waypoint>::new(
            "level:/waypoints/17-1/58",
        )?;
        let target = crate::model::ai::SourceObjectIdentity {
            role: crate::model::ai::SourceRole::Level,
            block: [5, 1],
            allocation: 9217,
        };
        let transition = Statement::If {
            condition: Expression::Compare {
                operation: Comparison::Less,
                left: Box::new(Expression::DistanceToWaypoint {
                    actor: ActorTarget::Keyword(crate::model::ai::ActorKeyword::MainActor),
                    waypoint: waypoint.clone(),
                }),
                right: Box::new(Expression::Real(5.0)),
            },
            then: vec![
                Statement::SetGameState(Expression::Integer(1)),
                Statement::ExecuteFade {
                    from: Expression::Integer(0),
                    to: Expression::Integer(255),
                    duration_seconds: Expression::Real(2.0),
                },
                Statement::KillPerso {
                    target: KillPersoTarget::Source(target),
                },
                Statement::SoundFadeAll {
                    target: Expression::Integer(0),
                    duration_ms: Expression::Integer(2000),
                },
                Statement::Wait {
                    milliseconds: Expression::Integer(2000),
                },
                Statement::SoundKillMusic,
                Statement::SoundFadeAll {
                    target: Expression::Integer(127),
                    duration_ms: Expression::Integer(1),
                },
                Statement::ChangeMapAtPosition {
                    level: "brigand".into(),
                    entry: "i_brigand_dummy_gogood_hype".into(),
                    camera_entry: "i_brigand_dummy_gogood_camera".into(),
                },
            ],
            otherwise: None,
        };
        let model = AiModel {
            path,
            name: "iDummy_Load_Manoir_MAD".into(),
            normal: Some(BehaviorSet {
                behaviors: vec![Behavior {
                    id: 0,
                    rules: Vec::new(),
                    schedule: Some(Script {
                        statements: vec![transition],
                    }),
                    unknown: [0; 3],
                }],
                default_behavior: Some(0),
                action_capacity: 0,
                unknown: [0; 3],
            }),
            reflex: None,
            variables: Vec::new(),
            designer_unknown: [0; 3],
            unknown: [0; 4],
        };
        let generated = compile_with_templates(
            &model,
            &[],
            None,
            &[],
            &[(waypoint, Some([0.0, 0.0, 0.0]), None)],
            &[],
        )?;
        let sequence = [
            "source_game_session()",
            "call(\"source_set_game_state\", int(1))",
            "if not source_execute_fade(int(0), int(255), float(2.0)):",
            "pass # source Proc_KillPerso target: source Level block 05:01 allocation 9217",
            "sound_fade_all(int(0), int(2000))",
            "var duration_2 := float(2000)",
            "sound_kill_music()",
            "sound_fade_all(int(127), int(1))",
            "LevelTransition.request_level_change(\"brigand\", \"i_brigand_dummy_gogood_hype\", \"i_brigand_dummy_gogood_camera\")",
        ];
        let positions = sequence
            .iter()
            .map(|text| {
                generated
                    .find(text)
                    .unwrap_or_else(|| panic!("missing {text:?} in generated code:\n{generated}"))
            })
            .collect::<Vec<_>>();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
        Ok(())
    }

    #[test]
    fn declares_waypoints_nested_in_integer_casts() -> anyhow::Result<()> {
        let path = AssetPath::new("level:/ai/models/waypoint-cast")?;
        let waypoint = crate::model::AssetRef::<crate::model::route::Waypoint>::new(
            "level:/waypoints/17-1/58",
        )?;
        let model = AiModel {
            path,
            name: "waypoint-cast".into(),
            normal: Some(BehaviorSet {
                behaviors: vec![Behavior {
                    id: 0,
                    rules: Vec::new(),
                    schedule: Some(Script {
                        statements: vec![Statement::Wait {
                            milliseconds: Expression::IntegerCast(Box::new(
                                Expression::WaypointPosition(waypoint.clone()),
                            )),
                        }],
                    }),
                    unknown: [0; 3],
                }],
                default_behavior: Some(0),
                action_capacity: 0,
                unknown: [0; 3],
            }),
            reflex: None,
            variables: Vec::new(),
            designer_unknown: [0; 3],
            unknown: [0; 4],
        };
        assert_eq!(model.waypoint_references(), vec![&waypoint]);
        let generated = compile_with_templates(
            &model,
            &[],
            None,
            &[],
            &[(waypoint.clone(), Some([1.0, 2.0, 3.0]), None)],
            &[],
        )?;
        let symbol = super::waypoint_constant(&waypoint);
        assert!(generated.contains(&format!("const {symbol} = Vector3(")));
        assert!(generated.contains(&format!(" = {symbol}")));
        Ok(())
    }

    #[test]
    fn declares_direct_waypoint_values_as_native_resources() -> anyhow::Result<()> {
        let path = AssetPath::new("level:/ai/models/direct-waypoint")?;
        let waypoint = crate::model::AssetRef::<crate::model::route::Waypoint>::new(
            "level:/waypoints/17-1/56",
        )?;
        let model = AiModel {
            path,
            name: "direct-waypoint".into(),
            normal: Some(BehaviorSet {
                behaviors: vec![Behavior {
                    id: 0,
                    rules: Vec::new(),
                    schedule: Some(Script {
                        statements: vec![Statement::MainActorFieldAssign {
                            variable: 0,
                            value: Expression::WaypointReference(waypoint.clone()),
                        }],
                    }),
                    unknown: [0; 3],
                }],
                default_behavior: Some(0),
                action_capacity: 0,
                unknown: [0; 3],
            }),
            reflex: None,
            variables: vec![crate::model::ai::DesignerVariable {
                default: crate::model::ai::DesignerValue::Waypoint(Some(waypoint.clone())),
                save: 0,
                initialize: 0,
            }],
            designer_unknown: [0; 3],
            unknown: [0; 4],
        };
        let resource = "res://waypoints/17-1/56.tres";
        let generated = compile_with_templates(
            &model,
            &[],
            None,
            &[],
            &[(
                waypoint.clone(),
                Some([1.0, 2.0, 3.0]),
                Some(resource.into()),
            )],
            &[],
        )?;
        let constant = super::waypoint_resource_constant(&waypoint);
        assert!(generated.contains(&format!(
            "func {constant}() -> Resource:\n    return load(\"{resource}\") as Resource"
        )));
        assert!(generated.contains(&format!(".set(\"dsg_0\", {constant}())")));
        assert!(
            generated.contains(
                "@export var dsg_0: Resource = null\n\nfunc _init() -> void:\n    super()"
            )
        );
        assert!(
            generated.contains(&format!("    dsg_0 = load(\"{resource}\")")),
            "{generated}"
        );
        assert!(!generated.contains("dsg_0.assign("));
        assert!(!generated.contains(&format!("preload(\"{resource}\")")));
        let array = crate::model::ai::DesignerValue::WaypointArray(DesignerArray::new(vec![
            Some(waypoint.clone()),
            None,
        ]));
        assert_eq!(
            super::literal(
                &array,
                &[(waypoint.clone(), None, Some(resource.into()))],
                true
            )?,
            format!("[{constant}(), null]")
        );
        Ok(())
    }

    #[test]
    fn generated_waypoint_array_instance_assignments_parse_in_godot() -> anyhow::Result<()> {
        let waypoints = [(
            crate::model::AssetRef::<crate::model::route::Waypoint>::new(
                "level:/waypoints/17-1/56",
            )?,
            None,
            Some("res://generated/hype/levels/manoir/waypoints/17-1/56.tres".into()),
        )];
        let resolved = DesignerValue::WaypointArray(DesignerArray::new(vec![None; 20]));
        let unresolved = DesignerValue::SourceWaypointArray(DesignerArray::new(vec![None]));
        let resolved_assignment =
            super::brain_variable_initialization("actor", 157, &resolved, &waypoints)?;
        let unresolved_assignment =
            super::brain_variable_initialization("actor", 158, &unresolved, &waypoints)?;
        assert!(resolved_assignment.starts_with("actor_brain.dsg_157.assign([null"));
        assert!(unresolved_assignment == "actor_brain.dsg_158.assign([null])");
        assert!(!resolved_assignment.contains("Array[Resource]("));

        let script = format!(
            "extends RefCounted\nvar actor_brain: Variant\nvar dsg_157: Array[Resource] = []\nvar dsg_158: Variant = []\nfunc _init() -> void:\n    actor_brain = self\n    {resolved_assignment}\n    {unresolved_assignment}\n"
        );
        let project = tempfile::tempdir()?;
        crate::godot::support::initialize(project.path())?;
        std::fs::write(project.path().join("waypoint_array_builder.gd"), script)?;
        let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
        crate::godot::run_godot(
            &engine,
            project.path(),
            &[
                "--headless",
                "--check-only",
                "--script",
                "res://waypoint_array_builder.gd",
                "--log-file",
                "waypoint-array-builder.log",
            ],
            std::time::Duration::from_secs(15),
        )?;
        Ok(())
    }

    #[test]
    fn compares_and_assigns_waypoint_resources_without_numeric_casts() -> anyhow::Result<()> {
        let path = AssetPath::new("level:/ai/models/waypoint-resource-comparison")?;
        let waypoint = crate::model::AssetRef::<crate::model::route::Waypoint>::new(
            "level:/waypoints/17-1/58",
        )?;
        let model = AiModel {
            path,
            name: "waypoint-resource-comparison".into(),
            normal: Some(BehaviorSet {
                behaviors: vec![Behavior {
                    id: 0,
                    rules: Vec::new(),
                    schedule: Some(Script {
                        statements: vec![Statement::If {
                            condition: Expression::Compare {
                                operation: Comparison::Equal,
                                left: Box::new(Expression::Variable(0)),
                                right: Box::new(Expression::Variable(1)),
                            },
                            then: vec![Statement::Assign {
                                variable: 0,
                                value: Expression::WaypointReference(waypoint.clone()),
                            }],
                            otherwise: None,
                        }],
                    }),
                    unknown: [0; 3],
                }],
                default_behavior: Some(0),
                action_capacity: 0,
                unknown: [0; 3],
            }),
            reflex: None,
            variables: vec![0, 1]
                .into_iter()
                .map(|_| crate::model::ai::DesignerVariable {
                    default: crate::model::ai::DesignerValue::Waypoint(Some(waypoint.clone())),
                    save: 0,
                    initialize: 0,
                })
                .collect(),
            designer_unknown: [0; 3],
            unknown: [0; 4],
        };
        let resource_path = "res://generated/hype/levels/manoir/waypoints/17-1/58.tres";
        let generated = compile_with_templates(
            &model,
            &[],
            None,
            &[],
            &[(waypoint.clone(), None, Some(resource_path.into()))],
            &[],
        )?;

        assert!(
            generated.contains("if bool((dsg_0 == dsg_1))"),
            "{generated}"
        );
        assert!(generated.contains("dsg_0 = waypoint_asset_"), "{generated}");
        assert!(!generated.contains("float(dsg_0)"), "{generated}");
        assert!(
            !generated.contains("Resource(waypoint_asset_"),
            "{generated}"
        );
        Ok(())
    }

    #[test]
    fn compiles_ultra_waypoint_distance_to_resolved_source_nodes() -> anyhow::Result<()> {
        let path = AssetPath::new("level:/ai/models/tunnel-trigger")?;
        let actor = crate::model::AssetRef::<Perso>::new("level:/persos/world/0/34")?;
        let waypoint = crate::model::AssetRef::<crate::model::route::Waypoint>::new(
            "level:/waypoints/17-1/58",
        )?;
        let model = AiModel {
            path,
            name: "tunnel-trigger".into(),
            normal: Some(BehaviorSet {
                behaviors: vec![Behavior {
                    id: 0,
                    rules: vec![Some(Script {
                        statements: vec![Statement::If {
                            condition: Expression::Compare {
                                operation: Comparison::Less,
                                left: Box::new(Expression::DistanceToWaypoint {
                                    actor: ActorTarget::Reference(actor.clone()),
                                    waypoint: waypoint.clone(),
                                }),
                                right: Box::new(Expression::Real(5.0)),
                            },
                            then: vec![Statement::Noop],
                            otherwise: None,
                        }],
                    })],
                    schedule: None,
                    unknown: [0; 3],
                }],
                default_behavior: Some(0),
                action_capacity: 0,
                unknown: [0; 3],
            }),
            reflex: None,
            variables: Vec::new(),
            designer_unknown: [0; 3],
            unknown: [0; 4],
        };
        let generated = compile_with_templates(
            &model,
            &[],
            None,
            &[],
            &[(waypoint, Some([-85.744194, 14.145693, 1.973552]), None)],
            &[],
        )?;
        assert!(generated.contains("character().source_actor_node(\"level:/persos/world/0/34\")"));
        assert!(generated.contains("source_distance_to_waypoint(distance_receiver_"));
        assert!(generated.contains("requires resolved absolute waypoint level:/waypoints/17-1/58"));
        Ok(())
    }

    #[test]
    fn compiles_camera_shift_getters_with_selectors_and_guarded_perso_receivers()
    -> anyhow::Result<()> {
        let model = AiModel {
            path: AssetPath::new("fix:/ai/models/camera-shift-alpha")?,
            name: "Camera Shift Alpha".into(),
            normal: Some(BehaviorSet {
                behaviors: vec![Behavior {
                    id: 0,
                    rules: vec![Some(Script {
                        statements: vec![
                            Statement::If {
                                condition: Expression::Compare {
                                    operation: Comparison::Greater,
                                    left: Box::new(Expression::CameraShiftAlpha {
                                        selector: Box::new(Expression::Integer(1)),
                                        receiver: ActorTarget::Keyword(ActorKeyword::SelfActor),
                                    }),
                                    right: Box::new(Expression::Real(0.0)),
                                },
                                then: vec![Statement::Noop],
                                otherwise: None,
                            },
                            Statement::If {
                                condition: Expression::Compare {
                                    operation: Comparison::Greater,
                                    left: Box::new(Expression::CameraShiftTeta {
                                        selector: Box::new(Expression::Integer(0)),
                                        receiver: ActorTarget::Keyword(ActorKeyword::SelfActor),
                                    }),
                                    right: Box::new(Expression::Real(0.0)),
                                },
                                then: vec![Statement::Noop],
                                otherwise: None,
                            },
                        ],
                    })],
                    schedule: None,
                    unknown: [0; 3],
                }],
                default_behavior: Some(0),
                action_capacity: 0,
                unknown: [0; 3],
            }),
            reflex: None,
            variables: Vec::new(),
            designer_unknown: [0; 3],
            unknown: [0; 4],
        };
        let generated = compile(&model, &[])?;
        assert!(
            generated.contains("source_camera_get_shift_alpha\", int(1)"),
            "{generated}"
        );
        assert!(
            generated.contains("source_camera_get_shift_teta\", int(0)"),
            "{generated}"
        );
        assert!(
            generated.contains("camera_shift_teta_actor_1 = character()"),
            "{generated}"
        );
        assert!(
            generated.contains("camera_shift_actor_0 = character()"),
            "{generated}"
        );
        assert!(
            generated.contains(
                "Function_Camera_GetShiftAlpha requires a mapped selector on a bound Perso"
            ),
            "{generated}"
        );
        assert!(
            generated.contains(
                "Function_Camera_GetShiftTeta requires a mapped selector on a bound Perso"
            ),
            "{generated}"
        );
        Ok(())
    }
}
