//! Launch-to-exit original-game recording and elapsed-time replay. A recording
//! is immutable; each run gets its own copy-on-write game/prefix and save root.
use crate::{
    config::{Config, digest},
    input_tap::InputTap,
    navigation::Navigation,
    orrery::GameSession,
    replay::{Edge, Key},
    video::{VideoCapture, VideoSummary},
};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use session_recording::{
    Coverage, Environment, Event, EventWriter, Manifest, SaveFixture, capture_saves, freeze_files,
    inspect_saves, read_events, recording_sha256, stage_saves, verify_saves,
};
use sha2::Digest;
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::Read,
    os::fd::AsRawFd,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const FRAME_BUDGET_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_FRAMES: usize = 100_000;
const MAX_PENDING_VIDEO_FINALIZERS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ReplayProfile {
    Literal,
    HypeTitleLoadV1,
}

impl ReplayProfile {
    fn label(self) -> &'static str {
        match self {
            Self::Literal => "literal",
            Self::HypeTitleLoadV1 => "hype-title-load-v1",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedKey {
    pub at_micros: u128,
    pub code: Option<u8>,
    pub key: Option<Key>,
    #[serde(default)]
    pub expected_keysym: Option<u32>,
    pub edge: Edge,
    #[serde(default)]
    pub repeat: bool,
}

pub struct RecordOptions<'a> {
    pub config_path: &'a Path,
    pub output: &'a Path,
    pub source_saves: Option<&'a Path>,
    pub title: &'a str,
    pub description: &'a str,
    pub tags: &'a [String],
    pub seconds: u64,
    pub frame_ms: u64,
    pub script_path: Option<&'a Path>,
}

fn validate_script(script: &[PlannedKey]) -> Result<()> {
    ensure!(script.len() <= 100_000, "input script exceeds event budget");
    let mut held = BTreeSet::new();
    let mut last = 0;
    for input in script {
        ensure!(input.at_micros >= last, "input script is out of order");
        ensure!(
            input.code.is_some() != input.key.is_some(),
            "script input needs one code or key"
        );
        let identity = (input.code, input.key);
        last = input.at_micros;
        match input.edge {
            Edge::Press if input.repeat => {
                ensure!(held.contains(&identity), "repeat without held key")
            }
            Edge::Press => ensure!(held.insert(identity), "duplicate scripted press"),
            Edge::Release => ensure!(held.remove(&identity), "scripted release without press"),
        }
    }
    ensure!(held.is_empty(), "input script ends with a held key");
    Ok(())
}

fn save_root(config: &Config) -> PathBuf {
    config.game.join("GameData/SaveGame")
}

fn reject_nested_save_root(source: &Path) -> Result<()> {
    let Some(name) = source.file_name().and_then(|name| name.to_str()) else {
        return Ok(());
    };
    let Some(slot) = name.strip_prefix("SaveGame") else {
        return Ok(());
    };
    if slot.is_empty() || !slot.bytes().all(|byte| byte.is_ascii_digit()) || !source.is_dir() {
        return Ok(());
    }
    let mut contains_save_files = false;
    for entry in fs::read_dir(source)? {
        if entry?
            .path()
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("sav"))
        {
            contains_save_files = true;
            break;
        }
    }
    ensure!(
        !contains_save_files,
        "selected save root is a SaveGameN slot containing .sav files; pass its parent directory so the SaveGame0/ structure is preserved"
    );
    Ok(())
}

fn environment(config: &Config) -> Result<Environment> {
    fn walk(
        root: &Path,
        directory: &Path,
        hasher: &mut sha2::Sha256,
        files: &mut u64,
    ) -> Result<()> {
        let mut entries = fs::read_dir(directory)?.collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let relative = path.strip_prefix(root)?;
            if relative == Path::new("GameData/SaveGame") {
                continue;
            }
            let metadata = fs::symlink_metadata(&path)?;
            ensure!(
                !metadata.file_type().is_symlink(),
                "game content tree contains a symlink"
            );
            if metadata.is_dir() {
                walk(root, &path, hasher, files)?;
            } else {
                ensure!(metadata.is_file(), "game content contains a non-file entry");
                let name = relative
                    .to_str()
                    .context("game content path is not UTF-8")?;
                hasher.update((name.len() as u64).to_le_bytes());
                hasher.update(name.as_bytes());
                hasher.update(metadata.len().to_le_bytes());
                let mut file = File::open(&path)?;
                let mut buffer = [0u8; 65536];
                loop {
                    let size = file.read(&mut buffer)?;
                    if size == 0 {
                        break;
                    }
                    hasher.update(&buffer[..size]);
                }
                *files += 1;
            }
        }
        Ok(())
    }
    let mut hasher = sha2::Sha256::new();
    let mut files = 0;
    walk(&config.game, &config.game, &mut hasher, &mut files)?;
    Ok(Environment {
        content_tree_sha256: format!("{:x}", hasher.finalize()),
        content_file_count: files,
        runtime_name: "Wine GE 8-26 with Hype Win32".into(),
        runtime_executable_sha256: digest(&config.wine)?,
        locale: std::env::var("LANG").unwrap_or_else(|_| "unknown".into()),
        display_profile:
            "gamescope SDL 1040x800 at 60 Hz; game window dimensions captured per frame".into(),
        control_profile: "private Xwayland physical keyboard; per-event keycode and keysym".into(),
        randomness_inputs: "unverified".into(),
    })
}

fn write_manifest(path: &Path, manifest: &Manifest) -> Result<()> {
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, serde_json::to_vec_pretty(manifest)?)?;
    fs::rename(temp, path)?;
    Ok(())
}

