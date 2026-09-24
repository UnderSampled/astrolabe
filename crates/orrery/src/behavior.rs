//! Functional behavior evidence shared by the OpenSpace and future Godot
//! backends. Milliseconds are functional elapsed-time targets, never the
//! verified consumer ordinals used by `replay`. Original jump recordings
//! subtract measured stopped-capture intervals from that elapsed time.
use crate::replay::{Edge, Event, Key};
use anyhow::{Context, Result, ensure};
use astrolabe::{
    model::ai::DesignerValue,
    openspace::level::live::{ActorObservation, ObservedDesignerValue},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    thread,
    time::{Duration, Instant},
};

pub mod godot;
mod jump;
mod magick;
mod moving_jump;
mod sword;
mod wall;
mod wall_jump;

#[cfg(target_os = "linux")]
use crate::{config::Config, orrery::NativeController};

pub const IDLE_ACTION: u32 = 47;
pub const TURN_LEFT_ACTION: u32 = 35;
pub const WALK_FORWARD_ACTION: u32 = 32;
pub const WALK_START_ACTION: u32 = 78;
pub const RUN_FORWARD_ACTION: u32 = 33;
pub const RUN_START_ACTION: u32 = 79;
pub const ORIGINAL_STARTUP_SETTLE_MS: u64 = 2_500;
const CLEAR_ROUTE_TARGET_HEADING_RAD: f64 = -1.77;
const CLEAR_ROUTE_HEADING_TOLERANCE_RAD: f64 = 0.035;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scenario {
    TurnReleaseIdle,
    WalkReleaseStop,
    RunVsWalk,
    WallContact,
    StationaryJump,
    MovingJump,
    WallJump,
    SwordAttack,
    FireCast,
}
impl Scenario {
    pub fn slug(self) -> &'static str {
        match self {
            Self::TurnReleaseIdle => "turn-release-idle",
            Self::WalkReleaseStop => "walk-release-stop",
            Self::RunVsWalk => "run-vs-walk",
            Self::WallContact => "wall-contact",
            Self::StationaryJump => "stationary-jump",
            Self::MovingJump => "moving-jump",
            Self::WallJump => "wall-jump",
            Self::SwordAttack => "sword-attack",
            Self::FireCast => "fire-cast",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimedInput {
    pub at_ms: u64,
    pub id: String,
    pub event: Event,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SamplePoint {
    pub at_ms: u64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrialRecording {
    pub inputs: Vec<TimedInput>,
    pub samples: Vec<SamplePoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionalRecording {
    pub schema: u32,
    pub scenario: Scenario,
    pub ready_state: String,
    pub inputs: Vec<TimedInput>,
    pub samples: Vec<SamplePoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preparatory_trial: Option<TrialRecording>,
}
impl FunctionalRecording {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.schema == 1, "unsupported functional recording schema");
        let expected_ready_state = match self.scenario {
            Scenario::TurnReleaseIdle => "supplied-english-save-manoir",
            Scenario::WalkReleaseStop | Scenario::RunVsWalk | Scenario::MovingJump => {
                "manoir-clear-route-after-physical-left-turn"
            }
            Scenario::WallContact => "manoir-corner-after-physical-left-turn",
            Scenario::WallJump => "manoir-grounded-near-verified-corner-wall",
            Scenario::StationaryJump | Scenario::SwordAttack | Scenario::FireCast => {
                "manoir-clear-ground-after-physical-down"
            }
        };
        ensure!(
            self.ready_state == expected_ready_state,
            "unknown ready state"
        );
        if self.scenario == Scenario::WallContact {
            return wall::validate(self);
        }
        if self.scenario == Scenario::StationaryJump {
            return jump::validate(self);
        }
        if self.scenario == Scenario::MovingJump {
            return moving_jump::validate(self);
        }
        if self.scenario == Scenario::WallJump {
            return wall_jump::validate(self);
        }
        if self.scenario == Scenario::SwordAttack {
            return sword::validate(self);
        }
        if self.scenario == Scenario::FireCast {
            return magick::validate(self);
        }
        if self.scenario == Scenario::RunVsWalk {
            let walk = self
                .preparatory_trial
                .as_ref()
                .context("run comparison lacks its walk trial")?;
            validate_trial(&walk.inputs, &walk.samples, &[Key::ArrowUp])?;
            validate_trial(&self.inputs, &self.samples, &[Key::LeftShift, Key::ArrowUp])?;
            ensure!(
                walk.inputs[1].at_ms - walk.inputs[0].at_ms
                    == self.inputs[2].at_ms - self.inputs[0].at_ms,
                "walk/run held durations differ"
            );
            return Ok(());
        }
        ensure!(
            self.preparatory_trial.is_none(),
            "ordinary movement has an unexpected preparatory trial"
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
    if scenario == Scenario::StationaryJump {
        return jump::recording();
    }
    if scenario == Scenario::MovingJump {
        return moving_jump::recording();
    }
    if scenario == Scenario::WallJump {
        return wall_jump::recording();
    }
    if scenario == Scenario::SwordAttack {
        return sword::recording();
    }
    if scenario == Scenario::FireCast {
        return magick::recording();
    }
    if scenario == Scenario::WallContact {
        return wall::recording();
    }
    if scenario == Scenario::RunVsWalk {
        let samples = [
            (0, "baseline"),
            (450, "held"),
            (600, "released"),
            (1_800, "settled_start"),
            (2_200, "settled_end"),
        ]
        .into_iter()
        .map(|(at_ms, name)| SamplePoint {
            at_ms,
            name: name.into(),
        })
        .collect::<Vec<_>>();
        let edge = |at_ms, id: &str, key, edge| TimedInput {
            at_ms,
            id: id.into(),
            event: Event {
                keyboard: 0,
                key,
                edge,
            },
        };
        return FunctionalRecording {
            schema: 1,
            scenario,
            ready_state: "manoir-clear-route-after-physical-left-turn".into(),
            preparatory_trial: Some(TrialRecording {
                inputs: vec![
                    edge(200, "walk_press", Key::ArrowUp, Edge::Press),
                    edge(500, "walk_release", Key::ArrowUp, Edge::Release),
                ],
                samples: samples.clone(),
            }),
            inputs: vec![
                edge(200, "run_shift_press", Key::LeftShift, Edge::Press),
                edge(200, "run_up_press", Key::ArrowUp, Edge::Press),
                edge(500, "run_up_release", Key::ArrowUp, Edge::Release),
                edge(500, "run_shift_release", Key::LeftShift, Edge::Release),
            ],
            samples,
        };
    }
    let (key, held_ms, release_ms, settled_start, settled_end) = match scenario {
        Scenario::TurnReleaseIdle => (Key::ArrowLeft, 600, 700, 2_500, 2_900),
        Scenario::WalkReleaseStop => (Key::ArrowUp, 450, 900, 2_200, 2_600),
        Scenario::RunVsWalk => unreachable!(),
        Scenario::WallContact => unreachable!(),
        Scenario::StationaryJump => unreachable!(),
        Scenario::MovingJump => unreachable!(),
        Scenario::WallJump => unreachable!(),
        Scenario::SwordAttack => unreachable!(),
        Scenario::FireCast => unreachable!(),
    };
    FunctionalRecording {
        schema: 1,
        scenario,
        ready_state: match scenario {
            Scenario::TurnReleaseIdle => "supplied-english-save-manoir",
            Scenario::WalkReleaseStop => "manoir-clear-route-after-physical-left-turn",
            Scenario::RunVsWalk => unreachable!(),
            Scenario::WallContact => unreachable!(),
            Scenario::StationaryJump => unreachable!(),
            Scenario::MovingJump => unreachable!(),
            Scenario::WallJump => unreachable!(),
            Scenario::SwordAttack => unreachable!(),
            Scenario::FireCast => unreachable!(),
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
        preparatory_trial: None,
    }
}

fn validate_trial(inputs: &[TimedInput], samples: &[SamplePoint], keys: &[Key]) -> Result<()> {
    ensure!(
        inputs.len() == keys.len() * 2,
        "trial key edge count differs"
    );
    for key in keys {
        ensure!(
            inputs
                .iter()
                .filter(|input| input.event.key == *key && input.event.edge == Edge::Press)
                .count()
                == 1
                && inputs
                    .iter()
                    .filter(|input| input.event.key == *key && input.event.edge == Edge::Release)
                    .count()
                    == 1,
            "trial is missing a key press or release"
        );
    }
    ensure!(
        inputs.iter().all(|input| input.event.keyboard == 0),
        "trial keyboard changed"
    );
    ensure!(
        inputs
            .iter()
            .filter(|input| input.event.edge == Edge::Press)
            .all(|input| input.at_ms == 200)
            && inputs
                .iter()
                .filter(|input| input.event.edge == Edge::Release)
                .all(|input| input.at_ms == 500),
        "trial hold schedule changed"
    );
    ensure!(
        samples
            .iter()
            .map(|point| (point.at_ms, point.name.as_str()))
            .eq([
                (0, "baseline"),
                (450, "held"),
                (600, "released"),
                (1_800, "settled_start"),
                (2_200, "settled_end")
            ]),
        "trial observation schedule changed"
    );
    Ok(())
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
fn heading_error_to(target: f64, actual: f64) -> f64 {
    let difference = target - actual;
    difference.sin().atan2(difference.cos())
}
fn correction_hold_ms(error: f64) -> u64 {
    ((error.abs() / 3.0) * 1_000.0).round().clamp(50.0, 100.0) as u64
}
fn action_is(sample: &ActorSample, action: u32) -> f64 {
    f64::from(u8::from(sample.action == Some(action)))
}
fn channel_active(sample: &ActorSample, channel: u32) -> Option<bool> {
    let key = channel.to_string();
    let visible = sample.raw_observation["channel_visibility"][key.as_str()].as_array()?;
    (visible.len() == 1).then(|| visible[0].as_bool()).flatten()
}
fn walking_action_is_active(sample: &ActorSample) -> f64 {
    // The source normal4 rule requests action 78 before switching to normal5;
    // normal5 then requests action 32. Both are observed during held Up input.
    f64::from(u8::from(matches!(
        sample.action,
        Some(WALK_START_ACTION | WALK_FORWARD_ACTION)
    )))
}

fn running_action_is_active(sample: &ActorSample) -> f64 {
    // Hype's source normal4 rule requests action 79 on entering the run;
    // normal6 changes it to sustained action 33 when the start action ends.
    f64::from(u8::from(matches!(
        sample.action,
        Some(RUN_START_ACTION | RUN_FORWARD_ACTION)
    )))
}

/// Independently sourced functional bounds. The 2026-09-23 Orrery manor
/// observation saw about 80 degrees/500 ms Left and 4.1 source units/1.8 s Up.
/// These broad limits classify motion without claiming timing or pixel parity.
pub fn assertions(scenario: Scenario, samples: &[ActorSample]) -> Result<Vec<Assertion>> {
    if scenario == Scenario::StationaryJump {
        return jump::assertions(samples);
    }
    if scenario == Scenario::MovingJump {
        return moving_jump::assertions(samples);
    }
    if scenario == Scenario::WallJump {
        return wall_jump::assertions(samples);
    }
    if scenario == Scenario::SwordAttack {
        return sword::assertions(samples);
    }
    if scenario == Scenario::FireCast {
        return magick::assertions(samples);
    }
    if scenario == Scenario::WallContact {
        return wall::assertions(samples);
    }
    if scenario == Scenario::RunVsWalk {
        return run_comparison_assertions(samples);
    }
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
            match scenario {
                Scenario::TurnReleaseIdle => action_is(held, TURN_LEFT_ACTION),
                Scenario::WalkReleaseStop => walking_action_is_active(held),
                Scenario::RunVsWalk => unreachable!(),
                Scenario::WallContact => unreachable!(),
                Scenario::StationaryJump => unreachable!(),
                Scenario::MovingJump => unreachable!(),
                Scenario::WallJump => unreachable!(),
                Scenario::SwordAttack => unreachable!(),
                Scenario::FireCast => unreachable!(),
            },
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
            if baseline.raw_observation.get("channel_visibility").is_some() {
                let source = "Orrery 2026-09-24 supplied save: channels 28, 71 and 80 inactive; 72, 116 and 136 active before and after a short walk";
                for sample in [baseline, held, settled_end] {
                    checks.push(bound(
                        &format!("walk_saved_weapon_channels_{}", sample.name),
                        f64::from(u8::from(
                            [28, 71, 80]
                                .into_iter()
                                .all(|channel| channel_active(sample, channel) == Some(false))
                                && [72, 116, 136]
                                    .into_iter()
                                    .all(|channel| channel_active(sample, channel) == Some(true)),
                        )),
                        Some(1.0),
                        Some(1.0),
                        source,
                    ));
                }
            }
        }
        Scenario::RunVsWalk => unreachable!(),
        Scenario::WallContact => unreachable!(),
        Scenario::StationaryJump => unreachable!(),
        Scenario::MovingJump => unreachable!(),
        Scenario::WallJump => unreachable!(),
        Scenario::SwordAttack => unreachable!(),
        Scenario::FireCast => unreachable!(),
    }
    Ok(checks)
}

fn run_comparison_assertions(samples: &[ActorSample]) -> Result<Vec<Assertion>> {
    ensure!(
        samples.len() == 10,
        "walk/run comparison requires ten samples"
    );
    for (offset, prefix) in [(0, "walk"), (5, "run")] {
        for (index, name) in [
            "baseline",
            "held",
            "released",
            "settled_start",
            "settled_end",
        ]
        .iter()
        .enumerate()
        {
            ensure!(
                samples[offset + index].name == format!("{prefix}_{name}"),
                "walk/run sample order changed"
            );
        }
    }
    let walk = &samples[..5];
    let run = &samples[5..];
    let walk_distance = distance_xy(&walk[0], &walk[4]);
    let run_distance = distance_xy(&run[0], &run[4]);
    let route_length = walk_distance.max(f64::EPSILON);
    let route_x = f64::from(walk[4].translation[0] - walk[0].translation[0]) / route_length;
    let route_y = f64::from(walk[4].translation[1] - walk[0].translation[1]) / route_length;
    let run_dx = f64::from(run[4].translation[0] - run[0].translation[0]);
    let run_dy = f64::from(run[4].translation[1] - run[0].translation[1]);
    let run_forward = run_dx * route_x + run_dy * route_y;
    let run_lateral = (run_dx * route_y - run_dy * route_x).abs();
    let source = "Orrery 2026-09-24 supplied manor save, equal 300 ms physical Up / Shift+Up trials from a reloaded clear-route start; settled 1.095 and 2.521 source units";
    Ok(vec![
        bound(
            "walk_baseline_idle",
            action_is(&walk[0], IDLE_ACTION),
            Some(1.),
            Some(1.),
            source,
        ),
        bound(
            "run_baseline_idle",
            action_is(&run[0], IDLE_ACTION),
            Some(1.),
            Some(1.),
            source,
        ),
        bound(
            "walk_active_action",
            walking_action_is_active(&walk[1]),
            Some(1.),
            Some(1.),
            source,
        ),
        bound(
            "run_active_action",
            running_action_is_active(&run[1]),
            Some(1.),
            Some(1.),
            source,
        ),
        bound(
            "walk_settled_idle",
            action_is(&walk[4], IDLE_ACTION),
            Some(1.),
            Some(1.),
            source,
        ),
        bound(
            "run_settled_idle",
            action_is(&run[4], IDLE_ACTION),
            Some(1.),
            Some(1.),
            source,
        ),
        bound(
            "equivalent_start_xy",
            distance_xy(&walk[0], &run[0]),
            None,
            Some(0.08),
            source,
        ),
        bound(
            "equivalent_heading_rad",
            heading_delta(&walk[0], &run[0]),
            None,
            Some(0.08),
            source,
        ),
        bound(
            "walk_distance_xy",
            walk_distance,
            Some(0.4),
            Some(1.8),
            source,
        ),
        bound(
            "run_distance_xy",
            run_distance,
            Some(1.2),
            Some(3.3),
            source,
        ),
        bound(
            "run_forward_distance",
            run_forward,
            Some(1.2),
            Some(3.3),
            source,
        ),
        bound(
            "run_to_walk_ratio",
            run_forward / route_length,
            Some(1.5),
            Some(3.5),
            source,
        ),
        bound("run_lateral_distance", run_lateral, None, Some(0.3), source),
        bound(
            "walk_post_release_drift",
            distance_xy(&walk[3], &walk[4]),
            None,
            Some(0.4),
            source,
        ),
        bound(
            "run_post_release_drift",
            distance_xy(&run[3], &run[4]),
            None,
            Some(0.4),
            source,
        ),
        bound(
            "walk_height_change",
            f64::from((walk[4].translation[2] - walk[0].translation[2]).abs()),
            None,
            Some(0.3),
            source,
        ),
        bound(
            "run_height_change",
            f64::from((run[4].translation[2] - run[0].translation[2]).abs()),
            None,
            Some(0.3),
            source,
        ),
    ])
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
    let expected_samples = match openspace.scenario {
        Scenario::TurnReleaseIdle | Scenario::WalkReleaseStop => 5,
        Scenario::RunVsWalk => 10,
        Scenario::WallContact => 7,
        Scenario::StationaryJump => 7,
        Scenario::MovingJump => 8,
        Scenario::WallJump => 8,
        Scenario::SwordAttack => 6,
        Scenario::FireCast => 6,
    };
    ensure!(
        openspace.samples.len() == godot.samples.len()
            && openspace.samples.len() == expected_samples,
        "comparison sample coverage differs"
    );
    let mut checks = vec![];
    for (index, (source, target)) in openspace.samples.iter().zip(&godot.samples).enumerate() {
        ensure!(
            source.name == target.name,
            "comparison sample order differs"
        );
        let held_walk_phase = match openspace.scenario {
            Scenario::WalkReleaseStop => source.name == "held",
            Scenario::RunVsWalk => source.name == "walk_held",
            Scenario::MovingJump | Scenario::WallJump => source.name == "walking",
            _ => false,
        };
        if openspace.scenario == Scenario::SwordAttack && source.name == "attack_early" {
            checks.push(bound(
                "sword_attack_early_phase_compatible",
                f64::from(u8::from(
                    matches!(source.action, Some(135 | 134))
                        && matches!(target.action, Some(135 | 134)),
                )),
                Some(1.),
                Some(1.),
                "Hype normal 29 executes consecutive sword actions 135 then 134",
            ));
        } else if openspace.scenario == Scenario::SwordAttack && source.name == "attack_late" {
            checks.push(bound(
                "sword_attack_late_phase_compatible",
                f64::from(u8::from(
                    matches!(source.action, Some(134 | 135 | 48))
                        && matches!(target.action, Some(134 | 135 | 48)),
                )),
                Some(1.),
                Some(1.),
                "the source swing may still play or finish at this sampled transition",
            ));
        } else if openspace.scenario == Scenario::FireCast && source.name == "armed" {
            checks.push(bound(
                "armed_selection_phase_compatible",
                f64::from(u8::from(
                    matches!(source.action, Some(45 | 48))
                        && matches!(target.action, Some(45 | 48)),
                )),
                Some(1.),
                Some(1.),
                "source sword selection is action 45 or armed idle 48 at this checkpoint",
            ));
        } else if openspace.scenario == Scenario::FireCast
            && matches!(source.name.as_str(), "cast_early" | "cast_late")
        {
            checks.push(bound(
                &format!("{}_phase_compatible", source.name),
                f64::from(u8::from(
                    matches!(source.action, Some(48 | 290 | 271))
                        && matches!(target.action, Some(48 | 290 | 271)),
                )),
                Some(1.),
                Some(1.),
                "the cast action may enter between the bounded samples; both backends must independently observe it",
            ));
        } else if openspace.scenario == Scenario::FireCast && source.name == "settled" {
            checks.push(bound(
                "settled_cast_phase_compatible",
                f64::from(u8::from(
                    matches!(source.action, Some(48 | 271))
                        && matches!(target.action, Some(48 | 271)),
                )),
                Some(1.),
                Some(1.),
                "source cast follow-through 271 may end in armed idle 48 by the settled checkpoint",
            ));
        } else if held_walk_phase {
            // The source 78 entry action and 32 sustained action are both
            // active walking at this elapsed-time checkpoint. Exact IDs stay
            // in the backend reports; consumption-frame parity is not claimed.
            let check_name = match openspace.scenario {
                Scenario::RunVsWalk => "walk_held_phase_compatible",
                Scenario::MovingJump | Scenario::WallJump => "moving_jump_walking_phase_compatible",
                _ => "held_walk_phase_compatible",
            };
            checks.push(bound(
                check_name,
                walking_action_is_active(source) * walking_action_is_active(target),
                Some(1.),
                Some(1.),
                "source normal 4 walk entry (78) leads to normal 5 walk (32)",
            ));
        } else {
            checks.push(bound(
                &format!("{}_action_match", source.name),
                f64::from(u8::from(source.action == target.action)),
                Some(1.),
                Some(1.),
                "canonical action identity at the same named checkpoint",
            ));
        }
        if openspace.scenario == Scenario::FireCast
            && matches!(
                source.name.as_str(),
                "baseline" | "armed" | "spent" | "settled"
            )
        {
            let source_mana = source.raw_observation["magick"]["current_points"].as_i64();
            let target_mana = target.raw_observation["magick"]["current_points"].as_i64();
            checks.push(bound(
                &format!("{}_mana_match", source.name),
                f64::from(u8::from(
                    source_mana.is_some() && source_mana == target_mana,
                )),
                Some(1.),
                Some(1.),
                "source and Godot mana at the named cast checkpoint",
            ));
        }
        let origin = if openspace.scenario == Scenario::RunVsWalk && index >= 5 {
            5
        } else {
            0
        };
        let sx = source.translation[0] - openspace.samples[origin].translation[0];
        let sy = source.translation[1] - openspace.samples[origin].translation[1];
        let tx = target.translation[0] - godot.samples[origin].translation[0];
        let ty = target.translation[1] - godot.samples[origin].translation[1];
        let path_error = f64::from(sx - tx).hypot(f64::from(sy - ty));
        checks.push(bound(
            &format!("{}_relative_xy_error", source.name),
            path_error,
            None,
            Some(1.0),
            "functional path tolerance; source-relative positions in canonical units",
        ));
        if matches!(
            openspace.scenario,
            Scenario::StationaryJump | Scenario::MovingJump | Scenario::WallJump
        ) {
            let source_height = source.translation[2] - openspace.samples[origin].translation[2];
            let target_height = target.translation[2] - godot.samples[origin].translation[2];
            checks.push(bound(
                &format!("{}_relative_height_error", source.name),
                f64::from((source_height - target_height).abs()),
                None,
                Some(1.0),
                "coarse sampled jump height; apex and timing are not established",
            ));
        }
        let source_yaw = source.heading_rad - openspace.samples[origin].heading_rad;
        let target_yaw = target.heading_rad - godot.samples[origin].heading_rad;
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
        timing_profile: if matches!(
            openspace.scenario,
            Scenario::StationaryJump
                | Scenario::MovingJump
                | Scenario::WallJump
                | Scenario::SwordAttack
                | Scenario::FireCast
        ) {
            "pause_compensated_functional_elapsed_ms_no_wall_or_consumption_parity"
        } else {
            "functional_elapsed_ms_no_consumption_parity"
        }
        .into(),
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
        let planned_inputs = self
            .recording
            .preparatory_trial
            .iter()
            .flat_map(|trial| &trial.inputs)
            .chain(&self.recording.inputs)
            .collect::<Vec<_>>();
        ensure!(
            self.delivered.len() == planned_inputs.len(),
            "backend input delivery is incomplete"
        );
        for (delivered, planned) in self.delivered.iter().zip(planned_inputs) {
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
        let planned_samples = self
            .recording
            .preparatory_trial
            .iter()
            .flat_map(|trial| trial.samples.iter().map(|point| ("walk_", point)))
            .chain(self.recording.samples.iter().map(|point| {
                (
                    if self.scenario == Scenario::RunVsWalk {
                        "run_"
                    } else {
                        ""
                    },
                    point,
                )
            }))
            .collect::<Vec<_>>();
        ensure!(
            self.samples.len() == planned_samples.len(),
            "backend sample coverage is incomplete"
        );
        for (sample, (prefix, planned)) in self.samples.iter().zip(planned_samples) {
            ensure!(
                sample.name == format!("{prefix}{}", planned.name)
                    && sample.planned_ms == planned.at_ms,
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
        let settled_windows: &[(usize, usize)] =
            if matches!(self.scenario, Scenario::SwordAttack | Scenario::FireCast) {
                &[(4, 5)]
            } else if matches!(self.scenario, Scenario::MovingJump | Scenario::WallJump) {
                &[(6, 7)]
            } else if matches!(
                self.scenario,
                Scenario::StationaryJump | Scenario::WallContact
            ) {
                &[(5, 6)]
            } else if self.scenario == Scenario::RunVsWalk {
                &[(3, 4), (8, 9)]
            } else {
                &[(3, 4)]
            };
        for &(start, end) in settled_windows {
            ensure!(
                self.samples[end]
                    .capture_start_ms
                    .saturating_sub(self.samples[start].capture_end_ms)
                    >= 300,
                "settled observation window was shortened by late sampling"
            );
        }
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
    initial_actor: ActorSample,
    setup_interventions: Vec<serde_json::Value>,
    ready_state: OriginalReadyState,
}

#[cfg(target_os = "linux")]
enum OriginalReadyState {
    FreshSave,
    ClearRoute,
    WallRoute,
    WallJumpStart,
    JumpStart,
    TurnedLeft {
        settled: ActorSample,
        recording_sha256: String,
    },
    Finished,
}

#[cfg(target_os = "linux")]
impl OriginalBackend {
    pub fn prepare_stationary_jump(&mut self) -> Result<()> {
        let reload = if matches!(self.ready_state, OriginalReadyState::Finished) {
            Some(self.reload_saved_state()?)
        } else {
            None
        };
        ensure!(
            matches!(self.ready_state, OriginalReadyState::FreshSave),
            "jump setup requires supplied save"
        );
        self.setup_interventions.clear();
        if let Some(reload) = reload {
            self.setup_interventions.push(reload);
        }
        let before = ActorSample::from_actor(
            "jump_setup_before".into(),
            0,
            0,
            0,
            self.controller.game()?.observe_main_actor()?,
        )?;
        self.controller.game()?.prepare_physical_input()?;
        let started = Instant::now();
        self.controller.game()?.physical_events(&[Event {
            keyboard: 0,
            key: Key::ArrowDown,
            edge: Edge::Press,
        }])?;
        thread::sleep(Duration::from_millis(800));
        self.controller.game()?.physical_events(&[Event {
            keyboard: 0,
            key: Key::ArrowDown,
            edge: Edge::Release,
        }])?;
        let release_ms = started.elapsed().as_millis() as u64;
        thread::sleep(Duration::from_millis(700));
        let after = ActorSample::from_actor(
            "jump_setup_after".into(),
            0,
            0,
            0,
            self.controller.game()?.observe_main_actor()?,
        )?;
        let moved = distance_xy(&before, &after);
        ensure!(
            after.action == Some(IDLE_ACTION)
                && (0.5..=3.5).contains(&moved)
                && heading_delta(&before, &after) < 0.1
                && (after.translation[2] - before.translation[2]).abs() < 0.15,
            "physical Down setup did not reach clear grounded idle: {after:?}"
        );
        self.setup_interventions.push(serde_json::json!({
            "method":"physical_down_to_clear_jump_start", "planned_hold_ms":800,
            "release_ms":release_ms,"settle_ms":700,"before":before,"after":after,
        }));
        self.ready_state = OriginalReadyState::JumpStart;
        Ok(())
    }
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
        let initial_actor =
            ActorSample::from_actor("supplied_save_start".into(), 0, 0, 0, settled_actor)?;
        Ok(Self {
            controller,
            config,
            initial_actor,
            setup_interventions: vec![],
            ready_state: OriginalReadyState::FreshSave,
        })
    }

    fn reload_saved_state(&mut self) -> Result<serde_json::Value> {
        let before = ActorSample::from_actor(
            "before_same_process_reload".into(),
            0,
            0,
            0,
            self.controller.game()?.observe_main_actor()?,
        )?;
        self.controller.game()?.keys(&["Escape".into()], 100, 300)?;
        let pause_deadline = Instant::now() + Duration::from_secs(8);
        let pause_state = loop {
            let state = self.controller.game()?.state()?;
            if state["phase"] == 16 {
                break state;
            }
            ensure!(
                Instant::now() < pause_deadline,
                "pause menu did not open: {state}"
            );
            thread::sleep(Duration::from_millis(100));
        };
        let exit_keys = ["Down", "Down", "Down", "Return", "Left", "Return"].map(str::to_owned);
        for key in &exit_keys[..4] {
            self.controller
                .game()?
                .keys(std::slice::from_ref(key), 100, 700)?;
        }
        thread::sleep(Duration::from_millis(500));
        self.controller.game()?.keys(&exit_keys[4..5], 100, 700)?;
        self.controller.game()?.keys(&exit_keys[5..], 100, 700)?;
        let title_deadline = Instant::now() + Duration::from_secs(12);
        let title_state = loop {
            let state = self.controller.game()?.state()?;
            if state["phase"] == 38 {
                break state;
            }
            ensure!(
                Instant::now() < title_deadline,
                "title menu did not open: {state}"
            );
            thread::sleep(Duration::from_millis(100));
        };
        let load_keys = ["Down", "Return", "Return"].map(str::to_owned);
        self.controller.game()?.keys(&load_keys, 100, 300)?;
        let deadline = Instant::now() + Duration::from_secs(35);
        loop {
            let state = self.controller.game()?.state()?;
            if state["phase"] == 9 && state["level"] == "manoir" {
                break;
            }
            ensure!(
                Instant::now() < deadline,
                "same-process save reload failed: {state}"
            );
            thread::sleep(Duration::from_millis(100));
        }
        let idle_deadline = Instant::now() + Duration::from_secs(12);
        loop {
            let action = self.controller.game()?.observe_main_actor()?.current_action;
            if action == Some(IDLE_ACTION) {
                break;
            }
            ensure!(
                Instant::now() < idle_deadline,
                "reloaded manor actor did not become idle: {action:?}"
            );
            thread::sleep(Duration::from_millis(100));
        }
        thread::sleep(Duration::from_millis(ORIGINAL_STARTUP_SETTLE_MS));
        let state = self.controller.game()?.state()?;
        let after = ActorSample::from_actor(
            "after_same_process_reload".into(),
            0,
            0,
            0,
            self.controller.game()?.observe_main_actor()?,
        )?;
        ensure!(
            state["phase"] == 9
                && state["level"] == "manoir"
                && after.action == Some(IDLE_ACTION)
                && distance_xy(&self.initial_actor, &after) < 0.08
                && heading_delta(&self.initial_actor, &after) < 0.08
                && (after.translation[2] - self.initial_actor.translation[2]).abs() < 0.05,
            "same-process save reload did not restore the supplied start: {after:?}"
        );
        self.ready_state = OriginalReadyState::FreshSave;
        Ok(serde_json::json!({
            "method": "same_process_supplied_save_reload",
            "exit_chords": exit_keys,
            "load_chords": load_keys,
            "pause_state": pause_state,
            "title_state": title_state,
            "menu_hold_ms": 100,
            "menu_gap_ms": 700,
            "startup_settle_ms": ORIGINAL_STARTUP_SETTLE_MS,
            "before": before,
            "after": after,
            "state": state,
        }))
    }

    /// The supplied save faces a barred gate. This documented Given turns the
    /// actor toward the clear manor route by actual input before the walking
    /// tape starts; it does not change actor state or position in memory.
    pub fn face_clear_path(&mut self) -> Result<()> {
        let reload = if matches!(self.ready_state, OriginalReadyState::Finished) {
            Some(self.reload_saved_state()?)
        } else {
            None
        };
        self.setup_interventions.clear();
        if let Some(reload) = reload {
            self.setup_interventions.push(reload);
        }
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
        let mut after = ActorSample::from_actor("setup_after".into(), 0, 0, 0, after)?;
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
            self.controller.game()?.physical_events(&[Event {
                keyboard: 0,
                key,
                edge: Edge::Press,
            }])?;
            thread::sleep(Duration::from_millis(hold_ms));
            self.controller.game()?.physical_events(&[Event {
                keyboard: 0,
                key,
                edge: Edge::Release,
            }])?;
            let actual_hold_ms = pulse.elapsed().as_millis() as u64;
            thread::sleep(Duration::from_millis(1_800));
            after = ActorSample::from_actor(
                "setup_corrected".into(),
                0,
                0,
                0,
                self.controller.game()?.observe_main_actor()?,
            )?;
            corrections.push(serde_json::json!({
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
            "clear-path setup did not produce target idle turn: action {:?}, heading {heading_change}, final yaw {}, target {}, translation {translation}",
            after.action,
            after.heading_rad,
            CLEAR_ROUTE_TARGET_HEADING_RAD,
        );
        self.setup_interventions.push(serde_json::json!({
            "reason": "supplied manor save faces a barred gate; orient toward measured clear route",
            "method": "private XTest physical ArrowLeft press and release",
            "press_ms": press_ms, "release_ms": release_ms,
            "settle_ms": 1_800, "heading_change_rad": heading_change,
            "heading_target_rad": CLEAR_ROUTE_TARGET_HEADING_RAD,
            "heading_tolerance_rad": CLEAR_ROUTE_HEADING_TOLERANCE_RAD,
            "corrections": corrections,
            "translation_xy": translation,
            "before": before, "after": after,
        }));
        self.ready_state = OriginalReadyState::ClearRoute;
        Ok(())
    }

    pub fn face_wall(&mut self) -> Result<()> {
        let reload = if matches!(self.ready_state, OriginalReadyState::Finished) {
            Some(self.reload_saved_state()?)
        } else {
            None
        };
        ensure!(
            matches!(self.ready_state, OriginalReadyState::FreshSave),
            "corner setup requires the supplied save"
        );
        self.setup_interventions.clear();
        if let Some(reload) = reload {
            self.setup_interventions.push(reload);
        }
        self.setup_interventions.push(wall::oracle());
        let before = ActorSample::from_actor(
            "wall_setup_before".into(),
            0,
            0,
            0,
            self.controller.game()?.observe_main_actor()?,
        )?;
        self.controller.game()?.prepare_physical_input()?;
        let origin = Instant::now();
        self.controller.game()?.physical_events(&[Event {
            keyboard: 0,
            key: Key::ArrowLeft,
            edge: Edge::Press,
        }])?;
        thread::sleep(Duration::from_millis(250));
        self.controller.game()?.physical_events(&[Event {
            keyboard: 0,
            key: Key::ArrowLeft,
            edge: Edge::Release,
        }])?;
        let release_ms = origin.elapsed().as_millis() as u64;
        thread::sleep(Duration::from_millis(1_800));
        let mut after = ActorSample::from_actor(
            "wall_setup_after".into(),
            0,
            0,
            0,
            self.controller.game()?.observe_main_actor()?,
        )?;
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
            self.controller.game()?.physical_events(&[Event {
                keyboard: 0,
                key,
                edge: Edge::Press,
            }])?;
            thread::sleep(Duration::from_millis(hold_ms));
            self.controller.game()?.physical_events(&[Event {
                keyboard: 0,
                key,
                edge: Edge::Release,
            }])?;
            let actual_hold_ms = pulse.elapsed().as_millis() as u64;
            thread::sleep(Duration::from_millis(1_800));
            after = ActorSample::from_actor(
                "wall_setup_corrected".into(),
                0,
                0,
                0,
                self.controller.game()?.observe_main_actor()?,
            )?;
            corrections.push(serde_json::json!({
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
            "corner setup did not reach the source route: heading {heading}, final yaw {}, target {}, distance {distance}, action {:?}",
            after.heading_rad,
            wall::TARGET_HEADING_RAD,
            after.action
        );
        self.setup_interventions.push(serde_json::json!({
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
        self.ready_state = OriginalReadyState::WallRoute;
        Ok(())
    }

    pub fn prepare_wall_jump(&mut self) -> Result<()> {
        self.face_wall()?;
        let game = self.controller.game()?;
        game.prepare_physical_input()?;
        game.physical_events(&[Event {
            keyboard: 0,
            key: Key::ArrowUp,
            edge: Edge::Press,
        }])?;
        thread::sleep(Duration::from_millis(3_200));
        game.physical_events(&[Event {
            keyboard: 0,
            key: Key::ArrowUp,
            edge: Edge::Release,
        }])?;
        thread::sleep(Duration::from_millis(400));
        let contact = ActorSample::from_actor(
            "wall_jump_setup_contact".into(),
            0,
            0,
            0,
            game.observe_main_actor()?,
        )?;
        game.physical_events(&[Event {
            keyboard: 0,
            key: Key::ArrowDown,
            edge: Edge::Press,
        }])?;
        thread::sleep(Duration::from_millis(700));
        game.physical_events(&[Event {
            keyboard: 0,
            key: Key::ArrowDown,
            edge: Edge::Release,
        }])?;
        thread::sleep(Duration::from_millis(700));
        let start = ActorSample::from_actor(
            "wall_jump_setup_start".into(),
            0,
            0,
            0,
            game.observe_main_actor()?,
        )?;
        wall_jump::validate_ready(&contact, &start)?;
        self.setup_interventions.push(serde_json::json!({
            "method": "physical_up_to_corner_then_down_to_grounded_wall_jump_start",
            "up_hold_ms": 3_200,
            "contact_settle_ms": 400,
            "down_hold_ms": 700,
            "start_settle_ms": 700,
            "contact": contact,
            "start": start,
        }));
        self.ready_state = OriginalReadyState::WallJumpStart;
        Ok(())
    }

    fn execute_trial(
        &mut self,
        inputs: &[TimedInput],
        points: &[SamplePoint],
        prefix: &str,
        screenshot_at: Option<&str>,
        pause_compensated: bool,
        observe_magick: bool,
    ) -> Result<(
        Vec<DeliveredInput>,
        Vec<ActorSample>,
        Option<serde_json::Value>,
    )> {
        self.controller.game()?.prepare_physical_input()?;
        let origin = Instant::now();
        let mut paused_total = Duration::ZERO;
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
            let due = origin
                + Duration::from_millis(at_ms)
                + if pause_compensated {
                    paused_total
                } else {
                    Duration::ZERO
                };
            if let Some(remaining) = due.checked_duration_since(Instant::now()) {
                thread::sleep(remaining);
            }
            if order == 0 {
                let input = inputs
                    .iter()
                    .find(|i| i.id == name)
                    .context("missing input")?;
                let actual_ms = origin.elapsed().saturating_sub(paused_total).as_millis() as u64;
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
                let capture_start_ms =
                    origin.elapsed().saturating_sub(paused_total).as_millis() as u64;
                let (actor, paused, magick_capture) = if observe_magick {
                    let observation = self.controller.game()?.observe_magick_behavior(
                        &[47, 48, 290, 271],
                        &[59],
                        Some(2),
                    )?;
                    let paused = Duration::from_micros(u64::try_from(
                        observation.pause.pause_duration_micros,
                    )?);
                    let ready = observation
                        .actor
                        .designer_values
                        .iter()
                        .find(|value| value.index == 59)
                        .and_then(|value| match &value.value {
                            ObservedDesignerValue::Value(DesignerValue::Boolean(ready)) => {
                                Some(*ready)
                            }
                            _ => None,
                        });
                    let raw = serde_json::json!({
                        "magick": {
                            "selected_spell_code": observation.magic_selection.source_code,
                            "current_points": observation.magic_selection.current_points,
                            "maximum_points": observation.magic_selection.maximum_points,
                        },
                        "designer_fields": {"59": ready},
                        "source_ai": observation.actor,
                        "source_scene_tail": observation.scene_tail,
                        "source_capture": observation.capture,
                    });
                    (observation.actor.actor, paused, Some(raw))
                } else {
                    let (actor, paused) = self.controller.game()?.observe_main_actor_timed()?;
                    (actor, paused, None)
                };
                if pause_compensated {
                    paused_total += paused;
                }
                let state = self.controller.game()?.state()?;
                ensure!(
                    state["phase"] == 9 && state["level"] == "manoir",
                    "game left the scenario level: {state}"
                );
                let capture_end_ms =
                    origin.elapsed().saturating_sub(paused_total).as_millis() as u64;
                let mut sample = ActorSample::from_actor(
                    format!("{prefix}{name}"),
                    at_ms,
                    capture_start_ms,
                    capture_end_ms,
                    actor,
                )?;
                if let Some(capture) = magick_capture {
                    let source = sample
                        .raw_observation
                        .as_object_mut()
                        .context("actor observation must serialize as object")?;
                    source.extend(
                        capture
                            .as_object()
                            .context("magick capture must serialize as object")?
                            .clone(),
                    );
                }
                if pause_compensated {
                    sample.raw_observation["capture_pause_micros"] =
                        serde_json::json!(paused.as_micros());
                    sample.raw_observation["time_basis"] =
                        serde_json::json!("wall_elapsed_minus_stopped_capture");
                }
                samples.push(sample);
                if screenshot_at == Some(name) {
                    held_screenshot = Some(self.controller.game()?.screenshot()?);
                }
            }
        }
        Ok((delivered, samples, held_screenshot))
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
                (OriginalReadyState::FreshSave, Scenario::TurnReleaseIdle)
                    | (OriginalReadyState::ClearRoute, Scenario::WalkReleaseStop)
                    | (OriginalReadyState::JumpStart, Scenario::StationaryJump)
                    | (OriginalReadyState::ClearRoute, Scenario::MovingJump)
                    | (OriginalReadyState::WallJumpStart, Scenario::WallJump)
                    | (OriginalReadyState::JumpStart, Scenario::SwordAttack)
                    | (OriginalReadyState::JumpStart, Scenario::FireCast)
            ),
            "the selected scenario's Given setup was not established"
        );
        let pause_compensated = matches!(
            recording.scenario,
            Scenario::StationaryJump
                | Scenario::MovingJump
                | Scenario::WallJump
                | Scenario::SwordAttack
                | Scenario::FireCast
        );
        let (delivered, samples, _) = self.execute_trial(
            &recording.inputs,
            &recording.samples,
            "",
            None,
            pause_compensated,
            recording.scenario == Scenario::FireCast,
        )?;
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
            observation_consistency: if pause_compensated {
                "stopped_bytes_pause_compensated_elapsed_no_completed_update_claim"
            } else {
                "stopped_bytes_no_completed_update_claim"
            }
            .into(),
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

    fn run_comparison(&mut self, recording: FunctionalRecording) -> Result<BackendResult> {
        ensure!(
            matches!(self.ready_state, OriginalReadyState::ClearRoute),
            "run comparison needs the verified clear-route start"
        );
        let walk = recording.preparatory_trial.as_ref().unwrap();
        let initial_setup = self.setup_interventions.clone();
        let (mut delivered, mut samples, _) =
            self.execute_trial(&walk.inputs, &walk.samples, "walk_", None, false, false)?;
        let walk_screenshot = self.controller.game()?.screenshot()?;
        let reload = self.reload_saved_state()?;
        self.face_clear_path()?;
        let mut setup_interventions = initial_setup;
        setup_interventions.push(serde_json::json!({
            "method": "walk_trial_completion",
            "screenshot": walk_screenshot,
            "settled": samples.last(),
        }));
        setup_interventions.push(reload);
        setup_interventions.extend(self.setup_interventions.clone());
        let (run_delivered, run_samples, _) = self.execute_trial(
            &recording.inputs,
            &recording.samples,
            "run_",
            None,
            false,
            false,
        )?;
        delivered.extend(run_delivered);
        samples.extend(run_samples);
        let screenshot = self.controller.game()?.screenshot()?;
        let checks = assertions(recording.scenario, &samples)?;
        let status = if checks.iter().all(|check| check.passed) {
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
            setup_interventions,
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
        self.ready_state = OriginalReadyState::Finished;
        Ok(result)
    }

    fn run_wall_contact(&mut self, recording: FunctionalRecording) -> Result<BackendResult> {
        ensure!(
            matches!(self.ready_state, OriginalReadyState::WallRoute),
            "wall recording needs the verified manor corner route"
        );
        let (delivered, samples, screenshot) = self.execute_trial(
            &recording.inputs,
            &recording.samples,
            "",
            Some("contact_end"),
            false,
            false,
        )?;
        let checks = assertions(recording.scenario, &samples)?;
        let status = if checks.iter().all(|check| check.passed) {
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
            screenshot,
            cleanup_complete: false,
            error: None,
        };
        self.ready_state = OriginalReadyState::Finished;
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
