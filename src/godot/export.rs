//! Traverse useful roots and export their target resources through one resolver.
use crate::model::{
    AssetPath, AssetRef, Selection,
    audio::Sound,
    content::{Asset, ContentSource},
    geometry::{Model, ModelInstance},
    io::{Content, ContentRoot, Load, Save},
    texture::Texture,
};
use anyhow::Result;
use std::{collections::BTreeSet, path::Path};
/// Both in-memory source sessions and reloaded Rete use this same composition.
pub fn export<R: ContentSource>(
    source: &mut R,
    roots: &[ContentRoot],
    selection: Selection,
    output: &Path,
    engine: &str,
) -> Result<super::Manifest> {
    let mut project = super::Project::new(output, selection)?;
    let mut written = BTreeSet::<AssetPath>::new();
    for root in roots {
        match Asset::load(source, root)? {
            Asset::Model(model) => {
                let instance = ModelInstance::new(model.path.clone(), &model);
                export_instance(&mut project, source, &model, &instance, &mut written)?;
            }
            Asset::ModelInstance(instance) => {
                let model = source.load(&instance.model)?;
                export_instance(&mut project, source, &model, &instance, &mut written)?;
            }
            Asset::Material(material) => {
                material.validate_reference(&root.reference)?;
                if written.insert(material.path.clone()) {
                    if let Some(id) = &material.texture {
                        texture(&mut project, source, id, &mut written)?;
                    }
                    project.write_material(&material)?;
                }
            }
            Asset::Texture(value) => {
                if written.insert(root.reference.clone()) {
                    project.save(&AssetRef::from_path(root.reference.clone()), value.as_ref())?;
                }
            }
            Asset::TextureSet(set) => {
                for entry in &set.entries {
                    texture(&mut project, source, &entry.texture.path, &mut written)?;
                }
            }
            Asset::Sound(value) => {
                let reference = AssetRef::<Sound>::from_path(root.reference.clone());
                project.save(&reference, value.as_ref())?;
            }
            Asset::SoundBank(bank) => {
                for (i, sound) in bank.sounds.iter().enumerate() {
                    let id = sound
                        .resource_id
                        .map_or_else(|| format!("sample_{i}"), |id| format!("resource_{id}"));
                    let reference = AssetRef::<Sound>::new(format!("{}/{}", root.reference, id))?;
                    project.save(&reference, sound)?;
                }
                if !bank.external_sounds.is_empty() {
                    project.diagnostic(format!(
                        "{}: {} external sound references require separately selected APM assets",
                        root.reference,
                        bank.external_sounds.len()
                    ));
                }
                for note in &bank.fidelity_notes {
                    project.diagnostic(format!("{}: {note}", root.reference));
                }
            }
            Asset::SoundDefinitions(value) => {
                project.diagnostic(format!(
                    "{}: {} event definitions retained; gameplay routing is milestone 6",
                    root.reference,
                    value.events.len()
                ));
            }
        }
    }
    project.finish(engine)
}

fn texture<R: Load<Texture>>(
    project: &mut super::Project,
    source: &mut R,
    id: &AssetPath,
    written: &mut BTreeSet<AssetPath>,
) -> Result<()> {
    if written.insert(id.clone()) {
        let reference = AssetRef::<Texture>::from_path(id.clone());
        let value = source.load(&reference)?;
        project.save(&reference, value.as_ref())?;
    }
    Ok(())
}

fn export_instance(
    project: &mut super::Project,
    source: &mut impl ContentSource,
    model: &Model,
    instance: &ModelInstance,
    written: &mut BTreeSet<AssetPath>,
) -> Result<()> {
    instance.validate_model(model)?;
    for part in &model.submeshes {
        let material = source.load(&part.material)?;
        material.validate_reference(&part.material.path)?;
        if written.insert(material.path.clone()) {
            if let Some(id) = &material.texture {
                texture(project, source, id, written)?;
            }
            project.write_material(&material)?;
        }
    }
    project.write_instance(model, instance, source)
}
