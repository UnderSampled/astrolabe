use anyhow::{Context, Result, ensure};
use astrolabe::godot;
use std::{fs, path::PathBuf, time::Duration};

#[test]
fn native_payload_banks_preserve_shared_configuration_and_activation_state() -> Result<()> {
    let output = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("output");
    fs::create_dir_all(&output)?;
    let project = tempfile::tempdir_in(output)?;
    godot::support::initialize(project.path())?;
    fs::create_dir_all(project.path().join("tests"))?;
    fs::write(
        project.path().join("tests/configured_bank_lifecycle.gd"),
        include_bytes!("../godot/tests/configured_bank_lifecycle.gd"),
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let log = project.path().join("configured-bank-lifecycle.log");
    let log_arg = log.to_str().context("non-UTF-8 Godot log path")?;
    let run = godot::run_godot(
        &engine,
        project.path(),
        &[
            "--headless",
            "--script",
            "res://tests/configured_bank_lifecycle.gd",
            "--log-file",
            log_arg,
        ],
        Duration::from_secs(30),
    );
    let diagnostics = fs::read_to_string(&log).unwrap_or_default();
    let output = run.map_err(|error| anyhow::anyhow!("{error:#}\nGodot log:\n{diagnostics}"))?;
    ensure!(
        output.contains("CONFIGURED_BANK_LIFECYCLE_OK")
            || diagnostics.contains("CONFIGURED_BANK_LIFECYCLE_OK"),
        "Native bank lifecycle marker missing; stdout:\n{output}\nlog:\n{diagnostics}"
    );
    ensure!(
        !diagnostics.contains("SCRIPT ERROR:") && !diagnostics.contains("ERROR:"),
        "Godot reported native bank errors:\n{diagnostics}"
    );
    Ok(())
}
