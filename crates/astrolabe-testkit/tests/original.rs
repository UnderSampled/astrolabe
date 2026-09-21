#![cfg(target_os = "linux")]
use anyhow::{Context, Result};
use astrolabe_testkit::config::Config;
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
fn run(fault: bool) -> Result<(PathBuf, Value)> {
    let config = PathBuf::from(
        std::env::var_os("ASTROLABE_HYPE_CONFIG").context("ASTROLABE_HYPE_CONFIG is required")?,
    );
    Config::load(&config)?;
    let focus = PathBuf::from(std::env::var_os("HOME").context("HOME missing")?)
        .join(".config/kwinrulesrc");
    let before = fs::read(&focus)?;
    let evidence = std::env::var_os("ASTROLABE_EVIDENCE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../TestResults"));
    fs::create_dir_all(&evidence)?;
    let output = evidence.join(format!(
        "original-{}-{}",
        if fault { "worker-death" } else { "inspect" },
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    let mut command = Command::new(env!("CARGO_BIN_EXE_astrolabe-testkit"));
    command
        .arg("inspect-original")
        .arg("--config")
        .arg(&config)
        .arg("--output")
        .arg(&output);
    if fault {
        command.arg("--fail-stopped-worker");
    }
    let status = command.status()?;
    let summary: Value = serde_json::from_slice(&fs::read(output.join("summary.json"))?)?;
    assert_eq!(summary["cleanup_complete"], true, "{summary}");
    assert_eq!(
        fs::read(&focus)?,
        before,
        "pre-existing desktop focus rule was not restored"
    );
    Config::load(&config)?; // Installed supplied save must remain unchanged.
    assert_eq!(status.success(), !fault, "{summary}");
    println!("native original evidence: {}\n{summary}", output.display());
    Ok((output, summary))
}
#[test]
#[ignore = "requires prepared Hype save, private Wine/Gamescope and KDE; selected by --local"]
fn native_manor_inspection_reads_actor_captures_pixels_and_cleans_up() -> Result<()> {
    let (output, summary) = run(false)?;
    assert_eq!(summary["status"], "passed");
    let report: Value = serde_json::from_slice(&fs::read(output.join("worker.json"))?)?;
    assert_eq!(report["game"]["phase"], 9);
    assert_eq!(report["game"]["level"], "manoir");
    assert_eq!(
        report["actor"]["object_types"],
        serde_json::json!([1, 1, 1])
    );
    assert_eq!(
        report["actor"]["action_members"].as_array().unwrap().len(),
        389
    );
    assert_eq!(report["actor"]["current_action"], 47);
    assert_eq!(report["consistency"], "stopped_bytes");
    assert_eq!(report["completed_update"], false);
    assert_eq!(report["frame_replay"], false);
    assert_eq!(report["behavior_parity"], false);
    let pid = report["process"]["pid"].as_u64().unwrap();
    assert!(!PathBuf::from(format!("/proc/{pid}")).exists());
    let mut reader = png::Decoder::new(std::io::BufReader::new(fs::File::open(
        output.join("original.png"),
    )?))
    .read_info()?;
    assert_eq!((reader.info().width, reader.info().height), (1024, 768));
    let mut pixels = vec![0; reader.output_buffer_size().context("pixel size overflow")?];
    let frame = reader.next_frame(&mut pixels)?;
    assert!(
        pixels[..frame.buffer_size()]
            .as_chunks::<3>()
            .0
            .iter()
            .any(|p| p != &pixels[..3]),
        "blank capture cannot pass"
    );
    let snapshot: Value = serde_json::from_slice(&fs::read(output.join("snapshot.json"))?)?;
    println!(
        "capture: {} microseconds; {} page reads; {} copied source bytes",
        snapshot["capture_micros"], snapshot["page_reads"], snapshot["copied_source_bytes"]
    );
    Ok(())
}
#[test]
#[ignore = "requires prepared Hype save, private Wine/Gamescope and KDE; selected by --local"]
fn native_owner_cleans_game_after_worker_dies_while_paused() -> Result<()> {
    let (output, summary) = run(true)?;
    assert_eq!(summary["status"], "failed");
    let fault: Value = serde_json::from_slice(&fs::read(output.join("fault-armed.json"))?)?;
    assert_eq!(fault["fault"], "worker death while game is stopped");
    let pid = fault["process"]["pid"].as_u64().unwrap();
    assert!(
        !PathBuf::from(format!("/proc/{pid}")).exists(),
        "paused game survived its worker"
    );
    assert!(
        !output.join("worker.json").exists(),
        "worker unexpectedly returned through normal cleanup"
    );
    Ok(())
}

#[test]
#[ignore = "requires prepared Hype save, private Wine/Gamescope and KDE; selected by --local"]
fn native_boundary_replay_publishes_18_completed_updates() -> Result<()> {
    let config = PathBuf::from(
        std::env::var_os("ASTROLABE_HYPE_CONFIG").context("ASTROLABE_HYPE_CONFIG is required")?,
    );
    Config::load(&config)?;
    let focus = PathBuf::from(std::env::var_os("HOME").context("HOME missing")?)
        .join(".config/kwinrulesrc");
    let focus_before = fs::read(&focus)?;
    let evidence = std::env::var_os("ASTROLABE_EVIDENCE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../TestResults"));
    fs::create_dir_all(&evidence)?;
    let output = evidence.join(format!(
        "original-replay-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    let status = Command::new(env!("CARGO_BIN_EXE_astrolabe-testkit"))
        .arg("replay-original")
        .arg("--config")
        .arg(&config)
        .arg("--output")
        .arg(&output)
        .status()?;
    let summary: Value = serde_json::from_slice(&fs::read(output.join("summary.json"))?)?;
    assert!(status.success(), "{summary}");
    assert_eq!(summary["cleanup_complete"], true);
    assert_eq!(fs::read(&focus)?, focus_before);
    Config::load(&config)?;
    let report: Value = serde_json::from_slice(&fs::read(output.join("worker.json"))?)?;
    assert_eq!(report["status"], "replay_complete");
    assert_eq!(report["consistency"], "completed_update_hook");
    assert_eq!(report["updates"].as_array().unwrap().len(), 18);
    for (index, row) in report["updates"].as_array().unwrap().iter().enumerate() {
        assert_eq!(row["ordinal"], index + 1);
        assert_eq!(row["boundary_acknowledged_ordinal"], index + 1);
        if index > 0 {
            assert_eq!(
                row["update_counter"].as_u64().unwrap(),
                report["updates"][index - 1]["update_counter"]
                    .as_u64()
                    .unwrap()
                    + 1
            );
        }
        let ordinal = index + 1;
        assert_eq!(row["planned_left"], matches!(ordinal, 3 | 5 | 8..=12));
        assert_eq!(row["planned_left_shift"], matches!(ordinal, 8..=12));
    }
    println!("native replay evidence: {}", output.display());
    Ok(())
}
