//! Montreal GF header, palette/component mapping, mip chains, and channel RLE.
//! Evidence: Raymap GF.cs; engine FUN_005b5d60 and FUN_005b5820/005b5a10.
use crate::model::texture::*;
use crate::{model::io::FormatSupport, openspace::codec::FormatCodec};
use anyhow::{Context, Result, bail, ensure};
use binrw::{BinRead, BinWrite};
use std::io::{Cursor, Read};
#[derive(BinRead, BinWrite, Debug)]
#[brw(little)]
struct GfHeader {
    #[br(assert(version == 1, "unsupported Montreal GF version"))]
    version: u8,
    width: u32,
    height: u32,
    channels: u8,
    repeat: u8,
    palette_entries: u16,
    palette_components: u8,
    color_key_rgb: [u8; 3],
    color_key_index: i32,
    pixels_per_channel: u32,
    color_type: u8,
}

pub struct Gf {
    pub usage: TextureUsage,
}
impl FormatCodec for Gf {
    type Value = Texture;
    fn support(&self) -> FormatSupport {
        FormatSupport {
            format: "GF",
            reads: true,
            writes: true,
            notes: "Hype palette/components, mipmaps and exact observed GF reconstruction",
        }
    }
    fn read(&self, bytes: &[u8]) -> Result<Texture> {
        decode_gf(bytes, self.usage)
    }
    fn write(&self, value: &Texture) -> Result<Vec<u8>> {
        ensure!(
            value.usage == self.usage,
            "GF usage differs from codec context"
        );
        encode_gf(value)
    }
}

/// Conservative decoded sample storage used by archive allocation preflight.
pub(super) fn decoded_size(header: &[u8]) -> Result<u64> {
    let header = GfHeader::read(&mut Cursor::new(header))?;
    let pixel_bytes =
        u64::from(header.pixels_per_channel) * if header.color_type == 5 { 1 } else { 4 };
    let palette_bytes = u64::from(header.palette_entries) * u64::from(header.palette_components);
    Ok(pixel_bytes + palette_bytes)
}

pub fn decode_gf(bytes: &[u8], usage: TextureUsage) -> Result<Texture> {
    let mut reader = Cursor::new(bytes);
    let h = GfHeader::read(&mut reader).context("GF header")?;
    ensure!(h.width > 0 && h.height > 0, "GF has zero dimensions");
    ensure!(
        h.pixels_per_channel <= 22_369_640,
        "GF exceeds 16M-base-pixel safety limit"
    );
    ensure!(
        u64::from(h.width) * u64::from(h.height) <= u64::from(h.pixels_per_channel),
        "GF pixel count is smaller than base image"
    );
    let colors = if h.color_type == 5 {
        ensure!(
            h.channels == 1 && h.palette_entries > 0 && h.palette_entries <= 256,
            "GF palette/channel declaration invalid"
        );
        let mut palette = vec![0u8; h.palette_entries as usize * h.palette_components as usize];
        reader.read_exact(&mut palette).context("GF palette")?;
        match h.palette_components {
            3 => TextureColors::PaletteRgb(
                palette
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|p| [p[2], p[1], p[0]])
                    .collect(),
            ),
            4 => TextureColors::PaletteRgba(
                palette
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|p| [p[2], p[1], p[0], p[3]])
                    .collect(),
            ),
            n => bail!("unsupported GF palette component count {n}"),
        }
    } else {
        ensure!(
            h.channels == 2 && h.palette_entries == 0 && h.palette_components == 0,
            "GF direct-color declaration invalid"
        );
        let bits = match h.color_type {
            10 => [5, 6, 5, 0],
            11 => [5, 5, 5, 1],
            12 => [4, 4, 4, 4],
            n => bail!("unsupported GF color type {n}"),
        };
        TextureColors::Components { bits }
    };
    let count = h.pixels_per_channel as usize;
    let mut channels = Vec::with_capacity(h.channels as usize);
    for channel in 0..h.channels {
        let mut values = Vec::with_capacity(count);
        while values.len() < count {
            let token =
                u8::read(&mut reader).with_context(|| format!("GF channel {channel} truncated"))?;
            if token == h.repeat {
                let value = u8::read(&mut reader).context("GF RLE value")?;
                let length = u8::read(&mut reader).context("GF RLE length")? as usize;
                ensure!(length > 0, "GF RLE zero-length run");
                ensure!(values.len() + length <= count, "GF RLE run exceeds channel");
                values.resize(values.len() + length, value);
            } else {
                values.push(token);
            }
        }
        channels.push(values);
    }
    ensure!(
        reader.position() == bytes.len() as u64,
        "GF has trailing data"
    );
    let mut levels = Vec::new();
    let (mut width, mut height, mut offset) = (h.width, h.height, 0usize);
    while offset < count {
        let n = (u64::from(width) * u64::from(height)) as usize;
        ensure!(
            offset.checked_add(n).is_some_and(|end| end <= count),
            "GF has incomplete mip level"
        );
        let pixels = match &colors {
            TextureColors::PaletteRgb(_) | TextureColors::PaletteRgba(_) => {
                TexturePixels::Indices(channels[0][offset..offset + n].to_vec())
            }
            TextureColors::Components { .. } => TexturePixels::Components(
                (offset..offset + n)
                    .map(|i| {
                        let word = u16::from(channels[0][i]) | (u16::from(channels[1][i]) << 8);
                        match h.color_type {
                            10 => {
                                let high = (word >> 11) as u8;
                                let low = (word & 31) as u8;
                                if usage == TextureUsage::Screen {
                                    [low, ((word >> 5) & 63) as u8, high, 0]
                                } else {
                                    [high, ((word >> 5) & 63) as u8, low, 0]
                                }
                            }
                            11 => [
                                ((word >> 10) & 31) as u8,
                                ((word >> 5) & 31) as u8,
                                (word & 31) as u8,
                                (word >> 15) as u8,
                            ],
                            12 => [
                                ((word >> 8) & 15) as u8,
                                ((word >> 4) & 15) as u8,
                                (word & 15) as u8,
                                (word >> 12) as u8,
                            ],
                            _ => unreachable!(),
                        }
                    })
                    .collect(),
            ),
        };
        levels.push(TextureLevel {
            width,
            height,
            pixels,
        });
        offset += n;
        ensure!(
            offset == count || (width, height) != (1, 1),
            "GF repeats terminal mip level"
        );
        width = (width / 2).max(1);
        height = (height / 2).max(1);
    }
    let texture = Texture {
        usage,
        colors,
        levels,
        openspace_color_key: OpenSpaceColorKey {
            rgb: h.color_key_rgb,
            palette_index: h.color_key_index,
        },
    };
    texture.validate()?;
    Ok(texture)
}

