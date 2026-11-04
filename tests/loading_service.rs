//! Native lifecycle checks for the actual production LevelTransition script.
use anyhow::{Context, Result, ensure};
use astrolabe::godot;
use std::{fs, time::Duration};

#[test]
fn production_loading_limit_restores_on_terminal_cancel_and_exit() -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let project = temporary.path();
    fs::create_dir(project.join("game"))?;
    fs::write(project.join("project.godot"), "config_version=5\n")?;
    fs::write(
        project.join("game/level_transition.gd"),
        include_bytes!("../godot/game/level_transition.gd"),
    )?;
    // level_transition.gd preloads the script warm-up (48680b0), which
    // compiles the shared generated scripts of a real project first.
    fs::create_dir_all(project.join("generated/hype/shared"))?;
    fs::write(
        project.join("game/script_warmup.gd"),
        include_bytes!("../godot/game/script_warmup.gd"),
    )?;
    fs::write(
        project.join("check.gd"),
        include_bytes!("../godot/tests/check_level_loading_cap.gd"),
    )?;
    fs::write(
        project.join("blocking_resource.gd"),
        "extends Resource\nstatic var block_next := false\nstatic var entered := Semaphore.new()\nstatic var release := Semaphore.new()\nfunc _init() -> void:\n\tif block_next:\n\t\tblock_next = false\n\t\tentered.post()\n\t\trelease.wait()\n",
    )?;
    for name in [
        "cap0",
        "cap120",
        "cap20",
        "cap30",
        "installfail",
        "exit",
        "cancel",
        "invalid",
    ] {
        let directory = project.join(format!("generated/hype/levels/{name}"));
        fs::create_dir_all(&directory)?;
        if name == "cancel" {
            fs::write(
                directory.join("blocked.tres"),
                "[gd_resource type=\"Resource\" load_steps=2 format=3]\n[ext_resource type=\"Script\" path=\"res://blocking_resource.gd\" id=\"block\"]\n[resource]\nscript = ExtResource(\"block\")\n",
            )?;
            fs::write(
                directory.join("level.tscn"),
                "[gd_scene load_steps=2 format=3]\n[ext_resource type=\"Resource\" path=\"blocked.tres\" id=\"blocked\"]\n[node name=\"Level\" type=\"Node3D\"]\nmetadata/block = ExtResource(\"blocked\")\n",
            )?;
        } else if name == "invalid" {
            fs::write(
                directory.join("level.tscn"),
                "[gd_scene format=3]\n[node name=\"InvalidLevel\" type=\"Node\"]\n",
            )?;
        } else {
            fs::write(
                directory.join("level.tscn"),
                "[gd_scene format=3]\n[node name=\"Level\" type=\"Node3D\"]\n",
            )?;
        }
    }
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    for (name, mode) in [("parser", "--check-only"), ("runtime", "--headless")] {
        let log = project.join(format!("{name}.log"));
        let output = godot::run_godot(
            &engine,
            project,
            &[
                "--headless",
                mode,
                "--log-file",
                log.to_str().context("log path")?,
                "--script",
                "res://check.gd",
            ],
            Duration::from_secs(15),
        )?;
        let diagnostics = fs::read_to_string(log)?;
        ensure!(
            !diagnostics.contains("ERROR:"),
            "loading service diagnostics: {diagnostics}"
        );
        if name == "runtime" {
            ensure!(
                !output.contains("ObjectDB instance was leaked"),
                "loading service leaked a native node: {output}"
            );
            ensure!(
                output.contains("LEVEL_LOADING_CAP_OK"),
                "missing lifecycle evidence: {output}"
            );
        }
    }
    Ok(())
}

#[test]
fn production_loading_presenter_draws_source_vignette_or_status() -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let project = temporary.path();
    fs::write(project.join("project.godot"), "config_version=5\n")?;
    for (path, bytes) in [
        (
            "addons/astrolabe/ui/loading/screen.gd",
            &include_bytes!("../godot/addons/astrolabe/ui/loading/screen.gd")[..],
        ),
        (
            "addons/astrolabe/ui/loading/screen.tscn",
            include_bytes!("../godot/addons/astrolabe/ui/loading/screen.tscn"),
        ),
        (
            "addons/astrolabe/ui/loading/vignette.gd",
            include_bytes!("../godot/addons/astrolabe/ui/loading/vignette.gd"),
        ),
        (
            "addons/astrolabe/ui/theme.tres",
            include_bytes!("../godot/addons/astrolabe/ui/theme.tres"),
        ),
        (
            "check.gd",
            include_bytes!("../godot/tests/check_loading_vignette.gd"),
        ),
    ] {
        let path = project.join(path);
        fs::create_dir_all(path.parent().context("parent")?)?;
        fs::write(path, bytes)?;
    }
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    for (name, mode) in [("parser", "--check-only"), ("runtime", "--headless")] {
        let log = project.join(format!("{name}.log"));
        let output = godot::run_godot(
            &engine,
            project,
            &[
                "--headless",
                mode,
                "--log-file",
                log.to_str().context("log path")?,
                "--script",
                "res://check.gd",
            ],
            Duration::from_secs(15),
        )?;
        let diagnostics = fs::read_to_string(log)?;
        ensure!(
            !diagnostics.contains("ERROR:"),
            "loading presenter diagnostics: {diagnostics}"
        );
        if name == "runtime" {
            ensure!(
                output.contains("LOADING_VIGNETTE_OK"),
                "missing presenter evidence: {output}"
            );
        }
    }
    Ok(())
}
