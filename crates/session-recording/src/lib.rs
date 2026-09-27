//! Engine-neutral, appendable evidence for an observed or replayed game session.
//! Controllers own delivery and capture; this crate owns the immutable contract.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File, FileTimes, OpenOptions},
    io::{BufRead, BufReader, Write},
    os::unix::fs::PermissionsExt,
    path::{Component, Path, PathBuf},
    time::{Duration, UNIX_EPOCH},
};

pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SaveRootKind {
    Absent,
    Empty,
    Populated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    /// Original metadata is evidence. A backend must report whether it restored it.
    pub modified_unix_nanos: Option<u128>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveFixture {
    pub kind: SaveRootKind,
    pub directories: Vec<String>,
    pub files: Vec<SaveFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coverage {
    pub input: String,
    pub video: String,
    pub state: String,
    pub ai_api: String,
    pub audio: String,
    pub gaps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub content_tree_sha256: String,
    pub content_file_count: u64,
    pub runtime_name: String,
    pub runtime_executable_sha256: String,
    pub locale: String,
    pub display_profile: String,
    pub control_profile: String,
    pub randomness_inputs: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub id: String,
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub driver: String,
    pub executable_sha256: String,
    pub environment: Environment,
    pub initial_saves: SaveFixture,
    pub started_unix_micros: u128,
    pub elapsed_micros: u128,
    pub ending: String,
    pub replay_of_sha256: Option<String>,
    pub coverage: Coverage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub sequence: u64,
    pub elapsed_micros: u128,
    pub kind: String,
    pub data: Value,
}

pub fn digest(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    std::io::copy(&mut file, &mut hash)?;
    Ok(format!("{:x}", hash.finalize()))
}

fn relative_path(path: &Path) -> Result<String> {
    ensure!(
        path.components()
            .all(|component| matches!(component, Component::Normal(_))),
        "save path is not a plain relative path"
    );
    let value = path.to_str().context("save path is not UTF-8")?;
    ensure!(!value.is_empty(), "empty save path");
    ensure!(!value.contains('\\'), "save path contains a backslash");
    Ok(value.replace('\\', "/"))
}

fn restore_modified(path: &Path, modified_unix_nanos: Option<u128>) -> Result<()> {
    if let Some(nanos) = modified_unix_nanos {
        let seconds: u64 = (nanos / 1_000_000_000).try_into()?;
        let subnanos: u32 = (nanos % 1_000_000_000).try_into()?;
        let modified = UNIX_EPOCH + Duration::new(seconds, subnanos);
        File::open(path)?.set_times(FileTimes::new().set_modified(modified))?;
    }
    Ok(())
}

/// Seal captured evidence files against ordinary writes. Replay still verifies
/// the content hash because directory entry replacement remains possible.
pub fn freeze_files(root: &Path) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        ensure!(
            !metadata.file_type().is_symlink(),
            "evidence contains a symlink"
        );
        if metadata.is_dir() {
            freeze_files(&path)?;
        } else {
            ensure!(metadata.is_file(), "evidence contains a non-file entry");
            let mut permissions = metadata.permissions();
            permissions.set_mode(permissions.mode() & !0o222);
            fs::set_permissions(path, permissions)?;
        }
    }
    Ok(())
}

fn collect(
    root: &Path,
    dir: &Path,
    directories: &mut Vec<String>,
    files: &mut Vec<SaveFile>,
    copy_to: Option<&Path>,
) -> Result<()> {
    let mut entries = fs::read_dir(dir)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        ensure!(
            !metadata.file_type().is_symlink(),
            "save fixture contains a symlink"
        );
        if metadata.is_dir() {
            let relative = path.strip_prefix(root)?;
            directories.push(relative_path(relative)?);
            if let Some(destination) = copy_to {
                fs::create_dir_all(destination.join(relative))?;
            }
            collect(root, &path, directories, files, copy_to)?;
        } else {
            ensure!(metadata.is_file(), "save fixture contains a non-file entry");
            let relative = path.strip_prefix(root)?;
            let name = relative_path(relative)?;
            let copied = if let Some(destination) = copy_to {
                let target = destination.join(relative);
                fs::create_dir_all(target.parent().context("save parent")?)?;
                fs::copy(&path, &target)?;
                ensure!(
                    digest(&path)? == digest(&target)?,
                    "save changed while copying"
                );
                restore_modified(
                    &target,
                    metadata
                        .modified()
                        .ok()
                        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                        .map(|time| time.as_nanos()),
                )?;
                target
            } else {
                path.clone()
            };
            files.push(SaveFile {
                path: name,
                size: metadata.len(),
                sha256: digest(&copied)?,
                modified_unix_nanos: metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|time| time.as_nanos()),
            });
        }
    }
    Ok(())
}

