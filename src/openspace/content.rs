//! Content selection from source packages. All address/encoding details stay in this adapter.
use super::{
    codec::{self, FormatCodec},
    level,
};
use crate::model::{
    AssetPath, AssetRef, Selection,
    audio::{Sound, SoundBank, SoundDefinitions},
    content::{Collection, TextureEntry, TextureSet},
    io::{Content, ContentRoot, Diagnostic, Load, Severity},
    texture::{Texture, TextureArchive, TextureUsage},
};
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct Imported {
    pub assets: Collection,
    pub selection: Selection,
    pub roots: Vec<ContentRoot>,
    /// Transient save list of actual content files, not a persistent source-address index.
    pub inventory: Vec<ContentRoot>,
    pub diagnostics: Vec<Diagnostic>,
    /// Debug observations may include source addresses; never persisted with Rete.
    pub source_summary: Option<level::LevelSummary>,
}
impl Imported {
    pub fn new(selection: Selection) -> Self {
        Self {
            assets: Collection::default(),
            selection,
            roots: vec![],
            inventory: vec![],
            diagnostics: vec![],
            source_summary: None,
        }
    }
    fn put<T: Content>(&mut self, id: &AssetRef<T>, value: T, root: bool) -> Result<()> {
        let entry = ContentRoot {
            kind: T::KIND.into(),
            reference: id.path.clone(),
        };
        if !self.assets.contains(&id.path) {
            self.inventory.push(entry.clone());
        }
        self.assets.insert(id, value)?;
        if root {
            self.roots.push(entry);
        }
        Ok(())
    }
}
impl<T: Content> Load<T> for Imported {
    fn load(&mut self, id: &AssetRef<T>) -> Result<Arc<T>> {
        self.assets.load(id)
    }
}

fn read(path: &Path) -> Result<Vec<u8>> {
    ensure!(
        path.metadata()?.len() <= 512 * 1024 * 1024,
        "source file exceeds 512MiB limit: {}",
        path.display()
    );
    fs::read(path).with_context(|| format!("read {}", path.display()))
}
fn texture_id(path: &str) -> Result<AssetRef<Texture>> {
    AssetRef::new(format!("texture:/{}", path.replace('\\', "/")))
}

pub fn import_model(game: &Path, selection: Selection) -> Result<Imported> {
    let mut imported = Imported::new(selection.clone());
    let mut model = level::import_first_model(game, &selection)?;
    let archive = codec::Cnt {
        usage: TextureUsage::Material,
    }
    .read(&read(&game.join("Gamedata/Textures.cnt"))?)?;
    for material in &mut model.materials {
        if let Some(original) = &material.texture {
            let found = archive
                .find_by_source_name(original.file())
                .with_context(|| format!("texture {} absent from CNT", original))?;
            let canonical = found.shared_with.as_deref().unwrap_or(&found.path);
            let reference = texture_id(canonical)?;
            if !imported.assets.contains(&reference.path) {
                imported.put(&reference, found.texture.clone(), false)?;
            }
            material.texture = Some(reference.path);
        }
        let reference = AssetRef::new(material.path.to_string())?;
        imported.put(&reference, material.clone(), false)?;
    }
    imported.source_summary = Some(model.source);
    imported.diagnostics.push(Diagnostic {
        code: "openspace.model.partial_import".into(),
        asset: Some(model.model.path.clone()),
        severity: Severity::Warning,
        message:
            "Imported the first LOD of one rooted model; this content is not a complete level."
                .into(),
    });
    imported
        .diagnostics
        .extend(model.diagnostics.into_iter().map(|message| Diagnostic {
            code: "openspace.model.observation".into(),
            asset: Some(model.model.path.clone()),
            severity: Severity::Info,
            message,
        }));
    let reference = AssetRef::new(model.model.path.to_string())?;
    imported.put(&reference, model.model, true)?;
    Ok(imported)
}

