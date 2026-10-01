//! Original OpenSpace backend for the engine-neutral behavior contract.
use crate::{config::Config, orrery::NativeController};
use anyhow::{Context, Result, ensure};
use astrolabe::{
    model::ai::DesignerValue,
    openspace::level::live::{ActorObservation, ObservedDesignerValue},
};
use behavior_contract::*;
use std::{
    path::Path,
    thread,
    time::{Duration, Instant},
};

pub const ORIGINAL_STARTUP_SETTLE_MS: u64 = 2_500;
const CROSSBOW_DIAGNOSTIC_FIELDS: &[u32] = &[7, 8, 13, 15, 59, 60, 95, 130];
const CROSSBOW_ARROW_DIAGNOSTIC_FIELDS: &[u32] = &[0, 1, 2, 3];

fn crossbow_contact_position(burst: &[serde_json::Value]) -> Option<[f32; 3]> {
    burst.iter().rev().find_map(|sample| {
        let children = sample["scene_tail_latest"]["children"].as_array()?;
        let arrow = children.iter().find(|child| {
            let offset = &child["ai"]["fix_block_offset"];
            offset["module"] == 17
                && offset["block"] == 0
                && offset["payload_offset"] == 0x4e324
                && child["dynamics"]["contact_classes"]["current"]
                    .as_u64()
                    .is_some_and(|value| value != 0)
        })?;
        let point = arrow["global_transform"]["Affine"]["translation"].as_array()?;
        Some([
            point.first()?.as_f64()? as f32,
            point.get(1)?.as_f64()? as f32,
            point.get(2)?.as_f64()? as f32,
        ])
    })
}

