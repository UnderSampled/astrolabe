//! Content kinds and in-memory session loading shared by all adapters.
use super::ai::AiModel;
use super::dialogue::{Dialogue, DialoguePresentation};
use super::environment::{Environment, Fog, PhysicalEnvironment, VisualEnvironment};
use super::mechanics::Mechanics;
use super::route::{Route, Waypoint};
use super::{
    AssetPath, AssetRef,
    io::{Content, Load},
};
use super::{
    animation::{AnimationAction, AnimationClip, TransformPool},
    character::{CollisionProfile, Family, ObjectList, Perso},
};
use super::{
    audio::{Sound, SoundBank, SoundDefinitions},
    collision::Collision,
    geometry::{Material, Model, ModelInstance},
    scene::Scene,
    texture::{Texture, TextureSet},
};
use super::{light::Light, sprite::Sprite};
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
    + Load<Scene>
    + Load<AnimationAction>
    + Load<AnimationClip>
    + Load<TransformPool>
    + Load<Family>
    + Load<ObjectList>
    + Load<Perso>
    + Load<Route>
    + Load<Waypoint>
    + Load<Dialogue>
    + Load<DialoguePresentation>
    + Load<AiModel>
    + Load<Collision>
    + Load<CollisionProfile>
    + Load<Environment>
    + Load<PhysicalEnvironment>
    + Load<VisualEnvironment>
    + Load<Fog>
    + Load<Light>
    + Load<Mechanics>
    + Load<Sprite>
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
        + Load<Scene>
        + Load<AnimationAction>
        + Load<AnimationClip>
        + Load<TransformPool>
        + Load<Family>
        + Load<ObjectList>
        + Load<Perso>
        + Load<Route>
        + Load<Waypoint>
        + Load<Dialogue>
        + Load<DialoguePresentation>
        + Load<AiModel>
        + Load<Collision>
        + Load<CollisionProfile>
        + Load<Environment>
        + Load<PhysicalEnvironment>
        + Load<VisualEnvironment>
        + Load<Fog>
        + Load<Light>
        + Load<Mechanics>
        + Load<Sprite>
{
}

/// Loaded built-in content. Exhaustive adapter matches make new domains visible
/// at each target boundary. Custom Content types still use Load/Save directly.
pub enum Asset {
    Dialogue(Arc<Dialogue>),
    DialoguePresentation(Arc<DialoguePresentation>),
    AiModel(Arc<AiModel>),
    Environment(Arc<Environment>),
    PhysicalEnvironment(Arc<PhysicalEnvironment>),
    VisualEnvironment(Arc<VisualEnvironment>),
    Fog(Arc<Fog>),
    Light(Arc<Light>),
    Mechanics(Arc<Mechanics>),
    Sprite(Arc<Sprite>),
    Texture(Arc<Texture>),
    TextureSet(Arc<TextureSet>),
    Sound(Arc<Sound>),
    SoundBank(Arc<SoundBank>),
    SoundDefinitions(Arc<SoundDefinitions>),
    Model(Arc<Model>),
    ModelInstance(Arc<ModelInstance>),
    Material(Arc<Material>),
    Scene(Arc<Scene>),
    Animation(Arc<AnimationClip>),
    AnimationAction(Arc<AnimationAction>),
    TransformPool(Arc<TransformPool>),
    Family(Arc<Family>),
    ObjectList(Arc<ObjectList>),
    Perso(Arc<Perso>),
    Route(Arc<Route>),
    Waypoint(Arc<Waypoint>),
    Collision(Arc<Collision>),
    CollisionProfile(Arc<CollisionProfile>),
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
            Dialogue::KIND => Self::Dialogue(load(source, &root.reference)?),
            DialoguePresentation::KIND => {
                Self::DialoguePresentation(load(source, &root.reference)?)
            }
            AiModel::KIND => Self::AiModel(load(source, &root.reference)?),
            Environment::KIND => Self::Environment(load(source, &root.reference)?),
            PhysicalEnvironment::KIND => Self::PhysicalEnvironment(load(source, &root.reference)?),
            VisualEnvironment::KIND => Self::VisualEnvironment(load(source, &root.reference)?),
            Fog::KIND => Self::Fog(load(source, &root.reference)?),
            Light::KIND => Self::Light(load(source, &root.reference)?),
            Mechanics::KIND => Self::Mechanics(load(source, &root.reference)?),
            Sprite::KIND => Self::Sprite(load(source, &root.reference)?),
            Texture::KIND => Self::Texture(load(source, &root.reference)?),
            TextureSet::KIND => Self::TextureSet(load(source, &root.reference)?),
            Sound::KIND => Self::Sound(load(source, &root.reference)?),
            SoundBank::KIND => Self::SoundBank(load(source, &root.reference)?),
            SoundDefinitions::KIND => Self::SoundDefinitions(load(source, &root.reference)?),
            Model::KIND => Self::Model(load(source, &root.reference)?),
            ModelInstance::KIND => Self::ModelInstance(load(source, &root.reference)?),
            AnimationAction::KIND => Self::AnimationAction(load(source, &root.reference)?),
            AnimationClip::KIND => Self::Animation(load(source, &root.reference)?),
            TransformPool::KIND => Self::TransformPool(load(source, &root.reference)?),
            Family::KIND => Self::Family(load(source, &root.reference)?),
            ObjectList::KIND => Self::ObjectList(load(source, &root.reference)?),
            Perso::KIND => Self::Perso(load(source, &root.reference)?),
            Route::KIND => Self::Route(load(source, &root.reference)?),
            Waypoint::KIND => Self::Waypoint(load(source, &root.reference)?),
            CollisionProfile::KIND => Self::CollisionProfile(load(source, &root.reference)?),
            Collision::KIND => Self::Collision(load(source, &root.reference)?),
            Scene::KIND => Self::Scene(load(source, &root.reference)?),
            Material::KIND => Self::Material(load(source, &root.reference)?),
            other => anyhow::bail!("unsupported useful content kind {other}"),
        })
    }
}
