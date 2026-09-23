//! Field-level comparison of a declared narrow actor slice. Static/default and
//! runtime values remain separate; a matched subset does not imply full parity.
use anyhow::{Context, Result, ensure};
use astrolabe::{
    model::{
        AssetRef,
        character::{Family, Perso},
        io::Load,
        scene::Scene,
    },
    openspace::level::live::{ActorObservation, ActorProjection},
};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct StaticActor {
    pub scene: String,
    pub node: String,
    pub actor: String,
    pub family: String,
    pub file_global_transform: astrolabe::model::geometry::SourceTransform,
    pub projection: ActorProjection,
}

#[derive(Debug, Serialize)]
pub struct StaticRoutes {
    pub direct: StaticActor,
    pub reopened_rete: StaticActor,
    pub source_diagnostics: Vec<astrolabe::model::io::Diagnostic>,
}

/// Import once, retain only owned comparison values, persist the actual useful
/// assets, drop the source resolver, then reopen the Rete store from disk.
pub fn prepare_routes(game: &Path, level: &str, rete_output: &Path) -> Result<StaticRoutes> {
    ensure!(
        !rete_output.exists(),
        "Rete evidence destination already exists"
    );
    let selection = astrolabe::model::Selection::hype(level, "en")?;
    let (direct, source_diagnostics) = {
        let mut imported = astrolabe::openspace::content::import_scene(game, selection.clone())?;
        let direct = load_main_actor(&mut imported, &[47])?;
        let inventory = imported.inventory.clone();
        let package = astrolabe::rete::Package {
            selection: selection.clone(),
            roots: imported.roots.clone(),
            diagnostics: imported.diagnostics.clone(),
        };
        astrolabe::rete::save_package(&mut imported, &inventory, &package, rete_output)?;
        (direct, package.diagnostics)
    };
    let (mut reopened, package) = astrolabe::rete::Store::open(rete_output)?;
    ensure!(
        package.selection == selection,
        "reopened Rete selection differs"
    );
    let reopened_rete = load_main_actor(&mut reopened, &[47])?;
    ensure!(
        direct.projection == reopened_rete.projection,
        "direct and reopened Rete projections differ before live comparison"
    );
    Ok(StaticRoutes {
        direct,
        reopened_rete,
        source_diagnostics,
    })
}

/// Rete-only comparison has no source-disc argument or fallback. Both this and
/// `prepare_routes` use the same canonical asset resolver and projection.
pub fn load_rete_actor(rete_root: &Path) -> Result<StaticActor> {
    let (mut source, _) = astrolabe::rete::Store::open(rete_root)?;
    load_main_actor(&mut source, &[47])
}

