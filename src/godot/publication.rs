//! Publish generated content while preserving authored and edited files.
use super::{Manifest, digest};
use crate::files::write_atomic;
use anyhow::{Context, Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
pub(super) fn publish(
    stage: &Path,
    output: &Path,
    generated: BTreeSet<String>,
    diagnostics: Vec<String>,
    version: String,
) -> Result<Manifest> {
    let old_path = output.join("generated/astrolabe-manifest.json");
    let old: Manifest = if old_path.exists() {
        serde_json::from_slice(&fs::read(&old_path)?)?
    } else {
        Manifest::default()
    };
    for path in old.files.keys().chain(generated.iter()) {
        validate_output_path(output, path)?;
    }
    // All stale-file conflicts are checked before publishing anything.
    for (path, hash) in &old.files {
        let full = output.join(path);
        if !generated.contains(path) && full.exists() {
            ensure!(
                digest(&fs::read(&full)?) == *hash,
                "stale generated file was edited: {}",
                full.display()
            );
        }
    }
    let mut manifest = Manifest {
        schema: 1,
        godot_version: version,
        files: BTreeMap::new(),
        diagnostics,
    };
    // Check every collision before modifying the destination.
    for path in &generated {
        let destination = output.join(path);
        if destination.exists() {
            let expected = old.files.get(path).with_context(|| {
                format!("refuse to overwrite unowned file {}", destination.display())
            })?;
            ensure!(
                &digest(&fs::read(&destination)?) == expected,
                "generated file was edited; preserve it before regeneration: {}",
                destination.display()
            );
        }
        manifest
            .files
            .insert(path.clone(), digest(&fs::read(stage.join(path))?));
    }
    for (path, hash) in &manifest.files {
        if old.files.get(path) == Some(hash) && output.join(path).exists() {
            continue;
        }
        write_atomic(&output.join(path), &fs::read(stage.join(path))?)?;
    }
    if !output.join("project.godot").exists() {
        write_atomic(
            &output.join("project.godot"),
            &fs::read(stage.join("project.godot"))?,
        )?;
    }
    // Stale files are deleted only when still identical to the previous generated version.
    for (path, hash) in &old.files {
        let full = output.join(path);
        if !manifest.files.contains_key(path) && full.exists() {
            ensure!(
                digest(&fs::read(&full)?) == *hash,
                "stale generated file was edited: {}",
                full.display()
            );
            fs::remove_file(full)?;
        }
    }
    write_atomic(&old_path, &serde_json::to_vec_pretty(&manifest)?)?;
    Ok(manifest)
}
fn validate_output_path(root: &Path, path: &str) -> Result<()> {
    ensure!(
        path.starts_with("generated/hype/"),
        "manifest path outside generated content: {path}"
    );
    ensure!(
        !path.contains(['\\', '\0'])
            && path
                .split('/')
                .all(|c| !c.is_empty() && c != "." && c != ".."),
        "invalid generated path: {path}"
    );
    let mut current = root.to_path_buf();
    for component in Path::new(path).components() {
        ensure!(
            matches!(component, std::path::Component::Normal(_)),
            "non-relative generated path"
        );
        current.push(component);
        if let Ok(meta) = fs::symlink_metadata(&current) {
            ensure!(
                !meta.file_type().is_symlink(),
                "generated output contains symlink: {}",
                current.display()
            );
        }
    }
    Ok(())
}
