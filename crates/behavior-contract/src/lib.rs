//! Functional behavior evidence shared by the OpenSpace and future Godot
//! backends. Milliseconds are functional elapsed-time targets, never the
//! verified consumer ordinals used by `replay`. Original jump recordings
//! subtract measured stopped-capture intervals from that elapsed time.
pub mod input;
use anyhow::{Context, Result, ensure};
pub use input::{Edge, Event, Key};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub mod crossbow;
pub mod jump;
pub mod magick;
pub mod moving_jump;
pub mod sword;
pub mod wall;
pub mod wall_jump;

pub const IDLE_ACTION: u32 = 47;
pub const TURN_LEFT_ACTION: u32 = 35;
pub const WALK_FORWARD_ACTION: u32 = 32;
pub const WALK_START_ACTION: u32 = 78;
pub const RUN_FORWARD_ACTION: u32 = 33;
pub const RUN_START_ACTION: u32 = 79;
pub const CLEAR_ROUTE_TARGET_HEADING_RAD: f64 = -1.77;
pub const CLEAR_ROUTE_HEADING_TOLERANCE_RAD: f64 = 0.035;

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
    CrossbowInitialLaunch,
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
            Self::CrossbowInitialLaunch => "crossbow-initial-launch",
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
            Scenario::CrossbowInitialLaunch => "manoir-crossbow-quantity-three",
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
        if self.scenario == Scenario::CrossbowInitialLaunch {
            return crossbow::validate(self);
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
    if scenario == Scenario::CrossbowInitialLaunch {
        return crossbow::recording();
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
        Scenario::CrossbowInitialLaunch => unreachable!(),
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
            Scenario::CrossbowInitialLaunch => unreachable!(),
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

pub fn validate_trial(inputs: &[TimedInput], samples: &[SamplePoint], keys: &[Key]) -> Result<()> {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assertion {
    pub name: String,
    pub passed: bool,
    pub actual: f64,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    pub source: String,
}
pub fn bound(
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
pub fn distance_xy(a: &ActorSample, b: &ActorSample) -> f64 {
    let dx = f64::from(b.translation[0] - a.translation[0]);
    let dy = f64::from(b.translation[1] - a.translation[1]);
    dx.hypot(dy)
}
pub fn heading_delta(a: &ActorSample, b: &ActorSample) -> f64 {
    let difference = b.heading_rad - a.heading_rad;
    difference.sin().atan2(difference.cos()).abs()
}
pub fn heading_error_to(target: f64, actual: f64) -> f64 {
    let difference = target - actual;
    difference.sin().atan2(difference.cos())
}
pub fn correction_hold_ms(error: f64) -> u64 {
    ((error.abs() / 3.0) * 1_000.0).round().clamp(50.0, 100.0) as u64
}
pub fn action_is(sample: &ActorSample, action: u32) -> f64 {
    f64::from(u8::from(sample.action == Some(action)))
}
pub fn channel_active(sample: &ActorSample, channel: u32) -> Option<bool> {
    let key = channel.to_string();
    let visible = sample.raw_observation["channel_visibility"][key.as_str()].as_array()?;
    (visible.len() == 1).then(|| visible[0].as_bool()).flatten()
}
pub fn walking_action_is_active(sample: &ActorSample) -> f64 {
    // The source normal4 rule requests action 78 before switching to normal5;
    // normal5 then requests action 32. Both are observed during held Up input.
    f64::from(u8::from(matches!(
        sample.action,
        Some(WALK_START_ACTION | WALK_FORWARD_ACTION)
    )))
}

pub fn running_action_is_active(sample: &ActorSample) -> f64 {
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
    if scenario == Scenario::CrossbowInitialLaunch {
        return crossbow::assertions(samples);
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
                Scenario::CrossbowInitialLaunch => unreachable!(),
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
        Scenario::CrossbowInitialLaunch => unreachable!(),
        Scenario::FireCast => unreachable!(),
    }
    Ok(checks)
}

pub fn run_comparison_assertions(samples: &[ActorSample]) -> Result<Vec<Assertion>> {
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
    if openspace.scenario == Scenario::CrossbowInitialLaunch {
        return Ok(CrossBackendComparison {
            status: "passed".into(),
            scenario: openspace.scenario,
            recording_sha256: openspace.recording_sha256.clone(),
            checks: vec![bound(
                "crossbow_initial_launch_both_backends",
                1.0,
                Some(1.0),
                Some(1.0),
                "Independent checked model-733 launch and displacement; impact, contact result, camera and ammunition parity are outside this initial-launch case",
            )],
            visual_status: "pending_completed_render_pairing".into(),
            timing_profile: "pause_compensated_source_burst_vs_native_godot_frame_trace".into(),
        });
    }
    let expected_samples = match openspace.scenario {
        Scenario::TurnReleaseIdle | Scenario::WalkReleaseStop => 5,
        Scenario::RunVsWalk => 10,
        Scenario::WallContact => 7,
        Scenario::StationaryJump => 7,
        Scenario::MovingJump => 8,
        Scenario::WallJump => 8,
        Scenario::SwordAttack => 6,
        Scenario::CrossbowInitialLaunch => unreachable!(),
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
        let settled_windows: &[(usize, usize)] = if self.scenario == Scenario::CrossbowInitialLaunch
        {
            &[]
        } else if matches!(self.scenario, Scenario::SwordAttack | Scenario::FireCast) {
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
