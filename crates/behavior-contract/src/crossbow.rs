//! Initial crossbow camera-mode shot through Hype's and the arrow's compiled AI.
use super::*;

const ARROW_MODEL: &str = "fix:/ai/models/733";
const ARROW_MODEL_OFFSET: u64 = 0x4e324;
// Saved checkpoint pose plus the source reaction box's minimum Y.
const CHECKPOINT_CONTACT_Y: f64 = 29.486_925_125_122_07 - 0.4;

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
            edge(1050, "crossbow_aim_press", Key::Keypad0, Edge::Press),
            edge(1100, "crossbow_aim_left_press", Key::ArrowLeft, Edge::Press),
            edge(
                1200,
                "crossbow_aim_left_release",
                Key::ArrowLeft,
                Edge::Release,
            ),
            edge(1250, "attack_press", Key::Space, Edge::Press),
            edge(1330, "attack_release", Key::Space, Edge::Release),
            edge(1450, "crossbow_aim_release", Key::Keypad0, Edge::Release),
        ],
        samples: [
            (0, "baseline"),
            (1000, "selected"),
            (1400, "camera_fire_active"),
            (1900, "flight_recorded"),
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
        "crossbow launch requires physical Z55, keypad-0 camera aim, and Space firing from quantity three"
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

fn contains_boolean(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Bool(_) => true,
        serde_json::Value::Array(values) => values.iter().any(contains_boolean),
        serde_json::Value::Object(values) => values.values().any(contains_boolean),
        _ => false,
    }
}

fn checked_global_identity(observation: &serde_json::Value) -> bool {
    let perso = &observation["perso_fix_block_offset"];
    let model = &observation["ai_model_fix_block_offset"];
    let fields = observation["actor"]["designer_values"].as_array();
    let field_14 = fields.and_then(|values| {
        values
            .iter()
            .find(|value| value["index"] == 14 && value["kind"] == 0)
    });
    observation["allocation_header_words"] == 20
        && perso["module"] == 5
        && perso["block"] == 0
        && perso["payload_offset"] == 0x314
        && model["module"] == 17
        && model["block"] == 0
        && model["payload_offset"] == 0x1c
        && field_14.is_some_and(|value| contains_boolean(&value["value"]))
}

fn original_arrow(sample: &serde_json::Value) -> Option<(u64, String, u64, [f64; 3])> {
    let elapsed = sample["elapsed_resumed_ms"].as_u64()?;
    let children = sample["scene"]["children"]
        .as_array()
        .or_else(|| sample["scene_tail_latest"]["children"].as_array())?;
    children.iter().find_map(|child| {
        let offset = &child["ai"]["fix_block_offset"];
        (offset["module"] == 17
            && offset["block"] == 0
            && offset["payload_offset"].as_u64() == Some(ARROW_MODEL_OFFSET))
        .then(|| {
            let position = position(&child["global_transform"]["Affine"]["translation"])?;
            // Newly allocated source actors can briefly expose an unpublished
            // zero transform. It is not a valid trajectory sample.
            if position
                .iter()
                .all(|component| component.abs() <= f64::EPSILON)
            {
                return None;
            }
            Some((
                child["index"].as_u64()?,
                child["node_locator"].as_str()?.to_owned(),
                elapsed,
                position,
            ))
        })
        .flatten()
    })
}

fn original_arrow_evidence(burst: &[serde_json::Value]) -> (bool, Option<f64>) {
    let candidates = burst.iter().filter_map(original_arrow).collect::<Vec<_>>();
    let moved = candidates.windows(2).find_map(|pair| {
        let (_, node_a, elapsed_a, position_a) = &pair[0];
        let (_, node_b, elapsed_b, position_b) = &pair[1];
        // The source frame stamp can still be zero on an inserted actor, and
        // is not a completed-update oracle. Captures must advance in time and
        // the same checked arrow must move.
        (node_a == node_b && elapsed_b > elapsed_a)
            .then(|| displacement(*position_a, *position_b))
            .filter(|distance| *distance > 0.05)
    });
    (!candidates.is_empty(), moved)
}