/// Uses explicit Scene.main_actor and its instance relationship, never display
/// names, value equality, or a fabricated source allocation index.
pub fn load_main_actor(
    source: &mut (impl Load<Scene> + Load<Perso> + Load<Family>),
    actions: &[u32],
) -> Result<StaticActor> {
    let scene = source.load(&AssetRef::<Scene>::new("level:/scene")?)?;
    let actor_ref = scene
        .main_actor
        .as_ref()
        .context("scene has no controlled actor role")?;
    let nodes: Vec<_> = scene
        .nodes
        .iter()
        .filter(|n| n.perso.as_ref() == Some(actor_ref))
        .collect();
    ensure!(
        nodes.len() == 1,
        "controlled actor role has missing or ambiguous scene instances"
    );
    let actor = source.load(actor_ref)?;
    let family = source.load(&actor.family)?;
    let transforms = scene.global_transforms()?;
    Ok(StaticActor {
        scene: scene.path.to_string(),
        node: nodes[0].id.clone(),
        actor: actor.path.to_string(),
        family: family.path.to_string(),
        file_global_transform: transforms
            .get(&nodes[0].id)
            .context("main actor transform is missing")?
            .clone(),
        projection: ActorProjection::from_model(&actor, &family, actions)?,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Matched,
    Different,
    Unmatched,
    Unavailable,
    Unsupported,
}
#[derive(Debug, Serialize)]
pub struct Field {
    pub path: String,
    pub policy: &'static str,
    pub required: bool,
    pub outcome: Outcome,
    pub expected: Value,
    pub actual: Value,
    pub reason: String,
}
#[derive(Debug, Serialize)]
pub struct ActorComparison {
    pub schema: u32,
    pub scope: &'static str,
    pub selected_invariants_match: bool,
    pub behavior_parity: bool,
    pub identity: Value,
    pub counts: BTreeMap<Outcome, usize>,
    pub fields: Vec<Field>,
}
fn exact(fields: &mut Vec<Field>, path: impl Into<String>, expected: Value, actual: Value) {
    fields.push(Field {
        path: path.into(),
        policy: "invariant exact semantic value",
        required: true,
        outcome: if expected == actual {
            Outcome::Matched
        } else {
            Outcome::Different
        },
        expected,
        actual,
        reason: String::new(),
    });
}

pub fn compare_main_actor(expected: &StaticActor, observed: &ActorObservation) -> ActorComparison {
    let actual = observed.projection();
    let mut fields = vec![];
    exact(
        &mut fields,
        "actor.role",
        json!(expected.projection.role),
        json!(actual.role),
    );
    exact(
        &mut fields,
        "actor.object_types",
        json!(expected.projection.object_types),
        json!(actual.object_types),
    );
    exact(
        &mut fields,
        "family.object_type",
        json!(expected.projection.family.object_type),
        json!(actual.family.object_type),
    );
    exact(
        &mut fields,
        "family.action_count",
        json!(expected.projection.family.action_count),
        json!(actual.family.action_count),
    );
    for action in &expected.projection.family.selected_actions {
        if let Some(found) = actual
            .family
            .selected_actions
            .iter()
            .find(|a| a.id == action.id)
        {
            for (name, left, right) in [
                ("speed", json!(action.speed), json!(found.speed)),
                ("repeats", json!(action.repeats), json!(found.repeats)),
                (
                    "transition_lock",
                    json!(action.transition_lock),
                    json!(found.transition_lock),
                ),
                ("next", json!(action.next), json!(found.next)),
            ] {
                exact(
                    &mut fields,
                    format!("family.actions[{}].{name}", action.id),
                    left,
                    right,
                );
            }
        } else {
            fields.push(Field {
                path: format!("family.actions[{}]", action.id),
                policy: "selected family member",
                required: true,
                outcome: Outcome::Unmatched,
                expected: json!(action),
                actual: Value::Null,
                reason: "selected action was not captured; absence is not agreement".into(),
            });
        }
    }
    for action in &actual.family.selected_actions {
        if !expected
            .projection
            .family
            .selected_actions
            .iter()
            .any(|a| a.id == action.id)
        {
            fields.push(Field {
                path: format!("family.actions[{}]", action.id),
                policy: "selected family member",
                required: true,
                outcome: Outcome::Unmatched,
                expected: Value::Null,
                actual: json!(action),
                reason: "unexpected captured action in selected profile".into(),
            });
        }
    }
    let mut selected_ids = std::collections::BTreeSet::new();
    for action in &actual.family.selected_actions {
        if !selected_ids.insert(action.id) {
            fields.push(Field {
                path: format!("family.actions[{}]", action.id),
                policy: "unique captured member identity",
                required: true,
                outcome: Outcome::Unmatched,
                expected: json!("one selected member"),
                actual: json!(action),
                reason: "duplicate selected member makes the observation ambiguous".into(),
            });
        }
    }
    fields.push(Field {path:"actor.selections".into(),policy:"save/initialization dependent and mutable",required:false,outcome:Outcome::Unavailable,expected:json!(expected.projection.selections),actual:json!(actual.selections),reason:"file defaults are retained beside runtime values; comparison requires a separately named save/checkpoint expectation".into()});
    fields.push(Field {
        path: "actor.global_transform".into(),
        policy: "mutable checkpoint value",
        required: false,
        outcome: Outcome::Unavailable,
        expected: json!(expected.file_global_transform),
        actual: json!(observed.global_transform),
        reason:
            "arbitrary file placement cannot be an oracle for the loaded save's current position"
                .into(),
    });
    fields.push(Field {path:"actor.runtime_lifetime".into(),policy:"session/load/replacement identity",required:false,outcome:Outcome::Unavailable,expected:Value::Null,actual:Value::Null,reason:"root matching within one capture does not establish a handle surviving removal or address reuse".into()});
    fields.push(Field {path:"family.remaining_definitions".into(),policy:"unselected definition fields",required:false,outcome:Outcome::Unsupported,expected:Value::Null,actual:Value::Null,reason:"this initial slice covers selected action scalars/next relationship; animation, mechanics and other members are not compared".into()});
    let mut counts = BTreeMap::new();
    for field in &fields {
        *counts.entry(field.outcome).or_default() += 1;
    }
    ActorComparison {
        schema: 1,
        scope: "controlled actor and selected invariant action definition",
        selected_invariants_match: fields
            .iter()
            .filter(|f| f.required)
            .all(|f| f.outcome == Outcome::Matched),
        behavior_parity: false,
        identity: json!({"policy":"explicit main-actor role, verified type tuple, resolved family relationship; capture-local only", "static_scene":expected.scene,"static_node":expected.node,"static_actor":expected.actor,"static_family":expected.family,"observed_node":observed.node,"observed_actor":observed.perso,"observed_family":observed.family,"persistent_runtime_handle":false}),
        counts,
        fields,
    }
}
