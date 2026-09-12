//! CNT membership, aliases, name encoding, and regenerated archive placement.
//! Evidence: Raymap CNT.cs and the normalized local corpus (docs/texture-rust.md).
use super::gf::{decode_gf, encode_gf};
use crate::model::{
    AssetPath, AssetRef,
    io::{Content, Load},
    texture::*,
};
use crate::{model::io::FormatSupport, openspace::codec::FormatCodec};
use anyhow::{Context, Result, bail, ensure};
use binrw::{BinRead, BinWrite};
use std::io::{Cursor, Read};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};
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

pub struct Cnt {
    pub usage: TextureUsage,
}
impl FormatCodec for Cnt {
    type Value = TextureArchive;
    fn support(&self) -> FormatSupport {
        FormatSupport {
            format: "CNT",
            reads: true,
            writes: true,
            notes: "Texture archives; membership/order/sharing retained, regenerated output is unencrypted",
        }
    }
    fn read(&self, bytes: &[u8]) -> Result<TextureArchive> {
        decode_cnt(bytes, self.usage)
    }
    fn write(&self, value: &TextureArchive) -> Result<Vec<u8>> {
        ensure!(
            value
                .images
                .values()
                .all(|texture| texture.usage == self.usage),
            "CNT texture usage differs from codec context"
        );
        encode_cnt(value)
    }
}
const MAX_CNT_DECODED_BYTES: u64 = 512 * 1024 * 1024;

/// An imported archive's dependency closure. The root is the same TextureSet
/// persisted by Rete; each image has exactly one value regardless of alias count.
#[derive(Debug, Clone, PartialEq)]
pub struct TextureArchive {
    pub set: TextureSet,
    pub images: BTreeMap<AssetPath, Texture>,
}
impl TextureArchive {
    pub fn image(&self, entry: &TextureEntry) -> Result<&Texture> {
        self.images
            .get(&entry.texture.path)
            .with_context(|| format!("missing archive image {}", entry.texture.path))
    }
    pub fn find_image(&self, source_name: &str) -> Option<&Texture> {
        let entry = self.find_entry(source_name)?;
        self.images.get(&entry.texture.path)
    }
}

impl TextureArchive {
    /// Match the source's complete Windows path, including its authored .tga spelling.
    pub fn find_entry(&self, name: &str) -> Option<&TextureEntry> {
        let mut path = name.replace('\\', "/").to_lowercase();
        if path.ends_with(".tga") {
            path.truncate(path.len() - 4);
            path.push_str(".gf");
        }
        self.set
            .entries
            .iter()
            .find(|entry| entry.name.to_lowercase() == path)
    }
}

impl Load<Texture> for TextureArchive {
    fn load(&mut self, reference: &AssetRef<Texture>) -> Result<Arc<Texture>> {
        Ok(Arc::new(
            self.images
                .get(&reference.path)
                .with_context(|| format!("missing archive image {}", reference.path))?
                .clone(),
        ))
    }
}

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
    // any pixel arrays. Count every logical member conservatively, including aliases.
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
        decoded_bytes = decoded_bytes
            .checked_add(super::gf::decoded_size(&header_bytes)?)
            .context("CNT decoded allocation overflow")?;
        ensure!(
            decoded_bytes <= limit,
            "CNT cumulative decoded allocation exceeds {limit} bytes at {path}"
        );
    }
    let mut ranges: Vec<(u64, u64)> = Vec::new();
    let mut members = Vec::with_capacity(entries.len());
    let mut images = BTreeMap::new();
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
        let reference = if let Some((owner, xor, checksum)) = shared_ranges.get(&(start, end)) {
            ensure!(
                *xor == location.xor_key && *checksum == location.checksum,
                "CNT aliased data has incompatible encoding declarations"
            );
            AssetRef::from_path(AssetPath::clone(owner))
        } else {
            let reference = AssetRef::new(format!("texture:/{path}"))?;
            let texture = decode_gf(&data, usage).with_context(|| format!("CNT member {path}"))?;
            images.insert(reference.path.clone(), texture);
            shared_ranges.insert(
                (start, end),
                (reference.path.clone(), location.xor_key, location.checksum),
            );
            reference
        };
        members.push(TextureEntry {
            name: path,
            texture: reference,
        });
    }
    ranges.sort_unstable();
    ensure!(
        ranges.windows(2).all(|w| w[0].1 <= w[1].0 || w[0] == w[1]),
        "CNT member data overlap"
    );
    Ok(TextureArchive {
        set: TextureSet {
            name: match usage {
                TextureUsage::Material => "Textures.cnt",
                TextureUsage::Screen => "Vignette.cnt",
            }
            .into(),
            directories,
            entries: members,
        },
        images,
    })
}

/// Rebuild a valid unencrypted CNT; offsets, sizes and checksums follow current content.
/// Original random XOR keys are deliberately not carried in the useful model.
pub fn encode_cnt(archive: &TextureArchive) -> Result<Vec<u8>> {
    encode_members(&archive.set, |entry| {
        encode_image(&archive.set, archive.image(entry)?)
    })
}

/// Reconstruct archive membership directly from the canonical root and resolver.
pub fn write_cnt(set: &TextureSet, resolver: &mut impl Load<Texture>) -> Result<Vec<u8>> {
    encode_members(set, |entry| {
        let texture = resolver.load(&entry.texture)?;
        encode_image(set, &texture)
    })
}

fn encode_image(set: &TextureSet, texture: &Texture) -> Result<Vec<u8>> {
    let usage = if set.name == "Textures.cnt" {
        TextureUsage::Material
    } else {
        TextureUsage::Screen
    };
    ensure!(
        texture.usage == usage,
        "CNT texture usage differs from archive context"
    );
    encode_gf(texture)
}

fn encode_members(
    set: &TextureSet,
    mut image: impl FnMut(&TextureEntry) -> Result<Vec<u8>>,
) -> Result<Vec<u8>> {
    set.validate()?;
    let mut directories = set.directories.clone();
    let mut first = BTreeMap::new();
    let mut paths = HashSet::new();
    let mut entries = Vec::new();
    for directory in &directories {
        validate_path(directory)?;
    }
    ensure!(
        directories.iter().collect::<HashSet<_>>().len() == directories.len(),
        "duplicate CNT directories"
    );
    for (member_index, named) in set.entries.iter().enumerate() {
        validate_path(&named.name)?;
        ensure!(
            paths.insert(named.name.to_lowercase()),
            "duplicate CNT member path {}",
            named.name
        );
        let (dir, name) = if let Some((dir, name)) = named.name.rsplit_once('/') {
            let index = match directories.iter().position(|d| d == dir) {
                Some(i) => i,
                None => {
                    directories.push(dir.into());
                    directories.len() - 1
                }
            };
            (i32::try_from(index)?, name)
        } else {
            (-1, named.name.as_str())
        };
        let shared_index = first.get(&named.texture.path).copied();
        first
            .entry(named.texture.path.clone())
            .or_insert(member_index);
        let data = if shared_index.is_none() {
            image(named)?
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
    fn cnt_preflight_bounds_logical_member_size_before_pixel_decode() {
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
        let reference = AssetRef::new("texture:/0.gf").unwrap();
        let archive = TextureArchive {
            set: TextureSet {
                name: "Textures.cnt".into(),
                directories: vec![],
                entries: (0..3)
                    .map(|i| TextureEntry {
                        name: format!("{i}.gf"),
                        texture: reference.clone(),
                    })
                    .collect(),
            },
            images: BTreeMap::from([(reference.path, texture)]),
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
