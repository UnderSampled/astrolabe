//! Godot-specific sampled character projection and temporary runtime support.
use super::{Project, disk_path, quoted, vector};
use crate::model::{
    AssetPath, AssetRef, animation::*, character::*, content::ContentSource, io::Content,
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fmt::Write as _};
const PLAYER: &str = "generated/hype/runtime/character.gd";
fn resource(project: &Project, path: &AssetPath) -> String {
    format!("res://{}", disk_path(&project.selection, path, "json"))
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
    if !project.generated.contains(PLAYER) {
        project.write(PLAYER.into(), include_bytes!("scripts/character.gd"))?;
    }
    project.diagnostic(format!("{}: sampled animation defaults exported; gameplay channel-visibility overrides, lip sync, root motion and AI state transitions require runtime support", perso.path));
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
    for reference in family
        .configurations
        .iter()
        .chain(family.default_configuration.iter())
        .chain(perso.configuration.iter())
        .chain(perso.initial_configuration.iter())
    {
        if !written.insert(reference.path.clone()) {
            continue;
        }
        let config = source.load(reference)?;
        config.validate_reference(&reference.path)?;
        config.validate()?;
        let mut tables = serde_json::Map::new();
        for (name, values) in [
            ("parts", Some(&config.parts)),
            ("reset_parts", config.reset_parts.as_ref()),
        ] {
            let Some(values) = values else {
                tables.insert(name.into(), Value::Null);
                continue;
            };
            let mut parts = Vec::new();
            for part in values {
                let scene = if let Some(reference) = &part.instance {
                    let instance = source.load(reference)?;
                    instance.validate_reference(&reference.path)?;
                    if written.insert(instance.path.clone()) {
                        let model = source.load(&instance.model)?;
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
                if let Some(reference) = &part.action {
                    export_action(project, source, reference, written)?;
                }
                parts.push(json!({"scene":scene,"scale":part.scale.map(|[x,y,z]|[x,z,y]),"action":part.action.as_ref().map(|r|resource(project,&r.path))}));
            }
            tables.insert(name.into(), json!(parts));
        }
        write_json(project, &config.path, &Value::Object(tables))?;
    }
    if written.insert(family.path.clone()) {
        let bounds = if let Some(reference) = &family.bounding_geometry {
            if written.insert(reference.path.clone()) {
                let geometry = source.load(reference)?;
                geometry.validate_reference(&reference.path)?;
                let geometry = geometry.resolve(source)?;
                let mut shapes = Vec::new();
                for element in &geometry.elements {
                    match element {
                        crate::model::collision::CollisionElement::Spheres(spheres) => {
                            for sphere in spheres {
                                if sphere.radius == 0. {
                                    continue;
                                }
                                shapes.push(json!({"kind":"sphere","center":vector(geometry.positions[sphere.center as usize]),"radius":sphere.radius}));
                            }
                        }
                        crate::model::collision::CollisionElement::Boxes(boxes) => {
                            for bounds in boxes {
                                let a = vector(geometry.positions[bounds.min as usize]);
                                let b = vector(geometry.positions[bounds.max as usize]);
                                let center =
                                    std::array::from_fn::<_, 3, _>(|i| (a[i] + b[i]) * 0.5);
                                let size = std::array::from_fn::<_, 3, _>(|i| (b[i] - a[i]).abs());
                                shapes.push(json!({"kind":"box","center":center,"size":size}));
                            }
                        }
                        _ => project.diagnostic(format!(
                            "{}: triangle bounding geometry has no debug primitive projection",
                            reference.path
                        )),
                    }
                }
                write_json(project, &reference.path, &json!({"shapes":shapes}))?;
            }
            Some(resource(project, &reference.path))
        } else {
            None
        };
        let mut states = Vec::new();
        for state in &family.states {
            let mechanics = if let Some(reference) = &state.mechanics {
                if written.insert(reference.path.clone()) {
                    let profile = source.load(reference)?;
                    profile.validate_reference(&reference.path)?;
                    write_json(
                        project,
                        &reference.path,
                        &serde_json::to_value(profile.as_ref())?,
                    )?;
                    project.diagnostic(format!("{}: mechanics parameters retained for state inspection; original dynamics simulation is not implemented",reference.path));
                }
                Some(resource(project, &reference.path))
            } else {
                None
            };
            let animation = if let Some(reference) = &state.animation {
                if written.insert(reference.path.clone()) {
                    let clip = source.load(reference)?;
                    clip.validate_reference(&reference.path)?;
                    export_clip(project, source, &clip, written)?;
                }
                Some(resource(project, &reference.path))
            } else {
                None
            };
            states.push(json!({"id":state.id,"animation":animation,"mechanics":mechanics,"speed":state.speed,"next":state.next,"transitions":state.transitions,"prohibited":state.prohibited}));
        }
        write_json(
            project,
            &family.path,
            &json!({"name":family.name,"object_type":family.object_type,"bounds":bounds,"states":states}),
        )?;
    }
    let config = perso
        .configuration
        .as_ref()
        .or(family.default_configuration.as_ref());
    write_json(
        project,
        &perso.path,
        &json!({"name":perso.name,"family":resource(project,&family.path),"configuration":config.map(|r|resource(project,&r.path)),"state":perso.current_state.or(perso.initial_state)}),
    )?;
    Ok(())
}
fn export_clip(
    project: &mut Project,
    source: &mut impl ContentSource,
    clip: &AnimationClip,
    written: &mut BTreeSet<AssetPath>,
) -> Result<()> {
    clip.validate()?;
    if clip
        .samples
        .iter()
        .any(|s| matches!(s.kind, ChannelKind::Light | ChannelKind::Unknown(_)))
    {
        project.diagnostic(format!("{}: light/reserved animation channels retain identity and transforms but their payloads are not rendered", clip.path));
    }
    let mut cues = Vec::new();
    for cue in &clip.cues {
        export_action(project, source, &cue.action, written)?;
        cues.push(json!({"action":resource(project, &cue.action.path),"frame":cue.frame,"channel":cue.channel,"configuration_slot":cue.configuration_slot}));
    }
    let mut samples = Vec::new();
    for sample in &clip.samples {
        let transform = if let Some(link) = &sample.transform {
            let pool = source.load(&link.pool)?;
            pool.validate_reference(&link.pool.path)?;
            ensure!(
                pool.entries
                    .binary_search_by_key(&link.entry, |e| e.id)
                    .is_ok(),
                "missing animation transform entry"
            );
            if written.insert(pool.path.clone()) {
                pool.validate()?;
                let mut entries = serde_json::Map::new();
                for entry in &pool.entries {
                    let t = entry.value.affine();
                    let basis = [
                        vector(t.basis[0]),
                        vector(t.basis[2]),
                        vector(t.basis[1]).map(|v| -v),
                    ];
                    let values: Vec<_> = basis
                        .into_iter()
                        .flatten()
                        .chain(vector(t.translation))
                        .collect();
                    entries.insert(entry.id.to_string(), json!(values));
                }
                write_json(project, &pool.path, &json!({"entries":entries}))?;
            }
            Some(json!({"pool":resource(project,&link.pool.path),"entry":link.entry}))
        } else {
            None
        };
        let object = if sample.kind == ChannelKind::Object {
            sample.object
        } else {
            None
        };
        samples.push(json!({"transform":transform,"object":object,"channel":sample.channel,"opacity":sample.opacity,"visibility":sample.visibility}));
    }
    write_json(
        project,
        &clip.path,
        &json!({"channel_count":clip.channel_count,"samples":samples,"frames":clip.frames,"speed":clip.speed,"frame_rate":clip.frame_rate,"cues":cues}),
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
    let sound = if let AnimationOperation::Sound { event: Some(event) } = &action.operation {
        Some(super::sound::export_event(project, source, event, written)?)
    } else {
        if !matches!(action.operation, AnimationOperation::Sound { .. }) {
            project.diagnostic(format!("{}: timed action signals exported; attachment/mechanics/camera behavior is not yet executed", action.path));
        }
        None
    };
    write_json(
        project,
        &action.path,
        &json!({"path":action.path,"operation":action.operation,"unknown":action.unknown,"sound":sound}),
    )
}
fn write_json(project: &mut Project, path: &AssetPath, value: &Value) -> Result<()> {
    project.write(
        disk_path(&project.selection, path, "json"),
        &serde_json::to_vec(value)?,
    )
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
        ensure!(
            self.generated.contains(PLAYER),
            "character dependencies were not exported"
        );
        let definition = resource(self, &perso.path);
        let path = definition
            .strip_prefix("res://")
            .context("invalid character resource path")?;
        ensure!(
            self.generated.contains(path),
            "missing character definition"
        );
        writeln!(
            self.builder,
            "    var {parent}_character = Node3D.new()\n    {parent}_character.name = \"Character\"\n    {parent}_character.set_script(load({}))\n    {parent}_character.definition_path = {}\n    {parent}.add_child({parent}_character)\n    {parent}_character.owner = {owner}\n    {parent}_character.set_meta(\"source_perso_name\", {})",
            quoted(&format!("res://{PLAYER}")),
            quoted(&definition),
            quoted(&perso.name)
        )?;
        Ok(())
    }
}
