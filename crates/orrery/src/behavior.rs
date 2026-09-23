//! Functional behavior evidence shared by the OpenSpace and future Godot
//! backends. Milliseconds here are wall-clock scheduling targets, never the
//! verified consumer ordinals used by `replay`.
use crate::replay::{Edge, Event, Key};
use anyhow::{Context, Result, ensure};
use astrolabe::openspace::level::live::ActorObservation;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    thread,
    time::{Duration, Instant},
};

#[cfg(target_os = "linux")]
use crate::{config::Config, orrery::NativeController};

pub const IDLE_ACTION: u32 = 47;
pub const TURN_LEFT_ACTION: u32 = 35;
pub const WALK_FORWARD_ACTION: u32 = 32;
pub const ORIGINAL_STARTUP_SETTLE_MS: u64 = 2_500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scenario {
    TurnReleaseIdle,
    WalkReleaseStop,
}
impl Scenario {
    pub fn slug(self) -> &'static str {
        match self {
            Self::TurnReleaseIdle => "turn-release-idle",
            Self::WalkReleaseStop => "walk-release-stop",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimedInput {
    pub at_ms: u64,
    pub id: String,
    pub event: Event,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SamplePoint {
    pub at_ms: u64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionalRecording {
    pub schema: u32,
    pub scenario: Scenario,
    pub ready_state: String,
    pub inputs: Vec<TimedInput>,
    pub samples: Vec<SamplePoint>,
}
impl FunctionalRecording {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.schema == 1, "unsupported functional recording schema");
        let expected_ready_state = match self.scenario {
            Scenario::TurnReleaseIdle => "supplied-english-save-manoir",
            Scenario::WalkReleaseStop => "manoir-clear-route-after-physical-left-turn",
        };
        ensure!(
            self.ready_state == expected_ready_state,
            "unknown ready state"
        );
        ensure!(
            self.inputs.len() == 2,
            "functional first pass has one press/release pair"
        );
        ensure!(
            self.inputs[0].at_ms < self.inputs[1].at_ms
                && self.inputs[0].event.edge == Edge::Press
                && self.inputs[1].event.edge == Edge::Release
                && self.inputs[0].event.key == self.inputs[1].event.key
                && self.inputs.iter().all(|i| i.event.keyboard == 0),
            "functional input must be one bounded physical press/release"
        );
        let names = self
            .samples
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>();
        ensure!(
            names
                == [
                    "baseline",
                    "held",
                    "released",
                    "settled_start",
                    "settled_end"
                ],
            "functional samples must cover the declared observation window"
        );
        ensure!(
            self.samples.windows(2).all(|p| p[0].at_ms < p[1].at_ms)
                && self.samples[0].at_ms < self.inputs[0].at_ms
                && self.samples[1].at_ms > self.inputs[0].at_ms
                && self.samples[1].at_ms < self.inputs[1].at_ms
                && self.samples[2].at_ms > self.inputs[1].at_ms
                && self.samples[4].at_ms - self.samples[3].at_ms >= 300,
            "functional schedule has missing or reordered observation points"
        );
        ensure!(
            self.samples.last().is_some_and(|s| s.at_ms <= 6_000),
            "functional recording exceeds six seconds"
        );
        Ok(())
    }
    pub fn sha256(&self) -> Result<String> {
        self.validate()?;
        Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(self)?)))
    }
}

