#![cfg(target_os = "linux")]
//! Run only after the native consumer/publication gate is accepted. The compared
//! fields here are invariant content; stopped bytes never establish behavior.
use anyhow::{Context, Result, ensure};
use astrolabe::{
    model::{
        AssetRef, Selection,
        character::Family,
        io::{Load, Save},
    },
    openspace::level::live::{ActorObservation, source_fingerprint},
};
use orrery::{
    comparison::{Outcome, compare_main_actor, load_rete_actor, prepare_routes},
    config::{Config, digest},
};
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
#[ignore = "requires prepared original-game fixture and local disc; selected by comparison --local"]
fn original_actor_definitions_match_direct_and_reopened_rete_and_detect_edit() -> Result<()> {
    let config_path = PathBuf::from(
        std::env::var_os("ASTROLABE_HYPE_CONFIG")
            .context("original fixture configuration missing")?,
    );
    let config = Config::load(&config_path)?;
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../disc"));
    let selection = Selection::hype("manoir", "en")?;
    let source = source_fingerprint(&game, &selection)?;
    ensure!(
        source == source_fingerprint(&config.game, &selection)?,
        "converter and private installation source fixtures differ"
    );
    let evidence = std::env::var_os("ASTROLABE_EVIDENCE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../TestResults"));
    let output = evidence.join(format!(
        "actor-comparison-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    fs::create_dir_all(&output)?;
    let routes = prepare_routes(&game, "manoir", &output.join("rete"))?;
    fs::write(
        output.join("static-routes.json"),
        serde_json::to_vec_pretty(&routes)?,
    )?;
    // This is the same native runtime used by inspection, with its own bounded
    // worker and outer owner. No Python memory broker or caller-supplied address.
    let status = Command::new(env!("CARGO_BIN_EXE_orrery"))
        .arg("inspect-original")
        .arg("--config")
        .arg(&config_path)
        .arg("--output")
        .arg(output.join("original"))
        .status()?;
    ensure!(
        status.success(),
        "native original inspection failed; evidence {}",
        output.display()
    );
    let summary: Value = serde_json::from_slice(&fs::read(output.join("original/summary.json"))?)?;
    ensure!(
        summary["status"] == "passed" && summary["cleanup_complete"] == true,
        "original runtime did not complete and clean up"
    );
    let worker_path = output.join("original/worker.json");
    let worker: Value = serde_json::from_slice(&fs::read(&worker_path)?)?;
    ensure!(
        worker["game"]["phase"] == 9 && worker["game"]["level"] == "manoir",
        "wrong live fixture"
    );
    ensure!(
        worker["fixture"]["exe_sha256"] == config.exe_sha256
            && worker["fixture"]["save_sha256"] == config.save_sha256,
        "live fixture provenance differs"
    );
    let observation: ActorObservation = serde_json::from_value(worker["actor"].clone())?;
    let direct = compare_main_actor(&routes.direct, &observation);
    let reopened = compare_main_actor(&routes.reopened_rete, &observation);
    ensure!(
        source == source_fingerprint(&game, &selection)?
            && source == source_fingerprint(&config.game, &selection)?,
        "source fixture changed during comparison"
    );
    Config::load(&config_path)?;

    // A real persisted edit must disagree with the unchanged original. Keep the
    // original package intact; preserve the edited Family as a standalone asset
    // and restore the baseline file after obtaining the reopened projection.
    let (mut edit_store, _) = astrolabe::rete::Store::open(output.join("rete"))?;
    let family_ref = AssetRef::<Family>::new(&routes.reopened_rete.family)?;
    let baseline = edit_store.load(&family_ref)?;
    let mut edited = baseline.as_ref().clone();
    edited
        .states
        .iter_mut()
        .find(|s| s.id == 47)
        .context("selected action missing")?
        .speed ^= 1;
    edit_store.save(&family_ref, &edited)?;
    fs::write(
        output.join("deliberately-edited-family.rete"),
        astrolabe::rete::encode(&edited)?,
    )?;
    drop(edit_store);
    let edited_projection = load_rete_actor(&output.join("rete"))?;
    let edit_report = compare_main_actor(&edited_projection, &observation);
    let (mut restore, _) = astrolabe::rete::Store::open(output.join("rete"))?;
    restore.save(&family_ref, baseline.as_ref())?;
    drop(restore);
    let report = json!({"schema":1,"scope":"fresh live invariant definition comparison; no behavior assertion",
        "source_fingerprint":source,"worker_report_sha256":digest(&worker_path)?,"live_consistency":worker["consistency"],
        "direct":direct,"reopened_rete":reopened,"deliberate_rete_edit":edit_report,
        "baseline_rete_restored":true,"behavior_parity":false});
    fs::write(
        output.join("comparison.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("actor comparison evidence: {}\n{report}", output.display());
    ensure!(
        direct.selected_invariants_match && reopened.selected_invariants_match,
        "selected invariant comparison failed"
    );
    ensure!(
        !edit_report.selected_invariants_match,
        "deliberate Rete edit was not detected"
    );
    ensure!(
        edit_report
            .fields
            .iter()
            .any(|f| f.path == "family.actions[47].speed" && f.outcome == Outcome::Different),
        "wrong deliberate-edit mismatch"
    );
    Ok(())
}
