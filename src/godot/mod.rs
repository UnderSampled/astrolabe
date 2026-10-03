//! Godot resources derived from useful model values; no OpenSpace or Rete internals.
use crate::model::{AssetPath, Selection};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

mod collision;
mod dialogue;
mod dialogue_presentation;
mod engine;
mod material;
mod project;
mod publication;
mod resources;
mod route;
mod scene;
pub mod support;
pub use engine::{gameplay_headless, run_godot, run_godot_gameplay};
pub use project::Project;

/// Godot is Y-up and right-handed. Canonical Hype values remain untouched.
pub fn vector([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, z, -y]
}
fn vec3(v: [f32; 3]) -> String {
    format!("Vector3({},{},{})", v[0], v[1], v[2])
}
fn vec2(v: [f32; 2]) -> String {
    format!("Vector2({},{})", v[0], v[1])
}
fn quoted(value: &str) -> String {
    serde_json::to_string(value).expect("strings serialize")
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn whole_asset(id: &AssetPath) -> Result<()> {
    ensure!(
        id.member().is_none(),
        "Godot saves whole assets; member reference requires owner resolution: {id}"
    );
    Ok(())
}
fn disk_path(selection: &Selection, id: &AssetPath, extension: &str) -> String {
    let role = match id.role() {
        crate::model::PackageRole::Fix => "shared".into(),
        crate::model::PackageRole::Level => format!("levels/{}", selection.level),
        crate::model::PackageRole::Texture => "textures".into(),
        crate::model::PackageRole::Sound => "sounds".into(),
    };
    format!("generated/hype/{role}/{}.{extension}", id.file())
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    pub schema: u32,
    pub godot_version: String,
    pub files: BTreeMap<String, String>,
    pub diagnostics: Vec<String>,
    #[serde(default)]
    pub support_interface: u32,
    /// Active-level closures; a shared resource can have several owners.
    #[serde(default)]
    pub selections: BTreeMap<String, std::collections::BTreeSet<String>>,
}

mod export;
pub use export::export;

pub mod ai;
mod animation;
mod character;
mod sprite;

mod light;

mod sector;
mod sound;

mod environment;

pub mod action;

mod bitmap_font;
mod inventory;
mod menu;
mod particle;
