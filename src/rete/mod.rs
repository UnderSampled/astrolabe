//! Per-content MessagePack files. The filesystem is the persistent lookup mechanism.
use crate::model::{
    AssetPath, AssetRef, Selection,
    io::{Content, Diagnostic, Load, Save},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    any::Any,
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

pub const SCHEMA: u32 = 47;
const MAGIC: &str = "astrolabe-rete";
const MAX_ASSET_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct Envelope<T> {
    format: String,
    schema: u32,
    kind: String,
    value: T,
}

/// This is a root list, not an index of engine allocations or a second asset database.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Package {
    pub selection: Selection,
    pub roots: Vec<Root>,
    /// Semantic source coverage survives save/reload independently of transient addresses.
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
}
pub use crate::model::io::ContentRoot as Root;
impl Content for Package {
    const KIND: &'static str = "package";
    fn validate(&self) -> Result<()> {
        ensure!(
            self.selection.profile == "hype-pc-montreal",
            "unsupported source profile"
        );
        let valid = Selection::hype(self.selection.level.clone(), self.selection.locale.clone())?;
        ensure!(valid == self.selection, "invalid package selection");
        for diagnostic in &self.diagnostics {
            diagnostic.validate()?;
        }
        let mut roots = std::collections::BTreeSet::new();
        for root in &self.roots {
            ensure!(!root.kind.trim().is_empty(), "empty content kind");
            ensure!(
                root.reference.member().is_none(),
                "package roots must identify whole assets"
            );
            ensure!(
                roots.insert(&root.reference),
                "duplicate/conflicting package root {}",
                root.reference
            );
        }
        Ok(())
    }
}

pub fn encode<T: Content + Serialize>(value: &T) -> Result<Vec<u8>> {
    value.validate()?;
    let envelope = Envelope {
        format: MAGIC.into(),
        schema: SCHEMA,
        kind: T::KIND.into(),
        value,
    };
    let bytes = rmp_serde::to_vec_named(&envelope).context("serialize Rete content")?;
    ensure!(
        bytes.len() as u64 <= MAX_ASSET_BYTES,
        "Rete asset exceeds size limit"
    );
    Ok(bytes)
}

pub fn decode<T: Content + DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let value = decode_unvalidated::<T>(bytes, T::KIND)?;
    value.validate()?;
    Ok(value)
}

fn decode_unvalidated<T: DeserializeOwned>(bytes: &[u8], expected_kind: &str) -> Result<T> {
    ensure!(
        bytes.len() as u64 <= MAX_ASSET_BYTES,
        "Rete asset exceeds size limit"
    );
    let mut cursor = std::io::Cursor::new(bytes);
    let envelope: Envelope<T> = rmp_serde::from_read(&mut cursor).context("decode Rete content")?;
    ensure!(
        cursor.position() == bytes.len() as u64,
        "trailing data after Rete asset"
    );
    ensure!(envelope.format == MAGIC, "not an Astrolabe Rete asset");
    ensure!(
        envelope.schema == SCHEMA,
        "unsupported Rete schema {}",
        envelope.schema
    );
    ensure!(
        envelope.kind == expected_kind,
        "Rete kind {} cannot load as {}",
        envelope.kind,
        expected_kind
    );
    Ok(envelope.value)
}

pub struct Store {
    root: PathBuf,
    selection: Selection,
    // Session-local whole-asset cache; it is never serialized.
    cache: HashMap<AssetPath, Arc<dyn Any + Send + Sync>>,
}
impl Store {
    pub fn new(root: impl Into<PathBuf>, selection: Selection) -> Result<Self> {
        Selection::hype(selection.level.clone(), selection.locale.clone())?;
        ensure!(
            selection.profile == "hype-pc-montreal",
            "unsupported source profile"
        );
        Ok(Self {
            root: root.into(),
            selection,
            cache: HashMap::new(),
        })
    }
    pub fn selection(&self) -> &Selection {
        &self.selection
    }
    pub fn select(&mut self, selection: Selection) -> Result<()> {
        Selection::hype(selection.level.clone(), selection.locale.clone())?;
        ensure!(
            selection.profile == "hype-pc-montreal",
            "unsupported source profile"
        );
        self.selection = selection;
        // Fix links can depend on the selected level. Never retain bound objects across sessions.
        self.cache.clear();
        Ok(())
    }
    pub fn path(&self, reference: &AssetPath) -> PathBuf {
        let role = match reference.role() {
            crate::model::PackageRole::Fix => PathBuf::from("fix"),
            crate::model::PackageRole::Level => PathBuf::from("levels").join(&self.selection.level),
            crate::model::PackageRole::Texture => PathBuf::from("textures"),
            crate::model::PackageRole::Sound => PathBuf::from("sounds"),
        };
        self.root
            .join(role)
            .join(format!("{}.rete", reference.file()))
    }
    pub fn save_package(&self, package: &Package) -> Result<()> {
        ensure!(
            package.selection == self.selection,
            "package selection differs from store"
        );
        write_file(&self.root.join("package.rete"), &encode(package)?)
    }
    pub fn open(root: impl Into<PathBuf>) -> Result<(Self, Package)> {
        let root = root.into();
        let package: Package = decode(&read_file(&root.join("package.rete"))?)?;
        let store = Self::new(root, package.selection.clone())?;
        Ok((store, package))
    }
}