pub fn assertions(samples: &[ActorSample]) -> Result<Vec<Assertion>> {
    ensure!(
        samples.len() == 4,
        "crossbow launch needs four observations"
    );
    ensure!(
        samples.iter().map(|sample| sample.name.as_str()).eq([
            "baseline",
            "selected",
            "camera_fire_active",
            "flight_recorded"
        ]),
        "crossbow observations changed order"
    );
    let [baseline, selected, camera_fire_active, flight] = samples else {
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
    let source = "One active source inventory handle is set to quantity three through the same handle field that Hype's InventoryQuantity and InventoryRemove functions use; Z55, keypad-0 camera aim, and Space firing are physical input";
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
        "Observed armed idle after physical Z selection and before physical Space/Button_Hit",
    ));
    let aim_schedule_active = if godot {
        camera_fire_active.raw_observation["normal_behavior"] == 40
    } else {
        camera_fire_active.raw_observation["source_ai"]["normal"]["current"] == 40
    };
    checks.push(bound(
        "crossbow_camera_aim_schedule_active_during_fire",
        f64::from(u8::from(aim_schedule_active)),
        Some(1.0),
        Some(1.0),
        "The player's compiled normal-40 Crossbow camera-aim/firing schedule remains active while keypad-0 is held",
    ));
    checks.push(bound(
        "crossbow_horizontal_aim_turns_player",
        heading_delta(selected, camera_fire_active),
        Some(0.005),
        None,
        "Physical Left while keypad-0 is held routes to BT_Camera_LookLeft and changes Hype's facing",
    ));
    let terminal_quantity = if godot {
        raw["inventory"]["weapon_80_quantity"].as_i64()
    } else {
        raw["inventory"]["active"]
            .as_array()
            .and_then(|members| {
                members
                    .iter()
                    .find(|member| member["kind"] == "Weapon" && member["id"] == 80)
            })
            .and_then(|member| member["quantity"].as_i64())
    };
    checks.push(bound(
        "crossbow_camera_fire_consumes_one_bolt",
        f64::from(u8::from(terminal_quantity == Some(2))),
        Some(1.0),
        Some(1.0),
        "The source and compiled BT_Camera_TireArbalete schedules each remove one bolt from the test-owned quantity-three inventory",
    ));
    if godot {
        checks.push(bound(
            "crossbow_camera_aim_brain_keeps_running",
            f64::from(u8::from(camera_fire_active.raw_observation["brain_running"] == true
                && camera_fire_active.raw_observation["brain_failure"] == "")),
            Some(1.0),
            Some(1.0),
            "The compiled camera-aim and firing behavior remains live without an unsupported source function",
        ));
    }
    if godot {
        let trace = raw["generated_trace"]
            .as_array()
            .context("Godot launch has no generated actor trace")?;
        let frames = trace
            .iter()
            .filter(|entry| entry["event"] == "frame" && entry["source_model_path"] == ARROW_MODEL)
            .collect::<Vec<_>>();
        let release = trace
            .iter()
            .find(|entry| entry["event"] == "release" && entry["source_model_path"] == ARROW_MODEL);
        let launch = frames
            .iter()
            .copied()
            .find(|entry| entry["brain_running"] == true);
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
            "crossbow_arrow_released_from_animation_channel",
            f64::from(u8::from(release.is_some_and(|entry| {
                entry["translation"] == entry["attachment_translation"]
            }))),
            Some(1.0),
            Some(1.0),
            "Shared native attachment cue copies the triggering animation channel transform before model-733 Brain activation",
        ));
        checks.push(bound(
            "crossbow_compiled_arrow_starts",
            f64::from(u8::from(launch.is_some_and(|entry| {
                entry["brain_running"] == true && entry["brain_failure"] == ""
            }))),
            Some(1.0),
            Some(1.0),
            "Compiled Hype model 28 creates model-733 arrow; arrow Brain runs at launch",
        ));
        let continued_flight = launch.and_then(|first| {
            let id = &first["instance_id"];
            let start = position(&first["translation"])?;
            frames.iter().find(|entry| {
                entry["instance_id"] == *id
                    && entry["brain_running"] == true
                    && entry["brain_failure"] == ""
                    && position(&entry["translation"])
                        .is_some_and(|current| displacement(start, current) > 0.05)
            })
        });
        checks.push(bound(
            "crossbow_compiled_arrow_ai_continues_in_flight",
            f64::from(u8::from(continued_flight.is_some())),
            Some(1.0),
            Some(1.0),
            "Model-733 Brain remains active after native no-contact evaluation while its arrow moves",
        ));
        checks.push(bound(
            "crossbow_initial_displacement",
            moving_pair.unwrap_or(-1.0),
            Some(0.05),
            None,
            "Initial native arrow movement while the compiled model-733 Brain remains active",
        ));
        let arrow_exit = trace.iter().find(|entry| {
            entry["event"] == "tree_exiting" && entry["source_model_path"] == ARROW_MODEL
        });
        let contact_cleanup = arrow_exit.is_some_and(|entry| {
            entry["slide_collision_count"]
                .as_u64()
                .is_some_and(|count| count > 0)
                && entry["brain_normal"] == 1
                && entry["brain_failure"] == ""
                && entry["main_actor_field_120"] == 0
        });
        checks.push(bound(
            "crossbow_wall_contact_runs_compiled_cleanup",
            f64::from(u8::from(contact_cleanup)),
            Some(1.0),
            Some(1.0),
            "The launched model-733 arrow reaches native slide contact and its compiled normal-1 behavior removes it without a Brain failure",
        ));
        let checkpoint_contact = arrow_exit.is_some_and(|entry| {
            entry["slide_contacts"].as_array().is_some_and(|contacts| {
                contacts.iter().any(|contact| {
                    contact["source_perso"] == "level:/persos/world/0/35"
                        && position(&contact["position"])
                            .is_some_and(|point| (-point[2] - CHECKPOINT_CONTACT_Y).abs() < 0.03)
                        && position(&contact["normal"])
                            .is_some_and(|normal| displacement(normal, [0.0, 0.0, 1.0]) < 0.03)
                })
            })
        });
        checks.push(bound(
            "crossbow_contacts_saved_checkpoint_reaction_box",
            f64::from(u8::from(checkpoint_contact)),
            Some(1.0), Some(1.0),
            "Native contact identifies the checkpoint asset and the independently decoded reaction box plane/normal; a later static-world hit does not qualify",
        ));
        let lifetime = release.zip(arrow_exit).and_then(|(start, end)| {
            Some(end["brain_clock_ms"].as_f64()? - start["brain_clock_ms"].as_f64()?)
        });
        checks.push(bound(
            "crossbow_contact_cleanup_lifetime_ms",
            lifetime.unwrap_or(-1.0), Some(1.0), Some(150.0),
            "Physics-clock animation release to tree exit at the nearby checkpoint; wall-clock timestamps retain render stalls and source disappearance is separately bracketed by stopped captures",
        ));
        let aligned_launch = frames
            .iter()
            .find(|entry| {
                entry["slide_collision_count"].as_u64().unwrap_or(0) == 0
                    && position(&entry["velocity"]).is_some_and(|v| displacement(v, [0.0; 3]) > 1.0)
            })
            .is_some_and(|entry| {
                position(&entry["velocity"])
                    .zip(position(&entry["sighting"]))
                    .is_some_and(|(velocity, sighting)| {
                        displacement(velocity, sighting.map(|v| v * 45.0)) < 0.01
                    })
            });
        checks.push(bound(
            "crossbow_launch_velocity_uses_released_arrow_sighting",
            f64::from(u8::from(aligned_launch)), Some(1.0), Some(1.0),
            "The decoded local forward speed 45 follows the released arrow basis; the original free-flight velocity is sighting times 45",
        ));
        let firing_guard_seen = frames.iter().any(|entry| {
            entry["source_model_path"] == ARROW_MODEL && entry["main_actor_field_120"] == 1
        });
        checks.push(bound(
            "crossbow_firing_guard_released_after_cleanup",
            f64::from(u8::from(firing_guard_seen && arrow_exit.is_some_and(|entry| entry["main_actor_field_120"] == 0))),
            Some(1.0),
            Some(1.0),
            "Owner AI firing guard is set while the arrow is active and reset when its compiled cleanup removes the arrow",
        ));
    } else {
        let global_identity = samples
            .iter()
            .all(|sample| checked_global_identity(&sample.raw_observation["crossbow_global"]));
        checks.push(bound(
            "crossbow_global_live_identity_and_field14",
            f64::from(u8::from(global_identity)),
            Some(1.0),
            Some(1.0),
            "Same-capture manor path, Fix Perso allocation, Global AI model, and typed live field 14",
        ));
        let main_ai = raw["source_ai"]["designer_values"].as_array();
        let owner_ai_fields = [7, 8, 13, 15, 59, 60, 95, 130];
        let owner_ai_observed = main_ai.is_some_and(|values| {
            owner_ai_fields.iter().all(|index| {
                values
                    .iter()
                    .any(|value| value["index"] == *index && !value["value"].is_null())
            })
        });
        checks.push(bound(
            "crossbow_owner_ai_fields_observed",
            f64::from(u8::from(owner_ai_observed)),
            Some(1.0),
            Some(1.0),
            "Typed source reads of crossbow branch selectors and remembered quantity",
        ));
        let burst = raw["source_projectile_burst"]
            .as_array()
            .context("original launch has no bounded scene-tail burst")?;
        let (spawned, moved) = original_arrow_evidence(burst);
        checks.push(bound(
            "crossbow_original_model733_spawned",
            f64::from(u8::from(spawned)),
            Some(1.0),
            Some(1.0),
            "Checked live Fix module 17/block 0 model pointer at payload offset 0x4e324",
        ));
        checks.push(bound(
            "crossbow_initial_displacement",
            moved.unwrap_or(-1.0),
            Some(0.05),
            None,
            "Two time-ordered positions of the same checked original arrow before it disappears",
        ));
        let contact_observed = burst.iter().any(|entry| {
            entry["scene_tail_latest"]["children"]
                .as_array()
                .is_some_and(|children| {
                    children.iter().any(|child| {
                        let offset = &child["ai"]["fix_block_offset"];
                        offset["module"] == 17
                            && offset["block"] == 0
                            && offset["payload_offset"].as_u64() == Some(ARROW_MODEL_OFFSET)
                            && child["dynamics"]["contact_classes"]["publication_consistent"]
                                == true
                            && child["dynamics"]["contact_classes"]["collide_with_nothing"] == false
                            && child["dynamics"]["contact_classes"]["current"]
                                .as_u64()
                                .is_some_and(|value| value != 0)
                    })
                })
        });
        checks.push(bound(
            "crossbow_source_collide_with_nothing_false_on_classified_contact",
            f64::from(u8::from(contact_observed)),
            Some(1.0),
            Some(1.0),
            "Checked model-733 instance had a published nonzero contact word; pinned handler operation 33 returns true only when that word is zero",
        ));
        let checkpoint_contact = burst.iter().any(|entry| {
            entry["scene_tail_latest"]["children"]
                .as_array()
                .is_some_and(|children| {
                    children.iter().any(|child| {
                        let offset = &child["ai"]["fix_block_offset"];
                        let contact = &child["dynamics"]["wall_contact"];
                        offset["module"] == 17
                            && offset["block"] == 0
                            && offset["payload_offset"].as_u64() == Some(ARROW_MODEL_OFFSET)
                            && child["dynamics"]["contact_classes"]["current"] == 4
                            && contact["kind"] == 2
                            && contact["object_types"] == serde_json::json!([29, 42, 108])
                            && position(&contact["point"])
                                .is_some_and(|point| (point[1] - CHECKPOINT_CONTACT_Y).abs() < 0.03)
                            && position(&contact["normal"])
                                .is_some_and(|normal| displacement(normal, [0.0, -1.0, 0.0]) < 0.03)
                    })
                })
        });
        checks.push(bound(
            "crossbow_contacts_saved_checkpoint_reaction_box",
            f64::from(u8::from(checkpoint_contact)), Some(1.0), Some(1.0),
            "Typed source wall result identifies checkpoint object types 29/42/108 and its decoded reaction box plane/normal; runtime sibling ordinals are not asset identities",
        ));
        let post_burst = &raw["crossbow_post_burst_diagnostic"];
        let post_burst_identity = post_burst["scene_ai_model"]["model"]["module"] == 17
            && post_burst["scene_ai_model"]["model"]["block"] == 0
            && post_burst["scene_ai_model"]["model"]["payload_offset"] == ARROW_MODEL_OFFSET;
        let arrow_absent_later = post_burst_identity
            && post_burst["scene_ai_model"]["instances"]
                .as_array()
                .is_some_and(|instances| instances.is_empty());
        checks.push(bound(
            "crossbow_source_arrow_absent_at_post_burst_checkpoint",
            f64::from(u8::from(arrow_absent_later)),
            Some(1.0),
            Some(1.0),
            "Later full-scene search tied to active Fix model-733 allocation found no live instance; removal cause is not asserted",
        ));
    }
    Ok(checks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_arrow_can_move_before_its_source_frame_stamp_is_published() {
        let observation = |elapsed, stamp, translation| {
            serde_json::json!({
                "elapsed_resumed_ms": elapsed,
                "scene": {"children": [{
                    "index": 176,
                    "node_locator": "live:03e0fe34",
                    "update_stamp": stamp,
                    "global_transform": {"Affine": {"translation": translation}},
                    "ai": {"fix_block_offset": {
                        "module": 17,
                        "block": 0,
                        "payload_offset": ARROW_MODEL_OFFSET
                    }}
                }]}
            })
        };
        let observations = [
            observation(1_234, 0, [-73.905_868, 28.456_821, 6.097_703]),
            observation(1_244, 313, [-73.737_892, 29.416_655, 6.035_868]),
        ];
        let (spawned, movement) = original_arrow_evidence(&observations);

        assert!(spawned);
        assert!(movement.is_some_and(|distance| distance > 0.05));
    }

    #[test]
    fn original_arrow_zero_allocation_transform_is_not_motion_evidence() {
        let observation = |elapsed, translation| {
            serde_json::json!({
                "elapsed_resumed_ms": elapsed,
                "scene_tail_latest": {"children": [{
                    "index": 176,
                    "node_locator": "live:03e0fe34",
                    "global_transform": {"Affine": {"translation": translation}},
                    "ai": {"fix_block_offset": {
                        "module": 17,
                        "block": 0,
                        "payload_offset": ARROW_MODEL_OFFSET
                    }}
                }]}
            })
        };
        let observations = [
            observation(1_230, [0.0, 0.0, 0.0]),
            observation(1_234, [-73.905_868, 28.456_821, 6.097_703]),
            observation(1_244, [-73.737_892, 29.416_655, 6.035_868]),
        ];

        let (spawned, movement) = original_arrow_evidence(&observations);

        assert!(spawned);
        assert!(movement.is_some_and(|distance| (0.97..1.01).contains(&distance)));
    }
}
