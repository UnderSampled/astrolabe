//! Bounded English menu presentation import. Source records end at this boundary.
use super::{
    container::{file, pointer_file, u32_at, wire},
    dialogue,
    records::*,
    session::SourceSession,
    sound::SoundSites,
};
use crate::model::{
    AssetPath, AssetRef, Selection,
    audio::{Sound, SoundDefinitionTarget, SoundOperation, SoundResourceData, SoundSampleRef},
    io::Content,
    menu::*,
};
use anyhow::{Context, Result, ensure};
use binrw::BinRead;
use std::{collections::BTreeMap, path::Path};

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
/// FUN_004105b0 uses position 8..10, localized string 0x13,
/// selected/normal font names 0x1c/0x24 and horizontal alignment 0x2c.
#[derive(BinRead)]
#[br(little)]
struct MenuTextWire {
    _links: [u32; 8],
    position: [f32; 3],
    name: [u8; 32],
    _text: u32,
    _state: [u32; 8],
    selected_font: [u8; 32],
    normal_font: [u8; 32],
    horizontal_alignment: u8,
}
#[derive(BinRead)]
#[br(little)]
struct MenuTextureListWire {
    mode: u32,
    _background: u32,
    _texts: u32,
}
#[derive(BinRead)]
#[br(little)]
struct MenuSliderWire {
    _links: [u32; 8],
    position: [f32; 3],
}
struct Font {
    tracking: f32,
    glyphs: BTreeMap<char, Glyph>,
}
struct Glyph {
    width: f32,
    half_height: f32,
    vertical_offset: f32,
    image: Option<(AssetPath, [[f32; 2]; 2])>,
}
fn plus(a: Address, n: usize) -> Address {
    Address {
        offset: a.offset + n,
        ..a
    }
}
fn string(s: &SourceSession, a: Address) -> Result<String> {
    let data = &s.block(a)?.data[a.offset..];
    let end = data
        .iter()
        .take(256)
        .position(|b| *b == 0)
        .context("unbounded menu string")?;
    dialogue::decode_cp1252(&data[..end])
}
fn fixed_string(bytes: &[u8]) -> Result<&str> {
    Ok(std::str::from_utf8(
        bytes.split(|b| *b == 0).next().unwrap(),
    )?)
}
fn font(s: &SourceSession, name: &str) -> Result<Font> {
    let key = BlockKey(19, 1);
    let mut found = None;
    for (offset, _) in s.allocation_indices(Owner::Fix, key)? {
        let a = Address {
            owner: Owner::Fix,
            key,
            offset,
        };
        let Ok(data) = s.bytes(a, 32) else { continue };
        let h: FontWire = wire(data)?;
        if h.encoding != 0x11 || h.count == 0 || h.count > 256 {
            continue;
        }
        let Ok(n) = s.required(a, 12) else { continue };
        if string(s, n)? != name {
            continue;
        }
        ensure!(found.is_none(), "duplicate menu font {name}");
        let mut glyphs = BTreeMap::new();
        let table = s.required(a, 24)?;
        for i in 0..h.count as usize {
            let a = s.required(plus(table, i * 4), 0)?;
            let g: GlyphWire = wire(s.bytes(a, 12)?)?;
            let character = dialogue::decode_cp1252(&[u8::try_from(g.code)?])?
                .chars()
                .next()
                .unwrap();
            let a = s.required(a, 8)?;
            let shape: ShapeWire = wire(s.bytes(a, 12)?)?;
            let image = if let Some(geometry) = s.pointer(a, 0)? {
                let geom: GeometryWire = wire(s.bytes(geometry, 48)?)?;
                ensure!(
                    geom.point_count == 4 && geom.element_count == 1,
                    "menu glyph is not a quad"
                );
                let triangles = s.required(s.required(geometry, 28)?, 0)?;
                let t: TrianglesWire = wire(s.bytes(triangles, 32)?)?;
                ensure!(
                    t.uv_count == 4 && t.triangle_count == 2,
                    "menu glyph UV profile differs"
                );
                let material = s.required(s.required(triangles, 0)?, 0)?;
                let v: VisualMaterialWire = wire(s.bytes(material, 132)?)?;
                let tex: TextureInfoWire = wire(s.bytes(s.required(material, 72)?, 188)?)?;
                ensure!(
                    tex.flags & 32 != 0
                        && v.flags & 0x20000201 == 0x20000201
                        && v.ambient == [1.; 4],
                    "menu glyph requires unsupported blending/color"
                );
                let name = fixed_string(&tex.name)?.replace('\\', "/");
                let points = s.floats::<2>(s.required(triangles, 20)?, 4)?;
                let lo = std::array::from_fn(|c| {
                    points.iter().map(|p| p[c]).fold(f32::INFINITY, f32::min)
                });
                let hi: [f32; 2] = std::array::from_fn(|c| {
                    points
                        .iter()
                        .map(|p| p[c])
                        .fold(f32::NEG_INFINITY, f32::max)
                });
                Some((
                    AssetPath::new(format!("texture:/{name}"))?,
                    [lo, [hi[0] - lo[0], hi[1] - lo[1]]],
                ))
            } else {
                None
            };
            ensure!(
                glyphs
                    .insert(
                        character,
                        Glyph {
                            width: shape.width,
                            half_height: shape.half_height,
                            vertical_offset: g.vertical_offset,
                            image
                        }
                    )
                    .is_none(),
                "duplicate menu character"
            );
        }
        found = Some(Font {
            tracking: h.tracking,
            glyphs,
        });
    }
    found.with_context(|| format!("required source menu font {name} absent"))
}
fn artwork(font: &Font, label: &str, position: [f32; 3]) -> Result<Vec<MenuSprite>> {
    aligned_artwork(font, label, position, 1)
}
/// FON_fn_vDrawText3D alignment: 0 starts the text at x, 1 centres it, 2 ends it at x
/// (glyph centres; see below).
fn aligned_artwork(
    font: &Font,
    label: &str,
    [x, y, z]: [f32; 3],
    alignment: u8,
) -> Result<Vec<MenuSprite>> {
    let (half_x, half_y, scale) = projection(z)?;
    let glyphs = label
        .chars()
        .map(|c| {
            font.glyphs
                .get(&c)
                .with_context(|| format!("menu character {c} absent"))
        })
        .collect::<Result<Vec<_>>>()?;
    let width = glyphs
        .iter()
        .enumerate()
        .map(|(i, g)| {
            if i == 0 {
                g.width * 0.5 + font.tracking
            } else if i + 1 == glyphs.len() {
                g.width * 0.5
            } else {
                g.width + font.tracking
            }
        })
        .sum::<f32>();
    // Left and right alignment put the first or last glyph centre at x, as
    // the centred case positions glyph centres (matched against the original
    // Controls page capture, take s2-controls).
    let span = glyphs
        .windows(2)
        .map(|pair| pair[0].width * 0.5 + font.tracking + pair[1].width * 0.5)
        .sum::<f32>();
    let mut center = match alignment {
        0 => x,
        2 => x - span,
        _ => x - width * 0.5,
    };
    // Original FON_fn_vDraw3DString (005bb730), centered single line.
    // Convert observable 640×480 quads ahead of runtime; no font VM is retained.
    let line_half = font
        .glyphs
        .get(&' ')
        .context("menu space metrics absent")?
        .half_height;
    let mut result = Vec::new();
    for (i, g) in glyphs.iter().enumerate() {
        if let Some((texture, uv)) = &g.image {
            result.push(MenuSprite {
                texture: AssetRef::from_path(texture.clone()),
                uv: *uv,
                rect: [
                    (half_x + center - g.width * 0.5) * scale,
                    (half_y - y - line_half * 0.5 + 2. * g.half_height - g.vertical_offset) * scale,
                    g.width * scale,
                    -2. * g.half_height * scale,
                ],
            });
        }
        center += g.width * 0.5 + font.tracking;
        if let Some(next) = glyphs.get(i + 1) {
            center += next.width * 0.5;
        }
    }
    Ok(result)
}
/// FON_fn_vDraw3DString (005bb730) projects depth into observable pixel extent.
fn projection(depth: f32) -> Result<(f32, f32, f32)> {
    ensure!(
        depth.is_finite() && (0.0..6.0).contains(&depth),
        "invalid menu text depth"
    );
    let factor = 1. - depth * 0.16666667;
    ensure!(factor > 0., "menu text projection is behind the camera");
    let half_x = 4.65 / factor;
    Ok((half_x, 3.49 / factor, 640. / (half_x + half_x)))
}
#[derive(Clone, Copy)]
enum MenuVariant {
    Main,
    Pause,
}
impl MenuVariant {
    fn name(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Pause => "pause",
        }
    }
    fn background(self) -> Option<&'static str> {
        match self {
            Self::Main => Some("Menu/menu_princ.bmp"),
            Self::Pause => None,
        }
    }
    fn depth(self) -> f32 {
        match self {
            Self::Main => 0.,
            Self::Pause => 0.3,
        }
    }
}
pub(crate) fn import_main_menu(
    game: &Path,
    selection: &Selection,
) -> Result<(MenuPresentation, Vec<(AssetPath, Sound)>)> {
    import_menu(game, selection, MenuVariant::Main)
}
pub(crate) fn import_pause_menu(
    game: &Path,
    selection: &Selection,
) -> Result<(MenuPresentation, Vec<(AssetPath, Sound)>)> {
    import_menu(game, selection, MenuVariant::Pause)
}
fn import_menu(
    game: &Path,
    selection: &Selection,
    variant: MenuVariant,
) -> Result<(MenuPresentation, Vec<(AssetPath, Sound)>)> {
    ensure!(
        selection.locale == "en",
        "menu currently supports English source data"
    );
    let mut s = SourceSession::open(game, selection)?;
    dialogue::load_language(&mut s, game, selection)?;
    let normal = font(&s, "MenuFont")?;
    let selected = font(&s, "MenuFont_H")?;
    let definitions: &[(&str, &str, &str)] = match variant {
        MenuVariant::Main => &[
            ("NewGame", "new_game", "new game"),
            ("SavedGames", "load_game", "load game"),
            ("Options", "configure", "configure"),
            ("Credits", "credits", "credits"),
            ("Internet", "internet", "internet"),
            ("Quit", "quit", "quit"),
        ],
        MenuVariant::Pause => &[
            ("Resume Game", "resume_game", "resume game"),
            ("NextGame", "new_adventure", "start a new adventure"),
            ("Options", "configure", "configure"),
            ("Main Menu", "main_menu", "main menu"),
        ],
    };
    let mut nodes = BTreeMap::new();
    let key = BlockKey(18, 0);
    // Main owns a source texture declaration. Pause selects the installed
    // Resume Game text root, whose texture-header absence is checked below.
    let mut menu_parent = None;
    for (offset, _) in s.allocation_indices(Owner::Fix, key)? {
        let a = Address {
            owner: Owner::Fix,
            key,
            offset,
        };
        if let Some(expected) = variant.background() {
            let Ok(data) = s.bytes(a, 12) else { continue };
            let header: MenuTextureListWire = wire(data)?;
            if header.mode != 1 {
                continue;
            }
            let Ok(background) = s.required(a, 4) else {
                continue;
            };
            let Ok(name) = string(&s, background) else {
                continue;
            };
            if !name.replace('\\', "/").eq_ignore_ascii_case(expected) {
                continue;
            }
            let body = s.required(a, 8)?;
            ensure!(
                menu_parent.replace(body).is_none(),
                "ambiguous source menu background root"
            );
        } else {
            let Ok(data) = s.bytes(a, 177) else {
                continue;
            };
            let h: MenuTextWire = wire(data)?;
            if fixed_string(&h.name).ok() != Some("Resume Game")
                || fixed_string(&h.normal_font).ok() != Some("MenuFont")
            {
                continue;
            }
            let label = string(&s, s.required(a, 76)?)?;
            if label != "resume game" {
                continue;
            }
            let body = s.required(a, 8)?;
            ensure!(
                menu_parent.replace(body).is_none(),
                "ambiguous source pause text root"
            );
        }
    }
    let parent = menu_parent.context("menu source layout root absent")?;
    for (offset, _) in s.allocation_indices(Owner::Fix, key)? {
        let a = Address {
            owner: Owner::Fix,
            key,
            offset,
        };
        if variant.background().is_none() {
            let Ok(data) = s.bytes(a, 12) else { continue };
            let header: MenuTextureListWire = wire(data)?;
            ensure!(
                header.mode != 1 || s.pointer(a, 8)? != Some(parent),
                "pause root unexpectedly declares a texture backdrop"
            );
        }
        let Ok(data) = s.bytes(a, 177) else { continue };
        let h: MenuTextWire = wire(data)?;
        let Ok(name) = fixed_string(&h.name) else {
            continue;
        };
        if !definitions.iter().any(|v| v.0 == name) {
            continue;
        }
        if s.pointer(a, 8).ok().flatten() != Some(parent) {
            continue;
        }
        if fixed_string(&h.normal_font)? != "MenuFont"
            || fixed_string(&h.selected_font)? != "MenuFont_H"
        {
            continue;
        }
        let label = string(&s, s.required(a, 76)?)?;
        if !definitions.iter().any(|v| v.0 == name && v.2 == label) {
            continue;
        }
        ensure!(
            h.horizontal_alignment == 1,
            "menu horizontal alignment differs"
        );
        ensure!(
            nodes.insert(name.to_owned(), (a, h, label)).is_none(),
            "ambiguous original menu item"
        );
    }
    ensure!(
        nodes.len() == definitions.len(),
        "original menu item count differs"
    );
    let initial_address = s.required(parent, 8)?;
    let initial_item = nodes
        .iter()
        .find(|(_, v)| v.0 == initial_address)
        .map(|(name, _)| name.clone())
        .context("source initial menu item absent")?;
    let node_actions: BTreeMap<_, _> = nodes.iter().map(|(name, v)| (v.0, name.clone())).collect();
    let mut items = Vec::new();
    for &(source_name, name, _) in definitions {
        let (a, h, label) = nodes.remove(source_name).unwrap();
        ensure!(
            s.required(a, 8)? == parent,
            "menu items belong to different menus"
        );
        ensure!(
            h.position[2] == variant.depth(),
            "source menu depth differs"
        );
        let (half_x, half_y, scale) = projection(h.position[2])?;
        // Retain unchanged main anchor arithmetic; nonzero source depth scales pause.
        let anchor = if h.position[2] == 0. {
            [
                (4.65 + h.position[0]) * 640. / 9.3,
                (3.49 - h.position[1]) * 640. / 9.3,
            ]
        } else {
            [
                (half_x + h.position[0]) * scale,
                (half_y - h.position[1]) * scale,
            ]
        };
        let mut normal = artwork(&normal, &label, h.position)?;
        let mut selected = artwork(&selected, &label, h.position)?;
        let all: Vec<_> = normal.iter().chain(&selected).collect();
        let x = all
            .iter()
            .map(|g| g.rect[0])
            .fold(f32::INFINITY, f32::min)
            .floor()
            - 1.;
        let y = all
            .iter()
            .map(|g| g.rect[1])
            .fold(f32::INFINITY, f32::min)
            .floor()
            - 1.;
        let right = all
            .iter()
            .map(|g| g.rect[0] + g.rect[2])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            + 1.;
        let bottom = all
            .iter()
            .map(|g| g.rect[1] + g.rect[3])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            + 1.;
        for g in normal.iter_mut().chain(&mut selected) {
            g.rect[0] -= x;
            g.rect[1] -= y;
        }
        let mut neighbors: [Option<String>; 4] = [None, None, None, None];
        for (i, field) in [12, 16, 20, 24].into_iter().enumerate() {
            if let Some(target) = s.pointer(a, field)? {
                neighbors[i] = Some(
                    node_actions
                        .get(&target)
                        .context("menu navigation targets unsupported item")?
                        .clone(),
                );
            }
        }
        items.push(MenuItem {
            action: source_name.into(),
            position: anchor,
            horizontal_alignment: h.horizontal_alignment,
            neighbors,
            name: name.into(),
            label,
            rect: [x - anchor[0], y - anchor[1], right - x, bottom - y],
            normal,
            selected,
        });
    }
    let sites = SoundSites::open(game, selection, &s)?;
    // Original custom menu FUN_004123b0 dispatches array[0] on confirm,
    // array[2] on navigation. English SND binds these to M_Menu Valid/Select.
    let matches_event = |link: &crate::model::audio::SoundEventLink, id| -> Result<bool> {
        Ok(
            matches!(&link.binding(&sites.library.1)?.target,SoundDefinitionTarget::Event(r) if std::path::Path::new(r.definitions.path.file()).file_name().is_some_and(|n|n.to_string_lossy().eq_ignore_ascii_case("M_Menu.csb")) && r.id==id),
        )
    };
    let mut pair = None;
    for (a, link) in &sites.events {
        let Some(confirm) = link else { continue };
        if !matches_event(confirm, 4)? {
            continue;
        }
        if let Some(Some(nav)) = sites.events.get(&plus(*a, 8))
            && matches_event(nav, 2)?
        {
            ensure!(pair.is_none(), "ambiguous main-menu sound array");
            pair = Some((nav.clone(), confirm.clone()));
        }
    }
    let (nav, confirm) = pair.context("original main-menu sound bindings absent")?;
    let graph = crate::openspace::audio::graph::SoundGraph::import(
        game,
        selection,
        &[sites.library],
        [&nav, &confirm],
    )?;
    let mut sounds = Vec::new();
    let mut gains = Vec::new();
    for (name, link) in [("navigation", nav), ("confirmation", confirm)] {
        let library = &graph.definitions[&link.library.path];
        let SoundDefinitionTarget::Event(event) = &link.binding(library)?.target else {
            unreachable!()
        };
        let SoundOperation::Play {
            resource: Some(resource),
            ..
        } = &graph.definitions[&event.definitions.path]
            .event(event.id)?
            .operation
        else {
            anyhow::bail!("menu event is not a sample play")
        };
        let resource = graph.definitions[&resource.definitions.path].resource(resource.id)?;
        let SoundResourceData::Sample {
            sample: Some(sample),
            ..
        } = &resource.data
        else {
            anyhow::bail!("menu sound is not a bound sample")
        };
        let sound = match sample {
            SoundSampleRef::Sound(r) => graph.sounds[&r.path].clone(),
            SoundSampleRef::Bank { bank, resource_id } => graph.banks[&bank.path]
                .sounds
                .iter()
                .find(|v| v.resource_id == Some(*resource_id))
                .context("menu sample absent from bank")?
                .clone(),
        };
        gains.push((resource.volume.unwrap_or(127.) / 127.).clamp(0., 1.));
        sounds.push((
            AssetPath::new(format!("sound:/menu/{}/en/{name}", variant.name()))?,
            sound,
        ));
    }
    let presentation = MenuPresentation {
        path: AssetPath::new(format!("fix:/menu/{}/en", variant.name()))?,
        locale: "en".into(),
        background: variant
            .background()
            .map(|name| AssetRef::new(format!("texture:/{name}")))
            .transpose()?,
        initial_item,
        navigation_gain: gains[0],
        confirmation_gain: gains[1],
        items,
        navigation: AssetRef::from_path(sounds[0].0.clone()),
        confirmation: AssetRef::from_path(sounds[1].0.clone()),
        configuration: if matches!(variant, MenuVariant::Main) {
            Some(configuration(&s)?)
        } else {
            None
        },
        credits: if matches!(variant, MenuVariant::Main) {
            Some(credits(&s, game)?)
        } else {
            None
        },
    };
    presentation.validate()?;
    Ok((presentation, sounds))
}

