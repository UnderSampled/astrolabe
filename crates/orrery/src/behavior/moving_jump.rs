//! A moving Ctrl jump from the verified clear-route turn. Up remains held
//! through the jump edge, so releasing and re-pressing it cannot select the
//! ordinary walk-stop action in place of the moving jump branch.
use super::*;

pub(super) fn recording() -> FunctionalRecording {
    let edge = |at_ms, id: &str, key, event| TimedInput {
        at_ms,
        id: id.into(),
        event: Event {
            keyboard: 0,
            key,
            edge: event,
        },
    };
    FunctionalRecording {
        schema: 1,
        scenario: Scenario::MovingJump,
        ready_state: "manoir-clear-route-after-physical-left-turn".into(),
        inputs: vec![
            edge(200, "walk_press", Key::ArrowUp, Edge::Press),
            edge(500, "jump_press", Key::LeftControl, Edge::Press),
            edge(600, "jump_release", Key::LeftControl, Edge::Release),
            edge(700, "walk_release", Key::ArrowUp, Edge::Release),
        ],
        samples: [
            (0, "baseline"),
            (450, "walking"),
            (800, "rise"),
            (1_100, "high"),
            (1_400, "descent"),
            (1_900, "floor"),
            (2_400, "settled_start"),
            (2_800, "settled_end"),
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

pub(super) fn validate(recording: &FunctionalRecording) -> Result<()> {
    let expected = self::recording();
    ensure!(
        recording.preparatory_trial.is_none()
            && recording.inputs == expected.inputs
            && recording.samples == expected.samples,
        "moving jump requires uninterrupted short walk, bounded jump pulse and eight observations"
    );
    Ok(())
}

pub(super) fn assertions(samples: &[ActorSample]) -> Result<Vec<Assertion>> {
    ensure!(samples.len() == 8, "moving jump needs eight observations");
    let [base, walk, rise, high, descent, floor, start, end] = samples else {
        unreachable!()
    };
    ensure!(
        samples
            .iter()
            .map(|sample| sample.name.as_str())
            .eq(self::recording()
                .samples
                .iter()
                .map(|point| point.name.as_str())),
        "moving jump observations changed order"
    );
    let source = "Orrery 2026-09-24 clear-route physical Up then Ctrl, with Up continuously held through takeoff";
    let height = |sample: &ActorSample| f64::from(sample.translation[2] - base.translation[2]);
    let walk_dx = f64::from(walk.translation[0] - base.translation[0]);
    let walk_dy = f64::from(walk.translation[1] - base.translation[1]);
    let walk_distance = walk_dx.hypot(walk_dy);
    let forward_after_walk = |sample: &ActorSample| {
        if walk_distance <= 0.0001 {
            return 0.0;
        }
        let dx = f64::from(sample.translation[0] - walk.translation[0]);
        let dy = f64::from(sample.translation[1] - walk.translation[1]);
        (dx * walk_dx + dy * walk_dy) / walk_distance
    };
    let mut checks = vec![
        bound(
            "moving_jump_baseline_idle",
            action_is(base, IDLE_ACTION),
            Some(1.),
            Some(1.),
            source,
        ),
        bound(
            "moving_jump_walking_action",
            walking_action_is_active(walk),
            Some(1.),
            Some(1.),
            source,
        ),
        bound(
            "moving_jump_walk_progress",
            walk_distance,
            Some(0.1),
            None,
            source,
        ),
        bound(
            "moving_jump_walk_level",
            height(walk).abs(),
            None,
            Some(0.1),
            source,
        ),
        bound(
            "moving_jump_launch_action",
            action_is(rise, 7),
            Some(1.),
            Some(1.),
            source,
        ),
        bound(
            "moving_jump_rise_height",
            height(rise),
            Some(0.5),
            None,
            source,
        ),
        bound(
            "moving_jump_sampled_gain",
            height(rise).max(height(high)).max(height(descent)),
            Some(1.0),
            None,
            source,
        ),
        bound(
            "moving_jump_forward_in_air",
            forward_after_walk(rise),
            Some(0.5),
            None,
            source,
        ),
        bound(
            "moving_jump_floor_return",
            height(floor).abs(),
            None,
            Some(0.2),
            source,
        ),
        bound(
            "moving_jump_settled_height",
            height(end).abs(),
            None,
            Some(0.2),
            source,
        ),
        bound(
            "moving_jump_settled_idle_start",
            action_is(start, IDLE_ACTION),
            Some(1.),
            Some(1.),
            source,
        ),
        bound(
            "moving_jump_settled_idle_end",
            action_is(end, IDLE_ACTION),
            Some(1.),
            Some(1.),
            source,
        ),
        bound(
            "moving_jump_post_landing_drift",
            distance_xy(start, end),
            None,
            Some(0.3),
            source,
        ),
    ];
    if rise.raw_observation.get("on_floor").is_some() {
        checks.push(bound(
            "moving_jump_native_airborne",
            f64::from(u8::from(rise.raw_observation["on_floor"] == false)),
            Some(1.),
            Some(1.),
            "Godot native CharacterBody3D airborne result",
        ));
    }
    if floor.raw_observation.get("on_floor").is_some() {
        checks.push(bound(
            "moving_jump_native_floor_contact",
            f64::from(u8::from(floor.raw_observation["on_floor"] == true)),
            Some(1.),
            Some(1.),
            "Godot native CharacterBody3D floor result",
        ));
    }
    Ok(checks)
}
