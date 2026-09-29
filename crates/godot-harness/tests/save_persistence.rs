use anyhow::{Context, Result, ensure};
use std::{fs, path::PathBuf, process::Command};

#[test]
#[ignore = "requires generated local manor and Godot; selected by --local"]
fn native_manor_save_survives_fresh_process_and_rejects_bad_slots() -> Result<()> {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../godot");
    ensure!(
        project
            .join("generated/hype/levels/manoir/level.tscn")
            .is_file(),
        "generate the local manor before running the save proof"
    );
    let temporary = tempfile::tempdir()?;
    let evidence = if let Some(root) = std::env::var_os("ASTROLABE_EVIDENCE_DIR") {
        let path = PathBuf::from(root).join("godot-save-persistence");
        fs::create_dir_all(&path)?;
        path
    } else {
        temporary.path().join("evidence")
    };
    fs::create_dir_all(&evidence)?;
    let data = temporary.path().join("data");
    fs::create_dir_all(&data)?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let xvfb_run = std::env::var("ASTROLABE_XVFB_RUN").unwrap_or_else(|_| "xvfb-run".into());
    for (stage, expected) in [
        (
            "write",
            "SAVE_PROOF wrote mana=23, inventory=2, and dummy dsg_8=true",
        ),
        (
            "reload",
            "SAVE_PROOF fresh process restored mana=23, inventory=2, and dummy dsg_8=true",
        ),
        (
            "corrupt",
            "SAVE_PROOF corrupt, unsupported version, extra state, and noncanonical inventory keys rejected",
        ),
    ] {
        let output = Command::new(&xvfb_run)
            .args(["-a", "-s", "-screen 0 1024x768x24 -nolisten tcp"])
            .arg(&engine)
            .args([
                "--path",
                project.to_str().context("non-UTF-8 project path")?,
                "--script",
                "res://tests/save_persistence.gd",
                "--",
                stage,
            ])
            .env("XDG_DATA_HOME", &data)
            .env_remove("WAYLAND_DISPLAY")
            .env("XDG_SESSION_TYPE", "x11")
            .output()
            .with_context(|| format!("starting Godot {stage} stage in authenticated Xvfb"))?;
        let log = format!(
            "exit_status: {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        fs::write(evidence.join(format!("{stage}.log")), &log)?;
        let proof_found = log.contains(expected);
        let first_unrecognized_error = log
            .lines()
            .find(|line| line.starts_with("ERROR:") && !allowed_error(stage, line));
        let first_error = log.lines().find(|line| line.starts_with("ERROR:"));
        let first_script_error = log.lines().find(|line| line.starts_with("SCRIPT ERROR:"));
        let classified = classify_stage_log(
            stage,
            expected,
            &log,
            output.status.success(),
            output.status.code(),
        );
        ensure!(
            classified.is_ok(),
            "Godot {stage} stage failed: status={}, proof_marker={proof_found}, first_unrecognized_error={first_unrecognized_error:?}, first_error={first_error:?}, first_script_error={first_script_error:?}, classifier={:?}; see {}",
            output.status,
            classified.err(),
            evidence.join(format!("{stage}.log")).display(),
        );
        if stage == "write" {
            let slot = data.join("godot/app_userdata/Astrolabe/manoir-save-v1.json");
            fs::copy(&slot, evidence.join("saved-slot.json"))
                .with_context(|| format!("preserving native slot {}", slot.display()))?;
        }
    }
    Ok(())
}

fn classify_stage_log(
    stage: &str,
    expected_marker: &str,
    log: &str,
    status_success: bool,
    status_code: Option<i32>,
) -> Result<()> {
    ensure!(
        log.contains(expected_marker),
        "missing stage success marker"
    );
    if let Some(line) = log.lines().find(|line| line.starts_with("SCRIPT ERROR:")) {
        anyhow::bail!("unexpected script error: {line}");
    }

    let errors = log
        .lines()
        .filter(|line| line.starts_with("ERROR:"))
        .collect::<Vec<_>>();
    if let Some(line) = errors.iter().find(|line| !allowed_error(stage, line)) {
        anyhow::bail!("unexpected engine error: {line}");
    }

    if status_success {
        return Ok(());
    }
    if status_code == Some(1)
        && !errors.is_empty()
        && errors.iter().all(|line| known_scene_error(line))
    {
        return Ok(());
    }
    anyhow::bail!("unexpected process exit status: code={status_code:?}")
}

fn allowed_error(stage: &str, line: &str) -> bool {
    known_scene_error(line) || (stage == "corrupt" && line.starts_with("ERROR: Parse JSON failed."))
}

fn known_scene_error(line: &str) -> bool {
    line.starts_with("ERROR: Attempting to initialize the wrong RID")
        || line.starts_with("ERROR: Initializing already initialized RID")
        || line.starts_with("ERROR: Parameter \"mem\" is null.")
        || line.starts_with("ERROR: Parameter \"m\" is null.")
        || line.starts_with("ERROR: unimplemented base type encountered in renderer scene cull")
        || line.starts_with("ERROR: Unsupported compiled behavior:")
        || line.starts_with("ERROR: Parameter \"t\" is null.")
        || line.starts_with("ERROR: Another resource is loaded from path ")
        || line.starts_with(
            "ERROR: 1 RID allocations of type 'PN13RendererDummy14TextureStorage12DummyTextureE' were leaked at exit.",
        )
}

#[cfg(test)]
mod tests {
    use super::classify_stage_log;

    const MARKER: &str = "SAVE_PROOF stage completed";
    const KNOWN: &str = "ERROR: Unsupported compiled behavior: test";

    #[test]
    fn rejects_unknown_engine_error_even_when_process_exits_successfully() {
        let log = format!("{MARKER}\nERROR: unknown failure\n");
        assert!(classify_stage_log("write", MARKER, &log, true, Some(0)).is_err());
    }

    #[test]
    fn rejects_script_error_even_with_known_engine_diagnostics_and_exit_one() {
        let log = format!("{MARKER}\n{KNOWN}\nSCRIPT ERROR: unknown failure\n");
        assert!(classify_stage_log("reload", MARKER, &log, false, Some(1)).is_err());
    }

    #[test]
    fn rejects_missing_success_marker() {
        assert!(classify_stage_log("write", MARKER, "completed\n", true, Some(0)).is_err());
    }

    #[test]
    fn accepts_clean_success_and_known_scene_diagnostics() {
        assert!(classify_stage_log("write", MARKER, MARKER, true, Some(0)).is_ok());
        let with_diagnostic = format!("{MARKER}\n{KNOWN}\n");
        assert!(classify_stage_log("reload", MARKER, &with_diagnostic, true, Some(0)).is_ok());
        assert!(classify_stage_log("reload", MARKER, &with_diagnostic, false, Some(1)).is_ok());
    }

    #[test]
    fn accepts_expected_corrupt_slot_diagnostic_only_in_corrupt_stage() {
        let log = format!("{MARKER}\nERROR: Parse JSON failed.\n");
        assert!(classify_stage_log("corrupt", MARKER, &log, true, Some(0)).is_ok());
        assert!(classify_stage_log("write", MARKER, &log, true, Some(0)).is_err());
    }
}
