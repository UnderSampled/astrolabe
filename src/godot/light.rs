//! Provisional native lights for dynamic meshes; baked radiosity stays unshaded.
use super::{Project, quoted};
use crate::model::{io::Content, light::Light};
use anyhow::Result;
use std::fmt::Write as _;
impl Project {
    pub(super) fn light_nodes(&mut self, light: &Light, parent: &str, index: usize) -> Result<()> {
        light.validate()?;
        let name = format!("{parent}_light_{index}");
        writeln!(
            self.builder,
            "    var {name} = Node3D.new()\n    {name}.name = {}\n    {parent}.add_child({name})\n    {name}.owner = {parent}\n    {name}.transform = {}\n    {name}.set_script(load(\"res://addons/astrolabe/rendering/source_light.gd\"))",
            quoted(&format!("Light_{index}")),
            super::scene::transform(&light.transform),
        )?;
        let c = light.color;
        writeln!(
            self.builder,
            "    {name}.source_path = {}\n    {name}.source_name = {}\n    {name}.kind = {}\n    {name}.lit = {}\n    {name}.sends = {}\n    {name}.cast_shadows = {}\n    {name}.creates_shadows = {}\n    {name}.rotating = {}\n    {name}.pulsing = {}\n    {name}.far = {}\n    {name}.near = {}\n    {name}.angles = Vector2({}, {})\n    {name}.rotation_step = {}\n    {name}.pulse_step = {}\n    {name}.source_color = Color({}, {}, {}, {})\n    {name}.painting = {}\n    {name}.intensity_min = {}\n    {name}.unresolved_dimmer = {}",
            quoted(light.path.as_str()),
            quoted(&light.name),
            light.kind,
            light.on != 0,
            light.send != 0,
            light.cast_shadows != 0,
            light.creates_shadows,
            light.rotating != 0,
            light.pulsing != 0,
            light.far,
            light.near,
            light.angles[0],
            light.angles[1],
            light.rotation_step,
            light.pulse_step,
            c[0],
            c[1],
            c[2],
            c[3],
            light.painting,
            light.intensity_min,
            light.unresolved_dimmer,
        )?;
        let class = match light.kind {
            1 | 5 => "DirectionalLight3D",
            2 | 7 | 8 => "OmniLight3D",
            3 => "SpotLight3D",
            _ => "",
        };
        let enabled = light.on != 0 && light.send != 0;
        if light.kind == 4 && enabled {
            let ambient = self.scene_ambient.get_or_insert([0.; 3]);
            for (dst, src) in ambient.iter_mut().zip(light.color) {
                *dst += 2. * src;
            }
        }
        if !class.is_empty() {
            for negative in [false, true] {
                let rgb: [f32; 3] = std::array::from_fn(|i| {
                    if negative {
                        (-light.color[i]).max(0.)
                    } else {
                        light.color[i].max(0.)
                    }
                });
                if rgb == [0.; 3] {
                    continue;
                }
                let v = format!("{name}_{}", if negative { "negative" } else { "positive" });
                writeln!(
                    self.builder,
                    "    var {v} = {class}.new()\n    {v}.name = {}\n    {name}.add_child({v})\n    {v}.owner = {parent}\n    {v}.light_color = Color({}, {}, {})\n    {v}.light_energy = 2.0\n    {v}.light_negative = {negative}\n    {v}.light_cull_mask = 2\n    {v}.visible = {enabled}",
                    quoted(if negative { "Negative" } else { "Positive" }),
                    rgb[0],
                    rgb[1],
                    rgb[2]
                )?;
                if class == "OmniLight3D" {
                    writeln!(
                        self.builder,
                        "    {v}.omni_range = {}",
                        light.far.max(0.001)
                    )?;
                }
                if class == "SpotLight3D" {
                    writeln!(
                        self.builder,
                        "    {v}.spot_range = {}\n    {v}.spot_angle = {}",
                        light.far.max(0.001),
                        (light.angles[1].to_degrees() * 0.5).clamp(0.01, 89.)
                    )?;
                }
            }
        }
        self.diagnostic(format!("{}: provisional native light projection; exact attenuation, sector filtering, spotlight orientation, fog, pulse/rotation and shadows remain unmapped; signed source values retained",light.path));
        Ok(())
    }
}
