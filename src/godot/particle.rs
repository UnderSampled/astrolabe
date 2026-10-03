//! Convert useful particle definitions to inspectable native emitter scenes.
use super::{Project, disk_path, quoted, vec3, vector};
use crate::model::{
    AssetPath, AssetRef, content::ContentSource, io::Content, particle::ParticleGenerator,
};
use anyhow::{Context, Result};
use std::{collections::BTreeSet, fmt::Write as _};

pub(super) fn export(
    project: &mut Project,
    source: &mut impl ContentSource,
    value: &ParticleGenerator,
    written: &mut BTreeSet<AssetPath>,
) -> Result<String> {
    value.validate()?;
    let path = disk_path(&project.selection, &value.path, "tscn");
    if !written.insert(value.path.clone()) {
        return Ok(path);
    }
    let sprite = source.load(&value.sprite)?;
    sprite.validate_reference(&value.sprite.path)?;
    sprite.validate()?;
    let view = sprite
        .views
        .first()
        .context("particle sprite has no view")?;
    let reference = view
        .material
        .as_ref()
        .context("particle sprite has no material")?;
    let material = source.load(reference)?;
    material.validate_reference(&reference.path)?;
    if written.insert(material.path.clone()) {
        if let Some(texture) = material.texture() {
            super::export::texture(project, source, texture, written)?;
        }
        project.write_material(&material)?;
    }
    let material_path = disk_path(&project.selection, &reference.path, "tres");
    let mut text = format!(
        "[gd_scene load_steps=4 format=3]\n\n[ext_resource type=\"Script\" path=\"res://addons/astrolabe/character/particle_generator.gd\" id=\"generator\"]\n[ext_resource type=\"Material\" path={} id=\"material\"]\n",
        quoted(&format!("res://{material_path}"))
    );
    // Keep the actual converted sprite material as the quad's material. The
    // native particle renderer supplies billboard orientation when emission
    // is implemented; the current selected/disabled contract renders none.
    writeln!(
        text,
        "\n[sub_resource type=\"QuadMesh\" id=\"sprite\"]\nsize = Vector2({}, {})\ncenter_offset = Vector3({}, {}, 0)\nmaterial = ExtResource(\"material\")\n\n[node name=\"ParticleGenerator\" type=\"CPUParticles3D\"]\nscript = ExtResource(\"generator\")\nemitting = false\namount = {}\nmesh = SubResource(\"sprite\")\nlocal_coords = false\ngenerator_uri = {}\nrelative_environment = {}\nbounds_kind = {}\nbox_min = {}\nbox_max = {}\nsphere_center = {}\nsphere_radius = {}",
        view.half_size[0] * 2.,
        view.half_size[1] * 2.,
        view.displacement[0],
        -view.displacement[1],
        value.capacity,
        quoted(value.path.as_str()),
        value.relative_environment,
        value.environment.bounds_kind,
        vec3(vector(value.environment.box_bounds[0])),
        vec3(vector(value.environment.box_bounds[1])),
        vec3(vector(value.environment.sphere_center)),
        value.environment.sphere_radius
    )?;
    project.diagnostic(format!("{}: native sprite-emitter selection and disabled initialization supported; active emission/random velocity/infinite lifetime/boundary cleanup remain explicit unsupported API work",value.path));
    project.write(path.clone(), text.as_bytes())?;
    Ok(path)
}

pub(super) fn constant(reference: &AssetRef<ParticleGenerator>) -> String {
    format!(
        "particle_generator_{}",
        super::digest(reference.path.as_str().as_bytes())
    )
}
