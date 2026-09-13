//! Publish selected content closures while preserving authored and edited files.
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
    selection: &str,
) -> Result<Manifest> {
    validate_output_path(output, "generated/hype/check")?;
    let old_path = output.join("generated/astrolabe-manifest.json");
    ensure!(
        !fs::symlink_metadata(&old_path).is_ok_and(|m| m.file_type().is_symlink()),
        "manifest is a symlink"
    );
    let old: Manifest = if old_path.exists() {
        serde_json::from_slice(&fs::read(&old_path)?)?
    } else {
        Manifest::default()
    };
    ensure!(
        old.schema == 0 || old.schema == 2,
        "old Godot output uses a superseded layout; generate into a fresh project"
    );
    for path in old.files.keys().chain(generated.iter()) {
        validate_output_path(output, path)?;
    }
    let mut selections = old.selections.clone();
    selections.insert(selection.into(), generated.clone());
    let retained: BTreeSet<String> = selections.values().flatten().cloned().collect();
    let other: BTreeSet<&String> = old
        .selections
        .iter()
        .filter(|(k, _)| *k != selection)
        .flat_map(|(_, v)| v)
        .collect();
    // All collisions, including stale-file edits, are checked before publishing anything.
    for (path, hash) in &old.files {
        let full = output.join(path);
        if !retained.contains(path) && full.exists() {
            ensure!(
                digest(&fs::read(&full)?) == *hash,
                "stale generated file was edited: {}",
                full.display()
            );
        }
    }
    let mut manifest = Manifest {
        schema: 2,
        godot_version: version,
        files: BTreeMap::new(),
        diagnostics,
        support_interface: super::support::INTERFACE,
        selections,
    };
    for path in &retained {
        if !generated.contains(path) {
            manifest.files.insert(
                path.clone(),
                old.files
                    .get(path)
                    .context("invalid ownership manifest")?
                    .clone(),
            );
            continue;
        }
        let destination = output.join(path);
        let hash = digest(&fs::read(stage.join(path))?);
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
        if other.contains(path) {
            ensure!(
                old.files.get(path) == Some(&hash),
                "shared generated resource changed while an unselected level depends on it: {path}; regenerate those levels together with compatible inputs"
            );
        }
        manifest.files.insert(path.clone(), hash);
    }
    super::support::initialize(output)?;
    for path in &generated {
        if old.files.get(path) == manifest.files.get(path) && output.join(path).exists() {
            continue;
        }
        write_atomic(&output.join(path), &fs::read(stage.join(path))?)?;
    }
    for path in old.files.keys().filter(|p| !retained.contains(*p)) {
        if output.join(path).exists() {
            fs::remove_file(output.join(path))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_closures_preserve_unselected_content_and_preflight_shared_edits() -> Result<()> {
        let output = tempfile::tempdir()?;
        let stage = tempfile::tempdir()?;
        let shared = "generated/hype/shared/prop.tres";
        let a = "generated/hype/levels/a/level.tscn";
        let b = "generated/hype/levels/b/level.tscn";
        for path in [shared, a, b] {
            write_atomic(&stage.path().join(path), path.as_bytes())?;
        }
        let publish_selection = |level: &str, paths: &[&str]| {
            publish(
                stage.path(),
                output.path(),
                paths.iter().map(|p| (*p).into()).collect(),
                vec![],
                "4.7.2".into(),
                level,
            )
        };
        publish_selection("a", &[shared, a])?;
        publish_selection("b", &[shared, b])?;
        write_atomic(&stage.path().join(a), b"edited a")?;
        publish_selection("a", &[shared, a])?;
        assert_eq!(fs::read(output.path().join(b))?, b.as_bytes());
        write_atomic(&stage.path().join(shared), b"changed shared resource")?;
        assert!(
            publish_selection("a", &[shared, a])
                .unwrap_err()
                .to_string()
                .contains("unselected level")
        );
        assert_eq!(fs::read(output.path().join(shared))?, shared.as_bytes());
        publish_selection("a", &[a])?;
        assert!(
            output.path().join(shared).exists(),
            "level b still owns shared resource"
        );
        publish_selection("b", &[b])?;
        assert!(
            !output.path().join(shared).exists(),
            "unchanged orphan was not removed"
        );
        Ok(())
    }
}
