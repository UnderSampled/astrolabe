#[path = "support/behavior_steps.rs"]
mod behavior_steps;
const BACKEND: behavior_steps::Mode = behavior_steps::Mode::Synthetic;

#[test]
fn synthetic_turn_and_walk_use_shared_bdd_steps_and_report_pending_godot() -> anyhow::Result<()> {
    let shared =
        behavior_steps::run_scenarios_shared(&["Turning while standing", "Walking and stopping"])?;
    let suite: serde_json::Value = serde_json::from_slice(&std::fs::read(&shared.suite_report)?)?;
    assert_eq!(suite["status"], "passed");
    assert_eq!(suite["launch_count"], 0);
    assert_eq!(suite["completed_reports"].as_array().map(Vec::len), Some(2));
    let shared_turn: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&shared.scenario_reports[0])?)?;
    let shared_walk: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&shared.scenario_reports[1])?)?;
    assert_eq!(shared_turn["session"]["id"], shared_walk["session"]["id"]);
    for scenario in ["Turning while standing", "Walking and stopping"] {
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
