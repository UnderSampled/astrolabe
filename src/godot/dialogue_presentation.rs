//! Source bitmap metrics become a native FontFile, not a runtime font interpreter.
use super::{Project, disk_path, quoted};
use crate::model::{AssetPath, AssetRef, content::ContentSource, dialogue::*, io::Content};
use anyhow::{Result, ensure};
use std::{collections::BTreeSet, fmt::Write as _};
const SCALE: f32 = 640.0 / (2.0 * 4.65);

fn image(
    project: &mut Project,
    source: &mut impl ContentSource,
    texture: &DialogueTexture,
    suffix: &str,
) -> Result<(String, Vec<u8>)> {
    let reference = AssetRef::<crate::model::texture::Texture>::from_path(texture.texture.clone());
    let value = source.load(&reference)?;
    project.retain_texture_mips(&reference, &value)?;
    ensure!(
        [value.width(), value.height()] == texture.size,
        "dialogue image dimensions differ"
    );
    let mut rgba = value.rgba8(0)?;
    if texture.color_key {
        rgba = super::material::keyed_rgba(&value, &rgba);
    }
    let path = disk_path(&project.selection, &texture.texture, suffix);
    project.write(
        path.clone(),
        &super::material::png(value.width(), value.height(), &rgba)?,
    )?;
    Ok((path, rgba))
}
pub(super) fn export(
    project: &mut Project,
    source: &mut impl ContentSource,
    value: &DialoguePresentation,
    written: &mut BTreeSet<AssetPath>,
) -> Result<String> {
    value.validate()?;
    let path = disk_path(&project.selection, &value.path, "tres");
    if !written.insert(value.path.clone()) {
        return Ok(format!("res://{path}"));
    }
    let font_path = disk_path(&project.selection, &value.path, "font.tres");
    project.reserve(&font_path)?;
    let (atlas, rgba) = image(project, source, &value.atlas, "dialogue.png")?;
    let (outline_rgba, outline_height, rects) = outline(value, &rgba)?;
    let outline_path = disk_path(&project.selection, &value.path, "outline.png");
    project.write(
        outline_path.clone(),
        &super::material::png(1024, outline_height, &outline_rgba)?,
    )?;
    super::bitmap_font::emit(
        project,
        super::bitmap_font::FontInput {
            name: &value.font_name,
            atlas: &format!("res://{atlas}"),
            atlas_size: value.atlas.size,
            glyphs: &value.glyphs,
            tracking: value.tracking,
            scale: SCALE,
            path: &format!("res://{font_path}"),
            outline: Some((&format!("res://{outline_path}"), &rects)),
        },
    )?;
    let mut text = format!(
        "[gd_resource type=\"Resource\" load_steps=9 format=3]\n[ext_resource type=\"Script\" path=\"res://addons/astrolabe/ui/dialogue_presentation.gd\" id=\"script\"]\n[ext_resource type=\"FontFile\" path={} id=\"font\"]\n",
        quoted(&format!("res://{font_path}"))
    );
    for (texture, name) in value
        .frame
        .iter()
        .zip(["corner", "border", "center", "side"])
    {
        let (png, _) = image(project, source, texture, "dialogue.png")?;
        writeln!(
            text,
            "[ext_resource type=\"Texture2D\" path={} id=\"{name}\"]",
            quoted(&format!("res://{png}"))
        )?;
    }
    let (continuation, _) = image(project, source, &value.continuation, "dialogue.png")?;
    writeln!(
        text,
        "[ext_resource type=\"Texture2D\" path={} id=\"continuation\"]",
        quoted(&format!("res://{continuation}"))
    )?;
    let (bullet, _) = image(project, source, &value.bullet, "dialogue.png")?;
    writeln!(
        text,
        "[ext_resource type=\"Texture2D\" path={} id=\"bullet\"]",
        quoted(&format!("res://{bullet}"))
    )?;
    let mut sounds = String::new();
    for (sound, name) in [
        (&value.cash_sound, "cash_sound"),
        (&value.scroll_sound, "scroll_sound"),
    ] {
        if let Some(link) = sound {
            let event = super::sound::export_event(project, source, link, written)?;
            writeln!(
                text,
                "[ext_resource type=\"Resource\" path={} id=\"{name}\"]",
                quoted(&event)
            )?;
            writeln!(sounds, "{name} = ExtResource(\"{name}\")")?;
        }
    }
    writeln!(
        text,
        "\n[resource]\nscript = ExtResource(\"script\")\nfont = ExtResource(\"font\")\nnormal_color = Color({},{},{},{})\nemphasis_color = Color({},{},{},{})\ncorner = ExtResource(\"corner\")\nborder = ExtResource(\"border\")\ncenter = ExtResource(\"center\")\nside = ExtResource(\"side\")\ncontinuation = ExtResource(\"continuation\")\nselection_color = Color({},{},{},{})\nbullet = ExtResource(\"bullet\")",
        value.normal_color[0],
        value.normal_color[1],
        value.normal_color[2],
        value.normal_color[3],
        value.emphasis_color[0],
        value.emphasis_color[1],
        value.emphasis_color[2],
        value.emphasis_color[3],
        value.selection_color[0],
        value.selection_color[1],
        value.selection_color[2],
        value.selection_color[3]
    )?;
    text.push_str(&sounds);
    project.write(path.clone(), text.as_bytes())?;
    Ok(format!("res://{path}"))
}

