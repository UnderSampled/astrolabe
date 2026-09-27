//! Orrery's persistent native controller for the private original-game copy.
//! The process owner, memory identity guard, private Gamescope display and XTest
//! input path are the same primitives used by the focused comparison probes.
use crate::replay::{Edge, Event, Key};
use crate::{
    config::Config,
    desktop::Desktop,
    linux::{Identity, Owner, Process},
};
use anyhow::{Context, Result, bail, ensure};
use astrolabe::openspace::level::{
    hype_runtime,
    live::{
        ActorAiObservation, ActorObservation, HypeBuild, InventoryObservation,
        MagicSelectionObservation, ReadMemory, SceneTailObservation, Status,
    },
};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    os::{fd::AsRawFd, unix::ffi::OsStrExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Running<'a>(&'a Process);
impl ReadMemory for Running<'_> {
    fn read(&mut self, address: u32, size: usize) -> Result<Vec<u8>> {
        self.0.read_running(address as usize, size)
    }
}

/// One coherent stopped-byte sample for magick behavior comparisons.
#[derive(Debug, Serialize)]
pub struct MagickBehaviorObservation {
    pub fixture: MagickFixtureProvenance,
    pub run: PathBuf,
    pub status: Status,
    pub actor: ActorAiObservation,
    pub inventory: InventoryObservation,
    pub magic_selection: MagicSelectionObservation,
    pub scene_tail: Option<SceneTailObservation>,
    pub capture: StoppedCaptureProvenance,
    pub pause: PauseProvenance,
    pub completed_update: bool,
    pub consistency_limit: &'static str,
}

#[derive(Debug, Serialize)]
pub struct MagickFixtureProvenance {
    pub executable_sha256: String,
    pub save_sha256: String,
}

#[derive(Debug, Serialize)]
pub struct StoppedCaptureProvenance {
    pub process: Identity,
    pub consistency: crate::linux::Consistency,
    pub capture_micros: u128,
    pub stopped_threads: usize,
    pub page_reads: usize,
    pub copied_source_bytes: usize,
}

#[derive(Debug, Serialize)]
pub struct PauseProvenance {
    /// Time from confirmed stop until capture completion, before SIGCONT.
    pub pause_duration_micros: u128,
    pub stopped_threads: usize,
    pub preserved_existing_stop: bool,
}

fn wait(duration: Duration) -> Result<()> {
    thread::sleep(duration);
    Ok(())
}

fn find_game(owner: &mut Owner, config: &Config, limit: Duration) -> Result<Process> {
    let deadline = Instant::now() + limit;
    loop {
        let mut found = Vec::new();
        for process in owner.processes()? {
            let pid = process.identity().pid;
            let cmd = fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
            if cmd.split(|b| *b == 0).next() != Some(config.exe.as_os_str().as_bytes()) {
                continue;
            }
            let environment = fs::read(format!("/proc/{pid}/environ")).unwrap_or_default();
            let prefix = format!("WINEPREFIX={}", config.prefix.display());
            if environment
                .split(|b| *b == 0)
                .any(|v| v == prefix.as_bytes())
                && process.read_running(0x400000, 2).is_ok_and(|b| b == b"MZ")
            {
                found.push(process);
            }
        }
        ensure!(found.len() <= 1, "ambiguous owned game processes");
        if let Some(process) = found.pop() {
            return Ok(process);
        }
        ensure!(
            Instant::now() < deadline,
            "Wine game did not become memory-readable"
        );
        thread::sleep(Duration::from_millis(50));
    }
}

fn parse_ascii(bytes: &[u8], what: &str) -> Result<String> {
    let end = bytes
        .iter()
        .position(|b| *b == 0)
        .context(format!("unterminated {what}"))?;
    ensure!(bytes[..end].is_ascii(), "non-ASCII {what}");
    Ok(std::str::from_utf8(&bytes[..end])?.to_owned())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Result<Vec<u8>> {
    ensure!(
        text.len().is_multiple_of(2),
        "hex value has an odd number of digits"
    );
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).map_err(Into::into))
        .collect()
}

