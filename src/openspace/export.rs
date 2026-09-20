//! Reconstruct supported source files from useful roots and their dependencies.
use super::codec::FormatCodec;
use crate::files::write_atomic;
use crate::model::{
    AssetPath, AssetRef,
    content::{Asset, ContentSource},
    geometry::{Model, ModelInstance},
    io::{Content, ContentRoot, Diagnostic},
    texture::Texture,
};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

const REPORT_PATH: &str = "reconstruction-report.json";
/// Explicit coverage accompanies partial reconstruction; a fragment is not a level.
#[derive(Debug, Serialize)]
pub struct ReconstructionReport {
    pub files: Vec<String>,
    pub coverage_and_gaps: Vec<String>,
    pub source_diagnostics: Vec<Diagnostic>,
}
impl ReconstructionReport {
    pub fn write(&self, output: &Path) -> Result<()> {
        write_atomic(&output.join(REPORT_PATH), &serde_json::to_vec_pretty(self)?)
    }
}

/// Track ownership across all roots and dependencies, after target naming.
/// Shared references emit once; distinct assets must never replace each other.
struct ExportFiles<'a> {
    output: &'a Path,
    owners: BTreeMap<String, AssetPath>,
    model_targets: BTreeMap<AssetPath, super::level::ReferenceTarget>,
    collision_targets: BTreeMap<AssetPath, super::level::ReferenceTarget>,
    collision_profiles: BTreeMap<AssetPath, super::level::CollisionProfileTargets>,
    next_collision_profile_base: u32,
    next_geometry_base: u32,
}
impl ExportFiles<'_> {
    fn contains(&self, owner: &AssetPath, name: &str) -> Result<bool> {
        ensure!(
            name != REPORT_PATH,
            "asset {owner} collides with the reconstruction report"
        );
        if let Some(previous) = self.owners.get(name) {
            ensure!(
                previous == owner,
                "OpenSpace output collision at {name}: {previous} and {owner}"
            );
            return Ok(true);
        }
        Ok(false)
    }
    fn write(&mut self, owner: &AssetPath, name: String, bytes: &[u8]) -> Result<()> {
        if self.contains(owner, &name)? {
            return Ok(());
        }
        write_atomic(&self.output.join(&name), bytes)?;
        self.owners.insert(name, owner.clone());
        Ok(())
    }
}