pub fn actor_sample_from_actor(
    name: String,
    planned_ms: u64,
    start_ms: u64,
    end_ms: u64,
    actor: ActorObservation,
) -> Result<ActorSample> {
    let affine = actor.global_transform.affine();
    let forward = affine.rotation[0];
    let heading_rad = f64::from(forward[1]).atan2(f64::from(forward[0]));
    ensure!(heading_rad.is_finite(), "actor heading is nonfinite");
    Ok(ActorSample {
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

#[cfg(target_os = "linux")]
pub struct OriginalBackend {
    controller: NativeController,
    config: Config,
    initial_actor: ActorSample,
    setup_interventions: Vec<serde_json::Value>,
    ready_state: OriginalReadyState,
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy)]
struct TrialCapture {
    pause_compensated: bool,
    extended: bool,
    projectile_burst: bool,
}

#[cfg(target_os = "linux")]
impl TrialCapture {
    const BASIC: Self = Self {
        pause_compensated: false,
        extended: false,
        projectile_burst: false,
    };
}

#[cfg(target_os = "linux")]
enum OriginalReadyState {
    FreshSave,
    CrossbowStart,
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
    /// Leave Hype's saved AI state intact while establishing the same
    /// quantity-three test input used by the Godot starting-state overlay.
    pub fn prepare_crossbow(&mut self) -> Result<()> {
        let reload = if matches!(self.ready_state, OriginalReadyState::Finished) {
            Some(self.reload_saved_state()?)
        } else {
            None
        };
        ensure!(
            matches!(self.ready_state, OriginalReadyState::FreshSave),
            "crossbow setup requires supplied manor save"
        );
        self.setup_interventions.clear();
        if let Some(reload) = reload {
            self.setup_interventions.push(reload);
        }
        let intervention = self.controller.game()?.prepare_crossbow_quantity()?;
        let actor = self.controller.game()?.observe_main_actor()?;
        ensure!(
            actor.current_action == Some(IDLE_ACTION),
            "crossbow starting state changed Hype's saved idle action"
        );
        self.setup_interventions.push(intervention);
        self.ready_state = OriginalReadyState::CrossbowStart;
        Ok(())
    }

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
        let before = actor_sample_from_actor(
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
        let after = actor_sample_from_actor(
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
            actor_sample_from_actor("supplied_save_start".into(), 0, 0, 0, settled_actor)?;
        Ok(Self {
            controller,
            config,
            initial_actor,
            setup_interventions: vec![],
            ready_state: OriginalReadyState::FreshSave,
        })
    }

    fn reload_saved_state(&mut self) -> Result<serde_json::Value> {
        let before = actor_sample_from_actor(
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
        let after = actor_sample_from_actor(
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
            let current = actor_sample_from_actor(
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
        let before = actor_sample_from_actor("setup_before".into(), 0, 0, 0, before)?;
        let mut after = actor_sample_from_actor("setup_after".into(), 0, 0, 0, after)?;
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
            after = actor_sample_from_actor(
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
        let before = actor_sample_from_actor(
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
        let mut after = actor_sample_from_actor(
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
            after = actor_sample_from_actor(
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
        let contact = actor_sample_from_actor(
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
        let start = actor_sample_from_actor(
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
        capture: TrialCapture,
    ) -> Result<(
        Vec<DeliveredInput>,
        Vec<ActorSample>,
        Option<serde_json::Value>,
    )> {
        let TrialCapture {
            pause_compensated,
            extended: observe_magick,
            projectile_burst: capture_projectile_burst,
        } = capture;
        self.controller.game()?.prepare_physical_input()?;
        let origin = Instant::now();
        let mut paused_total = Duration::ZERO;
        let mut delivered = vec![];
        let mut samples = vec![];
        let mut held_screenshot = None;
        let mut projectile_burst = vec![];
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
                if capture_projectile_burst && matches!(name, "attack_press" | "attack_release") {
                    let (first, count) = if name == "attack_press" {
                        (0, 4)
                    } else {
                        (4, 21)
                    };
                    for local_index in 0..count {
                        if local_index > 0 {
                            thread::sleep(Duration::from_millis(8));
                        }
                        let observation = self.controller.game()?.observe_latest_projectile()?;
                        ensure!(
                            observation.status.phase == 9 && observation.status.level == "manoir",
                            "game left the manor during arrow observation"
                        );
                        let paused = Duration::from_micros(u64::try_from(
                            observation.pause.pause_duration_micros,
                        )?);
                        paused_total += paused;
                        projectile_burst.push(serde_json::json!({
                            "index": first + local_index,
                            "elapsed_resumed_ms": origin.elapsed().saturating_sub(paused_total).as_millis(),
                            "pause_micros": paused.as_micros(),
                            "scene_tail_latest": observation.scene_tail,
                            "capture": observation.capture,
                            "completed_update": observation.completed_update,
                            "consistency_limit": observation.consistency_limit,
                        }));
                    }
                }
            } else {
                let capture_start_ms =
                    origin.elapsed().saturating_sub(paused_total).as_millis() as u64;
                let (actor, paused, magick_capture) = if observe_magick {
                    let designer_values = if capture_projectile_burst {
                        CROSSBOW_DIAGNOSTIC_FIELDS
                    } else {
                        &[59]
                    };
                    let observation = self.controller.game()?.observe_magick_behavior(
                        &[47, 48, 290, 271],
                        designer_values,
                        Some(2),
                        capture_projectile_burst,
                        capture_projectile_burst.then_some((17, 0, 0x4e324)),
                        if capture_projectile_burst {
                            CROSSBOW_ARROW_DIAGNOSTIC_FIELDS
                        } else {
                            &[]
                        },
                        (capture_projectile_burst && name == "flight_recorded")
                            .then(|| crossbow_contact_position(&projectile_burst))
                            .flatten()
                            .map(|point| (point, 3.0)),
                    )?;
                    ensure!(
                        observation.status.phase == 9 && observation.status.level == "manoir",
                        "game left the scenario level during extended observation"
                    );
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
                        "inventory": observation.inventory,
                        "source_status": observation.status,
                        "source_ai": observation.actor,
                        "crossbow_global": observation.crossbow_global,
                        "source_model_733_instances": observation.scene_ai_model,
                        "source_scene_tail": observation.scene_tail,
                        "nearby_scene_actors": observation.nearby_scene_actors,
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
                if !observe_magick {
                    let state = self.controller.game()?.state()?;
                    ensure!(
                        state["phase"] == 9 && state["level"] == "manoir",
                        "game left the scenario level: {state}"
                    );
                }
                let capture_end_ms =
                    origin.elapsed().saturating_sub(paused_total).as_millis() as u64;
                let mut sample = actor_sample_from_actor(
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
                if capture_projectile_burst && name == "flight_recorded" {
                    sample.raw_observation["source_projectile_burst"] =
                        serde_json::json!(projectile_burst);
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
                    | (
                        OriginalReadyState::CrossbowStart,
                        Scenario::CrossbowInitialLaunch
                    )
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
                | Scenario::CrossbowInitialLaunch
                | Scenario::FireCast
        );
        let (delivered, mut samples, _) = self.execute_trial(
            &recording.inputs,
            &recording.samples,
            "",
            None,
            TrialCapture {
                pause_compensated,
                extended: matches!(
                    recording.scenario,
                    Scenario::FireCast | Scenario::CrossbowInitialLaunch
                ),
                projectile_burst: recording.scenario == Scenario::CrossbowInitialLaunch,
            },
        )?;
        if recording.scenario == Scenario::CrossbowInitialLaunch {
            // The launch tape ends shortly after the 25-stop fast burst. Keep
            // that tape fixed, then take one later identity-checked snapshot
            // to see whether model 733 remains, changes behavior, or disappears.
            thread::sleep(Duration::from_millis(250));
            let observation = self.controller.game()?.observe_magick_behavior(
                &[47, 48, 290, 271],
                CROSSBOW_DIAGNOSTIC_FIELDS,
                Some(4),
                true,
                Some((17, 0, 0x4e324)),
                CROSSBOW_ARROW_DIAGNOSTIC_FIELDS,
                None,
            )?;
            let flight = samples
                .last_mut()
                .context("Crossbow launch has no terminal flight sample")?;
            flight.raw_observation["crossbow_post_burst_diagnostic"] =
                serde_json::to_value(observation)?;
        }
        let screenshot = self.controller.game()?.screenshot()?;
        let status = "recorded";
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
            assertions: vec![],
            screenshot: Some(screenshot),
            cleanup_complete: false,
            error: None,
        };
        self.ready_state = if result.scenario == Scenario::TurnReleaseIdle {
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
        let (mut delivered, mut samples, _) = self.execute_trial(
            &walk.inputs,
            &walk.samples,
            "walk_",
            None,
            TrialCapture::BASIC,
        )?;
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
            TrialCapture::BASIC,
        )?;
        delivered.extend(run_delivered);
        samples.extend(run_samples);
        let screenshot = self.controller.game()?.screenshot()?;
        let status = "recorded";
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
            assertions: vec![],
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
            TrialCapture::BASIC,
        )?;
        let status = "recorded";
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
            assertions: vec![],
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