pub fn display_bridge(output: &Path) -> Result<()> {
    use std::io::Write;
    let value = json!({
        "DISPLAY": std::env::var("DISPLAY").ok(),
        "XAUTHORITY": std::env::var("XAUTHORITY").ok(),
    });
    let mut file = File::create_new(output)?;
    serde_json::to_writer(&mut file, &value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

pub struct GameSession {
    owner: Owner,
    process: Process,
    desktop: Desktop,
    config: Config,
    run: PathBuf,
    started: Instant,
    started_unix_micros: u128,
    expires: Instant,
    _installation_lock: File,
}

impl GameSession {
    pub fn start(config_path: &Path, runs: &Path, duration: Duration) -> Result<Self> {
        Self::start_with_bridge(config_path, runs, duration, &std::env::current_exe()?)
    }

    pub fn start_with_bridge(
        config_path: &Path,
        runs: &Path,
        duration: Duration,
        bridge_binary: &Path,
    ) -> Result<Self> {
        Self::start_inner(config_path, runs, duration, bridge_binary, true, true)
    }

    /// Start a game from a private per-run installation, returning as soon as
    /// the process is identified so menu/startup capture can begin.
    pub fn start_session(config_path: &Path, runs: &Path, duration: Duration) -> Result<Self> {
        Self::start_inner(
            config_path,
            runs,
            duration,
            &std::env::current_exe()?,
            false,
            false,
        )
    }

    fn start_inner(
        config_path: &Path,
        runs: &Path,
        duration: Duration,
        bridge_binary: &Path,
        pinned_save: bool,
        wait_menu: bool,
    ) -> Result<Self> {
        ensure!(
            (60..=43_200).contains(&duration.as_secs()),
            "duration outside 60..43200 seconds"
        );
        let config_path = config_path.canonicalize()?;
        let directory = config_path.parent().context("configuration parent")?;
        let lock = File::open(directory.join("lock"))?;
        // SAFETY: flock operates only on this private installation lock file.
        ensure!(
            unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0,
            "prepared original-game installation is already in use"
        );
        let config = if pinned_save {
            Config::load(&config_path)?
        } else {
            Config::load_base(&config_path)?
        };
        for entry in fs::read_dir("/proc")? {
            let entry = entry?;
            if entry.file_name().as_bytes().iter().all(u8::is_ascii_digit) {
                let cmd = fs::read(entry.path().join("cmdline")).unwrap_or_default();
                ensure!(
                    cmd.split(|b| *b == 0).next() != Some(config.exe.as_os_str().as_bytes()),
                    "private game executable is already running outside Orrery ownership"
                );
            }
        }
        fs::create_dir_all(runs)?;
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let run = runs.join(format!("run-{stamp}"));
        fs::create_dir(&run)?;
        let handoff = run.join("display.json");
        let game_log = File::create_new(run.join("gamescope.log"))?;
        let host_display = std::env::var("DISPLAY").unwrap_or_default();
        let mut owner = Owner::new()?;
        let mut gamescope = Command::new("gamescope");
        gamescope
            .args([
                "--backend",
                "sdl",
                "-w",
                "1040",
                "-h",
                "800",
                "-W",
                "1040",
                "-H",
                "800",
                "-r",
                "60",
                "-o",
                "60",
                "--",
            ])
            .arg(bridge_binary)
            .arg("display-bridge")
            .arg("--output")
            .arg(&handoff)
            .env("SDL_VIDEODRIVER", "wayland")
            .env("SDL_VIDEO_WAYLAND_WMCLASS", "orrery")
            .env("SDL_APP_ID", "orrery")
            .env("SDL_WINDOW_NO_ACTIVATION_WHEN_SHOWN", "1")
            .env_remove("XDG_ACTIVATION_TOKEN")
            .env_remove("DESKTOP_STARTUP_ID")
            .stdout(Stdio::from(game_log.try_clone()?))
            .stderr(Stdio::from(game_log));
        let display_process = owner.spawn(&mut gamescope)?;
        let deadline = Instant::now() + Duration::from_secs(15);
        let guest: Value = loop {
            ensure!(
                display_process.check().is_ok(),
                "Gamescope exited during startup"
            );
            if let Ok(bytes) = fs::read(&handoff)
                && let Ok(value) = serde_json::from_slice(&bytes)
            {
                break value;
            }
            ensure!(
                Instant::now() < deadline,
                "Gamescope display startup timed out"
            );
            thread::sleep(Duration::from_millis(50));
        };
        let display = guest["DISPLAY"]
            .as_str()
            .context("nested DISPLAY missing")?
            .to_owned();
        ensure!(
            !display.is_empty() && display != host_display,
            "refusing host display"
        );
        let wine_log = File::create_new(run.join("wine.log"))?;
        let mut wine = Command::new(&config.wine);
        wine.arg(&config.exe)
            .current_dir(&config.game)
            .envs(config.environment()?)
            .env("DISPLAY", &display)
            .stdout(Stdio::from(wine_log.try_clone()?))
            .stderr(Stdio::from(wine_log));
        if let Some(authority) = guest["XAUTHORITY"].as_str() {
            wine.env("XAUTHORITY", authority);
        }
        let started = Instant::now();
        let started_unix_micros = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros();
        owner.spawn(&mut wine)?;
        let process = find_game(&mut owner, &config, Duration::from_secs(40))?;
        let image = fs::read(&config.exe)?;
        ensure!(
            process.read_running(0x400000, 512)? == image[..512],
            "live image header differs from pinned executable"
        );
        let build = HypeBuild::verified(&config.exe_sha256)?;
        if wait_menu {
            let ready = Instant::now() + Duration::from_secs(40);
            loop {
                if build
                    .status(&mut Running(&process))
                    .is_ok_and(|s| s.phase == 38)
                {
                    break;
                }
                ensure!(Instant::now() < ready, "game did not reach the main menu");
                thread::sleep(Duration::from_millis(100));
            }
        }
        let desktop =
            Desktop::connect_display(&display, &host_display, process.identity().pid as u32)?;
        if wait_menu {
            thread::sleep(Duration::from_millis(500));
            desktop.focus()?;
            thread::sleep(Duration::from_millis(800));
        }
        Ok(Self {
            owner,
            process,
            desktop,
            config,
            run,
            started,
            started_unix_micros,
            expires: Instant::now() + duration,
            _installation_lock: lock,
        })
    }

    fn check(&self) -> Result<()> {
        ensure!(
            Instant::now() < self.expires,
            "game session lifetime expired"
        );
        self.process.check()
    }

    pub fn status(&self) -> Result<Value> {
        self.check()?;
        Ok(
            json!({"pid":self.process.identity().pid,"starttime":self.process.identity().start_ticks,
            "run":self.run,"display":self.desktop.display,"isolated_keyboard":true,
            "backend":"gamescope-sdl-wayland","exe_sha256":self.config.exe_sha256,
            "remaining_seconds":self.expires.saturating_duration_since(Instant::now()).as_secs()}),
        )
    }

    pub fn state(&self) -> Result<Value> {
        self.check()?;
        let bytes = self
            .process
            .read_running(hype_runtime::STATUS as usize, 261)?;
        let pending = self.process.read_running(0x754525, 260)?;
        Ok(
            json!({"phase":bytes[0],"level":parse_ascii(&bytes[1..], "level")?,
            "pending_level":parse_ascii(&pending, "pending level")?}),
        )
    }

    /// Raw running clock sample; the four reads are not an atomic engine update.
    pub fn timing(&self) -> Result<Value> {
        self.check()?;
        let word = |address: u32| -> Result<u32> {
            let bytes = self.process.read_running(address as usize, 4)?;
            Ok(u32::from_le_bytes(bytes.as_slice().try_into()?))
        };
        Ok(json!({
            "update_counter":word(hype_runtime::UPDATE_COUNTER)?,
            "clock_ms":word(hype_runtime::CLOCK_MS)?,
            "raw_delta_ms":word(hype_runtime::RAW_DELTA_MS)?,
            "clamped_delta_ms":word(hype_runtime::CLAMPED_DELTA_MS)?,
            "consistency":"running_non_atomic",
        }))
    }

    pub fn mana(&self) -> Result<Value> {
        self.check()?;
        let pointer =
            u32::from_le_bytes(self.process.read_running(0x610b98, 4)?.try_into().unwrap())
                as usize;
        let bytes = self.process.read_running(pointer + 0x24, 5)?;
        let current = u16::from_le_bytes(bytes[0..2].try_into().unwrap());
        let maximum = u16::from_le_bytes(bytes[2..4].try_into().unwrap());
        Ok(
            json!({"address":format!("0x{:x}",pointer+0x24),"current":current,"maximum":maximum,
            "selected_spell":bytes[4],"fraction":if maximum == 0 { None } else { Some(current as f64 / maximum as f64) }}),
        )
    }

    /// Read a bounded, typed main-actor projection from one stopped-byte capture.
    /// This is coherent while captured, but is not evidence that an engine update
    /// completed at a particular frame boundary.
    pub fn inspect_main_actor(
        &self,
        actions: &[u32],
        designer_values: Option<&[u32]>,
    ) -> Result<Value> {
        self.inspect_main_actor_with_camera(actions, designer_values, false)
    }

    /// As `inspect_main_actor`, with the active viewport camera sampled in the
    /// same stopped-byte capture when requested.
    pub fn inspect_main_actor_with_camera(
        &self,
        actions: &[u32],
        designer_values: Option<&[u32]>,
        include_camera: bool,
    ) -> Result<Value> {
        ensure!(
            (1..=64).contains(&actions.len()),
            "inspect_main_actor requires 1..64 action ids"
        );
        ensure!(
            actions
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == actions.len(),
            "inspect_main_actor action ids must be unique"
        );
        if let Some(designer_values) = designer_values {
            ensure!(
                designer_values.len() <= 64,
                "inspect_main_actor requires at most 64 designer value ids"
            );
            ensure!(
                designer_values
                    .iter()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    == designer_values.len(),
                "inspect_main_actor designer value ids must be unique"
            );
        }
        self.check()?;
        let build = HypeBuild::verified(&self.config.exe_sha256)?;
        let pause = self.process.pause(Duration::from_secs(1))?;
        let mut capture = pause.capture()?;
        let status = build.status(&mut capture)?;
        let observation = if let Some(designer_values) = designer_values {
            serde_json::to_value(build.actor_with_ai(&mut capture, actions, designer_values)?)?
        } else {
            // Preserve the original tool's action-only response shape when the
            // opt-in AI field is absent.
            serde_json::to_value(build.actor_with_actions(&mut capture, actions)?)?
        };
        let camera = if include_camera {
            Some(serde_json::to_value(build.active_camera(&mut capture)?)?)
        } else {
            None
        };
        let snapshot = capture.finish()?;
        drop(pause);
        let mut result = json!({
            "fixture": {
                "executable_sha256": self.config.exe_sha256,
                "save_sha256": self.config.save_sha256,
            },
            "run": self.run,
            "status": status,
            "capture": snapshot,
            "completed_update": false,
            "consistency_limit": "stopped-byte capture is coherent during this read but has no completed-update publication evidence",
            "observation": observation,
        });
        if let Some(camera) = camera {
            result["active_camera"] = camera;
        }
        Ok(result)
    }

    /// Observe Hype's selected AI fields, inventory, magic selection and laser
    /// in one stopped capture so a short combat phase cannot change between reads.
    pub fn inspect_combat(&self, actions: &[u32], designer_values: &[u32]) -> Result<Value> {
        ensure!(
            (1..=64).contains(&actions.len()),
            "combat inspection needs 1..64 actions"
        );
        ensure!(
            designer_values.len() <= 64,
            "combat inspection needs at most 64 designer values"
        );
        ensure!(
            actions
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == actions.len(),
            "duplicate combat action"
        );
        ensure!(
            designer_values
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == designer_values.len(),
            "duplicate combat designer value"
        );
        self.check()?;
        let build = HypeBuild::verified(&self.config.exe_sha256)?;
        let pause = self.process.pause(Duration::from_secs(1))?;
        let mut capture = pause.capture()?;
        let status = build.status(&mut capture)?;
        let actor = build.actor_with_ai(&mut capture, actions, designer_values)?;
        let inventory = build.inventory(&mut capture)?;
        let magic_selection = build.magic_selection(&mut capture, &inventory)?;
        let laser_pointer = build.laser_pointer(&mut capture)?;
        let snapshot = capture.finish()?;
        drop(pause);
        Ok(json!({
            "fixture": {
                "executable_sha256": self.config.exe_sha256,
                "save_sha256": self.config.save_sha256,
            },
            "run": self.run,
            "status": status,
            "capture": snapshot,
            "completed_update": false,
            "consistency_limit": "stopped-byte capture is coherent during this read but has no completed-update publication evidence",
            "actor": actor,
            "inventory": inventory,
            "magic_selection": magic_selection,
            "laser_pointer": laser_pointer,
        }))
    }

    /// Capture the actor AI, selected inventory-derived spell and an optional
    /// short DynamicWorld tail while the original process is stopped once.
    /// This is source memory evidence, not a completed-update observation.
    pub fn observe_magick_behavior(
        &self,
        actions: &[u32],
        designer_values: &[u32],
        scene_tail_limit: Option<usize>,
    ) -> Result<MagickBehaviorObservation> {
        ensure!(
            (1..=64).contains(&actions.len()),
            "magick observation needs 1..64 actions"
        );
        ensure!(
            designer_values.len() <= 64,
            "magick observation needs at most 64 designer values"
        );
        ensure!(
            actions
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == actions.len(),
            "duplicate magick observation action"
        );
        ensure!(
            designer_values
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == designer_values.len(),
            "duplicate magick observation designer value"
        );
        if let Some(limit) = scene_tail_limit {
            ensure!(
                (1..=4).contains(&limit),
                "magick scene tail limit outside 1..4"
            );
        }

        self.check()?;
        let build = HypeBuild::verified(&self.config.exe_sha256)?;
        let pause_guard = self.process.pause(Duration::from_secs(1))?;
        let pause_started = Instant::now();
        let stopped_threads = pause_guard.stopped_threads();
        let preserved_existing_stop = pause_guard.preserved_existing_stop();
        let mut capture = pause_guard.capture()?;
        let status = build.status(&mut capture)?;
        ensure!(
            status.phase == 9 && !status.level.is_empty(),
            "magick observation requires loaded gameplay"
        );
        let actor = build.actor_with_ai(&mut capture, actions, designer_values)?;
        let inventory = build.inventory(&mut capture)?;
        let magic_selection = build.magic_selection(&mut capture, &inventory)?;
        let scene_tail = scene_tail_limit
            .map(|limit| build.scene_tail(&mut capture, limit))
            .transpose()?;
        let snapshot = capture.finish()?;
        let pause_duration_micros = pause_started.elapsed().as_micros();
        let pause_provenance = PauseProvenance {
            pause_duration_micros,
            stopped_threads,
            preserved_existing_stop,
        };
        let capture = StoppedCaptureProvenance {
            process: snapshot.process,
            consistency: snapshot.consistency,
            capture_micros: snapshot.capture_micros,
            stopped_threads: snapshot.stopped_threads,
            page_reads: snapshot.page_reads,
            copied_source_bytes: snapshot.copied_source_bytes,
        };
        drop(pause_guard);

        Ok(MagickBehaviorObservation {
            fixture: MagickFixtureProvenance {
                executable_sha256: self.config.exe_sha256.clone(),
                save_sha256: self.config.save_sha256.clone(),
            },
            run: self.run.clone(),
            status,
            actor,
            inventory,
            magic_selection,
            scene_tail,
            capture,
            pause: pause_provenance,
            completed_update: false,
            consistency_limit: "one coherent stopped-byte capture; no completed-update publication evidence",
        })
    }

    /// List a bounded tail of DynamicWorld children in one stopped capture.
    /// The returned transform can track a generated Perso while it lives.
    pub fn inspect_scene_tail(&self, limit: usize) -> Result<Value> {
        self.check()?;
        let build = HypeBuild::verified(&self.config.exe_sha256)?;
        let pause = self.process.pause(Duration::from_secs(1))?;
        let mut capture = pause.capture()?;
        let status = build.status(&mut capture)?;
        let scene = build.scene_tail(&mut capture, limit)?;
        let snapshot = capture.finish()?;
        drop(pause);
        Ok(json!({
            "fixture": {
                "executable_sha256": self.config.exe_sha256,
                "save_sha256": self.config.save_sha256,
            },
            "run": self.run,
            "status": status,
            "capture": snapshot,
            "completed_update": false,
            "consistency_limit": "stopped-byte capture is coherent during this read but has no completed-update publication evidence",
            "scene": scene,
        }))
    }

    /// Inspect a source-rooted Perso during one stopped capture. The path is
    /// resolved afresh, then checked against its source object-type identity.
    pub fn inspect_scene_actor(
        &self,
        level: &str,
        child_path: &[u32],
        object_types: [u32; 3],
        actions: &[u32],
        designer_values: &[u32],
    ) -> Result<Value> {
        self.inspect_scene_actor_with_camera(
            level,
            child_path,
            object_types,
            actions,
            designer_values,
            false,
        )
    }

    /// Selected actor plus its source camera records and active viewport pose
    /// from one stopped-byte capture when requested.
    pub fn inspect_scene_actor_with_camera(
        &self,
        level: &str,
        child_path: &[u32],
        object_types: [u32; 3],
        actions: &[u32],
        designer_values: &[u32],
        include_camera_records: bool,
    ) -> Result<Value> {
        ensure!(
            actions.len() <= 64 && (include_camera_records || !actions.is_empty()),
            "scene actor requires {}..64 actions",
            if include_camera_records { 0 } else { 1 }
        );
        ensure!(
            designer_values.len() <= 64,
            "scene actor requires at most 64 designer values"
        );
        self.check()?;
        let build = HypeBuild::verified(&self.config.exe_sha256)?;
        let pause = self.process.pause(Duration::from_secs(1))?;
        let mut capture = pause.capture()?;
        let status = build.status(&mut capture)?;
        let observation = build.scene_actor_with_ai(
            &mut capture,
            level,
            child_path,
            object_types,
            actions,
            designer_values,
        )?;
        let cameras = if include_camera_records {
            Some((
                serde_json::to_value(build.scene_camera_records(
                    &mut capture,
                    level,
                    child_path,
                    object_types,
                )?)?,
                serde_json::to_value(build.active_camera(&mut capture)?)?,
            ))
        } else {
            None
        };
        let snapshot = capture.finish()?;
        drop(pause);
        let mut result = json!({
            "fixture": {
                "executable_sha256": self.config.exe_sha256,
                "save_sha256": self.config.save_sha256,
            },
            "run": self.run,
            "status": status,
            "capture": snapshot,
            "completed_update": false,
            "consistency_limit": "stopped-byte capture is coherent during this read but has no completed-update publication evidence",
            "selector": {"level": level, "child_path": child_path, "object_types": object_types},
            "observation": observation,
        });
        if let Some((source_camera, active_camera)) = cameras {
            result["source_camera_records"] = source_camera;
            result["active_camera"] = active_camera;
        }
        Ok(result)
    }

    /// Search the live ActualWorld tree for a unique camera-bearing actor and
    /// capture its Brain, linked records and active viewport in one pause.
    pub fn inspect_scene_camera(
        &self,
        level: &str,
        object_types: [u32; 3],
        designer_values: &[u32],
    ) -> Result<Value> {
        ensure!(
            designer_values.len() <= 64,
            "scene camera requires at most 64 designer values"
        );
        self.check()?;
        let build = HypeBuild::verified(&self.config.exe_sha256)?;
        let pause = self.process.pause(Duration::from_secs(1))?;
        let mut capture = pause.capture()?;
        let status = build.status(&mut capture)?;
        let observation = build.scene_camera(&mut capture, level, object_types, designer_values)?;
        let snapshot = capture.finish()?;
        drop(pause);
        Ok(json!({
            "fixture": {
                "executable_sha256": self.config.exe_sha256,
                "save_sha256": self.config.save_sha256,
            },
            "run": self.run,
            "status": status,
            "capture": snapshot,
            "completed_update": false,
            "consistency_limit": "stopped-byte capture is coherent during this read but has no completed-update publication evidence",
            "selector": {"level": level, "object_types": object_types, "camera_link_required": true},
            "observation": observation,
        }))
    }

    /// A typed functional observation. Each call is one coherent stopped-byte
    /// capture; it does not claim a completed gameplay update.
    pub fn observe_main_actor(&self) -> Result<ActorObservation> {
        self.observe_main_actor_timed().map(|(actor, _)| actor)
    }

    /// Include the interval spent with the source process suspended so a
    /// behavior recording can distinguish game-running time from wall time.
    pub fn observe_main_actor_timed(&self) -> Result<(ActorObservation, Duration)> {
        self.check()?;
        let build = HypeBuild::verified(&self.config.exe_sha256)?;
        let pause = self.process.pause(Duration::from_secs(1))?;
        let paused_at = Instant::now();
        let mut capture = pause.capture()?;
        let actor = build.actor_with_actions(&mut capture, &[47])?;
        capture.finish()?;
        drop(pause);
        Ok((actor, paused_at.elapsed()))
    }

    /// A coherent selected-actor AI snapshot. It can reveal the current
    /// normal/reflex choice but cannot establish transitions between samples.
    pub fn observe_main_actor_with_ai_timed(&self) -> Result<(ActorAiObservation, Duration)> {
        self.check()?;
        let build = HypeBuild::verified(&self.config.exe_sha256)?;
        let pause = self.process.pause(Duration::from_secs(1))?;
        let paused_at = Instant::now();
        let mut capture = pause.capture()?;
        let actor = build.actor_with_ai(&mut capture, &[47], &[])?;
        capture.finish()?;
        drop(pause);
        Ok((actor, paused_at.elapsed()))
    }

    /// Focus once before a functional input schedule. Gameplay events use the
    /// same guarded private-display XTest path as precision replay.
    pub fn prepare_physical_input(&mut self) -> Result<()> {
        self.check()?;
        self.desktop.focus()
    }

    pub fn physical_events(&mut self, events: &[Event]) -> Result<()> {
        self.check()?;
        self.desktop.physical_events(events)
    }

    /// Send timed physical taps without reading game state or pausing its
    /// process. This is safe to use while an external debugger owns stops.
    pub fn physical_key_sequence(&mut self, steps: &[(Key, Duration, Duration)]) -> Result<Value> {
        ensure!(
            (1..=64).contains(&steps.len()),
            "key sequence must contain 1..64 steps"
        );
        let mut scheduled_ms = 0_u128;
        for (_, hold, gap) in steps {
            ensure!(
                (20..=10_000).contains(&hold.as_millis()),
                "hold outside 20..10000 ms"
            );
            ensure!(gap.as_millis() <= 2_000, "gap outside 0..2000 ms");
            scheduled_ms += hold.as_millis() + gap.as_millis();
        }
        ensure!(scheduled_ms <= 25_000, "key sequence exceeds 25 seconds");

        self.check()?;
        self.desktop.focus()?;
        wait(Duration::from_millis(200))?;
        let result = (|| -> Result<()> {
            for (key, hold, gap) in steps {
                self.physical_events(&[Event {
                    keyboard: 0,
                    key: *key,
                    edge: Edge::Press,
                }])?;
                let held = wait(*hold);
                let released = self.physical_events(&[Event {
                    keyboard: 0,
                    key: *key,
                    edge: Edge::Release,
                }]);
                held?;
                released?;
                wait(*gap)?;
            }
            Ok(())
        })();
        let released = self.desktop.release();
        result?;
        released?;
        let reported_steps = steps
            .iter()
            .map(|(key, hold, gap)| {
                json!({
                    "key": key,
                    "hold_ms": hold.as_millis(),
                    "gap_ms": gap.as_millis()
                })
            })
            .collect::<Vec<_>>();
        Ok(json!({
            "steps": reported_steps,
            "pause_free": true,
            "released": true
        }))
    }

    pub fn keys(&mut self, keys: &[String], hold_ms: u64, gap_ms: u64) -> Result<Value> {
        self.check()?;
        let before = (self.state()?["phase"] == 9)
            .then(|| self.mana())
            .transpose()?;
        self.desktop.timed_keys(
            keys,
            Duration::from_millis(hold_ms),
            Duration::from_millis(gap_ms),
            wait,
        )?;
        let after = before.as_ref().map(|_| self.mana()).transpose()?;
        Ok(
            json!({"keys":keys,"hold_ms":hold_ms,"gap_ms":gap_ms,"mana_before":before,"mana_after":after}),
        )
    }

    pub fn bindings(&self) -> Result<Vec<Value>> {
        self.check()?;
        let mut letters: BTreeMap<u16, String> = BTreeMap::new();
        for (scan, value) in (2_u16..12).zip("1234567890".chars()) {
            letters.insert(scan, value.to_string());
        }
        for (scan, value) in (16_u16..26).zip("qwertyuiop".chars()) {
            letters.insert(scan, value.to_string());
        }
        for (scan, value) in (30_u16..39).zip("asdfghjkl".chars()) {
            letters.insert(scan, value.to_string());
        }
        for (scan, value) in (44_u16..51).zip("zxcvbnm".chars()) {
            letters.insert(scan, value.to_string());
        }
        letters.insert(39, "semicolon".into());
        if self.process.read_running(0x87f25e, 1)? != [1] {
            let pairs = self.process.read_running(0x5ee1b8, 36)?;
            let original = letters.clone();
            for pair in pairs.as_chunks::<4>().0 {
                let source = i16::from_le_bytes(pair[..2].try_into().unwrap());
                let target = i16::from_le_bytes(pair[2..].try_into().unwrap());
                if source == -1 {
                    break;
                }
                if let Some(value) = original.get(&(source as u16)) {
                    letters.insert(target as u16, value.clone());
                }
            }
        }
        let root = self.process.read_running(0x87f268, 8)?;
        let count = u32::from_le_bytes(root[..4].try_into().unwrap()) as usize;
        let pointer = u32::from_le_bytes(root[4..].try_into().unwrap()) as usize;
        ensure!((1..4096).contains(&count), "invalid action table count");
        let table = self.process.read_running(pointer, count * 28)?;
        let mut result = Vec::new();
        for (index, row) in table.as_chunks::<28>().0.iter().enumerate() {
            let word = |at| u32::from_le_bytes(row[at..at + 4].try_into().unwrap());
            let (n, words, name) = (word(0) as usize, word(4) as usize, word(8) as usize);
            if !(1..=256).contains(&n) || name == 0 {
                continue;
            }
            let data = self.process.read_running(words, n * 8)?;
            let values = data
                .as_chunks::<8>()
                .0
                .iter()
                .map(|r| u32::from_le_bytes(r[..4].try_into().unwrap()) as u16)
                .collect::<Vec<_>>();
            if values.first() == Some(&0xf021) && n == 2 + 2 * values[1] as usize {
                let keys = values[3..]
                    .iter()
                    .step_by(2)
                    .rev()
                    .map(|k| {
                        letters
                            .get(k)
                            .cloned()
                            .unwrap_or_else(|| format!("<scan:{k}>"))
                    })
                    .collect::<Vec<_>>();
                let action = parse_ascii(&self.process.read_running(name, 128)?, "binding action")?;
                result.push(json!({"action":action,"code":keys.join(""),"keys":keys,"address":format!("0x{:x}",pointer+index*28)}));
            }
        }
        Ok(result)
    }

    pub fn cheat(&mut self, text: &str, hold_ms: u64, gap_ms: u64) -> Result<Value> {
        ensure!(
            (1..=64).contains(&text.len())
                && text
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()),
            "cheat must contain 1..64 lowercase ASCII letters/digits"
        );
        let matches = self
            .bindings()?
            .into_iter()
            .filter(|b| b["code"] == text)
            .collect::<Vec<_>>();
        ensure!(
            matches.len() == 1,
            "no unique live binding; inspect bindings"
        );
        let before = self.state()?;
        let result = self.keys(
            &text.chars().map(|c| c.to_string()).collect::<Vec<_>>(),
            hold_ms,
            gap_ms,
        )?;
        Ok(
            json!({"binding":matches[0],"recognized":true,"input":result,"before":before,"after":self.state()?}),
        )
    }

    pub fn screenshot(&mut self) -> Result<Value> {
        self.check()?;
        self.desktop.focus()?;
        let path = self.run.join(format!(
            "screen-{}.png",
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));
        Ok(serde_json::to_value(
            self.desktop.screenshot(&path, self.started)?,
        )?)
    }

    pub fn read_memory(&self, address: usize, kind: &str, size: usize) -> Result<Value> {
        self.check()?;
        let size = match kind {
            "u8" => 1,
            "u16" => 2,
            "u32" | "i32" | "f32" => 4,
            "hex" => size,
            _ => bail!("unsupported memory type"),
        };
        ensure!(
            (1..=65_536).contains(&size),
            "MCP read size outside 1..65536"
        );
        let bytes = self.process.read_running(address, size)?;
        let value = match kind {
            "u8" => json!(bytes[0]),
            "u16" => json!(u16::from_le_bytes(bytes.as_slice().try_into().unwrap())),
            "u32" => json!(u32::from_le_bytes(bytes.as_slice().try_into().unwrap())),
            "i32" => json!(i32::from_le_bytes(bytes.as_slice().try_into().unwrap())),
            "f32" => json!(f32::from_le_bytes(bytes.as_slice().try_into().unwrap())),
            "hex" => json!(hex(&bytes)),
            _ => unreachable!(),
        };
        Ok(json!({"address":format!("0x{address:x}"),"type":kind,"value":value,"hex":hex(&bytes)}))
    }

    fn packed(kind: &str, text: &str) -> Result<Vec<u8>> {
        Ok(match kind {
            "hex" => unhex(text)?,
            "u8" => vec![text.parse::<u8>()?],
            "u16" => text.parse::<u16>()?.to_le_bytes().to_vec(),
            "u32" => text.parse::<u32>()?.to_le_bytes().to_vec(),
            "i32" => text.parse::<i32>()?.to_le_bytes().to_vec(),
            "f32" => text.parse::<f32>()?.to_le_bytes().to_vec(),
            _ => bail!("unsupported memory type"),
        })
    }

    pub fn write_memory(
        &self,
        address: usize,
        kind: &str,
        value: &str,
        expected: &str,
    ) -> Result<Value> {
        self.check()?;
        let value = Self::packed(kind, value)?;
        let expected = Self::packed(kind, expected)?;
        self.process
            .pause(Duration::from_secs(1))?
            .guarded_write(address, &value, &expected)
    }

    /// Declare a scenario starting state without changing the save fixture.
    /// The source inventory list is resolved afresh in this stopped process;
    /// no heap address is accepted from a test or reused across runs.
    pub fn prepare_crossbow_quantity(&self) -> Result<Value> {
        self.check()?;
        let build = HypeBuild::verified(&self.config.exe_sha256)?;
        let pause = self.process.pause(Duration::from_secs(1))?;
        let mut capture = pause.capture()?;
        let status = build.status(&mut capture)?;
        ensure!(
            status.phase == 9 && status.level == "manoir",
            "crossbow quantity setup requires loaded manor gameplay"
        );
        let before = build.inventory(&mut capture)?;
        ensure!(
            before.selected_weapon == 80,
            "supplied save has a different selected weapon"
        );
        let address = build.inventory_quantity_address(&mut capture, "Weapon", 80, 0)?;
        let before_capture = capture.finish()?;
        let write = pause.guarded_write(
            usize::try_from(address)?,
            &3_i16.to_le_bytes(),
            &0_i16.to_le_bytes(),
        )?;
        let mut capture = pause.capture()?;
        let after = build.inventory(&mut capture)?;
        ensure!(
            after.selected_weapon == 80
                && after
                    .active
                    .iter()
                    .filter(|item| item.kind == "Weapon" && item.id == 80)
                    .count()
                    == 1
                && after
                    .active
                    .iter()
                    .any(|item| item.kind == "Weapon" && item.id == 80 && item.quantity == 3),
            "crossbow quantity did not reach the declared value"
        );
        let after_capture = capture.finish()?;
        drop(pause);
        Ok(json!({
            "method": "guarded_current_process_inventory_starting_state",
            "fixture": {
                "executable_sha256": self.config.exe_sha256,
                "save_sha256": self.config.save_sha256,
            },
            "status": status,
            "item_kind": "Weapon",
            "item_id": 80,
            "expected_quantity": 0,
            "declared_quantity": 3,
            "before": before,
            "after": after,
            "guarded_write": write,
            "before_capture": before_capture,
            "after_capture": after_capture,
            "save_files_modified": false,
        }))
    }

    pub fn memory_probe(&self) -> Result<Value> {
        self.check()?;
        let address = 0x754525;
        let value = b"orreryprobe\0";
        let pause = self.process.pause(Duration::from_secs(1))?;
        let original = self.process.read_running(address, value.len())?;
        let write = pause.guarded_write(address, value, &original)?;
        let rejected = pause
            .guarded_write(address, &original, &vec![b'!'; value.len()])
            .is_err();
        let restore = pause.guarded_write(address, &original, value)?;
        ensure!(rejected, "incorrect expected value was accepted");
        Ok(json!({"write":write,"restore":restore,"guard_rejected":true}))
    }

    pub fn navigate_level(&self, name: &str) -> Result<Value> {
        ensure!(
            (1..=30).contains(&name.len())
                && name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
            "invalid level name"
        );
        let levels = self.config.game.join("GameData/World/Levels");
        let matches = fs::read_dir(&levels)?
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name().to_string_lossy().eq_ignore_ascii_case(name) && e.path().is_dir()
            })
            .collect::<Vec<_>>();
        ensure!(
            matches.len() == 1
                && fs::read_dir(matches[0].path())?
                    .filter_map(Result::ok)
                    .any(|e| e
                        .path()
                        .extension()
                        .is_some_and(|x| x.eq_ignore_ascii_case("sna"))),
            "level is not installed"
        );
        let before = self.state()?;
        ensure!(
            before["phase"] == 9,
            "level navigation requires active gameplay phase 9"
        );
        ensure!(before["level"] != name, "already in that level");
        let mut padded = vec![0; 260];
        padded[..name.len()].copy_from_slice(name.as_bytes());
        let targets = [
            (0x754525, padded),
            (0x754a39, vec![0]),
            (0x754a6b, vec![0]),
            (0x754420, vec![6]),
        ];
        let pause = self.process.pause(Duration::from_secs(1))?;
        let originals = targets
            .iter()
            .map(|(a, v)| self.process.read_running(*a, v.len()))
            .collect::<Result<Vec<_>>>()?;
        for ((address, value), old) in targets.iter().zip(&originals) {
            pause.guarded_write(*address, value, old)?;
        }
        drop(pause);
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let after = self.state()?;
            if after["phase"] == 9 && after["level"] == name {
                return Ok(
                    json!({"method":"guarded memory transition","before":before,"after":after}),
                );
            }
            ensure!(
                Instant::now() < deadline,
                "level transition timeout: {after}"
            );
            thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn memory_maps(&self) -> Result<Vec<Value>> {
        self.check()?;
        self.process.memory_maps()
    }
    pub fn release(&mut self) -> Result<()> {
        self.desktop.release()
    }
    pub fn identity(&self) -> Identity {
        self.process.identity()
    }
    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }
    pub fn started_instant(&self) -> Instant {
        self.started
    }
    pub fn started_unix_micros(&self) -> u128 {
        self.started_unix_micros
    }
    pub fn run_directory(&self) -> &Path {
        &self.run
    }
    pub fn display(&self) -> &str {
        &self.desktop.display
    }
    pub fn window_id(&self) -> Result<u32> {
        self.desktop.window_id()
    }
    pub fn window_dimensions(&self, window: u32) -> Result<(u16, u16)> {
        self.desktop.window_dimensions(window)
    }
    pub fn keysym_for_code(&self, code: u8) -> Result<u32> {
        self.desktop.keysym_for_code(code)
    }
    pub fn keycode_for(&self, key: Key) -> Result<u8> {
        self.desktop.keycode_for(key)
    }
    pub fn physical_keycode(&mut self, code: u8, edge: Edge, repeat: bool) -> Result<()> {
        self.check()?;
        self.desktop.physical_keycode(code, edge, repeat)
    }
    pub fn capture_frame(&self, path: &Path) -> Result<crate::desktop::ScreenCapture> {
        self.check()?;
        self.desktop.screenshot(path, self.started)
    }
    pub fn save_root(&self) -> PathBuf {
        self.config.game.join("GameData/SaveGame")
    }
    pub fn close(mut self) -> Result<()> {
        let release = self.desktop.release();
        let close = self.owner.close();
        release?;
        close
    }
}

