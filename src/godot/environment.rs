//! Preview fog for scenes whose sectors all resolve to the same fog value.
//! Mixed sector environments require per-draw routing, as in Hype's 004df0e0.
use super::{Project, quoted, vec3, vector};
use crate::model::{content::ContentSource, environment::*, io::Content, scene::Scene};
use anyhow::Result;
use std::{collections::BTreeMap, fmt::Write as _};
impl Project {
    /// Project the common first physical environment as a native level
    /// resource. Additional entries still require source sector selection.
    pub(super) fn scene_physical_environment(
        &mut self,
        scene: &Scene,
        source: &mut impl ContentSource,
        name: &str,
    ) -> Result<Option<String>> {
        let mut selected: Option<PhysicalEnvironment> = None;
        let mut sector_count = 0;
        let mut additional = false;
        for sector in scene.nodes.iter().flat_map(|node| &node.sector) {
            sector_count += 1;
            additional |= sector.environments.len() > 1;
            let Some(reference) = sector.environments.first() else {
                self.diagnostic(format!("{}: sector {} has no physical environment; level gravity remains project-owned", scene.path, sector.name));
                return Ok(None);
            };
            let env = source.load(reference)?;
            env.validate_reference(&reference.path)?;
            let Some(reference) = &env.physical else {
                self.diagnostic(format!("{}: sector {} has no physical environment; level gravity remains project-owned", scene.path, sector.name));
                return Ok(None);
            };
            let physical = source.load(reference)?;
            physical.validate_reference(&reference.path)?;
            if let Some(first) = &selected {
                if first.gravity != physical.gravity || first.vertical != physical.vertical {
                    self.diagnostic(format!("{}: first sector physical environments differ; native level gravity needs sector selection", scene.path));
                    return Ok(None);
                }
            } else {
                selected = Some((*physical).clone());
            }
        }
        let Some(physical) = selected else {
            return Ok(None);
        };
        let variable = format!("{name}_physical_environment");
        writeln!(
            self.builder,
            "    var {variable} = Resource.new()\n    {variable}.set_script(load(\"res://addons/astrolabe/character/physical_environment.gd\"))\n    {variable}.gravity = {}\n    {variable}.vertical = {}",
            physical.gravity,
            vec3(vector(physical.vertical)),
        )?;
        self.diagnostic(format!("{}: {} sectors share first physical environment gravity {} and vertical {:?}; exported as a native level Resource", scene.path, sector_count, physical.gravity, physical.vertical));
        if additional {
            self.diagnostic(format!("{}: additional sector environments require dynamic selection; the common first physical environment is the level gameplay default", scene.path));
        }
        Ok(Some(variable))
    }

    pub(super) fn native_environment(&mut self, root: &str) -> Result<()> {
        let color = self.scene_ambient.unwrap_or([1.; 3]);
        let energy = if self.scene_ambient.is_some() {
            1.0
        } else {
            0.7
        };
        writeln!(
            self.builder,
            "    var {root}_environment = Environment.new()\n    {root}_environment.background_mode = Environment.BG_COLOR\n    {root}_environment.background_color = Color(0.08, 0.1, 0.14, 1)\n    {root}_environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR\n    {root}_environment.ambient_light_color = Color({}, {}, {}, 1)\n    {root}_environment.ambient_light_energy = {energy}",
            color[0].clamp(0., 1.),
            color[1].clamp(0., 1.),
            color[2].clamp(0., 1.)
        )?;
        if !self.scene_fog_conflict
            && let Some(Some(fog)) = &self.scene_fog
        {
            for property in preview_properties(fog).lines() {
                writeln!(self.builder, "    {root}_environment.{property}")?;
            }
        }
        writeln!(
            self.builder,
            "    var {root}_world_environment = WorldEnvironment.new()\n    {root}_world_environment.name = \"Environment\"\n    {root}_world_environment.environment = {root}_environment\n    {root}.add_child({root}_world_environment)\n    {root}_world_environment.owner = {root}\n    {root}.set_meta(\"astrolabe_support_interface\", {})",
            super::support::INTERFACE
        )?;
        Ok(())
    }

    pub(super) fn scene_environment(
        &mut self,
        scene: &Scene,
        source: &mut impl ContentSource,
        name: &str,
    ) -> Result<()> {
        let mut environments = BTreeMap::new();
        let mut visuals = BTreeMap::new();
        let mut fogs = BTreeMap::new();
        let mut selections = Vec::new();
        for sector in scene.nodes.iter().flat_map(|n| &n.sector) {
            if sector.environments.is_empty() {
                selections.push(None);
            }
            for reference in &sector.environments {
                let env = source.load(reference)?;
                env.validate_reference(&reference.path)?;
                let fog = if let Some(reference) = &env.visual {
                    let visual = source.load(reference)?;
                    visual.validate_reference(&reference.path)?;
                    let fog = if let Some(reference) = &visual.fog {
                        let fog = source.load(reference)?;
                        fog.validate_reference(&reference.path)?;
                        fogs.insert(reference.path.clone(), (*fog).clone());
                        Some((*fog).clone())
                    } else {
                        None
                    };
                    if visual.unresolved_water {
                        self.diagnostic(format!("{}: water effects remain unmapped", visual.path));
                    }
                    visuals.insert(visual.path.clone(), (*visual).clone());
                    fog
                } else {
                    None
                };
                environments.insert(env.path.clone(), (*env).clone());
                selections.push(fog);
            }
        }
        writeln!(
            self.builder,
            "    {name}.set_meta(\"source_environments\", {})",
            quoted(&serde_json::to_string(&(environments, visuals, fogs))?)
        )?;
        for fog in selections {
            if let Some(previous) = &self.scene_fog {
                // Asset identity may differ while the effective fog is identical.
                let same = match (previous, &fog) {
                    (None, None) => true,
                    (Some(a), Some(b)) => a.near == b.near && a.far == b.far && a.color == b.color,
                    _ => false,
                };
                self.scene_fog_conflict |= !same;
            } else {
                self.scene_fog = Some(fog);
            }
        }
        if self.scene_fog_conflict {
            self.diagnostic(format!("{}: multiple fog environments; preview fog disabled until sector/surface draw routing is implemented",scene.path));
        } else if self.scene_fog.as_ref().is_some_and(Option::is_some) {
            self.diagnostic(format!("{}: shared environment fog projected in the level with native depth fog; Godot's smooth depth curve approximates Hype's quantized linear Glide table; infinite distance is retained but unused by that driver",scene.path));
        }
        Ok(())
    }
}

pub(super) fn preview_properties(fog: &Fog) -> String {
    // Hype 0049a460 disables fog for near < 0 or near >= far.
    if fog.near < 0. || fog.near >= fog.far {
        return String::new();
    }
    format!(
        "fog_enabled = true\nfog_mode = 1\nfog_density = 1.0\nfog_depth_begin = {}\nfog_depth_end = {}\nfog_depth_curve = 1.0\nfog_light_color = Color({}, {}, {}, 1)\nfog_light_energy = 1.0\nfog_sky_affect = 0.0\n",
        fog.near,
        fog.far,
        fog.color[0].clamp(0., 1.),
        fog.color[1].clamp(0., 1.),
        fog.color[2].clamp(0., 1.)
    )
}