/// The source credits callback appends each localized text key in INI order.
/// English Fix block 18 contains the same 24-byte key slots; Fix.dlg's final
/// 400 Fix block-19 handles resolve their values in that registration order.
/// FUN_0040f3f0 and FUN_0040eb10 both stop at CDTCreditsNumEntries (397), while
/// FUN_0040ef40 appends every active entry (400).
fn credits(s: &SourceSession, game: &Path) -> Result<CreditsPresentation> {
    let ini = std::fs::read_to_string(game.join("Gamedata/credits.ini"))?;
    let mut in_credits = false;
    let mut keys = Vec::new();
    let mut displayed_entries = None;
    let mut max_visible_lines = None;
    let mut font_name = None;
    let mut text_spacing = None;
    let mut text_position = None;
    for raw in ini.lines() {
        let line = raw.split_once(';').map_or(raw, |(before, _)| before).trim();
        if line.starts_with("{CreditsMenuTextures:") {
            ensure!(!in_credits, "duplicate original credits section");
            in_credits = true;
            continue;
        }
        if !in_credits {
            continue;
        }
        if line.starts_with('}') {
            break;
        }
        let Some((name, arguments)) = line.split_once('(') else {
            continue;
        };
        let Some(arguments) = arguments.strip_suffix(')') else {
            continue;
        };
        let values = arguments
            .split(',')
            .map(|value| value.trim().trim_matches('"'))
            .collect::<Vec<_>>();
        match name.trim() {
            "CDTCreditsEntry" => {
                ensure!(
                    values.len() == 1 && !values[0].is_empty(),
                    "malformed credits key"
                );
                keys.push(values[0].to_owned());
            }
            "CDTCreditsNumEntries" => {
                ensure!(
                    values.len() == 1 && displayed_entries.is_none(),
                    "duplicate or malformed credits count"
                );
                displayed_entries = Some(values[0].parse::<usize>()?);
            }
            "CDTCreditsMaxLinesDisplayed" => {
                ensure!(
                    values.len() == 1 && max_visible_lines.is_none(),
                    "duplicate or malformed credits line limit"
                );
                max_visible_lines = Some(values[0].parse::<usize>()?);
            }
            "CDTCreditsFontName" => {
                ensure!(values.len() == 1, "malformed credits font");
                ensure!(
                    font_name.replace(values[0].to_owned()).is_none(),
                    "duplicate credits font"
                );
            }
            "CDTCreditsTextSpacing" => {
                ensure!(values.len() == 1, "malformed credits spacing");
                ensure!(
                    text_spacing.replace(values[0].parse::<f32>()?).is_none(),
                    "duplicate credits spacing"
                );
            }
            "CDTCreditsTextPosition" => {
                ensure!(
                    values.len() == 2
                        && text_position
                            .replace([values[0].parse::<f32>()?, values[1].parse::<f32>()?])
                            .is_none(),
                    "duplicate or malformed credits text position"
                );
            }
            // Source has no credits image directives; keep that boundary explicit.
            "CDTImage" | "CDTBackgroundFile" => {
                anyhow::bail!("unsupported active original credits artwork directive {name}");
            }
            _ => {}
        }
    }
    ensure!(in_credits, "original credits section absent");
    ensure!(
        keys.len() == 400,
        "original credits INI entry count differs"
    );
    let displayed_entries = displayed_entries.context("credits display count absent")?;
    ensure!(
        displayed_entries == 397,
        "original credits displayed-entry count differs"
    );
    let max_visible_lines = max_visible_lines.context("credits line limit absent")?;
    let font_name = font_name.context("credits font name absent")?;
    let text_spacing = text_spacing.context("credits text spacing absent")?;
    let text_position = text_position.context("credits text position absent")?;

    let key_block = BlockKey(18, 0);
    let mut matching_key_tables = 0;
    for (offset, _) in s.allocation_indices(Owner::Fix, key_block)? {
        let address = Address {
            owner: Owner::Fix,
            key: key_block,
            offset,
        };
        let Ok(bytes) = s.bytes(address, keys.len() * 24) else {
            continue;
        };
        let (slots, remainder) = bytes.as_chunks::<24>();
        ensure!(remainder.is_empty(), "credits key registry extent differs");
        let mut parsed = Vec::new();
        for slot in slots {
            let Some(end) = slot.iter().position(|byte| *byte == 0) else {
                parsed.clear();
                break;
            };
            let Ok(value) = std::str::from_utf8(&slot[..end]) else {
                parsed.clear();
                break;
            };
            if end == 0 {
                parsed.clear();
                break;
            }
            parsed.push(value.to_owned());
        }
        if parsed == keys {
            matching_key_tables += 1;
        }
    }
    ensure!(
        matching_key_tables == 1,
        "English Fix credits key registry differs from credits.ini"
    );

    let folder = game.join("LangData/English/world/levels");
    let dlg = file(&folder.join("Fix.dlg"))?;
    ensure!(dlg.len() % 4 == 0, "Fix DLG reference table extent differs");
    let relocations = pointer_file(&file(&folder.join("Fix.rtd"))?)?;
    let mut text_handles = Vec::new();
    let (references, remainder) = dlg.as_chunks::<4>();
    ensure!(
        remainder.is_empty(),
        "Fix DLG reference table extent differs"
    );
    for bytes in references {
        let value = u32_at(bytes, 0)?;
        if relocations.get(&value) == Some(&BlockKey(19, 0)) {
            text_handles.push(s.resolve(Owner::Fix, BlockKey(19, 0), value)?);
        }
    }
    ensure!(
        text_handles.len() == keys.len() + 10,
        "English Fix DLG text-handle sequence differs"
    );
    let credit_start = text_handles.len() - keys.len();
    let mut entries = Vec::with_capacity(keys.len());
    for (index, key) in keys.into_iter().enumerate() {
        let handle = text_handles[credit_start + index];
        let value = s
            .pointer(handle, 0)?
            .context("localized credits text handle lacks its string")?;
        let text = string(s, value)?;
        let alignment = match key.as_bytes()[0] {
            b'+' => 2,
            b'-' => 0,
            _ => 1,
        };
        entries.push(CreditsEntry {
            key,
            text,
            alignment,
        });
    }
    ensure!(
        entries[0].text == "Hype  -  The Time Quest"
            && entries[1].text.trim().is_empty()
            && entries[2].text.trim() == "Original Idea and Producer"
            && entries[3].text.trim() == "Alain Tascan",
        "English credits key/handle registration order differs"
    );

    let font_address = source_font(s, &font_name)?;
    let source_font = super::dialogue_presentation::font(s, font_address)?;
    let presentation = CreditsPresentation {
        path: AssetPath::new("fix:/menu/credits/en")?,
        locale: "en".into(),
        font: CreditsFont {
            name: source_font.name,
            atlas: source_font.atlas,
            glyphs: source_font.glyphs,
            tracking: source_font.tracking,
            color: source_font.color,
        },
        entries,
        displayed_entries,
        max_visible_lines,
        text_spacing,
        text_position,
    };
    presentation.validate()?;
    Ok(presentation)
}

