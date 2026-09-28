//! Reusable original-game discriminator for saved record `0:5:4986210`.
//!
//! To compile this research template without changing the test manifest, copy
//! it to `crates/behavior-suite/tests/magic_default_probe_temp.rs` and run:
//! `cargo test --locked --offline -p behavior-suite --test magic_default_probe_temp --no-run`
//! To execute the live experiment, append `-- --ignored --nocapture`. The live
//! run uses two isolated installations/save roots and requires permission for
//! Orrery's private PipeWire/PulseAudio Unix sockets and Gamescope runtime.

#![cfg(target_os = "linux")]

use anyhow::{Context, Result, ensure};
use orrery::{
    config::{Config, SAVE_SHA256, digest},
    orrery::GameSession,
    save, save_registry,
    session::prepare_isolated_config,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const TARGET_ID: &str = "0:5:4986210";
const TARGET_POSITION: usize = 4_986_210;
const EXPECTED_SAVE_SHA256: &str =
    "877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c";
const EXPECTED_EXE_SHA256: &str =
    "048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da";

fn evidence_root() -> Result<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../output/savegame-semantic-map/magic-default-restore-runs");
    fs::create_dir_all(&root)?;
    let path = root.join(format!(
        "run-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    fs::create_dir(&path)?;
    Ok(path)
}

fn extract_fixture(destination: &Path) -> Result<()> {
    let archive = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reference/Savefile_EN.zip");
    ensure!(
        digest(&archive)? == EXPECTED_SAVE_SHA256,
        "supplied fixture hash mismatch"
    );
    ensure!(
        SAVE_SHA256 == EXPECTED_SAVE_SHA256,
        "compiled Orrery fixture hash differs"
    );
    fs::create_dir(destination)?;
    for (relative, bytes) in save::members(&archive)? {
        let target = destination.join(relative);
        fs::create_dir_all(target.parent().context("save member parent")?)?;
        fs::write(target, bytes)?;
    }
    Ok(())
}

/// Local experimental writer: encode every decoded eight-byte group literally.
/// This is deliberately not a production save writer.
fn encode_literal(decoded: &[u8]) -> Result<Vec<u8>> {
    let decoded_size = u32::try_from(decoded.len()).context("save exceeds u32")?;
    let mut stream = Vec::with_capacity(decoded.len() + decoded.len().div_ceil(8) * 10);
    for chunk in decoded.chunks(8) {
        stream.extend_from_slice(&[0, 0xff]);
        stream.extend_from_slice(chunk);
        stream.extend(std::iter::repeat_n(0, 8 - chunk.len()));
        if chunk.len() < 8 {
            stream.push(chunk.len() as u8);
        }
    }
    let checksum = stream
        .iter()
        .fold(0_u8, |sum, byte| sum.wrapping_add(*byte));
    let mut encoded = Vec::with_capacity(stream.len() + 5);
    encoded.push(checksum ^ 0x53);
    encoded.extend_from_slice(&(decoded_size ^ 0x5455_5657).to_le_bytes());
    let mut key = 0x57_u8;
    for byte in stream {
        encoded.push(byte ^ key.reverse_bits() ^ 0xb9);
        key = key.wrapping_add(1);
    }
    ensure!(
        save::decode(&encoded)? == decoded,
        "literal re-encode mismatch"
    );
    Ok(encoded)
}

fn patch_target(level_save: &Path, value: u16) -> Result<Value> {
    let mut decoded = save::decode(&fs::read(level_save)?)?;
    let before = save::records(&decoded)?;
    let record = before.get(TARGET_ID).context("target record missing")?;
    ensure!(
        record["kind"] == "0x02"
            && record["flags"] == "0x0204"
            && record["position"] == 478
            && record["value"] == 20
            && record["value_hex"] == "1400",
        "unexpected baseline target record: {record}"
    );
    let position = record["position"].as_u64().context("position absent")? as usize;
    let payload = position
        .checked_add(10)
        .context("payload offset overflow")?;
    ensure!(
        decoded.get(payload..payload + 2) == Some(&[0x14, 0x00][..]),
        "raw target bytes are not 14 00"
    );
    decoded[payload..payload + 2].copy_from_slice(&value.to_le_bytes());
    let after = save::records(&decoded)?;
    let changed: Vec<_> = before
        .keys()
        .filter(|identity| before[*identity] != after[*identity])
        .cloned()
        .collect();
    ensure!(
        changed == [TARGET_ID.to_owned()],
        "unexpected changed identities: {changed:?}"
    );
    fs::write(level_save, encode_literal(&decoded)?)?;

    let save_root = level_save
        .parent()
        .and_then(Path::parent)
        .context("fixture SaveGame0 parent missing")?;
    let report = save::inspect_with_profile(save_root, Some("hype-pc-english-supplied-save"))?;
    let verified = report["files"]["SaveGame0/Level000.sav"]["records"][TARGET_ID].clone();
    let bytes = value.to_le_bytes();
    let bytes_hex = format!("{:02x}{:02x}", bytes[0], bytes[1]);
    ensure!(
        verified["value"] == value && verified["value_hex"] == bytes_hex,
        "post-encode full record verification failed: {verified}"
    );
    Ok(json!({
        "identity": TARGET_ID,
        "value": value,
        "encoded_bytes_le": bytes_hex,
        "record": verified,
        "changed_record_identities": changed,
    }))
}

fn wait_for_title(game: &GameSession) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(40);
    loop {
        if game.state().is_ok_and(|state| state["phase"] == 38) {
            thread::sleep(Duration::from_millis(2500));
            return Ok(());
        }
        ensure!(
            Instant::now() < deadline,
            "title menu was not ready within 40 s"
        );
        thread::sleep(Duration::from_millis(100));
    }
}

fn load_supplied_slot(game: &mut GameSession) -> Result<()> {
    game.keys(&["Down".into()], 100, 300)?;
    thread::sleep(Duration::from_millis(800));
    game.keys(&["Return".into()], 100, 300)?;
    thread::sleep(Duration::from_millis(1200));
    game.keys(&["Return".into()], 100, 300)?;
    Ok(())
}

fn wait_for_manoir(game: &mut GameSession) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        let state = game.state()?;
        if state["phase"] == 9 && state["level"] == "manoir" {
            let idle_deadline = Instant::now() + Duration::from_secs(12);
            loop {
                if game
                    .observe_main_actor()
                    .is_ok_and(|actor| actor.current_action == Some(47))
                {
                    break;
                }
                ensure!(
                    Instant::now() < idle_deadline,
                    "saved actor did not reach idle"
                );
                thread::sleep(Duration::from_millis(100));
            }
            // Allow the known manor fade/title state to settle before sampling.
            thread::sleep(Duration::from_secs(8));
            return Ok(game.state()?);
        }
        ensure!(Instant::now() < deadline, "manoir load timeout at {state}");
        thread::sleep(Duration::from_millis(100));
    }
}

