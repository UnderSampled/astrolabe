//! Preview fog for scenes whose sectors all resolve to the same fog value.
//! Mixed sector environments require per-draw routing, as in Hype's 004df0e0.
use super::{Project, quoted};
use crate::model::{content::ContentSource, environment::*, io::Content, scene::Scene};
use anyhow::Result;
use std::{collections::BTreeMap, fmt::Write as _};
impl Project {
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
            self.diagnostic(format!("{}: shared environment fog projected in the preview with native depth fog; Godot's smooth depth curve approximates Hype's quantized linear Glide table; infinite distance is retained but unused by that driver",scene.path));
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