/// An export-time coverage atlas supplies FontFile's native outline cache. Four
/// diagonal shifts reproduce the source's red emphasis footprint without custom
/// runtime drawing. 4 samples/pixel retain subpixel geometry; final pixel parity
/// still requires the original New Game captures on the pinned renderer.
fn outline(value: &DialoguePresentation, rgba: &[u8]) -> Result<(Vec<u8>, u32, Vec<[f32; 4]>)> {
    let mut rects = vec![[0.0; 4]; value.glyphs.len()];
    let mut x = 1u32;
    let mut y = 1u32;
    let mut row_height = 0u32;
    let pad = SCALE * 0.01;
    for (g, rect) in value.glyphs.iter().zip(&mut rects) {
        if g.uv.is_none() {
            continue;
        }
        let width = ((g.width * SCALE + 2.0 * pad) * 4.0).ceil() as u32;
        let height = ((-2.0 * g.half_height * SCALE + 2.0 * pad) * 4.0).ceil() as u32;
        ensure!(
            width > 0 && height > 0 && width < 1022 && height < 4096,
            "outline glyph exceeds bounds"
        );
        if x + width + 1 > 1024 {
            x = 1;
            y += row_height + 2;
            row_height = 0;
        }
        *rect = [x as f32, y as f32, width as f32, height as f32];
        x += width + 2;
        row_height = row_height.max(height);
    }
    let height = y + row_height + 1;
    ensure!(height <= 4096, "outline atlas exceeds bounds");
    let mut pixels = vec![0u8; 1024 * height as usize * 4];
    for (g, rect) in value.glyphs.iter().zip(&rects) {
        let Some([lo, ext]) = g.uv else { continue };
        let w = g.width * SCALE;
        let h = -2.0 * g.half_height * SCALE;
        let iw = value.atlas.size[0];
        let ih = value.atlas.size[1];
        for y in 0..rect[3] as u32 {
            for x in 0..rect[2] as u32 {
                let px = (x as f32 + 0.5) / rect[2] * (w + 2.0 * pad) - pad;
                let py = (y as f32 + 0.5) / rect[3] * (h + 2.0 * pad) - pad;
                let mut coverage = 0.0f32;
                for dx in [-pad, pad] {
                    for dy in [-pad, pad] {
                        let u = (px - dx) / w;
                        let v = (py - dy) / h;
                        if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) {
                            let sx = (lo[0] + u * ext[0]) * iw as f32 - 0.5;
                            let sy = (1.0 - lo[1] - ext[1] + v * ext[1]) * ih as f32 - 0.5;
                            let sample = alpha(rgba, iw, ih, sx, sy);
                            coverage = 1.0 - (1.0 - coverage) * (1.0 - sample);
                        }
                    }
                }
                let i = (((rect[1] as u32 + y) * 1024 + rect[0] as u32 + x) * 4) as usize;
                pixels[i..i + 4].copy_from_slice(&[
                    255,
                    255,
                    255,
                    (coverage * 255.0).round() as u8,
                ]);
            }
        }
    }
    Ok((pixels, height, rects))
}
fn alpha(rgba: &[u8], w: u32, h: u32, x: f32, y: f32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let fx = x - x.floor();
    let fy = y - y.floor();
    let mut value = 0.0;
    for (dx, wx) in [(0, 1.0 - fx), (1, fx)] {
        for (dy, wy) in [(0, 1.0 - fy), (1, fy)] {
            let px = (ix + dx).clamp(0, w as i32 - 1) as u32;
            let py = (iy + dy).clamp(0, h as i32 - 1) as u32;
            value += f32::from(rgba[((py * w + px) * 4 + 3) as usize]) / 255.0 * wx * wy;
        }
    }
    value
}
