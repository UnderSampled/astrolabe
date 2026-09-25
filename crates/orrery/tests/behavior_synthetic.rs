#[path = "support/behavior_steps.rs"]
mod behavior_steps;
const BACKEND: behavior_steps::Mode = behavior_steps::Mode::Synthetic;

#[test]
fn synthetic_stationary_jump_uses_shared_bdd_and_checks_height() -> anyhow::Result<()> {
    let path = behavior_steps::run_scenario("Jumping from a standstill and landing")?;
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    assert_eq!(report["result"]["status"], "passed");
    assert_eq!(report["godot"]["status"], "pending");
    use orrery::behavior::{self, Scenario};
    let mut source =
        behavior_steps::synthetic_result(behavior::recording(Scenario::StationaryJump))?;
    source.backend = "openspace".into();
    source.cleanup_complete = true;
    let mut godot = source.clone();
    godot.backend = "godot".into();
    assert_eq!(behavior::compare_results(&source, &godot)?.status, "passed");
    godot.samples[2].translation[2] += 2.0;
    godot.assertions = behavior::assertions(godot.scenario, &godot.samples)?;
    assert_eq!(behavior::compare_results(&source, &godot)?.status, "failed");
    Ok(())
}

#[test]
fn synthetic_moving_jump_requires_continuous_walk_and_forward_air_motion() -> anyhow::Result<()> {
    let path = behavior_steps::run_scenario("Jumping from a walk and landing")?;
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    assert_eq!(report["result"]["status"], "passed");
    use orrery::behavior::{self, Scenario};
    let mut source = behavior_steps::synthetic_result(behavior::recording(Scenario::MovingJump))?;
    source.backend = "openspace".into();
    source.cleanup_complete = true;
    let mut godot = source.clone();
    godot.backend = "godot".into();
    assert_eq!(behavior::compare_results(&source, &godot)?.status, "passed");
    source.samples[1].action = Some(78);
    source.assertions = behavior::assertions(source.scenario, &source.samples)?;
    let comparison = behavior::compare_results(&source, &godot)?;
    assert_eq!(comparison.status, "passed");
    assert!(
        comparison
            .checks
            .iter()
            .any(|check| check.name == "moving_jump_walking_phase_compatible" && check.passed)
    );
    godot.samples[2].translation[2] = source.samples[0].translation[2];
    godot.assertions = behavior::assertions(godot.scenario, &godot.samples)?;
    assert_eq!(godot.status, "passed");
    assert!(behavior::compare_results(&source, &godot).is_err());
    Ok(())
}

#[test]
fn synthetic_wall_jump_checks_source_faces_and_landing() -> anyhow::Result<()> {
    let path = behavior_steps::run_scenario("A moving jump meets the wall and lands")?;
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    assert_eq!(report["result"]["status"], "passed");
    Ok(())
}

#[test]
fn synthetic_sword_attack_requires_selection_attack_and_recovery() -> anyhow::Result<()> {
    let path = behavior_steps::run_scenario("Selecting the sword and attacking once")?;
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    assert_eq!(report["result"]["status"], "passed");
    assert_eq!(report["openspace"]["status"], "not_run");
    assert_eq!(report["godot"]["status"], "pending");
    use orrery::behavior::{self, Scenario};
    let mut source = behavior_steps::synthetic_result(behavior::recording(Scenario::SwordAttack))?;
    source.backend = "openspace".into();
    let mut godot = source.clone();
    godot.backend = "godot".into();
    godot.samples[3].action = Some(48);
    godot.assertions = behavior::assertions(godot.scenario, &godot.samples)?;
    assert_eq!(behavior::compare_results(&source, &godot)?.status, "passed");
    godot.samples[2].action = Some(47);
    godot.assertions = behavior::assertions(godot.scenario, &godot.samples)?;
    assert_eq!(godot.status, "passed");
    assert!(behavior::compare_results(&source, &godot).is_err());
    Ok(())
}

#[test]
fn synthetic_crossbow_initial_launch_requires_two_positions() -> anyhow::Result<()> {
    let path = behavior_steps::run_scenario("Launching one crossbow arrow initially")?;
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    assert_eq!(report["result"]["status"], "passed");
    use orrery::behavior::{self, Scenario};
    let mut result =
        behavior_steps::synthetic_result(behavior::recording(Scenario::CrossbowInitialLaunch))?;
    result.samples[2].raw_observation["generated_trace_first"][1]["translation"] =
        serde_json::json!([0.0, 0.0, 5.0]);
    result.assertions = behavior::assertions(result.scenario, &result.samples)?;
    assert!(
        result
            .assertions
            .iter()
            .any(|check| { check.name == "crossbow_initial_displacement" && !check.passed })
    );
    Ok(())
}

#[test]
fn synthetic_fire_cast_requires_action_order_and_mana_cost() -> anyhow::Result<()> {
    let path = behavior_steps::run_scenario("Casting the selected Fire spell once")?;
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    assert_eq!(report["result"]["status"], "passed");
    assert_eq!(report["godot"]["status"], "pending");
    use orrery::behavior::{self, Scenario};
    let mut source = behavior_steps::synthetic_result(behavior::recording(Scenario::FireCast))?;
    source.backend = "openspace".into();
    source.samples[1].action = Some(45);
    source.samples[5].action = Some(48);
    source.assertions = behavior::assertions(source.scenario, &source.samples)?;
    let mut target = source.clone();
    target.backend = "godot".into();
    target.samples[1].action = Some(48);
    target.samples[5].action = Some(271);
    target.assertions = behavior::assertions(target.scenario, &target.samples)?;
    assert_eq!(
        behavior::compare_results(&source, &target)?.status,
        "passed"
    );
    source.samples[4].raw_observation["magick"]["current_points"] = serde_json::json!(30);
    source.assertions = behavior::assertions(source.scenario, &source.samples)?;
    assert!(
        source
            .assertions
            .iter()
            .any(|check| check.name == "fire_mana_spent" && !check.passed)
    );
    Ok(())
}