fn prepared_game(
    base_config: &Path,
    output: &Path,
    fixture_dir: &Path,
    fixture: &SaveFixture,
) -> Result<PathBuf> {
    let base_config = base_config.canonicalize()?;
    let base = Config::load_base(&base_config)?;
    let base_directory = base_config.parent().context("base config parent")?;
    let lock = File::open(base_directory.join("lock"))?;
    // SAFETY: the lock protects a quiescent copy of the user's private installation.
    ensure!(
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0,
        "private original-game installation is in use"
    );
    let setup = output.join("setup");
    fs::create_dir(&setup)?;
    let copied_prefix = setup.join("prefix");
    let status = Command::new("cp")
        .arg("-a")
        .arg("--reflink=always")
        .arg(&base.prefix)
        .arg(&copied_prefix)
        .stdout(Stdio::null())
        .status()?;
    ensure!(
        status.success(),
        "copy-on-write private installation failed"
    );
    drop(lock);
    let game_relative = base.game.strip_prefix(&base.prefix)?.to_owned();
    let exe_relative = base.exe.strip_prefix(&base.prefix)?.to_owned();
    let mut copied = base;
    copied.prefix = copied_prefix.clone();
    copied.game = copied_prefix.join(game_relative);
    copied.exe = copied_prefix.join(exe_relative);
    ensure!(
        digest(&copied.exe)? == copied.exe_sha256,
        "staged executable differs"
    );
    let staged_saves = save_root(&copied);
    if staged_saves.exists() {
        ensure!(
            fs::symlink_metadata(&staged_saves)?.is_dir(),
            "staged save root is not a directory"
        );
        fs::remove_dir_all(&staged_saves)?;
    }
    stage_saves(fixture_dir, fixture, &staged_saves)?;
    copied.save_sha256 = format!("{:x}", sha2::Sha256::digest(serde_json::to_vec(fixture)?));
    fs::write(setup.join("lock"), [])?;
    let path = setup.join("config.json");
    fs::write(&path, serde_json::to_vec_pretty(&copied)?)?;
    Config::load_base(&path)?;
    Ok(path)
}

#[derive(Debug)]
struct RunResult {
    elapsed_micros: u128,
    ending: String,
    gaps: Vec<String>,
    frames: usize,
    input_events: usize,
    video_frames: u64,
}

struct RunControls {
    replay: bool,
    navigation: Option<Navigation>,
}

fn record_video_end(
    writer: &mut EventWriter,
    gaps: &mut Vec<String>,
    summary: &VideoSummary,
    event_elapsed_micros: u128,
    finalizer_join_wait_micros: u128,
    finalizer_elapsed_micros: u128,
    successor_start: Option<crate::video::ClockSample>,
) -> Result<()> {
    if summary.timestamp_gaps > 0 {
        gaps.push(format!(
            "{} has {} timestamp gaps",
            summary.path, summary.timestamp_gaps
        ));
    }
    if summary.clock_mapping_drift_micros.unsigned_abs() > 5_000 {
        gaps.push(format!(
            "{} wall-to-monotonic clock mapping drifted {} microseconds",
            summary.path, summary.clock_mapping_drift_micros
        ));
    }
    let stop_requested = (summary.clock_stop_requested.monotonic_before_micros
        + summary.clock_stop_requested.monotonic_after_micros)
        / 2;
    if summary.last_frame_elapsed_micros > stop_requested {
        gaps.push(format!(
            "{} includes {} microseconds of video timestamps after encoder stop was requested",
            summary.path,
            summary.last_frame_elapsed_micros - stop_requested
        ));
    }
    let mut data = serde_json::to_value(summary)?;
    let object = data
        .as_object_mut()
        .context("video summary is not an object")?;
    object.insert(
        "summary_collected_elapsed_micros".into(),
        json!(event_elapsed_micros),
    );
    object.insert(
        "finalizer_join_wait_micros".into(),
        json!(finalizer_join_wait_micros),
    );
    object.insert(
        "finalizer_elapsed_micros".into(),
        json!(finalizer_elapsed_micros),
    );
    object.insert(
        "capture_stop_to_finalization_micros".into(),
        json!(
            summary
                .clock_end
                .monotonic_after_micros
                .saturating_sub(summary.clock_stop_requested.monotonic_before_micros)
        ),
    );
    if let Some(successor_start) = successor_start {
        let successor_midpoint =
            (successor_start.monotonic_before_micros + successor_start.monotonic_after_micros) / 2;
        let stop_midpoint = (summary.clock_stop_requested.monotonic_before_micros
            + summary.clock_stop_requested.monotonic_after_micros)
            / 2;
        object.insert(
            "successor_start_clock".into(),
            serde_json::to_value(successor_start)?,
        );
        object.insert(
            "successor_start_to_stop_overlap_micros".into(),
            json!(stop_midpoint.saturating_sub(successor_midpoint)),
        );
    }
    writer.append(event_elapsed_micros, "video_stream_end", data)?;
    Ok(())
}

struct PendingVideo {
    media_path: PathBuf,
    index_path: PathBuf,
    successor_start: Option<crate::video::ClockSample>,
    expected_dimensions: (u16, u16),
    started: Instant,
    finalizer: Option<thread::JoinHandle<Result<VideoSummary>>>,
}

struct CollectedVideo {
    summary: VideoSummary,
    join_wait_micros: u128,
    finalizer_elapsed_micros: u128,
    successor_start: Option<crate::video::ClockSample>,
    expected_dimensions: (u16, u16),
}

#[derive(Default)]
struct VideoAccounting {
    frames: u64,
    bytes: u64,
}

impl PendingVideo {
    fn spawn(
        video: VideoCapture,
        recording_dir: &Path,
        successor_start: Option<crate::video::ClockSample>,
        expected_dimensions: (u16, u16),
    ) -> Result<Self> {
        let media_path = recording_dir.join(video.relative_path());
        let index_path = media_path.with_extension("frames.jsonl");
        let finalizer = thread::Builder::new()
            .name("orrery-video-finalizer".into())
            .spawn(move || video.finish())
            .context("spawn video finalizer")?;
        Ok(Self {
            media_path,
            index_path,
            successor_start,
            expected_dimensions,
            started: Instant::now(),
            finalizer: Some(finalizer),
        })
    }

    fn stored_bytes(&self) -> Result<u64> {
        Ok(file_len_if_present(&self.media_path)? + file_len_if_present(&self.index_path)?)
    }

    fn collect(mut self) -> Result<CollectedVideo> {
        let join_started = Instant::now();
        let expected_dimensions = self.expected_dimensions;
        let successor_start = self.successor_start;
        let summary = self
            .finalizer
            .take()
            .context("video finalizer already collected")?
            .join()
            .map_err(|_| anyhow::anyhow!("video finalizer thread panicked"))??;
        let join_wait_micros = join_started.elapsed().as_micros();
        let finalizer_elapsed_micros = self.started.elapsed().as_micros();
        Ok(CollectedVideo {
            summary,
            join_wait_micros,
            finalizer_elapsed_micros,
            successor_start,
            expected_dimensions,
        })
    }
}

