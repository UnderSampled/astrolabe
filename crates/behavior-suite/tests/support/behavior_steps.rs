use anyhow::{Context, Result, ensure};
use astrolabe::{
    model::{animation::TransformValue, geometry::SourceTransform},
    openspace::level::live::ActorObservation,
};
use behavior_contract::{self as behavior, BackendResult, Scenario};
use cucumber::{World as _, WriterExt as _, given, then, when, writer::Stats as _};
use godot_harness::behavior::GodotBackend;
use orrery::behavior::OriginalBackend;
use std::{
    cell::{Cell, RefCell},
    fs,
    path::PathBuf,
    process::Command,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Mode {
    Synthetic,
    Original,
    Godot,
}

enum SessionBackend {
    Original(Box<OriginalBackend>),
    Godot(Box<GodotBackend>),
}
impl SessionBackend {
    fn face_clear_path(&mut self) -> Result<()> {
        match self {
            Self::Original(backend) => backend.face_clear_path(),
            Self::Godot(backend) => backend.face_clear_path(),
        }
    }
    fn face_wall(&mut self) -> Result<()> {
        match self {
            Self::Original(backend) => backend.face_wall(),
            Self::Godot(backend) => backend.face_wall(),
        }
    }
    fn prepare_stationary_jump(&mut self) -> Result<()> {
        match self {
            Self::Original(backend) => backend.prepare_stationary_jump(),
            Self::Godot(backend) => backend.prepare_stationary_jump(),
        }
    }
    fn prepare_crossbow(&mut self) -> Result<()> {
        match self {
            Self::Original(backend) => backend.prepare_crossbow(),
            Self::Godot(backend) => backend.prepare_crossbow(),
        }
    }
    fn prepare_wall_jump(&mut self) -> Result<()> {
        match self {
            Self::Original(backend) => backend.prepare_wall_jump(),
            Self::Godot(backend) => backend.prepare_wall_jump(),
        }
    }
    fn run(&mut self, recording: behavior::FunctionalRecording) -> Result<BackendResult> {
        match self {
            Self::Original(backend) => backend.run(recording),
            Self::Godot(backend) => backend.run(recording),
        }
    }
    fn stop(&mut self) -> Result<()> {
        match self {
            Self::Original(backend) => backend.stop(),
            Self::Godot(backend) => backend.stop(),
        }
    }
}

struct ScenarioOutcome {
    scenario: Scenario,
    result: Option<BackendResult>,
    failure: Option<String>,
    local_cleanup_error: Option<String>,
}

fn source_provenance() -> serde_json::Value {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let revision = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|text| text.trim().to_owned());
    let dirty = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| !output.stdout.is_empty());
    serde_json::json!({"git_revision": revision, "working_tree_dirty": dirty})
}

#[derive(cucumber::World, Default)]
pub struct BehaviorWorld {
    backend: Option<SessionBackend>,
    result: Option<BackendResult>,
    scenario: Option<Scenario>,
    failure: Option<String>,
    outcomes: Option<Rc<RefCell<Vec<ScenarioOutcome>>>>,
    launch_count: Option<Rc<Cell<u32>>>,
}
impl std::fmt::Debug for BehaviorWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BehaviorWorld")
            .field("scenario", &self.scenario)
            .field("failure", &self.failure)
            .finish()
    }
}
impl Drop for BehaviorWorld {
    fn drop(&mut self) {
        let cleanup = self.backend.as_mut().map(SessionBackend::stop).transpose();
        if let (Some(scenario), Some(outcomes)) = (self.scenario, &self.outcomes) {
            outcomes.borrow_mut().push(ScenarioOutcome {
                scenario,
                result: self.result.take(),
                failure: self.failure.take(),
                local_cleanup_error: cleanup.err().map(|err| err.to_string()),
            });
        }
    }
}

