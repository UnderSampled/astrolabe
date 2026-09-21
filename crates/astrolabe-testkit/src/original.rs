//! Native prepared-fixture inspection. The outer process owns Gamescope and a
//! bounded worker; that worker links Astrolabe directly and owns Wine. No Python
//! broker, user-supplied memory addresses or gameplay replay fallback is involved.
use crate::{
    config::{Config, digest},
    desktop::{Desktop, SetupKey},
    linux::{Identity, Owner, Process},
};
use anyhow::{Context, Result, ensure};
use astrolabe::openspace::level::live::{HypeBuild, ReadMemory};
use serde_json::json;
use std::{
    fs::{self, File},
    os::{fd::AsRawFd, unix::ffi::OsStrExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

struct Cancellation {
    flag: Arc<AtomicBool>,
    registrations: Vec<signal_hook::SigId>,
}
impl Cancellation {
    fn new() -> Result<Self> {
        let mut result = Self {
            flag: Arc::new(AtomicBool::new(false)),
            registrations: vec![],
        };
        for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
            result
                .registrations
                .push(signal_hook::flag::register(signal, result.flag.clone())?);
        }
        Ok(result)
    }
    fn check(&self) -> Result<()> {
        ensure!(
            !self.flag.load(Ordering::SeqCst),
            "native controller interrupted"
        );
        Ok(())
    }
    fn wait(&self, duration: Duration) -> Result<()> {
        let deadline = Instant::now() + duration;
        while Instant::now() < deadline {
            self.check()?;
            thread::sleep(
                Duration::from_millis(5).min(deadline.saturating_duration_since(Instant::now())),
            );
        }
        self.check()
    }
}
impl Drop for Cancellation {
    fn drop(&mut self) {
        for id in &self.registrations {
            signal_hook::low_level::unregister(*id);
        }
    }
}
fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    use std::io::Write;
    ensure!(
        !path.exists(),
        "evidence file already exists: {}",
        path.display()
    );
    let temporary = path.with_extension("json.tmp");
    let mut file = File::create_new(&temporary)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    drop(file);
    fs::rename(temporary, path)?;
    Ok(())
}
struct Running<'a>(&'a Process);
impl ReadMemory for Running<'_> {
    fn read(&mut self, address: u32, size: usize) -> Result<Vec<u8>> {
        self.0.read_running(address as usize, size)
    }
}
fn phase(
    process: &Process,
    build: &HypeBuild,
    wanted: u8,
    level: Option<&str>,
    limit: Duration,
    cancel: &Cancellation,
) -> Result<()> {
    let deadline = Instant::now() + limit;
    loop {
        let status = build.status(&mut Running(process))?;
        if status.phase == wanted && level.is_none_or(|s| s == status.level) {
            return Ok(());
        }
        ensure!(
            Instant::now() < deadline,
            "game did not reach phase {wanted} / {level:?}; observed {} / {}",
            status.phase,
            status.level
        );
        cancel.wait(Duration::from_millis(50))?;
    }
}
fn find_game(owner: &mut Owner, config: &Config, cancel: &Cancellation) -> Result<Process> {
    let deadline = Instant::now() + Duration::from_secs(40);
    loop {
        let mut found = vec![];
        for process in owner.processes()? {
            let pid = process.identity().pid;
            let cmd = fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
            if cmd.split(|b| *b == 0).next() != Some(config.exe.as_os_str().as_bytes()) {
                continue;
            }
            let environment = fs::read(format!("/proc/{pid}/environ")).unwrap_or_default();
            let prefix = format!("WINEPREFIX={}", config.prefix.display());
            if !environment
                .split(|b| *b == 0)
                .any(|v| v == prefix.as_bytes())
            {
                continue;
            }
            if process.read_running(0x400000, 2).is_ok_and(|b| b == b"MZ") {
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
        cancel.wait(Duration::from_millis(50))?;
    }
}

/// Only the Gamescope-launched worker calls this. Its DISPLAY/XAUTHORITY are
/// inherited at exec from the isolated server, never changed in a running process.
pub fn worker(
    config_path: &Path,
    output: &Path,
    host_display: &str,
    fail_stopped_worker: bool,
) -> Result<()> {
    let cancel = Cancellation::new()?;
    let config = Config::load(config_path)?;
    let build = HypeBuild::verified(&config.exe_sha256)?;
    let started = Instant::now();
    let mut owner = Owner::new()?;
    write_json(&output.join("worker-started.json"), &Identity::current()?)?;
    let result = (|| -> Result<serde_json::Value> {
        ensure!(
            std::env::var("DISPLAY").is_ok_and(|d| d != host_display && d.starts_with(':')),
            "worker is not on a private display"
        );
        let wine_log = File::create_new(output.join("wine.log"))?;
        let mut command = Command::new(&config.wine);
        command
            .arg(&config.exe)
            .current_dir(&config.game)
            .envs(config.environment()?)
            .stdout(Stdio::from(wine_log.try_clone()?))
            .stderr(Stdio::from(wine_log));
        owner.spawn(&mut command)?;
        let process = find_game(&mut owner, &config, &cancel)?;
        // Verify the mapped image header against the fully hashed local module.
        let image = fs::read(&config.exe)?;
        ensure!(
            process.read_running(0x400000, 512)?
                == image.get(..512).context("short executable header")?,
            "live image header differs from pinned module"
        );
        phase(&process, &build, 38, None, Duration::from_secs(40), &cancel)?;
        let mut desktop = Desktop::connect(host_display, process.identity().pid as u32)?;
        cancel.wait(Duration::from_millis(500))?;
        desktop.focus()?;
        cancel.wait(Duration::from_millis(800))?;
        write_json(
            &output.join("menu-ready.json"),
            &json!({"phase":38,"pid":process.identity(),"display":desktop.display}),
        )?;
        if fail_stopped_worker {
            let _stopped = process.pause(Duration::from_secs(1))?;
            write_json(
                &output.join("fault-armed.json"),
                &json!({"fault":"worker death while game is stopped","process":process.identity()}),
            )?;
            // SAFETY: deliberate acceptance-test worker death, without unwinding
            // or resuming the game. The separate native owner must clean it up.
            unsafe {
                libc::_exit(77);
            }
        }
        let mut setup = vec![];
        for key in [SetupKey::Down, SetupKey::Return, SetupKey::Return] {
            let before = started.elapsed().as_micros();
            desktop.setup_key(key, |duration| cancel.wait(duration))?;
            setup.push(json!({"key":key,"start_micros":before,"end_micros":started.elapsed().as_micros(),"purpose":"load supplied save; timed menu setup only"}));
        }
        phase(
            &process,
            &build,
            9,
            Some("manoir"),
            Duration::from_secs(15),
            &cancel,
        )?;
        cancel.wait(Duration::from_secs(4))?;
        let capture_start = started.elapsed().as_micros();
        let stopped = process.pause(Duration::from_secs(1))?;
        let mut capture = stopped.capture()?;
        let status = build.status(&mut capture)?;
        let actor = build.actor(&mut capture)?;
        let snapshot = capture.finish()?;
        drop(stopped);
        let capture_end = started.elapsed().as_micros();
        write_json(&output.join("snapshot.json"), &snapshot)?;
        let screenshot = desktop.screenshot(&output.join("original.png"), started)?;
        desktop.release()?;
        let mut graphics = std::collections::BTreeMap::new();
        for name in [
            "glide3x.dll",
            "ddraw.dll",
            "d3d11.dll",
            "dxgi.dll",
            "dgVoodoo.conf",
            "dxvk.conf",
        ] {
            let path = config.game.join(name);
            if path.is_file() {
                graphics.insert(name, digest(&path)?);
            }
        }
        Ok(
            json!({"schema":1,"status":"inspection_complete","scope":"native fixture launch and stopped-byte actor inspection",
            "fixture":config,"graphics_sha256":graphics,"process":process.identity(),"game":status,"actor":actor,
            "setup":setup,"capture_start_micros":capture_start,"capture_end_micros":capture_end,
            "snapshot":"snapshot.json","screenshot":screenshot,"consistency":"stopped_bytes",
            "completed_update":false,"frame_replay":false,"behavior_parity":false}),
        )
    })();
    let cleanup_started = Instant::now();
    let cleanup = owner.close();
    let mut report = match &result {
        Ok(value) => value.clone(),
        Err(error) => json!({"schema":1,"status":"failed","error":format!("{error:#}")}),
    };
    report["cleanup_seconds"] = json!(cleanup_started.elapsed().as_secs_f64());
    report["cleanup_complete"] = json!(cleanup.is_ok());
    report["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
    if let Err(error) = &cleanup {
        report["status"] = json!("failed");
        report["cleanup_error"] = json!(format!("{error:#}"));
    }
    write_json(&output.join("worker.json"), &report)?;
    result?;
    cleanup?;
    Ok(())
}

struct Supervisor {
    owner: Owner,
    focus_file: PathBuf,
    original_focus: [Option<String>; 2],
    focus_changed: bool,
}
impl Supervisor {
    fn new() -> Result<Self> {
        let focus_file = PathBuf::from(std::env::var_os("HOME").context("HOME missing")?)
            .join(".config/kwinrulesrc");
        let text = fs::read_to_string(&focus_file)?;
        let mut section = "";
        let mut entries = std::collections::BTreeMap::new();
        for line in text.lines() {
            if line.starts_with('[') {
                section = line;
            }
            if let Some((key, value)) = line.split_once('=') {
                entries.insert((section.to_string(), key.to_string()), value.to_string());
            }
        }
        let key = |section: &str, name: &str| {
            entries.get(&(section.to_owned(), name.to_owned())).cloned()
        };
        let group = "[hype-harness-startup-focus]";
        ensure!(
            key(group, "wmclass").as_deref() == Some("hype-harness")
                && key(group, "fsplevel").as_deref() == Some("4")
                && key(group, "fsplevelrule").as_deref() == Some("2")
                && key("[General]", "rules")
                    .is_some_and(|s| s.split(',').any(|v| v == "hype-harness-startup-focus")),
            "prepared harness-specific KWin focus rule is missing"
        );
        Ok(Self {
            owner: Owner::new()?,
            focus_file,
            original_focus: [key(group, "acceptfocus"), key(group, "acceptfocusrule")],
            focus_changed: false,
        })
    }
    fn command(&mut self, command: &mut Command) -> Result<()> {
        let process = self
            .owner
            .spawn(command.stdout(Stdio::null()).stderr(Stdio::null()))?;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.owner.try_wait(&process)? {
                ensure!(status.success(), "focus helper failed: {status}");
                return Ok(());
            }
            ensure!(Instant::now() < deadline, "focus helper deadline exceeded");
            thread::sleep(Duration::from_millis(10));
        }
    }
    fn focus(&mut self, enable: bool) -> Result<()> {
        if !enable && !self.focus_changed {
            return Ok(());
        }
        self.focus_changed = true;
        for (index, key) in ["acceptfocus", "acceptfocusrule"].into_iter().enumerate() {
            let value = if enable {
                Some(if index == 0 { "false" } else { "2" }.to_owned())
            } else {
                self.original_focus[index].clone()
            };
            let mut command = Command::new("kwriteconfig6");
            command.arg("--file").arg(&self.focus_file).args([
                "--group",
                "hype-harness-startup-focus",
                "--key",
                key,
            ]);
            if let Some(value) = value {
                command.arg(value);
            } else {
                command.arg("--delete");
            }
            self.command(&mut command)?;
        }
        self.command(Command::new("qdbus6").args(["org.kde.KWin", "/KWin", "reconfigure"]))?;
        self.focus_changed = enable;
        Ok(())
    }
    fn close(&mut self) -> Result<()> {
        let focus = self.focus(false);
        let children = self.owner.close();
        focus?;
        children
    }
}
impl Drop for Supervisor {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// Owns a bounded inspection worker and restores the existing focus rule even
/// when that worker crashes or hangs. Both native and reference controllers use
/// the prepared installation's same advisory lock; its service is not stopped.
pub fn inspect(config_path: &Path, output: &Path, fail_stopped_worker: bool) -> Result<()> {
    let cancel = Cancellation::new()?;
    let config_path = config_path.canonicalize()?;
    let lock = File::open(
        config_path
            .parent()
            .context("configuration parent")?
            .join("lock"),
    )?;
    // SAFETY: flock operates only on this open installation lock descriptor.
    ensure!(
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0,
        "prepared original-game installation is already in use"
    );
    let config = Config::load(&config_path)?;
    // Refuse a leftover instance, including one whose former owner was killed.
    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        if !entry.file_name().as_bytes().iter().all(u8::is_ascii_digit) {
            continue;
        }
        let cmd = fs::read(entry.path().join("cmdline")).unwrap_or_default();
        ensure!(
            cmd.split(|b| *b == 0).next() != Some(config.exe.as_os_str().as_bytes()),
            "private game is already running"
        );
    }
    fs::create_dir(output).context("choose a new evidence directory with an existing parent")?;
    let output = output.canonicalize()?;
    let started = Instant::now();
    let mut supervisor = Supervisor::new()?;
    let result = (|| -> Result<()> {
        supervisor.focus(true)?;
        let log = File::create_new(output.join("gamescope.log"))?;
        let mut command = Command::new("gamescope");
        command
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
            .arg(std::env::current_exe()?)
            .arg("original-worker")
            .arg("--config")
            .arg(&config_path)
            .arg("--output")
            .arg(&output)
            .arg("--host-display")
            .arg(std::env::var("DISPLAY").unwrap_or_default())
            .env("SDL_VIDEODRIVER", "wayland")
            .env("SDL_VIDEO_WAYLAND_WMCLASS", "hype-harness")
            .env("SDL_APP_ID", "hype-harness")
            .env("SDL_WINDOW_NO_ACTIVATION_WHEN_SHOWN", "1")
            .env_remove("XDG_ACTIVATION_TOKEN")
            .env_remove("DESKTOP_STARTUP_ID")
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log));
        if fail_stopped_worker {
            command.arg("--fail-stopped-worker");
        }
        let gamescope = supervisor.owner.spawn(&mut command)?;
        loop {
            cancel.check()?;
            supervisor.owner.processes()?;
            if output.join("menu-ready.json").exists() {
                supervisor.focus(false)?;
            }
            if let Some(status) = supervisor.owner.try_wait(&gamescope)? {
                ensure!(
                    status.success(),
                    "Gamescope/worker failed: {status}; see gamescope.log"
                );
                break;
            }
            // Gamescope can wait for surviving descendants after its primary
            // worker dies. Observe the worker itself so a stopped game cannot
            // hide its owner's death until the whole-scenario deadline.
            let worker_path = output.join("worker-started.json");
            if worker_path.exists() && !output.join("worker.json").exists() {
                let identity: Identity = serde_json::from_slice(&fs::read(worker_path)?)?;
                let owned = supervisor.owner.processes()?;
                ensure!(
                    owned
                        .iter()
                        .any(|p| p.identity() == identity && p.check().is_ok()),
                    "native worker exited before publishing its result"
                );
            }
            ensure!(
                started.elapsed() < Duration::from_secs(180),
                "native inspection worker deadline exceeded"
            );
            cancel.wait(Duration::from_millis(25))?;
        }
        let report: serde_json::Value =
            serde_json::from_slice(&fs::read(output.join("worker.json"))?)?;
        ensure!(
            report["status"] == "inspection_complete" && report["cleanup_complete"] == true,
            "native worker did not finish inspection and cleanup"
        );
        Ok(())
    })();
    let cleanup = supervisor.close();
    write_json(
        &output.join("summary.json"),
        &json!({"schema":1,"status":if result.is_ok() && cleanup.is_ok() {"passed"} else {"failed"},
        "scope":"native original-game inspection, not frame replay or parity", "elapsed_seconds":started.elapsed().as_secs_f64(),
        "cleanup_complete":cleanup.is_ok(),"error":result.as_ref().err().map(|e|format!("{e:#}")),"cleanup_error":cleanup.as_ref().err().map(|e|format!("{e:#}")),"worker_report":"worker.json"}),
    )?;
    result?;
    cleanup?;
    Ok(())
}