pub struct NativeController {
    config: PathBuf,
    runs: PathBuf,
    bridge_binary: Option<PathBuf>,
    game: Option<GameSession>,
}

impl NativeController {
    pub fn new(config: PathBuf, runs: PathBuf) -> Self {
        Self {
            config,
            runs,
            bridge_binary: None,
            game: None,
        }
    }
    pub fn with_bridge_binary(mut self, bridge_binary: PathBuf) -> Self {
        self.bridge_binary = Some(bridge_binary);
        self
    }
    pub fn running(&mut self) -> bool {
        if self.game.as_ref().is_some_and(|g| g.check().is_err())
            && let Some(game) = self.game.take()
        {
            let _ = game.close();
        }
        self.game.is_some()
    }
    pub fn start(&mut self, seconds: u64) -> Result<Value> {
        if self.running() {
            let mut value = self.game.as_ref().unwrap().status()?;
            value["already_running"] = json!(true);
            return Ok(value);
        }
        let game = if let Some(bridge) = &self.bridge_binary {
            GameSession::start_with_bridge(
                &self.config,
                &self.runs,
                Duration::from_secs(seconds),
                bridge,
            )?
        } else {
            GameSession::start(&self.config, &self.runs, Duration::from_secs(seconds))?
        };
        let mut value = game.status()?;
        value["already_running"] = json!(false);
        self.game = Some(game);
        Ok(value)
    }
    pub fn stop(&mut self) -> Result<Value> {
        let Some(game) = self.game.take() else {
            return Ok(json!({"running":false,"already_stopped":true}));
        };
        game.close()?;
        Ok(json!({"running":false,"already_stopped":false}))
    }
    pub fn game(&mut self) -> Result<&mut GameSession> {
        self.game
            .as_mut()
            .context("game is not running; call start_game")
    }
}

impl Drop for NativeController {
    fn drop(&mut self) {
        if let Some(game) = self.game.take() {
            let _ = game.close();
        }
    }
}