pub fn recording(scenario: Scenario) -> FunctionalRecording {
    let (key, held_ms, release_ms, settled_start, settled_end) = match scenario {
        Scenario::TurnReleaseIdle => (Key::ArrowLeft, 600, 700, 2_500, 2_900),
        Scenario::WalkReleaseStop => (Key::ArrowUp, 450, 900, 2_200, 2_600),
    };
    FunctionalRecording {
        schema: 1,
        scenario,
        ready_state: match scenario {
            Scenario::TurnReleaseIdle => "supplied-english-save-manoir",
            Scenario::WalkReleaseStop => "manoir-clear-route-after-physical-left-turn",
        }
        .into(),
        inputs: vec![
            TimedInput {
                at_ms: 200,
                id: "press".into(),
                event: Event {
                    keyboard: 0,
                    key,
                    edge: Edge::Press,
                },
            },
            TimedInput {
                at_ms: release_ms,
                id: "release".into(),
                event: Event {
                    keyboard: 0,
                    key,
                    edge: Edge::Release,
                },
            },
        ],
        samples: [
            (0, "baseline"),
            (held_ms, "held"),
            (release_ms + 100, "released"),
            (settled_start, "settled_start"),
            (settled_end, "settled_end"),
        ]
        .into_iter()
        .map(|(at_ms, name)| SamplePoint {
            at_ms,
            name: name.into(),
        })
        .collect(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveredInput {
    pub id: String,
    pub planned_ms: u64,
    pub actual_ms: u64,
    pub event: Event,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorSample {
    pub name: String,
    pub planned_ms: u64,
    pub capture_start_ms: u64,
    pub capture_end_ms: u64,
    pub action: Option<u32>,
    pub translation: [f32; 3],
    pub heading_rad: f64,
    /// Target-native capture retained alongside coordinates normalized into
    /// OpenSpace source units and canonical action IDs.
    pub raw_observation: serde_json::Value,
}
impl ActorSample {
    pub fn from_actor(
        name: String,
        planned_ms: u64,
        start_ms: u64,
        end_ms: u64,
        actor: ActorObservation,
    ) -> Result<Self> {
        let affine = actor.global_transform.affine();
        let forward = affine.rotation[0];
        let heading_rad = f64::from(forward[1]).atan2(f64::from(forward[0]));
        ensure!(heading_rad.is_finite(), "actor heading is nonfinite");
        Ok(Self {
            name,
            planned_ms,
            capture_start_ms: start_ms,
            capture_end_ms: end_ms,
            action: actor.current_action,
            translation: affine.translation,
            heading_rad,
            raw_observation: serde_json::to_value(actor)?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assertion {
    pub name: String,
    pub passed: bool,
    pub actual: f64,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    pub source: String,
}
fn bound(
    name: &str,
    actual: f64,
    lower: Option<f64>,
    upper: Option<f64>,
    source: &str,
) -> Assertion {
    Assertion {
        name: name.into(),
        actual,
        lower,
        upper,
        source: source.into(),
        passed: actual.is_finite()
            && lower.is_none_or(|v| actual >= v)
            && upper.is_none_or(|v| actual <= v),
    }
}
fn distance_xy(a: &ActorSample, b: &ActorSample) -> f64 {
    let dx = f64::from(b.translation[0] - a.translation[0]);
    let dy = f64::from(b.translation[1] - a.translation[1]);
    dx.hypot(dy)
}
fn heading_delta(a: &ActorSample, b: &ActorSample) -> f64 {
    let difference = b.heading_rad - a.heading_rad;
    difference.sin().atan2(difference.cos()).abs()
}
fn action_is(sample: &ActorSample, action: u32) -> f64 {
    f64::from(u8::from(sample.action == Some(action)))
}

/// Independently sourced functional bounds. The 2026-09-23 Orrery manor
/// observation saw about 80 degrees/500 ms Left and 4.1 source units/1.8 s Up.
/// These broad limits classify motion without claiming timing or pixel parity.
pub fn assertions(scenario: Scenario, samples: &[ActorSample]) -> Result<Vec<Assertion>> {
    ensure!(samples.len() == 5, "five named samples required");
    ensure!(
        samples.iter().map(|s| s.name.as_str()).eq([
            "baseline",
            "held",
            "released",
            "settled_start",
            "settled_end"
        ]),
        "samples do not match functional recording"
    );
    let [baseline, held, released, settled_start, settled_end] = samples else {
        unreachable!()
    };
    let source =
        "Orrery 2026-09-23 clean manoir save, physical Left/Up observation; broad functional bound";
    let mut checks = vec![
        bound(
            "baseline_idle_action",
            action_is(baseline, IDLE_ACTION),
            Some(1.),
            Some(1.),
            source,
        ),
        bound(
            "active_action",
            action_is(
                held,
                match scenario {
                    Scenario::TurnReleaseIdle => TURN_LEFT_ACTION,
                    Scenario::WalkReleaseStop => WALK_FORWARD_ACTION,
                },
            ),
            Some(1.),
            Some(1.),
            source,
        ),
        bound(
            "settled_idle_action_start",
            action_is(settled_start, IDLE_ACTION),
            Some(1.),
            Some(1.),
            source,
        ),
        bound(
            "settled_idle_action_end",
            action_is(settled_end, IDLE_ACTION),
            Some(1.),
            Some(1.),
            source,
        ),
        bound(
            "post_release_drift_xy",
            distance_xy(settled_start, settled_end),
            None,
            Some(0.4),
            source,
        ),
    ];
    match scenario {
        Scenario::TurnReleaseIdle => {
            checks.extend([
                bound(
                    "heading_change_rad",
                    heading_delta(baseline, released),
                    Some(0.3),
                    Some(2.5),
                    source,
                ),
                bound(
                    "turn_translation_xy",
                    distance_xy(baseline, settled_end),
                    None,
                    Some(0.4),
                    source,
                ),
            ]);
        }
        Scenario::WalkReleaseStop => {
            checks.extend([
                bound(
                    "walk_translation_xy",
                    distance_xy(baseline, released),
                    Some(0.8),
                    Some(2.5),
                    source,
                ),
                bound(
                    "late_held_progress_xy",
                    distance_xy(held, released),
                    Some(0.45),
                    None,
                    "observed original walk must still advance after the held checkpoint, rather than stopping at the wall before release",
                ),
                bound(
                    "settling_continuation_xy",
                    distance_xy(released, settled_end),
                    None,
                    Some(0.6),
                    source,
                ),
                bound(
                    "walk_height_change",
                    f64::from((released.translation[2] - baseline.translation[2]).abs()),
                    None,
                    Some(0.5),
                    source,
                ),
                bound(
                    "heading_retained_rad",
                    heading_delta(baseline, released),
                    None,
                    Some(0.3),
                    source,
                ),
            ]);
        }
    }
    Ok(checks)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendResult {
    pub backend: String,
    pub scenario: Scenario,
    pub status: String,
    pub recording: FunctionalRecording,
    pub recording_sha256: String,
    pub executable_sha256: Option<String>,
    pub save_sha256: Option<String>,
    pub loaded_level: Option<String>,
    pub setup_interventions: Vec<serde_json::Value>,
    pub input_delivery: String,
    pub observation_consistency: String,
    pub completed_update: bool,
    pub delivered: Vec<DeliveredInput>,
    pub samples: Vec<ActorSample>,
    pub assertions: Vec<Assertion>,
    pub screenshot: Option<serde_json::Value>,
    pub cleanup_complete: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossBackendComparison {
    pub status: String,
    pub scenario: Scenario,
    pub recording_sha256: String,
    pub checks: Vec<Assertion>,
    pub visual_status: String,
    pub timing_profile: String,
}

/// Compare independently passing backend results at the same named samples.
/// Godot must report positions in source units and canonical action IDs while
/// retaining its native scene/physics observation in `raw_observation`.
pub fn compare_results(
    openspace: &BackendResult,
    godot: &BackendResult,
) -> Result<CrossBackendComparison> {
    ensure!(
        openspace.backend == "openspace" && godot.backend == "godot",
        "comparison requires named real backends"
    );
    ensure!(openspace.scenario == godot.scenario, "scenario mismatch");
    ensure!(
        openspace.recording_sha256 == godot.recording_sha256,
        "recording hash mismatch"
    );
    openspace.require_pass()?;
    godot.require_pass()?;
    ensure!(
        openspace.samples.len() == godot.samples.len() && openspace.samples.len() == 5,
        "comparison sample coverage differs"
    );
    let mut checks = vec![];
    for (source, target) in openspace.samples.iter().zip(&godot.samples) {
        ensure!(
            source.name == target.name,
            "comparison sample order differs"
        );
        checks.push(bound(
            &format!("{}_action_match", source.name),
            f64::from(u8::from(source.action == target.action)),
            Some(1.),
            Some(1.),
            "canonical action identity at the same named checkpoint",
        ));
        let sx = source.translation[0] - openspace.samples[0].translation[0];
        let sy = source.translation[1] - openspace.samples[0].translation[1];
        let tx = target.translation[0] - godot.samples[0].translation[0];
        let ty = target.translation[1] - godot.samples[0].translation[1];
        let path_error = f64::from(sx - tx).hypot(f64::from(sy - ty));
        checks.push(bound(
            &format!("{}_relative_xy_error", source.name),
            path_error,
            None,
            Some(1.0),
            "functional path tolerance; source-relative positions in canonical units",
        ));
        let source_yaw = source.heading_rad - openspace.samples[0].heading_rad;
        let target_yaw = target.heading_rad - godot.samples[0].heading_rad;
        let yaw_difference = source_yaw - target_yaw;
        checks.push(bound(
            &format!("{}_relative_heading_error", source.name),
            yaw_difference.sin().atan2(yaw_difference.cos()).abs(),
            None,
            Some(0.35),
            "functional heading tolerance; source-relative orientation",
        ));
    }
    ensure!(
        openspace.delivered.len() == godot.delivered.len(),
        "input delivery count differs"
    );
    for (source, target) in openspace.delivered.iter().zip(&godot.delivered) {
        ensure!(
            source.id == target.id && source.event == target.event,
            "physical input event mismatch"
        );
        checks.push(bound(
            &format!("{}_delivery_delta_ms", source.id),
            source.actual_ms.abs_diff(target.actual_ms) as f64,
            None,
            Some(250.),
            "functional scheduling tolerance; not verified consumer timing",
        ));
    }
    Ok(CrossBackendComparison {
        status: if checks.iter().all(|c| c.passed) {
            "passed"
        } else {
            "failed"
        }
        .into(),
        scenario: openspace.scenario,
        recording_sha256: openspace.recording_sha256.clone(),
        checks,
        visual_status: "pending_completed_render_pairing".into(),
        timing_profile: "functional_elapsed_ms_no_consumption_parity".into(),
    })
}
impl BackendResult {
    pub fn pending_godot(scenario: Scenario) -> serde_json::Value {
        serde_json::json!({
            "backend": "godot", "scenario": scenario, "status": "pending",
            "dependency": "normal manoir conversion retains Hype AI model and Brain and saved compiled player path drives native movement",
            "completion_check": "run the same functional recording through normal Godot input and evaluate the scenario assertions"
        })
    }
    pub fn require_pass(&self) -> Result<()> {
        ensure!(
            self.status == "passed" && self.cleanup_complete,
            "backend did not pass with cleanup: {}",
            self.error.as_deref().unwrap_or(&self.status)
        );
        self.recording.validate()?;
        ensure!(
            self.recording.scenario == self.scenario
                && self.recording.sha256()? == self.recording_sha256,
            "backend result recording identity differs"
        );
        ensure!(
            self.delivered.len() == self.recording.inputs.len(),
            "backend input delivery is incomplete"
        );
        for (delivered, planned) in self.delivered.iter().zip(&self.recording.inputs) {
            ensure!(
                delivered.id == planned.id
                    && delivered.event == planned.event
                    && delivered.planned_ms == planned.at_ms,
                "backend delivered a different physical event"
            );
            ensure!(
                delivered.actual_ms >= planned.at_ms.saturating_sub(10)
                    && delivered.actual_ms <= planned.at_ms + 250,
                "backend input delivery missed functional deadline"
            );
        }
        ensure!(
            self.samples.len() == self.recording.samples.len(),
            "backend sample coverage is incomplete"
        );
        for (sample, planned) in self.samples.iter().zip(&self.recording.samples) {
            ensure!(
                sample.name == planned.name && sample.planned_ms == planned.at_ms,
                "backend sample does not match recording checkpoint"
            );
            ensure!(
                sample.capture_end_ms >= sample.capture_start_ms
                    && sample.capture_start_ms >= planned.at_ms.saturating_sub(10)
                    && sample.capture_start_ms <= planned.at_ms + 150
                    && sample.capture_end_ms <= planned.at_ms + 250,
                "backend sample has an invalid capture interval"
            );
            ensure!(
                sample.translation.iter().all(|x| x.is_finite()) && sample.heading_rad.is_finite(),
                "backend sample contains nonfinite coordinates"
            );
        }
        ensure!(
            self.samples[4]
                .capture_start_ms
                .saturating_sub(self.samples[3].capture_end_ms)
                >= 300,
            "settled observation window was shortened by late sampling"
        );
        let expected_checks = assertions(self.scenario, &self.samples)?;
        ensure!(
            self.assertions.len() == expected_checks.len(),
            "backend assertion coverage differs"
        );
        for (check, expected) in self.assertions.iter().zip(&expected_checks) {
            ensure!(
                check.name == expected.name
                    && (check.actual - expected.actual).abs() <= 0.00001
                    && check.lower == expected.lower
                    && check.upper == expected.upper
                    && check.passed == expected.passed,
                "backend assertion evidence is stale or altered: {}",
                expected.name
            );
            ensure!(
                check.passed,
                "{} failed: {} outside {:?}..{:?}",
                check.name,
                check.actual,
                check.lower,
                check.upper
            );
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
pub struct OriginalBackend {
    controller: NativeController,
    config: Config,
    setup_interventions: Vec<serde_json::Value>,
    ready_state: OriginalReadyState,
}

#[cfg(target_os = "linux")]
enum OriginalReadyState {
    FreshSave,
    ClearRoute,
    TurnedLeft {
        settled: ActorSample,
        recording_sha256: String,
    },
    Finished,
}

#[cfg(target_os = "linux")]
impl OriginalBackend {
    pub fn start(config_path: &Path, runs: &Path) -> Result<Self> {
        Self::start_with_bridge(config_path, runs, None)
    }

    pub fn start_with_bridge(
        config_path: &Path,
        runs: &Path,
        bridge: Option<&Path>,
    ) -> Result<Self> {
        let config = Config::load(config_path)?;
        let mut controller = NativeController::new(config_path.to_owned(), runs.to_owned());
        if let Some(bridge) = bridge {
            controller = controller.with_bridge_binary(bridge.to_owned());
        }
        controller.start(180)?;
        controller
            .game()?
            .keys(&["Down".into(), "Return".into(), "Return".into()], 100, 300)?;
        let deadline = Instant::now() + Duration::from_secs(35);
        loop {
            let state = controller.game()?.state()?;
            if state["phase"] == 9 && state["level"] == "manoir" {
                break;
            }
            ensure!(
                Instant::now() < deadline,
                "supplied save did not reach manoir: {state}"
            );
            thread::sleep(Duration::from_millis(100));
        }
        // The loaded save briefly fades in. Wait for the same idle state used
        // to establish the original measurement, without changing game state.
        let deadline = Instant::now() + Duration::from_secs(12);
        loop {
            if controller.game()?.observe_main_actor()?.current_action == Some(IDLE_ACTION) {
                break;
            }
            ensure!(
                Instant::now() < deadline,
                "manoir actor did not become idle"
            );
            thread::sleep(Duration::from_millis(100));
        }
        // The save can report idle before startup streaming settles. Keep the
        // loading stutter outside the timed functional recording.
        thread::sleep(Duration::from_millis(ORIGINAL_STARTUP_SETTLE_MS));
        let settled_state = controller.game()?.state()?;
        let settled_actor = controller.game()?.observe_main_actor()?;
        ensure!(
            settled_state["phase"] == 9
                && settled_state["level"] == "manoir"
                && settled_actor.current_action == Some(IDLE_ACTION),
            "manor save changed during startup settle: {settled_state}"
        );
        Ok(Self {
            controller,
            config,
            setup_interventions: vec![],
            ready_state: OriginalReadyState::FreshSave,
        })
    }

    /// The supplied save faces a barred gate. This documented Given turns the
    /// actor toward the clear manor route by actual input before the walking
    /// tape starts; it does not change actor state or position in memory.
    pub fn face_clear_path(&mut self) -> Result<()> {
        self.setup_interventions.clear();
        if let OriginalReadyState::TurnedLeft {
            settled,
            recording_sha256,
        } = &self.ready_state
        {
            let state = self.controller.game()?.state()?;
            ensure!(
                state["phase"] == 9 && state["level"] == "manoir",
                "turn handoff left the expected manor level: {state}"
            );
            let current = ActorSample::from_actor(
                "handoff_current".into(),
                0,
                0,
                0,
                self.controller.game()?.observe_main_actor()?,
            )?;
            let translation = distance_xy(settled, &current);
            let heading_change = heading_delta(settled, &current);
            ensure!(
                current.action == Some(IDLE_ACTION) && translation <= 0.4 && heading_change <= 0.3,
                "turn handoff changed before walking: action {:?}, translation {translation}, heading {heading_change}",
                current.action
            );
            self.setup_interventions.push(serde_json::json!({
                "reason": "the preceding successful turn left Hype facing the measured clear route",
                "method": "compatible_previous_scenario",
                "previous_scenario": Scenario::TurnReleaseIdle,
                "previous_recording_sha256": recording_sha256,
                "translation_xy": translation,
                "heading_change_rad": heading_change,
                "previous_settled": settled,
                "current": current,
            }));
            self.ready_state = OriginalReadyState::ClearRoute;
            return Ok(());
        }
        ensure!(
            matches!(&self.ready_state, OriginalReadyState::FreshSave),
            "clear-path setup requires a clean save or compatible turn"
        );
        let before = self.controller.game()?.observe_main_actor()?;
        let start = Instant::now();
        self.controller.game()?.prepare_physical_input()?;
        let press_ms = start.elapsed().as_millis();
        self.controller.game()?.physical_events(&[Event {
            keyboard: 0,
            key: Key::ArrowLeft,
            edge: Edge::Press,
        }])?;
        thread::sleep(Duration::from_millis(500));
        let release_ms = start.elapsed().as_millis();
        self.controller.game()?.physical_events(&[Event {
            keyboard: 0,
            key: Key::ArrowLeft,
            edge: Edge::Release,
        }])?;
        thread::sleep(Duration::from_millis(1_800));
        let after = self.controller.game()?.observe_main_actor()?;
        let before = ActorSample::from_actor("setup_before".into(), 0, 0, 0, before)?;
        let after = ActorSample::from_actor("setup_after".into(), 0, 0, 0, after)?;
        let heading_change = heading_delta(&before, &after);
        let translation = distance_xy(&before, &after);
        ensure!(
            after.action == Some(IDLE_ACTION) && heading_change > 0.6 && translation < 0.4,
            "clear-path setup did not produce stationary idle turn: action {:?}, heading {heading_change}, translation {translation}",
            after.action
        );
        self.setup_interventions.push(serde_json::json!({
            "reason": "supplied manor save faces a barred gate; orient toward measured clear route",
            "method": "private XTest physical ArrowLeft press and release",
            "press_ms": press_ms, "release_ms": release_ms,
            "settle_ms": 1_800, "heading_change_rad": heading_change,
            "translation_xy": translation,
            "before": before, "after": after,
        }));
        self.ready_state = OriginalReadyState::ClearRoute;
        Ok(())
    }

    pub fn run(&mut self, recording: FunctionalRecording) -> Result<BackendResult> {
        recording.validate()?;
        ensure!(
            matches!(
                (&self.ready_state, recording.scenario),
                (OriginalReadyState::FreshSave, Scenario::TurnReleaseIdle)
                    | (OriginalReadyState::ClearRoute, Scenario::WalkReleaseStop)
            ),
            "the selected scenario's Given setup was not established"
        );
        self.controller.game()?.prepare_physical_input()?;
        let origin = Instant::now();
        let mut delivered = vec![];
        let mut samples = vec![];
        let mut actions = recording
            .inputs
            .iter()
            .map(|v| (v.at_ms, 0_u8, v.id.as_str()))
            .chain(
                recording
                    .samples
                    .iter()
                    .map(|v| (v.at_ms, 1_u8, v.name.as_str())),
            )
            .collect::<Vec<_>>();
        actions.sort_by_key(|(at, order, _)| (*at, *order));
        for (at_ms, order, name) in actions {
            let due = origin + Duration::from_millis(at_ms);
            if let Some(remaining) = due.checked_duration_since(Instant::now()) {
                thread::sleep(remaining);
            }
            if order == 0 {
                let input = recording
                    .inputs
                    .iter()
                    .find(|i| i.id == name)
                    .context("missing input")?;
                let actual_ms = origin.elapsed().as_millis() as u64;
                ensure!(
                    actual_ms <= at_ms + 250,
                    "physical input {} missed functional deadline by {} ms",
                    name,
                    actual_ms - at_ms
                );
                self.controller
                    .game()?
                    .physical_events(std::slice::from_ref(&input.event))?;
                delivered.push(DeliveredInput {
                    id: name.into(),
                    planned_ms: at_ms,
                    actual_ms,
                    event: input.event.clone(),
                });
            } else {
                let capture_start_ms = origin.elapsed().as_millis() as u64;
                let actor = self.controller.game()?.observe_main_actor()?;
                let state = self.controller.game()?.state()?;
                ensure!(
                    state["phase"] == 9 && state["level"] == "manoir",
                    "game left the scenario level: {state}"
                );
                let capture_end_ms = origin.elapsed().as_millis() as u64;
                samples.push(ActorSample::from_actor(
                    name.into(),
                    at_ms,
                    capture_start_ms,
                    capture_end_ms,
                    actor,
                )?);
            }
        }
        let screenshot = self.controller.game()?.screenshot()?;
        let checks = assertions(recording.scenario, &samples)?;
        let status = if checks.iter().all(|c| c.passed) {
            "passed"
        } else {
            "failed"
        };
        let result = BackendResult {
            backend: "openspace".into(),
            scenario: recording.scenario,
            status: status.into(),
            recording_sha256: recording.sha256()?,
            recording,
            executable_sha256: Some(self.config.exe_sha256.clone()),
            save_sha256: Some(self.config.save_sha256.clone()),
            loaded_level: Some("manoir".into()),
            setup_interventions: self.setup_interventions.clone(),
            input_delivery: "private_x11_xtest_physical_edges".into(),
            observation_consistency: "stopped_bytes_no_completed_update_claim".into(),
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
                OriginalReadyState::TurnedLeft {
                    settled: result.samples[4].clone(),
                    recording_sha256: result.recording_sha256.clone(),
                }
            } else {
                OriginalReadyState::Finished
            };
        Ok(result)
    }

    pub fn stop(&mut self) -> Result<()> {
        self.controller.stop()?;
        Config::load(&self.config_path()?)?;
        Ok(())
    }

    fn config_path(&self) -> Result<std::path::PathBuf> {
        // The game installation's private configuration sits beside its prefix.
        Ok(self
            .config
            .prefix
            .parent()
            .context("prefix parent")?
            .join("config.json"))
    }
}