fn source_font(s: &SourceSession, name: &str) -> Result<Address> {
    let key = BlockKey(19, 1);
    let mut found = None;
    for (offset, _) in s.allocation_indices(Owner::Fix, key)? {
        let address = Address {
            owner: Owner::Fix,
            key,
            offset,
        };
        let Ok(bytes) = s.bytes(address, 32) else {
            continue;
        };
        let Ok(header) = wire::<FontWire>(bytes) else {
            continue;
        };
        if header.encoding != 0x11 || header.count == 0 || header.count > 1024 {
            continue;
        }
        let Ok(name_address) = s.required(address, 12) else {
            continue;
        };
        if string(s, name_address).ok().as_deref() != Some(name) {
            continue;
        }
        ensure!(
            found.replace(address).is_none(),
            "duplicate source credits font {name}"
        );
    }
    found.with_context(|| format!("required source credits font {name} absent"))
}

/// Import the source-owned Options text root. FUN_00413bd0 positions its
/// three sliders from the persisted 0..127 values; the Godot UI replaces the
/// legacy input/runtime controls while retaining the original labels/artwork.
fn configuration(s: &SourceSession) -> Result<MenuConfiguration> {
    let expected = [
        ("Images", "image"),
        ("Controls", "controls"),
        ("Sound", "sound"),
        ("DisplayWithSound", "music"),
        ("DisplayWithSound", "sound"),
        ("DisplayWithSound", "voices"),
        ("DisplayWithSound", "stereo test"),
        ("DisplayWithSound", "reverse stereo"),
        ("DisplayWithImage", "graphic"),
        ("DisplayWithImage", "textures"),
        ("DisplayWithImage", "screen"),
    ];
    let key = BlockKey(18, 0);
    let mut parent = None;
    for (offset, _) in s.allocation_indices(Owner::Fix, key)? {
        let address = Address {
            owner: Owner::Fix,
            key,
            offset,
        };
        let Ok(bytes) = s.bytes(address, 12) else {
            continue;
        };
        let header: MenuTextureListWire = wire(bytes)?;
        if header.mode != 1 {
            continue;
        }
        let Ok(background) = s.required(address, 4).and_then(|at| string(s, at)) else {
            continue;
        };
        if !background
            .replace('\\', "/")
            .eq_ignore_ascii_case("Menu/config_back.bmp")
        {
            continue;
        }
        ensure!(
            parent.replace(s.required(address, 8)?).is_none(),
            "ambiguous source configuration root"
        );
    }
    let parent = parent.context("source configuration root absent")?;
    let mut nodes = BTreeMap::new();
    for (offset, _) in s.allocation_indices(Owner::Fix, key)? {
        let address = Address {
            owner: Owner::Fix,
            key,
            offset,
        };
        let Ok(bytes) = s.bytes(address, 177) else {
            continue;
        };
        let header: MenuTextWire = wire(bytes)?;
        if s.pointer(address, 8).ok().flatten() != Some(parent) {
            continue;
        }
        let action = fixed_string(&header.name)?.to_owned();
        if !expected.iter().any(|(name, _)| *name == action) {
            continue;
        }
        let label = string(s, s.required(address, 76)?)?;
        if !expected
            .iter()
            .any(|(name, text)| *name == action && *text == label)
        {
            continue;
        }
        ensure!(
            nodes
                .insert((action, label.clone()), (address, header, label))
                .is_none(),
            "ambiguous source configuration item"
        );
    }
    ensure!(
        nodes.len() == expected.len(),
        "source configuration item count differs"
    );
    let initial_address = s.required(parent, 8)?;
    let mut item_names = BTreeMap::new();
    for &(action, label) in &expected {
        let slug = |value: &str| value.to_ascii_lowercase().replace(' ', "_");
        item_names.insert((action, label), format!("{}_{}", slug(action), slug(label)));
    }
    let mut addresses = BTreeMap::new();
    for &(action, label) in &expected {
        addresses.insert(
            nodes[&(action.to_owned(), label.to_owned())].0,
            item_names[&(action, label)].clone(),
        );
    }
    let initial_item = addresses
        .get(&initial_address)
        .context("source configuration initial item absent")?
        .clone();
    let mut items = Vec::new();
    for &(source_action, source_label) in &expected {
        let (address, header, label) = nodes
            .remove(&(source_action.to_owned(), source_label.to_owned()))
            .unwrap();
        let name = item_names[&(source_action, source_label)].clone();
        let normal_font = font(s, fixed_string(&header.normal_font)?)?;
        let selected_font = font(s, fixed_string(&header.selected_font)?)?;
        let (half_x, half_y, scale) = projection(header.position[2])?;
        let anchor = if header.position[2] == 0. {
            [
                (4.65 + header.position[0]) * 640. / 9.3,
                (3.49 - header.position[1]) * 640. / 9.3,
            ]
        } else {
            [
                (half_x + header.position[0]) * scale,
                (half_y - header.position[1]) * scale,
            ]
        };
        let mut normal = artwork(&normal_font, &label, header.position)?;
        let mut selected = artwork(&selected_font, &label, header.position)?;
        let all: Vec<_> = normal.iter().chain(&selected).collect();
        let x = all
            .iter()
            .map(|g| g.rect[0])
            .fold(f32::INFINITY, f32::min)
            .floor()
            - 1.;
        let y = all
            .iter()
            .map(|g| g.rect[1])
            .fold(f32::INFINITY, f32::min)
            .floor()
            - 1.;
        let right = all
            .iter()
            .map(|g| g.rect[0] + g.rect[2])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            + 1.;
        let bottom = all
            .iter()
            .map(|g| g.rect[1] + g.rect[3])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            + 1.;
        for sprite in normal.iter_mut().chain(&mut selected) {
            sprite.rect[0] -= x;
            sprite.rect[1] -= y;
        }
        let mut neighbors: [Option<String>; 4] = [None, None, None, None];
        for (index, field) in [12, 16, 20, 24].into_iter().enumerate() {
            if let Some(target) = s.pointer(address, field)? {
                neighbors[index] = Some(
                    addresses
                        .get(&target)
                        .context("configuration navigation target absent")?
                        .clone(),
                );
            }
        }
        items.push(MenuItem {
            name,
            label,
            action: source_action.into(),
            position: anchor,
            horizontal_alignment: header.horizontal_alignment,
            neighbors,
            rect: [x - anchor[0], y - anchor[1], right - x, bottom - y],
            normal,
            selected,
        });
    }
    let mut slider_positions: [Option<[f32; 3]>; 3] = [None; 3];
    for (offset, _) in s.allocation_indices(Owner::Fix, key)? {
        let address = Address {
            owner: Owner::Fix,
            key,
            offset,
        };
        let Ok(block) = s.block(address) else {
            continue;
        };
        let Some(raw_name) = block.data.get(offset + 68..offset + 100) else {
            continue;
        };
        let end = raw_name
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(raw_name.len());
        let Ok(name) = std::str::from_utf8(&raw_name[..end]) else {
            continue;
        };
        let index = match name {
            "Slider1" => 0,
            "Slider2" => 1,
            "Slider3" => 2,
            _ => continue,
        };
        let position: MenuSliderWire = wire(s.bytes(address, 44)?)?;
        ensure!(
            slider_positions[index].replace(position.position).is_none(),
            "duplicate source {name}"
        );
    }
    let slider_positions = slider_positions
        .map(|value| value.context("source configuration slider absent"))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    Ok(MenuConfiguration {
        path: AssetPath::new("fix:/menu/configuration/en")?,
        background: AssetRef::new("texture:/Menu/config_back.bmp")?,
        initial_item,
        items,
        slider_positions: slider_positions
            .try_into()
            .map_err(|_| anyhow::anyhow!("source slider count differs"))?,
        controls: Some(controls(s)?),
    })
}

