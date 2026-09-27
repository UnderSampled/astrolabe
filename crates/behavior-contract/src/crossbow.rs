//! Initial crossbow launch through Hype's and the arrow's compiled AI.
//! Impact, contact cleanup, ammunition parity and camera effects are separate work.
use super::*;

const ARROW_MODEL: &str = "fix:/ai/models/733";
const ARROW_MODEL_OFFSET: u64 = 0x4e324;

pub fn recording() -> FunctionalRecording {
    let edge = |at_ms, id: &str, key, direction| TimedInput {
        at_ms,
        id: id.into(),
        event: Event {
            keyboard: 0,
            key,
            edge: direction,
        },
    };
    FunctionalRecording {
        schema: 1,
        scenario: Scenario::CrossbowInitialLaunch,
        ready_state: "manoir-crossbow-quantity-three".into(),
        inputs: vec![
            edge(200, "select_press", Key::Z, Edge::Press),
            edge(255, "select_release", Key::Z, Edge::Release),
            edge(1100, "attack_press", Key::Space, Edge::Press),
            edge(1180, "attack_release", Key::Space, Edge::Release),
        ],
        samples: [
            (0, "baseline"),
            (1000, "selected"),
            (1600, "flight_recorded"),
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
        "crossbow launch requires physical Z55, an armed checkpoint, and Space80 from quantity three"
    );
    Ok(())
}

fn position(value: &serde_json::Value) -> Option<[f64; 3]> {
    let values = value.as_array()?;
    Some([
        values.first()?.as_f64()?,
        values.get(1)?.as_f64()?,
        values.get(2)?.as_f64()?,
    ])
}

fn displacement(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>()
        .sqrt()
}

fn original_arrow(sample: &serde_json::Value) -> Option<(u64, String, u64, [f64; 3])> {
    let children = sample["scene"]["children"].as_array()?;
    children.iter().find_map(|child| {
        let offset = &child["ai"]["fix_block_offset"];
        (offset["module"] == 17
            && offset["block"] == 0
            && offset["payload_offset"].as_u64() == Some(ARROW_MODEL_OFFSET))
        .then(|| {
            Some((
                child["index"].as_u64()?,
                child["node_locator"].as_str()?.to_owned(),
                child["update_stamp"].as_u64()?,
                position(&child["global_transform"]["Affine"]["translation"])?,
            ))
        })
        .flatten()
    })
}

pub fn assertions(samples: &[ActorSample]) -> Result<Vec<Assertion>> {
    ensure!(
        samples.len() == 3,
        "crossbow launch needs three observations"
    );
    ensure!(
        samples.iter().map(|sample| sample.name.as_str()).eq([
            "baseline",
            "selected",
            "flight_recorded"
        ]),
        "crossbow observations changed order"
    );
    let [baseline, selected, flight] = samples else {
        unreachable!()
    };
    let raw = &flight.raw_observation;
    let godot = raw.get("generated_trace_first").is_some();
    let starting_quantity = if godot {
        &baseline.raw_observation["inventory"]["weapon_80_quantity"]
    } else {
        baseline.raw_observation["inventory"]["active"]
            .as_array()
            .and_then(|members| {
                members
                    .iter()
                    .find(|member| member["kind"] == "Weapon" && member["id"] == 80)
            })
            .map(|member| &member["quantity"])
            .unwrap_or(&serde_json::Value::Null)
    };
    let source = "Verified saved Weapon 80 quantity zero plus a test-owned quantity-three start; Z55/gap100/Space80 is physical input";
    let mut checks = vec![bound(
        "crossbow_quantity_three_start",
        f64::from(u8::from(starting_quantity.as_u64() == Some(3))),
        Some(1.0),
        Some(1.0),
        source,
    )];
    checks.push(bound(
        "crossbow_armed_before_attack",
        f64::from(u8::from(selected.action == Some(48))),
        Some(1.0),
        Some(1.0),
        "Observed armed idle after physical Z selection and before physical Space",
    ));

    if godot {
        let trace = raw["generated_trace_first"]
            .as_array()
            .context("Godot launch has no preserved first generated frames")?;
        let frames = trace
            .iter()
            .filter(|entry| entry["event"] == "frame" && entry["source_model_path"] == ARROW_MODEL)
            .collect::<Vec<_>>();
        let launch = frames.first().copied();
        let moving_pair = launch.and_then(|first| {
            let id = &first["instance_id"];
            let start = position(&first["translation"])?;
            frames.iter().skip(1).find_map(|later| {
                (later["instance_id"] == *id)
                    .then(|| position(&later["translation"]).map(|end| displacement(start, end)))
                    .flatten()
                    .filter(|distance| *distance > 0.05)
            })
        });
        checks.push(bound(
            "crossbow_compiled_arrow_starts",
            f64::from(u8::from(launch.is_some_and(|entry| {
                entry["brain_running"] == true && entry["brain_failure"] == ""
            }))),
            Some(1.0),
            Some(1.0),
            "Compiled Hype model 28 creates model-733 arrow; arrow Brain runs at launch",
        ));
        checks.push(bound(
            "crossbow_initial_displacement",
            moving_pair.unwrap_or(-1.0),
            Some(0.05),
            None,
            "Initial native arrow movement only; later Cond_CollideWithNothing remains unsupported and visible in the trace",
        ));
    } else {
        let burst = raw["source_projectile_burst"]
            .as_array()
            .context("original launch has no bounded scene-tail burst")?;
        let candidates = burst.iter().filter_map(original_arrow).collect::<Vec<_>>();
        let moved = candidates.windows(2).find_map(|pair| {
            let (index_a, node_a, stamp_a, position_a) = &pair[0];
            let (index_b, node_b, stamp_b, position_b) = &pair[1];
            (index_a == index_b && node_a == node_b && *stamp_a > 0 && stamp_b > stamp_a)
                .then(|| displacement(*position_a, *position_b))
                .filter(|distance| *distance > 0.05)
        });
        checks.push(bound(
            "crossbow_original_model733_spawned",
            f64::from(u8::from(!candidates.is_empty())),
            Some(1.0),
            Some(1.0),
            "Checked live Fix module 17/block 0 model pointer at payload offset 0x4e324",
        ));
        checks.push(bound(
            "crossbow_initial_displacement",
            moved.unwrap_or(-1.0),
            Some(0.05),
            None,
            "Two published positions of the same checked original arrow before it disappears",
        ));
    }
    Ok(checks)
}