pub fn export<R: ContentSource>(
    source: &mut R,
    roots: &[ContentRoot],
    output: &Path,
) -> Result<ReconstructionReport> {
    let mut files = ExportFiles {
        output,
        owners: BTreeMap::new(),
        model_targets: BTreeMap::new(),
        collision_targets: BTreeMap::new(),
        collision_profiles: BTreeMap::new(),
        next_collision_profile_base: 0x60000000,
        next_geometry_base: 0x10000000,
    };
    let mut gaps = Vec::new();
    for root in roots {
        match Asset::load(source, root)? {
            Asset::CollisionProfile(value) => {
                export_collision_profile(source, &value, &mut files, &mut gaps)?;
            }
            Asset::Mechanics(value) => {
                files.write(
                    &value.path,
                    fragment_path(&value.path, "bin"),
                    &super::level::write_mechanics(&value)?,
                )?;
            }
            Asset::Environment(_) | Asset::VisualEnvironment(_) => {
                anyhow::bail!("environment export requires its scene dependency layout")
            }
            Asset::Fog(value) => {
                files.write(
                    &value.path,
                    fragment_path(&value.path, "bin"),
                    &super::level::write_fog(&value)?,
                )?;
            }
            Asset::Light(light) => {
                files.write(
                    &light.path,
                    fragment_path(&light.path, "bin"),
                    &super::level::write_light(&light)?,
                )?;
            }
            Asset::Sprite(_) => anyhow::bail!("sprite export requires its owning model"),
            Asset::AnimationAction(_)
            | Asset::AiModel(_)
            | Asset::Animation(_)
            | Asset::TransformPool(_)
            | Asset::Family(_)
            | Asset::ObjectList(_)
            | Asset::Perso(_) => {
                anyhow::bail!("character content export requires its owning scene")
            }
            Asset::Collision(_) => {
                anyhow::bail!("collision export requires its owning scene")
            }
            Asset::Scene(scene) => {
                export_characters(source, &scene, &mut files, &mut gaps)?;
                gaps.push("AI script codec reconstruction is available separately; AI models, Brain/designer memory, sound operand placement and the GPT main-actor binding are not yet assembled by scene CLI export.".into());
                for reference in scene
                    .nodes
                    .iter()
                    .flat_map(|n| &n.sector)
                    .flat_map(|s| &s.environments)
                {
                    let env = source.load(reference)?;
                    env.validate_reference(&reference.path)?;
                    if let Some(reference) = &env.visual {
                        let visual = source.load(reference)?;
                        visual.validate_reference(&reference.path)?;
                        if let Some(reference) = &visual.fog {
                            let fog = source.load(reference)?;
                            fog.validate_reference(&reference.path)?;
                            files.write(
                                &fog.path,
                                fragment_path(&fog.path, "bin"),
                                &super::level::write_fog(&fog)?,
                            )?;
                        }
                    }
                }
                if scene
                    .nodes
                    .iter()
                    .flat_map(|n| &n.sector)
                    .any(|s| !s.environments.is_empty())
                {
                    gaps.push("Fog records emitted from shared environments; the controlled ENV/VET/sector writer requires resolved component layouts and is not yet composed into whole-scene CLI reconstruction.".into());
                }
                for node in &scene.nodes {
                    for reference in node.light.iter().chain(
                        node.sector
                            .iter()
                            .flat_map(|s| s.static_lights.iter().chain(&s.dynamic_lights)),
                    ) {
                        let light = source.load(reference)?;
                        light.validate_reference(&reference.path)?;
                        files.write(
                            &light.path,
                            fragment_path(&light.path, "bin"),
                            &super::level::write_light(&light)?,
                        )?;
                    }
                }
                for node in &scene.nodes {
                    for reference in node
                        .collision
                        .iter()
                        .chain(&node.properties.bounding_geometry)
                    {
                        let collision = source.load(reference)?;
                        collision.validate_reference(&reference.path)?;
                        export_collision(source, &collision, &mut files, &mut gaps)?;
                    }
                    if let Some(reference) = &node.instance {
                        let instance = source.load(reference)?;
                        instance.validate_reference(&reference.path)?;
                        let model = source.load(&instance.model)?;
                        export_instance(source, &model, &instance, &mut files, &mut gaps)?;
                    }
                }
                gaps.push(
                    "Supported model, collision/picking-bound, light, family/configuration, mechanics and animation fragments emitted. Controlled SO hierarchy reconstruction requires component layouts and is not yet composed into the scene CLI route."
                        .into(),
                );
            }
            Asset::TextureSet(set) => {
                let bytes = super::texture::write_cnt(&set, source)?;
                let name = format!("Gamedata/{}", set.name);
                files.write(&root.reference, name, &bytes)?;
                gaps.push("CNT uses derived unencrypted output; original XOR obfuscation is not reproduced".to_string());
            }
            Asset::Texture(value) => {
                let name = texture_path(root.reference.file());
                files.write(&root.reference, name, &super::texture::encode_gf(&value)?)?;
            }
            Asset::Sound(value) => {
                let name = root.reference.file().to_owned();
                files.write(&root.reference, name, &super::audio::write_apm(&value)?)?;
                gaps.push("APM waveform re-encoding is supported; original lossy ADPCM byte identity is not claimed".into());
            }
            Asset::SoundBank(bank) => match super::audio::write_bnm(&bank) {
                Ok(bytes) => {
                    let name = root.reference.file().to_owned();
                    files.write(&root.reference, name, &bytes)?;
                }
                Err(error) => gaps.push(format!("{}: {error:#}", root.reference)),
            },
            Asset::SoundDefinitions(value) => {
                match (super::codec::SoundScript {
                    name: value.name.clone(),
                })
                .write(&value)
                {
                    Ok(bytes) => {
                        let name = root.reference.file().to_owned();
                        files.write(&root.reference, name, &bytes)?;
                    }
                    Err(error) => gaps.push(format!("{}: {error:#}", root.reference)),
                }
            }
            Asset::Model(model) => {
                let instance = ModelInstance::new(model.path.clone(), &model);
                export_instance(source, &model, &instance, &mut files, &mut gaps)?;
            }
            Asset::ModelInstance(instance) => {
                let model = source.load(&instance.model)?;
                export_instance(source, &model, &instance, &mut files, &mut gaps)?;
            }
            Asset::Material(_) => anyhow::bail!(
                "standalone OpenSpace material reconstruction requires a model fragment"
            ),
        }
    }
    gaps.sort();
    gaps.dedup();
    let report = ReconstructionReport {
        files: files.owners.into_keys().collect(),
        coverage_and_gaps: gaps,
        source_diagnostics: Vec::new(),
    };
    report.write(output)?;
    Ok(report)
}

