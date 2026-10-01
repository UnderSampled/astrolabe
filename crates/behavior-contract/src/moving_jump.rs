//! A moving Ctrl jump from the verified clear-route turn. Up remains held
//! through takeoff, is released, then is pressed again while Hype is airborne
//! to exercise relative steering with a fresh input edge.
use super::*;

pub fn recording() -> FunctionalRecording {
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
            edge(850, "air_steer_press", Key::ArrowUp, Edge::Press),
            edge(1_050, "air_steer_release", Key::ArrowUp, Edge::Release),
        ],
        samples: [
            (0, "baseline"),
            (450, "walking"),
            (800, "rise"),
            (1_000, "high"),
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

pub fn validate(recording: &FunctionalRecording) -> Result<()> {
    let expected = self::recording();
    ensure!(
        recording.preparatory_trial.is_none()
            && recording.inputs == expected.inputs
            && recording.samples == expected.samples,
        "moving jump requires held-forward takeoff, bounded midair steering, and eight observations"
    );
    Ok(())
}

pub fn assertions(samples: &[ActorSample]) -> Result<Vec<Assertion>> {
    let mut checks = assertions_with_walk_bound(
        samples,
        0.1,
        "Orrery clear-route physical Up held through takeoff, then Up pressed again during the airborne phase",
    )?;
    let [base, walk, rise, high, ..] = samples else {
        unreachable!()
    };
    if high.raw_observation.get("external_velocity").is_some() {
        let horizontal_velocity = |sample: &ActorSample| -> Result<(f64, f64)> {
            let velocity = sample.raw_observation["external_velocity"]
                .as_array()
                .context("Godot jump observation lacks an external_velocity vector")?;
            ensure!(
                velocity.len() == 3,
                "Godot external_velocity needs three components"
            );
            Ok((
                velocity[0]
                    .as_f64()
                    .context("Godot velocity x is not numeric")?,
                -velocity[2]
                    .as_f64()
                    .context("Godot velocity z is not numeric")?,
            ))
        };
        let (rise_x, rise_y) = horizontal_velocity(rise)?;
        let (high_x, high_y) = horizontal_velocity(high)?;
        let steer_x = high_x - rise_x;
        let steer_y = high_y - rise_y;
        let steer_length = steer_x.hypot(steer_y);
        let walk_dx = f64::from(walk.translation[0] - base.translation[0]);
        let walk_dy = f64::from(walk.translation[1] - base.translation[1]);
        let walk_length = walk_dx.hypot(walk_dy);
        // Require a measurable response, not incidental horizontal drift.
        let alignment = if steer_length <= 0.5 || walk_length <= 0.0001 {
            0.0
        } else {
            (steer_x * walk_dx + steer_y * walk_dy) / (steer_length * walk_length)
        };
        checks.push(bound(
            "moving_jump_airborne_steering_follows_walk_heading",
            alignment,
            Some(0.99),
            None,
            "Godot horizontal velocity change during airborne Up input follows Hype's relative walk heading",
        ));
        checks.push(bound(
            "moving_jump_native_steering_airborne",
            f64::from(u8::from(high.raw_observation["on_floor"] == false)),
            Some(1.0),
            Some(1.0),
            "Godot native CharacterBody3D is airborne while Up is held",
        ));
    }
    Ok(checks)
}

pub fn assertions_with_walk_bound(
    samples: &[ActorSample],
    walk_progress_lower: f64,
    source: &str,
) -> Result<Vec<Assertion>> {
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
            Some(walk_progress_lower),
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