/// Re-derive the GF packing, channel count, histogram-selected RLE escape and runs.
/// No original RLE stream or escape-byte hint is used.
pub fn encode_gf(texture: &Texture) -> Result<Vec<u8>> {
    texture.validate()?;
    let count: usize = texture
        .levels
        .iter()
        .map(|l| l.width as usize * l.height as usize)
        .sum();
    let (kind, mut palette, entries, components, mut channels) = match &texture.colors {
        TextureColors::PaletteRgb(p) => (
            5,
            p.iter()
                .flat_map(|[r, g, b]| [*b, *g, *r])
                .collect::<Vec<_>>(),
            p.len() as u16,
            3,
            vec![Vec::with_capacity(count)],
        ),
        TextureColors::PaletteRgba(p) => (
            5,
            p.iter()
                .flat_map(|[r, g, b, a]| [*b, *g, *r, *a])
                .collect::<Vec<_>>(),
            p.len() as u16,
            4,
            vec![Vec::with_capacity(count)],
        ),
        TextureColors::Components { bits } => {
            let kind = match bits {
                [5, 6, 5, 0] => 10,
                [5, 5, 5, 1] => 11,
                [4, 4, 4, 4] => 12,
                _ => bail!("GF cannot exactly represent component precision {bits:?}"),
            };
            (
                kind,
                Vec::new(),
                0,
                0,
                vec![Vec::with_capacity(count), Vec::with_capacity(count)],
            )
        }
    };
    for level in &texture.levels {
        match &level.pixels {
            TexturePixels::Indices(v) => channels[0].extend_from_slice(v),
            TexturePixels::Components(v) => {
                for &[r, g, b, a] in v {
                    let (r, g, b, a) = (u16::from(r), u16::from(g), u16::from(b), u16::from(a));
                    let word = match kind {
                        10 if texture.usage == TextureUsage::Screen => b << 11 | g << 5 | r,
                        10 => r << 11 | g << 5 | b,
                        11 => a << 15 | r << 10 | g << 5 | b,
                        12 => a << 12 | r << 8 | g << 4 | b,
                        _ => unreachable!(),
                    };
                    channels[0].push(word as u8);
                    channels[1].push((word >> 8) as u8);
                }
            }
        }
    }
    let mut histogram = [0usize; 256];
    for &value in channels.iter().flatten() {
        histogram[value as usize] += 1;
    }
    let repeat = (0..256).min_by_key(|&i| histogram[i]).unwrap() as u8;
    let header = GfHeader {
        version: 1,
        width: texture.width(),
        height: texture.height(),
        channels: channels.len() as u8,
        repeat,
        palette_entries: entries,
        palette_components: components,
        color_key_rgb: texture.openspace_color_key.rgb,
        color_key_index: texture.openspace_color_key.palette_index,
        pixels_per_channel: count.try_into()?,
        color_type: kind,
    };
    let mut writer = Cursor::new(Vec::new());
    header.write(&mut writer)?;
    let mut output = writer.into_inner();
    output.append(&mut palette);
    for channel in channels {
        encode_rle(&channel, repeat, &mut output);
    }
    Ok(output)
}

fn encode_rle(values: &[u8], repeat: u8, output: &mut Vec<u8>) {
    let mut p = 0;
    while p < values.len() {
        let value = values[p];
        let mut run = 1usize;
        // The historical encoder leaves the final channel sample out of a
        // preceding run. All supplied GF files and the known-byte fixture
        // establish this rule; no original token boundaries are retained.
        while run < 255 && p + run < values.len().saturating_sub(1) && values[p + run] == value {
            run += 1;
        }
        if run >= 4 || value == repeat {
            output.extend_from_slice(&[repeat, value, run as u8]);
        } else {
            output.extend_from_slice(&values[p..p + run]);
        }
        p += run;
    }
}