impl<T: Content + DeserializeOwned> Load<T> for Store {
    fn load(&mut self, reference: &AssetRef<T>) -> Result<Arc<T>> {
        ensure!(
            reference.path.member().is_none(),
            "load the owning asset before resolving member {}",
            reference.path
        );
        if let Some(value) = self.cache.get(&reference.path) {
            return Arc::clone(value)
                .downcast::<T>()
                .map_err(|_| anyhow::anyhow!("cached asset kind mismatch at {}", reference.path));
        }
        let value = Arc::new(
            decode::<T>(&read_file(&self.path(&reference.path))?)
                .with_context(|| format!("load {}", reference.path))?,
        );
        value.validate_reference(&reference.path)?;
        self.cache.insert(reference.path.clone(), value.clone());
        Ok(value)
    }
}
impl<T: Content + Serialize> Save<T> for Store {
    fn save(&mut self, reference: &AssetRef<T>, value: &T) -> Result<()> {
        ensure!(
            reference.path.member().is_none(),
            "save a whole owning asset, not member {}",
            reference.path
        );
        value.validate_reference(&reference.path)?;
        write_file(&self.path(&reference.path), &encode(value)?)?;
        self.cache.remove(&reference.path);
        Ok(())
    }
}

fn read_file(path: &Path) -> Result<Vec<u8>> {
    ensure!(
        path.metadata()
            .with_context(|| format!("stat {}", path.display()))?
            .len()
            <= MAX_ASSET_BYTES,
        "asset too large"
    );
    fs::read(path).with_context(|| format!("read {}", path.display()))
}

pub(crate) use crate::files::write_atomic as write_file;

