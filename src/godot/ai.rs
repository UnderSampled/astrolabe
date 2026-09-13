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
    let script = compile(model, &events)?;
    project.write(path.clone(), script.as_bytes())?;
    Ok(path)
}
/// Exposed for authored tooling and focused tests; dependencies have already been
/// resolved to native resource paths by this adapter, never by generated code.
pub fn compile(
    model: &AiModel,
    events: &[(crate::model::audio::SoundEventLink, String)],
) -> Result<String> {
    model.validate()?;
    ensure!(
        model.reflex.as_ref().is_none_or(|s| s
            .behaviors
            .iter()
            .all(|b| b.schedule.is_none() && b.rules.iter().all(Option::is_none)))
            && model.normal.as_ref().is_none_or(|s| s
                .behaviors
                .iter()
                .all(|b| b.rules.iter().all(Option::is_none))),
        "Godot AI currently supports normal schedules; rule/reflex scheduling requires source validation"
    );
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
    out.push_str("\nfunc _run(token: int) -> void:\n");
    let mut any = false;
    for (name, set) in [("normal", &model.normal), ("reflex", &model.reflex)] {
        let Some(set) = set else { continue };
        if set.behaviors.is_empty() {
            continue;
        }
        any = true;
        writeln!(out, "    match current_{name}:")?;
        for behavior in &set.behaviors {
            writeln!(out, "        {}:", behavior.id)?;
            if behavior.rules.iter().all(Option::is_none) && behavior.schedule.is_none() {
                out.push_str("            pass\n");
            }
            for (i, _) in behavior
                .rules
                .iter()
                .enumerate()
                .filter(|(_, s)| s.is_some())
            {
                writeln!(out, "            {name}_{}_rule_{i}(token)", behavior.id)?;
            }
            if behavior.schedule.is_some() {
                writeln!(out, "            {name}_{}_schedule(token)", behavior.id)?;
            }
        }
        writeln!(
            out,
            "        -1:\n            pass\n        _:\n            push_error(\"Unknown compiled {name} behavior\")\n            stop()"
        )?;
    }
    if !any {
        out.push_str("    pass\n");
    }
    for (name, set) in [("normal", &model.normal), ("reflex", &model.reflex)] {
        for behavior in set.iter().flat_map(|s| &s.behaviors) {
            for (suffix, script) in behavior
                .rules
                .iter()
                .enumerate()
                .filter_map(|(i, s)| s.as_ref().map(|s| (format!("rule_{i}"), s)))
                .chain(behavior.schedule.iter().map(|s| ("schedule".into(), s)))
            {
                writeln!(
                    out,
                    "\nfunc {name}_{}_{suffix}(token: int) -> void:\n    while active(token):",
                    behavior.id
                )?;
                statements(&mut out, &script.statements, model, events, 2)?;
                out.push_str("        if not await next_tick(token): return\n");
            }
        }
    }
    Ok(out)
}
fn value_type(v: &DesignerValue) -> &'static str {
    match v {
        DesignerValue::Boolean(_) => "bool",
        DesignerValue::Integer(_) => "int",
        DesignerValue::Real(_) => "float",
    }
}
fn literal(v: &DesignerValue) -> String {
    match v {
        DesignerValue::Boolean(v) => v.to_string(),
        DesignerValue::Integer(v) => v.to_string(),
        DesignerValue::Real(v) => format!("{v:?}"),
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
        Expression::MainActorDistance => "distance_to_main_actor()".into(),
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
fn statements(
    out: &mut String,
    values: &[Statement],
    model: &AiModel,
    events: &[(crate::model::audio::SoundEventLink, String)],
    depth: usize,
) -> Result<()> {
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
                statements(out, then, model, events, depth + 1)?;
                if let Some(otherwise) = otherwise {
                    writeln!(out, "{pad}else:")?;
                    statements(out, otherwise, model, events, depth + 1)?;
                }
            }
            Statement::Assign { variable, value } => {
                writeln!(
                    out,
                    "{pad}dsg_{variable} = {}({})",
                    value_type(&model.variables[usize::from(*variable)].default),
                    expression(value, model)?
                )?;
            }
            Statement::Wait { milliseconds } => {
                writeln!(
                    out,
                    "{pad}if not await wait_milliseconds(float({}), token): return",
                    expression(milliseconds, model)?
                )?;
            }
            Statement::Sound { event: Some(event) } => {
                let i = events
                    .iter()
                    .position(|(e, _)| e == event)
                    .ok_or_else(|| anyhow::anyhow!("AI sound resource has not been exported"))?;
                writeln!(
                    out,
                    "{pad}send_sound(sound_{i})\n{pad}if not active(token): return"
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
