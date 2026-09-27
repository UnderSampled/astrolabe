//! The supplied-save Fire cast after a physical sword-selection setup.
//! Effect identity, contact and lifetime require separate observations.
use super::*;

const SELECTING_SWORD: u32 = 45;
const ARMED_IDLE: u32 = 48;
const CAST_ACTION: u32 = 290;
const FOLLOW_THROUGH: u32 = 271;

pub fn recording() -> FunctionalRecording {
    let edge = |at_ms, id: &str, key, edge| TimedInput {
        at_ms,
        id: id.into(),
        event: Event {
            keyboard: 0,
            key,
            edge,
        },
    };
    FunctionalRecording {
        schema: 1,
        scenario: Scenario::FireCast,
        ready_state: "manoir-clear-ground-after-physical-down".into(),
        inputs: vec![
            edge(200, "sword_press", Key::X, Edge::Press),
            edge(320, "sword_release", Key::X, Edge::Release),
            edge(1_400, "cast_press", Key::Enter, Edge::Press),
            edge(1_500, "cast_release", Key::Enter, Edge::Release),
        ],
        samples: [
            (0, "baseline"),
            (1_100, "armed"),
            (1_700, "cast_early"),
            (1_900, "cast_late"),
            (2_300, "spent"),
            (3_100, "settled"),
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
            && recording.ready_state == expected.ready_state
            && recording.inputs == expected.inputs
            && recording.samples == expected.samples,
        "Fire cast requires a bounded X selection and one Enter pulse from clear ground"
    );
    Ok(())
}

fn mana(sample: &ActorSample) -> f64 {
    sample.raw_observation["magick"]["current_points"]
        .as_f64()
        .unwrap_or(f64::NAN)
}

pub fn assertions(samples: &[ActorSample]) -> Result<Vec<Assertion>> {
    ensure!(samples.len() == 6, "Fire cast needs six observations");
    ensure!(
        samples
            .iter()
            .map(|sample| sample.name.as_str())
            .eq(self::recording()
                .samples
                .iter()
                .map(|point| point.name.as_str())),
        "Fire cast observations changed order"
    );
    let [base, armed, early, late, spent, settled] = samples else {
        unreachable!()
    };
    let source = "Orrery 2026-09-24 supplied manor save: Down800, X120, Enter100; action 290 then 271, mana 30 to 20";
    let cast_seen = [early, late]
        .iter()
        .any(|sample| sample.action == Some(CAST_ACTION))
        || settled.raw_observation["action_states"]
            .as_array()
            .is_some_and(|states| states.iter().any(|state| state["action"] == CAST_ACTION));
    Ok(vec![
        bound(
            "fire_baseline_idle",
            action_is(base, IDLE_ACTION),
            Some(1.0),
            Some(1.0),
            source,
        ),
        bound(
            "fire_baseline_mana",
            mana(base),
            Some(30.0),
            Some(30.0),
            source,
        ),
        bound(
            "fire_selected_code",
            armed.raw_observation["magick"]["selected_spell_code"]
                .as_f64()
                .unwrap_or(f64::NAN),
            Some(61.0),
            Some(61.0),
            "Verified function-232 selected MagicFire inventory code 61",
        ),
        bound(
            "fire_armed_selection_phase",
            f64::from(u8::from(matches!(
                armed.action,
                Some(SELECTING_SWORD | ARMED_IDLE)
            ))),
            Some(1.0),
            Some(1.0),
            "2026-09-24 Fire BDD original capture: action 45 with dsg_59 true at the armed checkpoint; action 48 appears after the cast",
        ),
        bound(
            "fire_cast_gate_ready",
            f64::from(u8::from(
                armed.raw_observation["designer_fields"]["59"] == true,
            )),
            Some(1.0),
            Some(1.0),
            source,
        ),
        bound(
            "fire_cast_action",
            f64::from(u8::from(cast_seen)),
            Some(1.0),
            Some(1.0),
            source,
        ),
        bound(
            "fire_mana_spent",
            mana(spent),
            Some(20.0),
            Some(20.0),
            source,
        ),
        bound(
            "fire_follow_through",
            action_is(spent, FOLLOW_THROUGH),
            Some(1.0),
            Some(1.0),
            source,
        ),
        bound(
            "fire_settled_mana",
            mana(settled),
            Some(20.0),
            Some(20.0),
            source,
        ),
    ])
}