fn read_live_default(game: &GameSession) -> Result<Value> {
    let table =
        game.read_memory(save_registry::MODULE_BLOCK_TABLE as usize + 5 * 4, "u32", 0)?["value"]
            .as_u64()
            .context("Fix module 5 block table missing")? as usize;
    ensure!(table > 0x10000, "Fix block table address is implausible");
    // The saved identity is Fix module 5, block 0. Read that entry at +0,
    // unlike Level module 5 block 1, whose table entry is at +16.
    let base = game.read_memory(table, "u32", 0)?["value"]
        .as_u64()
        .context("Fix block 0 base missing")? as usize;
    let end = game.read_memory(table + 4, "u32", 0)?["value"]
        .as_u64()
        .context("Fix block 0 end missing")? as usize;
    let address = base
        .checked_add(TARGET_POSITION)
        .context("Fix source address overflow")?;
    ensure!(
        base < address && address + 2 <= end,
        "Fix module 5 block 0 does not contain target: base=0x{base:x}, end=0x{end:x}"
    );
    let value = game.read_memory(address, "u16", 0)?["value"]
        .as_u64()
        .context("live target word missing")?;
    Ok(json!({
        "module": 5,
        "block": 0,
        "base": format!("0x{base:x}"),
        "end": format!("0x{end:x}"),
        "offset": TARGET_POSITION,
        "address": format!("0x{address:x}"),
        "raw_u16": value,
        "raw_bytes_le": format!("{:02x} {:02x}", value as u8, (value >> 8) as u8),
    }))
}

