//! Normal New Game checkpoint through the actual menu, with private saves and logs.
use anyhow::{Context, Result, ensure};
use std::{fs, path::PathBuf, process::Command};

#[test]
#[ignore = "requires current generated manor/pause assets and Godot; full intro parity pending"]
fn menu_new_game_preserves_slot_and_restarts_original_opening() -> Result<()> {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../godot");
    ensure!(
        project
            .join("generated/hype/levels/manoir/level.tscn")
            .is_file(),
        "generate the local manor before checking New Game"
    );
    ensure!(
        project
            .join("generated/hype/shared/menu/pause/en/menu.tscn")
            .is_file(),
        "generate the source pause menu before checking New Game"
    );
    let manifest: serde_json::Value = serde_json::from_slice(&fs::read(
        project.join("generated/astrolabe-manifest.json"),
    )?)?;
    ensure!(
        manifest["support_interface"] == astrolabe::godot::support::INTERFACE,
        "regenerate stale manor assets before checking the current opening"
    );
    let gogoud_code =
        fs::read_to_string(project.join("generated/hype/levels/manoir/ai/models/210.gd"))?;
    ensure!(
        gogoud_code.contains("source_reset_route(0)")
            && gogoud_code.contains("source_begin_route(6)"),
        "Gogoud's exported original AI still lacks the implemented route calls"
    );
    let temporary = tempfile::tempdir()?;
    let evidence = std::env::var_os("ASTROLABE_EVIDENCE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| temporary.path().join("evidence"))
        .join("new-game-opening");
    fs::create_dir_all(&evidence)?;
    // Keep the actual full source compilation even when a runtime assertion fails.
    for name in [
        "generated/astrolabe-manifest.json",
        "generated/hype/levels/manoir/ai/models/154.gd",
        "generated/hype/levels/manoir/ai/models/210.gd",
        "generated/hype/levels/manoir/ai/models/fix/28.gd",
    ] {
        fs::copy(
            project.join(name),
            evidence.join(PathBuf::from(name).file_name().unwrap()),
        )?;
    }
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let syntax = temporary.path().join("syntax");
    astrolabe::godot::support::initialize(&syntax)?;
    fs::create_dir_all(syntax.join("tests"))?;
    fs::write(
        syntax.join("tests/new_game_smoke.gd"),
        include_bytes!("../../../godot/tests/new_game_smoke.gd"),
    )?;
    for (name, script) in [
        ("session", "res://game/game_session.gd"),
        ("menu", "res://game/bootstrap.gd"),
        ("smoke", "res://tests/new_game_smoke.gd"),
    ] {
        let parser_log = evidence.join(format!("parser-{name}.log"));
        astrolabe::godot::run_godot(
            &engine,
            &syntax,
            &[
                "--headless",
                "--check-only",
                "--script",
                script,
                "--log-file",
                parser_log.to_str().context("parser log path")?,
            ],
            std::time::Duration::from_secs(10),
        )?;
        ensure!(fs::metadata(&parser_log)?.len() <= 1024 * 1024);
        ensure!(!fs::read_to_string(&parser_log)?.contains("ERROR:"));
    }
    let report = evidence.join("report.json");
    let engine_log = evidence.join("engine.log");
    let xvfb = std::env::var("ASTROLABE_XVFB_RUN").unwrap_or_else(|_| "xvfb-run".into());
    // The declared nextest deadline and owning supervisor bound this whole
    // process tree. The script also bounds both session loads at 70 seconds.
    let output = Command::new(&xvfb)
        .args(["-a", "-s", "-screen 0 1024x768x24 -nolisten tcp"])
        .arg(&engine)
        .args([
            "--path",
            project.to_str().context("project path")?,
            "--script",
            "res://tests/new_game_smoke.gd",
            "--log-file",
            engine_log.to_str().context("engine log path")?,
            "--",
            report.to_str().context("report path")?,
        ])
        .env("XDG_DATA_HOME", temporary.path().join("data"))
        .env("XDG_CONFIG_HOME", temporary.path().join("config"))
        .env("XDG_CACHE_HOME", temporary.path().join("cache"))
        .env_remove("WAYLAND_DISPLAY")
        .env("XDG_SESSION_TYPE", "x11")
        .output()
        .context("launch normal New Game in isolated Xvfb")?;
    let log = format!(
        "status: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    fs::write(evidence.join("stdout.log"), &log)?;
    // A rough opening deliberately retains unsupported-AI failures. All
    // parser, renderer, exit-status and other engine errors remain fatal.
    ensure!(output.status.success(), "New Game process failed: {log}");
    ensure!(
        !log.contains("SCRIPT ERROR:"),
        "New Game script failed: {log}"
    );
    ensure!(
        log.lines().filter(|line| line.starts_with("ERROR:")).all(
            |line| line.starts_with("ERROR: Unsupported compiled behavior:")
                // The smoke script stashes new_game.tscn on purpose to prove
                // the menu recovers from a failed New Game destination.
                || line.contains("res://game/new_game.tscn")
        ),
        "unexpected New Game error: {log}"
    );
    ensure!(
        log.contains("NEW_GAME_AND_LOAD_GAME_ROUTES_OK"),
        "New Game checkpoint marker missing: {log}"
    );
    let result: serde_json::Value = serde_json::from_slice(&fs::read(report)?)?;
    ensure!(result["status"] == "passed" && result["save_preserved"] == true);
    ensure!(
        result["pause_resume_and_main_menu"] == true,
        "normal New Game did not validate pause, Resume and Main Menu"
    );
    ensure!(
        result["hype_particle_initialization"]["initialized"] == true
            && result["hype_particle_initialization"]["capacity"] == 200
            && result["hype_particle_initialization"]["emitting"] == false,
        "normal New Game did not validate source particle initialization"
    );
    println!(
        "Normal New Game checkpoint evidence: {}",
        evidence.display()
    );
    Ok(())
}
