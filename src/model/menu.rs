//! Source-informed menu artwork. Authored Controls and application actions belong to Godot.
use super::{AssetPath, AssetRef, audio::Sound, io::Content};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuPresentation {
    pub path: AssetPath,
    pub locale: String,
    /// None is a source text-only overlay over the existing session viewport.
    pub background: Option<AssetRef<super::texture::Texture>>,
    pub initial_item: String,
    pub items: Vec<MenuItem>,
    pub navigation: AssetRef<Sound>,
    pub confirmation: AssetRef<Sound>,
    pub navigation_gain: f32,
    pub confirmation_gain: f32,
    /// Source configuration-root labels and backdrop shipped with the English main menu.
    #[serde(default)]
    pub configuration: Option<MenuConfiguration>,
    /// Original English credits registry, attached to the main menu presentation.
    #[serde(default)]
    pub credits: Option<CreditsPresentation>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuConfiguration {
    pub path: AssetPath,
    pub background: AssetRef<super::texture::Texture>,
    /// Original configuration text items. Repeated source actions identify dynamic rows.
    pub initial_item: String,
    pub items: Vec<MenuItem>,
    /// Slider1..3 source positions before applying persisted option values.
    pub slider_positions: [[f32; 3]; 3],
    /// The root's unnamed sprites: the localized stained-glass title tiles.
    #[serde(default)]
    pub title: Vec<MenuPicture>,
    /// The separate `Menu/control_back.bmp` root reached from the Controls item.
    #[serde(default)]
    pub controls: Option<MenuControls>,
}
/// Original key-remapping page: Fix 18:0 layout plus the input module's
/// user-facing entry actions, defaults and key names from Fix 7:0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuControls {
    pub path: AssetPath,
    pub background: AssetRef<super::texture::Texture>,
    /// The root's F1 help text.
    pub hint: String,
    /// Keyboard then Gamepad; left/right linked, no up/down.
    pub tabs: Vec<MenuItem>,
    pub keyboard: Vec<ControlRow>,
    pub gamepad: Vec<ControlRow>,
    /// DisplayWithKeyboard, DisplayWithGamepad and SelectBox sprites.
    pub pictures: Vec<MenuPicture>,
    /// SelectBox centre on the first row, set by FUN_00414b00 when a list opens.
    pub select_first_y: f32,
    /// Lowercased key/button names drawn right-aligned left of each row label.
    pub input_names: Vec<ControlInputName>,
    /// The "???" shown while the focused row waits for a key.
    pub capture: MenuItem,
    /// Every options-named entry in table order (saved by the source options file).
    pub entries: Vec<ControlEntry>,
    /// Script-facing actions and the options entries their expressions OR together.
    pub actions: Vec<ControlAction>,
}
impl MenuControls {
    /// Tabs, rows, key names and the capture marker: every baked label.
    pub fn items(&self) -> impl Iterator<Item = &MenuItem> {
        self.tabs
            .iter()
            .chain(
                self.keyboard
                    .iter()
                    .chain(&self.gamepad)
                    .map(|row| &row.item),
            )
            .chain(self.input_names.iter().map(|name| &name.artwork))
            .chain(std::iter::once(&self.capture))
    }
    pub fn items_mut(&mut self) -> impl Iterator<Item = &mut MenuItem> {
        self.tabs
            .iter_mut()
            .chain(
                self.keyboard
                    .iter_mut()
                    .chain(&mut self.gamepad)
                    .map(|row| &mut row.item),
            )
            .chain(self.input_names.iter_mut().map(|name| &mut name.artwork))
            .chain(std::iter::once(&mut self.capture))
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlRow {
    /// `KeyStrokes` rows bind `entry`; `DefaultValues` has none.
    pub item: MenuItem,
    pub entry: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuPicture {
    pub name: String,
    pub texture: AssetRef<super::texture::Texture>,
    /// Centre and size in the 640x480 canvas.
    pub center: [f32; 2],
    pub size: [f32; 2],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ControlInput {
    /// DirectInput scancode.
    Key(u16),
    /// Source pad button index (PadButton0 is "b1").
    PadButton(u8),
    /// Fixed pad direction 0..3 (up, down, left, right); not remappable.
    PadDirection(u8),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlEntry {
    pub name: String,
    pub default: Option<ControlInput>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlInputName {
    pub input: ControlInput,
    /// Artwork right-aligned at the key column, positioned on the first row.
    pub artwork: MenuItem,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlAction {
    pub name: String,
    pub entries: Vec<String>,
    /// Fixed joystick ranges the expression also ORs in.
    #[serde(default)]
    pub axes: Vec<ControlAxis>,
}
/// A joystick axis (0 X, 1 Y, 2 Z, 3..5 rotations) inside [min, max] on the
/// source +-100 scale (IPT opcode f010, handler 004a7c20).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlAxis {
    pub axis: u8,
    pub min: i16,
    pub max: i16,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuItem {
    pub name: String,
    pub label: String,
    pub action: String,
    /// Source UI anchor, projected into the virtual pixel canvas.
    pub position: [f32; 2],
    pub horizontal_alignment: u8,
    /// Up, down, left, right action identities; null means no source link.
    pub neighbors: [Option<String>; 4],
    /// Union of artwork states relative to position: x, y, width, height.
    pub rect: [f32; 4],
    pub normal: Vec<MenuSprite>,
    pub selected: Vec<MenuSprite>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuSprite {
    pub texture: AssetRef<super::texture::Texture>,
    /// Top-left pixel rectangle relative to the item's union rectangle.
    pub rect: [f32; 4],
    /// Bottom-left source UV rectangle; one-texel extrapolation is retained.
    pub uv: [[f32; 2]; 2],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreditsPresentation {
    pub path: AssetPath,
    pub locale: String,
    pub font: CreditsFont,
    /// Includes all 400 registered entries; the source routine displays only the first count.
    pub entries: Vec<CreditsEntry>,
    pub displayed_entries: usize,
    pub max_visible_lines: usize,
    pub text_spacing: f32,
    pub text_position: [f32; 2],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreditsFont {
    pub name: String,
    pub atlas: super::dialogue::DialogueTexture,
    pub glyphs: Vec<super::dialogue::DialogueGlyph>,
    pub tracking: f32,
    pub color: [f32; 4],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreditsEntry {
    /// The full key, including source +/- alignment markers.
    pub key: String,
    pub text: String,
    /// FON_fn_vDrawText3D text alignment: 0 left, 1 center, 2 right.
    pub alignment: u8,
}
impl CreditsPresentation {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.locale == "en"
                && self.path.role() == super::PackageRole::Fix
                && !self.font.name.is_empty()
                && self.font.name == "Playmo_Font2_0"
                && self.entries.len() == 400
                && self.displayed_entries == 397
                && self.displayed_entries <= self.entries.len()
                && self.max_visible_lines == 40
                && self.text_spacing.is_finite()
                && self.text_spacing > 0.0
                && self.text_position.iter().all(|v| v.is_finite()),
            "unsupported original credits profile"
        );
        ensure!(
            self.font.atlas.texture.role() == super::PackageRole::Texture
                && self.font.atlas.size.iter().all(|v| (2..=4096).contains(v))
                && self.font.tracking.is_finite()
                && self.font.tracking >= 0.0
                && self
                    .font
                    .color
                    .iter()
                    .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
                && !self.font.glyphs.is_empty()
                && self.font.glyphs.len() <= 1024,
            "invalid original credits font"
        );
        let mut glyphs = std::collections::BTreeSet::new();
        for glyph in &self.font.glyphs {
            ensure!(
                glyphs.insert(glyph.character)
                    && [glyph.width, glyph.half_height, glyph.vertical_offset]
                        .iter()
                        .all(|v| v.is_finite())
                    && glyph.width >= 0.0
                    && glyph.half_height <= 0.0,
                "invalid original credits glyph"
            );
            if let Some(uv) = glyph.uv {
                ensure!(
                    glyph.width > 0.0
                        && glyph.half_height < 0.0
                        && uv.iter().flatten().all(|v| v.is_finite())
                        && uv[1].iter().all(|v| *v > 0.0),
                    "invalid original credits glyph UV"
                );
            }
        }
        let mut keys = std::collections::BTreeSet::new();
        for entry in &self.entries {
            ensure!(
                !entry.key.is_empty()
                    && keys.insert(&entry.key)
                    && entry.key.len() <= 24
                    && !entry.text.is_empty()
                    && entry.text.len() <= 512
                    && entry.alignment <= 2,
                "invalid original credits entry"
            );
            let expected_alignment = match entry.key.as_bytes().first() {
                Some(b'+') => 2,
                Some(b'-') => 0,
                _ => 1,
            };
            ensure!(
                entry.alignment == expected_alignment,
                "credits alignment differs from key marker"
            );
        }
        Ok(())
    }
}
impl Content for MenuPresentation {
    const KIND: &'static str = "menu-presentation";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            self.locale == "en" && (1..=32).contains(&self.items.len()),
            "unsupported menu profile"
        );
        ensure!(
            self.background
                .as_ref()
                .is_none_or(|r| r.path.role() == super::PackageRole::Texture)
                && [self.navigation_gain, self.confirmation_gain]
                    .iter()
                    .all(|v| v.is_finite() && (0.0..=1.0).contains(v)),
            "invalid menu media binding"
        );
        let actions: std::collections::BTreeSet<_> =
            self.items.iter().map(|i| i.action.as_str()).collect();
        ensure!(
            actions.len() == self.items.len() && actions.contains(self.initial_item.as_str()),
            "invalid menu action/initial selection"
        );
        let mut names = std::collections::BTreeSet::new();
        for item in &self.items {
            ensure!(
                !item.action.is_empty()
                    && item.position.iter().all(|v| v.is_finite())
                    && item.horizontal_alignment <= 2
                    && item
                        .neighbors
                        .iter()
                        .flatten()
                        .all(|a| actions.contains(a.as_str())),
                "invalid menu layout/navigation"
            );
            ensure!(
                names.insert(&item.name) && !item.label.is_empty() && item.label.len() <= 64,
                "invalid menu item identity"
            );
            ensure!(
                item.name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b == b'_'),
                "unsafe menu item name"
            );
            ensure!(
                item.rect.iter().all(|v| v.is_finite())
                    && item.rect[2] > 0.
                    && item.rect[3] > 0.
                    && item.rect[2] <= 640.
                    && item.rect[3] <= 480.,
                "invalid menu item rectangle"
            );
            for sprites in [&item.normal, &item.selected] {
                ensure!(
                    !sprites.is_empty() && sprites.len() <= 64,
                    "invalid menu artwork count"
                );
                for sprite in sprites {
                    ensure!(
                        sprite.texture.path.role() == super::PackageRole::Texture,
                        "menu sprite has wrong asset role"
                    );
                    ensure!(
                        sprite.rect.iter().all(|v| v.is_finite())
                            && sprite.rect[0] >= 0.
                            && sprite.rect[1] >= 0.
                            && sprite.rect[2] > 0.
                            && sprite.rect[3] > 0.
                            && sprite.rect[0] + sprite.rect[2] <= item.rect[2] + 0.001
                            && sprite.rect[1] + sprite.rect[3] <= item.rect[3] + 0.001,
                        "menu sprite exceeds item rectangle"
                    );
                    ensure!(
                        sprite
                            .uv
                            .iter()
                            .flatten()
                            .all(|v| v.is_finite() && (0.0..=1.01).contains(v))
                            && sprite.uv[1].iter().all(|v| *v > 0.),
                        "invalid menu sprite UV"
                    );
                }
            }
        }
        ensure!(
            self.navigation.path.role() == super::PackageRole::Sound
                && self.confirmation.path.role() == super::PackageRole::Sound,
            "menu audio has wrong asset role"
        );
        if let Some(configuration) = &self.configuration {
            ensure!(
                configuration.background.path.role() == super::PackageRole::Texture
                    && (1..=32).contains(&configuration.items.len()),
                "invalid source configuration media/layout"
            );
            ensure!(
                configuration
                    .slider_positions
                    .iter()
                    .flatten()
                    .all(|value| value.is_finite()),
                "invalid source configuration slider positions"
            );
            let mut configuration_names = std::collections::BTreeSet::new();
            ensure!(
                configuration
                    .items
                    .iter()
                    .any(|item| item.name == configuration.initial_item),
                "configuration initial item absent"
            );
            for item in &configuration.items {
                ensure!(
                    configuration_names.insert(&item.name)
                        && !item.action.is_empty()
                        && !item.label.is_empty()
                        && item.label.len() <= 64
                        && item
                            .name
                            .bytes()
                            .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
                        && item.position.iter().all(|v| v.is_finite())
                        && item.horizontal_alignment <= 2
                        && item.rect.iter().all(|v| v.is_finite())
                        && item.rect[0].is_finite()
                        && item.rect[1].is_finite()
                        && item.rect[2] > 0.
                        && item.rect[3] > 0.
                        && item.rect[2] <= 640.
                        && item.rect[3] <= 480.
                        && !item.normal.is_empty()
                        && !item.selected.is_empty(),
                    "invalid source configuration item"
                );
            }
            ensure!(
                configuration.items.iter().all(|item| item
                    .neighbors
                    .iter()
                    .flatten()
                    .all(|name| configuration_names.contains(name))),
                "configuration navigation target absent"
            );
            for item in &configuration.items {
                for sprite in item.normal.iter().chain(&item.selected) {
                    ensure!(
                        sprite.texture.path.role() == super::PackageRole::Texture
                            && sprite.rect.iter().all(|v| v.is_finite())
                            && sprite.rect[0] >= 0.
                            && sprite.rect[1] >= 0.
                            && sprite.rect[2] > 0.
                            && sprite.rect[3] > 0.
                            && sprite.rect[0] + sprite.rect[2] <= item.rect[2] + 0.001
                            && sprite.rect[1] + sprite.rect[3] <= item.rect[3] + 0.001
                            && sprite.uv.iter().flatten().all(|v| v.is_finite()),
                        "invalid source configuration sprite"
                    );
                }
            }
        }
        if let Some(credits) = &self.credits {
            credits.validate()?;
        }
        Ok(())
    }
}
