//! One maintained support source; installation, updates and content are separate operations.
use super::digest;
use crate::files::write_atomic;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

pub const INTERFACE: u32 = 71;
const PACKAGE: &str = "addons/astrolabe";
const RECEIPT: &str = "addons/astrolabe/installed.json";

macro_rules! assets {
    ($($path:literal),* $(,)?) => { &[$(($path, include_bytes!(concat!("../../godot/", $path)).as_slice())),*] };
}
const SUPPORT: &[(&str, &[u8])] = assets![
    "addons/astrolabe/ui/map_name.gd",
    "addons/astrolabe/ui/map_name.gd.uid",
    "addons/astrolabe/ui/map_names.gd",
    "addons/astrolabe/ui/map_names.gd.uid",
    "addons/astrolabe/behavior/behavior.gd",
    "addons/astrolabe/behavior/behavior.gd.uid",
    "addons/astrolabe/character/mechanics.gd.uid",
    "addons/astrolabe/character/physical_environment.gd.uid",
    "addons/astrolabe/character/cue.gd.uid",
    "addons/astrolabe/character/character.gd.uid",
    "addons/astrolabe/audio/voice.gd.uid",
    "addons/astrolabe/audio/emitter.gd.uid",
    "addons/astrolabe/character/character.gd",
    "addons/astrolabe/character/configured_slot.gd",
    "addons/astrolabe/character/configured_slot.gd.uid",
    "addons/astrolabe/character/configured_payload.gd",
    "addons/astrolabe/character/configured_payload.gd.uid",
    "addons/astrolabe/character/configuration.gd",
    "addons/astrolabe/character/configuration.gd.uid",
    "addons/astrolabe/character/inventory_definition.gd",
    "addons/astrolabe/character/inventory_definition.gd.uid",
    "addons/astrolabe/character/armor_state.gd",
    "addons/astrolabe/character/snapshot_values.gd",
    "addons/astrolabe/character/snapshot_values.gd.uid",
    "addons/astrolabe/character/armor_catalog.gd",
    "addons/astrolabe/character/armor_catalog.gd.uid",
    "addons/astrolabe/character/armor_definition.gd",
    "addons/astrolabe/character/armor_definition.gd.uid",
    "addons/astrolabe/character/armor_piece.gd",
    "addons/astrolabe/character/armor_piece.gd.uid",
    "addons/astrolabe/character/particle_source.gd",
    "addons/astrolabe/character/particle_source.gd.uid",
    "addons/astrolabe/character/particle_generator.gd",
    "addons/astrolabe/character/particle_generator.gd.uid",
    "addons/astrolabe/character/route.gd",
    "addons/astrolabe/character/route.gd.uid",
    "addons/astrolabe/character/waypoint.gd",
    "addons/astrolabe/character/rendering.gd",
    "addons/astrolabe/character/rendering.gd.uid",
    "addons/astrolabe/character/inventory.gd",
    "addons/astrolabe/character/inventory_stock.gd",
    "addons/astrolabe/character/inventory_stock.gd.uid",
    "addons/astrolabe/character/magick.gd",
    "addons/astrolabe/character/hit_points.gd",
    "addons/astrolabe/character/hit_points.gd.uid",
    "addons/astrolabe/character/laser_pointer.gd",
    "addons/astrolabe/character/cue.gd",
    "addons/astrolabe/character/mechanics.gd",
    "addons/astrolabe/character/physical_environment.gd",
    "addons/astrolabe/camera/source_camera.gd",
    "addons/astrolabe/camera/source_controller_state.gd",
    "addons/astrolabe/camera/source_controller_state.gd.uid",
    "addons/astrolabe/audio/emitter.gd",
    "addons/astrolabe/audio/voice.gd",
    "addons/astrolabe/ui/inventory/item.gd",
    "addons/astrolabe/ui/inventory/item.gd.uid",
    "addons/astrolabe/ui/inventory/category.gd",
    "addons/astrolabe/ui/inventory/category.gd.uid",
    "addons/astrolabe/ui/inventory/slot.gd",
    "addons/astrolabe/ui/inventory/slot.gd.uid",
    "addons/astrolabe/ui/inventory/screen.gd",
    "addons/astrolabe/ui/inventory/screen.gd.uid",
    "addons/astrolabe/ui/menu/menu.gd",
    "addons/astrolabe/ui/menu/menu.gd.uid",
    "addons/astrolabe/ui/menu/button.gd",
    "addons/astrolabe/ui/menu/button.gd.uid",
    "addons/astrolabe/ui/menu/theme.tres",
    "addons/astrolabe/ui/screen.gd",
    "addons/astrolabe/ui/screen.gd.uid",
    "addons/astrolabe/ui/screen.tscn",
    "addons/astrolabe/ui/dialogue.gd",
    "addons/astrolabe/ui/dialogue.gd.uid",
    "addons/astrolabe/ui/dialogue.tscn",
    "addons/astrolabe/ui/dialogue_presentation.gd",
    "addons/astrolabe/ui/dialogue_presentation.gd.uid",
    "addons/astrolabe/ui/dialogue_frame.gd",
    "addons/astrolabe/ui/dialogue_frame.gd.uid",
    "addons/astrolabe/ui/dialogue_frame.tscn",
    "addons/astrolabe/ui/dialogue_span_view.tscn",
    "addons/astrolabe/ui/dialogue_item.gd",
    "addons/astrolabe/ui/dialogue_item.gd.uid",
    "addons/astrolabe/ui/dialogue_line.gd",
    "addons/astrolabe/ui/dialogue_line.gd.uid",
    "addons/astrolabe/ui/dialogue_span.gd",
    "addons/astrolabe/ui/dialogue_span.gd.uid",
    "addons/astrolabe/ui/hud.gd",
    "addons/astrolabe/ui/hud.gd.uid",
    "addons/astrolabe/ui/hud.tscn",
    "addons/astrolabe/ui/theme.tres",
    "addons/astrolabe/version.json",
    "addons/astrolabe/README.md",
    "addons/astrolabe/rendering/sprite.gd",
    "addons/astrolabe/rendering/sprite.gd.uid",
    "addons/astrolabe/rendering/visual_material_state.gd",
    "addons/astrolabe/rendering/source_diffuse_dynamic.gdshader",
    "addons/astrolabe/rendering/source_diffuse_prelit.gdshader",
    "addons/astrolabe/scene/sector_boundary.gd",
    "addons/astrolabe/scene/sector_boundary.gd.uid",
];
const STARTER: &[(&str, &[u8])] = assets![
    "project.godot",
    "README.md",
    ".gitignore",
    "game/bootstrap.gd",
    "game/bootstrap.gd.uid",
    "game/bootstrap.tscn",
    "game/credits_menu.gd",
    "game/credits_menu.tscn",
    "game/content_cache.gd",
    "game/content_cache.gd.uid",
    "game/level_transition.gd",
    "game/level_transition.gd.uid",
    "game/inventory_host.gd",
    "game/inventory_host.gd.uid",
    "game/pause_host.gd",
    "game/pause_host.gd.uid",
    "game/pause_host.tscn",
    "game/demo.gd",
    "game/demo.gd.uid",
    "game/demo.tscn",
    "game/new_game.tscn",
    "game/demo_camera.gd",
    "game/demo_camera.gd.uid",
    "game/manoir_start.gd",
    "game/manoir_start.gd.uid",
    "game/save_state.gd",
    "game/save_state.gd.uid",
    "game/settings_menu.gd",
    "game/user_settings.gd",
    "game/english_save_manoir.json",
    "game/characters/luminous_hype.tscn",
    "tests/example.tscn",
];

