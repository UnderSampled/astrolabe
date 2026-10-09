//! Bounded manor font/frame import. No source pointer or renderer state is persisted.
use super::{
    container::{file, pointer_file, u32_at, wire},
    records::*,
    session::SourceSession,
};
use crate::model::{AssetPath, dialogue::*, io::Content};
use anyhow::{Context, Result, ensure};
use binrw::BinRead;

#[derive(BinRead)]
#[br(little)]
struct FontWire {
    _runtime: [u32; 3],
    _name: u32,
    encoding: u32,
    count: u32,
    _glyphs: u32,
    tracking: f32,
}
#[derive(BinRead)]
#[br(little)]
struct GlyphWire {
    code: u16,
    _padding: u16,
    vertical_offset: f32,
    _shape: u32,
}
#[derive(BinRead)]
#[br(little)]
struct ShapeWire {
    _geometry: u32,
    half_height: f32,
    width: f32,
}
fn plus(a: Address, n: usize) -> Address {
    Address {
        offset: a.offset + n,
        ..a
    }
}
fn texture(s: &SourceSession, address: Address) -> Result<DialogueTexture> {
    let h: TextureInfoWire = wire(s.bytes(address, 188)?)?;
    let name = std::str::from_utf8(h.name.split(|b| *b == 0).next().unwrap())?
        .replace('\\', "/")
        .to_ascii_lowercase();
    ensure!(
        [h.stored_width, h.stored_height] == [h.logical_width, h.logical_height],
        "dialogue atlas requires logical/stored size agreement"
    );
    Ok(DialogueTexture {
        texture: AssetPath::new(format!("texture:/{name}"))?,
        size: [h.stored_width, h.stored_height],
        color_key: h.flags & 2 != 0,
    })
}
pub(super) struct FontInputs {
    pub(super) name: String,
    pub(super) tracking: f32,
    pub(super) glyphs: Vec<DialogueGlyph>,
    pub(super) atlas: DialogueTexture,
    pub(super) color: [f32; 4],
}

fn font_name(s: &SourceSession, address: Address) -> Result<String> {
    let name_data = &s.block(address)?.data[address.offset..];
    let end = name_data
        .iter()
        .take(256)
        .position(|b| *b == 0)
        .context("unbounded font name")?;
    super::dialogue::decode_cp1252(&name_data[..end])
}

/// Find a Fix sprite font by the name consumed by FON_fn_xGetFont.
///
/// Fix fonts in the supplied profile are type-17 declarations in block 19:1;
/// their source meaning is the same regardless of the UI that requests them.
pub(super) fn named_font(s: &SourceSession, name: &str) -> Result<(Address, FontInputs)> {
    let key = BlockKey(19, 1);
    let mut found = None;
    for &offset in s.allocation_indices(Owner::Fix, key)?.keys() {
        let address = Address {
            owner: Owner::Fix,
            key,
            offset,
        };
        let Ok(bytes) = s.bytes(address, 20) else {
            continue;
        };
        if u32_at(bytes, 16)? != 17 {
            continue;
        }
        let Some(name_address) = s.pointer(address, 12)? else {
            continue;
        };
        if font_name(s, name_address)? != name {
            continue;
        }
        ensure!(
            found.replace(address).is_none(),
            "duplicate Fix font {name}"
        );
    }
    let address = found.with_context(|| format!("required Fix font {name} absent"))?;
    Ok((address, font(s, address)?))
}