/// MenuText records on the Controls root (FUN_004105b0/FUN_00410230) carry the
/// keyboard and pad options-entry names at +0xba and +0xda, 32 bytes each.
const CONTROLS_KEYBOARD_ENTRY: usize = 0xba;
const CONTROLS_PAD_ENTRY: usize = 0xda;
/// FUN_00414b00 puts the SelectBox at y 173.0 (0x432d0000) when a list opens.
const CONTROLS_SELECT_FIRST_Y: f32 = 173.0;
/// Fix 7:0 (input module): 0x1c-byte entry actions from 0x68 and the
/// {u16 code, French, English, German} key-name records at 0x10898.
const ENTRY_ACTIONS: usize = 0x68;
const KEY_NAMES: usize = 0x10898;

/// One source text record projected like `configuration()` items.
fn text_item(s: &SourceSession, address: Address, name: String, alignment: u8) -> Result<MenuItem> {
    let header: MenuTextWire = wire(s.bytes(address, 177)?)?;
    let label = string(s, s.required(address, 76)?)?;
    let normal_font = font(s, fixed_string(&header.normal_font)?)?;
    let selected_font = font(s, fixed_string(&header.selected_font)?)?;
    labelled_item(
        name,
        &label,
        fixed_string(&header.name)?,
        header.position,
        alignment,
        &normal_font,
        &selected_font,
    )
}