fn export_instance(
    source: &mut impl ContentSource,
    model: &Model,
    instance: &ModelInstance,
    files: &mut ExportFiles<'_>,
    gaps: &mut Vec<String>,
) -> Result<super::level::ReferenceTarget> {
    instance.validate_model(model)?;
    if files.contains(&instance.path, &fragment_path(&instance.path, "bin"))? {
        return files
            .model_targets
            .get(&instance.path)
            .copied()
            .context("model fragment layout missing");
    }
    let mut materials = BTreeMap::new();
    let mut sprites = BTreeMap::new();
    for element in &model.sprite_elements {
        for placement in &element.sprites {
            let sprite = source.load(&placement.sprite)?;
            sprite.validate_reference(&placement.sprite.path)?;
            for view in &sprite.views {
                if let Some(reference) = &view.material {
                    let material = source.load(reference)?;
                    material.validate_reference(&reference.path)?;
                    materials.insert(reference.path.clone(), (*material).clone());
                }
            }
            sprites.insert(sprite.path.clone(), (*sprite).clone());
        }
    }
    for reference in model
        .submeshes
        .iter()
        .map(|s| &s.material)
        .chain(model.vertex_materials.iter().flatten())
    {
        let material = source.load(reference)?;
        material.validate_reference(&reference.path)?;
        materials.insert(reference.path.clone(), (*material).clone());
    }
    for material in materials.values() {
        if let Some(id) = material.texture() {
            let name = texture_path(id.file());
            if files.contains(id, &name)? {
                continue;
            }
            let texture = source.load(&AssetRef::<Texture>::from_path(id.clone()))?;
            files.write(
                id,
                name,
                &super::codec::Gf {
                    usage: texture.usage,
                }
                .write(&texture)?,
            )?;
        }
    }
    let base = files.next_geometry_base;
    let fragment = super::level::write_model_fragment(
        model,
        instance,
        &materials.into_values().collect::<Vec<_>>(),
        &sprites.into_values().collect::<Vec<_>>(),
        base,
    )?;
    files.next_geometry_base = base
        .checked_add(((fragment.bytes.len() + 3) & !3).try_into()?)
        .context("model fragment address overflow")?;
    ensure!(
        files.next_geometry_base < 0x20000000,
        "model fragments exceed controlled address range"
    );
    let target = super::level::ReferenceTarget {
        address: base
            .checked_add(fragment.physical_object_offset.try_into()?)
            .context("physical object address overflow")?,
        block: (6, 2),
    };
    let framed = super::level::write_sna_block(&fragment.bytes, (6, 2), base)?;
    for (extension, bytes) in [
        ("sna", framed),
        ("bin", fragment.bytes),
        ("rtb", fragment.rtb),
        ("ptx", fragment.ptx),
        ("rtt", fragment.rtt),
    ] {
        let name = fragment_path(&instance.path, extension);
        files.write(&instance.path, name, &bytes)?;
    }
    gaps.extend(fragment.coverage);
    gaps.push("Model fragment contains generated records/relocations; it is not a complete loadable SNA level".into());
    files.model_targets.insert(instance.path.clone(), target);
    Ok(target)
}