fn add_archive(imported: &mut Imported, name: &str, archive: TextureArchive) -> Result<()> {
    let mut entries = Vec::new();
    for member in &archive.textures {
        let canonical = member.shared_with.as_deref().unwrap_or(&member.path);
        let reference = texture_id(canonical)?;
        if imported.assets.contains(&reference.path) {
            let old: Arc<Texture> = imported.load(&reference)?;
            ensure!(
                *old == member.texture,
                "distinct textures collide at {}",
                reference.path
            );
        } else {
            imported.put(&reference, member.texture.clone(), false)?;
        }
        entries.push(TextureEntry {
            name: member.path.clone(),
            texture: reference,
        });
    }
    let set = TextureSet {
        name: name.into(),
        directories: archive.directories,
        entries,
    };
    imported.put(
        &AssetRef::new(format!("texture:/archives/{name}"))?,
        set,
        true,
    )
}
pub fn import_textures(game: &Path, selection: Selection) -> Result<Imported> {
    let mut imported = Imported::new(selection);
    for (name, usage) in [
        ("Textures.cnt", TextureUsage::Material),
        ("Vignette.cnt", TextureUsage::Screen),
    ] {
        add_archive(
            &mut imported,
            name,
            codec::Cnt { usage }.read(&read(&game.join("Gamedata").join(name))?)?,
        )?;
    }
    Ok(imported)
}

/// Explicit files support focused conversion. Empty selection imports the supported sound corpus.
pub fn import_audio(game: &Path, selection: Selection, files: &[PathBuf]) -> Result<Imported> {
    ensure!(
        selection.locale.eq_ignore_ascii_case("en"),
        "standalone audio currently supports only the supplied English (en) selection"
    );
    let mut imported = Imported::new(selection);
    let selected = if files.is_empty() {
        let mut result = Vec::new();
        for dir in [
            game.join("Gamedata/World/Sound"),
            game.join("LangData/English"),
            game.join("Sound"),
        ] {
            if dir.is_dir() {
                collect_audio(&dir, game, &mut result)?;
            }
        }
        result.sort();
        result
    } else {
        files.to_vec()
    };
    let game = game.canonicalize()?;
    for relative in selected {
        ensure!(
            !relative.is_absolute()
                && relative
                    .components()
                    .all(|c| matches!(c, std::path::Component::Normal(_))),
            "sound path must be relative to game root"
        );
        let path = game.join(&relative).canonicalize()?;
        ensure!(path.starts_with(&game), "sound path leaves game root");
        let name = relative
            .to_str()
            .context("source filename is not UTF-8")?
            .replace('\\', "/");
        let bytes = read(&path)?;
        let extension = path
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_lowercase();
        match extension.as_str() {
            "apm" => imported.put(
                &AssetRef::<Sound>::new(format!("sound:/{name}"))?,
                codec::Apm { name }.read(&bytes)?,
                true,
            )?,
            "bnm" => imported.put(
                &AssetRef::<SoundBank>::new(format!("sound:/{name}"))?,
                codec::Bnm { name }.read(&bytes)?,
                true,
            )?,
            "csb" | "lcb" | "sif" => imported.put(
                &AssetRef::<SoundDefinitions>::new(format!("sound:/{name}"))?,
                codec::SoundScript { name }.read(&bytes)?,
                true,
            )?,
            _ => anyhow::bail!("unsupported audio file {}", path.display()),
        }
    }
    ensure!(
        !imported.roots.is_empty(),
        "no supported sound files selected"
    );
    Ok(imported)
}
fn collect_audio(dir: &Path, game: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let path = entry.path();
        if ty.is_dir() {
            collect_audio(&path, game, out)?;
        } else if ty.is_file()
            && path.extension().and_then(|x| x.to_str()).is_some_and(|e| {
                matches!(
                    e.to_ascii_lowercase().as_str(),
                    "apm" | "bnm" | "csb" | "lcb" | "sif"
                )
            })
        {
            out.push(path.strip_prefix(game)?.to_owned());
        }
    }
    Ok(())
}
pub fn rebuild_texture_set<R: Load<Texture>>(
    set: &TextureSet,
    resolver: &mut R,
) -> Result<Vec<u8>> {
    set.validate()?;
    let mut first = BTreeMap::<AssetPath, String>::new();
    let mut textures = Vec::new();
    for entry in &set.entries {
        let shared_with = first.get(&entry.texture.path).cloned();
        first
            .entry(entry.texture.path.clone())
            .or_insert_with(|| entry.name.clone());
        textures.push(crate::model::texture::NamedTexture {
            path: entry.name.clone(),
            texture: (*resolver.load(&entry.texture)?).clone(),
            shared_with,
        });
    }
    let usage = match set.name.as_str() {
        "Textures.cnt" => TextureUsage::Material,
        "Vignette.cnt" => TextureUsage::Screen,
        _ => anyhow::bail!("unsupported texture archive {}", set.name),
    };
    codec::Cnt { usage }.write(&TextureArchive {
        directories: set.directories.clone(),
        textures,
    })
}
