//! The Godot harness consumes frozen evidence without depending on Orrery.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use session_recording::{
    Coverage, Environment, Manifest, SaveRootKind, digest, read_events, recording_sha256,
    stage_saves, verify_saves,
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use window_video::VideoCapture;
use x11rb::{
    connection::Connection,
    protocol::xproto::{AtomEnum, ConnectionExt as _, MapState, Window},
    rust_connection::RustConnection,
};

const GODOT_WINDOW_DEADLINE: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy)]
struct GodotWindow {
    id: Window,
    width: u16,
    height: u16,
}

fn find_owned_window(
    connection: &RustConnection,
    root: Window,
    pid: u32,
) -> Result<Option<GodotWindow>> {
    let pid_atom = connection.intern_atom(false, b"_NET_WM_PID")?.reply()?.atom;
    let mut pending = vec![(root, 0_u8)];
    let mut matches = Vec::new();
    while let Some((window, depth)) = pending.pop() {
        if let Ok(tree) = connection.query_tree(window)
            && let Ok(tree) = tree.reply()
            && depth < 6
        {
            pending.extend(tree.children.into_iter().map(|child| (child, depth + 1)));
        }
        let Ok(property) =
            connection.get_property(false, window, pid_atom, AtomEnum::CARDINAL, 0, 1)
        else {
            continue;
        };
        let Ok(property) = property.reply() else {
            continue;
        };
        if property.value32().and_then(|mut value| value.next()) != Some(pid) {
            continue;
        }
        let Ok(attributes) = connection.get_window_attributes(window) else {
            continue;
        };
        let Ok(attributes) = attributes.reply() else {
            continue;
        };
        if attributes.map_state != MapState::VIEWABLE {
            continue;
        }
        let Ok(geometry) = connection.get_geometry(window) else {
            continue;
        };
        let Ok(geometry) = geometry.reply() else {
            continue;
        };
        matches.push(GodotWindow {
            id: window,
            width: geometry.width,
            height: geometry.height,
        });
    }
    Ok(matches
        .into_iter()
        .max_by_key(|window| u32::from(window.width) * u32::from(window.height)))
}

fn wait_for_owned_window(
    connection: &RustConnection,
    root: Window,
    pid: u32,
    child: &mut std::process::Child,
) -> Result<(GodotWindow, Duration)> {
    let started = Instant::now();
    let deadline = started + GODOT_WINDOW_DEADLINE;
    loop {
        if let Some(status) = child.try_wait()? {
            anyhow::bail!("Godot exited with {status} before showing a window");
        }
        if let Some(window) = find_owned_window(connection, root, pid)? {
            return Ok((window, started.elapsed()));
        }
        if Instant::now() >= deadline {
            child.kill()?;
            child.wait()?;
            anyhow::bail!(
                "Godot did not show a mapped X11 window within {} seconds",
                GODOT_WINDOW_DEADLINE.as_secs()
            );
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn owned_window_dimensions(connection: &RustConnection, window: Window) -> Result<(u16, u16)> {
    let geometry = connection.get_geometry(window)?.reply()?;
    Ok((geometry.width, geometry.height))
}

fn append_video_event(recording: &Path, elapsed_micros: u128, data: Value) -> Result<()> {
    let events_path = recording.join("events.jsonl");
    let (events, complete) = read_events(&events_path)?;
    ensure!(
        complete,
        "Godot event stream is torn before video metadata append"
    );
    let previous = events.last().context("Godot event stream is empty")?;
    ensure!(
        elapsed_micros >= previous.elapsed_micros,
        "video event timestamp precedes the closed Godot event stream"
    );
    let mut events_file = OpenOptions::new().append(true).open(events_path)?;
    writeln!(
        events_file,
        "{}",
        json!({
            "sequence": previous.sequence + 1,
            "elapsed_micros": elapsed_micros,
            "kind": "video_stream_end",
            "data": data,
        })
    )?;
    events_file.sync_all()?;
    Ok(())
}

#[derive(Parser)]
#[command(about = "Replay an OpenSpace session through the authored Godot game")]
struct Cli {
    #[arg(long)]
    recording: PathBuf,
    #[arg(long)]
    project: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value = "godot")]
    godot: PathBuf,
}

fn absolute(path: &Path) -> Result<PathBuf> {
    Ok(if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    })
}

