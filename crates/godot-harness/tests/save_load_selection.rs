//! Selection, gold ownership and saved-level startup using production helpers.
use anyhow::{Context, Result, ensure};
use std::{fs, path::PathBuf, time::Duration};

#[test]
fn save_identity_gold_and_startup_require_complete_restoration() -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let project = temporary.path().join("project");
    astrolabe::godot::support::initialize(&project)?;
    fs::write(
        project.join("project.godot"),
        "config_version=5\n[application]\nconfig/name=\"SaveLoadSelectionRegression\"\n",
    )?;
    fs::write(
        project.join("tests/save_load_selection.gd"),
        include_bytes!("../../../godot/tests/save_load_selection.gd"),
    )?;
    let evidence = std::env::var_os("ASTROLABE_EVIDENCE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| temporary.path().join("evidence"))
        .join("save-load-selection");
    fs::create_dir_all(&evidence)?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    for syntax in [true, false] {
        let stage = if syntax { "syntax" } else { "contract" };
        let log_path = evidence.join(format!("{stage}-engine.log"));
        let log_arg = log_path.to_str().context("non-UTF-8 log path")?;
        let mut args = vec!["--headless", "--log-file", log_arg];
        if syntax {
            args.push("--check-only");
        }
        args.extend(["--script", "res://tests/save_load_selection.gd"]);
        let log = astrolabe::godot::run_godot(&engine, &project, &args, Duration::from_secs(10))?;
        fs::write(evidence.join(format!("{stage}.log")), &log)?;
        let diagnostics = fs::read_to_string(&log_path)?;
        ensure!(
            !diagnostics.contains("SCRIPT ERROR:") && !diagnostics.contains("ERROR:"),
            "save selection {stage} diagnostics: {diagnostics}"
        );
        ensure!(
            syntax || log.contains("SAVE_LOAD_SELECTION_PROOF"),
            "save selection contract did not finish: {log}"
        );
    }
    Ok(())
}
