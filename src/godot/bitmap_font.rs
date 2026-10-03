//! Shared export-time bitmap font caches; no source font interpreter at runtime.
use super::{Project, quoted};
use crate::model::dialogue::DialogueGlyph;
use anyhow::Result;
use std::fmt::Write as _;
const FONT_SIZE: u32 = 21;
pub(super) struct FontInput<'a> {
    pub name: &'a str,
    pub atlas: &'a str,
    pub atlas_size: [u32; 2],
    pub glyphs: &'a [DialogueGlyph],
    pub tracking: f32,
    pub scale: f32,
    pub path: &'a str,
    pub outline: Option<(&'a str, &'a [[f32; 4]])>,
}
pub(super) fn emit(project: &mut Project, value: FontInput<'_>) -> Result<()> {
    let mut font_builder = String::new();
    writeln!(
        font_builder,
        "    var ui_font = FontFile.new()\n    ui_font.fixed_size = {FONT_SIZE}\n    ui_font.fixed_size_scale_mode = TextServer.FIXED_SIZE_SCALE_DISABLE\n    ui_font.subpixel_positioning = TextServer.SUBPIXEL_POSITIONING_ONE_QUARTER\n    ui_font.allow_system_fallback = false\n    ui_font.set_font_name({})\n    ui_font.set_cache_ascent(0, {FONT_SIZE}, {:?})\n    ui_font.set_cache_descent(0, {FONT_SIZE}, {:?})\n    ui_font.set_cache_scale(0, {FONT_SIZE}, 1.0)\n    ui_font.set_texture_image(0, Vector2i({FONT_SIZE},0), 0, Image.load_from_file({}))\n    ui_font.set_texture_image(0, Vector2i({FONT_SIZE},1), 0, Image.load_from_file({}))",
        quoted(value.name),
        value.scale * 0.25,
        value.scale * 0.05,
        quoted(value.atlas),
        quoted(value.outline.map_or(value.atlas, |(path, _)| path))
    )?;
    for (index, g) in value.glyphs.iter().enumerate() {
        let code = g.character as u32;
        let width = g.width * value.scale;
        let height = -2.0 * g.half_height * value.scale;
        let bearing = (2.0 * g.half_height - g.vertical_offset) * value.scale;
        writeln!(
            font_builder,
            "    ui_font.set_glyph_advance(0,{FONT_SIZE},{code},Vector2({:?},0.0))",
            (g.width + value.tracking) * value.scale
        )?;
        for outline_size in [0, 1]
            .into_iter()
            .filter(|n| *n == 0 || value.outline.is_some())
        {
            let pad = if outline_size == 1 && g.uv.is_some() {
                value.scale * 0.01
            } else {
                0.0
            };
            let rect = if outline_size == 1 {
                value.outline.unwrap().1[index]
            } else {
                g.uv.map_or([0.0; 4], |[lo, ext]| {
                    [
                        lo[0] * value.atlas_size[0] as f32,
                        (1.0 - lo[1] - ext[1]) * value.atlas_size[1] as f32,
                        ext[0] * value.atlas_size[0] as f32,
                        ext[1] * value.atlas_size[1] as f32,
                    ]
                })
            };
            let texture = if g.uv.is_some() { 0 } else { -1 };
            writeln!(
                font_builder,
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
        font_builder,
        "    if ResourceSaver.save(ui_font, {}) != OK:\n        push_error(\"Cannot save bitmap FontFile\")\n        quit(1)\n        return",
        quoted(value.path)
    )?;
    project.builder.push_str(&font_builder);
    project.dialogue_font_checks.push(font_builder);
    Ok(())
}
