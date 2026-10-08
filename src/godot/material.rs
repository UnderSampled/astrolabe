//! Godot projection of Hype's material/texture sampling controls.
use super::{Project, disk_path, project::TextureMipInput, quoted, whole_asset};
use crate::model::{
    AssetPath, AssetRef,
    geometry::Material,
    texture::{Texture, TextureColors, TexturePixels, TextureUsage},
};
use anyhow::{Context, Result, ensure};
use std::fmt::Write as _;

pub(super) fn png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header()?.write_image_data(rgba)?;
    }
    Ok(bytes)
}

fn full_mip_level_count(mut width: u32, mut height: u32) -> usize {
    let mut levels = 1;
    while width > 1 || height > 1 {
        width = (width / 2).max(1);
        height = (height / 2).max(1);
        levels += 1;
    }
    levels
}

/// The selected palette entry has identity even when another entry has the same
/// RGB. Hype substitutes its GF color-key RGB into that entry on upload
/// (00481310, 0049a2e0). Never erase all black pixels or alter canonical samples.
pub(super) fn keyed_rgba(texture: &Texture, rgba: &[u8]) -> Vec<u8> {
    keyed_rgba_level(texture, 0, rgba)
}

fn keyed_rgba_level(texture: &Texture, level_index: usize, rgba: &[u8]) -> Vec<u8> {
    let mut out = rgba.to_vec();
    let level = &texture.levels[level_index];
    for (i, pixel) in out.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let x = i % level.width as usize;
        let y = i / level.width as usize;
        let sy = if texture.usage == TextureUsage::Material {
            level.height as usize - 1 - y
        } else {
            y
        };
        let source = sy * level.width as usize + x;
        let keyed = match (&texture.colors, &level.pixels) {
            (
                TextureColors::PaletteRgb(_) | TextureColors::PaletteRgba(_),
                TexturePixels::Indices(indices),
            ) => i32::from(indices[source]) == texture.openspace_color_key.palette_index,
            (TextureColors::Components { bits }, TexturePixels::Components(samples))
                if bits[3] == 0 =>
            {
                (0..3).all(|c| {
                    samples[source][c] == texture.openspace_color_key.rgb[c] >> (8 - bits[c])
                })
            }
            // ARGB1555/4444 already carry the source coverage in their alpha.
            _ => false,
        };
        if keyed {
            pixel[3] = 0;
        }
    }
    out
}

impl Project {
    /// Retain supplied source levels as a native Godot ImageTexture. 2D
    /// consumers continue to use their existing base PNGs and filters.
    pub(super) fn retain_texture_mips(
        &mut self,
        reference: &AssetRef<Texture>,
        texture: &Texture,
    ) -> Result<()> {
        if texture.levels.len() <= 1 {
            return Ok(());
        }
        if reference.path.member().is_some() {
            self.diagnostics.push(format!(
                "{}: source mip levels remain intact in the model; Godot mip resource omitted for a member reference",
                reference.path
            ));
            return Ok(());
        }
        self.register_texture_mip_source(reference, texture)
    }

    pub(super) fn register_texture_mip_source(
        &mut self,
        reference: &AssetRef<Texture>,
        texture: &Texture,
    ) -> Result<()> {
        texture.validate()?;
        whole_asset(&reference.path)?;
        if !self.texture_mip_sources.contains_key(&reference.path) {
            let inputs = self.stage_mip_inputs(&reference.path, texture)?;
            self.texture_mip_sources
                .insert(reference.path.clone(), inputs);
        }
        if texture.levels.len() > 1 {
            self.ensure_mip_texture_resource(&reference.path, false, false)?;
        }
        Ok(())
    }

    fn stage_mip_inputs(
        &mut self,
        path: &AssetPath,
        texture: &Texture,
    ) -> Result<Vec<TextureMipInput>> {
        let input_id = self.next_mip_input;
        self.next_mip_input += 1;
        let mut inputs = Vec::with_capacity(texture.levels.len());
        for (index, level) in texture.levels.iter().enumerate() {
            let base_path = disk_path(&self.selection, path, "png");
            let relative = if index == 0 && self.generated.contains(&base_path) {
                base_path
            } else {
                let rgba = texture.rgba8(index)?;
                let png = png(level.width, level.height, &rgba)?;
                let relative =
                    format!("build_inputs/texture-mips/{input_id:06}/level-{index:02}.png");
                self.write_build_input(&relative, &png)?;
                relative
            };
            inputs.push(TextureMipInput {
                width: level.width,
                height: level.height,
                path: relative,
            });
        }
        Ok(inputs)
    }

