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

/// The next level of an RGBA8 chain, as Godot's Image::generate_mipmaps makes
/// it: each output pixel averages a 2x2 block, rounding half up; a side of one
/// pixel repeats its row or column.
fn half_level(width: u32, height: u32, rgba: &[u8]) -> (u32, u32, Vec<u8>) {
    let (w, h) = ((width / 2).max(1), (height / 2).max(1));
    let (right, down) = (usize::from(width > 1), usize::from(height > 1));
    let at = |x: usize, y: usize, c: usize| u32::from(rgba[(y * width as usize + x) * 4 + c]);
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h as usize {
        for x in 0..w as usize {
            let (x0, y0) = (x * 2, y * 2);
            for c in 0..4 {
                let sum = at(x0, y0, c)
                    + at(x0 + right, y0, c)
                    + at(x0, y0 + down, c)
                    + at(x0 + right, y0 + down, c);
                out.push(((sum + 2) >> 2) as u8);
            }
        }
    }
    (w, h, out)
}

/// An uncompressed RGBA8 DDS holding the whole chain. Godot loads it as a
/// native texture with these exact levels; its importer would make its own.
fn dds(levels: &[(u32, u32, Vec<u8>)]) -> Vec<u8> {
    let (width, height) = (levels[0].0, levels[0].1);
    let mut out = Vec::new();
    let mut put = |value: u32| out.extend(value.to_le_bytes());
    put(u32::from_le_bytes(*b"DDS "));
    // Header: size, flags CAPS|HEIGHT|WIDTH|PITCH|PIXELFORMAT|MIPMAPCOUNT.
    for value in [
        124,
        0x2100f,
        height,
        width,
        width * 4,
        0,
        levels.len() as u32,
    ] {
        put(value);
    }
    for _ in 0..11 {
        put(0);
    }
    // Pixel format: RGB|ALPHAPIXELS, 32 bits, byte order R, G, B, A.
    for value in [32, 0x41, 0, 32, 0xff, 0xff00, 0xff0000, 0xff00_0000] {
        put(value);
    }
    // Caps: COMPLEX|TEXTURE|MIPMAP.
    for value in [0x401008, 0, 0, 0, 0] {
        put(value);
    }
    for level in levels {
        out.extend(&level.2);
    }
    out
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
    /// Retain supplied source levels as a DDS mip chain. 2D
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
        _path: &AssetPath,
        texture: &Texture,
    ) -> Result<Vec<TextureMipInput>> {
        texture
            .levels
            .iter()
            .enumerate()
            .map(|(index, level)| {
                Ok(TextureMipInput {
                    width: level.width,
                    height: level.height,
                    rgba: texture.rgba8(index)?,
                })
            })
            .collect()
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
            "color-key.mips.dds"
        } else {
            "mips.dds"
        };
        let output = disk_path(&self.selection, path, suffix);
        let mut levels = inputs
            .into_iter()
            .map(|input| (input.width, input.height, input.rgba))
            .collect::<Vec<_>>();
        while levels.len() < total_levels {
            let last = levels.last().expect("a chain has a base level");
            levels.push(half_level(last.0, last.1, &last.2));
        }
        self.write(output.clone(), &dds(&levels))?;
        let source_kind = if use_keyed { "color-keyed " } else { "" };
        let tail_note = if generated_levels == 0 {
            "all supplied levels preserved"
        } else {
            "missing tail generated from the last supplied level"
        };
        self.diagnostics.push(format!(
            "{output}: {source_kind}DDS retains {} supplied OpenSpace mip level(s); {generated_levels} tail level(s) generated ({tail_note})",
            total_levels - generated_levels
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
        Ok(texture
            .levels
            .iter()
            .zip(levels)
            .map(|(level, rgba)| TextureMipInput {
                width: level.width,
                height: level.height,
                rgba: rgba.clone(),
            })
            .collect())
    }

    pub fn write_material(&mut self, material: &Material) -> Result<()> {
        material.validate()?;
        whole_asset(&material.path)?;
        if let Some(texture) = material.texture() {
            whole_asset(texture)?;
        }
        let source_material_path = material.path.to_string();
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
            "metadata/source_material_flags = {}\nmetadata/source_texture_flags = {}\nmetadata/source_material_path = {}\nmetadata/source_visual_ambient = Color({}, {}, {}, {})\nmetadata/source_visual_diffuse = Color({}, {}, {}, {})",
            material.flags,
            flags,
            quoted(&source_material_path),
            material.ambient[0],
            material.ambient[1],
            material.ambient[2],
            material.ambient[3],
            material.diffuse[0],
            material.diffuse[1],
            material.diffuse[2],
            material.diffuse[3],
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