fn labelled_item(
    name: String,
    label: &str,
    action: &str,
    position: [f32; 3],
    alignment: u8,
    normal_font: &Font,
    selected_font: &Font,
) -> Result<MenuItem> {
    let (half_x, half_y, scale) = projection(position[2])?;
    let anchor = [
        (half_x + position[0]) * scale,
        (half_y - position[1]) * scale,
    ];
    let mut normal = aligned_artwork(normal_font, label, position, alignment)?;
    let mut selected = aligned_artwork(selected_font, label, position, alignment)?;
    let all: Vec<_> = normal.iter().chain(&selected).collect();
    if all.is_empty() {
        // The pad list's DefaultValues label is three spaces; nothing is drawn.
        ensure!(
            label.trim().is_empty(),
            "controls label {label:?} has no artwork"
        );
        return Ok(MenuItem {
            name,
            label: label.to_owned(),
            action: action.to_owned(),
            position: anchor,
            horizontal_alignment: alignment,
            neighbors: [None, None, None, None],
            rect: [-1., -1., 2., 2.],
            normal,
            selected,
        });
    }
    let x = all
        .iter()
        .map(|g| g.rect[0])
        .fold(f32::INFINITY, f32::min)
        .floor()
        - 1.;
    let y = all
        .iter()
        .map(|g| g.rect[1])
        .fold(f32::INFINITY, f32::min)
        .floor()
        - 1.;
    let right = all
        .iter()
        .map(|g| g.rect[0] + g.rect[2])
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        + 1.;
    let bottom = all
        .iter()
        .map(|g| g.rect[1] + g.rect[3])
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        + 1.;
    for sprite in normal.iter_mut().chain(&mut selected) {
        sprite.rect[0] -= x;
        sprite.rect[1] -= y;
    }
    Ok(MenuItem {
        name,
        label: label.to_owned(),
        action: action.to_owned(),
        position: anchor,
        horizontal_alignment: alignment,
        neighbors: [None, None, None, None],
        rect: [x - anchor[0], y - anchor[1], right - x, bottom - y],
        normal,
        selected,
    })
}

