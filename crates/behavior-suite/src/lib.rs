//! Runs shared functional scenarios against the original game or Godot.
//! Engine adapters remain in their respective controller crates.
use anyhow::{Context, Result, ensure};
use behavior_contract::{BackendResult, CrossBackendComparison, assertions, compare_results};
use serde_json::json;
use std::{fs, path::Path};

pub fn evaluate_backend_result(mut result: BackendResult) -> Result<BackendResult> {
    result.recording.validate()?;
    let checks = assertions(result.scenario, &result.samples)?;
    result.assertions = checks;
    result.status = if result.assertions.iter().all(|check| check.passed) {
        "passed".into()
    } else {
        "failed".into()
    };
    Ok(result)
}

fn load_report(path: &Path, backend: &str) -> Result<BackendResult> {
    let report: serde_json::Value = serde_json::from_slice(
        &fs::read(path).with_context(|| format!("read {}", path.display()))?,
    )?;
    ensure!(report["schema"] == 1, "unsupported scenario report schema");
    ensure!(
        report["cleanup_complete"] == true,
        "scenario cleanup incomplete"
    );
    ensure!(
        report["selected_backend"] == backend,
        "scenario report backend differs"
    );
    let result: BackendResult = serde_json::from_value(report["result"].clone())?;
    ensure!(
        result.status == "passed",
        "suite did not mark backend result passed"
    );
    result.require_pass()?;
    Ok(result)
}

pub fn compare_reports(
    openspace: &Path,
    godot: &Path,
    output: &Path,
) -> Result<CrossBackendComparison> {
    let source = load_report(openspace, "openspace")?;
    let target = load_report(godot, "godot")?;
    let comparison = compare_results(&source, &target)?;
    fs::write(
        output,
        serde_json::to_vec_pretty(&json!({
            "schema": 1,
            "openspace_report": openspace,
            "godot_report": godot,
            "comparison": comparison,
        }))?,
    )?;
    Ok(comparison)
}