fn collect_pending_video(
    pending: &mut Vec<PendingVideo>,
    index: usize,
    writer: &mut EventWriter,
    gaps: &mut Vec<String>,
    accounting: &mut VideoAccounting,
    event_elapsed_micros: u128,
    scheduling_backpressure: bool,
) -> Result<()> {
    let collected = pending.remove(index).collect()?;
    let CollectedVideo {
        summary,
        join_wait_micros,
        finalizer_elapsed_micros,
        successor_start,
        expected_dimensions,
    } = collected;
    if (summary.width, summary.height)
        != (
            u32::from(expected_dimensions.0),
            u32::from(expected_dimensions.1),
        )
    {
        gaps.push(format!(
            "{} dimensions differed from the observed window",
            summary.path
        ));
    }
    if scheduling_backpressure && join_wait_micros > 10_000 {
        gaps.push(format!(
            "video finalizer backpressure delayed session scheduling for {join_wait_micros} microseconds"
        ));
    }
    accounting.frames += summary.frames;
    accounting.bytes += summary.bytes;
    record_video_end(
        writer,
        gaps,
        &summary,
        event_elapsed_micros,
        join_wait_micros,
        finalizer_elapsed_micros,
        successor_start,
    )?;
    Ok(())
}

impl Drop for PendingVideo {
    fn drop(&mut self) {
        if let Some(finalizer) = self.finalizer.take() {
            // Ensure FFmpeg cannot outlive a failed session and race recording cleanup.
            // Normal completion joins explicitly and propagates its errors.
            let _ = finalizer.join();
        }
    }
}

fn file_len_if_present(path: &Path) -> Result<u64> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.len()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(error) => Err(error.into()),
    }
}

