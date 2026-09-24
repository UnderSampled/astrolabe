#![cfg(target_os = "linux")]
#[path = "support/behavior_steps.rs"]
mod behavior_steps;
const BACKEND: behavior_steps::Mode = behavior_steps::Mode::Original;

#[test]
#[ignore = "requires prepared Hype save and private Wine/Gamescope; selected by --local"]
fn original_stationary_jump_and_landing() -> anyhow::Result<()> {
    let path = behavior_steps::run_scenario("Jumping from a standstill and landing")?;
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    anyhow::ensure!(
        report["openspace"]["status"] == "passed",
        "source jump did not pass"
    );
    println!("OpenSpace jump BDD evidence: {}", path.display());
    Ok(())
}

#[test]
#[ignore = "requires prepared Hype save and private Wine/Gamescope; selected by --local"]
fn original_moving_jump_and_landing() -> anyhow::Result<()> {
    let path = behavior_steps::run_scenario("Jumping from a walk and landing")?;
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    anyhow::ensure!(
        report["openspace"]["status"] == "passed",
        "source moving jump did not pass"
    );
    println!("OpenSpace moving jump BDD evidence: {}", path.display());
    Ok(())
}

#[test]
#[ignore = "requires prepared Hype save and private Wine/Gamescope; selected by --local"]
fn original_wall_jump_and_landing() -> anyhow::Result<()> {
    let path = behavior_steps::run_scenario("A moving jump meets the wall and lands")?;
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    anyhow::ensure!(
        report["openspace"]["status"] == "passed",
        "source wall jump did not pass"
    );
    println!("OpenSpace wall jump BDD evidence: {}", path.display());
    Ok(())
}

#[test]
#[ignore = "requires prepared Hype save and private Wine/Gamescope; selected by --local"]
fn original_movement_scenarios_share_one_game_session() -> anyhow::Result<()> {
    let evidence = behavior_steps::run_scenarios_shared(&[
        "Turning while standing",
        "Walking and stopping",
        "Running instead of walking",
        "A wall blocks movement",
    ])?;
    anyhow::ensure!(
        evidence.scenario_reports.len() == 4,
        "four scenario results required"
    );
    let turn: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&evidence.scenario_reports[0])?)?;
    let walk: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&evidence.scenario_reports[1])?)?;
    let run: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&evidence.scenario_reports[2])?)?;
    let wall: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&evidence.scenario_reports[3])?)?;
    anyhow::ensure!(
        turn["session"]["id"] == walk["session"]["id"],
        "game session changed"
    );
    anyhow::ensure!(walk["session"]["launch_count"] == 1, "game restarted");
    anyhow::ensure!(
        run["session"]["id"] == turn["session"]["id"],
        "run game session changed"
    );
    anyhow::ensure!(
        run["session"]["launch_count"] == 1,
        "run restarted the game"
    );
    anyhow::ensure!(
        wall["session"]["id"] == turn["session"]["id"],
        "wall game session changed"
    );
    anyhow::ensure!(
        wall["session"]["launch_count"] == 1,
        "wall restarted the game"
    );
    anyhow::ensure!(
        walk["result"]["setup_interventions"][0]["method"] == "compatible_previous_scenario",
        "walk did not verify the preceding turn handoff"
    );
    for path in &evidence.scenario_reports {
        println!("OpenSpace BDD evidence: {}", path.display());
    }
    println!("OpenSpace BDD session: {}", evidence.suite_report.display());
    Ok(())
}
