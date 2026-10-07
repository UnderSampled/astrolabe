//! Content selection from source packages. All address/encoding details stay in this adapter.
use super::{
    codec::{self, FormatCodec},
    level,
    texture::TextureArchive,
};
use crate::model::{
    AssetRef, Selection,
    audio::{Sound, SoundBank, SoundDefinitions},
    content::Collection,
    io::{Content, ContentRoot, Diagnostic, Load, Severity},
    texture::{Texture, TextureUsage},
};
use anyhow::{Context, Result, ensure};
use std::{
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

pub fn import_scene(game: &Path, selection: Selection) -> Result<Imported> {
    let mut imported = Imported::new(selection.clone());
    let mut scene = level::import_scene(game, &selection)?;
    let sound_graph = super::audio::graph::SoundGraph::import(
        game,
        &selection,
        &scene.sound_definitions,
        scene
            .actions
            .iter()
            .filter_map(|a| match &a.operation {
                crate::model::animation::AnimationOperation::Sound { event: Some(event) } => {
                    Some(event)
                }
                _ => None,
            })
            .chain(scene.ai_models.iter().flat_map(|m| m.sound_events()))
            .chain(scene.dialogues.iter().filter_map(|d| d.voice.as_ref())),
    )?;
    scene.sound_definitions = sound_graph.definitions.into_iter().collect();
    for (path, bank) in sound_graph.banks {
        imported.put(&AssetRef::from_path(path), bank, false)?;
    }
    for (path, sound) in sound_graph.sounds {
        imported.put(&AssetRef::from_path(path), sound, false)?;
    }
    scene.diagnostics.extend(
        sound_graph
            .diagnostics
            .into_iter()
            .map(|message| Diagnostic {
                code: "openspace.sound.program_coverage".into(),
                asset: Some(scene.scene.path.clone()),
                severity: Severity::Warning,
                message,
            }),
    );
    for action in std::mem::take(&mut scene.actions) {
        imported.put(&AssetRef::from_path(action.path.clone()), action, false)?;
    }
    for (path, definitions) in std::mem::take(&mut scene.sound_definitions) {
        imported.put(&AssetRef::from_path(path), definitions, false)?;
    }
    for value in std::mem::take(&mut scene.environments) {
        imported.put(&AssetRef::from_path(value.path.clone()), value, false)?;
    }
    for value in std::mem::take(&mut scene.physical_environments) {
        imported.put(&AssetRef::from_path(value.path.clone()), value, false)?;
    }
    for value in std::mem::take(&mut scene.visual_environments) {
        imported.put(&AssetRef::from_path(value.path.clone()), value, false)?;
    }
    for value in std::mem::take(&mut scene.fogs) {
        imported.put(&AssetRef::from_path(value.path.clone()), value, false)?;
    }
    for light in std::mem::take(&mut scene.lights) {
        imported.put(&AssetRef::from_path(light.path.clone()), light, false)?;
    }
    for sprite in std::mem::take(&mut scene.sprites) {
        imported.put(&AssetRef::from_path(sprite.path.clone()), sprite, false)?;
    }
    for generator in std::mem::take(&mut scene.particle_generators) {
        imported.put(
            &AssetRef::from_path(generator.path.clone()),
            generator,
            false,
        )?;
    }
    let archive = codec::Cnt {
        usage: TextureUsage::Material,
    }
    .read(&read(&game.join("Gamedata/Textures.cnt"))?)?;
    if let Some(mut presentation) = scene.dialogue_presentation.take() {
        for texture in std::iter::once(&mut presentation.atlas).chain(&mut presentation.frame) {
            let found = archive
                .find_entry(texture.texture.file())
                .with_context(|| format!("dialogue texture {} absent from CNT", texture.texture))?;
            ensure!(
                (
                    archive.image(found)?.width(),
                    archive.image(found)?.height()
                ) == (texture.size[0], texture.size[1]),
                "dialogue texture/CNT dimensions differ"
            );
            texture.texture = found.texture.path.clone();
            if !imported.assets.contains(&texture.texture) {
                imported.put(&found.texture, archive.image(found)?.clone(), false)?;
            }
        }
        imported.put(
            &AssetRef::from_path(presentation.path.clone()),
            presentation,
            false,
        )?;
    }
    for material in &mut scene.materials {
        if let Some(original) = material.texture() {
            let found = archive
                .find_entry(original.file())
                .with_context(|| format!("texture {original} absent from CNT"))?;
            let reference = found.texture.clone();
            if !imported.assets.contains(&reference.path) {
                imported.put(&reference, archive.image(found)?.clone(), false)?;
            }
            material.visual.as_mut().unwrap().texture = Some(reference.path);
        }
        imported.put(
            &AssetRef::from_path(material.path.clone()),
            material.clone(),
            false,
        )?;
    }
    for collision in scene.collisions {
        imported.put(
            &AssetRef::from_path(collision.path.clone()),
            collision,
            false,
        )?;
    }
    for asset in scene.collision_profiles {
        imported.put(&AssetRef::from_path(asset.path.clone()), asset, false)?;
    }
    for asset in scene.mechanics {
        imported.put(&AssetRef::from_path(asset.path.clone()), asset, false)?;
    }
    for asset in scene.families {
        imported.put(&AssetRef::from_path(asset.path.clone()), asset, false)?;
    }
    for asset in scene.persos {
        imported.put(&AssetRef::from_path(asset.path.clone()), asset, false)?;
    }
    for asset in scene.routes {
        imported.put(&AssetRef::from_path(asset.path.clone()), asset, false)?;
    }
    for asset in scene.waypoints {
        imported.put(&AssetRef::from_path(asset.path.clone()), asset, false)?;
    }
    for asset in scene.dialogues {
        imported.put(&AssetRef::from_path(asset.path.clone()), asset, true)?;
    }
    for asset in scene.ai_models {
        imported.put(&AssetRef::from_path(asset.path.clone()), asset, false)?;
    }
    for asset in scene.configurations {
        imported.put(&AssetRef::from_path(asset.path.clone()), asset, false)?;
    }
    for asset in scene.animations {
        imported.put(&AssetRef::from_path(asset.path.clone()), asset, false)?;
    }
    for asset in scene.transform_pools {
        imported.put(&AssetRef::from_path(asset.path.clone()), asset, false)?;
    }
    for model in scene.models {
        imported.put(&AssetRef::from_path(model.path.clone()), model, false)?;
    }
    for instance in scene.instances {
        imported.put(&AssetRef::from_path(instance.path.clone()), instance, false)?;
    }
    imported.put(
        &AssetRef::from_path(scene.scene.path.clone()),
        scene.scene,
        true,
    )?;
    imported.source_summary = Some(scene.source);
    imported.diagnostics = scene.diagnostics;
    Ok(imported)
}

pub fn import_model(game: &Path, selection: Selection) -> Result<Imported> {
    let mut imported = Imported::new(selection.clone());
    let mut model = level::import_first_model(game, &selection)?;
    let archive = codec::Cnt {
        usage: TextureUsage::Material,
    }
    .read(&read(&game.join("Gamedata/Textures.cnt"))?)?;
    for material in &mut model.materials {
        if let Some(original) = material.texture() {
            let found = archive
                .find_entry(original.file())
                .with_context(|| format!("texture {} absent from CNT", original))?;
            let reference = found.texture.clone();
            if !imported.assets.contains(&reference.path) {
                imported.put(&reference, archive.image(found)?.clone(), false)?;
            }
            material.visual.as_mut().unwrap().texture = Some(reference.path);
        }
        let reference = AssetRef::new(material.path.to_string())?;
        imported.put(&reference, material.clone(), false)?;
    }
    for sprite in model.sprites {
        imported.put(&AssetRef::from_path(sprite.path.clone()), sprite, false)?;
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
    imported.put(&reference, model.model, false)?;
    let reference = AssetRef::from_path(model.instance.path.clone());
    imported.put(&reference, model.instance, true)?;
    Ok(imported)
}

fn add_archive(imported: &mut Imported, name: &str, archive: TextureArchive) -> Result<()> {
    for (path, image) in archive.images {
        let reference = AssetRef::from_path(path);
        if imported.assets.contains(&reference.path) {
            let old: Arc<Texture> = imported.load(&reference)?;
            ensure!(
                *old == image,
                "distinct textures collide at {}",
                reference.path
            );
        } else {
            imported.put(&reference, image, false)?;
        }
    }
    imported.put(
        &AssetRef::new(format!("texture:/archives/{name}"))?,
        archive.set,
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
pub use super::texture::write_cnt as rebuild_texture_set;

/// Main menu artwork/audio is a separate small closure from gameplay scene import.
pub fn import_menu(game: &Path, selection: Selection) -> Result<Imported> {
    import_menu_variant(game, selection, false)
}
pub fn import_pause_menu(game: &Path, selection: Selection) -> Result<Imported> {
    import_menu_variant(game, selection, true)
}
fn import_menu_variant(game: &Path, selection: Selection, pause: bool) -> Result<Imported> {
    let mut imported = Imported::new(Selection::hype(
        if pause { "pause-menu" } else { "menu" },
        selection.locale.clone(),
    )?);
    let (mut menu, sounds) = if pause {
        level::import_pause_menu(game, &selection)?
    } else {
        level::import_main_menu(game, &selection)?
    };
    let archive = codec::Cnt {
        usage: TextureUsage::Material,
    }
    .read(&read(&game.join("Gamedata/Textures.cnt"))?)?;
    for sprite in menu
        .items
        .iter_mut()
        .flat_map(|i| i.normal.iter_mut().chain(&mut i.selected))
    {
        let entry = archive
            .find_entry(sprite.texture.path.file())
            .context("menu atlas absent from CNT")?;
        sprite.texture = entry.texture.clone();
        if !imported.assets.contains(&entry.texture.path) {
            imported.put(&entry.texture, archive.image(entry)?.clone(), false)?;
        }
    }
    for (path, sound) in sounds {
        imported.put(&AssetRef::from_path(path), sound, false)?;
    }
    if let Some(background) = &menu.background {
        let vignette = codec::Cnt {
            usage: TextureUsage::Screen,
        }
        .read(&read(&game.join("Gamedata/Vignette.cnt"))?)?;
        let entry = vignette
            .find_entry(
                &Path::new(background.path.file())
                    .with_extension("gf")
                    .to_string_lossy(),
            )
            .context("required main menu backdrop absent")?;
        menu.background = Some(entry.texture.clone());
        imported.put(&entry.texture, vignette.image(entry)?.clone(), false)?;
    }
    imported.put(&AssetRef::from_path(menu.path.clone()), menu, true)?;
    Ok(imported)
}

pub fn import_inventory(game: &Path, selection: Selection) -> Result<Imported> {
    let mut imported = Imported::new(Selection::hype("inventory", selection.locale.clone())?);
    let mut value = level::import_inventory_presentation(game, &selection)?;
    let archive = codec::Cnt {
        usage: TextureUsage::Material,
    }
    .read(&read(&game.join("Gamedata/Textures.cnt"))?)?;
    for image in value
        .categories
        .iter_mut()
        .flat_map(|c| &mut c.images)
        .chain(value.items.iter_mut().flat_map(|i| &mut i.images))
    {
        let entry = archive
            .find_entry(image.texture.path.file())
            .context("inventory image absent from CNT")?;
        image.texture = entry.texture.clone();
        if !imported.assets.contains(&entry.texture.path) {
            imported.put(&entry.texture, archive.image(entry)?.clone(), false)?;
        }
    }
    let entry = archive
        .find_entry(value.font.atlas.texture.file())
        .context("InventoryFont atlas absent from CNT")?;
    ensure!(
        [
            archive.image(entry)?.width(),
            archive.image(entry)?.height()
        ] == value.font.atlas.size,
        "inventory atlas dimensions differ"
    );
    value.font.atlas.texture = entry.texture.path.clone();
    if !imported.assets.contains(&entry.texture.path) {
        imported.put(&entry.texture, archive.image(entry)?.clone(), false)?;
    }
    let vignette = codec::Cnt {
        usage: TextureUsage::Screen,
    }
    .read(&read(&game.join("Gamedata/Vignette.cnt"))?)?;
    let entry = vignette
        .find_entry(
            &Path::new(value.background.path.file())
                .with_extension("gf")
                .to_string_lossy(),
        )
        .context("required inventory backdrop absent")?;
    value.background = entry.texture.clone();
    imported.put(&entry.texture, vignette.image(entry)?.clone(), false)?;
    imported.put(&AssetRef::from_path(value.path.clone()), value, true)?;
    Ok(imported)
}

/// Import the shared, source-authored manor HUD and retain unresolved GLI texture handles.
pub fn import_hud(game: &Path, selection: Selection) -> Result<Imported> {
    let mut imported = Imported::new(selection.clone());
    let presentation = level::import_hud_presentation(game, &selection)?;
    imported.put(
        &AssetRef::from_path(presentation.path.clone()),
        presentation,
        true,
    )?;
    Ok(imported)
}