fn write_manifest(path: &Path, manifest: &Manifest) -> Result<()> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(manifest)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn resolve_binary(path: &Path) -> Result<PathBuf> {
    if path.components().count() > 1 || path.is_absolute() {
        return Ok(path.canonicalize()?);
    }
    let search = std::env::var_os("PATH").context("PATH is unavailable")?;
    for directory in std::env::split_paths(&search) {
        let candidate = directory.join(path);
        if candidate.is_file() {
            return Ok(candidate.canonicalize()?);
        }
    }
    anyhow::bail!("Godot executable not found")
}

/// Hash the authored project and generated game content, excluding only Godot's
/// derived editor/import cache. Stable ordering makes the value independent of
/// directory enumeration order and of the host checkout location.
fn project_content_digest(project: &Path) -> Result<(String, u64)> {
    fn walk(root: &Path, dir: &Path, hash: &mut Sha256, count: &mut u64) -> Result<()> {
        let mut entries = fs::read_dir(dir)?.collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            if dir == root && entry.file_name() == ".godot" {
                continue;
            }
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)?;
            ensure!(
                !metadata.file_type().is_symlink(),
                "Godot project content contains a symlink: {}",
                path.display()
            );
            if metadata.is_dir() {
                walk(root, &path, hash, count)?;
            } else {
                ensure!(metadata.is_file(), "Godot project has a non-file entry");
                let relative = path.strip_prefix(root)?;
                let name = relative
                    .to_str()
                    .context("Godot project path is not UTF-8")?;
                hash.update((name.len() as u64).to_le_bytes());
                hash.update(name.as_bytes());
                hash.update(metadata.len().to_le_bytes());
                let mut file = File::open(&path)?;
                let mut buffer = [0u8; 65_536];
                loop {
                    let size = file.read(&mut buffer)?;
                    if size == 0 {
                        break;
                    }
                    hash.update(&buffer[..size]);
                }
                *count += 1;
            }
        }
        Ok(())
    }
    let mut hash = Sha256::new();
    let mut count = 0;
    walk(project, project, &mut hash, &mut count)?;
    Ok((format!("{:x}", hash.finalize()), count))
}

