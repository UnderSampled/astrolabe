//! Hype/Montreal GF images and texture CNT archives.
//!
//! Evidence: Raymap GF.cs/CNT.cs; engine FUN_005b5d60 (26-byte GF header,
//! channel RLE), FUN_005b5820/005b5a10 (color-key fields). See docs/texture-rust.md.
use crate::model::texture::*;
use anyhow::{Context, Result, bail, ensure};
use binrw::{BinRead, BinWrite};
use std::collections::{HashMap, HashSet};
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

#[derive(BinRead, BinWrite, Debug)]
#[brw(little)]
struct CntHeader {
    directories: u32,
    files: u32,
    #[br(assert(xor <= 1))]
    xor: u8,
    #[br(assert(checksums <= 1))]
    checksums: u8,
    string_key: u8,
}

#[derive(BinRead, BinWrite, Clone, Debug)]
#[brw(little)]
struct CntLocation {
    xor_key: [u8; 4],
    checksum: u32,
    offset: u32,
    size: u32,
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

// Counts decoded pixel/palette storage for every owned logical member, including
// aliases. Kept separate from the per-image bound and encoded archive size.
const MAX_CNT_DECODED_BYTES: u64 = 512 * 1024 * 1024;

pub fn decode_cnt(bytes: &[u8], usage: TextureUsage) -> Result<TextureArchive> {
    decode_cnt_with_limit(bytes, usage, MAX_CNT_DECODED_BYTES)
}

fn decode_cnt_with_limit(bytes: &[u8], usage: TextureUsage, limit: u64) -> Result<TextureArchive> {
    let mut reader = Cursor::new(bytes);
    let h = CntHeader::read(&mut reader).context("CNT header (texture archives only)")?;
    ensure!(
        h.directories as usize <= bytes.len() / 4 && h.files as usize <= bytes.len() / 24,
        "CNT declared counts exceed file bounds"
    );
    let key = if h.xor == 1 { h.string_key } else { 0 };
    let mut directories = Vec::with_capacity(h.directories as usize);
    let mut checksum = 0u8;
    for _ in 0..h.directories {
        let raw = read_name(&mut reader, key)?;
        checksum = raw.iter().fold(checksum, |s, &b| s.wrapping_add(b));
        let directory = decode_name(&raw)?;
        validate_path(&directory)?;
        directories.push(directory);
    }
    let expected = u8::read(&mut reader).context("CNT directory checksum")?;
    if h.checksums == 1 {
        ensure!(checksum == expected, "CNT directory checksum mismatch");
    }
    let mut entries = Vec::with_capacity(h.files as usize);
    let mut paths = HashSet::new();
    for _ in 0..h.files {
        let directory = i32::read_le(&mut reader).context("CNT member directory")?;
        ensure!(
            directory >= -1 && directory < directories.len() as i32,
            "CNT directory index out of bounds"
        );
        let name = decode_name(&read_name(&mut reader, key)?)?;
        ensure!(
            !name.contains('/'),
            "CNT member name contains a directory separator"
        );
        validate_path(&name)?;
        let location = CntLocation::read(&mut reader).context("CNT member location")?;
        let path = if directory == -1 {
            name
        } else {
            format!("{}/{}", directories[directory as usize], name)
        };
        ensure!(
            paths.insert(path.to_lowercase()),
            "duplicate CNT member path {path}"
        );
        entries.push((path, location));
    }
    let table_end = reader.position();
    // Preflight the small, independently decrypted GF headers before allocating
    // any pixel arrays. A shared source range still produces an owned model value.
    let mut decoded_bytes = 0u64;
    for (path, location) in &entries {
        let start = u64::from(location.offset);
        let end = start + u64::from(location.size);
        ensure!(
            start >= table_end && end <= bytes.len() as u64,
            "CNT member {path} is outside archive data"
        );
        let source = &bytes[start as usize..end as usize];
        let mut header_bytes: [u8; 26] = source
            .get(..26)
            .context("CNT member has truncated GF header")?
            .try_into()?;
        let encrypted_length = source.len() / 4 * 4;
        for (i, byte) in header_bytes.iter_mut().enumerate() {
            if i < encrypted_length {
                *byte ^= location.xor_key[i % 4];
            }
        }
        let header = GfHeader::read(&mut Cursor::new(header_bytes))?;
        let pixel_bytes =
            u64::from(header.pixels_per_channel) * if header.color_type == 5 { 1 } else { 4 };
        let palette_bytes =
            u64::from(header.palette_entries) * u64::from(header.palette_components);
        decoded_bytes = decoded_bytes
            .checked_add(pixel_bytes + palette_bytes)
            .context("CNT decoded allocation overflow")?;
        ensure!(
            decoded_bytes <= limit,
            "CNT cumulative decoded allocation exceeds {limit} bytes at {path}"
        );
    }
    let mut ranges: Vec<(u64, u64)> = Vec::new();
    let mut textures = Vec::with_capacity(entries.len());
    let mut shared_ranges = HashMap::new();
    for (path, location) in entries {
        let start = u64::from(location.offset);
        let end = start + u64::from(location.size);
        ensure!(
            start >= table_end && end <= bytes.len() as u64,
            "CNT member {path} is outside archive data"
        );
        ranges.push((start, end));
        let mut data = bytes[start as usize..end as usize].to_vec();
        let encrypted_length = data.len() / 4 * 4;
        for (i, byte) in data[..encrypted_length].iter_mut().enumerate() {
            *byte ^= location.xor_key[i % 4];
        }
        if h.checksums == 1 {
            ensure!(
                checksum32(&data) == location.checksum,
                "CNT member checksum mismatch: {path}"
            );
        }
        let texture = decode_gf(&data, usage).with_context(|| format!("CNT member {path}"))?;
        let shared_with = if let Some((owner, xor, checksum)) = shared_ranges.get(&(start, end)) {
            ensure!(
                *xor == location.xor_key && *checksum == location.checksum,
                "CNT aliased data has incompatible encoding declarations"
            );
            Some(String::clone(owner))
        } else {
            shared_ranges.insert(
                (start, end),
                (path.clone(), location.xor_key, location.checksum),
            );
            None
        };
        textures.push(NamedTexture {
            path,
            texture,
            shared_with,
        });
    }
    ranges.sort_unstable();
    ensure!(
        ranges.windows(2).all(|w| w[0].1 <= w[1].0 || w[0] == w[1]),
        "CNT member data overlap"
    );
    Ok(TextureArchive {
        directories,
        textures,
    })
}

/// Rebuild a valid unencrypted CNT; offsets, sizes and checksums follow current content.
/// Original random XOR keys are deliberately not carried in the useful model.
pub fn encode_cnt(archive: &TextureArchive) -> Result<Vec<u8>> {
    let mut directories = archive.directories.clone();
    let mut paths = HashSet::new();
    let mut entries = Vec::new();
    for directory in &directories {
        validate_path(directory)?;
    }
    ensure!(
        directories.iter().collect::<HashSet<_>>().len() == directories.len(),
        "duplicate CNT directories"
    );
    for (member_index, named) in archive.textures.iter().enumerate() {
        validate_path(&named.path)?;
        ensure!(
            paths.insert(named.path.to_lowercase()),
            "duplicate CNT member path {}",
            named.path
        );
        let (dir, name) = if let Some((dir, name)) = named.path.rsplit_once('/') {
            let index = match directories.iter().position(|d| d == dir) {
                Some(i) => i,
                None => {
                    directories.push(dir.into());
                    directories.len() - 1
                }
            };
            (i32::try_from(index)?, name)
        } else {
            (-1, named.path.as_str())
        };
        let shared_index = if let Some(owner) = &named.shared_with {
            let index = archive.textures[..member_index]
                .iter()
                .position(|entry| &entry.path == owner)
                .ok_or_else(|| {
                    anyhow::anyhow!("shared CNT member must name a preceding member: {owner}")
                })?;
            ensure!(
                named.texture == archive.textures[index].texture,
                "shared CNT texture {} differs from {owner}; edit both or detach shared_with",
                named.path
            );
            Some(index)
        } else {
            None
        };
        let data = if shared_index.is_none() {
            encode_gf(&named.texture).with_context(|| format!("CNT member {}", named.path))?
        } else {
            Vec::new()
        };
        entries.push((dir, encode_name(name)?, data, shared_index));
    }
    let mut writer = Cursor::new(Vec::new());
    CntHeader {
        directories: directories.len().try_into()?,
        files: entries.len().try_into()?,
        xor: 0,
        checksums: 1,
        string_key: 0,
    }
    .write(&mut writer)?;
    let mut checksum = 0u8;
    for directory in &directories {
        let bytes = encode_name(&directory.replace('/', "\\"))?;
        checksum = bytes.iter().fold(checksum, |s, &b| s.wrapping_add(b));
        write_name(&mut writer, &bytes)?;
    }
    checksum.write(&mut writer)?;
    let table_end = writer.position()
        + entries
            .iter()
            .map(|(_, name, _, _)| 24 + name.len() as u64)
            .sum::<u64>();
    let mut offset = table_end;
    let mut locations: Vec<CntLocation> = Vec::with_capacity(entries.len());
    for (directory, name, data, shared) in &entries {
        let location = if let Some(owner) = shared {
            locations[*owner].clone()
        } else {
            let location = CntLocation {
                xor_key: [0; 4],
                checksum: checksum32(data),
                offset: offset.try_into()?,
                size: data.len().try_into()?,
            };
            offset += data.len() as u64;
            location
        };
        directory.write_le(&mut writer)?;
        write_name(&mut writer, name)?;
        location.write(&mut writer)?;
        locations.push(location);
    }
    let mut output = writer.into_inner();
    for (_, _, mut data, _) in entries {
        output.append(&mut data);
    }
    Ok(output)
}

/// Hype CNT checksum sums decrypted little-endian words, ignoring a short tail.
pub fn checksum32(bytes: &[u8]) -> u32 {
    bytes.as_chunks::<4>().0.iter().fold(0u32, |sum, word| {
        sum.wrapping_add(u32::from_le_bytes(*word))
    })
}

fn read_name(reader: &mut Cursor<&[u8]>, key: u8) -> Result<Vec<u8>> {
    let length = u32::read_le(reader).context("CNT string length")? as usize;
    ensure!(
        length > 0 && length <= 4096,
        "CNT string length outside 1..=4096"
    );
    let mut bytes = vec![0u8; length];
    reader.read_exact(&mut bytes).context("CNT string")?;
    for byte in &mut bytes {
        *byte ^= key;
    }
    Ok(bytes)
}
fn write_name(writer: &mut Cursor<Vec<u8>>, bytes: &[u8]) -> Result<()> {
    u32::try_from(bytes.len())?.write_le(writer)?;
    use std::io::Write;
    writer.write_all(bytes)?;
    Ok(())
}

// CP1252 paths are byte strings in the engine. Undefined slots map reversibly to
// corresponding controls instead of silently replacing bytes with U+FFFD.
const CP1252: [char; 32] = [
    '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž', '\u{8f}',
    '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}', 'ž', 'Ÿ',
];
fn decode_name(bytes: &[u8]) -> Result<String> {
    ensure!(!bytes.contains(&0), "CNT name contains a NUL");
    Ok(bytes
        .iter()
        .map(|&b| {
            if (0x80..=0x9f).contains(&b) {
                CP1252[(b - 0x80) as usize]
            } else {
                b as char
            }
        })
        .collect::<String>()
        .replace('\\', "/"))
}
fn encode_name(name: &str) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    for ch in name.chars() {
        let value = ch as u32;
        if value < 128 || (160..=255).contains(&value) {
            bytes.push(value as u8);
        } else if let Some(index) = CP1252.iter().position(|&c| c == ch) {
            bytes.push(index as u8 + 0x80);
        } else {
            bail!("CNT path cannot be represented in CP1252: {name}");
        }
    }
    ensure!(
        !bytes.is_empty() && bytes.len() <= 4096 && !bytes.contains(&0),
        "invalid CNT name length/content"
    );
    Ok(bytes)
}
fn validate_path(path: &str) -> Result<()> {
    ensure!(
        !path.is_empty()
            && !path.contains(['\\', ':', '\0'])
            && path
                .split('/')
                .all(|p| !p.is_empty() && p != "." && p != ".."),
        "unsafe/invalid relative texture path {path:?}"
    );
    Ok(())
}

