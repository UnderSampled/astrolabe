//! One physically selected sword swing from clear ground in the manor.
//! Target damage and miss behavior are separate evidence and acceptance work.
use super::*;

const EQUIP_ACTION: u32 = 45;
const ATTACK_ACTION: u32 = 134;
const ATTACK_FOLLOW_THROUGH_ACTION: u32 = 135;
const ARMED_IDLE_ACTION: u32 = 48;

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
        scenario: Scenario::SwordAttack,
        ready_state: "manoir-clear-ground-after-physical-down".into(),
        inputs: vec![
            edge(200, "sword_press", Key::X, Edge::Press),
            edge(320, "sword_release", Key::X, Edge::Release),
            edge(1_400, "attack_press", Key::Space, Edge::Press),
            edge(1_500, "attack_release", Key::Space, Edge::Release),
        ],
        samples: [
            (0, "baseline"),
            (1_100, "equipped"),
            (1_700, "attack_early"),
            (2_000, "attack_late"),
            (2_700, "recovered"),
            (3_200, "settled"),
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
            && recording.ready_state == expected.ready_state
            && recording.inputs == expected.inputs
            && recording.samples == expected.samples,
        "sword attack requires a bounded X selection and one Space pulse from clear ground"
    );
    Ok(())
}

pub(super) fn assertions(samples: &[ActorSample]) -> Result<Vec<Assertion>> {
    ensure!(samples.len() == 6, "sword attack needs six observations");
    let [base, equipped, early, late, recovered, settled] = samples else {
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
        "sword observations changed order"
    );
    let source = "Orrery 2026-09-24 supplied manor save, physical Down800 then X120, Space100 away from save point; source 45 at draw, 134 or 135 during swing, then 48";
    let mut checks = vec![
        bound(
            "sword_baseline_idle",
            action_is(base, IDLE_ACTION),
            Some(1.0),
            Some(1.0),
            source,
        ),
        bound(
            "sword_equipping_action",
            action_is(equipped, EQUIP_ACTION),
            Some(1.0),
            Some(1.0),
            source,
        ),
        bound(
            "sword_attack_early_phase",
            f64::from(u8::from(matches!(
                early.action,
                Some(ATTACK_ACTION | ATTACK_FOLLOW_THROUGH_ACTION)
            ))),
            Some(1.0),
            Some(1.0),
            source,
        ),
        bound(
            "sword_attack_late_phase",
            f64::from(u8::from(matches!(
                late.action,
                Some(ATTACK_ACTION | ATTACK_FOLLOW_THROUGH_ACTION | ARMED_IDLE_ACTION)
            ))),
            Some(1.0),
            Some(1.0),
            source,
        ),
        bound(
            "sword_recovered_idle",
            action_is(recovered, ARMED_IDLE_ACTION),
            Some(1.0),
            Some(1.0),
            source,
        ),
        bound(
            "sword_settled_idle",
            action_is(settled, ARMED_IDLE_ACTION),
            Some(1.0),
            Some(1.0),
            source,
        ),
        bound(
            "sword_stationary_xy",
            distance_xy(base, settled),
            None,
            Some(0.5),
            source,
        ),
        bound(
            "sword_post_attack_drift",
            distance_xy(recovered, settled),
            None,
            Some(0.2),
            source,
        ),
    ];
    if equipped.raw_observation.get("on_floor").is_some() {
        checks.push(bound(
            "sword_native_floor_contact",
            f64::from(u8::from(
                equipped.raw_observation["on_floor"] == true
                    && settled.raw_observation["on_floor"] == true,
            )),
            Some(1.0),
            Some(1.0),
            source,
        ));
    }
    if base.raw_observation.get("channel_visibility").is_some() {
        checks.push(bound(
            "sword_saved_weapon_channel_baseline",
            f64::from(u8::from(
                [28, 71, 80]
                    .into_iter()
                    .all(|channel| channel_active(base, channel) == Some(false))
                    && [72, 116, 136]
                        .into_iter()
                        .all(|channel| channel_active(base, channel) == Some(true)),
            )),
            Some(1.0),
            Some(1.0),
            "Orrery 2026-09-24 supplied save: source channel bits and persistent deactivation list",
        ));
    }
    if let Some(trace) = settled.raw_observation["generated_trace"].as_array() {
        let launch = trace.iter().find(|entry| entry["event"] == "frame");
        let exit = trace.iter().find(|entry| entry["event"] == "tree_exiting");
        let generated = &settled.raw_observation["generated_object_count"];
        checks.push(bound(
            "sword_beam_compiled_brain_runs",
            f64::from(u8::from(
                generated.as_u64() == Some(1)
                    && launch.is_some_and(|entry| {
                        entry["brain_running"] == true
                            && entry["brain_normal"] == 1
                            && entry["brain_failure"] == ""
                    }),
            )),
            Some(1.0),
            Some(1.0),
            "Imported beam AI model 708 starts and enters normal behavior 1",
        ));
        let motion = launch
            .zip(exit)
            .and_then(|(first, last)| {
                let first_y = first["translation"][1].as_f64()?;
                let last_y = last["translation"][1].as_f64()?;
                (first["instance_id"] == last["instance_id"]).then_some(last_y - first_y)
            })
            .unwrap_or(f64::NAN);
        checks.push(bound(
            "sword_beam_moves_before_cleanup",
            motion,
            Some(0.5),
            Some(4.0),
            "Imported beam AI model 708 calls GoRelative before its 100 ms lifetime ends",
        ));
        checks.push(bound(
            "sword_beam_scripted_lifetime",
            exit.and_then(|entry| entry["brain_clock_ms"].as_f64())
                .unwrap_or(f64::NAN),
            Some(90.0),
            Some(120.0),
            "Imported beam AI model 708 calls KillPerso after TimeElapsed(100)",
        ));
    }
    Ok(checks)
}
