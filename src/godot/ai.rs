//! Typed AI trees become ordinary GDScript control flow and native resource calls.
use super::{Project, disk_path, quoted};
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
    let path = disk_path(&project.selection, &model.path, "gd");
    if project.generated.contains(&path) {
        return Ok(path);
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
    let script = compile_for_family(model, &events, family.as_deref())?;
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
        DesignerValue::Actor(_) => "Node3D",
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
        DesignerValue::Actor(_) => "null".into(),
    }
}
fn expression(e: &Expression, model: &AiModel) -> Result<String> {
    Ok(match e {
        Expression::Integer(v) => v.to_string(),
        Expression::Real(v) => format!("{v:?}"),
        Expression::Variable(v) => {
            ensure!(
                usize::from(*v) < model.variables.len(),
                "AI variable missing"
            );
            format!("dsg_{v}")
        }
        Expression::Action(v) => v.as_ref().map_or(-1, |r| i64::from(r.state)).to_string(),
        Expression::IsInAction(action) => format!(
            "character().is_in_action(int({}))",
            expression(action, model)?
        ),
        Expression::ActionFinished => "character().action_complete".into(),
        Expression::IsInBehavior(target) => format!(
            "(current_{} == {})",
            group_name(target.group),
            target.behavior
        ),
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
        Expression::Multiply(a, b) => format!(
            "(float({}) * float({}))",
            expression(a, model)?,
            expression(b, model)?
        ),
        Expression::Negate(v) => format!("(-float({}))", expression(v, model)?),
        Expression::Compare {
            operation,
            left,
            right,
        } => {
            let op = match operation {
                Comparison::Equal => "==",
                Comparison::Different => "!=",
                Comparison::Less => "<",
                Comparison::Greater => ">",
                Comparison::LessOrEqual => "<=",
                Comparison::GreaterOrEqual => ">=",
            };
            format!(
                "(float({}) {op} float({}))",
                expression(left, model)?,
                expression(right, model)?
            )
        }
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
        Statement::Wait { milliseconds } => {
            writeln!(
                out,
                "{pad}    var duration_{id} := float({})\n{pad}    assert(is_finite(duration_{id}) and duration_{id} >= 0.0)\n{pad}    var deadline_{id} := clock_milliseconds + int(duration_{id})",
                expression(milliseconds, model)?
            )?;
            format!("clock_milliseconds + 0.0000001 >= deadline_{id}")
        }
        Statement::WaitFrames(frames) => {
            writeln!(
                out,
                "{pad}    var remaining_{id} := int({})",
                expression(frames, model)?
            )?;
            format!("remaining_{id} <= 0")
        }
        Statement::ExecuteAction(action) => {
            writeln!(
                out,
                "{pad}    var requested_{id} := int({})\n{pad}    var accepted_{id} := character().request_action(requested_{id}, true)",
                expression(action, model)?
            )?;
            format!(
                "not accepted_{id} or not character().is_in_action(requested_{id}) or character().action_complete"
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
                writeln!(out, "{pad}if bool({}):", expression(condition, model)?)?;
                statements(out, then, model, events, depth + 1, flow)?;
                if let Some(otherwise) = otherwise {
                    writeln!(out, "{pad}else:")?;
                    statements(out, otherwise, model, events, depth + 1, flow)?;
                }
            }
            Statement::Assign { variable, value } => {
                let kind = &model.variables[usize::from(*variable)].default;
                let value = expression(value, model)?;
                let value = if matches!(kind, DesignerValue::Actor(_)) {
                    value
                } else {
                    format!("{}({value})", value_type(kind))
                };
                writeln!(out, "{pad}dsg_{variable} = {value}")?;
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
                writeln!(
                    out,
                    "{pad}dsg_{variable} = {}(dsg_{variable} - float({}))",
                    value_type(kind),
                    expression(value, model)?
                )?;
            }
            Statement::Turn { degrees_per_second } => {
                writeln!(
                    out,
                    "{pad}character().turn(float({}), frame_seconds)",
                    expression(degrees_per_second, model)?
                )?;
            }
            Statement::ArrayAssign {
                array,
                index,
                value,
            } => {
                let Expression::Variable(variable) = array else {
                    anyhow::bail!("array assignment requires a native designer array property");
                };
                let kind = match &model.variables[usize::from(*variable)].default {
                    DesignerValue::IntegerArray(_) | DesignerValue::ActionArray(_) => "int",
                    DesignerValue::RealArray(_) => "float",
                    DesignerValue::VectorArray(_) => "Vector3",
                    _ => anyhow::bail!("AI assignment target dsg_{variable} is not an array"),
                };
                writeln!(
                    out,
                    "{pad}dsg_{variable}[int({})] = {kind}({})",
                    expression(index, model)?,
                    expression(value, model)?
                )?;
            }
            Statement::Wait { .. }
            | Statement::WaitFrames(_)
            | Statement::ExecuteAction(_)
            | Statement::WaitEndOfAction
            | Statement::WaitEndOfAnimation => {
                blocking(out, statement, model, depth, flow)?;
            }
            Statement::ChangeBehavior {
                target,
                pause_after,
            } => {
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
                let value = expression(action, model)?;
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
            Statement::Sound { event: None } | Statement::Noop => writeln!(out, "{pad}pass")?,
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