fn texture_path(file: &str) -> String {
    let name = format!("textures/{file}");
    if file.to_ascii_lowercase().ends_with(".gf") {
        name
    } else {
        format!("{name}.gf")
    }
}

fn fragment_path(path: &AssetPath, extension: &str) -> String {
    format!(
        "fragments/{}.{}",
        path.as_str().replace(":/", "/"),
        extension
    )
}
fn export_collision_profile(
    source: &mut impl ContentSource,
    value: &crate::model::character::CollisionProfile,
    files: &mut ExportFiles<'_>,
    gaps: &mut Vec<String>,
) -> Result<()> {
    if files.collision_profiles.contains_key(&value.path) {
        return Ok(());
    }
    for reference in value.geometry.iter().flat_map(|g| &g.slots).flatten() {
        let geometry = source.load(reference)?;
        geometry.validate_reference(&reference.path)?;
        export_collision(source, &geometry, files, gaps)?;
    }
    let output = super::level::write_collision_profile(
        value,
        &files.collision_targets,
        files.next_collision_profile_base,
    )?;
    files.next_collision_profile_base = files
        .next_collision_profile_base
        .checked_add(
            u32::try_from(output.fragment.bytes.len())?
                .checked_add(3)
                .context("profile size overflow")?
                & !3,
        )
        .context("profile address overflow")?;
    ensure!(
        files.next_collision_profile_base < 0x70000000,
        "collision profiles exceed controlled address range"
    );
    files.write(
        &value.path,
        fragment_path(&value.path, "bin"),
        &output.fragment.bytes,
    )?;
    files.write(
        &value.path,
        fragment_path(&value.path, "rtb"),
        &output.fragment.rtb,
    )?;
    files
        .collision_profiles
        .insert(value.path.clone(), output.targets);
    gaps.extend(output.fragment.coverage);
    Ok(())
}
fn export_characters(
    source: &mut impl ContentSource,
    scene: &crate::model::scene::Scene,
    files: &mut ExportFiles<'_>,
    gaps: &mut Vec<String>,
) -> Result<()> {
    let mut family_values = BTreeMap::new();
    let mut mechanics_paths = BTreeSet::new();
    let mut clips = BTreeSet::new();
    let mut configurations = BTreeSet::new();
    let mut action_paths = BTreeSet::new();
    for reference in scene.nodes.iter().filter_map(|n| n.perso.as_ref()) {
        let perso = source.load(reference)?;
        perso.validate_reference(&reference.path)?;
        if let Some(collisions) = &perso.collisions {
            for reference in collisions.groups.iter().filter_map(|g| g.profile.as_ref()) {
                let profile = source.load(reference)?;
                profile.validate_reference(&reference.path)?;
                export_collision_profile(source, &profile, files, gaps)?;
            }
            let fragment = super::level::write_character_collisions(
                collisions,
                &files.collision_profiles,
                perso.path.role(),
                0x70000000,
            )?;
            for (extension, bytes) in [
                ("collisions.bin", fragment.bytes),
                ("collisions.rtb", fragment.rtb),
            ] {
                files.write(&perso.path, fragment_path(&perso.path, extension), &bytes)?;
            }
            gaps.extend(fragment.coverage);
        }
        let family = source.load(&perso.family)?;
        family.validate_reference(&perso.family.path)?;
        if let Some(reference) = &family.bounding_geometry {
            let geometry = source.load(reference)?;
            geometry.validate_reference(&reference.path)?;
            export_collision(source, &geometry, files, gaps)?;
        }
        mechanics_paths.extend(
            family
                .states
                .iter()
                .filter_map(|s| s.mechanics.as_ref())
                .map(|r| r.path.clone()),
        );
        family_values.insert(family.path.clone(), family.as_ref().clone());
        for r in family
            .configurations
            .iter()
            .chain(family.default_configuration.iter())
            .chain(perso.configuration.iter())
            .chain(perso.initial_configuration.iter())
        {
            configurations.insert(r.path.clone());
        }
        for r in family.states.iter().filter_map(|s| s.animation.as_ref()) {
            clips.insert(r.path.clone());
        }
    }
    for path in &configurations {
        let config = source.load(&AssetRef::<crate::model::character::ObjectList>::from_path(
            path.clone(),
        ))?;
        config.validate_reference(path)?;
        action_paths.extend(
            config
                .all_parts()
                .filter_map(|p| p.action.as_ref())
                .map(|r| r.path.clone()),
        );
        for reference in config.all_parts().filter_map(|p| p.instance.as_ref()) {
            let instance = source.load(reference)?;
            instance.validate_reference(&reference.path)?;
            let model = source.load(&instance.model)?;
            export_instance(source, &model, &instance, files, gaps)?;
        }
    }
    for path in &clips {
        let clip = source
            .load(&AssetRef::<crate::model::animation::AnimationClip>::from_path(path.clone()))?;
        clip.validate_reference(path)?;
        action_paths.extend(clip.cues.iter().map(|c| c.action.path.clone()));
    }
    let mut action_groups = BTreeMap::<_, Vec<_>>::new();
    let mut libraries = BTreeSet::new();
    for path in action_paths {
        let action = source
            .load(&AssetRef::<crate::model::animation::AnimationAction>::from_path(path.clone()))?;
        action.validate_reference(&path)?;
        if let crate::model::animation::AnimationOperation::Sound { event: Some(event) } =
            &action.operation
        {
            libraries.insert(event.library.path.clone());
            let library = source.load(&event.library)?;
            event.binding(&library)?;
        }
        action_groups
            .entry(path.role())
            .or_default()
            .push(action.as_ref().clone());
    }
    for path in libraries {
        let value = source
            .load(&AssetRef::<crate::model::audio::SoundDefinitions>::from_path(path.clone()))?;
        let bytes = (super::codec::SoundScript {
            name: value.name.clone(),
        })
        .write(&value)?;
        files.write(&path, path.file().to_owned(), &bytes)?;
    }
    let mut actions = BTreeMap::new();
    for (role, values) in action_groups {
        let (role_name, base) = match role {
            crate::model::PackageRole::Fix => ("fix", 0x50000000),
            crate::model::PackageRole::Level => ("level", 0x60000000),
            _ => anyhow::bail!("invalid action package role"),
        };
        let out = super::level::write_animation_actions(&values, base)?;
        let owner = AssetPath::new(format!("{role_name}:/animation-actions"))?;
        for (extension, bytes) in [("bin", out.bytes), ("snd", out.snd), ("rts", out.rts)] {
            files.write(&owner, fragment_path(&owner, extension), &bytes)?;
        }
        actions.extend(out.targets);
    }
    let mut pools = BTreeMap::new();
    let mut clip_targets = BTreeMap::new();
    let mut clip_base = 0x20000000u32;
    for path in clips {
        let clip = source
            .load(&AssetRef::<crate::model::animation::AnimationClip>::from_path(path.clone()))?;
        clip.validate_reference(&path)?;
        for link in clip.samples.iter().filter_map(|s| s.transform.as_ref()) {
            if pools.contains_key(&link.pool.path) {
                continue;
            }
            let pool = source.load(&link.pool)?;
            pool.validate_reference(&link.pool.path)?;
            let base = if pool.path.role() == crate::model::PackageRole::Fix {
                0x30000000
            } else {
                0x40000000
            };
            let out = super::level::write_transform_pool(&pool, base)?;
            files.write(&pool.path, fragment_path(&pool.path, "bin"), &out.bytes)?;
            pools.insert(pool.path.clone(), out.targets);
        }
        let out = super::level::write_animation(&clip, &pools, &actions, clip_base)?;
        clip_targets.insert(
            path.clone(),
            super::level::ReferenceTarget {
                address: clip_base,
                block: if path.role() == crate::model::PackageRole::Fix {
                    (5, 0)
                } else {
                    (5, 1)
                },
            },
        );
        clip_base = clip_base
            .checked_add(((out.bytes.len() + 3) & !3).try_into()?)
            .context("animation address overflow")?;
        ensure!(
            clip_base < 0x30000000,
            "animations exceed controlled address range"
        );
        files.write(&path, fragment_path(&path, "bin"), &out.bytes)?;
        files.write(&path, fragment_path(&path, "rtb"), &out.rtb)?;
        gaps.extend(out.coverage);
    }
    let mut mechanics = BTreeMap::new();
    for path in mechanics_paths {
        let value = source.load(&AssetRef::<crate::model::mechanics::Mechanics>::from_path(
            path.clone(),
        ))?;
        value.validate_reference(&path)?;
        let bytes = super::level::write_mechanics(&value)?;
        let target = super::level::ReferenceTarget {
            address: files.next_geometry_base,
            block: if path.role() == crate::model::PackageRole::Fix {
                (6, 1)
            } else {
                (6, 2)
            },
        };
        files.next_geometry_base = files
            .next_geometry_base
            .checked_add(bytes.len().try_into()?)
            .context("mechanics address overflow")?;
        ensure!(
            files.next_geometry_base < 0x20000000,
            "mechanics exceed controlled address range"
        );
        files.write(&path, fragment_path(&path, "bin"), &bytes)?;
        mechanics.insert(path, target);
    }
    let mut config_values = Vec::new();
    for path in configurations {
        let value = source.load(&AssetRef::<crate::model::character::ObjectList>::from_path(
            path.clone(),
        ))?;
        config_values.push(value.as_ref().clone());
    }
    let mut config_targets = BTreeMap::new();
    for (role, base, name) in [
        (crate::model::PackageRole::Fix, 0x70000000, "fix"),
        (crate::model::PackageRole::Level, 0x78000000, "level"),
    ] {
        let families = family_values
            .values()
            .filter(|f| f.path.role() == role)
            .cloned()
            .collect::<Vec<_>>();
        let configs = config_values
            .iter()
            .filter(|c| c.path.role() == role)
            .cloned()
            .collect::<Vec<_>>();
        if families.is_empty() && configs.is_empty() {
            continue;
        }
        let out = super::level::write_families(
            &families,
            &configs,
            &super::level::FamilyDependencies {
                animations: &clip_targets,
                physical_objects: &files.model_targets,
                actions: &actions,
                bounds: &files.collision_targets,
                mechanics: &mechanics,
                configurations: &config_targets,
            },
            base,
        )?;
        config_targets.extend(out.configurations);
        let owner = AssetPath::new(format!("{name}:/families"))?;
        for (ext, bytes) in [("bin", out.fragment.bytes), ("rtb", out.fragment.rtb)] {
            files.write(&owner, fragment_path(&owner, ext), &bytes)?;
        }
        gaps.extend(out.fragment.coverage);
    }
    Ok(())
}

fn export_collision(
    source: &mut impl ContentSource,
    collision: &crate::model::collision::Collision,
    files: &mut ExportFiles<'_>,
    gaps: &mut Vec<String>,
) -> Result<()> {
    if files.contains(&collision.path, &fragment_path(&collision.path, "bin"))? {
        return Ok(());
    }
    let base = files.next_geometry_base;
    let out = super::level::write_collision(collision, source, base)?;
    files.next_geometry_base = base
        .checked_add(((out.bytes.len() + 3) & !3).try_into()?)
        .context("collision address overflow")?;
    ensure!(
        files.next_geometry_base < 0x20000000,
        "collision exceeds controlled address range"
    );
    files.collision_targets.insert(
        collision.path.clone(),
        super::level::ReferenceTarget {
            address: base,
            block: (6, 2),
        },
    );
    for (ext, bytes) in [("bin", out.bytes), ("rtb", out.rtb)] {
        files.write(&collision.path, fragment_path(&collision.path, ext), &bytes)?;
    }
    gaps.extend(out.coverage);
    Ok(())
}