fn run(cli: Cli) -> Result<Value> {
    let source = absolute(&cli.recording)?.canonicalize()?;
    let project = absolute(&cli.project)?.canonicalize()?;
    let output = absolute(&cli.output)?;
    let godot = resolve_binary(&cli.godot)?;
    ensure!(
        project.join("project.godot").is_file(),
        "Godot project is missing"
    );
    ensure!(
        project.join("tests/session_replay.gd").is_file(),
        "session harness script is missing"
    );
    let source_manifest: Manifest =
        serde_json::from_slice(&fs::read(source.join("manifest.json"))?)?;
    ensure!(
        source_manifest.schema == session_recording::SCHEMA
            && matches!(
                source_manifest.ending.as_str(),
                "completed" | "interrupted" | "storage_budget"
            ),
        "source recording is incomplete or unsupported"
    );
    let (source_events, complete) = read_events(&source.join("events.jsonl"))?;
    ensure!(complete, "source event stream is torn");
    let source_hash = recording_sha256(&source)?;
    let source_fixture = source.join("initial-saves");
    verify_saves(
        &source_fixture,
        &session_recording::SaveFixture {
            kind: if source_manifest.initial_saves.kind == SaveRootKind::Absent {
                SaveRootKind::Empty
            } else {
                source_manifest.initial_saves.kind
            },
            directories: source_manifest.initial_saves.directories.clone(),
            files: source_manifest.initial_saves.files.clone(),
        },
    )?;
    fs::create_dir(&output)?;
    let run_recording = output.join("recording");
    fs::create_dir(&run_recording)?;
    stage_saves(
        &source_fixture,
        &source_manifest.initial_saves,
        &run_recording.join("initial-saves"),
    )?;
    if !run_recording.join("initial-saves").exists() {
        fs::create_dir(run_recording.join("initial-saves"))?;
    }
    fs::create_dir(run_recording.join("frames"))?;
    let executable_sha256 = digest(&godot)?;
    let (project_sha256, project_file_count) = project_content_digest(&project)?;
    let version = Command::new(&godot).arg("--version").output()?;
    ensure!(version.status.success(), "cannot read Godot version");
    let godot_version = String::from_utf8(version.stdout)?;
    let display = std::env::var("DISPLAY").context("Godot session replay requires X11 DISPLAY")?;
    ensure!(
        !display.trim().is_empty(),
        "Godot session replay requires X11 DISPLAY"
    );
    let (x11, screen) = x11rb::connect(Some(&display))
        .with_context(|| format!("cannot connect to Godot X11 display {display}"))?;
    let root = x11.setup().roots[screen].root;
    let mut started_unix_micros = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros();
    let mut manifest = Manifest {
        recording_mode: session_recording::RecordingMode::Full,
        schema:session_recording::SCHEMA,
        id:format!("{}-godot-{started_unix_micros}",source_manifest.id),
        title:source_manifest.title.clone(),
        description:source_manifest.description.clone(),
        tags:source_manifest.tags.clone(),
        driver:"godot_replay".into(),
        executable_sha256:executable_sha256.clone(),
        environment:Environment {
            content_tree_sha256:project_sha256,
            content_file_count:project_file_count,
            runtime_name:format!("Godot {} project harness", godot_version.trim()),
            runtime_executable_sha256:executable_sha256,
            locale:std::env::var("LANG").unwrap_or_else(|_| "unknown".into()),
            display_profile:"project viewport; measured per captured frame".into(),
            control_profile:"native Godot InputEventKey dispatch; physical device delivery not verified".into(),
            randomness_inputs:"unverified".into(),
        },
        initial_saves:source_manifest.initial_saves.clone(),
        started_unix_micros,
        elapsed_micros:0,
        ending:"in_progress".into(),
        replay_of_sha256:Some(source_hash.clone()),
        coverage:Coverage {
            input:"Godot InputEventKey physical keycode dispatch; actual device delivery unverified".into(),
            video:"timestamped, sparse viewport PNG samples; continuous video and render/update association unavailable".into(),
            state:"main menu/loading and authored manor player/Brain selected fields".into(),
            ai_api:"selected authored actor state only; API tracing unavailable".into(),
            audio:"not captured".into(),
            gaps:vec![
                "original save files retained as fixture evidence; Godot save import/mapping is not implemented".into(),
                "Godot New Game and original intro/menu structure are not implemented".into(),
                "Godot derived .godot import cache is excluded from the content hash".into(),
            ],
        },
    };
    let log = File::create_new(output.join("godot.log"))?;
    let mut command = Command::new(&godot);
    command
        .arg("--display-driver")
        .arg("x11")
        .arg("--rendering-method")
        .arg("gl_compatibility")
        .arg("--windowed")
        .arg("--resolution")
        .arg("1024x768")
        .arg("--path")
        .arg(&project)
        .arg("--script")
        .arg("res://tests/session_replay.gd")
        .arg("--")
        .arg(&source)
        .arg(&run_recording)
        .env("XDG_CONFIG_HOME", output.join("config"))
        .env("XDG_DATA_HOME", output.join("data"))
        .env("XDG_CACHE_HOME", output.join("cache"))
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log));
    // Use one launch point for video PTS mapping and the manifest's wall clock.
    let process_launch = Instant::now();
    started_unix_micros = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros();
    manifest.started_unix_micros = started_unix_micros;
    command.arg(started_unix_micros.to_string());
    write_manifest(&run_recording.join("manifest.json"), &manifest)?;
    let mut child = command.spawn()?;
    let process_id = child.id();
    let (godot_window, window_ready_after) =
        match wait_for_owned_window(&x11, root, process_id, &mut child) {
            Ok(window) => window,
            Err(error) => {
                if child.try_wait()?.is_none() {
                    child.kill()?;
                    child.wait()?;
                }
                return Err(error).with_context(|| {
                    format!(
                        "Godot window startup failed; inspect {}",
                        output.join("godot.log").display()
                    )
                });
            }
        };
    let capture_log = output.join("video-encoder.log");
    let capture = match VideoCapture::start(
        &display,
        godot_window.id,
        &run_recording,
        process_launch,
        &capture_log,
        0,
    ) {
        Ok(capture) => capture,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error).context("start continuous Godot window video capture");
        }
    };
    if (godot_window.width, godot_window.height) != (1024, 768) {
        let _ = child.kill();
        let _ = child.wait();
        anyhow::bail!(
            "Godot window opened at unexpected size {}x{}",
            godot_window.width,
            godot_window.height
        );
    }
    let mut capture = Some(capture);
    let deadline = Instant::now()
        + Duration::from_micros(
            u64::try_from(source_manifest.elapsed_micros)?.saturating_add(90_000_000),
        );
    let summary_path = run_recording.join("summary.json");
    let marker_path = run_recording.join("video-finalized");
    let mut video_finalized = false;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if !video_finalized {
            if let Err(error) = capture
                .as_mut()
                .context("video capture already finalized")?
                .bytes()
            {
                child.kill()?;
                child.wait()?;
                return Err(error).context("continuous Godot video capture stopped unexpectedly");
            }
            let dimensions = owned_window_dimensions(&x11, godot_window.id)?;
            if dimensions != (1024, 768) {
                child.kill()?;
                child.wait()?;
                anyhow::bail!(
                    "Godot window resized during capture from 1024x768 to {}x{}; recording rejected to avoid clipped video",
                    dimensions.0,
                    dimensions.1
                );
            }
        }
        if !video_finalized && summary_path.is_file() {
            // GDScript closes its event stream before writing summary.json. Wait
            // for that point before appending the controller-owned video event.
            let video = match capture
                .take()
                .context("video capture already finalized")?
                .finish()
            {
                Ok(video) => video,
                Err(error) => {
                    child.kill()?;
                    child.wait()?;
                    return Err(error).context("finalize continuous Godot video capture");
                }
            };
            ensure!(
                video.frames > 0,
                "Godot continuous video contains no indexed frames"
            );
            let video_json = serde_json::to_value(&video)?;
            append_video_event(
                &run_recording,
                process_launch.elapsed().as_micros(),
                video_json.clone(),
            )?;
            let mut summary: Value = serde_json::from_slice(&fs::read(&summary_path)?)?;
            summary["video"] = video_json.clone();
            summary["video_frames"] = json!(video.frames);
            fs::write(&summary_path, serde_json::to_vec_pretty(&summary)?)?;
            File::create(&marker_path)?.sync_all()?;
            video_finalized = true;
            // The child now observes the marker and quits; keep the deadline
            // active so a stuck script cannot orphan the test run.
        }
        if Instant::now() >= deadline {
            child.kill()?;
            child.wait()?;
            anyhow::bail!(
                "Godot session timed out; partial evidence retained at {}",
                output.display()
            );
        }
        thread::sleep(Duration::from_millis(50));
    };
    if video_finalized {
        fs::remove_file(&marker_path)?;
    }
    let summary: Value = if summary_path.is_file() {
        serde_json::from_slice(&fs::read(&summary_path)?)?
    } else {
        json!({"ending":"failed_before_summary","elapsed_micros":0})
    };
    // The script stamps session_end before waiting for video finalization.
    // Encoder drain and child shutdown are administrative time, not replay time.
    manifest.elapsed_micros = summary["elapsed_micros"]
        .as_u64()
        .filter(|elapsed| *elapsed > 0)
        .map(u128::from)
        .unwrap_or_else(|| process_launch.elapsed().as_micros());
    manifest.coverage.video = if summary["video_frames"].as_u64().unwrap_or(0) > 0 {
        "continuous 30 Hz lossless FFV1 video of the owned Godot X11 window; per-frame timestamps indexed; gameplay render/update association unverified".into()
    } else {
        manifest
            .coverage
            .gaps
            .push("continuous video capture produced no indexed frames".into());
        "continuous video capture failed; no indexed frames".into()
    };
    manifest.ending = if status.success() {
        summary["ending"].as_str().unwrap_or("failed").to_owned()
    } else {
        "godot_failed".into()
    };
    write_manifest(&run_recording.join("manifest.json"), &manifest)?;
    let (events, stream_complete) = read_events(&run_recording.join("events.jsonl"))?;
    ensure!(stream_complete, "Godot event stream is torn");
    let dispatches = events
        .iter()
        .filter(|event| event.kind == "input_dispatch")
        .collect::<Vec<_>>();
    let final_state = events
        .iter()
        .filter(|event| event.kind == "state")
        .filter_map(|event| event.data.get("state"))
        .next_back();
    let report = json!({
        "source_sha256":source_hash,
        "recording_sha256":recording_sha256(&run_recording)?,
        "source_input_events":source_events.iter().filter(|event|event.kind=="input").count(),
        "godot_input_dispatches":dispatches.len(),
        "maximum_dispatch_lateness_micros":dispatches.iter()
            .filter_map(|event|event.data["late_micros"].as_u64()).max(),
        "frames":events.iter().filter(|event|event.kind=="frame").count(),
        "video_frames":summary["video_frames"],
        "video":summary.get("video"),
        "states":events.iter().filter(|event|event.kind=="state").count(),
        "capture_gaps":events.iter().filter(|event|event.kind=="capture_gap").count(),
        "frame_gap_events":events.iter().filter(|event|event.kind=="frame_gap").count(),
        "final_state":final_state,
        "source_elapsed_micros":source_manifest.elapsed_micros,
        "godot_elapsed_micros":manifest.elapsed_micros,
        "godot_window":{
            "display":display,
            "process_id":process_id,
            "window_id":godot_window.id,
            "width":godot_window.width,
            "height":godot_window.height,
            "seconds_after_process_spawn":window_ready_after.as_secs_f64(),
        },
        "ending":manifest.ending,
        "cleanup_complete":status.success(),
        "coverage":manifest.coverage,
    });
    fs::write(
        output.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    ensure!(
        status.success(),
        "Godot replay failed; inspect {}",
        output.join("godot.log").display()
    );
    Ok(report)
}

fn main() -> Result<()> {
    let report = run(Cli::parse())?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_provenance_includes_generated_assets_but_excludes_import_cache() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let project = directory.path();
        fs::write(project.join("project.godot"), "config_version=5")?;
        fs::create_dir(project.join("generated"))?;
        fs::write(project.join("generated/level.tscn"), "first level")?;
        let (initial, count) = project_content_digest(project)?;
        assert_eq!(count, 2);

        fs::create_dir(project.join(".godot"))?;
        fs::write(project.join(".godot/cache"), "derived")?;
        assert_eq!(project_content_digest(project)?.0, initial);

        fs::write(project.join("generated/level.tscn"), "edited level")?;
        assert_ne!(project_content_digest(project)?.0, initial);
        Ok(())
    }
}
