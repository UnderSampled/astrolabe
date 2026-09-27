//! Source-owned finite-face oracle for the first manor corner contact case.
//! The face vertices and three movement spheres are documented in
//! notes/wall-contact-2026-09-24.md. This checks recorded positions; it does
//! not claim sub-sample continuous collision reconstruction.
use super::{
    ActorSample, Assertion, FunctionalRecording, IDLE_ACTION, SamplePoint, Scenario, TimedInput,
    WALK_FORWARD_ACTION, action_is, bound, distance_xy, heading_error_to, walking_action_is_active,
};
use crate::input::{Edge, Event, Key};
use anyhow::{Result, ensure};
use serde_json::{Value, json};

pub const TARGET_HEADING_RAD: f64 = -2.40;
pub const HEADING_TOLERANCE_RAD: f64 = 0.05;
const RADIUS: f64 = 0.55;
const OFFSETS: [f64; 3] = [0.53, 1.10, 1.65];
const START: [f64; 3] = [-74.37866, 27.48651, 4.55087];
const ANGLED: [[f64; 3]; 3] = [
    [-80.609_024, 32.603_770, 5.035_125],
    [-79.015_520, 28.686_500, 4.536_334],
    [-79.317_210, 28.002_354, 10.880_944],
];
const CROSS: [[f64; 3]; 3] = [
    [-80.609_024, 32.603_770, 5.035_125],
    [-78.642_820, 32.603_714, 10.114_957],
    [-78.642_810, 32.603_714, 5.035_129],
];
const SOURCE: &str = "source manoir world/1/5/0 finite faces: material 7/group 1/triangle 1 and adjacent material 6/group 0/triangle 6; Hype movement zones 196716/196724/196736, radius 0.55; Orrery 2026-09-24 corner probe";

pub fn heading_error(actual: f64) -> f64 {
    heading_error_to(TARGET_HEADING_RAD, actual)
}

pub fn oracle() -> Value {
    json!({
        "method": "source_finite_corner_oracle",
        "source_node_id": "world/1/5/0",
        "angled": {"material": "level:/models/world/1/5/0/materials/7", "group": 1, "triangle": 1, "vertices": ANGLED},
        "cross": {"material": "level:/models/world/1/4/0/materials/6", "group": 0, "triangle": 6, "vertices": CROSS},
        "movement_sphere_radius": RADIUS,
        "movement_sphere_vertical_offsets": OFFSETS,
        "evidence": "notes/wall-contact-2026-09-24.md",
        "claim_scope": "sampled finite-face clearance and native contact",
    })
}

