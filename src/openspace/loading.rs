//! `<level>pgb.bin` loading-vignette sections. 00402b60 falls back to this
//! binary form of the level's Vignette script and 0050ef30 reads it; the text
//! exporter 0050f370 names the same fields (`LoadLevelVignette`,
//! `SetBitmapBarMode`, `SetBitmapBarPosition`, `InitBar*Color`, `CreateBar`,
//! `MaxValueBar`). See notes/research/loading-vignette-2026-11-04.md.
use crate::model::{
    AssetRef,
    io::FormatSupport,
    loading::{LoadingBarArt, LoadingVignette},
};
use crate::openspace::codec::FormatCodec;
use anyhow::{Context, Result, ensure};
use binrw::{BinRead, BinWrite, binrw};
use std::{io::Cursor, path::Path};

/// 0050ef30 decodes each name byte as `stored + step*i - offset` (wrapping).
fn decode(bytes: &[u8], step: u8, offset: u8) -> Vec<u8> {
    (0..bytes.len())
        .map(|i| {
            bytes[i]
                .wrapping_add(step.wrapping_mul(i as u8))
                .wrapping_sub(offset)
        })
        .collect()
}
fn encode(bytes: &[u8], step: u8, offset: u8) -> Vec<u8> {
    (0..bytes.len())
        .map(|i| {
            bytes[i]
                .wrapping_add(offset)
                .wrapping_sub(step.wrapping_mul(i as u8))
        })
        .collect()
}

#[binrw]
#[brw(little)]
struct ProgressBarWire {
    #[br(temp)]
    #[bw(calc = image.len() as i32)]
    image_len: i32,
    #[br(count = image_len, map = |b: Vec<u8>| decode(&b, 3, 7))]
    #[bw(map = |b: &Vec<u8>| encode(b, 3, 7))]
    image: Vec<u8>,
    /// Zero length selects the drawn default bar.
    #[br(temp)]
    #[bw(calc = bar.len() as i32)]
    bar_len: i32,
    #[br(count = bar_len, map = |b: Vec<u8>| decode(&b, 6, 11))]
    #[bw(map = |b: &Vec<u8>| encode(b, 6, 11))]
    bar: Vec<u8>,
    frame_y: i32,
    fill_y: i32,
    min_x: i32,
    max_x: i32,
    outline_color: [f32; 4],
    inside_color: [f32; 4],
    bar_colors: [[f32; 4]; 4],
    bar_rect: [i32; 4],
    max_value: i32,
}

fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| char::from(b)).collect()
}
fn bytes(text: &str) -> Result<Vec<u8>> {
    text.chars()
        .map(|c| u8::try_from(u32::from(c)).context("loading vignette name is not Latin-1"))
        .collect()
}
/// 0050e310 replaces the stored extension and looks the image up in Vignette.cnt.
pub fn texture_reference(name: &str) -> Result<AssetRef<crate::model::texture::Texture>> {
    let gf = Path::new(&name.replace('\\', "/")).with_extension("gf");
    AssetRef::new(format!("texture:/{}", gf.to_string_lossy()))
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProgressBarScript;

impl FormatCodec for ProgressBarScript {
    type Value = LoadingVignette;
    fn support(&self) -> FormatSupport {
        FormatSupport {
            format: "PGB.BIN",
            reads: true,
            writes: true,
            notes: "Level loading vignette and progress bar; image names bind to Vignette.cnt",
        }
    }
    fn read(&self, data: &[u8]) -> Result<LoadingVignette> {
        let mut cursor = Cursor::new(data);
        let wire = ProgressBarWire::read(&mut cursor).context("read loading vignette")?;
        ensure!(
            cursor.position() == data.len() as u64,
            "trailing data after loading vignette"
        );
        let image_name = latin1(&wire.image);
        let bar = if wire.bar.is_empty() {
            None
        } else {
            let name = latin1(&wire.bar);
            Some(LoadingBarArt {
                image: texture_reference(&name)?,
                name,
            })
        };
        let value = LoadingVignette {
            image: texture_reference(&image_name)?,
            image_name,
            bar,
            frame_y: wire.frame_y,
            fill_y: wire.fill_y,
            min_x: wire.min_x,
            max_x: wire.max_x,
            outline_color: wire.outline_color,
            inside_color: wire.inside_color,
            bar_colors: wire.bar_colors,
            bar_rect: wire.bar_rect,
            max_value: wire.max_value,
        };
        value.validate()?;
        Ok(value)
    }
    fn write(&self, value: &LoadingVignette) -> Result<Vec<u8>> {
        value.validate()?;
        let wire = ProgressBarWire {
            image: bytes(&value.image_name)?,
            bar: value
                .bar
                .as_ref()
                .map_or(Ok(Vec::new()), |bar| bytes(&bar.name))?,
            frame_y: value.frame_y,
            fill_y: value.fill_y,
            min_x: value.min_x,
            max_x: value.max_x,
            outline_color: value.outline_color,
            inside_color: value.inside_color,
            bar_colors: value.bar_colors,
            bar_rect: value.bar_rect,
            max_value: value.max_value,
        };
        let mut out = Cursor::new(Vec::new());
        wire.write(&mut out)?;
        Ok(out.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic bytes in the manoir layout: names, geometry, colours, rect, max.
    fn sample() -> Vec<u8> {
        let mut out = Vec::new();
        let image = encode(b"epoque1.bmp", 3, 7);
        out.extend((image.len() as i32).to_le_bytes());
        out.extend(&image);
        let bar = encode(b"bar.bmp", 6, 11);
        out.extend((bar.len() as i32).to_le_bytes());
        out.extend(&bar);
        for v in [0i32, 60, 50, 461] {
            out.extend(v.to_le_bytes());
        }
        for i in 0..24 {
            out.extend(if i % 4 == 3 { 1.0f32 } else { 0.0 }.to_le_bytes());
        }
        for v in [110i32, 410, 621, 469, 50] {
            out.extend(v.to_le_bytes());
        }
        out
    }

    #[test]
    fn manoir_name_rule_matches_source_bytes() {
        // First bytes of the installed manoirpgb.bin image name.
        assert_eq!(decode(b"ltpop]& QYY", 3, 7), b"epoque1.bmp".to_vec());
    }

    #[test]
    fn reads_meaning_and_rebuilds_bytes() {
        let bytes = sample();
        let value = ProgressBarScript.read(&bytes).unwrap();
        assert_eq!(value.image_name, "epoque1.bmp");
        assert_eq!(value.image.path.as_str(), "texture:/epoque1.gf");
        assert_eq!(
            value.bar.as_ref().unwrap().image.path.as_str(),
            "texture:/bar.gf"
        );
        assert_eq!(value.bar_art_rect(), [60, 410, 512, 60]);
        assert_eq!((value.min_x, value.max_x, value.max_value), (50, 461, 50));
        assert_eq!(ProgressBarScript.write(&value).unwrap(), bytes);
    }

    #[test]
    #[ignore = "requires local Hype game files"]
    fn installed_level_vignettes_rebuild_and_select_era_art() -> Result<()> {
        let game = std::env::var_os("ASTROLABE_GAME_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("disc"));
        let levels = game.join("Gamedata/World/Levels");
        let mut checked = 0;
        for entry in std::fs::read_dir(&levels)? {
            let level = entry?.file_name().to_string_lossy().into_owned();
            let path = levels.join(&level).join(format!("{level}pgb.bin"));
            if !path.is_file() {
                continue;
            }
            let bytes = std::fs::read(&path)?;
            let value = ProgressBarScript.read(&bytes)?;
            ensure!(
                ProgressBarScript.write(&value)? == bytes,
                "{level} bytes differ"
            );
            ensure!(
                value.bar.as_ref().map(|b| b.name.to_lowercase()) == Some("bar.bmp".into())
                    && value.bar_art_rect() == [60, 410, 512, 60]
                    && (value.frame_y, value.fill_y, value.min_x, value.max_x) == (0, 60, 50, 461),
                "{level} bar layout differs"
            );
            if ["manoir", "brigand", "toras"].contains(&level.as_str()) {
                ensure!(
                    value.image_name.eq_ignore_ascii_case("epoque1.bmp"),
                    "{level} uses {}",
                    value.image_name
                );
            }
            checked += 1;
        }
        ensure!(checked == 37, "only {checked} level vignettes found");
        Ok(())
    }

    #[test]
    fn rejects_trailing_bytes() {
        let mut bytes = sample();
        bytes.push(0);
        assert!(ProgressBarScript.read(&bytes).is_err());
    }
}