    fn ensure_mip_texture_resource(
        &mut self,
        path: &AssetPath,
        keyed: bool,
        force: bool,
    ) -> Result<Option<String>> {
        let use_keyed = keyed && self.texture_color_key_mip_sources.contains_key(path);
        let cache_key = (path.clone(), use_keyed);
        if let Some(output) = self.texture_mip_resources.get(&cache_key) {
            return Ok(Some(output.clone()));
        }
        let inputs = if use_keyed {
            self.texture_color_key_mip_sources.get(path)
        } else {
            self.texture_mip_sources.get(path)
        }
        .cloned()
        .with_context(|| format!("texture mip source was not registered for {path}"))?;
        if inputs.len() == 1 && !force {
            return Ok(None);
        }
        let total_levels = full_mip_level_count(inputs[0].width, inputs[0].height);
        ensure!(
            inputs.len() <= total_levels,
            "{} has {} mip levels for a maximum chain of {}",
            path,
            inputs.len(),
            total_levels
        );
        let generated_levels = total_levels - inputs.len();
        let suffix = if use_keyed {
            "color-key.mips.tres"
        } else {
            "mips.tres"
        };
        let output = disk_path(&self.selection, path, suffix);
        self.reserve(&output)?;
        let input_paths = inputs
            .iter()
            .map(|input| quoted(&format!("res://{}", input.path)))
            .collect::<Vec<_>>()
            .join(", ");
        let output_res = format!("res://{output}");
        let append_tail = generated_levels > 0;
        let expected_mips = total_levels - 1;
        self.mip_texture_build_calls.push(format!(
            "\tif not build_mip_texture(PackedStringArray([{input_paths}]), {}, {append_tail}, {expected_mips}):\n\t\tpush_error({})\n\t\tquit(1)\n\t\treturn",
            quoted(&output_res),
            quoted(&format!("Failed to build source mip resource for {path}")),
        ));
        let source_kind = if use_keyed { "color-keyed " } else { "" };
        let tail_note = if generated_levels == 0 {
            "all supplied levels preserved"
        } else {
            "missing tail generated from the last supplied level"
        };
        self.diagnostics.push(format!(
            "{output}: {source_kind}ImageTexture retains {} supplied OpenSpace mip level(s); {generated_levels} tail level(s) generated ({tail_note})",
            inputs.len()
        ));
        self.texture_mip_resources.insert(cache_key, output.clone());
        Ok(Some(output))
    }

    pub(super) fn prepare_color_key(
        &mut self,
        reference: &AssetRef<Texture>,
        texture: &Texture,
    ) -> Result<()> {
        let source_levels = (0..texture.levels.len())
            .map(|index| texture.rgba8(index))
            .collect::<Result<Vec<_>>>()?;
        let keyed_levels = source_levels
            .iter()
            .enumerate()
            .map(|(index, rgba)| keyed_rgba_level(texture, index, rgba))
            .collect::<Vec<_>>();
        if keyed_levels
            .iter()
            .zip(&source_levels)
            .any(|(keyed, source)| keyed != source)
        {
            let keyed_inputs = self.stage_mip_inputs_from_rgba(texture, &keyed_levels)?;
            self.texture_color_key_mip_sources
                .insert(reference.path.clone(), keyed_inputs);
            self.texture_color_keys.insert(
                reference.path.clone(),
                Some(png(texture.width(), texture.height(), &keyed_levels[0])?),
            );
        }
        Ok(())
    }

    fn stage_mip_inputs_from_rgba(
        &mut self,
        texture: &Texture,
        levels: &[Vec<u8>],
    ) -> Result<Vec<TextureMipInput>> {
        let input_id = self.next_mip_input;
        self.next_mip_input += 1;
        let mut inputs = Vec::with_capacity(texture.levels.len());
        for (index, (level, rgba)) in texture.levels.iter().zip(levels).enumerate() {
            let relative = format!("build_inputs/texture-mips/{input_id:06}/level-{index:02}.png");
            self.write_build_input(&relative, &png(level.width, level.height, rgba)?)?;
            inputs.push(TextureMipInput {
                width: level.width,
                height: level.height,
                path: relative,
            });
        }
        Ok(inputs)
    }

