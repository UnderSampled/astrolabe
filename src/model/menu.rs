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
        Ok(())
    }
}
