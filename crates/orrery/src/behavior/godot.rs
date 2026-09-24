//! One persistent Godot process for the shared functional movement scenarios.
//! The test wrapper sends physical key edges to the saved, compiled Hype Brain.
use super::{
    ActorSample, BackendResult, CLEAR_ROUTE_HEADING_TOLERANCE_RAD, CLEAR_ROUTE_TARGET_HEADING_RAD,
    DeliveredInput, FunctionalRecording, IDLE_ACTION, SamplePoint, Scenario, TimedInput,
    assertions, correction_hold_ms, distance_xy, heading_delta, heading_error_to, wall, wall_jump,
};
use crate::replay::{Edge, Event, Key};
use anyhow::{Context, Result, ensure};
use astrolabe::{
    godot,
    model::{
        AssetPath, Selection,
        io::{Content, ContentRoot},
        scene::Scene,
    },
    openspace::content,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tempfile::TempDir;

const SCENE: &str = "generated/hype/levels/manoir/level.tscn";
const SCRIPT: &str = "tests/behavior_gameplay.gd";
const FIXTURE: &str = "tests/fixtures/english_save_manoir.json";
const STARTUP_SETTLE_MS: u64 = 5_000;
const FIXTURE_BYTES: &[u8] =
    include_bytes!("../../../../godot/tests/fixtures/english_save_manoir.json");

enum ReadyState {
    Fresh,
    ClearRoute,
    WallRoute,
    WallJumpStart,
    JumpStart,
    TurnedLeft { settled: ActorSample, hash: String },
    Finished,
}

pub struct GodotBackend {
    _temporary_project: Option<TempDir>,
    control: TempDir,
    child: Option<Child>,
    next_request: u64,
    runs: PathBuf,
    setup_interventions: Vec<Value>,
    fixture_intervention: Value,
    ready_state: ReadyState,
    project_sha256: String,
    headless: bool,
}

impl GodotBackend {
    pub fn prepare_stationary_jump(&mut self) -> Result<()> {
        let reload = if matches!(self.ready_state, ReadyState::Finished) {
            Some(self.reload_saved_state()?)
        } else {
            None
        };
        ensure!(
            matches!(self.ready_state, ReadyState::Fresh),
            "Godot jump setup requires supplied save"
        );
        self.setup_interventions = vec![self.fixture_intervention.clone()];
        if let Some(reload) = reload {
            self.setup_interventions.push(reload);
        }
        let origin = Instant::now();
        let before = self.observe("jump_setup_before", 0, origin)?;
        self.physical_event(&Event {
            keyboard: 0,
            key: Key::ArrowDown,
            edge: Edge::Press,
        })?;
        thread::sleep(Duration::from_millis(800));
        self.physical_event(&Event {
            keyboard: 0,
            key: Key::ArrowDown,
            edge: Edge::Release,
        })?;
        let release_ms = origin.elapsed().as_millis() as u64;
        thread::sleep(Duration::from_millis(700));
        let after = self.observe("jump_setup_after", 0, origin)?;
        let moved = distance_xy(&before, &after);
        ensure!(
            after.action == Some(IDLE_ACTION)
                && (0.5..=3.5).contains(&moved)
                && heading_delta(&before, &after) < 0.1
                && (after.translation[2] - before.translation[2]).abs() < 0.15,
            "Godot physical Down setup did not reach clear grounded idle: {after:?}"
        );
        self.setup_interventions.push(json!({
            "method":"physical_down_to_clear_jump_start", "planned_hold_ms":800,
            "release_ms":release_ms,"settle_ms":700,"before":before,"after":after,
        }));
        self.ready_state = ReadyState::JumpStart;
        Ok(())
    }
    pub fn start(runs: &Path) -> Result<Self> {
        fs::create_dir_all(runs)?;
        let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
        let (project, temporary) = if let Some(project) =
            std::env::var_os("ASTROLABE_GODOT_BEHAVIOR_PROJECT")
        {
            (PathBuf::from(project), None)
        } else {
            let temporary = tempfile::tempdir()?;
            let project = temporary.path().join("project");
            let game = std::env::var_os("ASTROLABE_GAME_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../disc"));
            let selection = Selection::hype("manoir", "en")?;
            let mut imported = content::import_scene(&game, selection.clone())?;
            godot::support::initialize(&project)?;
            godot::export(
                &mut imported,
                &[ContentRoot {
                    kind: Scene::KIND.into(),
                    reference: AssetPath::new("level:/scene")?,
                }],
                selection,
                &project,
                &engine,
            )?;
            (project, Some(temporary))
        };
        ensure!(
            project.join(SCENE).is_file(),
            "normal manor scene is missing"
        );
        fs::create_dir_all(project.join("tests"))?;
        fs::write(
            project.join(SCRIPT),
            include_str!("../../../../godot/tests/behavior_gameplay.gd"),
        )?;
        fs::create_dir_all(project.join("tests/fixtures"))?;
        fs::write(project.join(FIXTURE), FIXTURE_BYTES)?;
        let fixture: Value = serde_json::from_slice(FIXTURE_BYTES)?;
        ensure!(fixture["schema"] == 1, "unsupported Godot save fixture");
        let fixture_intervention = json!({
            "method": "supplied_english_save_fixture",
            "name": fixture["name"],
            "fixture_sha256": format!("{:x}", Sha256::digest(FIXTURE_BYTES)),
            "source_archive_sha256": fixture["source_archive_sha256"],
            "source_executable_sha256": fixture["source_executable_sha256"],
            "source_translation": fixture["source_translation"],
            "normal_behavior": fixture["normal_behavior"],
            "reflex_behavior": fixture["reflex_behavior"],
        });
        let project_sha256 = format!("{:x}", Sha256::digest(fs::read(project.join(SCENE))?));
        let control = tempfile::tempdir()?;
        let log = File::create(control.path().join("godot.log"))?;
        let mut command = Command::new(&engine);
        command.arg("--path").arg(&project);
        let headless = godot::gameplay_headless()?;
        if headless {
            command.arg("--headless");
        }
        let child = command
            .args(["--script", "res://tests/behavior_gameplay.gd", "--"])
            .arg(control.path())
            .arg(SCENE)
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log))
            .spawn()
            .with_context(|| format!("launch rendered Godot gameplay in {}", project.display()))?;
        let mut backend = Self {
            _temporary_project: temporary,
            control,
            child: Some(child),
            next_request: 0,
            runs: runs.to_owned(),
            setup_interventions: vec![fixture_intervention.clone()],
            fixture_intervention,
            ready_state: ReadyState::Fresh,
            project_sha256,
            headless,
        };
        let ready_path = backend.control.path().join("ready.json");
        let deadline = Instant::now() + Duration::from_secs(70);
        let ready = loop {
            if ready_path.exists() {
                break serde_json::from_slice::<Value>(&fs::read(&ready_path)?)?;
            }
            if let Some(status) = backend.child.as_mut().unwrap().try_wait()? {
                anyhow::bail!(
                    "Godot gameplay exited before ready ({status}): {}",
                    backend.log_tail()
                );
            }
            ensure!(
                Instant::now() < deadline,
                "Godot gameplay startup exceeded 70s: {}",
                backend.log_tail()
            );
            thread::sleep(Duration::from_millis(20));
        };
        backend.setup_interventions[0]["godot_setup_snapshots"] = ready["setup_snapshots"].clone();
        backend.fixture_intervention["godot_setup_snapshots"] = ready["setup_snapshots"].clone();
        backend.validate_observation(&ready)?;
        ensure!(
            ready["action"].as_i64() == Some(i64::from(IDLE_ACTION)),
            "saved Hype did not reach idle after {STARTUP_SETTLE_MS}ms: {ready}"
        );
        Ok(backend)
    }

    fn log_tail(&self) -> String {
        fs::read_to_string(self.control.path().join("godot.log"))
            .unwrap_or_default()
            .chars()
            .rev()
            .take(4_000)
            .collect::<String>()
            .chars()
            .rev()
            .collect()
    }

    fn request(&mut self, request: Value, timeout: Duration) -> Result<Value> {
        self.next_request += 1;
        let temporary = self
            .control
            .path()
            .join(format!("request-{}.tmp", self.next_request));
        let path = self.control.path().join("request.json");
        let response_path = self.control.path().join("response.json");
        ensure!(
            !path.exists() && !response_path.exists(),
            "Godot request protocol is busy"
        );
        fs::write(&temporary, serde_json::to_vec(&request)?)?;
        fs::rename(temporary, &path)?;
        let deadline = Instant::now() + timeout;
        loop {
            if response_path.exists()
                && let Ok(response) = serde_json::from_slice::<Value>(&fs::read(&response_path)?)
            {
                fs::remove_file(response_path)?;
                ensure!(
                    response["error"].as_str().is_none_or(str::is_empty),
                    "Godot gameplay request failed: {response}"
                );
                return Ok(response);
            }
            if let Some(status) = self
                .child
                .as_mut()
                .context("Godot process already stopped")?
                .try_wait()?
            {
                anyhow::bail!("Godot gameplay exited ({status}): {}", self.log_tail());
            }
            ensure!(
                Instant::now() < deadline,
                "Godot gameplay request timed out: {request}; {}",
                self.log_tail()
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn validate_observation(&self, value: &Value) -> Result<()> {
        ensure!(
            value["error"].as_str().is_none_or(str::is_empty),
            "Godot observation failed: {value}"
        );
        ensure!(
            value["brain_failure"].as_str() == Some("") && value["brain_running"] == true,
            "saved compiled Hype Brain stopped: {value}; {}",
            self.log_tail()
        );
        ensure!(
            value["motion_enabled"] == true,
            "saved Hype gameplay motion is disabled: {value}"
        );
        Ok(())
    }

    fn observe(&mut self, name: &str, planned_ms: u64, origin: Instant) -> Result<ActorSample> {
        let start = origin.elapsed().as_millis() as u64;
        let value = self.request(json!({"action": "observe"}), Duration::from_secs(5))?;
        let end = origin.elapsed().as_millis() as u64;
        self.validate_observation(&value)?;
        let coordinates = value["translation"]
            .as_array()
            .context("Godot translation missing")?;
        ensure!(coordinates.len() == 3, "Godot translation is not a vector");
        let mut translation = [0_f32; 3];
        for (i, coordinate) in coordinates.iter().enumerate() {
            translation[i] = coordinate.as_f64().context("Godot coordinate missing")? as f32;
        }
        let action = value["action"].as_i64().context("Godot action missing")?;
        Ok(ActorSample {
            name: name.into(),
            planned_ms,
            capture_start_ms: start,
            capture_end_ms: end,
            action: u32::try_from(action).ok(),
            translation,
            heading_rad: value["heading_rad"]
                .as_f64()
                .context("Godot heading missing")?,
            raw_observation: value,
        })
    }

    fn physical_event(&mut self, event: &Event) -> Result<()> {
        ensure!(
            event.keyboard == 0,
            "only the primary physical keyboard is supported"
        );
        let response = self.request(
            json!({"action": "key", "key": match event.key {
                Key::ArrowLeft => "ArrowLeft", Key::ArrowRight => "ArrowRight",
                Key::ArrowUp => "ArrowUp", Key::ArrowDown => "ArrowDown",
                Key::LeftShift => "LeftShift",
                Key::LeftControl => "LeftControl",
            }, "pressed": event.edge == Edge::Press}),
            Duration::from_secs(5),
        )?;
        self.validate_observation(&response)
    }

    fn reload_saved_state(&mut self) -> Result<Value> {
        let before = self.observe("before_same_process_reload", 0, Instant::now())?;
        let after = self.request(json!({"action": "reload_fixture"}), Duration::from_secs(15))?;
        self.validate_observation(&after)?;
        let source_position = self.fixture_intervention["source_translation"]
            .as_array()
            .context("fixture source position missing")?;
        let position = after["translation"]
            .as_array()
            .context("reloaded Godot position missing")?;
        for axis in 0..3 {
            let expected = source_position[axis]
                .as_f64()
                .context("fixture coordinate missing")?;
            let actual = position[axis]
                .as_f64()
                .context("reloaded coordinate missing")?;
            ensure!(
                (actual - expected).abs() < if axis == 2 { 0.06 } else { 0.08 },
                "same-process Godot reload changed saved position: {after}"
            );
        }
        ensure!(
            after["action"] == IDLE_ACTION
                && after["heading_rad"]
                    .as_f64()
                    .is_some_and(|heading| (heading.abs() - std::f64::consts::PI).abs() < 0.08),
            "same-process Godot reload did not restore idle facing: {after}"
        );
        self.ready_state = ReadyState::Fresh;
        Ok(json!({
            "method": "same_process_packed_scene_reinstantiation",
            "before": before,
            "after": after,
            "resource_settle_ms": STARTUP_SETTLE_MS,
        }))
    }

    /// The successful turn scenario leaves the same process facing a clear route.
    pub fn face_clear_path(&mut self) -> Result<()> {
        let reload = if matches!(self.ready_state, ReadyState::Finished) {
            Some(self.reload_saved_state()?)
        } else {
            None
        };
        self.setup_interventions = vec![self.fixture_intervention.clone()];
        if let Some(reload) = reload {
            self.setup_interventions.push(reload);
        }
        if let ReadyState::TurnedLeft { settled, hash } = &self.ready_state {
            let settled = settled.clone();
            let hash = hash.clone();
            let current = self.observe("handoff_current", 0, Instant::now())?;
            let translation = distance_xy(&settled, &current);
            let heading_change = heading_delta(&settled, &current);
            ensure!(
                current.action == Some(IDLE_ACTION) && translation <= 0.4 && heading_change <= 0.3,
                "Godot turn handoff changed before walking: {current:?}"
            );
            self.setup_interventions.push(json!({
                "method": "compatible_previous_scenario",
                "previous_scenario": Scenario::TurnReleaseIdle,
                "previous_recording_sha256": hash,
                "translation_xy": translation,
                "heading_change_rad": heading_change,
                "previous_settled": settled,
                "current": current,
            }));
            self.ready_state = ReadyState::ClearRoute;
            return Ok(());
        }
        ensure!(
            matches!(self.ready_state, ReadyState::Fresh),
            "clear path requires fresh or compatible turn state"
        );
        let origin = Instant::now();
        let before = self.observe("setup_before", 0, origin)?;
        self.physical_event(&Event {
            keyboard: 0,
            key: Key::ArrowLeft,
            edge: Edge::Press,
        })?;
        thread::sleep(Duration::from_millis(500));
        self.physical_event(&Event {
            keyboard: 0,
            key: Key::ArrowLeft,
            edge: Edge::Release,
        })?;
        thread::sleep(Duration::from_millis(1_800));
        let mut after = self.observe("setup_after", 0, origin)?;
        let mut corrections = Vec::new();
        for _ in 0..6 {
            let error = heading_error_to(CLEAR_ROUTE_TARGET_HEADING_RAD, after.heading_rad);
            if error.abs() <= CLEAR_ROUTE_HEADING_TOLERANCE_RAD {
                break;
            }
            let (key, direction) = if error > 0.0 {
                (Key::ArrowLeft, "left")
            } else {
                (Key::ArrowRight, "right")
            };
            let hold_ms = correction_hold_ms(error);
            let heading_before = after.heading_rad;
            let pulse = Instant::now();
            self.physical_event(&Event {
                keyboard: 0,
                key,
                edge: Edge::Press,
            })?;
            thread::sleep(Duration::from_millis(hold_ms));
            self.physical_event(&Event {
                keyboard: 0,
                key,
                edge: Edge::Release,
            })?;
            let actual_hold_ms = pulse.elapsed().as_millis() as u64;
            thread::sleep(Duration::from_millis(1_800));
            after = self.observe("setup_corrected", 0, origin)?;
            corrections.push(json!({
                "direction": direction,
                "planned_hold_ms": hold_ms,
                "actual_hold_ms": actual_hold_ms,
                "settle_ms": 1_800,
                "heading_before_rad": heading_before,
                "heading_after_rad": after.heading_rad,
            }));
        }
        let heading_change = heading_delta(&before, &after);
        let translation = distance_xy(&before, &after);
        ensure!(
            after.action == Some(IDLE_ACTION)
                && heading_error_to(CLEAR_ROUTE_TARGET_HEADING_RAD, after.heading_rad).abs()
                    <= CLEAR_ROUTE_HEADING_TOLERANCE_RAD
                && heading_change > 0.6
                && translation < 0.4,
            "Godot clear-path turn did not reach the target idle direction {} ± {}: {after:?}",
            CLEAR_ROUTE_TARGET_HEADING_RAD,
            CLEAR_ROUTE_HEADING_TOLERANCE_RAD,
        );
        self.setup_interventions.push(json!({
            "method": "Godot physical ArrowLeft press and release",
            "press_ms": 0, "release_ms": 500, "settle_ms": 1_800,
            "heading_target_rad": CLEAR_ROUTE_TARGET_HEADING_RAD,
            "heading_tolerance_rad": CLEAR_ROUTE_HEADING_TOLERANCE_RAD,
            "corrections": corrections,
            "heading_change_rad": heading_change, "translation_xy": translation,
            "before": before, "after": after,
        }));
        self.ready_state = ReadyState::ClearRoute;
        Ok(())
    }

    pub fn face_wall(&mut self) -> Result<()> {
        let reload = if matches!(self.ready_state, ReadyState::Finished) {
            Some(self.reload_saved_state()?)
        } else {
            None
        };
        ensure!(
            matches!(self.ready_state, ReadyState::Fresh),
            "corner setup requires the supplied save"
        );
        self.setup_interventions = vec![self.fixture_intervention.clone()];
        if let Some(reload) = reload {
            self.setup_interventions.push(reload);
        }
        self.setup_interventions.push(wall::oracle());
        let origin = Instant::now();
        let before = self.observe("wall_setup_before", 0, origin)?;
        self.physical_event(&Event {
            keyboard: 0,
            key: Key::ArrowLeft,
            edge: Edge::Press,
        })?;
        thread::sleep(Duration::from_millis(250));
        self.physical_event(&Event {
            keyboard: 0,
            key: Key::ArrowLeft,
            edge: Edge::Release,
        })?;
        let release_ms = origin.elapsed().as_millis() as u64;
        thread::sleep(Duration::from_millis(1_800));
        let mut after = self.observe("wall_setup_after", 0, origin)?;
        let mut corrections = Vec::new();
        for _ in 0..6 {
            let error = wall::heading_error(after.heading_rad);
            if error.abs() <= wall::HEADING_TOLERANCE_RAD {
                break;
            }
            let (key, direction) = if error > 0.0 {
                (Key::ArrowLeft, "left")
            } else {
                (Key::ArrowRight, "right")
            };
            let hold_ms = correction_hold_ms(error);
            let heading_before = after.heading_rad;
            let pulse = Instant::now();
            self.physical_event(&Event {
                keyboard: 0,
                key,
                edge: Edge::Press,
            })?;
            thread::sleep(Duration::from_millis(hold_ms));
            self.physical_event(&Event {
                keyboard: 0,
                key,
                edge: Edge::Release,
            })?;
            let actual_hold_ms = pulse.elapsed().as_millis() as u64;
            thread::sleep(Duration::from_millis(1_800));
            after = self.observe("wall_setup_corrected", 0, origin)?;
            corrections.push(json!({
                "direction": direction,
                "planned_hold_ms": hold_ms,
                "actual_hold_ms": actual_hold_ms,
                "settle_ms": 1_800,
                "heading_before_rad": heading_before,
                "heading_after_rad": after.heading_rad,
            }));
        }
        let heading = heading_delta(&before, &after);
        let distance = distance_xy(&before, &after);
        ensure!(
            after.action == Some(IDLE_ACTION)
                && wall::heading_error(after.heading_rad).abs() <= wall::HEADING_TOLERANCE_RAD
                && (0.25..=1.4).contains(&heading)
                && distance < 0.4,
            "Godot corner setup did not reach the source route: heading {heading}, final yaw {}, target {}, distance {distance}, action {:?}",
            after.heading_rad,
            wall::TARGET_HEADING_RAD,
            after.action
        );
        self.setup_interventions.push(json!({
            "method": "physical_left_turn_to_manor_corner",
            "planned_hold_ms": 250,
            "release_ms": release_ms,
            "settle_ms": 1_800,
            "heading_target_rad": wall::TARGET_HEADING_RAD,
            "heading_tolerance_rad": wall::HEADING_TOLERANCE_RAD,
            "corrections": corrections,
            "before": before,
            "after": after,
        }));
        self.ready_state = ReadyState::WallRoute;
        Ok(())
    }

    pub fn prepare_wall_jump(&mut self) -> Result<()> {
        self.face_wall()?;
        let origin = Instant::now();
        self.physical_event(&Event {
            keyboard: 0,
            key: Key::ArrowUp,
            edge: Edge::Press,
        })?;
        thread::sleep(Duration::from_millis(3_200));
        self.physical_event(&Event {
            keyboard: 0,
            key: Key::ArrowUp,
            edge: Edge::Release,
        })?;
        thread::sleep(Duration::from_millis(400));
        let contact = self.observe("wall_jump_setup_contact", 0, origin)?;
        self.physical_event(&Event {
            keyboard: 0,
            key: Key::ArrowDown,
            edge: Edge::Press,
        })?;
        thread::sleep(Duration::from_millis(700));
        self.physical_event(&Event {
            keyboard: 0,
            key: Key::ArrowDown,
            edge: Edge::Release,
        })?;
        thread::sleep(Duration::from_millis(700));
        let start = self.observe("wall_jump_setup_start", 0, origin)?;
        wall_jump::validate_ready(&contact, &start)?;
        self.setup_interventions.push(json!({
            "method": "physical_up_to_corner_then_down_to_grounded_wall_jump_start",
            "up_hold_ms": 3_200,
            "contact_settle_ms": 400,
            "down_hold_ms": 700,
            "start_settle_ms": 700,
            "contact": contact,
            "start": start,
        }));
        self.ready_state = ReadyState::WallJumpStart;
        Ok(())
    }

    fn execute_trial(
        &mut self,
        inputs: &[TimedInput],
        points: &[SamplePoint],
        prefix: &str,
        screenshot_at: Option<&str>,
    ) -> Result<(Vec<DeliveredInput>, Vec<ActorSample>, Option<Value>)> {
        let origin = Instant::now();
        let mut delivered = vec![];
        let mut samples = vec![];
        let mut held_screenshot = None;
        let mut actions = inputs
            .iter()
            .map(|v| (v.at_ms, 0_u8, v.id.as_str()))
            .chain(points.iter().map(|v| (v.at_ms, 1_u8, v.name.as_str())))
            .collect::<Vec<_>>();
        actions.sort_by_key(|(at, order, _)| (*at, *order));
        for (at_ms, order, name) in actions {
            if let Some(remaining) =
                (origin + Duration::from_millis(at_ms)).checked_duration_since(Instant::now())
            {
                thread::sleep(remaining);
            }
            if order == 0 {
                let input = inputs
                    .iter()
                    .find(|v| v.id == name)
                    .context("input missing")?;
                let actual_ms = origin.elapsed().as_millis() as u64;
                ensure!(
                    actual_ms <= at_ms + 250,
                    "Godot input {name} missed deadline"
                );
                self.physical_event(&input.event)?;
                delivered.push(DeliveredInput {
                    id: name.into(),
                    planned_ms: at_ms,
                    actual_ms,
                    event: input.event.clone(),
                });
            } else {
                samples.push(self.observe(&format!("{prefix}{name}"), at_ms, origin)?);
                if screenshot_at == Some(name) {
                    held_screenshot = Some(self.screenshot("wall-contact-held")?);
                }
            }
        }
        Ok((delivered, samples, held_screenshot))
    }

    fn screenshot(&mut self, slug: &str) -> Result<Value> {
        if self.headless {
            return Ok(json!({"status": "not_captured_headless"}));
        }
        let screenshot_path = self.runs.join(format!(
            "godot-{}-{}.png",
            slug,
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));
        let screenshot_response = self.request(
            json!({"action": "screenshot", "path": screenshot_path}),
            Duration::from_secs(5),
        )?;
        ensure!(
            screenshot_response["path"].as_str()
                == Some(screenshot_path.to_string_lossy().as_ref()),
            "Godot screenshot path changed: requested {}, received {}",
            screenshot_path.display(),
            screenshot_response
        );
        let screenshot_hash = format!("{:x}", Sha256::digest(fs::read(&screenshot_path)?));
        Ok(json!({"path": screenshot_path, "sha256": screenshot_hash}))
    }

    pub fn run(&mut self, recording: FunctionalRecording) -> Result<BackendResult> {
        recording.validate()?;
        if recording.scenario == Scenario::RunVsWalk {
            return self.run_comparison(recording);
        }
        if recording.scenario == Scenario::WallContact {
            return self.run_wall_contact(recording);
        }
        ensure!(
            matches!(
                (&self.ready_state, recording.scenario),
                (ReadyState::Fresh, Scenario::TurnReleaseIdle)
                    | (ReadyState::ClearRoute, Scenario::WalkReleaseStop)
                    | (ReadyState::JumpStart, Scenario::StationaryJump)
                    | (ReadyState::ClearRoute, Scenario::MovingJump)
                    | (ReadyState::WallJumpStart, Scenario::WallJump)
            ),
            "Godot Given setup is not established"
        );
        let (delivered, samples, _) =
            self.execute_trial(&recording.inputs, &recording.samples, "", None)?;
        let screenshot = self.screenshot(recording.scenario.slug())?;
        let checks = assertions(recording.scenario, &samples)?;
        let status = if checks.iter().all(|c| c.passed) {
            "passed"
        } else {
            "failed"
        };
        let result = BackendResult {
            backend: "godot".into(),
            scenario: recording.scenario,
            status: status.into(),
            recording_sha256: recording.sha256()?,
            recording,
            executable_sha256: Some(self.project_sha256.clone()),
            save_sha256: self.fixture_intervention["source_archive_sha256"]
                .as_str()
                .map(str::to_owned),
            loaded_level: Some("manoir".into()),
            setup_interventions: self.setup_interventions.clone(),
            input_delivery: "godot_inputeventkey_physical_edges".into(),
            observation_consistency: "rendered_frame_scene_observation".into(),
            completed_update: false,
            delivered,
            samples,
            assertions: checks,
            screenshot: Some(screenshot),
            cleanup_complete: false,
            error: None,
        };
        self.ready_state =
            if result.status == "passed" && result.scenario == Scenario::TurnReleaseIdle {
                ReadyState::TurnedLeft {
                    settled: result.samples[4].clone(),
                    hash: result.recording_sha256.clone(),
                }
            } else {
                ReadyState::Finished
            };
        Ok(result)
    }

    fn run_comparison(&mut self, recording: FunctionalRecording) -> Result<BackendResult> {
        ensure!(
            matches!(self.ready_state, ReadyState::ClearRoute),
            "Godot run comparison needs the verified clear-route start"
        );
        let walk = recording.preparatory_trial.as_ref().unwrap();
        let initial_setup = self.setup_interventions.clone();
        let (mut delivered, mut samples, _) =
            self.execute_trial(&walk.inputs, &walk.samples, "walk_", None)?;
        let walk_screenshot = self.screenshot("run-comparison-walk")?;
        let reload = self.reload_saved_state()?;
        self.face_clear_path()?;
        let mut setup_interventions = initial_setup;
        setup_interventions.push(json!({
            "method": "walk_trial_completion",
            "screenshot": walk_screenshot,
            "settled": samples.last(),
        }));
        setup_interventions.push(reload);
        setup_interventions.extend(self.setup_interventions.clone());
        let (run_delivered, run_samples, _) =
            self.execute_trial(&recording.inputs, &recording.samples, "run_", None)?;
        delivered.extend(run_delivered);
        samples.extend(run_samples);
        let screenshot = self.screenshot("run-comparison-run")?;
        let checks = assertions(recording.scenario, &samples)?;
        let status = if checks.iter().all(|check| check.passed) {
            "passed"
        } else {
            "failed"
        };
        let result = BackendResult {
            backend: "godot".into(),
            scenario: recording.scenario,
            status: status.into(),
            recording_sha256: recording.sha256()?,
            recording,
            executable_sha256: Some(self.project_sha256.clone()),
            save_sha256: self.fixture_intervention["source_archive_sha256"]
                .as_str()
                .map(str::to_owned),
            loaded_level: Some("manoir".into()),
            setup_interventions,
            input_delivery: "godot_inputeventkey_physical_edges".into(),
            observation_consistency: "rendered_frame_scene_observation".into(),
            completed_update: false,
            delivered,
            samples,
            assertions: checks,
            screenshot: Some(screenshot),
            cleanup_complete: false,
            error: None,
        };
        self.ready_state = ReadyState::Finished;
        Ok(result)
    }

    fn run_wall_contact(&mut self, recording: FunctionalRecording) -> Result<BackendResult> {
        ensure!(
            matches!(self.ready_state, ReadyState::WallRoute),
            "Godot wall recording needs the verified manor corner route"
        );
        let (delivered, samples, screenshot) = self.execute_trial(
            &recording.inputs,
            &recording.samples,
            "",
            Some("contact_end"),
        )?;
        let checks = assertions(recording.scenario, &samples)?;
        let status = if checks.iter().all(|check| check.passed) {
            "passed"
        } else {
            "failed"
        };
        let result = BackendResult {
            backend: "godot".into(),
            scenario: recording.scenario,
            status: status.into(),
            recording_sha256: recording.sha256()?,
            recording,
            executable_sha256: Some(self.project_sha256.clone()),
            save_sha256: self.fixture_intervention["source_archive_sha256"]
                .as_str()
                .map(str::to_owned),
            loaded_level: Some("manoir".into()),
            setup_interventions: self.setup_interventions.clone(),
            input_delivery: "godot_inputeventkey_physical_edges".into(),
            observation_consistency: "rendered_frame_scene_observation".into(),
            completed_update: false,
            delivered,
            samples,
            assertions: checks,
            screenshot,
            cleanup_complete: false,
            error: None,
        };
        self.ready_state = ReadyState::Finished;
        Ok(result)
    }

    pub fn stop(&mut self) -> Result<()> {
        if self.child.is_none() {
            return Ok(());
        }
        if self.child.as_mut().unwrap().try_wait()?.is_none() {
            let _ = self.request(json!({"action": "stop"}), Duration::from_secs(2));
        }
        let mut child = self.child.take().unwrap();
        if child.try_wait()?.is_none() {
            let deadline = Instant::now() + Duration::from_secs(2);
            while child.try_wait()?.is_none() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(20));
            }
            if child.try_wait()?.is_none() {
                child.kill()?;
            }
        }
        child.wait()?;
        Ok(())
    }
}

impl Drop for GodotBackend {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
