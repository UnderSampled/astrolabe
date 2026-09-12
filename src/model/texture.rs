//! Useful image values, independent of archive and image-file encodings.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

/// A material image uses bottom-left source rows; screen images use top-left rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextureUsage {
    Material,
    Screen,
}

/// Component precision describes the actual samples, not a cached GF encoding.
/// Alpha precision zero means opaque. Samples retain integer component values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextureColors {
    PaletteRgb(Vec<[u8; 3]>),
    PaletteRgba(Vec<[u8; 4]>),
    Components { bits: [u8; 4] },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TexturePixels {
    Indices(#[serde(with = "serde_bytes")] Vec<u8>),
    Components(Vec<[u8; 4]>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextureLevel {
    pub width: u32,
    pub height: u32,
    pub pixels: TexturePixels,
}

/// These GF header values are exposed by the engine's color-key queries.
/// They can be inactive and must not be replaced with values guessed from pixels.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenSpaceColorKey {
    pub rgb: [u8; 3],
    /// -1 means no selected palette entry; the engine also uses rgb for ARGB1555.
    pub palette_index: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Texture {
    pub usage: TextureUsage,
    pub colors: TextureColors,
    /// Largest image first, followed by supplied mip levels. Missing mips stay missing.
    pub levels: Vec<TextureLevel>,
    pub openspace_color_key: OpenSpaceColorKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedTexture {
    /// Relative content path with `/` separators, including the original member name.
    pub path: String,
    pub texture: Texture,
    /// Another member owning the same content. Clear this link to detach an edit.
    /// Only the logical identity is retained, never a source location.
    pub shared_with: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextureArchive {
    /// Directory identity/order includes empty directories; member offsets are derived.
    pub directories: Vec<String>,
    /// Preserves original member order while owning useful texture values.
    pub textures: Vec<NamedTexture>,
}

impl Texture {
    pub fn width(&self) -> u32 {
        self.levels.first().map_or(0, |x| x.width)
    }
    pub fn height(&self) -> u32 {
        self.levels.first().map_or(0, |x| x.height)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(!self.levels.is_empty(), "texture has no image levels");
        let palette_len = match &self.colors {
            TextureColors::PaletteRgb(p) => Some(p.len()),
            TextureColors::PaletteRgba(p) => Some(p.len()),
            TextureColors::Components { bits } => {
                ensure!(
                    bits[..3].iter().all(|&n| n > 0 && n <= 8) && bits[3] <= 8,
                    "invalid texture component precision {bits:?}"
                );
                None
            }
        };
        if let Some(n) = palette_len {
            ensure!(n > 0 && n <= 256, "palette must have 1..=256 entries");
        }
        let mut previous = None;
        for level in &self.levels {
            ensure!(
                level.width > 0 && level.height > 0,
                "zero texture dimensions"
            );
            let count = u64::from(level.width) * u64::from(level.height);
            ensure!(
                count <= 16_777_216,
                "texture level exceeds 16M-pixel safety limit"
            );
            if let Some((w, h)) = previous {
                ensure!((w, h) != (1, 1), "duplicate terminal mip level");
                ensure!(
                    (level.width, level.height) == ((w / 2).max(1), (h / 2).max(1)),
                    "mip dimensions do not halve the previous level"
                );
            }
            previous = Some((level.width, level.height));
            match (&self.colors, &level.pixels) {
                (
                    TextureColors::PaletteRgb(_) | TextureColors::PaletteRgba(_),
                    TexturePixels::Indices(indices),
                ) => {
                    ensure!(
                        indices.len() == count as usize,
                        "palette-index count does not match dimensions"
                    );
                    ensure!(
                        indices.iter().all(|&v| (v as usize) < palette_len.unwrap()),
                        "texture palette index out of bounds"
                    );
                }
                (TextureColors::Components { bits }, TexturePixels::Components(pixels)) => {
                    ensure!(
                        pixels.len() == count as usize,
                        "component sample count does not match dimensions"
                    );
                    for pixel in pixels {
                        for c in 0..4 {
                            let maximum = if bits[c] == 0 {
                                0
                            } else {
                                (1u16 << bits[c]) - 1
                            };
                            ensure!(
                                u16::from(pixel[c]) <= maximum,
                                "texture component exceeds declared precision"
                            );
                        }
                    }
                }
                _ => anyhow::bail!("texture colors and pixel values disagree"),
            }
        }
        Ok(())
    }

    /// Top-left RGBA8 presentation of one mip; leaves exact samples untouched.
    pub fn rgba8(&self, mip: usize) -> Result<Vec<u8>> {
        self.validate()?;
        let level = self
            .levels
            .get(mip)
            .ok_or_else(|| anyhow::anyhow!("mip {mip} is absent"))?;
        let mut rgba = Vec::with_capacity(level.width as usize * level.height as usize * 4);
        for y in 0..level.height as usize {
            let sy = if self.usage == TextureUsage::Material {
                level.height as usize - 1 - y
            } else {
                y
            };
            for x in 0..level.width as usize {
                let i = sy * level.width as usize + x;
                let color = match (&self.colors, &level.pixels) {
                    (TextureColors::PaletteRgb(p), TexturePixels::Indices(v)) => {
                        let [r, g, b] = p[v[i] as usize];
                        [r, g, b, 255]
                    }
                    (TextureColors::PaletteRgba(p), TexturePixels::Indices(v)) => p[v[i] as usize],
                    (TextureColors::Components { bits }, TexturePixels::Components(p)) => {
                        let mut result = [0, 0, 0, 255];
                        for c in 0..4 {
                            if bits[c] != 0 {
                                result[c] = (u32::from(p[i][c]) * 255 / ((1 << bits[c]) - 1)) as u8;
                            }
                        }
                        result
                    }
                    _ => unreachable!("validate checked texture representation"),
                };
                rgba.extend_from_slice(&color);
            }
        }
        Ok(rgba)
    }
}

impl TextureArchive {
    /// Resolve a source TextureInfo path (.tga) against its GF archive member.
    /// Match complete paths with Windows case/separator rules, never basenames.
    pub fn find_by_source_name(&self, name: &str) -> Option<&NamedTexture> {
        let mut path = name.replace('\\', "/").to_lowercase();
        if path.ends_with(".tga") {
            path.truncate(path.len() - 4);
            path.push_str(".gf");
        }
        self.textures
            .iter()
            .find(|entry| entry.path.to_lowercase() == path)
    }
}