fn run_trial(
    root: &Path,
    label: &str,
    input: u16,
    config_path: &Path,
    bridge: &Path,
) -> Result<Value> {
    let trial = root.join(label);
    fs::create_dir(&trial)?;
    let source = trial.join("source-fixture");
    extract_fixture(&source)?;
    let encoded = patch_target(&source.join("SaveGame0/Level000.sav"), input)?;
    let isolated = trial.join("isolated");
    let isolated_config = prepare_isolated_config(config_path, &source, &isolated)?;
    let mut game = GameSession::start_session_with_bridge(
        &isolated_config,
        &isolated.join("runs"),
        Duration::from_secs(180),
        bridge,
    )?;
    let began = Instant::now();
    let observation = (|| -> Result<Value> {
        wait_for_title(&game)?;
        load_supplied_slot(&mut game)?;
        let state = wait_for_manoir(&mut game)?;
        ensure!(
            state["phase"] == 9 && state["level"] == "manoir",
            "unexpected loaded state: {state}"
        );
        let mode = game.read_memory(0x0075_4420, "u8", 0)?;
        ensure!(mode["value"] == 9, "unexpected load-mode word: {mode}");
        let mut samples = Vec::new();
        for sample in 0..4 {
            samples.push(json!({
                "sample": sample,
                "elapsed_ms": began.elapsed().as_millis(),
                "live_default": read_live_default(&game)?,
                "mana": game.mana()?,
                "state": game.state()?,
            }));
            if sample != 3 {
                thread::sleep(Duration::from_millis(500));
            }
        }
        let stable = samples.windows(2).all(|pair| {
            pair[0]["live_default"]["raw_u16"] == pair[1]["live_default"]["raw_u16"]
                && pair[0]["mana"]["current"] == pair[1]["mana"]["current"]
                && pair[0]["mana"]["maximum"] == pair[1]["mana"]["maximum"]
        });
        ensure!(stable, "post-load observations were unstable: {samples:?}");
        for sample in &samples {
            ensure!(
                sample["state"]["phase"] == 9
                    && sample["state"]["level"] == "manoir"
                    && sample["live_default"]["raw_u16"] == input
                    && sample["mana"]["current"] == 30
                    && sample["mana"]["maximum"] == 50,
                "opposed save did not restore the expected live state: {sample}"
            );
        }
        let after =
            save::inspect_with_profile(&game.save_root(), Some("hype-pc-english-supplied-save"))?;
        let target_after = after["files"]["SaveGame0/Level000.sav"]["records"][TARGET_ID].clone();
        ensure!(
            target_after["kind"] == "0x02"
                && target_after["flags"] == "0x0204"
                && target_after["value"] == input
                && target_after["value_hex"] == encoded["encoded_bytes_le"],
            "opposed input was rewritten on disk: {target_after}"
        );
        Ok(json!({
            "label": label,
            "input_value": input,
            "encoded_input": encoded,
            "loaded_state": state,
            "load_mode_word": mode,
            "stable": stable,
            "samples": samples,
            "target_after_load_on_disk": target_after,
            "input_overwritten_on_disk": target_after["value"] != input,
            "elapsed_ms": began.elapsed().as_millis(),
        }))
    })();
    // Always close the owned game before returning either success or failure.
    let closed = game.close();
    let observation = observation?;
    closed?;
    fs::write(
        trial.join("trial-result.json"),
        serde_json::to_vec_pretty(&observation)?,
    )?;
    Ok(observation)
}

#[test]
#[ignore = "live original-game probe; requires private Gamescope/PipeWire runtime permission"]
fn original_magic_default_fresh_process_opposed_values() -> Result<()> {
    let config_path = std::env::var_os("ASTROLABE_HYPE_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/stephen/Projects/hype-harness/local/config.json"));
    let config = Config::load(&config_path)?;
    ensure!(
        config.exe_sha256 == EXPECTED_EXE_SHA256,
        "pinned executable hash mismatch"
    );
    let root = evidence_root()?;
    let bridge = std::env::current_exe()?
        .parent()
        .and_then(Path::parent)
        .context("test binary outside Cargo profile directory")?
        .join("orrery");
    ensure!(
        bridge.is_file(),
        "Orrery bridge binary missing: {}",
        bridge.display()
    );
    let began = Instant::now();
    let low = run_trial(&root, "value-8", 8, &config_path, &bridge)?;
    let high = run_trial(&root, "value-40", 40, &config_path, &bridge)?;
    let report = json!({
        "schema": 1,
        "status": "complete",
        "scope": "fresh-process restore for 0:5:4986210 and supplied first-slot manor only",
        "fixture_sha256": EXPECTED_SAVE_SHA256,
        "executable_sha256": EXPECTED_EXE_SHA256,
        "meaning": "unsigned DefaultMagicPoints value; MagicLevel clamps it to maximum and seeds current points",
        "trials": [low, high],
        "total_elapsed_ms": began.elapsed().as_millis(),
    });
    fs::write(
        root.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("Magic default restore evidence: {}", root.display());
    Ok(())
}