pub fn recording() -> FunctionalRecording {
    FunctionalRecording {
        schema: 1,
        scenario: Scenario::WallContact,
        ready_state: "manoir-corner-after-physical-left-turn".into(),
        inputs: vec![
            TimedInput {
                at_ms: 200,
                id: "wall_walk_press".into(),
                event: Event {
                    keyboard: 0,
                    key: Key::ArrowUp,
                    edge: Edge::Press,
                },
            },
            TimedInput {
                at_ms: 8_000,
                id: "wall_walk_release".into(),
                event: Event {
                    keyboard: 0,
                    key: Key::ArrowUp,
                    edge: Edge::Release,
                },
            },
        ],
        samples: [
            (0, "baseline"),
            (1_500, "approach"),
            (4_200, "contact_start"),
            (6_500, "contact_end"),
            (8_200, "released"),
            (9_000, "settled_start"),
            (9_500, "settled_end"),
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
        "wall recording has an extra trial"
    );
    let expected = self::recording();
    ensure!(
        recording.ready_state == expected.ready_state
            && recording.inputs == expected.inputs
            && recording.samples == expected.samples,
        "wall recording changed from its source-backed input and observation schedule"
    );
    Ok(())
}

type Point = [f64; 3];

fn subtract(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: Point, b: Point) -> Point {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn scale(a: Point, amount: f64) -> Point {
    [a[0] * amount, a[1] * amount, a[2] * amount]
}
fn length(a: Point) -> f64 {
    dot(a, a).sqrt()
}

fn actor_facing_normal(face: [Point; 3]) -> Point {
    let n = cross(subtract(face[1], face[0]), subtract(face[2], face[0]));
    let n = scale(n, 1.0 / length(n));
    if dot(n, subtract(START, face[0])) >= 0.0 {
        n
    } else {
        scale(n, -1.0)
    }
}

fn projection_inside(point: Point, face: [Point; 3]) -> bool {
    let v0 = subtract(face[1], face[0]);
    let v1 = subtract(face[2], face[0]);
    let v2 = subtract(point, face[0]);
    let d00 = dot(v0, v0);
    let d01 = dot(v0, v1);
    let d11 = dot(v1, v1);
    let d20 = dot(v2, v0);
    let d21 = dot(v2, v1);
    let denominator = d00 * d11 - d01 * d01;
    let v = (d11 * d20 - d01 * d21) / denominator;
    let w = (d00 * d21 - d01 * d20) / denominator;
    v >= -1e-6 && w >= -1e-6 && v + w <= 1.0 + 1e-6
}

fn edge_distance(point: Point, a: Point, b: Point) -> f64 {
    let edge = subtract(b, a);
    let t = (dot(subtract(point, a), edge) / dot(edge, edge)).clamp(0.0, 1.0);
    length(subtract(
        point,
        [a[0] + edge[0] * t, a[1] + edge[1] * t, a[2] + edge[2] * t],
    ))
}

fn sphere_clearance(center: Point, face: [Point; 3]) -> f64 {
    let normal = actor_facing_normal(face);
    let signed = dot(normal, subtract(center, face[0]));
    let projection = subtract(center, scale(normal, signed));
    if projection_inside(projection, face) {
        signed - RADIUS
    } else {
        edge_distance(center, face[0], face[1])
            .min(edge_distance(center, face[1], face[2]))
            .min(edge_distance(center, face[2], face[0]))
            - RADIUS
    }
}

fn clearance(sample: &ActorSample, face: [Point; 3]) -> f64 {
    OFFSETS
        .into_iter()
        .map(|offset| {
            sphere_clearance(
                [
                    f64::from(sample.translation[0]),
                    f64::from(sample.translation[1]),
                    f64::from(sample.translation[2]) + offset,
                ],
                face,
            )
        })
        .fold(f64::INFINITY, f64::min)
}

pub fn corner_clearances(sample: &ActorSample) -> (f64, f64) {
    (clearance(sample, ANGLED), clearance(sample, CROSS))
}

fn native_contact(sample: &ActorSample) -> Option<(bool, bool)> {
    let contacts = sample.raw_observation.get("slide_contacts")?.as_array()?;
    let found = contacts
        .iter()
        .any(|contact| contact["source_node_id"] == "world/1/5/0");
    Some((found, sample.raw_observation["on_floor"] == true))
}

pub fn assertions(samples: &[ActorSample]) -> Result<Vec<Assertion>> {
    ensure!(samples.len() == 7, "wall recording needs seven samples");
    let names = [
        "baseline",
        "approach",
        "contact_start",
        "contact_end",
        "released",
        "settled_start",
        "settled_end",
    ];
    ensure!(
        samples.iter().map(|sample| sample.name.as_str()).eq(names),
        "wall sample order changed"
    );
    let cross_clearance = samples
        .iter()
        .map(|sample| clearance(sample, CROSS))
        .collect::<Vec<_>>();
    let angled_clearance = samples
        .iter()
        .map(|sample| clearance(sample, ANGLED))
        .collect::<Vec<_>>();
    let mut checks = vec![
        bound(
            "wall_baseline_idle",
            action_is(&samples[0], IDLE_ACTION),
            Some(1.0),
            Some(1.0),
            SOURCE,
        ),
        bound(
            "wall_approach_active",
            walking_action_is_active(&samples[1]),
            Some(1.0),
            Some(1.0),
            SOURCE,
        ),
        bound(
            "wall_contact_start_active",
            action_is(&samples[2], WALK_FORWARD_ACTION),
            Some(1.0),
            Some(1.0),
            SOURCE,
        ),
        bound(
            "wall_contact_end_active",
            action_is(&samples[3], WALK_FORWARD_ACTION),
            Some(1.0),
            Some(1.0),
            SOURCE,
        ),
        bound(
            "wall_settled_idle",
            action_is(&samples[6], IDLE_ACTION),
            Some(1.0),
            Some(1.0),
            SOURCE,
        ),
        bound(
            "wall_approach_distance_xy",
            distance_xy(&samples[0], &samples[1]),
            Some(1.0),
            Some(5.0),
            SOURCE,
        ),
        bound(
            "wall_cross_approach_clearance_change",
            cross_clearance[0] - cross_clearance[1],
            Some(1.0),
            None,
            SOURCE,
        ),
        bound(
            "wall_cross_contact_start_clearance",
            cross_clearance[2],
            Some(-0.08),
            Some(0.25),
            SOURCE,
        ),
        bound(
            "wall_cross_contact_end_clearance",
            cross_clearance[3],
            Some(-0.08),
            Some(0.25),
            SOURCE,
        ),
        bound(
            "wall_angled_contact_start_clearance",
            angled_clearance[2],
            Some(-0.08),
            Some(0.25),
            SOURCE,
        ),
        bound(
            "wall_angled_contact_end_clearance",
            angled_clearance[3],
            Some(-0.08),
            Some(0.25),
            SOURCE,
        ),
        bound(
            "wall_cross_sampled_min_clearance",
            cross_clearance
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min),
            Some(-0.08),
            None,
            SOURCE,
        ),
        bound(
            "wall_angled_sampled_min_clearance",
            angled_clearance
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min),
            Some(-0.08),
            None,
            SOURCE,
        ),
        bound(
            "wall_held_translation_xy",
            distance_xy(&samples[2], &samples[3]),
            None,
            Some(0.20),
            SOURCE,
        ),
        bound(
            "wall_held_height_change",
            f64::from((samples[3].translation[2] - samples[2].translation[2]).abs()),
            None,
            Some(0.20),
            SOURCE,
        ),
        bound(
            "wall_post_release_drift",
            distance_xy(&samples[5], &samples[6]),
            None,
            Some(0.20),
            SOURCE,
        ),
    ];
    if let Some((start, floor_start)) = native_contact(&samples[2]) {
        let (end, floor_end) = native_contact(&samples[3]).unwrap_or((false, false));
        checks.push(bound(
            "wall_native_named_contact",
            f64::from(u8::from(start || end)),
            Some(1.0),
            Some(1.0),
            SOURCE,
        ));
        checks.push(bound(
            "wall_native_floor_contact",
            f64::from(u8::from(floor_start && floor_end)),
            Some(1.0),
            Some(1.0),
            SOURCE,
        ));
    }
    Ok(checks)
}