struct PrefixCleanup(PathBuf);
impl Drop for PrefixCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run_loop(
    game: &mut GameSession,
    recording_dir: &Path,
    writer: &mut EventWriter,
    duration: Duration,
    frame_interval: Duration,
    script: &[PlannedKey],
    controls: RunControls,
) -> Result<RunResult> {
    let RunControls {
        replay,
        mut navigation,
    } = controls;
    writer.append(
        game.elapsed().as_micros(),
        "process_identified",
        json!({
            "identity": game.identity(), "display": game.display(), "first_frame_yet": false,
        }),
    )?;
    let host_display = std::env::var("DISPLAY").unwrap_or_default();
    ensure!(
        game.display().starts_with(':') && game.display() != host_display,
        "refusing original-game video capture outside Orrery's private display"
    );
    let window_deadline = Instant::now() + Duration::from_secs(40);
    let window = loop {
        if let Ok(window) = game.window_id() {
            break window;
        }
        ensure!(
            Instant::now() < window_deadline,
            "game window did not appear"
        );
        thread::sleep(Duration::from_millis(20));
    };
    let input_tap = InputTap::start(
        game.display(),
        &host_display,
        window,
        game.started_instant(),
    )?;
    game.prepare_physical_input()?;
    writer.append(game.elapsed().as_micros(), "input_tap_armed", json!({
        "window":window, "coverage":"xrecord_server_delivered_key_edges", "consumer_verified":false,
    }))?;
    fs::create_dir(recording_dir.join("frames"))?;
    fs::create_dir(recording_dir.join("save-snapshots"))?;
    let mut video_dimensions = game.window_dimensions(window)?;
    let mut video_segment = 0u32;
    let mut video = VideoCapture::start(
        game.display(),
        window,
        recording_dir,
        game.started_instant(),
        &recording_dir
            .parent()
            .context("recording output parent")?
            .join("video-000000.log"),
        video_segment,
    )?;
    writer.append(
        game.elapsed().as_micros(),
        "video_stream_start",
        json!({
            "path":video.relative_path(),
            "capture":"ffmpeg_x11grab_owned_window",
            "codec":"ffv1",
            "requested_frames_per_second":30,
            "clock_start":video.start_clock(),
            "window":window,
            "width":video_dimensions.0,
            "height":video_dimensions.1,
            "render_update_association_verified":false,
        }),
    )?;
    let mut inputs = 0usize;
    let mut observed_held = BTreeSet::new();
    let mut next_input = 0usize;
    let mut frames = 0usize;
    let mut frame_bytes = 0u64;
    let mut video_accounting = VideoAccounting::default();
    let mut pending_videos = Vec::<PendingVideo>::new();
    let mut next_video_geometry = Duration::ZERO;
    let mut next_frame = Duration::ZERO;
    let mut next_state = Duration::ZERO;
    let mut next_save_check = Duration::ZERO;
    let mut last_stable_saves = inspect_saves(&game.save_root())?;
    let mut pending_saves: Option<(SaveFixture, u128)> = None;
    let mut save_snapshots = 0usize;
    let mut last_phase: Option<u64> = None;
    let mut last_level: Option<String> = None;
    let mut level_epoch = 0u64;
    let mut gaps = Vec::new();
    let cancel = Arc::new(AtomicBool::new(false));
    let registrations = [libc::SIGINT, libc::SIGTERM]
        .into_iter()
        .map(|signal| signal_hook::flag::register(signal, cancel.clone()))
        .collect::<Result<Vec<_>, _>>()?;
    let mut ending = "completed".to_owned();
    while game.elapsed()
        < duration
            + navigation
                .as_ref()
                .map_or(Duration::ZERO, Navigation::delay)
    {
        if cancel.load(Ordering::SeqCst) {
            ending = "interrupted".into();
            break;
        }
        let finished = pending_videos.iter().position(|pending| {
            pending
                .finalizer
                .as_ref()
                .is_some_and(thread::JoinHandle::is_finished)
        });
        if let Some(index) = finished {
            collect_pending_video(
                &mut pending_videos,
                index,
                writer,
                &mut gaps,
                &mut video_accounting,
                game.elapsed().as_micros(),
                false,
            )?;
        }
        let now = game.elapsed();
        let pending_bytes = pending_videos.iter().try_fold(0u64, |total, pending| {
            Ok::<_, anyhow::Error>(total.saturating_add(pending.stored_bytes()?))
        })?;
        if video_accounting
            .bytes
            .saturating_add(pending_bytes)
            .saturating_add(video.bytes()?)
            >= FRAME_BUDGET_BYTES
        {
            ending = "storage_budget".into();
            gaps.push("continuous video storage budget reached".into());
            break;
        }
        if now >= next_video_geometry {
            let dimensions = game.window_dimensions(window)?;
            if dimensions != video_dimensions {
                video_segment += 1;
                let next = VideoCapture::start(
                    game.display(),
                    window,
                    recording_dir,
                    game.started_instant(),
                    &recording_dir
                        .parent()
                        .context("recording output parent")?
                        .join(format!("video-{video_segment:06}.log")),
                    video_segment,
                )?;
                writer.append(
                    game.elapsed().as_micros(),
                    "video_stream_start",
                    json!({
                        "path":next.relative_path(), "capture":"ffmpeg_x11grab_owned_window",
                        "codec":"ffv1", "requested_frames_per_second":30,
                        "clock_start":next.start_clock(), "window":window,
                        "width":dimensions.0, "height":dimensions.1,
                        "render_update_association_verified":false,
                    }),
                )?;
                let previous = std::mem::replace(&mut video, next);
                pending_videos.push(PendingVideo::spawn(
                    previous,
                    recording_dir,
                    Some(video.start_clock()),
                    video_dimensions,
                )?);
                while pending_videos.len() > MAX_PENDING_VIDEO_FINALIZERS {
                    collect_pending_video(
                        &mut pending_videos,
                        0,
                        writer,
                        &mut gaps,
                        &mut video_accounting,
                        game.elapsed().as_micros(),
                        true,
                    )?;
                }
                gaps.push(format!(
                    "window resized from {}x{} to {}x{}; the preceding segment may clip frames until the resize was observed",
                    video_dimensions.0, video_dimensions.1, dimensions.0, dimensions.1
                ));
                video_dimensions = dimensions;
            }
            next_video_geometry = game.elapsed() + Duration::from_millis(50);
        }
        while let Some(input) = script.get(next_input) {
            let requested =
                input.at_micros + navigation.as_ref().map_or(0, Navigation::delay_micros);
            if requested > now.as_micros() {
                break;
            }
            if let Some(gates) = navigation.as_mut() {
                let decision = gates.before_input(next_input, now.as_micros(), last_phase)?;
                for event in decision.events {
                    writer.append(game.elapsed().as_micros(), event.kind, event.data)?;
                }
                if let Some(failure) = decision.failure {
                    anyhow::bail!("{failure}");
                }
                if !decision.ready {
                    break;
                }
            }
            let requested = input.at_micros;
            let code = input
                .code
                .or_else(|| input.key.and_then(|key| game.keycode_for(key).ok()))
                .context("script key has no unique private-display mapping")?;
            if let Some(expected) = input.expected_keysym {
                ensure!(
                    game.keysym_for_code(code)? == expected,
                    "replay keyboard mapping differs from recording"
                );
            }
            game.physical_keycode(code, input.edge, input.repeat)?;
            writer.append(
                game.elapsed().as_micros(),
                "input_dispatch",
                json!({
                    "source_elapsed_micros": requested,
                    "late_micros": game.elapsed().as_micros().saturating_sub(requested),
                    "adapted_target_elapsed_micros": requested + navigation.as_ref().map_or(0, Navigation::delay_micros),
                    "navigation_profile":navigation.as_ref().map(Navigation::profile),
                    "keyboard":0, "code":code, "edge":input.edge, "repeat":input.repeat,
                }),
            )?;
            next_input += 1;
        }
        for key in input_tap.drain()? {
            let keysym = game.keysym_for_code(key.code).ok();
            let repeat = if key.edge == "press" {
                !observed_held.insert(key.code)
            } else {
                if !observed_held.remove(&key.code) {
                    gaps.push("input release was observed without a press".into());
                }
                false
            };
            writer.append(
                game.elapsed().as_micros(),
                if replay { "input_observed" } else { "input" },
                json!({
                    "keyboard":0, "code":key.code, "keysym":keysym, "edge":key.edge,
                    "observed_micros":key.elapsed_micros, "server_time_ms":key.server_time_ms,
                    "modifiers":key.modifiers,
                    "consumption_verified":false, "repeat":repeat,
                }),
            )?;
            inputs += 1;
        }
        if now >= next_state {
            match game.state() {
                Ok(state) => {
                    let phase = state["phase"].as_u64();
                    let level = state["level"].as_str().map(str::to_owned);
                    if phase != last_phase || level != last_level {
                        if level != last_level {
                            level_epoch += 1;
                        }
                        writer.append(
                            game.elapsed().as_micros(),
                            "epoch",
                            json!({
                                "phase":phase, "level":level.clone(), "level_epoch":level_epoch,
                            }),
                        )?;
                        last_phase = phase;
                        last_level = level;
                    }
                    let timing = match game.timing() {
                        Ok(timing) => Some(timing),
                        Err(error) => {
                            writer.append(
                                game.elapsed().as_micros(),
                                "capture_gap",
                                json!({
                                    "stream":"timing", "error":error.to_string(),
                                }),
                            )?;
                            None
                        }
                    };
                    writer.append(
                        game.elapsed().as_micros(),
                        "state",
                        json!({
                            "state":state, "timing":timing, "level_epoch":level_epoch,
                            "consistency":"running_non_atomic", "completed_update":false,
                        }),
                    )?;
                    if state["phase"] == 9 {
                        match game.observe_main_actor_with_ai_timed() {
                            Ok((sample, pause)) => {
                                writer.append(game.elapsed().as_micros(), "actor", json!({
                                "actor":sample.actor,
                                "normal_behavior":sample.normal,
                                "reflex_behavior":sample.reflex,
                                "ai_evidence":"sampled_selection_only; transient transitions unobserved",
                                "consistency":"stopped_bytes", "completed_update":false,
                                "pause_micros":pause.as_micros(),
                            }))?;
                            }
                            Err(error) => {
                                writer.append(
                                    game.elapsed().as_micros(),
                                    "capture_gap",
                                    json!({"stream":"actor","error":error.to_string()}),
                                )?;
                            }
                        }
                    }
                }
                Err(error) => {
                    writer.append(
                        game.elapsed().as_micros(),
                        "capture_gap",
                        json!({"stream":"state","error":error.to_string()}),
                    )?;
                }
            }
            next_state = now + Duration::from_millis(500);
        }
        if now >= next_save_check {
            match inspect_saves(&game.save_root()) {
                Ok(current) if current == last_stable_saves => pending_saves = None,
                Ok(current) => {
                    if let Some((pending, first_seen)) = &pending_saves {
                        if *pending == current
                            && now.as_micros().saturating_sub(*first_seen) >= 500_000
                        {
                            let relative = format!("save-snapshots/{save_snapshots:06}");
                            let destination = recording_dir.join(&relative);
                            match capture_saves(&game.save_root(), &destination).and_then(
                                |captured| {
                                    ensure!(captured == current, "save changed during snapshot");
                                    ensure!(
                                        inspect_saves(&game.save_root())? == current,
                                        "save changed after snapshot"
                                    );
                                    Ok(captured)
                                },
                            ) {
                                Ok(captured) => {
                                    writer.append(game.elapsed().as_micros(), "save_snapshot", json!({
                                        "path":relative, "saves":captured,
                                        "first_observed_micros":first_seen,
                                        "stability_micros":now.as_micros().saturating_sub(*first_seen),
                                        "operation_boundary_verified":false,
                                    }))?;
                                    last_stable_saves = current;
                                    pending_saves = None;
                                    save_snapshots += 1;
                                }
                                Err(error) => {
                                    let _ = fs::remove_dir_all(&destination);
                                    writer.append(
                                        game.elapsed().as_micros(),
                                        "capture_gap",
                                        json!({
                                            "stream":"save_snapshot", "error":error.to_string(),
                                        }),
                                    )?;
                                    pending_saves = None;
                                }
                            }
                        } else if *pending != current {
                            pending_saves = Some((current, now.as_micros()));
                        }
                    } else {
                        pending_saves = Some((current, now.as_micros()));
                    }
                }
                Err(error) => {
                    writer.append(
                        game.elapsed().as_micros(),
                        "capture_gap",
                        json!({
                            "stream":"save_snapshot", "error":error.to_string(),
                        }),
                    )?;
                    pending_saves = None;
                }
            }
            next_save_check = now + Duration::from_millis(500);
        }
        if now >= next_frame {
            if frames >= MAX_FRAMES || frame_bytes >= FRAME_BUDGET_BYTES {
                ending = "storage_budget".into();
                gaps.push("PNG checkpoint storage budget reached".into());
                break;
            }
            let path = recording_dir
                .join("frames")
                .join(format!("{frames:06}.png"));
            match game.capture_frame(&path) {
                Ok(frame) => {
                    frame_bytes += fs::metadata(&path)?.len();
                    writer.append(
                        game.elapsed().as_micros(),
                        "frame",
                        json!({
                            "path":format!("frames/{frames:06}.png"),
                            "width":frame.width, "height":frame.height, "window":frame.window,
                            "request_start_micros":frame.request_start_micros,
                            "reply_end_micros":frame.reply_end_micros,
                            "association":frame.association,
                        }),
                    )?;
                    if let Some(gates) = navigation.as_mut() {
                        let relative = format!("frames/{frames:06}.png");
                        for event in gates.observe_frame(
                            &path,
                            &relative,
                            game.elapsed().as_micros(),
                            last_phase,
                        )? {
                            writer.append(game.elapsed().as_micros(), event.kind, event.data)?;
                        }
                    }
                    frames += 1;
                }
                Err(error) => {
                    writer.append(
                        game.elapsed().as_micros(),
                        "capture_gap",
                        json!({"stream":"video","error":error.to_string()}),
                    )?;
                }
            }
            let next = next_frame + frame_interval;
            if now >= next {
                let skipped = now.saturating_sub(next).as_micros() / frame_interval.as_micros() + 1;
                writer.append(
                    game.elapsed().as_micros(),
                    "frame_gap",
                    json!({
                        "stream":"png_checkpoint",
                        "skipped_intervals":skipped,
                        "reason":"PNG checkpoint capture exceeded selected interval",
                    }),
                )?;
                gaps.push(format!(
                    "PNG checkpoint capture skipped {skipped} intervals"
                ));
                next_frame = now + frame_interval;
            } else {
                next_frame = next;
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    for key in input_tap.stop()? {
        let keysym = game.keysym_for_code(key.code).ok();
        let repeat = if key.edge == "press" {
            !observed_held.insert(key.code)
        } else {
            if !observed_held.remove(&key.code) {
                gaps.push("input release was observed without a press".into());
            }
            false
        };
        writer.append(
            game.elapsed().as_micros(),
            if replay { "input_observed" } else { "input" },
            json!({
                "keyboard":0, "code":key.code, "keysym":keysym, "edge":key.edge,
                "observed_micros":key.elapsed_micros, "server_time_ms":key.server_time_ms,
                "modifiers":key.modifiers, "consumption_verified":false, "repeat":repeat,
            }),
        )?;
        inputs += 1;
    }
    for registration in registrations {
        signal_hook::low_level::unregister(registration);
    }
    if next_input != script.len() {
        gaps.push("session ended before all scheduled inputs".into());
    }
    if !observed_held.is_empty() {
        gaps.push(format!(
            "session ended with {} observed keys held",
            observed_held.len()
        ));
    }
    let session_end = game.elapsed().as_micros();
    writer.append(session_end, "session_end", json!({"ending":ending}))?;
    pending_videos.push(PendingVideo::spawn(
        video,
        recording_dir,
        None,
        video_dimensions,
    )?);
    while !pending_videos.is_empty() {
        collect_pending_video(
            &mut pending_videos,
            0,
            writer,
            &mut gaps,
            &mut video_accounting,
            game.elapsed().as_micros(),
            false,
        )?;
    }
    Ok(RunResult {
        elapsed_micros: session_end,
        ending,
        gaps,
        frames,
        input_events: inputs,
        video_frames: video_accounting.frames,
    })
}

fn source_session_duration(
    manifest_elapsed_micros: u128,
    manifest_ending: &str,
    events: &[Event],
) -> Result<u128> {
    let mut session_ends = events
        .iter()
        .enumerate()
        .filter(|(_, event)| event.kind == "session_end");
    let (end_index, session_end) = session_ends
        .next()
        .context("recording has no session_end event")?;
    ensure!(
        session_ends.next().is_none(),
        "recording has multiple session_end events"
    );
    ensure!(
        session_end.elapsed_micros == manifest_elapsed_micros
            && session_end.data["ending"].as_str() == Some(manifest_ending),
        "recording manifest disagrees with session_end event"
    );
    ensure!(
        events[end_index + 1..]
            .iter()
            .all(|event| event.kind == "video_stream_end"),
        "recording contains session activity after session_end"
    );
    Ok(manifest_elapsed_micros)
}

fn finalize(
    output: &Path,
    game: GameSession,
    manifest: &mut Manifest,
    result: Result<RunResult>,
) -> Result<Value> {
    let elapsed_micros = game.elapsed().as_micros();
    let saves = game.save_root();
    let close = game.close();
    let output_saves = capture_saves(&saves, &output.join("output-saves"));
    // The per-run prefix is disposable; captured saves and recording evidence remain.
    let clean = fs::remove_dir_all(output.join("setup/prefix"));
    close?;
    let output_saves = output_saves?;
    clean?;
    let (result, failure) = match result {
        Ok(result) => (result, None),
        Err(error) => {
            let (events, _) = read_events(&output.join("recording/events.jsonl"))?;
            let partial = RunResult {
                elapsed_micros,
                ending: "failed".into(),
                gaps: vec![format!("session capture failed: {error:#}")],
                frames: events.iter().filter(|event| event.kind == "frame").count(),
                input_events: events
                    .iter()
                    .filter(|event| event.kind == "input" || event.kind == "input_observed")
                    .count(),
                video_frames: events
                    .iter()
                    .filter(|event| event.kind == "video_stream_end")
                    .filter_map(|event| event.data["frames"].as_u64())
                    .next_back()
                    .unwrap_or(0),
            };
            (partial, Some(error))
        }
    };
    manifest.elapsed_micros = result.elapsed_micros;
    manifest.ending = result.ending;
    manifest.coverage.gaps.extend(result.gaps);
    if result.video_frames > 0 {
        manifest.coverage.video = "continuous lossless FFV1 Matroska window capture at 30 Hz with wall-clock frame index; sparse PNG checkpoints; game render/update association unverified".into();
        manifest.coverage.gaps.retain(|gap| gap != "continuous video is not captured; frame-to-render/update association is unverified; audio is not captured");
        manifest.coverage.gaps.push("video frames are presented-window samples; source render/update association is unverified and audio is not captured".into());
    }
    write_manifest(&output.join("recording/manifest.json"), manifest)?;
    freeze_files(&output.join("recording"))?;
    let sha256 = recording_sha256(&output.join("recording"))?;
    let report = json!({
        "recording_sha256":sha256,
        "recording":output.join("recording"),
        "error":failure.as_ref().map(|error| format!("{error:#}")),
        "frames":result.frames,
        "video_frames":result.video_frames,
        "input_events":result.input_events,
        "elapsed_micros":result.elapsed_micros,
        "ending":manifest.ending,
        "output_saves":output_saves,
        "cleanup_complete":true,
        "coverage":manifest.coverage,
    });
    fs::write(
        output.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    if let Some(error) = failure {
        return Err(error.context(format!(
            "session failed; readable evidence retained at {}",
            output.join("recording").display()
        )));
    }
    Ok(report)
}

pub fn record(options: RecordOptions<'_>) -> Result<Value> {
    let RecordOptions {
        config_path,
        output,
        source_saves,
        title,
        description,
        tags,
        seconds,
        frame_ms,
        script_path,
    } = options;
    let absolute_output = if output.is_absolute() {
        output.to_owned()
    } else {
        std::env::current_dir()?.join(output)
    };
    let output = absolute_output.as_path();
    ensure!(
        (5..=43_000).contains(&seconds),
        "duration outside 5..43000 seconds"
    );
    ensure!(
        (50..=1_000).contains(&frame_ms),
        "frame interval outside 50..1000 ms"
    );
    ensure!(
        !title.is_empty() && title.len() <= 128,
        "invalid recording title"
    );
    let script: Vec<PlannedKey> = script_path
        .map(|path| -> Result<_> { Ok(serde_json::from_slice(&fs::read(path)?)?) })
        .transpose()?
        .unwrap_or_default();
    validate_script(&script)?;
    let base = Config::load_base(config_path)?;
    let run_environment = environment(&base)?;
    let source = source_saves
        .map(Path::to_path_buf)
        .unwrap_or_else(|| save_root(&base));
    reject_nested_save_root(&source)?;
    fs::create_dir(output)?;
    let recording = output.join("recording");
    fs::create_dir(&recording)?;
    let fixture = capture_saves(&source, &recording.join("initial-saves"))?;
    let _prefix_cleanup = PrefixCleanup(output.join("setup/prefix"));
    let config = prepared_game(
        config_path,
        output,
        &recording.join("initial-saves"),
        &fixture,
    )?;
    let created_unix_micros = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros();
    let mut manifest = Manifest {
        schema:session_recording::SCHEMA,
        id:format!("hype-{created_unix_micros}"),
        title:title.to_owned(), description:description.to_owned(), tags:tags.to_vec(),
        driver:if script.is_empty() { "human" } else { "scripted" }.into(),
        executable_sha256:base.exe_sha256,
        environment:run_environment,
        initial_saves:fixture,
        started_unix_micros:0, elapsed_micros:0, ending:"in_progress".into(),
        replay_of_sha256:None,
        coverage:Coverage {
            input:"xrecord_server_delivered_key_edges; consumption unverified".into(),
            video:"timestamped, sparse lossless PNG samples; continuous video and render update association unavailable".into(),
            state:"phase/level running sample and actor/action/normal/reflex stopped-byte sample in gameplay".into(),
            ai_api:"not instrumented".into(), audio:"not captured".into(),
            gaps:vec![
                "capture attaches after process identification and visible window; startup before first frame is explicit".into(),
                "input consumption is unverified; mouse and gamepad input are not captured".into(),
                "focus changes are not captured; keyboard edges are scoped to the game window".into(),
                "continuous video is not captured; frame-to-render/update association is unverified; audio is not captured".into(),
                "AI transitions and API arguments/results are not traced".into(),
                "save changes are sampled and snapshotted after observed stability; engine save-operation boundaries and completion are unverified".into(),
            ],
        },
    };
    write_manifest(&recording.join("manifest.json"), &manifest)?;
    let mut writer = EventWriter::create(&recording.join("events.jsonl"))?;
    let mut game = GameSession::start_session(
        &config,
        &output.join("runtime"),
        Duration::from_secs(seconds.max(60) + 60),
    )?;
    manifest.started_unix_micros = game.started_unix_micros();
    write_manifest(&recording.join("manifest.json"), &manifest)?;
    writer.append(
        0,
        "launch",
        json!({"recording_armed_before_process_launch":true}),
    )?;
    let result = run_loop(
        &mut game,
        &recording,
        &mut writer,
        Duration::from_secs(seconds),
        Duration::from_millis(frame_ms),
        &script,
        RunControls {
            replay: false,
            navigation: None,
        },
    );
    if let Err(error) = &result {
        let _ = writer.append(
            game.elapsed().as_micros(),
            "session_failure",
            json!({"error":format!("{error:#}")}),
        );
    }
    finalize(output, game, &mut manifest, result)
}

pub fn replay(
    config_path: &Path,
    source_recording: &Path,
    output: &Path,
    frame_ms: u64,
    profile: ReplayProfile,
) -> Result<Value> {
    let absolute_output = if output.is_absolute() {
        output.to_owned()
    } else {
        std::env::current_dir()?.join(output)
    };
    let output = absolute_output.as_path();
    ensure!(
        (50..=1_000).contains(&frame_ms),
        "frame interval outside 50..1000 ms"
    );
    let source_sha256 = recording_sha256(source_recording)?;
    let manifest: Manifest =
        serde_json::from_slice(&fs::read(source_recording.join("manifest.json"))?)?;
    ensure!(
        manifest.schema == session_recording::SCHEMA
            && matches!(
                manifest.ending.as_str(),
                "completed" | "interrupted" | "storage_budget"
            ),
        "recording is incomplete or unsupported"
    );
    let (events, complete) = read_events(&source_recording.join("events.jsonl"))?;
    ensure!(complete, "recording has a torn final event");
    let original_duration =
        source_session_duration(manifest.elapsed_micros, &manifest.ending, &events)?;
    let source_fixture = source_recording.join("initial-saves");
    verify_saves(
        &source_fixture,
        &SaveFixture {
            kind: if manifest.initial_saves.kind == session_recording::SaveRootKind::Absent {
                session_recording::SaveRootKind::Empty
            } else {
                manifest.initial_saves.kind
            },
            directories: manifest.initial_saves.directories.clone(),
            files: manifest.initial_saves.files.clone(),
        },
    )?;
    let script = events
        .iter()
        .filter(|event| event.kind == "input")
        .map(|event| -> Result<PlannedKey> {
            let data = &event.data;
            Ok(PlannedKey {
                at_micros: data["observed_micros"]
                    .as_u64()
                    .map(u128::from)
                    .unwrap_or(event.elapsed_micros),
                code: Some(
                    data["code"]
                        .as_u64()
                        .context("recorded input code missing")?
                        .try_into()?,
                ),
                key: None,
                expected_keysym: data["keysym"].as_u64().map(u32::try_from).transpose()?,
                edge: serde_json::from_value(data["edge"].clone())?,
                repeat: data["repeat"].as_bool().unwrap_or(false),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    validate_script(&script)?;
    ensure!(
        script.iter().all(|key| key.at_micros <= original_duration),
        "recording input extends beyond session_end"
    );
    let navigation = match profile {
        ReplayProfile::Literal => None,
        ReplayProfile::HypeTitleLoadV1 => Some(Navigation::new(&script)?),
    };
    let base = Config::load_base(config_path)?;
    ensure!(
        base.exe_sha256 == manifest.executable_sha256,
        "recording executable build differs"
    );
    let current_environment = environment(&base)?;
    ensure!(
        current_environment.content_tree_sha256 == manifest.environment.content_tree_sha256
            && current_environment.runtime_executable_sha256
                == manifest.environment.runtime_executable_sha256,
        "recording content or Wine runtime differs"
    );
    fs::create_dir(output)?;
    let _prefix_cleanup = PrefixCleanup(output.join("setup/prefix"));
    let recording = output.join("recording");
    fs::create_dir(&recording)?;
    stage_saves(
        &source_fixture,
        &manifest.initial_saves,
        &recording.join("initial-saves"),
    )?;
    if !recording.join("initial-saves").exists() {
        fs::create_dir(recording.join("initial-saves"))?;
    }
    ensure!(
        recording_sha256(source_recording)? == source_sha256,
        "source recording changed while replay inputs were copied"
    );
    let config = prepared_game(
        config_path,
        output,
        &recording.join("initial-saves"),
        &manifest.initial_saves,
    )?;
    let mut replay_manifest = manifest;
    replay_manifest.id = format!(
        "{}-replay-{}",
        replay_manifest.id,
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    );
    replay_manifest.driver = format!("replay:{}", profile.label());
    if profile != ReplayProfile::Literal {
        replay_manifest.coverage.gaps.push(
            "readiness-gated navigation may add explicit delays; it does not establish literal startup timing equivalence or input consumption".into(),
        );
    }
    replay_manifest.replay_of_sha256 = Some(source_sha256.clone());
    replay_manifest.elapsed_micros = 0;
    replay_manifest.ending = "in_progress".into();
    replay_manifest.started_unix_micros = 0;
    write_manifest(&recording.join("manifest.json"), &replay_manifest)?;
    let mut writer = EventWriter::create(&recording.join("events.jsonl"))?;
    let duration = Duration::from_micros(u64::try_from(original_duration)?.max(5_000_000));
    let mut game = GameSession::start_session(
        &config,
        &output.join("runtime"),
        duration.max(Duration::from_secs(60)) + Duration::from_secs(120),
    )?;
    replay_manifest.started_unix_micros = game.started_unix_micros();
    write_manifest(&recording.join("manifest.json"), &replay_manifest)?;
    writer.append(
        0,
        "launch",
        json!({"recording_armed_before_process_launch":true,"replay_of_sha256":source_sha256,"replay_profile":profile.label()}),
    )?;
    let result = run_loop(
        &mut game,
        &recording,
        &mut writer,
        duration,
        Duration::from_millis(frame_ms),
        &script,
        RunControls {
            replay: true,
            navigation,
        },
    );
    if let Err(error) = &result {
        let _ = writer.append(
            game.elapsed().as_micros(),
            "session_failure",
            json!({"error":format!("{error:#}")}),
        );
    }
    let mut report = finalize(output, game, &mut replay_manifest, result)?;
    let (replayed, complete) = read_events(&recording.join("events.jsonl"))?;
    ensure!(complete, "replay event stream is torn");
    let source_input = events
        .iter()
        .filter(|event| event.kind == "input")
        .map(|event| {
            (
                event.data["code"].as_u64(),
                event.data["edge"].as_str().map(str::to_owned),
                event.data["modifiers"].as_u64(),
            )
        })
        .collect::<Vec<_>>();
    let observed_input = replayed
        .iter()
        .filter(|event| event.kind == "input_observed")
        .map(|event| {
            (
                event.data["code"].as_u64(),
                event.data["edge"].as_str().map(str::to_owned),
                event.data["modifiers"].as_u64(),
            )
        })
        .collect::<Vec<_>>();
    let without_repeat = |events: &[session_recording::Event], kind: &str| {
        events
            .iter()
            .filter(|event| event.kind == kind && event.data["repeat"] != true)
            .map(|event| {
                (
                    event.data["code"].as_u64(),
                    event.data["edge"].as_str().map(str::to_owned),
                )
            })
            .collect::<Vec<_>>()
    };
    let source_nonrepeat = without_repeat(&events, "input");
    let observed_nonrepeat = without_repeat(&replayed, "input_observed");
    let first_input_difference = (0..source_input.len().max(observed_input.len())).find(|&index| {
        match (source_input.get(index), observed_input.get(index)) {
            (Some(source), Some(observed)) => {
                source.0 != observed.0
                    || source.1 != observed.1
                    || source
                        .2
                        .is_some_and(|modifiers| observed.2 != Some(modifiers))
            }
            _ => true,
        }
    });
    let dispatch_lateness = replayed
        .iter()
        .filter(|event| event.kind == "input_dispatch")
        .filter_map(|event| event.data["late_micros"].as_u64())
        .max();
    let dispatched_input_events = replayed
        .iter()
        .filter(|event| event.kind == "input_dispatch")
        .count();
    let navigation_waits = replayed
        .iter()
        .filter(|event| event.kind == "navigation_wait_end")
        .map(|event| event.data.clone())
        .collect::<Vec<_>>();
    let navigation_added_delay_micros = navigation_waits
        .last()
        .and_then(|event| event["total_added_delay_micros"].as_u64())
        .unwrap_or(0);
    let delivery = json!({
        "replay_profile":profile.label(),
        "navigation_waits":navigation_waits,
        "navigation_added_delay_micros":navigation_added_delay_micros,
        "source_input_events":source_input.len(),
        "dispatched_input_events":dispatched_input_events,
        "observed_input_events":observed_input.len(),
        "input_sequence_equal":first_input_difference.is_none(),
        "source_nonrepeat_edges":source_nonrepeat.len(),
        "observed_nonrepeat_edges":observed_nonrepeat.len(),
        "nonrepeat_edge_sequence_equal":source_nonrepeat == observed_nonrepeat,
        "modifier_samples_available":source_input.iter().filter(|event| event.2.is_some()).count(),
        "first_input_difference":first_input_difference,
        "source_edge_at_first_difference":first_input_difference.and_then(|index| source_input.get(index)),
        "observed_edge_at_first_difference":first_input_difference.and_then(|index| observed_input.get(index)),
        "maximum_dispatch_lateness_micros":dispatch_lateness,
    });
    fs::write(
        output.join("delivery.json"),
        serde_json::to_vec_pretty(&delivery)?,
    )?;
    report["delivery"] = delivery;
    report["delivery_validation"] = json!({
        "status":if first_input_difference.is_none() { "passed" } else { "failed" },
        "rule":"X-server observed keyboard edge identity/order must equal the source recording; game consumption remains unverified",
    });
    if first_input_difference.is_some() {
        report["error"] = json!("replay physical input delivery differs from recording");
    }
    fs::write(
        output.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    ensure!(
        first_input_difference.is_none(),
        "replay physical input delivery differs from recording"
    );
    Ok(report)
}

#[cfg(test)]
mod video_event_tests {
    use super::*;
    use crate::video::ClockSample;

    #[test]
    fn video_end_event_uses_collection_time_and_preserves_capture_timing() {
        let directory = tempfile::tempdir().unwrap();
        let mut writer = EventWriter::create(&directory.path().join("events.jsonl")).unwrap();
        writer
            .append(30_000_000, "session_end", json!({"ending":"completed"}))
            .unwrap();
        let summary = VideoSummary {
            path: "video/000000.mkv".into(),
            frame_index_path: "video/000000.frames.jsonl".into(),
            codec: "ffv1",
            container: "matroska",
            capture: "x11grab",
            requested_frames_per_second: 30,
            frames: 3,
            bytes: 128,
            first_frame_elapsed_micros: 100,
            last_frame_elapsed_micros: 200,
            width: 800,
            height: 600,
            timestamp_gaps: 0,
            clock_mapping_drift_micros: 0,
            clock_start: ClockSample {
                monotonic_before_micros: 90,
                wall_unix_micros: 1_000,
                monotonic_after_micros: 100,
            },
            clock_stop_requested: ClockSample {
                monotonic_before_micros: 210,
                wall_unix_micros: 1_120,
                monotonic_after_micros: 220,
            },
            clock_end: ClockSample {
                monotonic_before_micros: 41_999_990,
                wall_unix_micros: 42_001_150,
                monotonic_after_micros: 42_000_010,
            },
            ffmpeg_exit_code: Some(255),
        };
        let successor_start = ClockSample {
            monotonic_before_micros: 180,
            wall_unix_micros: 1_090,
            monotonic_after_micros: 190,
        };

        record_video_end(
            &mut writer,
            &mut Vec::new(),
            &summary,
            42_000_000,
            20,
            120,
            Some(successor_start),
        )
        .unwrap();

        let (events, complete) = read_events(&directory.path().join("events.jsonl")).unwrap();
        assert!(complete);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].elapsed_micros, 30_000_000);
        assert_eq!(events[0].kind, "session_end");
        assert_eq!(events[1].elapsed_micros, 42_000_000);
        assert_eq!(
            events[1].data["clock_end"]["monotonic_after_micros"],
            42_000_010
        );
        assert_eq!(
            events[1].data["summary_collected_elapsed_micros"],
            42_000_000
        );
        assert_eq!(events[1].data["finalizer_join_wait_micros"], 20);
        assert_eq!(events[1].data["finalizer_elapsed_micros"], 120);
        assert_eq!(events[1].data["successor_start_to_stop_overlap_micros"], 30);
        assert_eq!(
            source_session_duration(30_000_000, "completed", &events).unwrap(),
            30_000_000
        );
    }

    #[test]
    fn replay_uses_session_end_before_later_video_finalization_event() {
        let events = vec![
            Event {
                sequence: 0,
                elapsed_micros: 30_000_000,
                kind: "session_end".into(),
                data: json!({"ending":"completed"}),
            },
            Event {
                sequence: 1,
                elapsed_micros: 42_000_000,
                kind: "video_stream_end".into(),
                data: json!({}),
            },
        ];
        assert_eq!(
            source_session_duration(30_000_000, "completed", &events).unwrap(),
            30_000_000
        );
        assert!(source_session_duration(42_000_000, "completed", &events).is_err());
    }

    #[test]
    fn selected_save_slot_is_rejected_but_empty_or_absent_roots_are_allowed() {
        let directory = tempfile::tempdir().unwrap();
        let slot = directory.path().join("SaveGame0");
        fs::create_dir(&slot).unwrap();
        fs::write(slot.join("save.sav"), b"fixture").unwrap();
        assert!(reject_nested_save_root(&slot).is_err());
        fs::remove_file(slot.join("save.sav")).unwrap();
        assert!(reject_nested_save_root(&slot).is_ok());
        assert!(reject_nested_save_root(&directory.path().join("absent")).is_ok());
    }
}
