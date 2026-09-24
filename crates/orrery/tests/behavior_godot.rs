#[path = "support/behavior_steps.rs"]
mod behavior_steps;
const BACKEND: behavior_steps::Mode = behavior_steps::Mode::Godot;

#[test]
#[ignore = "requires local Hype manor data and Godot; selected by --local"]
fn godot_movement_scenarios_share_saved_scene_and_brain() -> anyhow::Result<()> {
    let evidence =
        behavior_steps::run_scenarios_shared(&["Turning while standing", "Walking and stopping"])?;
    anyhow::ensure!(
        evidence.scenario_reports.len() == 2,
        "two Godot scenarios required"
    );
    let turn: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&evidence.scenario_reports[0])?)?;
    let walk: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&evidence.scenario_reports[1])?)?;
    anyhow::ensure!(
        turn["session"]["id"] == walk["session"]["id"],
        "game session changed"
    );
    anyhow::ensure!(walk["session"]["launch_count"] == 1, "Godot restarted");
    anyhow::ensure!(
        walk["result"]["setup_interventions"]
            .as_array()
            .is_some_and(|values| values
                .iter()
                .any(|entry| entry["method"] == "compatible_previous_scenario")),
        "walk did not verify the preceding turn handoff"
    );
    for path in &evidence.scenario_reports {
        println!("Godot BDD evidence: {}", path.display());
    }
    println!("Godot BDD session: {}", evidence.suite_report.display());
    Ok(())
}