    pub fn write_material(&mut self, material: &Material) -> Result<()> {
        material.validate()?;
        whole_asset(&material.path)?;
        if let Some(texture) = material.texture() {
            whole_asset(texture)?;
        }
        let path = disk_path(&self.selection, &material.path, "tres");
        let material = material
            .visual
            .as_ref()
            .context("material has no visual component")?;
        let flags = material.texture_info.as_ref().map_or(0, |info| info.flags);
        // Hype FUN_00499ed0: texture +8 bit 1 enables chromakey; bit 3 selects
        // alpha; bit 5 selects ONE+ONE additive blending. Material flags gate
        // texture use (bit 0) and shaded blending (0x200 and 0x20000000).
        let textured = material.texture.is_some() && material.flags & 1 != 0;
        let key = textured && flags & 2 != 0;
        let blend = textured && material.flags & 0x20000200 == 0x20000200;
        let additive = blend && flags & 0x20 != 0;
        let alpha = textured && flags & 8 != 0;
        let mut resource =
            String::from("[gd_resource type=\"StandardMaterial3D\" load_steps=2 format=3]\n\n");
        if let Some(texture) = &material.texture {
            if key && let Some(Some(bytes)) = self.texture_color_keys.get(texture).cloned() {
                self.write(disk_path(&self.selection, texture, "color-key.png"), &bytes)?;
                self.texture_color_keys.insert(texture.clone(), None);
            }
            let texture_path = self
                .ensure_mip_texture_resource(texture, key, true)?
                .context("3D material mip resource was not created")?;
            writeln!(
                resource,
                "[ext_resource type=\"Texture2D\" path={} id=\"texture\"]\n",
                quoted(&format!("res://{texture_path}"))
            )?;
        }
        // Hype uses bilinear sampling; the native texture now supplies the
        // preserved source chain (or an explicit generated tail) to Godot.
        resource.push_str("[resource]\nshading_mode = 2\nroughness = 1.0\nmetallic_specular = 0.0\ntexture_filter = 3\nvertex_color_use_as_albedo = true\n");
        // A zero flag set is also useful for independently authored materials.
        if material.texture.is_some() {
            resource.push_str("albedo_texture = ExtResource(\"texture\")\n");
        }
        if additive {
            resource.push_str("transparency = 1\nblend_mode = 1\n");
        } else if alpha && blend {
            resource.push_str("transparency = 1\n");
        } else if alpha {
            // The original alpha test remains active when shaded blending is
            // disabled: GREATER than reference 7 on eight-bit source alpha.
            resource.push_str("transparency = 2\nalpha_scissor_threshold = 0.03137255\n");
        } else if key {
            // Binary coverage stays in the depth-writing cutout pass.
            resource.push_str("transparency = 2\nalpha_scissor_threshold = 0.5\n");
        }
        writeln!(
            resource,
            "albedo_color = Color({}, {}, {}, {})",
            material.color[0], material.color[1], material.color[2], material.color[3]
        )?;
        writeln!(
            resource,
            "metadata/source_material_flags = {}\nmetadata/source_texture_flags = {}",
            material.flags, flags
        )?;
        self.diagnostics.push(format!("{path}: vertex-lighting projection; linear mip filtering and color-key/alpha/additive modes mapped; source LOD selection, full render context, chrome and scrolling remain provisional"));
        self.write(path, resource.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::texture::{OpenSpaceColorKey, TextureLevel};

    #[test]
    fn palette_key_uses_entry_identity_and_preserves_opaque_duplicate_rgb() -> Result<()> {
        let texture = Texture {
            usage: TextureUsage::Material,
            colors: TextureColors::PaletteRgb(vec![[0, 0, 0], [0, 0, 0], [240, 20, 80]]),
            levels: vec![TextureLevel {
                width: 2,
                height: 2,
                pixels: TexturePixels::Indices(vec![0, 1, 2, 0]),
            }],
            openspace_color_key: OpenSpaceColorKey {
                rgb: [84, 45, 24],
                palette_index: 0,
            },
        };
        let original = texture.rgba8(0)?;
        let keyed = keyed_rgba(&texture, &original);
        assert_eq!(
            keyed,
            [240, 20, 80, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255]
        );
        assert_eq!(texture.rgba8(0)?, original);
        let mut inactive = texture;
        inactive.openspace_color_key.palette_index = -1;
        assert_eq!(keyed_rgba(&inactive, &original), original);
        Ok(())
    }
}