/// The separate Controls root (`Menu/control_back.bmp`). FUN_00414b00 enters it
/// from the Options `Controls` item; its body's first item is the Keyboard tab.
fn controls(s: &SourceSession) -> Result<MenuControls> {
    let key = BlockKey(18, 0);
    let mut root = None;
    for (offset, _) in s.allocation_indices(Owner::Fix, key)? {
        let address = Address {
            owner: Owner::Fix,
            key,
            offset,
        };
        let Ok(bytes) = s.bytes(address, 12) else {
            continue;
        };
        let header: MenuTextureListWire = wire(bytes)?;
        if header.mode != 1 {
            continue;
        }
        let Ok(background) = s.required(address, 4).and_then(|at| string(s, at)) else {
            continue;
        };
        if background
            .replace('\\', "/")
            .eq_ignore_ascii_case("Menu/control_back.bmp")
        {
            ensure!(
                root.replace(address).is_none(),
                "ambiguous source controls root"
            );
        }
    }
    let root = root.context("source controls root absent")?;
    let body = s.required(root, 8)?;
    let hint = {
        let data = &s.block(root)?.data[root.offset + 12..];
        let end = data
            .iter()
            .take(1024)
            .position(|b| *b == 0)
            .context("unbounded controls hint")?;
        dialogue::decode_cp1252(&data[..end])?.trim_end().to_owned()
    };
    let keyboard_tab = s.required(body, 8)?;
    let gamepad_tab = s.required(keyboard_tab, 24)?;
    ensure!(
        s.pointer(keyboard_tab, 20)? == Some(gamepad_tab)
            && s.pointer(gamepad_tab, 20)? == Some(keyboard_tab)
            && s.pointer(gamepad_tab, 24)? == Some(keyboard_tab),
        "source controls tabs are not a left/right pair"
    );
    let mut tabs = Vec::new();
    let mut lists = Vec::new();
    for (tab, tab_name, entry_field) in [
        (keyboard_tab, "keyboard", CONTROLS_KEYBOARD_ENTRY),
        (gamepad_tab, "gamepad", CONTROLS_PAD_ENTRY),
    ] {
        let header: MenuTextWire = wire(s.bytes(tab, 177)?)?;
        let action = fixed_string(&header.name)?;
        ensure!(
            action.eq_ignore_ascii_case(tab_name),
            "source controls tab {action} differs"
        );
        let mut item = text_item(s, tab, format!("controls_{tab_name}"), 1)?;
        item.neighbors[2] = Some(
            if tab_name == "keyboard" {
                "controls_gamepad"
            } else {
                "controls_keyboard"
            }
            .into(),
        );
        item.neighbors[3] = item.neighbors[2].clone();
        tabs.push(item);
        // The child ring: up/down wrap through every row back to the first.
        let first = s.required(tab, 28)?;
        let mut rows = Vec::new();
        let mut at = first;
        loop {
            let header: MenuTextWire = wire(s.bytes(at, 177)?)?;
            let action = fixed_string(&header.name)?.to_owned();
            let entry = fixed_string(s.bytes(plus(at, entry_field), 32)?)?.to_owned();
            let (entry, alignment) = match action.as_str() {
                "KeyStrokes" => {
                    ensure!(!entry.is_empty(), "controls row has no {tab_name} entry");
                    (Some(entry), header.horizontal_alignment)
                }
                "DefaultValues" => (None, header.horizontal_alignment),
                other => anyhow::bail!("unsupported controls row action {other}"),
            };
            let index = rows.len();
            let item = text_item(s, at, format!("controls_{tab_name}_{index}"), alignment)?;
            rows.push((at, ControlRow { item, entry }));
            ensure!(rows.len() <= 32, "controls row ring does not close");
            at = s.required(at, 16)?;
            if at == first {
                break;
            }
        }
        let count = rows.len();
        for index in 0..count {
            let up = s.required(rows[index].0, 12)?;
            ensure!(
                up == rows[(index + count - 1) % count].0,
                "controls rows are not an up/down ring"
            );
            rows[index].1.item.neighbors[0] = Some(format!(
                "controls_{tab_name}_{}",
                (index + count - 1) % count
            ));
            rows[index].1.item.neighbors[1] =
                Some(format!("controls_{tab_name}_{}", (index + 1) % count));
        }
        lists.push(rows.into_iter().map(|(_, row)| row).collect::<Vec<_>>());
    }
    let gamepad = lists.pop().unwrap();
    let keyboard = lists.pop().unwrap();
    // The root's sprite list (list node +0x8c head, records linked at +0):
    // name +0x24, centre +0x1c/+0x20 in pixels, size +0x74/+0x78 and a GLI
    // texture at +0x64 or +0x68. The four unnamed records tile the localized
    // stained-glass title; the named ones are the pictures and SelectBox.
    let mut anchor_sprite = None;
    for (offset, _) in s.allocation_indices(Owner::Fix, key)? {
        let address = Address {
            owner: Owner::Fix,
            key,
            offset,
        };
        let Ok(bytes) = s.allocation_bytes(address) else {
            continue;
        };
        if bytes.len() >= 0x7c && fixed_string(&bytes[0x24..0x44]).ok() == Some("SelectBox") {
            ensure!(
                anchor_sprite.replace(address).is_none(),
                "ambiguous source SelectBox"
            );
        }
    }
    let list = s.required(anchor_sprite.context("source SelectBox absent")?, 8)?;
    let mut pictures = Vec::new();
    let mut at = s.pointer(list, 0x8c)?;
    while let Some(address) = at {
        ensure!(pictures.len() < 32, "controls sprite list does not end");
        let bytes = s.allocation_bytes(address)?;
        ensure!(bytes.len() >= 0x7c, "controls sprite record is short");
        let texture = match s.pointer(address, 0x64)? {
            Some(texture) => texture,
            None => s.required(address, 0x68)?,
        };
        let info: TextureInfoWire = wire(s.bytes(texture, 188)?)?;
        let float = |at: usize| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        let name = fixed_string(&bytes[0x24..0x44])?;
        pictures.push(MenuPicture {
            name: if name.is_empty() {
                format!("Title{}", pictures.len())
            } else {
                name.to_owned()
            },
            texture: AssetRef::new(format!(
                "texture:/{}",
                fixed_string(&info.name)?.replace('\\', "/")
            ))?,
            center: [float(0x1c), float(0x20)],
            size: [float(0x74), float(0x78)],
        });
        at = s.pointer(address, 0)?;
    }
    for name in ["DisplayWithKeyboard", "DisplayWithGamepad", "SelectBox"] {
        ensure!(
            pictures.iter().any(|picture| picture.name == name),
            "source controls sprite {name} absent"
        );
    }
    let (entries, actions) = entry_actions(s)?;
    // Key names: IPT_fn_szGetStringOf through _strlwr, in the row's +0x90 font,
    // at x - 0.4 with alignment 2 (FUN_004105b0).
    let first_row = s.required(keyboard_tab, 28)?;
    let row: MenuTextWire = wire(s.bytes(first_row, 177)?)?;
    let name_font = font(s, fixed_string(&row.normal_font)?)?;
    let name_position = [row.position[0] - 0.4, row.position[1], row.position[2]];
    let mut input_names = Vec::new();
    let table = Address {
        owner: Owner::Fix,
        key: BlockKey(7, 0),
        offset: KEY_NAMES,
    };
    for index in 0..512 {
        let record = plus(table, index * 16);
        let code = u16::from_le_bytes(s.bytes(record, 2)?.try_into().unwrap());
        if code == 0xffff {
            break;
        }
        let input = if code & 0x8000 != 0 {
            // Pad codes share the entry encoding: 0x0a + button index.
            let pad = code & 0x7fff;
            if !(0x0a..0x2a).contains(&pad) {
                continue;
            }
            ControlInput::PadButton(u8::try_from(pad - 0x0a)?)
        } else {
            ControlInput::Key(code)
        };
        let english = string(s, s.required(record, 8)?)?.to_lowercase();
        let label: String = english
            .chars()
            .filter(|c| name_font.glyphs.contains_key(c))
            .collect();
        if label.trim().is_empty() {
            continue;
        }
        let artwork = labelled_item(
            format!("controls_input_{index}"),
            &label,
            "KeyName",
            name_position,
            2,
            &name_font,
            &name_font,
        )?;
        input_names.push(ControlInputName { input, artwork });
    }
    ensure!(input_names.len() >= 100, "controls key-name table is short");
    let capture = labelled_item(
        "controls_capture".into(),
        "???",
        "KeyName",
        name_position,
        2,
        &name_font,
        &name_font,
    )?;
    Ok(MenuControls {
        path: AssetPath::new("fix:/menu/controls/en")?,
        background: AssetRef::new("texture:/Menu/control_back.bmp")?,
        hint,
        tabs,
        keyboard,
        gamepad,
        pictures,
        select_first_y: CONTROLS_SELECT_FIRST_Y,
        input_names,
        capture,
        entries,
        actions,
    })
}