/// Persist the selected dependency closure, with no dependency on the importer.
pub fn save_package(
    source: &mut impl crate::model::content::ContentSource,
    inventory: &[Root],
    package: &Package,
    output: &Path,
) -> Result<()> {
    let mut store = Store::new(output, package.selection.clone())?;
    for entry in inventory {
        let asset = crate::model::content::Asset::load(source, entry)?;
        store.save_asset(&entry.reference, &asset)?;
    }
    store.save_package(package)
}
impl Store {
    pub fn save_asset(
        &mut self,
        path: &AssetPath,
        asset: &crate::model::content::Asset,
    ) -> Result<()> {
        use crate::model::content::Asset;
        match asset {
            Asset::ArmorCatalog(value) => {
                self.save(&AssetRef::from_path(path.clone()), value.as_ref())
            }
            Asset::InventoryPresentation(value) => {
                self.save(&AssetRef::from_path(path.clone()), value.as_ref())
            }
            Asset::ParticleGenerator(value) => {
                self.save(&AssetRef::from_path(path.clone()), value.as_ref())
            }
            Asset::MenuPresentation(value) => {
                self.save(&AssetRef::from_path(path.clone()), value.as_ref())
            }
            Asset::HudPresentation(value) => {
                self.save(&AssetRef::from_path(path.clone()), value.as_ref())
            }
            Asset::DialoguePresentation(value) => {
                self.save(&AssetRef::from_path(path.clone()), value.as_ref())
            }
            Asset::Dialogue(value) => self.save(&AssetRef::from_path(path.clone()), value.as_ref()),
            Asset::AiModel(value) => self.save(&AssetRef::from_path(path.clone()), value.as_ref()),
            Asset::Texture(value) => self.save(&AssetRef::from_path(path.clone()), value.as_ref()),
            Asset::TextureSet(value) => {
                self.save(&AssetRef::from_path(path.clone()), value.as_ref())
            }
            Asset::Sound(value) => self.save(&AssetRef::from_path(path.clone()), value.as_ref()),
            Asset::SoundBank(value) => {
                self.save(&AssetRef::from_path(path.clone()), value.as_ref())
            }
            Asset::SoundDefinitions(value) => {
                self.save(&AssetRef::from_path(path.clone()), value.as_ref())
            }
            Asset::Environment(value) => {
                self.save(&AssetRef::from_path(path.clone()), value.as_ref())
            }
            Asset::PhysicalEnvironment(value) => {
                self.save(&AssetRef::from_path(path.clone()), value.as_ref())
            }
            Asset::VisualEnvironment(value) => {
                self.save(&AssetRef::from_path(path.clone()), value.as_ref())
            }
            Asset::Fog(value) => self.save(&AssetRef::from_path(path.clone()), value.as_ref()),
            Asset::Light(value) => self.save(&AssetRef::from_path(path.clone()), value.as_ref()),
            Asset::CollisionProfile(value) => {
                self.save(&AssetRef::from_path(path.clone()), value.as_ref())
            }
            Asset::Mechanics(value) => {
                self.save(&AssetRef::from_path(path.clone()), value.as_ref())
            }
            Asset::Sprite(value) => self.save(&AssetRef::from_path(path.clone()), value.as_ref()),
            Asset::Model(value) => self.save(&AssetRef::from_path(path.clone()), value.as_ref()),
            Asset::ModelInstance(value) => {
                self.save(&AssetRef::from_path(path.clone()), value.as_ref())
            }
            Asset::AnimationAction(value) => {
                self.save(&AssetRef::from_path(path.clone()), value.as_ref())
            }
            Asset::Animation(value) => {
                self.save(&AssetRef::from_path(path.clone()), value.as_ref())
            }
            Asset::TransformPool(value) => {
                self.save(&AssetRef::from_path(path.clone()), value.as_ref())
            }
            Asset::Family(value) => self.save(&AssetRef::from_path(path.clone()), value.as_ref()),
            Asset::ObjectList(value) => {
                self.save(&AssetRef::from_path(path.clone()), value.as_ref())
            }
            Asset::Perso(value) => self.save(&AssetRef::from_path(path.clone()), value.as_ref()),
            Asset::Route(value) => self.save(&AssetRef::from_path(path.clone()), value.as_ref()),
            Asset::Waypoint(value) => self.save(&AssetRef::from_path(path.clone()), value.as_ref()),
            Asset::Collision(value) => {
                self.save(&AssetRef::from_path(path.clone()), value.as_ref())
            }
            Asset::Scene(value) => self.save(&AssetRef::from_path(path.clone()), value.as_ref()),
            Asset::Material(value) => self.save(&AssetRef::from_path(path.clone()), value.as_ref()),
        }
    }
    pub fn inspect(&mut self, entry: &Root) -> Result<String> {
        use crate::model::content::Asset;
        if entry.kind == crate::model::hud::HudPresentation::KIND {
            let value: crate::model::hud::HudPresentation = decode_unvalidated(
                &read_file(&self.path(&entry.reference))?,
                crate::model::hud::HudPresentation::KIND,
            )?;
            value.validate_reference(&entry.reference)?;
            return Ok(serde_json::to_string_pretty(&value)?);
        }
        let asset = Asset::load(self, entry)?;
        Ok(match asset {
            Asset::ArmorCatalog(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::InventoryPresentation(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::ParticleGenerator(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::MenuPresentation(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::HudPresentation(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::DialoguePresentation(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::Dialogue(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::AiModel(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::Texture(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::TextureSet(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::Sound(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::SoundBank(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::SoundDefinitions(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::Environment(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::PhysicalEnvironment(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::VisualEnvironment(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::Fog(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::Light(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::CollisionProfile(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::Mechanics(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::Sprite(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::Model(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::ModelInstance(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::AnimationAction(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::Animation(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::TransformPool(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::Family(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::ObjectList(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::Perso(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::Route(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::Waypoint(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::Collision(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::Scene(value) => serde_json::to_string_pretty(value.as_ref())?,
            Asset::Material(value) => serde_json::to_string_pretty(value.as_ref())?,
        })
    }
}
