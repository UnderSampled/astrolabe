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
    // Export localized native dialogue resources before saving the scene composition.
    for root in roots
        .iter()
        .filter(|r| r.kind == crate::model::dialogue::Dialogue::KIND)
    {
        if let Asset::Dialogue(value) = Asset::load(source, root)? {
            super::dialogue::export(&mut project, source, &value, &mut written)?;
        }
    }
    for root in roots {
        match Asset::load(source, root)? {
            Asset::ArmorCatalog(value) => {
                super::armor::export(&mut project, source, &value)?;
            }
            Asset::InventoryCatalog(value) => {
                if written.insert(value.path.clone()) {
                    super::inventory_gameplay::export(&mut project, source, &value)?;
                }
            }
            Asset::ParticleGenerator(value) => {
                super::particle::export(&mut project, source, &value, &mut written)?;
            }
            Asset::Environment(_)
            | Asset::PhysicalEnvironment(_)
            | Asset::VisualEnvironment(_)
            | Asset::Fog(_)
            | Asset::Light(_)
            | Asset::Mechanics(_) => {
                anyhow::bail!(
                    "environment/light/mechanics export requires a scene for placement or state context"
                )
            }
            Asset::MenuPresentation(value) => super::menu::export(&mut project, source, &value)?,
            Asset::InventoryPresentation(value) => {
                super::inventory::export(&mut project, source, &value)?
            }
            Asset::HudPresentation(value) => {
                super::hud::export(&mut project, source, &value, &mut written)?
            }
            Asset::Dialogue(_) => {}
            Asset::DialoguePresentation(value) => {
                super::dialogue_presentation::export(&mut project, source, &value, &mut written)?;
            }
            Asset::Sprite(_) => anyhow::bail!("sprite export requires its owning model"),
            Asset::AiModel(model) => {
                super::ai::export_program(&mut project, source, &model, &mut written)?;
            }
            Asset::AnimationAction(_)
            | Asset::Animation(_)
            | Asset::TransformPool(_)
            | Asset::Family(_)
            | Asset::ObjectList(_)
            | Asset::CollisionProfile(_)
            | Asset::Perso(_) => {
                anyhow::bail!("character content export requires its owning scene")
            }
            Asset::Route(route) => {
                super::route::export_route(&mut project, source, &route)?;
            }
            Asset::Waypoint(waypoint) => {
                super::route::export_waypoint(&mut project, &waypoint)?;
            }
            Asset::Collision(_) => {
                anyhow::bail!("collision export requires its owning scene")
            }
            Asset::Scene(scene) => {
                if let Some(reference) = &scene.inventory {
                    let inventory = source.load(reference)?;
                    inventory.validate_reference(&reference.path)?;
                    if written.insert(reference.path.clone()) {
                        super::inventory_gameplay::export(&mut project, source, &inventory)?;
                    }
                }
                if let Some(reference) = &scene.armor {
                    let armor = source.load(reference)?;
                    armor.validate_reference(&reference.path)?;
                    super::armor::export(&mut project, source, &armor)?;
                }
                if let Some(reference) = &scene.hud {
                    let hud = source.load(reference)?;
                    hud.validate_reference(&reference.path)?;
                    super::hud::export(&mut project, source, &hud, &mut written)?;
                }
                let preview_start = project.preview_instances.len();
                for node in &scene.nodes {
                    if let Some(reference) = &node.perso {
                        let perso = source.load(reference)?;
                        perso.validate_reference(&reference.path)?;
                        super::character::export_perso(&mut project, source, &perso, &mut written)?;
                    }
                    if let Some(reference) = &node.instance {
                        let instance = source.load(reference)?;
                        instance.validate_reference(&reference.path)?;
                        anyhow::ensure!(
                            instance.transforms.is_empty(),
                            "scene-owned instance must use node placement"
                        );
                        let model = source.load(&instance.model)?;
                        export_instance(&mut project, source, &model, &instance, &mut written)?;
                    }
                }
                project.preview_instances.truncate(preview_start);
                project.write_scene(&scene, source)?;
            }
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
                    if let Some(id) = material.texture() {
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

pub(super) fn texture<R: Load<Texture>>(
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

pub(super) fn export_instance(
    project: &mut super::Project,
    source: &mut impl ContentSource,
    model: &Model,
    instance: &ModelInstance,
    written: &mut BTreeSet<AssetPath>,
) -> Result<()> {
    instance.validate_model(model)?;
    for element in &model.sprite_elements {
        for placement in &element.sprites {
            let sprite = source.load(&placement.sprite)?;
            sprite.validate_reference(&placement.sprite.path)?;
            for view in &sprite.views {
                if let Some(reference) = &view.material {
                    let material = source.load(reference)?;
                    material.validate_reference(&reference.path)?;
                    if written.insert(material.path.clone()) {
                        if let Some(id) = material.texture() {
                            texture(project, source, id, written)?;
                        }
                        project.write_material(&material)?;
                    }
                }
            }
        }
    }
    for part in &model.submeshes {
        let material = source.load(&part.material)?;
        material.validate_reference(&part.material.path)?;
        if written.insert(material.path.clone()) {
            if let Some(id) = material.texture() {
                texture(project, source, id, written)?;
            }
            project.write_material(&material)?;
        }
    }
    project.write_instance(model, instance, source)
}