#[test]
fn synthetic_movement_uses_shared_bdd_steps_and_report_pending_godot() -> anyhow::Result<()> {
    let shared = behavior_steps::run_scenarios_shared(&[
        "Turning while standing",
        "Walking and stopping",
        "Running instead of walking",
        "A wall blocks movement",
    ])?;
    let suite: serde_json::Value = serde_json::from_slice(&std::fs::read(&shared.suite_report)?)?;
    assert_eq!(suite["status"], "passed");
    assert_eq!(suite["launch_count"], 0);
    assert_eq!(suite["completed_reports"].as_array().map(Vec::len), Some(4));
    let shared_turn: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&shared.scenario_reports[0])?)?;
    let shared_walk: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&shared.scenario_reports[1])?)?;
    assert_eq!(shared_turn["session"]["id"], shared_walk["session"]["id"]);
    for scenario in [
        "Turning while standing",
        "Walking and stopping",
        "Running instead of walking",
        "A wall blocks movement",
    ] {
        let path = behavior_steps::run_scenario(scenario)?;
        let report: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        assert_eq!(report["selected_backend"], "synthetic");
        assert_eq!(report["openspace"]["status"], "not_run");
        println!("synthetic BDD evidence: {}", path.display());
    }
    use orrery::behavior::{self, Scenario};
    let mut source =
        behavior_steps::synthetic_result(behavior::recording(Scenario::WalkReleaseStop))?;
    source.backend = "openspace".into();
    source.cleanup_complete = true;
    let mut godot = source.clone();
    godot.backend = "godot".into();
    assert_eq!(behavior::compare_results(&source, &godot)?.status, "passed");
    source.samples[1].action = Some(78);
    source.assertions = behavior::assertions(source.scenario, &source.samples)?;
    godot.samples[1].action = Some(32);
    godot.assertions = behavior::assertions(godot.scenario, &godot.samples)?;
    let walk_phase_comparison = behavior::compare_results(&source, &godot)?;
    assert_eq!(walk_phase_comparison.status, "passed");
    assert!(
        walk_phase_comparison
            .checks
            .iter()
            .any(|check| check.name == "held_walk_phase_compatible" && check.passed)
    );
    let mut run_source =
        behavior_steps::synthetic_result(behavior::recording(Scenario::RunVsWalk))?;
    run_source.backend = "openspace".into();
    run_source.cleanup_complete = true;
    let mut run_godot = run_source.clone();
    run_godot.backend = "godot".into();
    assert_eq!(
        behavior::compare_results(&run_source, &run_godot)?.status,
        "passed"
    );
    run_source.samples[1].action = Some(78);
    run_source.assertions = behavior::assertions(run_source.scenario, &run_source.samples)?;
    run_godot.samples[1].action = Some(32);
    run_godot.assertions = behavior::assertions(run_godot.scenario, &run_godot.samples)?;
    let run_phase_comparison = behavior::compare_results(&run_source, &run_godot)?;
    assert_eq!(run_phase_comparison.status, "passed");
    assert!(
        run_phase_comparison
            .checks
            .iter()
            .any(|check| check.name == "walk_held_phase_compatible" && check.passed)
    );
    let mut wall_source =
        behavior_steps::synthetic_result(behavior::recording(Scenario::WallContact))?;
    wall_source.backend = "openspace".into();
    wall_source.cleanup_complete = true;
    let mut wall_godot = wall_source.clone();
    wall_godot.backend = "godot".into();
    assert_eq!(
        behavior::compare_results(&wall_source, &wall_godot)?.status,
        "passed"
    );
    let mut turn_source =
        behavior_steps::synthetic_result(behavior::recording(Scenario::TurnReleaseIdle))?;
    turn_source.backend = "openspace".into();
    turn_source.cleanup_complete = true;
    let mut different_turn = turn_source.clone();
    different_turn.backend = "godot".into();
    for sample in &mut different_turn.samples[2..] {
        sample.heading_rad += 0.6;
    }
    different_turn.assertions =
        behavior::assertions(different_turn.scenario, &different_turn.samples)?;
    assert_eq!(
        behavior::compare_results(&turn_source, &different_turn)?.status,
        "failed"
    );
    godot = source.clone();
    godot.backend = "godot".into();
    godot.recording_sha256 = "wrong recording".into();
    assert!(behavior::compare_results(&source, &godot).is_err());
    godot = source.clone();
    godot.backend = "godot".into();
    for sample in &mut godot.samples {
        sample.translation = [0., 0., 0.];
    }
    assert!(
        behavior::assertions(godot.scenario, &godot.samples)?
            .iter()
            .any(|c| !c.passed)
    );
    godot.assertions = behavior::assertions(godot.scenario, &godot.samples)?;
    godot.status = "failed".into();
    assert!(behavior::compare_results(&source, &godot).is_err());
    let mut malformed = behavior::recording(Scenario::WalkReleaseStop);
    malformed.inputs.pop();
    assert!(malformed.validate().is_err());
    let mut late = source.clone();
    late.samples[4].capture_start_ms += 500;
    late.samples[4].capture_end_ms += 500;
    assert!(late.require_pass().is_err());
    let mut short_window = source.clone();
    short_window.samples[3].capture_end_ms = short_window.samples[3].planned_ms + 200;
    assert!(short_window.require_pass().is_err());
    Ok(())
}
