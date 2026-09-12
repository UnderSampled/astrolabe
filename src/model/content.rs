//! Content kinds and in-memory session loading shared by all adapters.
use super::{
    AssetPath, AssetRef,
    io::{Content, Load},
};
use super::{
    audio::{Sound, SoundBank, SoundDefinitions},
    geometry::{Material, Model, ModelInstance},
    texture::{Texture, TextureSet},
};
use anyhow::{Context, Result, ensure};
use std::{any::Any, collections::BTreeMap, sync::Arc};

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

/// Resolver for the built-in content graph. Individual codecs/consumers can keep
/// narrower Load<T> bounds; package exporters handle every built-in root kind.
pub trait ContentSource:
    Load<Texture>
    + Load<TextureSet>
    + Load<Sound>
    + Load<SoundBank>
    + Load<SoundDefinitions>
    + Load<Model>
    + Load<ModelInstance>
    + Load<Material>
{
}
impl<T> ContentSource for T where
    T: Load<Texture>
        + Load<TextureSet>
        + Load<Sound>
        + Load<SoundBank>
        + Load<SoundDefinitions>
        + Load<Model>
        + Load<ModelInstance>
        + Load<Material>
{
}

/// Loaded built-in content. Exhaustive adapter matches make new domains visible
/// at each target boundary. Custom Content types still use Load/Save directly.
pub enum Asset {
    Texture(Arc<Texture>),
    TextureSet(Arc<TextureSet>),
    Sound(Arc<Sound>),
    SoundBank(Arc<SoundBank>),
    SoundDefinitions(Arc<SoundDefinitions>),
    Model(Arc<Model>),
    ModelInstance(Arc<ModelInstance>),
    Material(Arc<Material>),
}
impl Asset {
    pub fn load(source: &mut impl ContentSource, root: &super::io::ContentRoot) -> Result<Self> {
        fn load<T: Content>(source: &mut impl Load<T>, path: &AssetPath) -> Result<Arc<T>> {
            let value = source.load(&AssetRef::from_path(path.clone()))?;
            value.validate()?;
            value.validate_reference(path)?;
            Ok(value)
        }
        Ok(match root.kind.as_str() {
            Texture::KIND => Self::Texture(load(source, &root.reference)?),
            TextureSet::KIND => Self::TextureSet(load(source, &root.reference)?),
            Sound::KIND => Self::Sound(load(source, &root.reference)?),
            SoundBank::KIND => Self::SoundBank(load(source, &root.reference)?),
            SoundDefinitions::KIND => Self::SoundDefinitions(load(source, &root.reference)?),
            Model::KIND => Self::Model(load(source, &root.reference)?),
            ModelInstance::KIND => Self::ModelInstance(load(source, &root.reference)?),
            Material::KIND => Self::Material(load(source, &root.reference)?),
            other => anyhow::bail!("unsupported useful content kind {other}"),
        })
    }
}