#[cfg(test)]
mod allocation_tests {
    use super::*;

    #[test]
    fn shared_cnt_ranges_count_every_owned_image_before_decode() {
        let texture = Texture {
            usage: TextureUsage::Material,
            colors: TextureColors::Components { bits: [5, 6, 5, 0] },
            levels: vec![TextureLevel {
                width: 1,
                height: 1,
                pixels: TexturePixels::Components(vec![[31, 0, 0, 0]]),
            }],
            openspace_color_key: OpenSpaceColorKey {
                rgb: [0; 3],
                palette_index: -1,
            },
        };
        let archive = TextureArchive {
            directories: vec![],
            textures: (0..3)
                .map(|i| NamedTexture {
                    path: format!("{i}.gf"),
                    texture: texture.clone(),
                    shared_with: (i != 0).then(|| "0.gf".into()),
                })
                .collect(),
        };
        let source = encode_cnt(&archive).unwrap();
        assert_eq!(
            decode_cnt_with_limit(&source, TextureUsage::Material, 12).unwrap(),
            archive
        );
        let error = decode_cnt_with_limit(&source, TextureUsage::Material, 8).unwrap_err();
        assert!(
            error.to_string().contains("cumulative decoded allocation"),
            "{error:#}"
        );
        // Corrupt the pixel payload. A small budget must reject the archive in
        // preflight, before checksum/RLE/pixel decoding reaches the corruption.
        let mut invalid = source;
        *invalid.last_mut().unwrap() ^= 0x7f;
        let error = decode_cnt_with_limit(&invalid, TextureUsage::Material, 8).unwrap_err();
        assert!(
            error.to_string().contains("cumulative decoded allocation"),
            "{error:#}"
        );
    }
}
