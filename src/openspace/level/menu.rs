//! Bounded English menu presentation import. Source records end at this boundary.
use super::{container::wire, dialogue, records::*, session::SourceSession, sound::SoundSites};
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
        if h.encoding != 0x11 || h.count != 71 {
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
fn artwork(font: &Font, label: &str, [x, y, z]: [f32; 3]) -> Result<Vec<MenuSprite>> {
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
    let mut center = x - width * 0.5;
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
        depth.is_finite() && (0.0..=1.0).contains(&depth),
        "invalid menu text depth"
    );
    let factor = 1. - depth * 0.16666667;
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
    };
    presentation.validate()?;
    Ok((presentation, sounds))
}
