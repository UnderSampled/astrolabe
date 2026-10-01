//! A moving jump from a physical near-wall start at the verified manor corner.
//! The wall case keeps its original forward input through takeoff. The
//! clear-route midair steering input is omitted to preserve its contact path.
use super::*;

pub fn recording() -> FunctionalRecording {
    let mut recording = moving_jump::recording();
    recording.scenario = Scenario::WallJump;
    recording.ready_state = "manoir-grounded-near-verified-corner-wall".into();
    recording.inputs.truncate(4);
    // Keep the established wall-contact observation window.
    recording.samples[3].at_ms = 1_100;
    recording.samples[4].at_ms = 1_400;
    recording
}

pub fn validate(recording: &FunctionalRecording) -> Result<()> {
    let expected = self::recording();
    ensure!(
        recording.preparatory_trial.is_none()
            && recording.ready_state == expected.ready_state
            && recording.inputs == expected.inputs
            && recording.samples == expected.samples,
        "wall jump requires the near-wall start and uninterrupted forward approach schedule"
    );
    Ok(())
}

pub fn validate_ready(contact: &ActorSample, start: &ActorSample) -> Result<()> {
    let contact_clearance = wall::corner_clearances(contact);
    let start_clearance = wall::corner_clearances(start);
    ensure!(
        contact_near_corner(contact),
        "physical wall approach missed the known corner: {contact:?}, clearance {contact_clearance:?}"
    );
    ensure!(
        start.action == Some(IDLE_ACTION)
            && (0.25..=1.5).contains(&start_clearance.0)
            && (0.25..=1.8).contains(&start_clearance.1)
            && (0.5..=2.5).contains(&distance_xy(contact, start))
            && heading_delta(contact, start) <= 0.2
            && (contact.translation[2] - start.translation[2]).abs() <= 0.4
            && start
                .raw_observation
                .get("on_floor")
                .is_none_or(|v| v == true),
        "physical wall retreat missed grounded jump start: {start:?}, clearance {start_clearance:?}"
    );
    Ok(())
}

pub fn contact_near_corner(contact: &ActorSample) -> bool {
    let clearance = wall::corner_clearances(contact);
    contact.action == Some(IDLE_ACTION)
        && (-0.08..=0.25).contains(&clearance.0)
        && (-0.08..=0.25).contains(&clearance.1)
}

pub fn assertions(samples: &[ActorSample]) -> Result<Vec<Assertion>> {
    let source =
        "Orrery 2026-09-24 physical near-wall manor route and source finite-face corner oracle";
    let mut checks = moving_jump::assertions_with_walk_bound(samples, 0.05, source)?;
    let clearances = samples
        .iter()
        .map(wall::corner_clearances)
        .collect::<Vec<_>>();
    let min_angled = clearances.iter().map(|v| v.0).fold(f64::INFINITY, f64::min);
    let min_cross = clearances.iter().map(|v| v.1).fold(f64::INFINITY, f64::min);
    let in_air_angled = clearances[2..=4]
        .iter()
        .map(|v| v.0)
        .fold(f64::INFINITY, f64::min);
    checks.extend([
        bound(
            "wall_jump_start_angled_clearance",
            clearances[0].0,
            Some(0.25),
            Some(1.5),
            source,
        ),
        bound(
            "wall_jump_start_cross_clearance",
            clearances[0].1,
            Some(0.25),
            Some(1.8),
            source,
        ),
        bound(
            "wall_jump_airborne_wall_approach",
            in_air_angled,
            None,
            Some(0.25),
            source,
        ),
        bound(
            "wall_jump_angled_no_passage",
            min_angled,
            Some(-0.08),
            None,
            source,
        ),
        bound(
            "wall_jump_cross_no_passage",
            min_cross,
            Some(-0.08),
            None,
            source,
        ),
    ]);
    if samples[2].raw_observation.get("slide_contacts").is_some() {
        let contact = samples[2..=4].iter().any(|sample| {
            if sample.raw_observation["on_floor"] != false {
                return false;
            }
            sample.raw_observation["slide_contacts"]
                .as_array()
                .is_some_and(|contacts| {
                    contacts
                        .iter()
                        .any(|entry| entry["source_node_id"] == "world/1/5/0")
                })
        });
        checks.push(bound(
            "wall_jump_native_named_airborne_contact",
            f64::from(u8::from(contact)),
            Some(1.0),
            Some(1.0),
            source,
        ));
        if samples[3].raw_observation.get("slope_contact").is_some() {
            let steep_wall_is_not_slope = samples[3..=4].iter().all(|sample| {
                sample.raw_observation["on_floor"] == false
                    && sample.raw_observation["ground_contact"] == false
                    && sample.raw_observation["slope_contact"] == false
            });
            checks.push(bound(
                "wall_jump_native_wall_not_floor_or_slope",
                f64::from(u8::from(steep_wall_is_not_slope)),
                Some(1.0),
                Some(1.0),
                source,
            ));
        }
    }
    Ok(checks)
}
