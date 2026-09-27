//! Stationary Ctrl jump observed from the supplied manor save after physical
//! backward movement reaches a clear grounded start.
use super::*;

pub fn recording() -> FunctionalRecording {
    FunctionalRecording {
        schema: 1,
        scenario: Scenario::StationaryJump,
        ready_state: "manoir-clear-ground-after-physical-down".into(),
        inputs: [
            (200, "jump_press", Edge::Press),
            (300, "jump_release", Edge::Release),
        ]
        .into_iter()
        .map(|(at_ms, id, edge)| TimedInput {
            at_ms,
            id: id.into(),
            event: Event {
                keyboard: 0,
                key: Key::LeftControl,
                edge,
            },
        })
        .collect(),
        samples: [
            (0, "baseline"),
            (550, "rise"),
            (800, "high"),
            (1_100, "descent"),
            (1_600, "floor"),
            (2_100, "settled_start"),
            (2_500, "settled_end"),
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
    ensure!(
        recording.preparatory_trial.is_none(),
        "jump has an unexpected trial"
    );
    ensure!(
        recording.inputs == self::recording().inputs,
        "jump input schedule changed"
    );
    ensure!(
        recording
            .samples
            .iter()
            .map(|p| (p.at_ms, p.name.as_str()))
            .eq([
                (0, "baseline"),
                (550, "rise"),
                (800, "high"),
                (1_100, "descent"),
                (1_600, "floor"),
                (2_100, "settled_start"),
                (2_500, "settled_end"),
            ]),
        "jump observation schedule changed"
    );
    Ok(())
}

pub fn assertions(samples: &[ActorSample]) -> Result<Vec<Assertion>> {
    ensure!(
        samples.len() == 7,
        "stationary jump requires seven observations"
    );
    let [base, rise, high, descent, floor, start, end] = samples else {
        unreachable!()
    };
    ensure!(
        samples.iter().map(|s| s.name.as_str()).eq([
            "baseline",
            "rise",
            "high",
            "descent",
            "floor",
            "settled_start",
            "settled_end",
        ]),
        "jump samples changed order"
    );
    let source = "Orrery 2026-09-24 supplied manoir save, physical Down800 then Control_L100; bounded sampled rise and return";
    let rise_z = f64::from(rise.translation[2] - base.translation[2]);
    let high_z = f64::from(high.translation[2] - base.translation[2]);
    let descent_z = f64::from(descent.translation[2] - base.translation[2]);
    let mut checks = vec![
        bound(
            "jump_baseline_idle",
            action_is(base, IDLE_ACTION),
            Some(1.),
            Some(1.),
            source,
        ),
        bound(
            "jump_rise_action",
            f64::from(u8::from(matches!(rise.action, Some(2 | 4)))),
            Some(1.),
            Some(1.),
            source,
        ),
        bound("jump_rise_height", rise_z, Some(0.25), None, source),
        bound(
            "jump_sampled_gain",
            rise_z.max(high_z).max(descent_z),
            Some(1.0),
            None,
            source,
        ),
        bound(
            "jump_floor_return",
            f64::from((floor.translation[2] - base.translation[2]).abs()),
            None,
            Some(0.15),
            source,
        ),
        bound(
            "jump_settled_height",
            f64::from((end.translation[2] - base.translation[2]).abs()),
            None,
            Some(0.15),
            source,
        ),
        bound(
            "jump_settled_idle_start",
            action_is(start, IDLE_ACTION),
            Some(1.),
            Some(1.),
            source,
        ),
        bound(
            "jump_settled_idle_end",
            action_is(end, IDLE_ACTION),
            Some(1.),
            Some(1.),
            source,
        ),
        bound(
            "jump_stationary_xy",
            distance_xy(base, end),
            None,
            Some(0.5),
            source,
        ),
        bound(
            "jump_post_landing_drift",
            distance_xy(start, end),
            None,
            Some(0.3),
            source,
        ),
    ];
    if floor.raw_observation.get("on_floor").is_some() {
        checks.push(bound(
            "jump_native_floor_contact",
            f64::from(u8::from(floor.raw_observation["on_floor"] == true)),
            Some(1.),
            Some(1.),
            "Godot native CharacterBody3D floor result",
        ));
    }
    if end.raw_observation.get("generated_object_count").is_some() {
        checks.push(bound(
            "jump_native_object_generated_once",
            end.raw_observation["generated_object_count"]
                .as_u64()
                .map_or(-1.0, |count| count as f64),
            Some(1.0),
            Some(1.0),
            "Godot native PackedScene generation count for reached landing action",
        ));
    }
    Ok(checks)
}