/// Copy a quiescent complete save root into an immutable fixture directory.
/// An unreadable root fails rather than being interpreted as an empty fixture.
pub fn capture_saves(root: &Path, destination: &Path) -> Result<SaveFixture> {
    ensure!(
        !destination.exists(),
        "save fixture destination already exists"
    );
    if let Some(parent) = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let kind = match fs::symlink_metadata(root) {
        Ok(metadata) => {
            ensure!(
                metadata.is_dir() && !metadata.file_type().is_symlink(),
                "invalid save root"
            );
            SaveRootKind::Empty
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => SaveRootKind::Absent,
        Err(error) => return Err(error.into()),
    };
    fs::create_dir(destination)?;
    let mut directories = Vec::new();
    let mut files = Vec::new();
    if kind != SaveRootKind::Absent {
        collect(root, root, &mut directories, &mut files, Some(destination))?;
    }
    let kind = if files.is_empty() && directories.is_empty() {
        kind
    } else {
        SaveRootKind::Populated
    };
    let fixture = SaveFixture {
        kind,
        directories,
        files,
    };
    verify_saves(
        destination,
        &SaveFixture {
            kind: if fixture.kind == SaveRootKind::Absent {
                SaveRootKind::Empty
            } else {
                fixture.kind
            },
            directories: fixture.directories.clone(),
            files: fixture.files.clone(),
        },
    )?;
    freeze_files(destination)?;
    Ok(fixture)
}

/// Inspect a live save root without mutating it. A caller must compare repeated
/// observations before treating a change as stable; this alone cannot identify
/// an engine save-operation boundary.
pub fn inspect_saves(root: &Path) -> Result<SaveFixture> {
    let kind = match fs::symlink_metadata(root) {
        Ok(metadata) => {
            ensure!(
                metadata.is_dir() && !metadata.file_type().is_symlink(),
                "invalid save root"
            );
            SaveRootKind::Empty
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => SaveRootKind::Absent,
        Err(error) => return Err(error.into()),
    };
    let mut directories = Vec::new();
    let mut files = Vec::new();
    if kind != SaveRootKind::Absent {
        collect(root, root, &mut directories, &mut files, None)?;
    }
    Ok(SaveFixture {
        kind: if files.is_empty() && directories.is_empty() {
            kind
        } else {
            SaveRootKind::Populated
        },
        directories,
        files,
    })
}

pub fn verify_saves(root: &Path, expected: &SaveFixture) -> Result<()> {
    let actual = match fs::symlink_metadata(root) {
        Ok(metadata) => {
            ensure!(
                metadata.is_dir() && !metadata.file_type().is_symlink(),
                "invalid save root"
            );
            let mut directories = Vec::new();
            let mut files = Vec::new();
            collect(root, root, &mut directories, &mut files, None)?;
            SaveFixture {
                kind: if files.is_empty() && directories.is_empty() {
                    SaveRootKind::Empty
                } else {
                    SaveRootKind::Populated
                },
                directories,
                files,
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => SaveFixture {
            kind: SaveRootKind::Absent,
            directories: vec![],
            files: vec![],
        },
        Err(error) => return Err(error.into()),
    };
    ensure!(actual.kind == expected.kind, "save root kind differs");
    ensure!(
        actual.directories == expected.directories,
        "save directories differ"
    );
    ensure!(
        actual.files.len() == expected.files.len(),
        "save file count differs"
    );
    for (actual, expected) in actual.files.iter().zip(&expected.files) {
        ensure!(
            actual.path == expected.path
                && actual.size == expected.size
                && actual.sha256 == expected.sha256
                && actual.modified_unix_nanos == expected.modified_unix_nanos,
            "save file differs: {}",
            expected.path
        );
    }
    Ok(())
}

/// Populate one fresh replay root from verified captured bytes. Never merge.
pub fn stage_saves(source: &Path, fixture: &SaveFixture, destination: &Path) -> Result<()> {
    verify_saves(
        source,
        &SaveFixture {
            kind: if fixture.kind == SaveRootKind::Absent {
                SaveRootKind::Empty
            } else {
                fixture.kind
            },
            directories: fixture.directories.clone(),
            files: fixture.files.clone(),
        },
    )?;
    ensure!(!destination.exists(), "replay save root already exists");
    if fixture.kind == SaveRootKind::Absent {
        return Ok(());
    }
    fs::create_dir(destination)?;
    for directory in &fixture.directories {
        let relative = PathBuf::from(directory);
        relative_path(&relative)?;
        fs::create_dir_all(destination.join(relative))?;
    }
    for file in &fixture.files {
        let relative = PathBuf::from(&file.path);
        relative_path(&relative)?;
        let target = destination.join(&relative);
        fs::create_dir_all(target.parent().context("save parent")?)?;
        fs::copy(source.join(&relative), &target)?;
        let mut permissions = fs::metadata(&target)?.permissions();
        permissions.set_mode(permissions.mode() | 0o200);
        fs::set_permissions(&target, permissions)?;
        restore_modified(&target, file.modified_unix_nanos)?;
    }
    verify_saves(destination, fixture)
}

pub struct EventWriter {
    file: File,
    next_sequence: u64,
    last_elapsed: u128,
}
impl EventWriter {
    pub fn create(path: &Path) -> Result<Self> {
        Ok(Self {
            file: OpenOptions::new().write(true).create_new(true).open(path)?,
            next_sequence: 0,
            last_elapsed: 0,
        })
    }
    pub fn append(&mut self, elapsed_micros: u128, kind: &str, data: Value) -> Result<Event> {
        ensure!(
            elapsed_micros >= self.last_elapsed,
            "event timeline went backwards"
        );
        ensure!(!kind.is_empty(), "event kind is empty");
        let event = Event {
            sequence: self.next_sequence,
            elapsed_micros,
            kind: kind.to_owned(),
            data,
        };
        serde_json::to_writer(&mut self.file, &event)?;
        self.file.write_all(b"\n")?;
        self.file.flush()?;
        self.next_sequence += 1;
        self.last_elapsed = elapsed_micros;
        Ok(event)
    }
}

/// A torn final line is reported as an incomplete prefix; earlier lines remain usable.
pub fn read_events(path: &Path) -> Result<(Vec<Event>, bool)> {
    let file = File::open(path)?;
    let mut events = Vec::new();
    let mut complete = true;
    for line in BufReader::new(file).split(b'\n') {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        match serde_json::from_slice::<Event>(&line) {
            Ok(event) => {
                ensure!(event.sequence == events.len() as u64, "event sequence gap");
                ensure!(
                    events
                        .last()
                        .is_none_or(|last: &Event| event.elapsed_micros >= last.elapsed_micros),
                    "event timeline went backwards"
                );
                events.push(event);
            }
            Err(_) => {
                complete = false;
                break;
            }
        }
    }
    Ok((events, complete))
}

pub fn recording_sha256(directory: &Path) -> Result<String> {
    let mut hash = Sha256::new();
    fn hash_tree(root: &Path, path: &Path, hash: &mut Sha256) -> Result<()> {
        let mut entries = fs::read_dir(path)?.collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)?;
            ensure!(
                !metadata.file_type().is_symlink(),
                "recording contains a symlink"
            );
            let relative = relative_path(path.strip_prefix(root)?)?;
            hash.update(relative.as_bytes());
            if metadata.is_dir() {
                hash_tree(root, &path, hash)?;
            } else {
                ensure!(metadata.is_file(), "recording contains a non-file entry");
                let mut file = File::open(path)?;
                let mut buffer = [0u8; 64 * 1024];
                loop {
                    let count = std::io::Read::read(&mut file, &mut buffer)?;
                    if count == 0 {
                        break;
                    }
                    hash.update(&buffer[..count]);
                }
            }
        }
        Ok(())
    }
    hash_tree(directory, directory, &mut hash)?;
    Ok(format!("{:x}", hash.finalize()))
}

pub fn validate_input_events(events: &[Event]) -> Result<()> {
    let mut held = BTreeSet::new();
    for event in events.iter().filter(|event| event.kind == "input") {
        let keyboard = event.data["keyboard"]
            .as_u64()
            .context("input keyboard missing")?;
        let code = event.data["code"]
            .as_u64()
            .context("input keycode missing")?;
        ensure!(code <= u8::MAX as u64, "input keycode outside byte range");
        let edge = event.data["edge"].as_str().context("input edge missing")?;
        ensure!(keyboard == 0, "unsupported keyboard");
        match edge {
            "press" if event.data["repeat"] == true => {
                ensure!(held.contains(&code), "repeat without press")
            }
            "press" => ensure!(held.insert(code), "duplicate input press"),
            "release" => ensure!(held.remove(&code), "input release without press"),
            _ => anyhow::bail!("unknown input edge"),
        }
    }
    ensure!(held.is_empty(), "recording ends with held inputs");
    Ok(())
}
