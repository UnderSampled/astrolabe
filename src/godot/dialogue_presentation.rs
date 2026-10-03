//! Source bitmap metrics become a native FontFile, not a runtime font interpreter.
use super::{Project, disk_path, quoted};
use crate::model::{AssetPath, AssetRef, content::ContentSource, dialogue::*, io::Content};
use anyhow::{Result, ensure};
use std::{collections::BTreeSet, fmt::Write as _};
const SCALE: f32 = 640.0 / (2.0 * 4.65);
const FONT_SIZE: u32 = 21;

fn image(
    project: &mut Project,
    source: &mut impl ContentSource,
    texture: &DialogueTexture,
    suffix: &str,
) -> Result<(String, Vec<u8>)> {
    let value = source.load(&AssetRef::<crate::model::texture::Texture>::from_path(
        texture.texture.clone(),
    ))?;
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
    // One local builder variable is safe: each presentation is fully saved before
    // the next begins. Images and cache entries serialize with the native resource.
    writeln!(
        project.builder,
        "    var ui_font = FontFile.new()\n    ui_font.fixed_size = {FONT_SIZE}\n    ui_font.fixed_size_scale_mode = TextServer.FIXED_SIZE_SCALE_DISABLE\n    ui_font.subpixel_positioning = TextServer.SUBPIXEL_POSITIONING_ONE_QUARTER\n    ui_font.allow_system_fallback = false\n    ui_font.set_font_name({})\n    ui_font.set_cache_ascent(0, {FONT_SIZE}, {:?})\n    ui_font.set_cache_descent(0, {FONT_SIZE}, {:?})\n    ui_font.set_cache_scale(0, {FONT_SIZE}, 1.0)\n    ui_font.set_texture_image(0, Vector2i({FONT_SIZE},0), 0, Image.load_from_file({}))\n    ui_font.set_texture_image(0, Vector2i({FONT_SIZE},1), 0, Image.load_from_file({}))",
        quoted(&value.font_name),
        SCALE * 0.25,
        SCALE * 0.05,
        quoted(&format!("res://{atlas}")),
        quoted(&format!("res://{outline_path}"))
    )?;
    for (index, g) in value.glyphs.iter().enumerate() {
        let code = g.character as u32;
        let width = g.width * SCALE;
        let height = -2.0 * g.half_height * SCALE;
        let bearing = (2.0 * g.half_height - g.vertical_offset) * SCALE;
        writeln!(
            project.builder,
            "    ui_font.set_glyph_advance(0,{FONT_SIZE},{code},Vector2({:?},0.0))",
            (g.width + value.tracking) * SCALE
        )?;
        for outline_size in [0, 1] {
            let pad = if outline_size == 1 && g.uv.is_some() {
                SCALE * 0.01
            } else {
                0.0
            };
            let rect = if outline_size == 1 {
                rects[index]
            } else {
                g.uv.map_or([0.0; 4], |[lo, ext]| {
                    [
                        lo[0] * value.atlas.size[0] as f32,
                        (1.0 - lo[1] - ext[1]) * value.atlas.size[1] as f32,
                        ext[0] * value.atlas.size[0] as f32,
                        ext[1] * value.atlas.size[1] as f32,
                    ]
                })
            };
            let texture = if g.uv.is_some() { 0 } else { -1 };
            writeln!(
                project.builder,
                "    ui_font.set_glyph_offset(0,Vector2i({FONT_SIZE},{outline_size}),{code},Vector2({:?},{:?}))\n    ui_font.set_glyph_size(0,Vector2i({FONT_SIZE},{outline_size}),{code},Vector2({:?},{:?}))\n    ui_font.set_glyph_texture_idx(0,Vector2i({FONT_SIZE},{outline_size}),{code},{texture})\n    ui_font.set_glyph_uv_rect(0,Vector2i({FONT_SIZE},{outline_size}),{code},Rect2({:?},{:?},{:?},{:?}))",
                -pad,
                bearing - pad,
                width + 2.0 * pad,
                height + 2.0 * pad,
                rect[0],
                rect[1],
                rect[2],
                rect[3]
            )?;
        }
    }
    writeln!(
        project.builder,
        "    if ResourceSaver.save(ui_font, {}) != OK:\n        push_error(\"Cannot save dialogue FontFile\")\n        quit(1)\n        return",
        quoted(&format!("res://{font_path}"))
    )?;
    let mut text = format!(
        "[gd_resource type=\"Resource\" load_steps=7 format=3]\n[ext_resource type=\"Script\" path=\"res://addons/astrolabe/ui/dialogue_presentation.gd\" id=\"script\"]\n[ext_resource type=\"FontFile\" path={} id=\"font\"]\n",
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
    writeln!(
        text,
        "\n[resource]\nscript = ExtResource(\"script\")\nfont = ExtResource(\"font\")\nnormal_color = Color({},{},{},{})\nemphasis_color = Color({},{},{},{})\ncorner = ExtResource(\"corner\")\nborder = ExtResource(\"border\")\ncenter = ExtResource(\"center\")\nside = ExtResource(\"side\")",
        value.normal_color[0],
        value.normal_color[1],
        value.normal_color[2],
        value.normal_color[3],
        value.emphasis_color[0],
        value.emphasis_color[1],
        value.emphasis_color[2],
        value.emphasis_color[3]
    )?;
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