pub(super) fn font(s: &SourceSession, address: Address) -> Result<FontInputs> {
    let h: FontWire = wire(s.bytes(address, 32)?)?;
    ensure!(
        h.encoding == 0x11 && h.count > 0 && h.count <= 1024,
        "unsupported dialogue font profile"
    );
    let name = font_name(s, s.required(address, 12)?)?;
    let table = s.required(address, 24)?;
    s.bytes(table, h.count as usize * 4)?;
    let mut glyphs = Vec::new();
    let mut atlas = None;
    let mut color = None;
    for index in 0..h.count as usize {
        let a = s.required(plus(table, index * 4), 0)?;
        let glyph: GlyphWire = wire(s.bytes(a, 12)?)?;
        let code = u8::try_from(glyph.code).context("font has non-single-byte character")?;
        let character = super::dialogue::decode_cp1252(&[code])?
            .chars()
            .next()
            .unwrap();
        let a = s.required(a, 8)?;
        let shape: ShapeWire = wire(s.bytes(a, 12)?)?;
        let uv = if let Some(geometry) = s.pointer(a, 0)? {
            let g: GeometryWire = wire(s.bytes(geometry, 48)?)?;
            ensure!(
                g.point_count == 4 && g.element_count == 1,
                "font glyph is not a single quad"
            );
            let triangles = s.required(s.required(geometry, 28)?, 0)?;
            let t: TrianglesWire = wire(s.bytes(triangles, 32)?)?;
            ensure!(
                t.uv_count == 4 && t.triangle_count == 2,
                "font glyph triangles/UVs differ"
            );
            let material = s.required(s.required(triangles, 0)?, 0)?;
            let v: VisualMaterialWire = wire(s.bytes(material, 132)?)?;
            let tex = texture(s, s.required(material, 72)?)?;
            ensure!(
                atlas.as_ref().is_none_or(|old| old == &tex)
                    && color.as_ref().is_none_or(|old| old == &v.ambient),
                "dialogue font uses multiple atlases/colors"
            );
            atlas = Some(tex);
            color = Some(v.ambient);
            let points = s.floats::<2>(s.required(triangles, 20)?, 4)?;
            let lo = [
                points.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min),
                points.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min),
            ];
            let hi = [
                points
                    .iter()
                    .map(|p| p[0])
                    .fold(f32::NEG_INFINITY, f32::max),
                points
                    .iter()
                    .map(|p| p[1])
                    .fold(f32::NEG_INFINITY, f32::max),
            ];
            Some([lo, [hi[0] - lo[0], hi[1] - lo[1]]])
        } else {
            None
        };
        glyphs.push(DialogueGlyph {
            character,
            width: shape.width,
            half_height: shape.half_height,
            vertical_offset: glyph.vertical_offset,
            uv,
        });
    }
    Ok(FontInputs {
        name,
        tracking: h.tracking,
        glyphs,
        atlas: atlas.context("font has no atlas")?,
        color: color.context("font has no color")?,
    })
}
pub(super) fn import(
    s: &SourceSession,
    game: &std::path::Path,
    normal: Address,
    emphasis: Address,
) -> Result<DialoguePresentation> {
    let font = font(s, normal)?;
    let other = self::font(s, emphasis)?;
    ensure!(
        font.tracking == other.tracking && font.glyphs == other.glyphs && font.atlas == other.atlas,
        "normal/emphasis fonts need distinct metrics"
    );
    // Installed Fix GPT: DLG_fn_vSNAWriteInFix (0056bbe0) writes corner,
    // border, center, side and red_arrow materials consecutively. Resolve the
    // arrow by its declared material role, without guessing a texture filename.
    let gpt = file(&game.join("Gamedata/World/Levels/Fix.gpt"))?;
    ensure!(gpt.len() == 19344, "installed Fix GPT extent differs");
    let refs = pointer_file(&file(&game.join("Gamedata/World/Levels/Fix.rtp"))?)?;
    let mut frame = Vec::new();
    for (index, name) in ["coin", "top", "middle", "side"].iter().enumerate() {
        let word = u32_at(&gpt, 18084 + index * 4)?;
        let key = *refs
            .get(&word)
            .context("Fix GPT stencil root lacks RTP binding")?;
        let material = s.resolve(Owner::Fix, key, word)?;
        let input = texture(s, s.required(material, 72)?)?;
        ensure!(
            input.texture.as_str() == format!("texture:/fixtex/stencil/{name}.tga")
                && input.size == [32, 32],
            "installed dialogue stencil material/texture binding differs"
        );
        frame.push(input);
    }
    let word = u32_at(&gpt, 18084 + 4 * 4)?;
    let key = *refs
        .get(&word)
        .context("Fix GPT continuation stencil root lacks RTP binding")?;
    let material = s.resolve(Owner::Fix, key, word)?;
    let continuation = texture(s, s.required(material, 72)?)?;
    let value = DialoguePresentation {
        path: AssetPath::new("level:/dialogue/presentation/en")?,
        font_name: font.name,
        atlas: font.atlas,
        glyphs: font.glyphs,
        tracking: font.tracking,
        normal_color: font.color,
        emphasis_color: other.color,
        frame: frame.try_into().unwrap(),
        continuation,
    };
    value.validate()?;
    Ok(value)
}
