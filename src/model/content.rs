//! Content kinds and in-memory session loading shared by all adapters.
use super::{
    AssetPath, AssetRef,
    io::{Content, Load},
};
use super::{
    audio::{Sound, SoundBank, SoundDefinitions},
    geometry::{Material, Model},
    texture::Texture,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

impl Content for Texture {
    const KIND: &'static str = "texture";
    fn validate(&self) -> Result<()> {
        Texture::validate(self)
    }
}
impl Content for Sound {
    const KIND: &'static str = "sound";
    fn validate(&self) -> Result<()> {
        Sound::validate(self)
    }
}
impl Content for SoundBank {
    const KIND: &'static str = "sound-bank";
    fn validate(&self) -> Result<()> {
        SoundBank::validate(self)
    }
}
impl Content for SoundDefinitions {
    const KIND: &'static str = "sound-definitions";
    fn validate(&self) -> Result<()> {
        SoundDefinitions::validate(self)
    }
}
impl Content for Model {
    const KIND: &'static str = "model";
    fn validate(&self) -> Result<()> {
        Model::validate(self)
    }
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
}
impl Content for Material {
    const KIND: &'static str = "material";
    fn validate(&self) -> Result<()> {
        Material::validate(self)
    }
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
}

/// Ordered archive membership. Shared entries point at the same independently stored texture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextureSet {
    pub name: String,
    pub directories: Vec<String>,
    pub entries: Vec<TextureEntry>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextureEntry {
    pub name: String,
    pub texture: AssetRef<Texture>,
}
impl Content for TextureSet {
    const KIND: &'static str = "texture-set";
    fn validate(&self) -> Result<()> {
        ensure!(
            self.name == "Textures.cnt" || self.name == "Vignette.cnt",
            "unsupported texture archive name"
        );
        let mut names = BTreeSet::new();
        for entry in &self.entries {
            AssetPath::new(format!("texture:/{}", entry.name))?;
            ensure!(
                names.insert(entry.name.to_lowercase()),
                "duplicate texture member {}",
                entry.name
            );
            ensure!(
                entry.texture.path.role() == super::PackageRole::Texture,
                "texture has wrong package role"
            );
        }
        Ok(())
    }
}

/// A conversion session can retain its useful content in memory instead of writing Rete.
#[derive(Default)]
pub struct Collection {
    assets: BTreeMap<AssetPath, Arc<dyn Any + Send + Sync>>,
}
impl Collection {
    pub fn insert<T: Content>(&mut self, reference: &AssetRef<T>, value: T) -> Result<()> {
        value.validate()?;
        value.validate_reference(&reference.path)?;
        ensure!(
            reference.path.member().is_none(),
            "insert the owning asset rather than a member"
        );
        self.assets.insert(reference.path.clone(), Arc::new(value));
        Ok(())
    }
    pub fn contains(&self, path: &AssetPath) -> bool {
        self.assets.contains_key(path)
    }
}
impl<T: Content> Load<T> for Collection {
    fn load(&mut self, reference: &AssetRef<T>) -> Result<Arc<T>> {
        Arc::clone(
            self.assets
                .get(&reference.path)
                .with_context(|| format!("missing content {}", reference.path))?,
        )
        .downcast::<T>()
        .map_err(|_| anyhow::anyhow!("wrong content kind at {}", reference.path))
    }
}
