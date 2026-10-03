//! Godot projection of Hype's material/texture sampling controls.
use super::{Project, disk_path, quoted, whole_asset};
use crate::model::{
    AssetRef,
    geometry::Material,
    texture::{Texture, TextureColors, TexturePixels, TextureUsage},
};
use anyhow::{Context, Result};
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

/// The selected palette entry has identity even when another entry has the same
/// RGB. Hype substitutes its GF color-key RGB into that entry on upload
/// (00481310, 0049a2e0). Never erase all black pixels or alter canonical samples.
pub(super) fn keyed_rgba(texture: &Texture, rgba: &[u8]) -> Vec<u8> {
    let mut out = rgba.to_vec();
    let level = &texture.levels[0];
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
    pub(super) fn prepare_color_key(
        &mut self,
        reference: &AssetRef<Texture>,
        texture: &Texture,
        rgba: &[u8],
    ) -> Result<()> {
        let keyed = keyed_rgba(texture, rgba);
        if keyed != rgba {
            self.texture_color_keys.insert(
                reference.path.clone(),
                Some(png(texture.width(), texture.height(), &keyed)?),
            );
        }
        Ok(())
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
            let mut texture_path = disk_path(&self.selection, texture, "png");
            if key && let Some(pending) = self.texture_color_keys.get(texture) {
                texture_path = disk_path(&self.selection, texture, "color-key.png");
                if let Some(bytes) = pending {
                    self.write(texture_path.clone(), &bytes.clone())?;
                    self.texture_color_keys.insert(texture.clone(), None);
                }
            }
            writeln!(
                resource,
                "[ext_resource type=\"Texture2D\" path={} id=\"texture\"]\n",
                quoted(&format!("res://{texture_path}"))
            )?;
        }
        // Hype Glide initialization at 0049ba75/0049d4e2 calls
        // grTexFilterMode(tmu, BILINEAR, BILINEAR). Native linear sampling
        // preserves this magnification behavior; source mip LOD dithering
        // remains a separate projection gap.
        resource.push_str("[resource]\nshading_mode = 2\nroughness = 1.0\nmetallic_specular = 0.0\ntexture_filter = 1\nvertex_color_use_as_albedo = true\n");
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
        self.diagnostics.push(format!("{path}: vertex-lighting projection; bilinear filtering and color-key/alpha/additive modes mapped; source mip LOD dithering, full render context, chrome and scrolling remain provisional"));
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
