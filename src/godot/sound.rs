//! Native audio projection of shared sound programs and bank-owned waveforms.
use super::{Project, disk_path};
use crate::model::{AssetPath, AssetRef, audio::*, content::ContentSource, io::Save};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub(super) const PLAYER: &str = "generated/hype/runtime/sound.gd";

#[cfg(test)]
#[path = "sound_tests.rs"]
mod tests;

fn member(project: &Project, reference: &SoundMemberRef) -> Value {
    json!({"file":format!("res://{}",disk_path(&project.selection,&reference.definitions.path,"json")),"id":reference.id})
}
fn identity(reference: &SoundMemberRef) -> String {
    format!("{}#{}", reference.definitions.path, reference.id)
}

pub(super) fn export_event(
    project: &mut Project,
    source: &mut impl ContentSource,
    link: &SoundEventLink,
    written: &mut BTreeSet<AssetPath>,
) -> Result<Value> {
    if !project.generated.contains(PLAYER) {
        project.write(PLAYER.into(), include_bytes!("scripts/sound.gd"))?;
        project.diagnostic("Sound programs use native sample/loop, weighted choice and selector playback. Godot attenuation, volume response, random generator and reverb are approximations; automatic material sensing and live switch updates are pending.");
    }
    let library = source.load(&link.library)?;
    library.validate()?;
    let SoundDefinitionTarget::Event(reference) = &link.binding(&library)?.target else {
        unreachable!()
    };
    let definitions = source.load(&reference.definitions)?;
    definitions.validate()?;
    let event = definitions.event(reference.id)?;
    let mut result = json!({"bank":link.bank,"member":link.member,"identity":identity(reference),
        "definition":format!("{}^SndEventM:{}",reference.definitions.path.file().rsplit('/').next().unwrap(),reference.id),"name":event.name});
    match &event.operation {
        SoundOperation::Play {
            resource,
            pitch_variation,
            stoppable,
            dynamic,
        } => {
            if let Some(r) = resource {
                export_resource(project, source, r, written, &mut BTreeSet::new())?;
            }
            result["operation"] = json!("play");
            result["resource"] = json!(resource.as_ref().map(|r| member(project, r)));
            result["pitch_variation"] = json!(pitch_variation);
            result["stoppable"] = json!(stoppable);
            result["dynamic"] = json!(dynamic);
        }
        SoundOperation::Stop { event } => {
            if let Some(r) = event {
                source.load(&r.definitions)?.event(r.id)?;
            }
            result["operation"] = json!("stop");
            result["target"] = json!(event.as_ref().map(identity));
        }
        SoundOperation::Other { name } => {
            project.diagnostic(format!(
                "{}: sound operation {name} is not executed",
                identity(reference)
            ));
            result["operation"] = json!("unsupported");
        }
    }
    Ok(result)
}

fn export_resource(
    project: &mut Project,
    source: &mut impl ContentSource,
    reference: &SoundMemberRef,
    written: &mut BTreeSet<AssetPath>,
    active: &mut BTreeSet<(AssetPath, u32)>,
) -> Result<()> {
    reference.validate()?;
    let key = (reference.definitions.path.clone(), reference.id);
    ensure!(
        active.insert(key.clone()) && active.len() <= 64,
        "cyclic or excessive sound program nesting"
    );
    let definitions = source.load(&reference.definitions)?;
    definitions.validate()?;
    let resource = definitions.resource(reference.id)?;
    if written.insert(reference.definitions.path.clone()) {
        // One target program file per useful definitions asset, with whole-owner
        // member lookup. Only source-bound samples need waveform projection.
        let mut resources = serde_json::Map::new();
        for r in &definitions.resources {
            let data = match &r.data {
                SoundResourceData::Sample { sample, controls } => {
                    json!({"kind":"sample","stream":sample.as_ref().map(|s|export_sample(project,source,s,written)).transpose()?,"controls":controls})
                }
                SoundResourceData::Random {
                    silence_probability,
                    repeat_silence,
                    choices,
                } => {
                    json!({"kind":"random","silence_probability":silence_probability,"repeat_silence":repeat_silence,
                    "choices":choices.iter().map(|c|json!({"resource":member(project,&c.resource),"probability":c.probability,"repeat":c.repeat})).collect::<Vec<_>>()})
                }
                SoundResourceData::Switch {
                    selector,
                    default,
                    dynamic,
                    maintain,
                    choices,
                } => {
                    json!({"kind":"switch","selector":format!("{}#{}",selector.definitions.path,selector.name),"default":default,"dynamic":dynamic,"maintain":maintain,
                    "choices":choices.iter().map(|c|json!({"resource":member(project,&c.resource),"value":c.value})).collect::<Vec<_>>()})
                }
                SoundResourceData::Other { kind, .. } => {
                    json!({"kind":"unsupported","source_kind":kind})
                }
            };
            resources.insert(r.id.to_string(),json!({"name":r.name,"volume":r.volume,"looped":r.looped,"loop_region":r.loop_region,"data":data}));
        }
        project.write(
            disk_path(&project.selection, &reference.definitions.path, "json"),
            &serde_json::to_vec(&json!({"resources":resources}))?,
        )?;
    }
    match &resource.data {
        SoundResourceData::Sample { sample, .. } => {
            ensure!(sample.is_some(), "unbound sample {}", identity(reference))
        }
        SoundResourceData::Other { kind, .. } => project.diagnostic(format!(
            "{}: sound resource {kind} is not executed",
            identity(reference)
        )),
        _ => {
            for child in resource.data.members() {
                export_resource(project, source, child, written, active)?;
            }
        }
    }
    active.remove(&key);
    Ok(())
}

pub(super) fn export_sample(
    project: &mut Project,
    source: &mut impl ContentSource,
    sample: &SoundSampleRef,
    written: &mut BTreeSet<AssetPath>,
) -> Result<String> {
    let reference = match sample {
        SoundSampleRef::Sound(reference) => {
            if written.insert(reference.path.clone()) {
                project.save(reference, source.load(reference)?.as_ref())?;
            }
            reference.clone()
        }
        SoundSampleRef::Bank { bank, resource_id } => {
            let reference =
                AssetRef::<Sound>::new(format!("{}/resource_{resource_id}", bank.path))?;
            if written.insert(reference.path.clone()) {
                let bank = source.load(bank)?;
                let value = bank
                    .sounds
                    .iter()
                    .find(|s| s.resource_id == Some(*resource_id))
                    .context("missing bank-owned sample")?;
                project.save(&reference, value)?;
            }
            reference
        }
    };
    Ok(format!(
        "res://{}",
        disk_path(&project.selection, &reference.path, "tres")
    ))
}