enum InputTerm {
    And(Box<InputTerm>, Box<InputTerm>),
    Or(Box<InputTerm>, Box<InputTerm>),
    Entry(usize),
    NotEntry,
    /// A raw key/pad/axis test outside the options entries.
    Fixed {
        negative: bool,
    },
    Input(ControlInput),
}

/// Fix 7:0 entry actions (IPT_fn_vLoadBinaryFile 004a5060): +0 element count,
/// +4 elements, +8 name, +0xc options name, +0x10 state (-2 at load). Elements
/// are 8 bytes in prefix order: f001 AND, f002 OR, f00a entry, f00b NOT entry,
/// f004/f006/f007 key (pressed/held/not held), f018/f01a pad button, f00e pad
/// direction, f010 axis range.
fn entry_actions(s: &SourceSession) -> Result<(Vec<ControlEntry>, Vec<ControlAction>)> {
    let table = Address {
        owner: Owner::Fix,
        key: BlockKey(7, 0),
        offset: ENTRY_ACTIONS,
    };
    let mut records = Vec::new();
    for index in 0..1024 {
        let at = plus(table, index * 0x1c);
        let Ok(bytes) = s.bytes(at, 0x1c) else { break };
        let state = i32::from_le_bytes(bytes[16..20].try_into().unwrap());
        if state != -2 {
            break;
        }
        let Ok(Some(name)) = s.pointer(at, 8) else {
            break;
        };
        let options = s.pointer(at, 12)?.map(|a| string(s, a)).transpose()?;
        let count = u32_at(bytes, 0)? as usize;
        ensure!(count <= 256, "entry action element count is implausible");
        records.push((at, string(s, name)?, options, count));
    }
    ensure!(records.len() > 400, "Fix 7:0 entry-action table is short");
    let index_of: BTreeMap<Address, usize> =
        records.iter().enumerate().map(|(i, r)| (r.0, i)).collect();
    let parse_entry = |i: usize| -> Result<Option<InputTerm>> {
        let (at, _, _, count) = &records[i];
        if *count == 0 {
            return Ok(None);
        }
        let elements = s.required(*at, 4)?;
        let value = |k: usize| -> Result<u32> { u32_at(s.bytes(plus(elements, k * 8), 4)?, 0) };
        let entry = |k: usize| -> Result<usize> {
            let target = s.required(plus(elements, k * 8), 0)?;
            index_of
                .get(&target)
                .copied()
                .context("entry action references an unknown entry")
        };
        fn term(
            k: &mut usize,
            count: usize,
            value: &dyn Fn(usize) -> Result<u32>,
            entry: &dyn Fn(usize) -> Result<usize>,
        ) -> Result<Option<InputTerm>> {
            ensure!(*k < count, "entry action expression is truncated");
            let opcode = value(*k)?;
            *k += 1;
            Ok(Some(match opcode {
                0xf001 | 0xf002 => {
                    let Some(a) = term(k, count, value, entry)? else {
                        return Ok(None);
                    };
                    let Some(b) = term(k, count, value, entry)? else {
                        return Ok(None);
                    };
                    if opcode == 0xf001 {
                        InputTerm::And(Box::new(a), Box::new(b))
                    } else {
                        InputTerm::Or(Box::new(a), Box::new(b))
                    }
                }
                0xf00a | 0xf00b => {
                    let target = entry(*k)?;
                    *k += 1;
                    if opcode == 0xf00a {
                        InputTerm::Entry(target)
                    } else {
                        InputTerm::NotEntry
                    }
                }
                0xf004 | 0xf006 | 0xf007 => {
                    let code = value(*k)?;
                    *k += 1;
                    if opcode == 0xf006 {
                        InputTerm::Input(ControlInput::Key(u16::try_from(code)?))
                    } else {
                        InputTerm::Fixed {
                            negative: opcode == 0xf007,
                        }
                    }
                }
                0xf018 | 0xf01a => {
                    let button = value(*k + 1)?;
                    *k += 2;
                    // Button codes PadButton0..31 are 0x0a..0x29 (table 0x5ee514);
                    // other codes are fixed menu/inventory pad tests.
                    if opcode == 0xf01a && (0x0a..0x2a).contains(&button) {
                        InputTerm::Input(ControlInput::PadButton(u8::try_from(button - 0x0a)?))
                    } else {
                        InputTerm::Fixed { negative: false }
                    }
                }
                0xf00e => {
                    let direction = value(*k + 1)?;
                    *k += 2;
                    InputTerm::Input(ControlInput::PadDirection(u8::try_from(direction)?))
                }
                0xf010 => {
                    *k += 4;
                    InputTerm::Fixed { negative: false }
                }
                // Typed cheat strings and other device tests are not controls.
                _ => return Ok(None),
            }))
        }
        let mut k = 0;
        let parsed = term(&mut k, *count, &value, &entry)?;
        Ok(parsed.filter(|_| k == *count))
    };
    let terms = (0..records.len())
        .map(parse_entry)
        .collect::<Result<Vec<_>>>()?;
    let direct = |i: usize| match &terms[i] {
        Some(InputTerm::Input(input)) => Some(input.clone()),
        _ => None,
    };
    let entries: Vec<ControlEntry> = records
        .iter()
        .enumerate()
        .filter(|(_, r)| r.2.is_some())
        .map(|(i, r)| ControlEntry {
            name: r.2.clone().unwrap(),
            default: direct(i),
        })
        .collect();
    // OR branches contribute their options entries; a negated guard narrows
    // but does not add input. Two positive operands (a chord) and raw keys
    // outside the options entries have no single remappable input and are
    // left out.
    fn leaves(
        term: &InputTerm,
        terms: &[Option<InputTerm>],
        records: &[(Address, String, Option<String>, usize)],
        visiting: &mut Vec<usize>,
        out: &mut Vec<String>,
    ) {
        let positive =
            |t: &InputTerm| !matches!(t, InputTerm::NotEntry | InputTerm::Fixed { negative: true });
        match term {
            InputTerm::Or(a, b) => {
                leaves(a, terms, records, visiting, out);
                leaves(b, terms, records, visiting, out);
            }
            InputTerm::And(a, b) => match (positive(a), positive(b)) {
                (true, false) => leaves(a, terms, records, visiting, out),
                (false, true) => leaves(b, terms, records, visiting, out),
                _ => {}
            },
            InputTerm::Entry(i) => {
                if let Some(options) = &records[*i].2
                    && matches!(terms[*i], Some(InputTerm::Input(_)))
                {
                    if !out.contains(options) {
                        out.push(options.clone());
                    }
                } else if !visiting.contains(i)
                    && let Some(inner) = &terms[*i]
                {
                    visiting.push(*i);
                    leaves(inner, terms, records, visiting, out);
                    visiting.pop();
                }
            }
            _ => {}
        }
    }
    let mut actions = Vec::new();
    for (i, record) in records.iter().enumerate() {
        let Some(term) = &terms[i] else { continue };
        if matches!(term, InputTerm::Input(_)) {
            continue;
        }
        let mut out = Vec::new();
        leaves(term, &terms, &records, &mut vec![i], &mut out);
        if !out.is_empty() {
            actions.push(ControlAction {
                name: record.1.clone(),
                entries: out,
            });
        }
    }
    Ok((entries, actions))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires local Hype Fix data"]
    fn controls_page_matches_the_original_layout_defaults_and_actions() -> Result<()> {
        let game = std::env::var_os("ASTROLABE_GAME_DIR")
            .map(std::path::PathBuf::from)
            .context("ASTROLABE_GAME_DIR required for the Controls page regression")?;
        let selection = Selection::hype("manoir", "en")?;
        let mut s = SourceSession::open(&game, &selection)?;
        dialogue::load_language(&mut s, &game, &selection)?;
        let controls = controls(&s)?;
        let labels = |rows: &[ControlRow]| -> Vec<(String, Option<String>)> {
            rows.iter()
                .map(|row| (row.item.label.clone(), row.entry.clone()))
                .collect()
        };
        let keyboard = labels(&controls.keyboard);
        ensure!(keyboard.len() == 16 && controls.gamepad.len() == 10);
        ensure!(keyboard[0] == ("move forward".into(), Some("Up_Key".into())));
        ensure!(keyboard[14] == ("look".into(), Some("Look_Key".into())));
        ensure!(keyboard[15] == ("reset to default values".into(), None));
        ensure!(labels(&controls.gamepad)[8] == ("look".into(), Some("ModeLook_Pad".into())));
        ensure!(
            controls
                .tabs
                .iter()
                .map(|t| t.label.as_str())
                .collect::<Vec<_>>()
                == ["keyboard", "gamepad"]
        );
        ensure!(controls.keyboard[0].item.neighbors[0].as_deref() == Some("controls_keyboard_15"));
        let default = |name: &str| {
            controls
                .entries
                .iter()
                .find(|e| e.name == name)
                .and_then(|e| e.default.clone())
        };
        ensure!(default("Up_Key") == Some(ControlInput::Key(0xc8)));
        ensure!(default("Look_Key") == Some(ControlInput::Key(0x52)));
        ensure!(default("Shield_Key") == Some(ControlInput::Key(0x19)));
        ensure!(default("Activate_Map_Key") == Some(ControlInput::Key(0xcf)));
        ensure!(default("SpeedUp_Pad") == Some(ControlInput::PadButton(6)));
        ensure!(default("Up_Pad") == Some(ControlInput::PadDirection(0)));
        let action = |name: &str| {
            controls
                .actions
                .iter()
                .find(|a| a.name == name)
                .map(|a| a.entries.clone())
                .unwrap_or_default()
        };
        ensure!(action("Button_Hit") == ["Action_Key", "Action_Pad"]);
        ensure!(action("BT_Camera_Look") == ["Look_Key", "ModeLook_Pad"]);
        ensure!(action("Button_Left") == ["Left_Key", "Left_Pad"]);
        ensure!(action("Button_SSRight") == ["SSRight_Key"]);
        ensure!(action("Button_ActivateMap") == ["Activate_Map_Key"]);
        ensure!(action("TestInventoryMode") == ["Inventory_Key", "Inventory_Pad"]);
        let name = |input: ControlInput| {
            controls
                .input_names
                .iter()
                .find(|n| n.input == input)
                .map(|n| n.artwork.label.clone())
        };
        ensure!(name(ControlInput::Key(0x52)).as_deref() == Some("pad0"));
        ensure!(name(ControlInput::Key(0x2a)).as_deref() == Some("left shift"));
        ensure!(name(ControlInput::PadButton(0)).as_deref() == Some("b1"));
        ensure!(
            name(ControlInput::Key(0x01)).is_none(),
            "escape has no display name"
        );
        ensure!(
            controls
                .pictures
                .iter()
                .map(|p| p.texture.path.as_str())
                .collect::<Vec<_>>()
                == [
                    "texture:/multilanguage/English/fixtex/menus/controlsmenu/contr_ta.tga",
                    "texture:/multilanguage/English/fixtex/menus/controlsmenu/contr_tb.tga",
                    "texture:/multilanguage/English/fixtex/menus/controlsmenu/contr_tc.tga",
                    "texture:/multilanguage/English/fixtex/menus/controlsmenu/contr_td.tga",
                    "texture:/fixtex/menus/controlsmenu/kboard.tga",
                    "texture:/fixtex/menus/controlsmenu/gamepad.tga",
                    "texture:/fixtex/menus/controlsmenu/select.tga"
                ]
        );
        ensure!(
            controls
                .hint
                .starts_with("Use the arrows to select the action")
        );
        Ok(())
    }
}