fn synthetic_actor(action: u32, translation: [f32; 3], heading: f32) -> ActorObservation {
    let (sine, cosine) = heading.sin_cos();
    let transform = SourceTransform {
        translation,
        rotation: [[cosine, sine, 0.], [-sine, cosine, 0.], [0., 0., 1.]],
        basis: [[cosine, sine, 0.], [-sine, cosine, 0.], [0., 0., 1.]],
    };
    ActorObservation {
        node: "synthetic:node".into(),
        perso: "synthetic:perso".into(),
        data3d: "synthetic:data3d".into(),
        family: "synthetic:family".into(),
        object_types: [1, 1, 1],
        family_type: 1,
        action_members: vec![],
        action_definitions: vec![],
        initial_action: Some(47),
        current_action: Some(action),
        secondary_action: None,
        current_frame: None,
        global_transform: TransformValue::Affine(transform),
    }
}
pub fn synthetic_result(recording: behavior::FunctionalRecording) -> Result<BackendResult> {
    let scenario = recording.scenario;
    let (points, active) = match scenario {
        Scenario::TurnReleaseIdle => (
            [
                (0., 0., 0.),
                (0., 0., 1.),
                (0., 0., 1.),
                (0., 0., 1.),
                (0., 0., 1.),
            ],
            35,
        ),
        Scenario::WalkReleaseStop => (
            [
                (0., 0., 0.),
                (1., 0., 0.),
                (2.2, 0., 0.),
                (2.25, 0., 0.),
                (2.25, 0., 0.),
            ],
            32,
        ),
        Scenario::RunVsWalk => {
            let points = [
                (0., 47),
                (0.5, 32),
                (0.8, 32),
                (1.0, 47),
                (1.0, 47),
                (0., 47),
                (1.0, 33),
                (1.7, 33),
                (2.4, 47),
                (2.4, 47),
            ];
            let samples = points
                .into_iter()
                .enumerate()
                .map(|(index, (distance, action))| {
                    let phase = if index < 5 { "walk" } else { "run" };
                    let point = &recording.samples[index % 5];
                    orrery::behavior::actor_sample_from_actor(
                        format!("{phase}_{}", point.name),
                        point.at_ms,
                        point.at_ms,
                        point.at_ms + 1,
                        synthetic_actor(action, [distance, 0., 0.], 0.),
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            let checks = behavior::assertions(scenario, &samples)?;
            let delivered = recording
                .preparatory_trial
                .as_ref()
                .unwrap()
                .inputs
                .iter()
                .chain(&recording.inputs)
                .map(|input| behavior::DeliveredInput {
                    id: input.id.clone(),
                    planned_ms: input.at_ms,
                    actual_ms: input.at_ms,
                    event: input.event.clone(),
                })
                .collect();
            return Ok(BackendResult {
                backend: "synthetic".into(),
                scenario,
                status: if checks.iter().all(|check| check.passed) {
                    "passed"
                } else {
                    "failed"
                }
                .into(),
                recording_sha256: recording.sha256()?,
                recording,
                executable_sha256: None,
                save_sha256: None,
                loaded_level: Some("synthetic_manoir".into()),
                input_delivery: "synthetic_fixture".into(),
                observation_consistency: "synthetic".into(),
                completed_update: false,
                setup_interventions: vec![],
                delivered,
                samples,
                assertions: checks,
                screenshot: None,
                cleanup_complete: false,
                error: None,
            });
        }
        Scenario::WallContact => {
            let positions = [
                ([-74.37866, 27.48651, 4.55087], 47),
                ([-76.4, 29.5, 4.7], 32),
                ([-79.82, 32.034, 5.007], 32),
                ([-79.823, 32.034, 5.007], 32),
                ([-79.823, 32.034, 5.007], 37),
                ([-79.823, 32.034, 5.007], 47),
                ([-79.823, 32.034, 5.007], 47),
            ];
            let samples = positions
                .into_iter()
                .zip(&recording.samples)
                .map(|((position, action), point)| {
                    let mut sample = orrery::behavior::actor_sample_from_actor(
                        point.name.clone(),
                        point.at_ms,
                        point.at_ms,
                        point.at_ms + 1,
                        synthetic_actor(action, position, 0.),
                    )?;
                    sample.raw_observation = serde_json::json!({
                        "on_floor": true,
                        "slide_contacts": if point.name.starts_with("contact") {
                            serde_json::json!([{"source_node_id": "world/1/5/0"}])
                        } else { serde_json::json!([]) },
                    });
                    Ok(sample)
                })
                .collect::<Result<Vec<_>>>()?;
            let checks = behavior::assertions(scenario, &samples)?;
            let delivered = recording
                .inputs
                .iter()
                .map(|input| behavior::DeliveredInput {
                    id: input.id.clone(),
                    planned_ms: input.at_ms,
                    actual_ms: input.at_ms,
                    event: input.event.clone(),
                })
                .collect();
            return Ok(BackendResult {
                backend: "synthetic".into(),
                scenario,
                status: if checks.iter().all(|check| check.passed) {
                    "passed"
                } else {
                    "failed"
                }
                .into(),
                recording_sha256: recording.sha256()?,
                recording,
                executable_sha256: None,
                save_sha256: None,
                loaded_level: Some("synthetic_manoir".into()),
                input_delivery: "synthetic_fixture".into(),
                observation_consistency: "synthetic".into(),
                completed_update: false,
                setup_interventions: vec![],
                delivered,
                samples,
                assertions: checks,
                screenshot: None,
                cleanup_complete: true,
                error: None,
            });
        }
        Scenario::StationaryJump => {
            let positions = [
                ([0.0, 0.0, 4.55], 47),
                ([0.0, 0.0, 5.35], 2),
                ([0.0, 0.0, 6.75], 2),
                ([0.0, 0.0, 5.85], 4),
                ([0.0, 0.0, 4.55], 0),
                ([0.0, 0.0, 4.55], 47),
                ([0.0, 0.0, 4.55], 47),
            ];
            let samples = positions.into_iter().zip(&recording.samples)
                .map(|((position, action), point)| {
                    let mut sample = orrery::behavior::actor_sample_from_actor(point.name.clone(), point.at_ms, point.at_ms, point.at_ms + 1, synthetic_actor(action, position, 0.))?;
                    sample.raw_observation = serde_json::json!({"on_floor": point.name == "floor" || point.name.starts_with("settled")});
                    Ok(sample)
                }).collect::<Result<Vec<_>>>()?;
            let checks = behavior::assertions(scenario, &samples)?;
            let delivered = recording
                .inputs
                .iter()
                .map(|input| behavior::DeliveredInput {
                    id: input.id.clone(),
                    planned_ms: input.at_ms,
                    actual_ms: input.at_ms,
                    event: input.event.clone(),
                })
                .collect();
            return Ok(BackendResult {
                backend: "synthetic".into(),
                scenario,
                status: if checks.iter().all(|check| check.passed) {
                    "passed"
                } else {
                    "failed"
                }
                .into(),
                recording_sha256: recording.sha256()?,
                recording,
                executable_sha256: None,
                save_sha256: None,
                loaded_level: Some("synthetic_manoir".into()),
                input_delivery: "synthetic_fixture".into(),
                observation_consistency: "synthetic".into(),
                completed_update: false,
                setup_interventions: vec![],
                delivered,
                samples,
                assertions: checks,
                screenshot: None,
                cleanup_complete: true,
                error: None,
            });
        }
        Scenario::MovingJump | Scenario::WallJump => {
            let positions = if scenario == Scenario::WallJump {
                [
                    ([-78.68, 30.818, 4.852], 47),
                    ([-78.745, 30.88, 4.852], 32),
                    ([-79.50, 31.70, 6.20], 7),
                    ([-79.83, 32.03, 7.10], 70),
                    ([-79.83, 32.03, 5.60], 4),
                    ([-79.83, 32.03, 4.95], 0),
                    ([-79.83, 32.03, 4.95], 47),
                    ([-79.83, 32.03, 4.95], 47),
                ]
            } else {
                [
                    ([0.0, 0.0, 4.55], 47),
                    ([0.4, 0.0, 4.55], 32),
                    ([1.3, 0.0, 5.7], 7),
                    ([2.4, 0.0, 6.8], 70),
                    ([3.4, 0.0, 5.3], 4),
                    ([4.0, 0.0, 4.55], 0),
                    ([4.0, 0.0, 4.55], 47),
                    ([4.0, 0.0, 4.55], 47),
                ]
            };
            let samples = positions
                .into_iter()
                .zip(&recording.samples)
                .map(|((position, action), point)| {
                    let mut sample = orrery::behavior::actor_sample_from_actor(
                        point.name.clone(),
                        point.at_ms,
                        point.at_ms,
                        point.at_ms + 1,
                        synthetic_actor(action, position, 0.),
                    )?;
                    sample.raw_observation = serde_json::json!({
                        "on_floor": !matches!(point.name.as_str(), "rise" | "high" | "descent")
                    });
                    Ok(sample)
                })
                .collect::<Result<Vec<_>>>()?;
            let checks = behavior::assertions(scenario, &samples)?;
            let delivered = recording
                .inputs
                .iter()
                .map(|input| behavior::DeliveredInput {
                    id: input.id.clone(),
                    planned_ms: input.at_ms,
                    actual_ms: input.at_ms,
                    event: input.event.clone(),
                })
                .collect();
            return Ok(BackendResult {
                backend: "synthetic".into(),
                scenario,
                status: if checks.iter().all(|check| check.passed) {
                    "passed"
                } else {
                    "failed"
                }
                .into(),
                recording_sha256: recording.sha256()?,
                recording,
                executable_sha256: None,
                save_sha256: None,
                loaded_level: Some("synthetic_manoir".into()),
                input_delivery: "synthetic_fixture".into(),
                observation_consistency: "synthetic".into(),
                completed_update: false,
                setup_interventions: vec![],
                delivered,
                samples,
                assertions: checks,
                screenshot: None,
                cleanup_complete: true,
                error: None,
            });
        }
        Scenario::SwordAttack => {
            let actions = [47, 45, 134, 134, 48, 48];
            let samples = actions
                .into_iter()
                .zip(&recording.samples)
                .map(|(action, point)| {
                    orrery::behavior::actor_sample_from_actor(
                        point.name.clone(),
                        point.at_ms,
                        point.at_ms,
                        point.at_ms + 1,
                        synthetic_actor(action, [0.0, 0.0, 4.55], 0.0),
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            let checks = behavior::assertions(scenario, &samples)?;
            let delivered = recording
                .inputs
                .iter()
                .map(|input| behavior::DeliveredInput {
                    id: input.id.clone(),
                    planned_ms: input.at_ms,
                    actual_ms: input.at_ms,
                    event: input.event.clone(),
                })
                .collect();
            return Ok(BackendResult {
                backend: "synthetic".into(),
                scenario,
                status: if checks.iter().all(|check| check.passed) {
                    "passed"
                } else {
                    "failed"
                }
                .into(),
                recording_sha256: recording.sha256()?,
                recording,
                executable_sha256: None,
                save_sha256: None,
                loaded_level: Some("synthetic_manoir".into()),
                input_delivery: "synthetic_fixture".into(),
                observation_consistency: "synthetic".into(),
                completed_update: false,
                setup_interventions: vec![],
                delivered,
                samples,
                assertions: checks,
                screenshot: None,
                cleanup_complete: true,
                error: None,
            });
        }
        Scenario::CrossbowInitialLaunch => {
            let mut samples = recording
                .samples
                .iter()
                .map(|point| {
                    orrery::behavior::actor_sample_from_actor(
                        point.name.clone(),
                        point.at_ms,
                        point.at_ms,
                        point.at_ms + 1,
                        synthetic_actor(47, [0.0, 0.0, 4.55], 0.0),
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            samples[0].raw_observation["inventory"] =
                serde_json::json!({"selected_weapon": 80, "weapon_80_quantity": 3});
            samples[1].action = Some(48);
            samples[2].heading_rad = 0.1;
            samples[2].raw_observation["normal_behavior"] = serde_json::json!(40);
            samples[2].raw_observation["brain_running"] = serde_json::json!(true);
            samples[2].raw_observation["brain_failure"] = serde_json::json!("");
            samples[3].raw_observation["generated_trace_first"] = serde_json::json!([]);
            samples[3].raw_observation["inventory"] =
                serde_json::json!({"selected_weapon": 80, "weapon_80_quantity": 2});
            samples[3].raw_observation["generated_trace"] = serde_json::json!([
                {"event":"release","instance_id":1,"source_model_path":"fix:/ai/models/733",
                 "at_ms":1300,"brain_clock_ms":0.0,
                 "translation":[0.0,0.0,5.0],"attachment_translation":[0.0,0.0,5.0]},
                {"event":"frame","instance_id":1,"source_model_path":"fix:/ai/models/733",
                 "brain_running":true,"brain_failure":"","translation":[0.0,0.0,5.0],
                 "velocity":[0.0,0.0,45.0],"sighting":[0.0,0.0,1.0],
                 "main_actor_field_120":1},
                {"event":"frame","instance_id":1,"source_model_path":"fix:/ai/models/733",
                 "brain_running":true,"brain_failure":"","translation":[0.0,0.0,5.5],
                 "main_actor_field_120":1},
                {"event":"tree_exiting","instance_id":1,"source_model_path":"fix:/ai/models/733",
                 "slide_collision_count":1,"brain_normal":1,"brain_failure":"",
                 "at_ms":1350,"brain_clock_ms":50.0,"slide_contacts":[{"source_perso":"level:/persos/world/0/35",
                 "position":[-73.98,6.09,-29.08692512512207],"normal":[0.0,0.0,1.0]}],
                 "main_actor_field_120":0}
            ]);
            let checks = behavior::assertions(scenario, &samples)?;
            let delivered = recording
                .inputs
                .iter()
                .map(|input| behavior::DeliveredInput {
                    id: input.id.clone(),
                    planned_ms: input.at_ms,
                    actual_ms: input.at_ms,
                    event: input.event.clone(),
                })
                .collect();
            return Ok(BackendResult {
                backend: "synthetic".into(),
                scenario,
                status: if checks.iter().all(|check| check.passed) {
                    "passed"
                } else {
                    "failed"
                }
                .into(),
                recording_sha256: recording.sha256()?,
                recording,
                executable_sha256: None,
                save_sha256: None,
                loaded_level: Some("synthetic_manoir".into()),
                input_delivery: "synthetic_fixture".into(),
                observation_consistency: "synthetic".into(),
                completed_update: false,
                setup_interventions: vec![],
                delivered,
                samples,
                assertions: checks,
                screenshot: None,
                cleanup_complete: true,
                error: None,
            });
        }
        Scenario::FireCast => {
            let actions = [47, 48, 290, 290, 271, 271];
            let mana = [30, 30, 30, 30, 20, 20];
            let samples = actions
                .into_iter()
                .zip(mana)
                .zip(&recording.samples)
                .map(|((action, mana), point)| {
                    let mut sample = orrery::behavior::actor_sample_from_actor(
                        point.name.clone(),
                        point.at_ms,
                        point.at_ms,
                        point.at_ms + 1,
                        synthetic_actor(action, [0.0, 0.0, 4.55], 0.0),
                    )?;
                    sample.raw_observation["magick"] = serde_json::json!({
                        "selected_spell_code": 61,
                        "current_points": mana,
                        "maximum_points": 50,
                    });
                    sample.raw_observation["designer_fields"] =
                        serde_json::json!({"59": action != 47});
                    sample.raw_observation["action_states"] = serde_json::json!([
                        {"action": 290}, {"action": 271}
                    ]);
                    Ok(sample)
                })
                .collect::<Result<Vec<_>>>()?;
            let checks = behavior::assertions(scenario, &samples)?;
            let delivered = recording
                .inputs
                .iter()
                .map(|input| behavior::DeliveredInput {
                    id: input.id.clone(),
                    planned_ms: input.at_ms,
                    actual_ms: input.at_ms,
                    event: input.event.clone(),
                })
                .collect();
            return Ok(BackendResult {
                backend: "synthetic".into(),
                scenario,
                status: if checks.iter().all(|check| check.passed) {
                    "passed"
                } else {
                    "failed"
                }
                .into(),
                recording_sha256: recording.sha256()?,
                recording,
                executable_sha256: None,
                save_sha256: None,
                loaded_level: Some("synthetic_manoir".into()),
                input_delivery: "synthetic_fixture".into(),
                observation_consistency: "synthetic".into(),
                completed_update: false,
                setup_interventions: vec![],
                delivered,
                samples,
                assertions: checks,
                screenshot: None,
                cleanup_complete: true,
                error: None,
            });
        }
    };
    let samples = recording
        .samples
        .iter()
        .zip(points)
        .enumerate()
        .map(|(index, (point, (x, y, heading)))| {
            orrery::behavior::actor_sample_from_actor(
                point.name.clone(),
                point.at_ms,
                point.at_ms,
                point.at_ms + 1,
                synthetic_actor(
                    if index == 1 || index == 2 { active } else { 47 },
                    [x, y, 0.],
                    heading,
                ),
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let checks = behavior::assertions(scenario, &samples)?;
    let delivered = recording
        .inputs
        .iter()
        .map(|input| behavior::DeliveredInput {
            id: input.id.clone(),
            planned_ms: input.at_ms,
            actual_ms: input.at_ms,
            event: input.event.clone(),
        })
        .collect();
    Ok(BackendResult {
        backend: "synthetic".into(),
        scenario,
        status: if checks.iter().all(|c| c.passed) {
            "passed"
        } else {
            "failed"
        }
        .into(),
        recording_sha256: recording.sha256()?,
        recording,
        executable_sha256: None,
        save_sha256: None,
        loaded_level: Some("synthetic_manoir".into()),
        input_delivery: "synthetic_fixture".into(),
        observation_consistency: "synthetic".into(),
        completed_update: false,
        setup_interventions: vec![],
        delivered,
        samples,
        assertions: checks,
        screenshot: None,
        cleanup_complete: false,
        error: None,
    })
}

#[given("a clear path is ahead of Hype")]
fn clear_path(world: &mut BehaviorWorld) {
    if crate::BACKEND != Mode::Synthetic {
        let result = world
            .backend
            .as_mut()
            .context("behavior backend not started")
            .and_then(SessionBackend::face_clear_path);
        if let Err(err) = result {
            world.failure = Some(err.to_string());
            panic!("clear-path setup failed: {err}")
        }
    }
}

#[given("the manor corner is ahead of Hype")]
fn wall_path(world: &mut BehaviorWorld) {
    if crate::BACKEND != Mode::Synthetic {
        let result = world
            .backend
            .as_mut()
            .context("behavior backend not started")
            .and_then(SessionBackend::face_wall);
        if let Err(err) = result {
            world.failure = Some(err.to_string());
            panic!("corner setup failed: {err}")
        }
    }
}

#[given("Hype is grounded near the verified manor corner wall")]
fn wall_jump_start(world: &mut BehaviorWorld) {
    if crate::BACKEND != Mode::Synthetic {
        let result = world
            .backend
            .as_mut()
            .context("behavior backend not started")
            .and_then(SessionBackend::prepare_wall_jump);
        if let Err(err) = result {
            world.failure = Some(err.to_string());
            panic!("wall-jump setup failed: {err}")
        }
    }
}

#[given("Hype has backed into a clear grounded jump start")]
fn clear_jump_start(world: &mut BehaviorWorld) {
    if crate::BACKEND != Mode::Synthetic {
        let result = world
            .backend
            .as_mut()
            .context("behavior backend not started")
            .and_then(SessionBackend::prepare_stationary_jump);
        if let Err(err) = result {
            world.failure = Some(err.to_string());
            panic!("jump setup failed: {err}");
        }
    }
}

#[given("Hype is grounded away from the manor save point")]
fn clear_sword_start(world: &mut BehaviorWorld) {
    clear_jump_start(world);
}

#[given("Hype has three test arrows in the supplied manor state")]
fn crossbow_start(world: &mut BehaviorWorld) {
    if crate::BACKEND != Mode::Synthetic {
        let result = world
            .backend
            .as_mut()
            .context("behavior backend not started")
            .and_then(SessionBackend::prepare_crossbow);
        if let Err(err) = result {
            world.failure = Some(err.to_string());
            panic!("crossbow setup failed: {err}");
        }
    }
}

#[given("Hype is ready to move in a clear part of the manor")]
fn ready(world: &mut BehaviorWorld) {
    if crate::BACKEND != Mode::Synthetic && world.backend.is_none() {
        let result = (|| -> Result<SessionBackend> {
            let runs = std::env::var_os("ASTROLABE_EVIDENCE_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../TestResults")
                })
                .join("behavior-runs");
            Ok(match crate::BACKEND {
                Mode::Original => {
                    let config = PathBuf::from(
                        std::env::var_os("ASTROLABE_HYPE_CONFIG")
                            .context("ASTROLABE_HYPE_CONFIG missing")?,
                    );
                    let executable = std::env::var_os("ASTROLABE_ORRERY_BINARY")
                        .map(PathBuf::from)
                        .unwrap_or_else(|| {
                            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                                .join("../../target/validation/orrery")
                        });
                    ensure!(
                        executable.is_file(),
                        "Orrery executable is missing: {}",
                        executable.display()
                    );
                    SessionBackend::Original(Box::new(OriginalBackend::start_with_bridge(
                        &config,
                        &runs,
                        Some(&executable),
                    )?))
                }
                Mode::Godot => SessionBackend::Godot(Box::new(GodotBackend::start(&runs)?)),
                Mode::Synthetic => unreachable!(),
            })
        })();
        match result {
            Ok(backend) => {
                world.backend = Some(backend);
                if let Some(count) = &world.launch_count {
                    count.set(count.get() + 1);
                }
            }
            Err(err) => {
                world.failure = Some(err.to_string());
                panic!("behavior setup failed: {err}")
            }
        }
    }
}
fn perform(world: &mut BehaviorWorld, scenario: Scenario) {
    world.scenario = Some(scenario);
    let tape = behavior::recording(scenario);
    if let Err(err) = tape.validate() {
        world.failure = Some(err.to_string());
        panic!("shared case schedule is invalid: {err}");
    }
    let result = match crate::BACKEND {
        Mode::Synthetic => synthetic_result(tape),
        Mode::Original | Mode::Godot => world
            .backend
            .as_mut()
            .context("behavior backend not started")
            .and_then(|backend| backend.run(tape)),
    };
    match result {
        Ok(result) => match behavior_suite::evaluate_backend_result(result) {
            Ok(result) => world.result = Some(result),
            Err(err) => {
                world.failure = Some(err.to_string());
                panic!("behavior suite evaluation failed: {err}")
            }
        },
        Err(err) => {
            world.failure = Some(err.to_string());
            panic!("behavior recording failed: {err}")
        }
    }
}

#[when("the player turns and releases the turning control")]
fn turn(world: &mut BehaviorWorld) {
    perform(world, Scenario::TurnReleaseIdle);
}

#[when("the player walks forward and releases the movement control")]
fn walk(world: &mut BehaviorWorld) {
    perform(world, Scenario::WalkReleaseStop);
}

#[when("the player walks and runs from equivalent starts")]
fn run_vs_walk(world: &mut BehaviorWorld) {
    perform(world, Scenario::RunVsWalk);
}

#[when("the player keeps walking into the corner")]
fn wall_contact(world: &mut BehaviorWorld) {
    perform(world, Scenario::WallContact);
}

#[when("the player presses and releases jump while standing")]
fn stationary_jump(world: &mut BehaviorWorld) {
    perform(world, Scenario::StationaryJump);
}

#[when("the player jumps while walking and steers after takeoff")]
fn moving_jump(world: &mut BehaviorWorld) {
    perform(world, Scenario::MovingJump);
}

#[when("the player jumps forward toward the wall")]
fn wall_jump(world: &mut BehaviorWorld) {
    perform(world, Scenario::WallJump);
}

#[when("the player selects the sword and presses attack once")]
fn sword_attack(world: &mut BehaviorWorld) {
    perform(world, Scenario::SwordAttack);
}

#[when("the player selects the crossbow and presses attack once")]
fn crossbow_launch(world: &mut BehaviorWorld) {
    perform(world, Scenario::CrossbowInitialLaunch);
}

#[then("the compiled scripts spawn a model 733 arrow")]
fn crossbow_arrow_spawned(world: &mut BehaviorWorld) {
    require_checks(
        world,
        &[
            "crossbow_quantity_three_start",
            "crossbow_armed_before_attack",
        ],
    );
    if crate::BACKEND == Mode::Original {
        require_checks(world, &["crossbow_original_model733_spawned"]);
    } else {
        require_checks(world, &["crossbow_compiled_arrow_starts"]);
    }
}

#[then("the arrow moves between two initial observations")]
fn crossbow_arrow_moves(world: &mut BehaviorWorld) {
    require_checks(world, &["crossbow_initial_displacement"]);
}

#[when("the player selects the sword and casts Fire once")]
fn fire_cast(world: &mut BehaviorWorld) {
    perform(world, Scenario::FireCast);
}

#[then("Hype enters the Fire cast and spends ten magic points")]
fn fire_cast_cost(world: &mut BehaviorWorld) {
    require_checks(
        world,
        &[
            "fire_baseline_idle",
            "fire_baseline_mana",
            "fire_selected_code",
            "fire_armed_selection_phase",
            "fire_cast_gate_ready",
            "fire_cast_action",
            "fire_mana_spent",
        ],
    );
}

#[then("Hype reaches the Fire follow-through action")]
fn fire_cast_follow_through(world: &mut BehaviorWorld) {
    require_checks(world, &["fire_follow_through", "fire_settled_mana"]);
}

#[then("Hype equips the sword and performs an attack action")]
fn sword_attack_action(world: &mut BehaviorWorld) {
    require_checks(
        world,
        &[
            "sword_baseline_idle",
            "sword_equipping_action",
            "sword_attack_early_phase",
            "sword_attack_late_phase",
        ],
    );
    if crate::BACKEND == Mode::Godot {
        require_checks(world, &["sword_native_floor_contact"]);
    }
}

#[then("Hype returns to armed idle without moving away")]
fn sword_attack_recovered(world: &mut BehaviorWorld) {
    require_checks(
        world,
        &[
            "sword_recovered_idle",
            "sword_settled_idle",
            "sword_stationary_xy",
            "sword_post_attack_drift",
        ],
    );
}

#[then("Hype meets the wall while airborne without passing through")]
fn wall_jump_contact(world: &mut BehaviorWorld) {
    require_checks(
        world,
        &[
            "moving_jump_baseline_idle",
            "moving_jump_walking_action",
            "moving_jump_walk_progress",
            "moving_jump_launch_action",
            "moving_jump_rise_height",
            "moving_jump_forward_in_air",
            "wall_jump_start_angled_clearance",
            "wall_jump_start_cross_clearance",
            "wall_jump_airborne_wall_approach",
            "wall_jump_angled_no_passage",
            "wall_jump_cross_no_passage",
        ],
    );
    if crate::BACKEND == Mode::Godot {
        require_checks(
            world,
            &[
                "moving_jump_native_airborne",
                "wall_jump_native_named_airborne_contact",
                "wall_jump_native_wall_not_floor_or_slope",
            ],
        );
    }
}

#[then("Hype lands and settles after meeting the wall")]
fn wall_jump_lands(world: &mut BehaviorWorld) {
    moving_jump_lands(world);
}

#[then("Hype jumps upward and forward from the walk")]
fn moving_jump_rises(world: &mut BehaviorWorld) {
    require_checks(
        world,
        &[
            "moving_jump_baseline_idle",
            "moving_jump_walking_action",
            "moving_jump_walk_progress",
            "moving_jump_walk_level",
            "moving_jump_launch_action",
            "moving_jump_rise_height",
            "moving_jump_sampled_gain",
            "moving_jump_forward_in_air",
        ],
    );
    if crate::BACKEND == Mode::Godot {
        require_checks(world, &["moving_jump_native_airborne"]);
    }
}

#[then("Hype lands and settles after the moving jump")]
fn moving_jump_lands(world: &mut BehaviorWorld) {
    require_checks(
        world,
        &[
            "moving_jump_floor_return",
            "moving_jump_settled_height",
            "moving_jump_settled_idle_start",
            "moving_jump_settled_idle_end",
            "moving_jump_post_landing_drift",
        ],
    );
    if crate::BACKEND == Mode::Godot {
        require_checks(world, &["moving_jump_native_floor_contact"]);
    }
}

#[then("Hype rises above the floor during the jump")]
fn jump_rises(world: &mut BehaviorWorld) {
    require_checks(
        world,
        &[
            "jump_baseline_idle",
            "jump_rise_action",
            "jump_rise_height",
            "jump_sampled_gain",
            "jump_stationary_xy",
        ],
    );
}

#[then("Hype returns to the starting floor and settles")]
fn jump_lands(world: &mut BehaviorWorld) {
    require_checks(
        world,
        &[
            "jump_floor_return",
            "jump_settled_height",
            "jump_settled_idle_start",
            "jump_settled_idle_end",
            "jump_post_landing_drift",
        ],
    );
    if crate::BACKEND == Mode::Godot {
        require_checks(world, &["jump_native_floor_contact"]);
    }
}

fn require_checks(world: &mut BehaviorWorld, names: &[&str]) {
    let result = world
        .result
        .as_ref()
        .expect("When step produced no backend result");
    let mut failure = None;
    for name in names {
        match result.assertions.iter().find(|check| check.name == *name) {
            Some(check) if check.passed => (),
            Some(check) => {
                failure = Some(format!(
                    "{}: {} outside {:?}..{:?}",
                    check.name, check.actual, check.lower, check.upper
                ));
                break;
            }
            None => {
                failure = Some(format!("missing assertion {name}"));
                break;
            }
        }
    }
    if let Some(failure) = failure {
        world.failure = Some(failure.clone());
        panic!("{failure}")
    }
}

#[then("Hype faces the new direction without walking away")]
fn turned(world: &mut BehaviorWorld) {
    require_checks(
        world,
        &["active_action", "heading_change_rad", "turn_translation_xy"],
    );
}

#[then("Hype moves forward from the starting place")]
fn moved(world: &mut BehaviorWorld) {
    require_checks(
        world,
        &[
            "active_action",
            "walk_translation_xy",
            "late_held_progress_xy",
            "settling_continuation_xy",
            "walk_height_change",
            "heading_retained_rad",
        ],
    );
}

#[then("Hype returns to standing without continuing to drift")]
fn stopped(world: &mut BehaviorWorld) {
    require_checks(
        world,
        &[
            "baseline_idle_action",
            "settled_idle_action_start",
            "settled_idle_action_end",
            "post_release_drift_xy",
        ],
    );
}

#[then("running moves Hype faster than walking")]
fn ran_faster(world: &mut BehaviorWorld) {
    require_checks(
        world,
        &[
            "walk_active_action",
            "run_active_action",
            "equivalent_start_xy",
            "equivalent_heading_rad",
            "walk_distance_xy",
            "run_distance_xy",
            "run_forward_distance",
            "run_to_walk_ratio",
            "run_lateral_distance",
            "walk_settled_idle",
            "run_settled_idle",
            "walk_post_release_drift",
            "run_post_release_drift",
            "walk_height_change",
            "run_height_change",
        ],
    );
}

#[then("Hype reaches the source wall without passing through it")]
fn reached_wall(world: &mut BehaviorWorld) {
    require_checks(
        world,
        &[
            "wall_approach_active",
            "wall_approach_distance_xy",
            "wall_cross_approach_clearance_change",
            "wall_cross_contact_start_clearance",
            "wall_cross_contact_end_clearance",
            "wall_angled_contact_start_clearance",
            "wall_angled_contact_end_clearance",
            "wall_cross_sampled_min_clearance",
            "wall_angled_sampled_min_clearance",
        ],
    );
    if crate::BACKEND != Mode::Original {
        require_checks(
            world,
            &["wall_native_named_contact", "wall_native_floor_contact"],
        );
    }
}

#[then("Hype stops at the corner and returns to standing after release")]
fn stopped_at_wall(world: &mut BehaviorWorld) {
    require_checks(
        world,
        &[
            "wall_baseline_idle",
            "wall_contact_start_active",
            "wall_contact_end_active",
            "wall_held_translation_xy",
            "wall_held_height_change",
            "wall_post_release_drift",
            "wall_settled_idle",
        ],
    );
}

fn selected_scenario(name: &str) -> Result<Scenario> {
    Ok(match name {
        "Turning while standing" => Scenario::TurnReleaseIdle,
        "Walking and stopping" => Scenario::WalkReleaseStop,
        "Running instead of walking" => Scenario::RunVsWalk,
        "A wall blocks movement" => Scenario::WallContact,
        "Jumping from a standstill and landing" => Scenario::StationaryJump,
        "Jumping from a walk and landing" => Scenario::MovingJump,
        "A moving jump meets the wall and lands" => Scenario::WallJump,
        "Selecting the sword and attacking once" => Scenario::SwordAttack,
        "Launching one crossbow arrow initially" => Scenario::CrossbowInitialLaunch,
        "Casting the selected Fire spell once" => Scenario::FireCast,
        _ => anyhow::bail!("unknown BDD scenario: {name}"),
    })
}

fn evidence_directory() -> PathBuf {
    std::env::var_os("ASTROLABE_EVIDENCE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../TestResults"))
}

fn write_json(path: &PathBuf, value: &serde_json::Value) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    fs::write(path, bytes).with_context(|| format!("write {}", path.display()))
}

#[allow(dead_code)]
pub struct SessionEvidence {
    pub scenario_reports: Vec<PathBuf>,
    pub suite_report: PathBuf,
}

fn run_selected(names: &[&'static str], shared_session: bool) -> Result<SessionEvidence> {
    ensure!(!names.is_empty(), "no BDD scenarios requested");
    ensure!(
        shared_session || names.len() == 1,
        "isolated run selects one scenario"
    );
    let directory = evidence_directory();
    fs::create_dir_all(&directory)?;
    let session_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .to_string();
    let session = Rc::new(RefCell::new(None::<SessionBackend>));
    let outcomes = Rc::new(RefCell::new(Vec::<ScenarioOutcome>::new()));
    let launch_count = Rc::new(Cell::new(0_u32));
    let runtime = tokio::runtime::Runtime::new()?;
    let mut run_error = None;
    for name in names {
        let name = *name;
        let scenario = selected_scenario(name)?;
        let before_session = Rc::clone(&session);
        let before_outcomes = Rc::clone(&outcomes);
        let before_launch_count = Rc::clone(&launch_count);
        let after_session = Rc::clone(&session);
        let writer = runtime.block_on(async {
            let feature = if matches!(
                scenario,
                Scenario::SwordAttack | Scenario::CrossbowInitialLaunch
            ) {
                concat!(env!("CARGO_MANIFEST_DIR"), "/tests/features/combat.feature")
            } else if scenario == Scenario::FireCast {
                concat!(env!("CARGO_MANIFEST_DIR"), "/tests/features/magick.feature")
            } else if matches!(
                scenario,
                Scenario::StationaryJump | Scenario::MovingJump | Scenario::WallJump
            ) {
                concat!(env!("CARGO_MANIFEST_DIR"), "/tests/features/jump.feature")
            } else {
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/tests/features/movement.feature"
                )
            };
            BehaviorWorld::cucumber()
                .with_writer(cucumber::writer::Basic::stdout().summarized())
                .fail_on_skipped()
                .max_concurrent_scenarios(1)
                .before(move |_, _, _, world| {
                    world.scenario = Some(scenario);
                    world.outcomes = Some(Rc::clone(&before_outcomes));
                    world.launch_count = Some(Rc::clone(&before_launch_count));
                    world.backend = before_session.borrow_mut().take();
                    Box::pin(async {})
                })
                .after(move |_, _, _, finished, world| {
                    if let Some(world) = world {
                        if !matches!(finished, cucumber::event::ScenarioFinished::StepPassed) {
                            world.failure.get_or_insert_with(|| {
                                "Cucumber scenario did not finish with passing steps".into()
                            });
                        }
                        if world.failure.is_none()
                            && world.result.as_ref().is_some_and(|r| r.status == "passed")
                            && let Some(backend) = world.backend.take()
                        {
                            *after_session.borrow_mut() = Some(backend);
                        }
                    }
                    Box::pin(async {})
                })
                .with_default_cli()
                .filter_run(feature, move |_, _, candidate| candidate.name == name)
                .await
        });
        if writer.passed_steps()
            < if matches!(
                name,
                "Walking and stopping"
                    | "A wall blocks movement"
                    | "Jumping from a standstill and landing"
            ) {
                5
            } else {
                4
            }
            || writer.failed_steps() != 0
            || writer.skipped_steps() != 0
            || writer.parsing_errors() != 0
            || writer.hook_errors() != 0
        {
            run_error = Some(format!("BDD scenario {name} was incomplete or failed"));
            break;
        }
    }
    let session_cleanup = session
        .borrow_mut()
        .take()
        .map(|mut backend| backend.stop())
        .transpose();
    if let Err(err) = &session_cleanup {
        run_error.get_or_insert_with(|| format!("BDD game cleanup failed: {err}"));
    }
    let cleanup_complete = session_cleanup.is_ok();
    let mut paths = vec![];
    for mut outcome in outcomes.borrow_mut().drain(..) {
        let local_cleanup_complete = outcome.local_cleanup_error.is_none();
        let error = outcome
            .failure
            .take()
            .or(outcome.local_cleanup_error.take())
            .or_else(|| session_cleanup.as_ref().err().map(ToString::to_string));
        if let Some(result) = &mut outcome.result {
            result.cleanup_complete = cleanup_complete && local_cleanup_complete;
            if let Some(error) = &error {
                result.status = "failed".into();
                result.error = Some(error.clone());
            } else if let Err(err) = result.require_pass() {
                result.status = "failed".into();
                result.error = Some(err.to_string());
                run_error.get_or_insert_with(|| format!("{}: {err}", outcome.scenario.slug()));
            }
        }
        let selected_backend = match crate::BACKEND {
            Mode::Synthetic => "synthetic",
            Mode::Original => "openspace",
            Mode::Godot => "godot",
        };
        let selected_status = outcome
            .result
            .as_ref()
            .map(|r| r.status.as_str())
            .unwrap_or("failed");
        let report = serde_json::json!({
            "schema": 1, "scenario": outcome.scenario,
            "selected_backend": selected_backend,
            "source": source_provenance(),
            "session": {"id": session_id, "shared": shared_session, "launch_count": launch_count.get(),
                "startup_settle_ms": match crate::BACKEND { Mode::Original => Some(orrery::behavior::ORIGINAL_STARTUP_SETTLE_MS), Mode::Godot => Some(5_000), Mode::Synthetic => None }},
            "result": outcome.result,
            "openspace": if crate::BACKEND == Mode::Original { serde_json::json!({"status": selected_status}) } else { serde_json::json!({"status": "not_run"}) },
            "godot": if crate::BACKEND == Mode::Godot { serde_json::json!({"status": selected_status}) } else { BackendResult::pending_godot(outcome.scenario) },
            "cross_backend": {"status": "not_run", "reason": "independent backend result must be paired by recording hash"},
            "cleanup_complete": cleanup_complete && local_cleanup_complete,
            "error": error,
        });
        let path = directory.join(format!(
            "behavior-{}-{session_id}.json",
            outcome.scenario.slug()
        ));
        write_json(&path, &report)?;
        eprintln!("behavior evidence: {}", path.display());
        let validation = (|| -> Result<()> {
            let stored: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
            ensure!(stored["cleanup_complete"] == true, "BDD cleanup failed");
            ensure!(stored["result"]["status"] == "passed", "BDD backend failed");
            let result: BackendResult = serde_json::from_value(stored["result"].clone())?;
            result.require_pass()?;
            if crate::BACKEND == Mode::Original {
                ensure!(
                    stored["openspace"]["status"] == "passed",
                    "OpenSpace result missing"
                );
                ensure!(stored["godot"]["status"] == "pending", "Godot slot missing");
            } else if crate::BACKEND == Mode::Godot {
                ensure!(
                    stored["godot"]["status"] == "passed",
                    "Godot result missing"
                );
            }
            Ok(())
        })();
        if let Err(err) = validation {
            run_error.get_or_insert_with(|| format!("{}: {err}", outcome.scenario.slug()));
        }
        paths.push(path);
    }
    let not_run = names.iter().skip(paths.len()).copied().collect::<Vec<_>>();
    if !not_run.is_empty() {
        run_error.get_or_insert_with(|| "BDD scenario report missing".into());
    }
    if crate::BACKEND != Mode::Synthetic && shared_session && launch_count.get() != 1 {
        run_error.get_or_insert_with(|| "shared run restarted the game".into());
    }
    let suite_report = directory.join(format!("behavior-suite-{session_id}.json"));
    write_json(
        &suite_report,
        &serde_json::json!({
            "schema": 1,
            "session_id": session_id,
            "requested": names,
            "completed_reports": paths,
            "not_run": not_run,
            "shared_session": shared_session,
            "launch_count": launch_count.get(),
            "startup_settle_ms": match crate::BACKEND { Mode::Original => Some(orrery::behavior::ORIGINAL_STARTUP_SETTLE_MS), Mode::Godot => Some(5_000), Mode::Synthetic => None },
            "cleanup_complete": cleanup_complete,
            "status": if run_error.is_none() && not_run.is_empty() { "passed" } else { "failed" },
            "error": run_error,
        }),
    )?;
    ensure!(
        run_error.is_none(),
        "{} (suite: {})",
        run_error.as_deref().unwrap_or_default(),
        suite_report.display()
    );
    Ok(SessionEvidence {
        scenario_reports: paths,
        suite_report,
    })
}

#[allow(dead_code)]
pub fn run_scenario(name: &'static str) -> Result<PathBuf> {
    run_selected(&[name], false)?
        .scenario_reports
        .into_iter()
        .next()
        .context("BDD scenario report missing")
}

#[allow(dead_code)]
pub fn run_scenarios_shared(names: &[&'static str]) -> Result<SessionEvidence> {
    run_selected(names, true)
}
