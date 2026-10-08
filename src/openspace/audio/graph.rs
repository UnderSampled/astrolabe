//! Source-side dependency binding for useful sound programs. No Godot I/O here.
use super::{
    SoundScript,
    apm::read_apm,
    bnm::{read_bnm, resource_descriptor_at},
};
use crate::model::{AssetPath, AssetRef, Selection, audio::*};
use crate::openspace::codec::FormatCodec;
use anyhow::{Context, Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

pub(crate) struct SoundGraph {
    pub definitions: BTreeMap<AssetPath, SoundDefinitions>,
    pub banks: BTreeMap<AssetPath, SoundBank>,
    pub sounds: BTreeMap<AssetPath, Sound>,
    pub diagnostics: Vec<String>,
    files: SourceFiles,
    visited: BTreeSet<(AssetPath, u32)>,
    active: BTreeSet<(AssetPath, u32)>,
    bank_sources: BTreeMap<AssetPath, Vec<u8>>,
}
struct SourceFiles {
    game: PathBuf,
    files: Vec<PathBuf>,
}
impl SourceFiles {
    fn new(game: &Path) -> Result<Self> {
        fn collect(dir: &Path, root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    collect(&entry.path(), root, files)?;
                } else if entry.file_type()?.is_file() {
                    files.push(entry.path().strip_prefix(root)?.to_owned());
                }
            }
            Ok(())
        }
        let game = game.canonicalize()?;
        let mut files = Vec::new();
        for dir in ["Sound", "Gamedata/World/Sound"] {
            collect(&game.join(dir), &game, &mut files)?;
        }
        files.sort();
        Ok(Self { game, files })
    }
    fn canonical(&self, path: &AssetPath) -> Result<Option<AssetPath>> {
        let wanted = Path::new(path.file());
        let exact = self
            .files
            .iter()
            .find(|p| p.to_string_lossy().eq_ignore_ascii_case(path.file()));
        let found = if exact.is_some() {
            exact
        } else {
            let matches: Vec<_> = self
                .files
                .iter()
                .filter(|p| {
                    p.file_name().is_some_and(|s| {
                        s.to_string_lossy().eq_ignore_ascii_case(
                            &wanted.file_name().unwrap_or_default().to_string_lossy(),
                        )
                    })
                })
                .collect();
            // The current locale is English. Prefer its explicit directory when the
            // source reference supplies a filename shared by localized variants.
            let english: Vec<_> = matches
                .iter()
                .filter(|p| {
                    p.components().any(|c| {
                        c.as_os_str()
                            .to_string_lossy()
                            .eq_ignore_ascii_case("English")
                    })
                })
                .copied()
                .collect();
            let candidates = if english.is_empty() { matches } else { english };
            ensure!(candidates.len() <= 1, "ambiguous source sound file {path}");
            candidates.first().copied()
        };
        found
            .map(|p| AssetPath::new(format!("sound:/{}", p.to_string_lossy())))
            .transpose()
    }
    fn read(&self, path: &AssetPath) -> Result<Vec<u8>> {
        let file = self
            .game
            .join(path.file())
            .canonicalize()
            .with_context(|| format!("source sound {path}"))?;
        ensure!(file.starts_with(&self.game), "sound file leaves game root");
        ensure!(
            file.metadata()?.len() <= 512 * 1024 * 1024,
            "source sound file exceeds size limit"
        );
        Ok(std::fs::read(file)?)
    }
    fn normalize(&self, definitions: &mut SoundDefinitions) -> Result<()> {
        let canonical = |path: &mut AssetPath| -> Result<()> {
            if let Some(found) = self.canonical(path)? {
                *path = found;
            }
            Ok(())
        };
        for binding in &mut definitions.bindings {
            canonical(&mut binding.target.member_mut().definitions.path)?;
        }
        for event in &mut definitions.events {
            if let SoundOperation::Play {
                resource: Some(r), ..
            }
            | SoundOperation::Stop { event: Some(r) } = &mut event.operation
            {
                canonical(&mut r.definitions.path)?;
            }
        }
        for resource in &mut definitions.resources {
            for member in resource.data.members_mut() {
                canonical(&mut member.definitions.path)?;
            }
            if let SoundResourceData::Switch { selector, .. } = &mut resource.data {
                canonical(&mut selector.definitions.path)?;
            }
        }
        Ok(())
    }
}
impl SoundGraph {
    pub(crate) fn import<'a>(
        game: &Path,
        selection: &Selection,
        libraries: &[(AssetPath, SoundDefinitions)],
        events: impl IntoIterator<Item = &'a SoundEventLink>,
    ) -> Result<Self> {
        ensure!(
            selection.locale == "en",
            "sound program binding currently requires English"
        );
        let mut graph = Self {
            definitions: BTreeMap::new(),
            banks: BTreeMap::new(),
            sounds: BTreeMap::new(),
            diagnostics: Vec::new(),
            files: SourceFiles::new(game)?,
            visited: BTreeSet::new(),
            active: BTreeSet::new(),
            bank_sources: BTreeMap::new(),
        };
        for (path, library) in libraries {
            let mut library = library.clone();
            graph.files.normalize(&mut library)?;
            graph.definitions.insert(path.clone(), library);
        }
        for event in events {
            graph.event(event)?;
        }
        graph.bind_complete_shared_resources()?;
        graph.bank_sources.clear();
        for value in graph.definitions.values() {
            value.validate()?;
        }
        Ok(graph)
    }
    /// A generated sound-program script represents the whole definitions asset,
    /// not just the events a selected map happens to exercise. Resolve every
    /// resource in each touched shared CSB through its canonical library so
    /// global sound definitions do not acquire level-dependent partial sample
    /// bindings.
    fn bind_complete_shared_resources(&mut self) -> Result<()> {
        let touched: BTreeSet<_> = self
            .definitions
            .iter()
            .filter(|(_, definitions)| matches!(&definitions.kind, SoundDefinitionKind::Events))
            .map(|(path, _)| path.clone())
            .collect();
        if touched.is_empty() {
            return Ok(());
        }
        let mut tasks = BTreeSet::<(AssetPath, AssetPath, u32)>::new();
        for (library_path, library) in &self.definitions {
            if !matches!(&library.kind, SoundDefinitionKind::Library) {
                continue;
            }
            for binding in &library.bindings {
                let SoundDefinitionTarget::Resource(reference) = &binding.target else {
                    continue;
                };
                if touched.contains(&reference.definitions.path) {
                    tasks.insert((
                        library_path.clone(),
                        reference.definitions.path.clone(),
                        reference.id,
                    ));
                }
            }
        }
        for (library_path, definitions_path, id) in tasks {
            let reference = SoundMemberRef {
                definitions: AssetRef::from_path(definitions_path),
                id,
            };
            self.resource(&reference, &library_path, false)?;
        }
        Ok(())
    }
    fn definitions(&mut self, path: &AssetPath) -> Result<()> {
        if self.definitions.contains_key(path) {
            return Ok(());
        }
        let mut value = SoundScript {
            name: path.file().into(),
        }
        .read(&self.files.read(path)?)?;
        self.files.normalize(&mut value)?;
        self.definitions.insert(path.clone(), value);
        Ok(())
    }
    fn event(&mut self, event: &SoundEventLink) -> Result<()> {
        self.definitions(&event.library.path)?;
        let SoundDefinitionTarget::Event(reference) = &event
            .binding(&self.definitions[&event.library.path])?
            .target
        else {
            unreachable!()
        };
        let reference = reference.clone();
        self.definitions(&reference.definitions.path)?;
        let operation = self.definitions[&reference.definitions.path]
            .event(reference.id)?
            .operation
            .clone();
        match operation {
            SoundOperation::Play {
                resource: Some(r), ..
            } => self.resource(&r, &event.library.path, true)?,
            SoundOperation::Stop { event: Some(r) } => {
                self.definitions(&r.definitions.path)?;
                self.definitions[&r.definitions.path].event(r.id)?;
            }
            SoundOperation::Other { name } => self.diagnostics.push(format!(
                "{} event {}: unsupported sound operation {name}",
                reference.definitions.path, reference.id
            )),
            _ => {}
        }
        Ok(())
    }
    fn resource(
        &mut self,
        reference: &SoundMemberRef,
        library: &AssetPath,
        materialize_samples: bool,
    ) -> Result<()> {
        let key = (reference.definitions.path.clone(), reference.id);
        if self.visited.contains(&key) {
            return Ok(());
        }
        ensure!(
            self.active.insert(key.clone()),
            "cyclic sound resource program at {}#{}",
            key.0,
            key.1
        );
        ensure!(self.active.len() <= 64, "sound program nesting limit");
        self.definitions(&reference.definitions.path)?;
        let data = self.definitions[&reference.definitions.path]
            .resource(reference.id)?
            .data
            .clone();
        match &data {
            SoundResourceData::Sample { .. } => {
                self.sample(reference, library, materialize_samples)?
            }
            SoundResourceData::Switch {
                selector,
                default,
                choices,
                ..
            } => {
                self.definitions(&selector.definitions.path)?;
                let values = &self.definitions[&selector.definitions.path]
                    .selectors
                    .iter()
                    .find(|s| s.name == selector.name)
                    .context("missing sound selector")?
                    .values;
                ensure!(
                    values.iter().any(|(n, _)| n == default)
                        && choices
                            .iter()
                            .all(|c| values.iter().any(|(n, _)| n == &c.value)),
                    "sound switch value missing from selector"
                );
                for child in data.members() {
                    self.resource(child, library, materialize_samples)?;
                }
            }
            SoundResourceData::Random { .. } => {
                for child in data.members() {
                    self.resource(child, library, materialize_samples)?;
                }
            }
            SoundResourceData::Other { kind, .. } => self.diagnostics.push(format!(
                "{} resource {}: unsupported sound resource {kind}",
                reference.definitions.path, reference.id
            )),
        }
        self.active.remove(&key);
        self.visited.insert(key);
        Ok(())
    }
    fn sample(
        &mut self,
        reference: &SoundMemberRef,
        library: &AssetPath,
        materialize: bool,
    ) -> Result<()> {
        let mut bindings = self.definitions[library]
            .bindings
            .iter()
            .filter(|b| matches!(&b.target,SoundDefinitionTarget::Resource(r) if r == reference));
        let binding = bindings
            .next()
            .with_context(|| {
                format!(
                    "missing library sample binding {}#{}",
                    reference.definitions.path, reference.id
                )
            })?
            .clone();
        ensure!(
            bindings.next().is_none(),
            "ambiguous library sample binding"
        );
        let proposed = AssetPath::new(format!(
            "sound:/Gamedata/World/Sound/Bnk_{}.bnm",
            binding.bank
        ))?;
        let bank_path = self
            .files
            .canonical(&proposed)?
            .with_context(|| format!("missing sound bank {}", binding.bank))?;
        if !self.bank_sources.contains_key(&bank_path) {
            let bytes = self.files.read(&bank_path)?;
            self.bank_sources.insert(bank_path.clone(), bytes);
        }
        let descriptor = resource_descriptor_at(&self.bank_sources[&bank_path], binding.member)?;
        ensure!(
            descriptor.resource_type == 1,
            "selected sample uses unsupported bank resource type {}",
            descriptor.resource_type
        );
        ensure!(
            descriptor.embedded <= 1,
            "unsupported BNM storage flag for {}",
            descriptor.name
        );
        let resource_id = descriptor.resource_id;
        let name = Path::new(reference.definitions.path.file())
            .file_stem()
            .context("sound definitions lack filename")?
            .to_string_lossy();
        let group = self.definitions[library]
            .groups
            .iter()
            .find(|g| g.name.eq_ignore_ascii_case(&name))
            .context("sound resource group absent from library")?;
        ensure!(
            group.id <= u16::MAX.into()
                && reference.id <= u16::MAX.into()
                && resource_id == (group.id << 16) | reference.id,
            "library sound binding disagrees with bank resource identity"
        );
        let sample = if descriptor.embedded == 1 {
            if materialize {
                self.load_bank(&bank_path)?;
                ensure!(
                    self.banks[&bank_path]
                        .sounds
                        .iter()
                        .any(|sound| sound.resource_id == Some(resource_id)),
                    "embedded bank sample {resource_id} is absent from decoded bank"
                );
            }
            SoundSampleRef::Bank {
                bank: AssetRef::from_path(bank_path),
                resource_id,
            }
        } else {
            let proposed =
                AssetPath::new(format!("sound:/Gamedata/World/Sound/{}", descriptor.name))?;
            let path = self
                .files
                .canonical(&proposed)?
                .with_context(|| format!("external sample {} missing", descriptor.name))?;
            if materialize {
                self.load_bank(&bank_path)?;
                ensure!(
                    self.banks[&bank_path]
                        .external_sounds
                        .iter()
                        .any(|sound| sound.resource_id == resource_id),
                    "external bank sample {resource_id} is absent from decoded bank"
                );
                if !self.sounds.contains_key(&path) {
                    let sound = read_apm(&self.files.read(&path)?, path.file())?;
                    self.sounds.insert(path.clone(), sound);
                }
            }
            SoundSampleRef::Sound(AssetRef::from_path(path))
        };
        let SoundResourceData::Sample { sample: target, .. } = &mut self
            .definitions
            .get_mut(&reference.definitions.path)
            .unwrap()
            .resource_mut(reference.id)?
            .data
        else {
            unreachable!()
        };
        *target = Some(sample);
        Ok(())
    }
    fn load_bank(&mut self, path: &AssetPath) -> Result<()> {
        if !self.banks.contains_key(path) {
            let bank = read_bnm(&self.bank_sources[path], path.file())?;
            self.banks.insert(path.clone(), bank);
        }
        Ok(())
    }
}