#[derive(Default, Serialize, Deserialize)]
struct Receipt {
    interface: u32,
    files: BTreeMap<String, String>,
}
fn source_checkout(output: &Path) -> bool {
    output.exists()
        && output.canonicalize().ok()
            == Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("godot")
                .canonicalize()
                .ok()
}
fn checked_path(root: &Path, relative: &str) -> Result<()> {
    ensure!(
        relative
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..")
            && !relative.contains(['\\', '\0']),
        "invalid support path"
    );
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
        if let Ok(meta) = fs::symlink_metadata(&path) {
            ensure!(
                !meta.file_type().is_symlink(),
                "support/project path is a symlink: {}",
                path.display()
            );
        }
    }
    Ok(())
}
pub(super) fn compatible(output: &Path) -> Result<()> {
    checked_path(output, "addons/astrolabe/version.json")?;
    let version: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join(PACKAGE).join("version.json"))?)?;
    ensure!(
        version["interface"].as_u64() == Some(u64::from(INTERFACE)),
        "incompatible Astrolabe support interface; expected {INTERFACE}; run update-godot-support explicitly"
    );
    Ok(())
}
/// Prepare validation with the consumer's actual support (including local edits).
pub(super) fn stage(stage: &Path, output: &Path) -> Result<()> {
    for (path, bytes) in STARTER {
        write_atomic(&stage.join(path), bytes)?;
    }
    let installed = output.join(PACKAGE).exists();
    if installed {
        compatible(output)?;
    }
    for (path, bytes) in SUPPORT {
        checked_path(output, path)?;
        let bytes = if installed {
            fs::read(output.join(path)).with_context(|| {
                format!("missing installed support {path}; update support explicitly")
            })?
        } else {
            bytes.to_vec()
        };
        write_atomic(&stage.join(path), &bytes)?;
    }
    Ok(())
}
/// Initialize once. An existing project's adopted settings and authored files survive.
pub fn initialize(output: &Path) -> Result<()> {
    for (path, _) in STARTER.iter().chain(SUPPORT) {
        checked_path(output, path)?;
    }
    if output.join(PACKAGE).exists() {
        compatible(output)?;
    } else {
        update(output)?;
    }
    for (path, bytes) in STARTER {
        if !output.join(path).exists() {
            write_atomic(&output.join(path), bytes)?;
        }
    }
    Ok(())
}
/// Explicit, preflighted package update. Never overwrites edited or unowned support.
pub fn update(output: &Path) -> Result<()> {
    ensure!(
        !source_checkout(output),
        "the source checkout owns support through Git; edit/review godot/addons/astrolabe instead of installing bundled copies"
    );
    checked_path(output, RECEIPT)?;
    let old: Receipt = if output.join(RECEIPT).exists() {
        serde_json::from_slice(&fs::read(output.join(RECEIPT))?)?
    } else {
        Receipt::default()
    };
    let mut next = Receipt {
        interface: INTERFACE,
        ..Default::default()
    };
    for path in old.files.keys() {
        ensure!(
            path.starts_with("addons/astrolabe/"),
            "support receipt outside package"
        );
        checked_path(output, path)?;
        if output.join(path).exists() {
            ensure!(
                digest(&fs::read(output.join(path))?) == old.files[path],
                "support file was edited: {path}; preserve/review it before updating"
            );
        }
    }
    for (path, bytes) in SUPPORT {
        checked_path(output, path)?;
        let hash = digest(bytes);
        if output.join(path).exists() && !old.files.contains_key(*path) {
            ensure!(
                digest(&fs::read(output.join(path))?) == hash,
                "unowned support collision: {path}"
            );
        }
        next.files.insert((*path).into(), hash);
    }
    for (path, bytes) in SUPPORT {
        write_atomic(&output.join(path), bytes)?;
    }
    for path in old.files.keys().filter(|p| !next.files.contains_key(*p)) {
        if output.join(path).exists() {
            fs::remove_file(output.join(path))?;
        }
    }
    write_atomic(&output.join(RECEIPT), &serde_json::to_vec_pretty(&next)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    #[test]
    fn adoption_update_and_support_edits_have_distinct_ownership() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let output = temp.path();
        initialize(output)?;
        let project = b"; independently adopted settings\n[application]\nconfig/name=\"Mine\"\n";
        fs::write(output.join("project.godot"), project)?;
        fs::write(output.join("game/variation.tscn"), b"authored composition")?;
        // Simulate a previously installed support revision, using its receipt.
        let version_path = "addons/astrolabe/version.json";
        let previous = b"{\"interface\":0}";
        fs::write(output.join(version_path), previous)?;
        let mut receipt: Receipt = serde_json::from_slice(&fs::read(output.join(RECEIPT))?)?;
        receipt.interface = 0;
        receipt.files.insert(version_path.into(), digest(previous));
        fs::write(output.join(RECEIPT), serde_json::to_vec(&receipt)?)?;
        assert!(compatible(output).is_err());
        update(output)?;
        compatible(output)?;
        initialize(output)?;
        assert_eq!(fs::read(output.join("project.godot"))?, project);
        assert_eq!(
            fs::read(output.join("game/variation.tscn"))?,
            b"authored composition"
        );
        let edited = "addons/astrolabe/rendering/sprite.gd";
        let mut bytes = fs::read(output.join(edited))?;
        bytes.extend(b"\n# authored support experiment\n");
        fs::write(output.join(edited), &bytes)?;
        let stage_dir = tempfile::tempdir()?;
        stage(stage_dir.path(), output)?;
        assert_eq!(fs::read(stage_dir.path().join(edited))?, bytes);
        assert!(
            stage_dir
                .path()
                .join("addons/astrolabe/character/snapshot_values.gd")
                .is_file()
        );
        assert!(
            update(output)
                .unwrap_err()
                .to_string()
                .contains("was edited")
        );
        assert_eq!(fs::read(output.join(edited))?, bytes);
        Ok(())
    }

    #[test]
    fn every_bundled_gdscript_preload_is_staged() -> Result<()> {
        let files: BTreeSet<&str> = STARTER
            .iter()
            .chain(SUPPORT)
            .map(|(path, _)| *path)
            .collect();
        for (path, bytes) in STARTER.iter().chain(SUPPORT) {
            if !path.ends_with(".gd") {
                continue;
            }
            let source = std::str::from_utf8(bytes)?;
            for prefix in ["preload(\"res://", "load(\"res://"] {
                let mut offset = 0;
                while let Some(relative) = source[offset..].find(prefix) {
                    let start = offset + relative + prefix.len();
                    let Some(end) = source[start..].find('"') else {
                        anyhow::bail!("unterminated resource path in {path}");
                    };
                    let dependency = &source[start..start + end];
                    if dependency.starts_with("addons/astrolabe/") {
                        ensure!(
                            files.contains(dependency),
                            "{path} references {dependency}, which the support stage omits"
                        );
                    }
                    offset = start + end + 1;
                }
            }
        }
        Ok(())
    }

    #[test]
    fn actor_display_receiver_helper_is_in_interface_70_and_staged() -> Result<()> {
        let output = tempfile::tempdir()?;
        initialize(output.path())?;
        let version: serde_json::Value = serde_json::from_slice(&fs::read(
            output.path().join("addons/astrolabe/version.json"),
        )?)?;
        assert_eq!(version["interface"].as_u64(), Some(u64::from(INTERFACE)));
        assert_eq!(INTERFACE, 71);

        let staged = tempfile::tempdir()?;
        stage(staged.path(), output.path())?;
        for path in [
            "addons/astrolabe/character/snapshot_values.gd",
            "addons/astrolabe/character/snapshot_values.gd.uid",
            "addons/astrolabe/character/configured_slot.gd",
            "addons/astrolabe/character/configured_slot.gd.uid",
            "addons/astrolabe/character/configured_payload.gd",
            "addons/astrolabe/character/configured_payload.gd.uid",
            "addons/astrolabe/character/configuration.gd",
            "addons/astrolabe/character/configuration.gd.uid",
            "addons/astrolabe/character/inventory_definition.gd",
            "addons/astrolabe/character/inventory_definition.gd.uid",
            "addons/astrolabe/character/inventory_stock.gd",
            "addons/astrolabe/character/inventory_stock.gd.uid",
            "addons/astrolabe/ui/map_name.gd",
            "addons/astrolabe/ui/map_names.gd",
        ] {
            assert_eq!(
                fs::read(staged.path().join(path))?,
                fs::read(output.path().join(path))?,
                "staged support differs for {path}"
            );
        }
        Ok(())
    }
}
