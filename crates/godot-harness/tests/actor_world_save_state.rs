//! Production slot regression replacing f21a7fc's detached Boolean helper test.

use anyhow::{Context, Result, ensure};
use std::{fs, path::PathBuf, process::Command};

#[test]
fn actor_brain_state_survives_fresh_process_reload() -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let project = temporary.path().join("project");
    astrolabe::godot::support::initialize(&project)?;
    // Use packaged production files, with a source-free headless scene fixture.
    fs::write(
        project.join("project.godot"),
        "config_version=5\n[application]\nconfig/name=\"ActorSlotRegression\"\n",
    )?;
    fs::write(
        project.join("tests/actor_world_save_state.gd"),
        include_bytes!("../../../godot/tests/actor_world_save_state.gd"),
    )?;
    fs::write(
        project.join("tests/actor_world_save_fixture.gd"),
        include_bytes!("../../../godot/tests/actor_world_save_fixture.gd"),
    )?;
    let evidence = std::env::var_os("ASTROLABE_EVIDENCE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| temporary.path().join("evidence"))
        .join("actor-world-save-state");
    fs::create_dir_all(&evidence)?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    for (stage, marker) in [
        ("syntax", None),
        ("write", Some("ACTOR_SLOT_WRITE production slot persisted")),
        (
            "reload",
            Some("ACTOR_SLOT_RELOAD fresh process restored named Boolean fields"),
        ),
    ] {
        let owned_log = evidence.join(format!("{stage}-engine.log"));
        let mut command = Command::new(&engine);
        command
            .arg("--headless")
            .arg("--path")
            .arg(&project)
            .arg("--log-file")
            .arg(&owned_log);
        if stage == "syntax" {
            command.arg("--check-only");
        }
        let output = command
            .args(["--script", "res://tests/actor_world_save_state.gd"])
            .args(["--", stage])
            .env("XDG_DATA_HOME", temporary.path().join("data"))
            .env("XDG_CONFIG_HOME", temporary.path().join("config"))
            .env("XDG_CACHE_HOME", temporary.path().join("cache"))
            .env_remove("DISPLAY")
            .env_remove("WAYLAND_DISPLAY")
            .output()
            .with_context(|| format!("starting Godot {stage} production-slot regression"))?;
        let log = format!(
            "exit_status: {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        fs::write(evidence.join(format!("{stage}.log")), &log)?;
        ensure!(
            output.status.success()
                && !log.contains("SCRIPT ERROR:")
                && !log.contains("ERROR:")
                && marker.is_none_or(|marker| log.contains(marker)),
            "Godot {stage} production-slot regression failed: {log}"
        );
        if stage == "write" {
            fs::copy(
                temporary
                    .path()
                    .join("data/godot/app_userdata/ActorSlotRegression/manoir-save.cfg"),
                evidence.join("synthetic-slot.json"),
            )?;
        }
    }
    Ok(())
}
