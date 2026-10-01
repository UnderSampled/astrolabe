//! Typed AI trees become ordinary GDScript control flow and native resource calls.
use super::{Project, quoted};
use crate::model::{AssetPath, ai::*, character::Perso, content::ContentSource, io::Content};
use anyhow::{Result, ensure};
use std::{collections::BTreeSet, fmt::Write as _};

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
        let brain = target
            .brain
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("{} actor field target lacks a Brain", actor.path))?;
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
    let path = project.level_path(&format!("{}.gd", model.path.file()));
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
    let script = compile_with_templates(model, &events, family.as_deref(), &templates)?;
    project.write(path.clone(), script.as_bytes())?;
    Ok(path)
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
    compile_with_templates(model, events, family, &[])
}
fn compile_with_templates(
    model: &AiModel,
    events: &[(crate::model::audio::SoundEventLink, String)],
    family: Option<&crate::model::character::Family>,
    templates: &[(crate::model::AssetRef<Perso>, String)],
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
    for (i, v) in model.variables.iter().enumerate() {
        writeln!(
            out,
            "@export var dsg_{i}: {} = {}",
            value_type(&v.default),
            literal(&v.default)
        )?;
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
    }
}
fn literal(v: &DesignerValue) -> String {
    let array = |kind: &str, values: Vec<String>| format!("{kind}([{}])", values.join(", "));
    match v {
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
        DesignerValue::ActorList(_) => "[]".into(),
    }
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
fn actor_list_capacity(model: &AiModel, index: u16) -> Result<usize> {
    match model.variables.get(usize::from(index)).map(|v| &v.default) {
        Some(DesignerValue::ActorList(list)) => Ok(list.slots.len()),
        _ => anyhow::bail!("AI field {index} is not a source actor list"),
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
    } else {
        format!("(float({left_code}) {op} float({right_code}))")
    }
}
fn needs_guard(e: &Expression, model: &AiModel) -> bool {
    match e {
        Expression::Unsupported(_)
        | Expression::CollideWithNothing
        | Expression::LaserDirection
        | Expression::MagicCurrentSpell
        | Expression::MagicPoints
        | Expression::MagicRemovePoints(_) => true,
        Expression::IsInBehavior {
            actor: ActorKeyword::MainActor,
            ..
        } => true,
        Expression::Variable(index) => unresolved_variable(model, *index),
        Expression::ActorListContains { actor, .. } => needs_guard(actor, model),
        Expression::ActorFieldRead { .. } | Expression::MainActorFieldRead(_) => true,
        Expression::Vector(values) => values.iter().any(|value| needs_guard(value, model)),
        Expression::Multiply(a, b)
        | Expression::Plus(a, b)
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
        | Expression::InventoryQuantity(value) => needs_guard(value, model),
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
        Expression::Unsupported(fragment) => {
            let id = flow.local;
            flow.local += 1;
            let local = format!("partial_expr_{id}");
            writeln!(
                out,
                "{pad}var {local}\n{pad}fail_unsupported({})\n{pad}{exit}",
                quoted(&format!(
                    "{} source AI node {}: {}",
                    model.path, fragment.start, fragment.reason
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
        Expression::Vector(values) => format!(
            "Vector3(float({}), float({}), float({}))",
            guarded_expression(out, &values[0], model, depth, flow)?,
            guarded_expression(out, &values[1], model, depth, flow)?,
            guarded_expression(out, &values[2], model, depth, flow)?
        ),
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
        Expression::MagicCurrentSpell | Expression::MagicPoints => {
            writeln!(
                out,
                "{pad}if not character().source_magic_ready():\n{pad}    fail_unsupported({})\n{pad}    {exit}",
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
        Expression::IsInBehavior {
            actor: ActorKeyword::MainActor,
            target,
        } => {
            let id = flow.local;
            flow.local += 1;
            let brain = format!("behavior_brain_{id}");
            let failure = quoted(&format!(
                "{} requires MainActor Brain model {} for behavior query",
                model.path, target.model.path
            ));
            writeln!(
                out,
                "{pad}if not is_instance_valid(main_actor):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}var {brain} := main_actor.get_node_or_null(\"Brain\")\n{pad}if not is_instance_valid({brain}) or str({brain}.get(\"source_model_path\")) != {}:\n{pad}    fail_unsupported({failure})\n{pad}    {exit}",
                quoted(target.model.path.as_str())
            )?;
            format!(
                "(int({brain}.get(\"current_{}\")) == {})",
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
        Expression::Integer(v) => v.to_string(),
        Expression::Real(v) => format!("{v:?}"),
        Expression::SelfPosition => "character().source_position()".into(),
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
        Expression::ActionFinished => "character().action_complete".into(),
        Expression::CollideWithGround => "character().source_collide_with_ground()".into(),
        Expression::CollideWithNothing => "character().source_collide_with_nothing()".into(),
        Expression::CollidePersoZdeNoWithTypeZde { zone, maximum } => {
            format!("character().source_collide_perso_zde_no_with_type_zde({zone}, {maximum})")
        }
        Expression::CollideWithSlope => "character().source_collide_with_slope()".into(),
        Expression::IsTypeOfGmtCollide(kind) => {
            format!("character().source_is_type_of_gmt_collide({kind})")
        }
        Expression::InTopOfJump => "character().source_in_top_of_jump()".into(),
        Expression::IsInBehavior { actor, target } => {
            ensure!(
                *actor == ActorKeyword::SelfActor && target.model.path == model.path,
                "external behavior query requires a guarded actor binding"
            );
            format!(
                "(current_{} == {})",
                group_name(target.group),
                target.behavior
            )
        }
        Expression::ArrayElement { array, index } => format!(
            "{}[int({})]",
            expression(array, model)?,
            expression(index, model)?
        ),
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
        Expression::InventoryWeapon => "character().source_inventory_weapon()".into(),
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
/// A blocking request is an ordinary coroutine with captured native values.
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
    let operand = match statement {
        Statement::Wait { milliseconds } => {
            Some(guarded_expression(out, milliseconds, model, depth, flow)?)
        }
        Statement::WaitFrames(frames) | Statement::ExecuteAction(frames) => {
            Some(guarded_expression(out, frames, model, depth, flow)?)
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
    writeln!(
        out,
        "{pad}        await {}\n{pad}    if not active_{group}(token): {exit}",
        flow.tick()
    )?;
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
        match statement {
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
                let value = guarded_expression(out, value, model, depth, flow)?;
                let value = if matches!(kind, DesignerValue::Actor(_)) {
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
            Statement::SetVectorSpeed(value) | Statement::SetImpulse(value) => {
                let value = guarded_expression(out, value, model, depth, flow)?;
                let method = if matches!(statement, Statement::SetVectorSpeed(_)) {
                    "source_set_vector_speed"
                } else {
                    "source_set_impulse"
                };
                writeln!(out, "{pad}character().{method}({value})")?;
            }
            Statement::GoRelative => {
                writeln!(out, "{pad}character().source_go_relative()")?;
            }
            Statement::KillSelf => {
                writeln!(out, "{pad}character().source_kill()\n{pad}{exit}")?;
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
                    "{pad}character().source_set_particle_generator_enabled({enabled})"
                )?;
            }
            Statement::SetLaserPointerActive(active) => {
                writeln!(
                    out,
                    "{pad}character().source_set_laser_pointer_active({active})"
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
                ensure!(
                    *target == ActorKeyword::MainActor,
                    "camera targeting requires MainActor"
                );
                let failure = quoted(&format!(
                    "{} requires camera target state for {}",
                    model.path, camera.path
                ));
                writeln!(
                    out,
                    "{pad}if not is_instance_valid(main_actor):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}\n{pad}if not character().source_camera_change_targeted_perso({}, main_actor):\n{pad}    fail_unsupported({failure})\n{pad}    {exit}",
                    quoted(&camera.path.to_string())
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
            Statement::SetHudVisible { indicator, visible } => {
                let visible = guarded_expression(out, visible, model, depth, flow)?;
                let indicator = match indicator {
                    crate::model::ai::HudIndicator::Life => "life",
                    crate::model::ai::HudIndicator::Armor => "armor",
                    crate::model::ai::HudIndicator::Magic => "magic",
                    crate::model::ai::HudIndicator::SelectedObject => "selected_object",
                    crate::model::ai::HudIndicator::Weapon => "weapon",
                };
                writeln!(
                    out,
                    "{pad}character().source_set_hud_indicator_visible({}, bool({visible}))",
                    quoted(indicator)
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
            Statement::SoundFadeMusic { arg0, arg1 } => {
                let arg0 = guarded_expression(out, arg0, model, depth, flow)?;
                let arg1 = guarded_expression(out, arg1, model, depth, flow)?;
                writeln!(
                    out,
                    "{pad}sound_fade_music(int({arg0}), int({arg1}))\n{pad}if not active_{group}(token): {exit}"
                )?;
            }
            Statement::Sound { event: None } | Statement::Noop => writeln!(out, "{pad}pass")?,
            Statement::Unsupported(fragment) => {
                writeln!(
                    out,
                    "{pad}fail_unsupported({})\n{pad}{exit}",
                    quoted(&format!(
                        "{} source AI node {}: {}",
                        model.path, fragment.start, fragment.reason
                    ))
                )?;
            }
        }
    }
    Ok(())
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
    let script = export_program(project, source, &model, written)?;
    writeln!(
        project.builder,
        "    var {parent}_brain = Node3D.new()\n    {parent}_brain.name = \"Brain\"\n    {parent}_brain.set_script(native_resource({}))\n    {parent}.add_child({parent}_brain)\n    {parent}_brain.owner = {parent}",
        quoted(&format!("res://{script}"))
    )?;
    for (i, v) in brain.variables.iter().enumerate() {
        if matches!(v, DesignerValue::ActorList(_)) {
            writeln!(
                project.builder,
                "    var {parent}_brain_list_{i}: Array[Node3D] = []\n    {parent}_brain.dsg_{i} = {parent}_brain_list_{i}"
            )?;
            continue;
        }
        writeln!(
            project.builder,
            "    {parent}_brain.dsg_{i} = {}",
            literal(v)
        )?;
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
