#![cfg(target_os = "linux")]
//! Source-only persistence case using the real Hype menu and save operation.
use anyhow::{Context, Result, ensure};
use astrolabe::openspace::level::live::InventoryObservation;
use orrery::{
    config::{Config, SAVE_SHA256, digest},
    desktop::Desktop,
    orrery::GameSession,
    save, save_registry,
    session::prepare_isolated_config,
};
use serde_json::{Value, json};
use session_recording::{capture_saves, inspect_saves, verify_saves};
use std::{
    fs,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Copy)]
enum SaveScreen {
    ListOrName,
    ConfirmYes,
    ConfirmNo,
    Transition,
}

fn save_screen(path: &Path) -> Result<SaveScreen> {
    let mut decoder = png::Decoder::new(BufReader::new(File::open(path)?));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let mut pixels = vec![0; reader.output_buffer_size().context("PNG output buffer")?];
    let info = reader.next_frame(&mut pixels)?;
    ensure!(
        info.width == 1024 && info.height == 768 && info.color_type == png::ColorType::Rgb,
        "save menu screenshot does not match the verified PC display profile"
    );
    let color = |x: usize, y: usize| -> [u8; 3] {
        let at = (y * 1024 + x) * 3;
        pixels[at..at + 3].try_into().unwrap()
    };
    let [background, _, _] = color(500, 380);
    if background >= 80 {
        return Ok(SaveScreen::ListOrName);
    }
    let red_count = |x0: usize, x1: usize| -> usize {
        (438..465)
            .flat_map(|y| (x0..x1).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let [r, g, b] = color(x, y);
                r > 120 && r > g.saturating_mul(2) && r > b.saturating_mul(2)
            })
            .count()
    };
    let yes = red_count(395, 485);
    let no = red_count(545, 625);
    if no > 8 && no > yes {
        Ok(SaveScreen::ConfirmNo)
    } else if yes > 8 && yes > no {
        Ok(SaveScreen::ConfirmYes)
    } else {
        Ok(SaveScreen::Transition)
    }
}

fn wait_manoir(game: &mut GameSession) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        let state = game.state()?;
        if state["phase"] == 9 && state["level"] == "manoir" {
            let ready = Instant::now() + Duration::from_secs(12);
            loop {
                if game
                    .observe_main_actor()
                    .is_ok_and(|actor| actor.current_action == Some(47))
                {
                    break;
                }
                ensure!(
                    Instant::now() < ready,
                    "saved actor did not reach idle after fade-in"
                );
                thread::sleep(Duration::from_millis(100));
            }
            // Phase 9 and the idle action appear while the manor title/fade is
            // still visible. This observed startup wait is a menu readiness
            // gate, not a frame-precise gameplay replay.
            thread::sleep(Duration::from_millis(8000));
            return game.state();
        }
        if Instant::now() >= deadline {
            let screenshot = game.screenshot()?;
            anyhow::bail!("save did not reach manoir: {state}; screenshot: {screenshot}");
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn wait_title_menu(game: &GameSession) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(40);
    loop {
        if game.state().is_ok_and(|state| state["phase"] == 38) {
            thread::sleep(Duration::from_millis(2500));
            return Ok(());
        }
        ensure!(
            Instant::now() < deadline,
            "Hype title menu did not become ready"
        );
        thread::sleep(Duration::from_millis(100));
    }
}

fn load_first_slot(game: &mut GameSession) -> Result<()> {
    game.keys(&["Down".into()], 100, 300)?;
    thread::sleep(Duration::from_millis(800));
    game.keys(&["Return".into()], 100, 300)?;
    thread::sleep(Duration::from_millis(1200));
    game.keys(&["Return".into()], 100, 300)?;
    Ok(())
}

fn save_first_slot(game: &mut GameSession, evidence: &Path, approach_lectern: bool) -> Result<()> {
    fs::write(
        evidence.join("save-approach-before.json"),
        serde_json::to_vec_pretty(&json!({
            "state":game.state()?,
            "actor":game.observe_main_actor()?,
            "screenshot":game.screenshot()?,
        }))?,
    )?;
    if approach_lectern {
        let input = game.keys(&["Up".into()], 1500, 250)?;
        fs::write(
            evidence.join("save-approach-after.json"),
            serde_json::to_vec_pretty(&json!({
                "input":input,
                "state":game.state()?,
                "actor":game.observe_main_actor()?,
                "screenshot":game.screenshot()?,
            }))?,
        )?;
    }
    game.keys(&["space".into()], 150, 250)?;
    if let Err(first_error) = wait_save_menu(game) {
        fs::write(
            evidence.join("save-menu-missed.json"),
            serde_json::to_vec_pretty(&json!({
                "error":format!("{first_error:#}"),
                "state":game.state()?,
                "actor":game.observe_main_actor()?,
                "screenshot":game.screenshot()?,
            }))?,
        )?;
        ensure!(
            !approach_lectern,
            "save menu did not open after approaching lectern: {first_error:#}"
        );
        let state = game.state()?;
        let actor = game.observe_main_actor()?;
        ensure!(
            state["phase"] == 9 && actor.current_action == Some(47),
            "lever save retry requires idle manor gameplay: {state}, action {:?}",
            actor.current_action
        );
        // A second distinct press is permitted only after the first press
        // definitively left gameplay active. This covers a missed input edge
        // following the lever's own Space interaction without writing a save
        // through any synthetic path.
        thread::sleep(Duration::from_millis(500));
        let retry_input = game.keys(&["space".into()], 150, 250)?;
        let retry = wait_save_menu(game);
        fs::write(
            evidence.join("save-menu-retry.json"),
            serde_json::to_vec_pretty(&json!({
                "input":retry_input,
                "result":retry.as_ref().map_err(|error| format!("{error:#}")),
                "state":game.state()?,
                "screenshot":game.screenshot()?,
            }))?,
        )?;
        retry?;
    }
    fs::write(
        evidence.join("menu-open.json"),
        serde_json::to_vec_pretty(&game.state()?)?,
    )?;
    let mut observations = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        let state = game.state()?;
        if state["phase"] == 9 {
            fs::write(
                evidence.join("save-menu-observations.json"),
                serde_json::to_vec_pretty(&observations)?,
            )?;
            return Ok(());
        }
        ensure!(
            state["phase"] == 38,
            "unexpected phase while confirming game save: {state}"
        );
        let capture = game.screenshot()?;
        let path = Path::new(capture["path"].as_str().context("save screenshot path")?);
        let screen = save_screen(path)?;
        observations
            .push(json!({"frame":path,"screen":format!("{screen:?}"),"phase":state["phase"]}));
        match screen {
            SaveScreen::ListOrName | SaveScreen::ConfirmYes => {
                game.keys(&["Return".into()], 80, 500)?;
            }
            SaveScreen::ConfirmNo => {
                game.keys(&["Left".into()], 80, 500)?;
            }
            SaveScreen::Transition => thread::sleep(Duration::from_millis(250)),
        }
    }
    fs::write(
        evidence.join("save-menu-observations.json"),
        serde_json::to_vec_pretty(&observations)?,
    )?;
    anyhow::bail!("save menu did not return to gameplay: {}", game.state()?)
}

fn wait_save_menu(game: &GameSession) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let state = game.state()?;
        if state["phase"] == 38 {
            return Ok(state);
        }
        ensure!(Instant::now() < deadline, "save menu did not open: {state}");
        thread::sleep(Duration::from_millis(100));
    }
}

fn new_case_dir(label: &str) -> Result<PathBuf> {
    let base = std::env::var_os("ASTROLABE_EVIDENCE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../output/savegame-investigation")
        });
    fs::create_dir_all(&base)?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = base.join(format!("source-{label}-proof-{stamp}"));
    fs::create_dir(&path)?;
    Ok(path)
}

fn archive_fixture(root: &Path) -> Result<PathBuf> {
    let archive = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reference/Savefile_EN.zip");
    ensure!(digest(&archive)? == SAVE_SHA256, "supplied archive changed");
    let destination = root.join("source-fixture");
    fs::create_dir(&destination)?;
    for (relative, bytes) in save::members(&archive)? {
        let target = destination.join(relative);
        fs::create_dir_all(target.parent().context("save member parent")?)?;
        fs::write(target, bytes)?;
    }
    Ok(destination)
}

/// Research fixture encoder: every 8-byte window is emitted as literals.
/// This is deliberately local to the original-game discriminator, not a
/// general save writer or an Astrolabe content codec.
fn encode_literal_save(decoded: &[u8]) -> Result<Vec<u8>> {
    let decoded_size = u32::try_from(decoded.len()).context("decoded save exceeds u32")?;
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
        "literal save did not round-trip"
    );
    Ok(encoded)
}

fn set_isolated_save_byte(path: &Path, id: &str, value: u8) -> Result<()> {
    set_isolated_save_byte_from(path, id, 0, value)
}

fn set_isolated_save_byte_from(path: &Path, id: &str, expected: u8, value: u8) -> Result<()> {
    let mut decoded = save::decode(&fs::read(path)?)?;
    let before = save::records(&decoded)?;
    let record = before.get(id).context("target save identity absent")?;
    ensure!(
        record["kind"] == "0x01"
            && record["flags"] == "0x0a0c"
            && record["value"] == json!(expected),
        "target save record changed from guarded one-byte fixture value {expected}: {record}"
    );
    let position = record["position"]
        .as_u64()
        .context("record position missing")? as usize;
    let at = position
        .checked_add(10)
        .context("record value offset overflow")?;
    ensure!(
        decoded.get(at) == Some(&expected),
        "target payload differs from guarded value {expected}"
    );
    decoded[at] = value;
    let after = save::records(&decoded)?;
    let changed = before
        .keys()
        .filter(|identity| before[*identity] != after[*identity])
        .cloned()
        .collect::<Vec<_>>();
    let expected_changes = if expected == value {
        Vec::new()
    } else {
        vec![id.to_owned()]
    };
    ensure!(
        changed == expected_changes,
        "literal fixture changed other records: {changed:?}"
    );
    fs::write(path, encode_literal_save(&decoded)?)?;
    Ok(())
}

fn bridge_binary() -> Result<PathBuf> {
    let executable = std::env::current_exe()?;
    let profile_dir = executable
        .parent()
        .and_then(Path::parent)
        .context("test executable is outside a Cargo profile directory")?;
    let bridge = profile_dir.join("orrery");
    ensure!(
        bridge.is_file(),
        "Orrery display bridge is missing: {}",
        bridge.display()
    );
    Ok(bridge)
}

fn crossbow_quantity(inventory: &InventoryObservation) -> Result<i16> {
    let matches = inventory
        .active
        .iter()
        .filter(|item| item.kind == "Weapon" && item.id == 80)
        .collect::<Vec<_>>();
    ensure!(
        matches.len() == 1,
        "expected one active crossbow entry: {inventory:?}"
    );
    Ok(matches[0].quantity)
}

fn lever_unlock(value: &Value) -> Result<bool> {
    let selected = &value["observation"]["designer_values"][0];
    ensure!(
        selected["index"] == 8,
        "lever flag observation has the wrong designer index"
    );
    selected["value"]["Value"]["Boolean"]
        .as_bool()
        .context("lever unlock is not a typed Boolean")
}

fn run_no_action_control(
    base_config: &Path,
    bridge: &Path,
    root: &Path,
    run_name: &str,
) -> Result<Value> {
    fs::create_dir(root)?;
    let source = archive_fixture(root)?;
    let source_manifest = inspect_saves(&source)?;
    ensure!(
        source_manifest.files.len() == 4,
        "independently staged supplied slot is incomplete"
    );
    let isolated = root.join("isolated");
    let isolated_config = prepare_isolated_config(base_config, &source, &isolated)?;
    let pinned = Config::load_base(&isolated_config)?;
    ensure!(
        pinned.exe_sha256 == Config::load(base_config)?.exe_sha256
            && digest(
                &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reference/Savefile_EN.zip")
            )? == SAVE_SHA256,
        "isolated config does not match the pinned executable and supplied save archive"
    );
    let mut game = GameSession::start_session_with_bridge(
        &isolated_config,
        &isolated.join("runs"),
        Duration::from_secs(180),
        bridge,
    )?;
    let run_result = (|| -> Result<Value> {
        wait_title_menu(&game)?;
        load_first_slot(&mut game)?;
        let loaded = wait_manoir(&mut game)?;
        let mana_before = game.mana()?;
        let inventory_before = game.inventory()?;
        let dummy_before = game.inspect_scene_actor_by_types("manoir", [4, 5, 50], &[0], &[8])?;
        ensure!(
            mana_before["current"] == 30 && mana_before["maximum"] == 50,
            "no-action control did not load supplied 30/50 mana: {mana_before}"
        );
        ensure!(
            !inventory_before.active.iter().any(|item| {
                (item.kind == "MagicIce" && item.id == 54)
                    || (item.kind == "MagicLightning" && item.id == 74)
            }),
            "supplied fixture already has MagicIce or MagicLightning available"
        );
        ensure!(
            !lever_unlock(&dummy_before)?,
            "supplied fixture already has the dummy unlock flag"
        );
        save_first_slot(&mut game, root, true)?;
        thread::sleep(Duration::from_millis(750));
        let stable = inspect_saves(&game.save_root())?;
        thread::sleep(Duration::from_millis(750));
        ensure!(
            stable == inspect_saves(&game.save_root())?,
            "no-action game-written save root was not stable"
        );
        let written_root = root.join("written-saves");
        let captured = capture_saves(&game.save_root(), &written_root)?;
        verify_saves(&written_root, &captured)?;
        ensure!(
            captured.files.len() >= 4,
            "game-written save root is incomplete"
        );
        let mana_after = game.mana()?;
        let inventory_after = game.inventory()?;
        let dummy_after = game.inspect_scene_actor_by_types("manoir", [4, 5, 50], &[0], &[8])?;
        ensure!(
            mana_before["current"] == mana_after["current"]
                && mana_before["maximum"] == mana_after["maximum"],
            "normal save changed live mana: before={mana_before}, after={mana_after}"
        );
        ensure!(
            inventory_before.active == inventory_after.active,
            "normal save changed live typed inventory"
        );
        ensure!(
            lever_unlock(&dummy_before)? == lever_unlock(&dummy_after)?,
            "normal save changed live dummy unlock"
        );
        fs::write(
            root.join("live-before.json"),
            serde_json::to_vec_pretty(&json!({
                "loaded_state": loaded,
                "mana": mana_before,
                "inventory": inventory_before,
                "dummy_flag": dummy_before,
            }))?,
        )?;
        fs::write(
            root.join("live-after.json"),
            serde_json::to_vec_pretty(&json!({
                "mana": mana_after,
                "inventory": inventory_after,
                "dummy_flag": dummy_after,
            }))?,
        )?;
        Ok(json!({
            "run": run_name,
            "archive_sha256": digest(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reference/Savefile_EN.zip"))?,
            "config_executable_sha256": pinned.exe_sha256,
            "isolated_config_sha256": digest(&isolated_config)?,
            "captured_save_root": written_root,
            "save_root_manifest": captured,
        }))
    })();
    let close = game.close();
    let run = run_result?;
    close?;

    let diff = save::diff_with_slots(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reference/Savefile_EN.zip"),
        &root.join("written-saves"),
        None,
        None,
        Some("hype-pc-english-supplied-save"),
    )?;
    for (id, expected) in [("0:5:4986212", json!(30)), ("0:5:4986214", json!(50))] {
        let record = &diff["files"]["SaveGame0/Level000.sav"]["record_changes"][id];
        ensure!(
            record.is_null()
                || (record["before"]["value"] == expected && record["after"]["value"] == expected),
            "no-action control changed known mana record {id}: {record}"
        );
    }
    for id in ["0:10:1560", "0:10:2800"] {
        let record = &diff["files"]["SaveGame0/Level000.sav"]["record_changes"][id];
        ensure!(
            record.is_null()
                || (record["before"]["value_hex"] == "00" && record["after"]["value_hex"] == "00"),
            "no-action control changed known magic availability record {id}: {record}"
        );
    }
    let dummy = &diff["files"]["SaveGame0/Level000.sav"]["record_changes"]["0:17:297732"];
    ensure!(
        dummy.is_null()
            || (dummy["before"]["value_hex"] == "00" && dummy["after"]["value_hex"] == "00"),
        "no-action control changed known dummy unlock record: {dummy}"
    );
    fs::write(
        root.join("save-diff.json"),
        serde_json::to_vec_pretty(&diff)?,
    )?;
    Ok(json!({"run": run, "raw_save_diff": diff}))
}

#[test]
#[ignore = "requires prepared Hype installation, Wine and Gamescope; selected by --local"]
fn original_no_action_save_is_stable_across_independent_runs() -> Result<()> {
    let config = std::env::var_os("ASTROLABE_HYPE_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/stephen/Projects/hype-harness/local/config.json"));
    let pinned = Config::load(&config)?;
    let root = new_case_dir("no-action-control")?;
    let bridge = bridge_binary()?;
    let first = run_no_action_control(&config, &bridge, &root.join("run-1"), "run-1")?;
    let second = run_no_action_control(&config, &bridge, &root.join("run-2"), "run-2")?;
    let changed_ids = |run: &Value| -> Result<Vec<String>> {
        Ok(
            run["raw_save_diff"]["files"]["SaveGame0/Level000.sav"]["record_changes"]
                .as_object()
                .context("control has no player-record diff")?
                .keys()
                .cloned()
                .collect(),
        )
    };
    ensure!(
        changed_ids(&first)? == changed_ids(&second)?,
        "independent no-action saves changed different player record identities"
    );
    let report = json!({
        "schema": 1,
        "status": "passed",
        "scope": "original_no_action_save_control",
        "archive_sha256": SAVE_SHA256,
        "executable_sha256": pinned.exe_sha256,
        "independent_staged_copies": true,
        "runs": [first, second],
    });
    fs::write(
        root.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("Original no-action save control: {}", root.display());
    Ok(())
}

#[test]
#[ignore = "requires prepared Hype installation, Wine and Gamescope; selected by --local"]
fn original_player_and_named_save_precedence_for_manor_actor() -> Result<()> {
    const FIELD: &str = "1:5:833347";
    const OFFSET: u32 = 833347;
    let config = std::env::var_os("ASTROLABE_HYPE_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/stephen/Projects/hype-harness/local/config.json"));
    let pinned = Config::load(&config)?;
    let root = new_case_dir("player-named-precedence")?;
    let bridge = bridge_binary()?;
    let mut outcomes = Vec::new();
    for (label, player, named) in [("named-one", 0_u8, 1_u8), ("player-one", 1, 0)] {
        let case = root.join(label);
        fs::create_dir(&case)?;
        let source = archive_fixture(&case)?;
        set_isolated_save_byte(&source.join("SaveGame0/Level000.sav"), FIELD, player)?;
        set_isolated_save_byte(&source.join("SaveGame0/manoir.sav"), FIELD, named)?;
        let inspection = save::inspect(&source)?;
        for (file, expected) in [("Level000.sav", player), ("manoir.sav", named)] {
            let key = format!("SaveGame0/{file}");
            ensure!(
                inspection["files"][key.as_str()]["records"][FIELD]["value"] == json!(expected),
                "isolated conflict fixture did not retain {file} value {expected}"
            );
        }
        let isolated = case.join("isolated");
        let isolated_config = prepare_isolated_config(&config, &source, &isolated)?;
        let mut game = GameSession::start_session_with_bridge(
            &isolated_config,
            &isolated.join("runs"),
            Duration::from_secs(150),
            &bridge,
        )?;
        let run =
            (|| -> Result<Value> {
                wait_title_menu(&game)?;
                load_first_slot(&mut game)?;
                let state = wait_manoir(&mut game)?;
                let pointer =
                    game.read_memory(save_registry::MODULE_BLOCK_TABLE as usize + 5 * 4, "u32", 0)?
                        ["value"]
                        .as_u64()
                        .context("level module block-table pointer missing")?
                        as usize;
                ensure!(pointer > 0x10000, "level module block table is absent");
                let block_entry = pointer + 16;
                let base = game.read_memory(block_entry, "u32", 0)?["value"]
                    .as_u64()
                    .context("level block base missing")? as usize;
                let end = game.read_memory(block_entry + 4, "u32", 0)?["value"]
                    .as_u64()
                    .context("level block end missing")? as usize;
                let field_address = base
                    .checked_add(OFFSET as usize)
                    .context("level field address overflow")?;
                ensure!(
                    base < field_address && field_address < end,
                    "target field is outside loaded level block"
                );
                let first = game.read_memory(field_address, "u8", 0)?;
                let load_mode = game.read_memory(0x0075_4420, "u8", 0)?;
                thread::sleep(Duration::from_millis(300));
                let second = game.read_memory(field_address, "u8", 0)?;
                ensure!(
                    first["value"] == second["value"],
                    "field changed during observation"
                );
                Ok(json!({
                    "label": label,
                    "phase": state["phase"],
                    "level": state["level"],
                    "player_save_value": player,
                    "named_save_value": named,
                    "observed_value": first["value"],
                    "load_mode_at_observation": load_mode["value"],
                    "field_identity": FIELD,
                    "field_address": format!("0x{field_address:x}"),
                    "module_block_table": format!("0x{pointer:x}"),
                    "source_files": {
                        "player_sha256": digest(&source.join("SaveGame0/Level000.sav"))?,
                        "named_sha256": digest(&source.join("SaveGame0/manoir.sav"))?,
                    },
                }))
            })();
        let close = game.close();
        let observation = run?;
        close?;
        outcomes.push(observation);
    }
    let values = outcomes
        .iter()
        .map(|outcome| outcome["observed_value"].as_u64())
        .collect::<Vec<_>>();
    let result = match values.as_slice() {
        [Some(1), Some(0)] => "named_file_value_after_first_slot_load",
        [Some(0), Some(1)] => "player_file_value_after_first_slot_load",
        _ => "later_initialization_or_no_apply",
    };
    let report = json!({
        "schema": 1,
        "status": if result == "later_initialization_or_no_apply" {"inconclusive"} else {"passed"},
        "result": result,
        "original_archive_sha256": SAVE_SHA256,
        "executable_sha256": pinned.exe_sha256,
        "cases": outcomes,
    });
    fs::write(
        root.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    ensure!(
        result != "later_initialization_or_no_apply",
        "conflicting-file test was inconclusive; see {}",
        root.join("report.json").display()
    );
    println!("Original player/named precedence: {}", root.display());
    Ok(())
}

/// Exercise the state-5 level-change path separately from first-slot startup.
/// Each case begins with an opposed player/named byte, then uses the verified
/// manor teleporter route to leave for Astrolabe and return to the manor.
#[test]
#[ignore = "requires prepared Hype installation, Wine and Gamescope; selected by --local"]
fn original_named_level_mode5_manor_record_precedence() -> Result<()> {
    const FIELD: &str = "1:5:833347";
    const OFFSET: u32 = 833347;
    let config = std::env::var_os("ASTROLABE_HYPE_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/stephen/Projects/hype-harness/local/config.json"));
    let pinned = Config::load(&config)?;
    let root = new_case_dir("named-mode5-transition")?;
    let bridge = bridge_binary()?;
    let mut cases = Vec::new();

    for (label, player_value, named_value) in [("named-one", 0_u8, 1_u8), ("player-one", 1, 0)] {
        let case_root = root.join(label);
        fs::create_dir_all(&case_root)?;
        let source = archive_fixture(&case_root)?;
        set_isolated_save_byte(&source.join("SaveGame0/Level000.sav"), FIELD, player_value)?;
        set_isolated_save_byte(&source.join("SaveGame0/manoir.sav"), FIELD, named_value)?;
        let verified = save::inspect(&source)?;
        for (member, expected) in [("Level000.sav", player_value), ("manoir.sav", named_value)] {
            let key = format!("SaveGame0/{member}");
            ensure!(
                verified["files"][key.as_str()]["records"][FIELD]["value"] == json!(expected),
                "conflict fixture failed decoded verification for {member}: {verified}"
            );
        }
        fs::write(
            case_root.join("fixture-verification.json"),
            serde_json::to_vec_pretty(&json!({
                "label":label,
                "player_value":player_value,
                "named_value":named_value,
                "player_sha256":digest(&source.join("SaveGame0/Level000.sav"))?,
                "named_sha256":digest(&source.join("SaveGame0/manoir.sav"))?,
                "inspection":verified,
            }))?,
        )?;

        let isolated = case_root.join("isolated");
        let isolated_config = prepare_isolated_config(&config, &source, &isolated)?;
        let game_start = GameSession::start_session_with_bridge(
            &isolated_config,
            &isolated.join("runs"),
            Duration::from_secs(180),
            &bridge,
        );
        let mut game = match game_start {
            Ok(game) => game,
            Err(error) => {
                let failure = json!({
                    "label":label,
                    "checkpoint":"owned_session_start",
                    "error":format!("{error:#}"),
                    "isolated_config":isolated_config,
                    "source_fixture":source,
                    "run_logs":isolated.join("runs"),
                    "screenshot_available":false,
                });
                fs::write(
                    case_root.join("startup-failure.json"),
                    serde_json::to_vec_pretty(&failure)?,
                )?;
                let partial = json!({
                    "schema":1,
                    "status":"inconclusive",
                    "failed_case":label,
                    "checkpoint":"owned_session_start",
                    "failure":failure,
                    "archive_sha256":SAVE_SHA256,
                    "executable_sha256":pinned.exe_sha256,
                });
                fs::write(
                    root.join("report.json"),
                    serde_json::to_vec_pretty(&partial)?,
                )?;
                return Err(error);
            }
        };
        let run = (|| -> Result<Value> {
            wait_title_menu(&game)?;
            load_first_slot(&mut game)?;
            let initial = wait_manoir(&mut game)?;
            let initial_field = read_registered_manor_byte(&game, OFFSET)?;
            ensure!(
                initial_field["value"] == json!(player_value),
                "first-slot startup did not expose player fixture value: {initial_field}"
            );
            let initial_capture = json!({
                "state":initial,
                "load_mode":game.read_memory(0x0075_4420, "u8", 0)?,
                "field":initial_field,
                "save_members":save_member_snapshot(&game, &case_root.join("saves-initial.json"))?,
                "screenshot":game.screenshot()?,
            });
            fs::write(
                case_root.join("initial-manoir.json"),
                serde_json::to_vec_pretty(&initial_capture)?,
            )?;

            // Tele11 is gated by the hidden manor lever in this supplied slot.
            // Reproduce its documented in-game unlock path; the helper restores
            // the lever placement and does not save or alter fixture files.
            let activation =
                game.activate_secret_manor_lever(&case_root.join("lever-navigation.json"))?;
            ensure!(
                lever_unlock(&activation["activation"]["after_flag"])?
                    && activation["save_files_modified"] == false,
                "tele11 prerequisite was not activated in-process: {activation}"
            );

            let away = capture_transition(&mut game, "tele11", "astrolabe", &case_root)?;
            ensure!(
                away["reached"]["phase"] == 9 && away["reached"]["level"] == "astrolabe",
                "verified tele11 destination was not reached: {away}"
            );
            ensure!(
                away["mode5_observed"] == true,
                "state-5 load mode was not sampled during tele11: {away}"
            );
            let saves_after_away =
                save_member_snapshot(&game, &case_root.join("saves-after-tele11.json"))?;
            ensure!(
                saves_after_away["members"]["HYPE/manoir.sav"]["record"]["value"]
                    == json!(player_value),
                "tele11 departure did not leave the expected player value in HYPE/manoir.sav: {saves_after_away}"
            );
            // The state-6 departure path saves the outgoing level to HYPE
            // before switching names. Reinsert the opposed named byte in this
            // isolated working file after tele11 so tele05 can discriminate
            // mode-5 application from the departure autosave. This is a
            // disk-file intervention in the disposable copy, never a live
            // process-memory write.
            let hype_manor = game.save_root().join("HYPE/manoir.sav");
            set_isolated_save_byte_from(&hype_manor, FIELD, player_value, named_value)?;
            let saves_before_return =
                save_member_snapshot(&game, &case_root.join("saves-before-tele05.json"))?;
            ensure!(
                saves_before_return["members"]["HYPE/manoir.sav"]["record"]["value"]
                    == json!(named_value)
                    && saves_before_return["members"]["SaveGame0/manoir.sav"]["record"]["value"]
                        == json!(named_value),
                "opposed HYPE and SaveGame0 named bytes were not verified before tele05: {saves_before_return}"
            );
            let back = capture_transition(&mut game, "tele05", "manoir", &case_root)?;
            ensure!(
                back["reached"]["phase"] == 9 && back["reached"]["level"] == "manoir",
                "verified tele05 return destination was not reached: {back}"
            );
            ensure!(
                back["mode5_observed"] == true,
                "state-5 load mode was not sampled during tele05 return: {back}"
            );
            let saves_after_return =
                save_member_snapshot(&game, &case_root.join("saves-after-tele05.json"))?;
            ensure!(
                saves_after_return["members"]["HYPE/manoir.sav"]["record"]["value"]
                    == json!(named_value),
                "reinserted named conflict did not remain in HYPE/manoir.sav at return: {saves_after_return}"
            );
            let playable = wait_manoir(&mut game)?;
            let first = read_registered_manor_byte(&game, OFFSET)?;
            thread::sleep(Duration::from_millis(300));
            let second = read_registered_manor_byte(&game, OFFSET)?;
            ensure!(
                first["value"] == second["value"],
                "named-level field changed during stable post-return observation: {first} -> {second}"
            );
            let result = json!({
                "label":label,
                "player_save_value":player_value,
                "named_save_value":named_value,
                "initial_player_value":initial_field["value"],
                "initial_state":initial,
                "tele11_away":away,
                "tele05_return":back,
                "saves_after_tele11":saves_after_away,
                "saves_before_tele05":saves_before_return,
                "named_byte_reinserted_into_hype_manor_after_tele11":named_value,
                "save_file_intervention":"decoded and re-encoded only isolated GameData/SaveGame/HYPE/manoir.sav after tele11 autosaved the outgoing manor; no live process memory was patched",
                "saves_after_tele05":saves_after_return,
                "playable_return_state":playable,
                "post_return_first":first,
                "post_return_second":second,
                "final_screenshot":game.screenshot()?,
                "field_identity":FIELD,
                "expected_after_mode5":named_value,
            });
            fs::write(
                case_root.join("case-result.json"),
                serde_json::to_vec_pretty(&result)?,
            )?;
            Ok(result)
        })();
        let observation = match run {
            Ok(value) => {
                game.close()?;
                value
            }
            Err(error) => {
                let failure = json!({
                    "label":label,
                    "error":format!("{error:#}"),
                    "state":game.state().ok(),
                    "screenshot":game.screenshot().ok(),
                });
                let _ = fs::write(
                    case_root.join("failure.json"),
                    serde_json::to_vec_pretty(&failure).unwrap_or_default(),
                );
                game.close()?;
                let partial = json!({
                    "schema":1,"status":"inconclusive","failed_case":label,
                    "completed_cases":cases,"failure":failure,
                    "archive_sha256":SAVE_SHA256,"executable_sha256":pinned.exe_sha256,
                });
                fs::write(
                    root.join("report.json"),
                    serde_json::to_vec_pretty(&partial)?,
                )?;
                return Err(error);
            }
        };
        cases.push(observation);
    }

    let follows_named = cases
        .iter()
        .all(|case| case["post_return_second"]["value"] == case["named_save_value"]);
    let follows_player = cases
        .iter()
        .all(|case| case["post_return_second"]["value"] == case["player_save_value"]);
    let named_fixture_reinserted_before_return = cases.iter().all(|case| {
        case["named_byte_reinserted_into_hype_manor_after_tele11"] == case["named_save_value"]
            && case["saves_after_tele05"]["members"]["HYPE/manoir.sav"]["record"]["value"]
                == case["named_save_value"]
    });
    let result = if follows_named {
        "named_file_value_after_state5_level_return"
    } else if follows_player {
        "player_file_value_after_state5_level_return"
    } else {
        "other_or_later_initialization"
    };
    let report = json!({
        "schema":1,
        "status":if follows_named && named_fixture_reinserted_before_return {"passed"} else {"inconclusive"},
        "scope":"original_named_level_mode5_manor_record_precedence",
        "result":result,
        "archive_sha256":SAVE_SHA256,
        "executable_sha256":pinned.exe_sha256,
        "cases":cases,
        "named_fixture_reinserted_before_return":named_fixture_reinserted_before_return,
        "interpretation_limit":"The ordinary tele11 departure auto-saves manor to HYPE/manoir.sav, so this test reinserts the opposed named byte in that ignored file after tele11 and before tele05. It tests state-5/mode-8 named-file application under that explicit file intervention, not normal unmodified save continuity or quest progression.",
    });
    fs::write(
        root.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    ensure!(
        follows_named && named_fixture_reinserted_before_return,
        "named-level mode-5 application was not demonstrated (result: {result}, named file was reinserted and verified: {named_fixture_reinserted_before_return}); see {}",
        root.join("report.json").display()
    );
    println!("Original named-level mode-5 test: {}", root.display());
    Ok(())
}

fn save_member_snapshot(game: &GameSession, path: &Path) -> Result<Value> {
    let report =
        save::inspect_with_profile(&game.save_root(), Some("hype-pc-english-supplied-save"))?;
    let files = report["files"]
        .as_object()
        .context("save inspection files missing")?;
    let mut members = serde_json::Map::new();
    for name in [
        "SaveGame0/Level000.sav",
        "SaveGame0/manoir.sav",
        "HYPE/Level000.sav",
        "HYPE/manoir.sav",
    ] {
        if let Some(file) = files.get(name) {
            members.insert(
                name.to_owned(),
                json!({
                    "sha256":file["sha256"],
                    "decoded_size":file["decoded_size"],
                    "record":file["records"]["1:5:833347"],
                }),
            );
        }
    }
    let snapshot = json!({"save_root":game.save_root(),"members":members});
    fs::write(path, serde_json::to_vec_pretty(&snapshot)?)?;
    Ok(snapshot)
}

fn read_registered_manor_byte(game: &GameSession, offset: u32) -> Result<Value> {
    let pointer =
        game.read_memory(save_registry::MODULE_BLOCK_TABLE as usize + 5 * 4, "u32", 0)?["value"]
            .as_u64()
            .context("level module block-table pointer missing")? as usize;
    ensure!(pointer > 0x10000, "level module block table is absent");
    let entry = pointer + 16;
    let base = game.read_memory(entry, "u32", 0)?["value"]
        .as_u64()
        .context("level block base missing")? as usize;
    let end = game.read_memory(entry + 4, "u32", 0)?["value"]
        .as_u64()
        .context("level block end missing")? as usize;
    let address = base
        .checked_add(offset as usize)
        .context("target field overflow")?;
    ensure!(
        base < address && address < end,
        "target field is outside loaded manor block"
    );
    Ok(json!({
        "value":game.read_memory(address,"u8",0)?["value"],
        "address":format!("0x{address:x}"),
        "block_base":format!("0x{base:x}"),
        "block_end":format!("0x{end:x}"),
    }))
}

fn capture_transition(
    game: &mut GameSession,
    cheat: &str,
    destination: &str,
    evidence: &Path,
) -> Result<Value> {
    let input = game.cheat(cheat, 80, 100)?;
    let deadline = Instant::now() + Duration::from_secs(45);
    let mut samples = Vec::new();
    let reached = loop {
        let state = game.state()?;
        let mode = game.read_memory(0x0075_4420, "u8", 0)?;
        samples.push(json!({"state":state,"load_mode":mode,"elapsed_ms":45000 - deadline.saturating_duration_since(Instant::now()).as_millis()}));
        if state["phase"] == 9 && state["level"] == destination {
            break state;
        }
        if Instant::now() >= deadline {
            break state;
        }
        thread::sleep(Duration::from_millis(15));
    };
    let mode5 = samples
        .iter()
        .any(|sample| sample["load_mode"]["value"] == 5);
    let screenshot = game.screenshot()?;
    let observation = json!({
        "cheat":cheat,
        "input":input,
        "expected_destination":destination,
        "reached":reached,
        "mode5_observed":mode5,
        "samples":samples,
        "screenshot":screenshot,
    });
    fs::write(
        evidence.join(format!("transition-{cheat}.json")),
        serde_json::to_vec_pretty(&observation)?,
    )?;
    Ok(observation)
}

#[test]
#[ignore = "requires prepared Hype installation, Wine and Gamescope; selected by --local"]
fn original_mana_save_survives_fresh_process() -> Result<()> {
    let config = std::env::var_os("ASTROLABE_HYPE_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/stephen/Projects/hype-harness/local/config.json"));
    Config::load(&config)?;
    let root = new_case_dir("mana")?;
    let source = archive_fixture(&root)?;
    let initial = inspect_saves(&source)?;
    ensure!(initial.files.len() == 4, "supplied slot is incomplete");
    let first_config = prepare_isolated_config(&config, &source, &root.join("first"))?;
    let bridge = bridge_binary()?;
    let mut first = GameSession::start_session_with_bridge(
        &first_config,
        &root.join("first/runs"),
        Duration::from_secs(180),
        &bridge,
    )?;
    let first_result = (|| -> Result<(Value, Value, Value)> {
        wait_title_menu(&first)?;
        first.screenshot()?;
        load_first_slot(&mut first)?;
        let loaded = wait_manoir(&mut first)?;
        first.screenshot()?;
        fs::write(
            root.join("first/loaded-actor.json"),
            serde_json::to_vec_pretty(&first.observe_main_actor()?)?,
        )?;
        let before = first.mana()?;
        fs::write(
            root.join("first/loaded-mana.json"),
            serde_json::to_vec_pretty(&before)?,
        )?;
        ensure!(
            before["current"] == 30 && before["maximum"] == 50,
            "selected load did not reach the supplied 30/50 mana state: {before}"
        );
        let cheat = first.cheat("druidik", 80, 150)?;
        fs::write(
            root.join("first/cheat.json"),
            serde_json::to_vec_pretty(&cheat)?,
        )?;
        let deadline = Instant::now() + Duration::from_secs(3);
        let changed = loop {
            let mana = first.mana()?;
            if mana["current"] == 50 || Instant::now() >= deadline {
                break mana;
            }
            thread::sleep(Duration::from_millis(100));
        };
        first.screenshot()?;
        ensure!(
            changed["current"] == 50 && changed["maximum"] == 50,
            "druidik did not refill mana: {changed}"
        );
        save_first_slot(&mut first, &root.join("first"), true)?;
        let deadline = Instant::now() + Duration::from_secs(12);
        let written = loop {
            let report = save::inspect_with_profile(
                &first.save_root(),
                Some("hype-pc-english-supplied-save"),
            )?;
            if report["files"]["SaveGame0/Level000.sav"]["records"]["0:5:4986212"]["value"] == 50 {
                break report;
            }
            ensure!(
                Instant::now() < deadline,
                "game save did not contain refilled mana; final state: {}",
                first.state()?
            );
            thread::sleep(Duration::from_millis(250));
        };
        thread::sleep(Duration::from_millis(750));
        let stable = inspect_saves(&first.save_root())?;
        thread::sleep(Duration::from_millis(750));
        ensure!(
            stable == inspect_saves(&first.save_root())?,
            "save files were not stable"
        );
        let captured = capture_saves(&first.save_root(), &root.join("first/written-saves"))?;
        verify_saves(&root.join("first/written-saves"), &captured)?;
        ensure!(
            captured.files.len() >= 4,
            "game-written save set is incomplete"
        );
        fs::write(
            root.join("first/save-inspect.json"),
            serde_json::to_vec_pretty(&written)?,
        )?;
        Ok((loaded, before, changed))
    })();
    let first_close = first.close();
    let (loaded, before, changed) = first_result?;
    first_close?;

    let diff = save::diff_with_slots(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reference/Savefile_EN.zip"),
        &root.join("first/written-saves"),
        None,
        None,
        Some("hype-pc-english-supplied-save"),
    )?;
    ensure!(
        diff["files"]["SaveGame0/Level000.sav"]["record_changes"]["0:5:4986212"]["after"]["value"]
            == 50,
        "source save diff did not include mana refill"
    );
    ensure!(
        diff["files"]["SaveGame0/Level000.sav"]["verified_semantic_changes"]["0:5:4986212"]["before"]
            == 30
            && diff["files"]["SaveGame0/Level000.sav"]["verified_semantic_changes"]["0:5:4986212"]
                ["after"]
                == 50,
        "verified mana change was not separated from raw record changes"
    );
    fs::write(
        root.join("first/save-diff.json"),
        serde_json::to_vec_pretty(&diff)?,
    )?;

    let second_config = prepare_isolated_config(
        &config,
        &root.join("first/written-saves"),
        &root.join("reload"),
    )?;
    let mut second = GameSession::start_session_with_bridge(
        &second_config,
        &root.join("reload/runs"),
        Duration::from_secs(180),
        &bridge,
    )?;
    let reload_result = (|| -> Result<(Value, Value)> {
        wait_title_menu(&second)?;
        load_first_slot(&mut second)?;
        let state = wait_manoir(&mut second)?;
        let mana = second.mana()?;
        ensure!(
            mana["current"] == 50 && mana["maximum"] == 50,
            "fresh-process reload lost mana: {mana}"
        );
        Ok((state, mana))
    })();
    let second_close = second.close();
    let (reloaded, restored) = reload_result?;
    second_close?;
    let report = json!({
        "schema": 1, "status": "passed", "scope": "original_mana_save_reload",
        "archive_sha256": SAVE_SHA256, "executable_sha256": Config::load(&config)?.exe_sha256,
        "first_loaded": loaded, "mana_before": before, "mana_after_cheat": changed,
        "reload_state": reloaded, "reload_mana": restored,
        "source_case": "druidik", "save_operation": "normal lectern and overwrite confirmation",
        "fresh_process": true, "fixture_isolated": true,
    });
    fs::write(
        root.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("Original mana save proof: {}", root.display());
    Ok(())
}

#[test]
#[ignore = "requires prepared Hype installation, Wine and Gamescope; selected by --local"]
fn original_crossbow_quantity_resets_after_reload() -> Result<()> {
    let config = std::env::var_os("ASTROLABE_HYPE_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/stephen/Projects/hype-harness/local/config.json"));
    Config::load(&config)?;
    let root = new_case_dir("crossbow")?;
    let source = archive_fixture(&root)?;
    let bridge = bridge_binary()?;
    let first_config = prepare_isolated_config(&config, &source, &root.join("first"))?;
    let mut first = GameSession::start_session_with_bridge(
        &first_config,
        &root.join("first/runs"),
        Duration::from_secs(180),
        &bridge,
    )?;
    let first_result = (|| -> Result<(InventoryObservation, InventoryObservation, InventoryObservation)> {
        wait_title_menu(&first)?;
        load_first_slot(&mut first)?;
        wait_manoir(&mut first)?;
        let baseline = first.inventory()?;
        ensure!(crossbow_quantity(&baseline)? == 0, "source crossbow baseline changed");
        let intervention = first.prepare_crossbow_quantity()?;
        fs::write(root.join("first/intervention.json"), serde_json::to_vec_pretty(&intervention)?)?;
        let prepared = first.inventory()?;
        ensure!(crossbow_quantity(&prepared)? == 3, "intervention did not establish three arrows");
        // Isolate the one field set by the guarded intervention before
        // investigating whether a physical attack consumes ammunition.
        let before_save = first.inventory()?;
        ensure!(crossbow_quantity(&before_save)? == 3, "prepared quantity changed before save");
        save_first_slot(&mut first, &root.join("first"), true)?;
        let deadline = Instant::now() + Duration::from_secs(12);
        loop {
            let report = save::inspect(&first.save_root())?;
            let initial = save::inspect(&source)?;
            if report["files"]["SaveGame0/Level000.sav"]["sha256"] != initial["files"]["SaveGame0/Level000.sav"]["sha256"] {
                fs::write(root.join("first/save-inspect.json"), serde_json::to_vec_pretty(&report)?)?;
                break;
            }
            ensure!(Instant::now() < deadline, "crossbow case did not write a changed player file");
            thread::sleep(Duration::from_millis(250));
        }
        thread::sleep(Duration::from_millis(750));
        let stable = inspect_saves(&first.save_root())?;
        thread::sleep(Duration::from_millis(750));
        ensure!(stable == inspect_saves(&first.save_root())?, "crossbow save was not stable");
        capture_saves(&first.save_root(), &root.join("first/written-saves"))?;
        Ok((baseline, prepared, before_save))
    })();
    let first_close = first.close();
    let (baseline, prepared, before_save) = first_result?;
    first_close?;
    let diff = save::diff_with_slots(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reference/Savefile_EN.zip"),
        &root.join("first/written-saves"),
        None,
        None,
        Some("hype-pc-english-supplied-save"),
    )?;
    fs::write(
        root.join("first/save-diff.json"),
        serde_json::to_vec_pretty(&diff)?,
    )?;

    let second_config = prepare_isolated_config(
        &config,
        &root.join("first/written-saves"),
        &root.join("reload"),
    )?;
    let mut second = GameSession::start_session_with_bridge(
        &second_config,
        &root.join("reload/runs"),
        Duration::from_secs(180),
        &bridge,
    )?;
    let reload_result = (|| -> Result<InventoryObservation> {
        wait_title_menu(&second)?;
        load_first_slot(&mut second)?;
        wait_manoir(&mut second)?;
        let restored = second.inventory()?;
        ensure!(
            crossbow_quantity(&restored)? == 0,
            "guarded crossbow quantity unexpectedly persisted: {restored:?}"
        );
        Ok(restored)
    })();
    let second_close = second.close();
    let restored = reload_result?;
    second_close?;
    let report = json!({
        "schema":1, "status":"source_reset_demonstrated", "scope":"original_crossbow_quantity_save_reload",
        "fixture_archive_sha256":SAVE_SHA256,
        "baseline":baseline, "intervention_quantity_three":prepared,
        "before_save":before_save, "fresh_reload":restored,
        "intervention":"guarded current-process quantity setup; no save bytes edited",
        "source_record_mapping":"not found in this game-written diff",
        "acceptance":"negative case; inventory persistence requires a different source action",
    });
    fs::write(
        root.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("Original crossbow save candidate: {}", root.display());
    Ok(())
}

#[test]
#[ignore = "requires prepared Hype installation, Wine and Gamescope; selected by --local"]
fn original_secret_lever_world_state_survives_reload() -> Result<()> {
    let config = std::env::var_os("ASTROLABE_HYPE_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/stephen/Projects/hype-harness/local/config.json"));
    Config::load(&config)?;
    let root = new_case_dir("lever")?;
    let source = archive_fixture(&root)?;
    let bridge = bridge_binary()?;
    let first_config = prepare_isolated_config(&config, &source, &root.join("first"))?;
    let mut first = GameSession::start_session_with_bridge(
        &first_config,
        &root.join("first/runs"),
        Duration::from_secs(180),
        &bridge,
    )?;
    let first_result = (|| -> Result<Value> {
        wait_title_menu(&first)?;
        load_first_slot(&mut first)?;
        wait_manoir(&mut first)?;
        let activation =
            first.activate_secret_manor_lever(&root.join("first/lever-guard-observation.json"))?;
        fs::write(
            root.join("first/activation.json"),
            serde_json::to_vec_pretty(&activation)?,
        )?;
        ensure!(
            !lever_unlock(&activation["before_flag"])?,
            "lever unlock was already active"
        );
        ensure!(
            lever_unlock(&activation["activation"]["after_flag"])?,
            "lever interaction did not set the world flag"
        );
        let ready = Instant::now() + Duration::from_secs(6);
        while first.observe_main_actor()?.current_action != Some(47) {
            ensure!(
                Instant::now() < ready,
                "Hype did not finish the lever interaction"
            );
            thread::sleep(Duration::from_millis(100));
        }
        // The lever interaction can leave Hype just outside the lectern's
        // interaction range. The same physical approach used by the other
        // save scenarios reaches the save point from both observed poses.
        save_first_slot(&mut first, &root.join("first"), true)?;
        thread::sleep(Duration::from_millis(750));
        let stable = inspect_saves(&first.save_root())?;
        thread::sleep(Duration::from_millis(750));
        ensure!(
            stable == inspect_saves(&first.save_root())?,
            "lever save was not stable"
        );
        capture_saves(&first.save_root(), &root.join("first/written-saves"))?;
        Ok(activation)
    })();
    let first_close = first.close();
    let activation = first_result?;
    first_close?;
    let diff = save::diff_with_slots(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reference/Savefile_EN.zip"),
        &root.join("first/written-saves"),
        None,
        None,
        Some("hype-pc-english-supplied-save"),
    )?;
    let unlock_record = &diff["files"]["SaveGame0/Level000.sav"]["record_changes"]["0:17:297732"];
    ensure!(
        unlock_record["before"]["value_hex"] == "00"
            && unlock_record["after"]["value_hex"] == "01"
            && unlock_record["after"]["flags"] == "0x0204",
        "game-written player file did not carry the mapped dummy DSG[8] unlock: {unlock_record}"
    );
    let semantic_unlock =
        &diff["files"]["SaveGame0/Level000.sav"]["verified_semantic_changes"]["0:17:297732"];
    ensure!(
        semantic_unlock["before"] == false && semantic_unlock["after"] == true,
        "verified manor unlock change was not separated from raw record changes: {semantic_unlock}"
    );
    fs::write(
        root.join("first/save-diff.json"),
        serde_json::to_vec_pretty(&diff)?,
    )?;

    let second_config = prepare_isolated_config(
        &config,
        &root.join("first/written-saves"),
        &root.join("reload"),
    )?;
    let mut second = GameSession::start_session_with_bridge(
        &second_config,
        &root.join("reload/runs"),
        Duration::from_secs(180),
        &bridge,
    )?;
    let reload_result = (|| -> Result<(Value, Value)> {
        wait_title_menu(&second)?;
        load_first_slot(&mut second)?;
        wait_manoir(&mut second)?;
        let restored = second.inspect_scene_actor_by_types("manoir", [4, 5, 50], &[0], &[8])?;
        fs::write(
            root.join("reload/flag.json"),
            serde_json::to_vec_pretty(&restored)?,
        )?;
        ensure!(
            lever_unlock(&restored)?,
            "fresh-process reload lost the lever unlock flag"
        );
        let teleport = second.cheat("tele11", 80, 100)?;
        fs::write(
            root.join("reload/teleport-input.json"),
            serde_json::to_vec_pretty(&teleport)?,
        )?;
        let deadline = Instant::now() + Duration::from_secs(45);
        let reached = loop {
            let state = second.state()?;
            if (state["level"] == "astrolabe" && state["phase"] == 9) || Instant::now() >= deadline
            {
                break state;
            }
            thread::sleep(Duration::from_millis(100));
        };
        ensure!(
            reached["level"] == "astrolabe" && reached["phase"] == 9,
            "restored unlock did not permit tele11: {reached}"
        );
        let actor = second.observe_main_actor()?;
        ensure!(
            actor
                .global_transform
                .affine()
                .translation
                .iter()
                .all(|axis| axis.is_finite()),
            "Astrolabe gameplay actor did not have a valid pose"
        );
        fs::write(
            root.join("reload/astrolabe-actor.json"),
            serde_json::to_vec_pretty(&actor)?,
        )?;
        Ok((restored, reached))
    })();
    let second_close = second.close();
    let (restored, reached) = reload_result?;
    second_close?;
    let report = json!({
        "schema":1,"status":"passed","scope":"original_manor_lever_world_state_save_reload",
        "archive_sha256":SAVE_SHA256,"executable_sha256":Config::load(&config)?.exe_sha256,
        "activation":activation,"fresh_reload_flag":restored,"post_reload_teleport":reached,
        "intervention":"guarded lever movement to player's location, restored before game save",
        "save_operation":"normal lectern and overwrite confirmation",
        "source_record_mapping":{"file":"SaveGame0/Level000.sav","identity":"0:17:297732","owner":"iDummy_PourCercles","field":"DSG[8]","kind":"Boolean","flags":"0x0204"},
    });
    fs::write(
        root.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("Original lever world-state proof: {}", root.display());
    Ok(())
}

#[test]
#[ignore = "requires prepared Hype installation, Wine and Gamescope; selected by --local"]
fn original_magic_inventory_survives_reload() -> Result<()> {
    for key in ["Down", "Return", "Up", "space", "x", "Delete"] {
        Desktop::validate_chord(key)?;
    }
    let config = std::env::var_os("ASTROLABE_HYPE_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/stephen/Projects/hype-harness/local/config.json"));
    Config::load(&config)?;
    let root = new_case_dir("magic-inventory")?;
    let source = archive_fixture(&root)?;
    let bridge = bridge_binary()?;
    let first_config = prepare_isolated_config(&config, &source, &root.join("first"))?;
    let mut first = GameSession::start_session_with_bridge(
        &first_config,
        &root.join("first/runs"),
        Duration::from_secs(180),
        &bridge,
    )?;
    let first_result = (|| -> Result<Value> {
        wait_title_menu(&first)?;
        load_first_slot(&mut first)?;
        wait_manoir(&mut first)?;
        let phase_before = first.state()?;
        let before = first.inventory()?;
        ensure!(
            !before
                .active
                .iter()
                .any(|item| item.kind == "MagicIce" && item.id == 54)
                && !before
                    .active
                    .iter()
                    .any(|item| item.kind == "MagicLightning" && item.id == 74),
            "supplied fixture already has both selected magic entries"
        );
        let mana_before = first.mana()?;
        let input = first.cheat("thereyougo", 80, 100)?;
        thread::sleep(Duration::from_millis(500));
        let phase_after = first.state()?;
        let after = first.inventory()?;
        let mana_after = first.mana()?;
        let screenshot = first.screenshot().map_err(|error| format!("{error:#}"));
        fs::write(
            root.join("first/cheat-observation.json"),
            serde_json::to_vec_pretty(&json!({
                "phase_before":phase_before,
                "phase_after":phase_after,
                "inventory_before":before,
                "inventory_after":after,
                "mana_before":mana_before,
                "mana_after":mana_after,
                "input":input,
                "screenshot":screenshot,
            }))?,
        )?;
        ensure!(
            after
                .active
                .iter()
                .any(|item| item.kind == "MagicIce" && item.id == 54)
                && after
                    .active
                    .iter()
                    .any(|item| item.kind == "MagicLightning" && item.id == 74),
            "thereyougo did not add the two typed magic entries: {after:?}"
        );
        save_first_slot(&mut first, &root.join("first"), true)?;
        thread::sleep(Duration::from_millis(750));
        let stable = inspect_saves(&first.save_root())?;
        thread::sleep(Duration::from_millis(750));
        ensure!(
            stable == inspect_saves(&first.save_root())?,
            "magic inventory save was not stable"
        );
        capture_saves(&first.save_root(), &root.join("first/written-saves"))?;
        Ok(
            json!({"before":before,"after":after,"mana_before":mana_before,"mana_after":mana_after,"input":input}),
        )
    })();
    let first_close = first.close();
    let changed = first_result?;
    first_close?;
    let diff = save::diff_with_slots(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reference/Savefile_EN.zip"),
        &root.join("first/written-saves"),
        None,
        None,
        Some("hype-pc-english-supplied-save"),
    )?;
    for id in ["0:10:1560", "0:10:2800"] {
        let record = &diff["files"]["SaveGame0/Level000.sav"]["record_changes"][id];
        ensure!(
            record["before"]["value"] == 0
                && record["after"]["value"] == 1
                && record["after"]["kind"] == "0x01"
                && record["after"]["flags"] == "0x0204",
            "game-written inventory selected-state flag {id} did not change 0 to 1: {record}"
        );
        let semantic = &diff["files"]["SaveGame0/Level000.sav"]["verified_semantic_changes"][id];
        ensure!(
            semantic["before"] == false && semantic["after"] == true,
            "verified inventory selection was not separated from raw record changes: {semantic}"
        );
    }
    fs::write(
        root.join("first/save-diff.json"),
        serde_json::to_vec_pretty(&diff)?,
    )?;
    let second_config = prepare_isolated_config(
        &config,
        &root.join("first/written-saves"),
        &root.join("reload"),
    )?;
    let mut second = GameSession::start_session_with_bridge(
        &second_config,
        &root.join("reload/runs"),
        Duration::from_secs(180),
        &bridge,
    )?;
    let reload_result = (|| -> Result<Value> {
        wait_title_menu(&second)?;
        load_first_slot(&mut second)?;
        wait_manoir(&mut second)?;
        let restored = second.inventory()?;
        fs::write(
            root.join("reload/inventory.json"),
            serde_json::to_vec_pretty(&restored)?,
        )?;
        ensure!(
            restored
                .active
                .iter()
                .any(|item| item.kind == "MagicIce" && item.id == 54)
                && restored
                    .active
                    .iter()
                    .any(|item| item.kind == "MagicLightning" && item.id == 74),
            "fresh-process reload lost the added magic entries: {restored:?}"
        );
        let selection_before = second.magic_selection()?;
        let draw_before = json!({
            "selection":selection_before,
            "actor":second.observe_main_actor()?,
            "screenshot":second.screenshot()?,
        });
        fs::write(
            root.join("reload/weapon-draw.json"),
            serde_json::to_vec_pretty(&json!({"before":draw_before,"attempted":false}))?,
        )?;
        // The manual says DEL selects combat spells while a weapon is in hand.
        // The prior screenshot shows the sword sheathed on Hype's back.
        let draw_input = second.keys(&["x".into()], 100, 400)?;
        thread::sleep(Duration::from_millis(700));
        let weapon_draw = json!({
            "before":draw_before,
            "input":draw_input,
            "after_selection":second.magic_selection()?,
            "after_actor":second.observe_main_actor()?,
            "after_screenshot":second.screenshot()?,
            "manual_precondition":"DEL selects a spell with a weapon in hand; review screenshot for drawn weapon",
        });
        fs::write(
            root.join("reload/weapon-draw.json"),
            serde_json::to_vec_pretty(&weapon_draw)?,
        )?;
        let cycle_input = second.keys(&["Delete".into()], 100, 250)?;
        thread::sleep(Duration::from_millis(400));
        let cycle_selection = second.magic_selection()?;
        let cycle_observation = json!({
            "input":cycle_input,
            "selection":cycle_selection,
            "mana":second.mana()?,
            "actor":second.observe_main_actor()?,
            "screenshot":second.screenshot()?,
        });
        fs::write(
            root.join("reload/delete-after-draw.json"),
            serde_json::to_vec_pretty(&cycle_observation)?,
        )?;
        let active_spell = cycle_selection
            .source_code
            .filter(|code| *code == 54 || *code == 74);
        let cast_observation = if let Some(code) = active_spell {
            let before_selection = second.magic_selection()?;
            let before_mana = before_selection.current_points;
            let before_actor = second.observe_main_actor()?;
            let before_screenshot = second.screenshot()?;
            let pre_cast = json!({
                "attempted":false,
                "source_code":code,
                "cycle_selection":cycle_selection,
                "before_selection":before_selection,
                "before_mana":before_mana,
                "before_actor":before_actor,
                "before_screenshot":before_screenshot,
            });
            fs::write(
                root.join("reload/spell-cast-observation.json"),
                serde_json::to_vec_pretty(&pre_cast)?,
            )?;
            ensure!(
                before_selection.source_code == Some(code),
                "selected spell changed before Enter was sent; see reload/spell-cast-observation.json"
            );
            let input = second.keys(&["Return".into()], 120, 400)?;
            thread::sleep(Duration::from_millis(700));
            let after_selection = second.magic_selection()?;
            let after_mana = second.mana()?;
            let after_actor = second.observe_main_actor()?;
            let screenshot = second.screenshot()?;
            let observation = json!({
                "attempted":true,
                "source_code":code,
                "input":input,
                "cycle_selection":cycle_selection,
                "before_selection":before_selection,
                "before_mana":before_mana,
                "before_actor":before_actor,
                "after_selection":after_selection,
                "after_mana":after_mana,
                "after_actor":after_actor,
                "screenshot":screenshot,
            });
            // Keep the full input and post-cast evidence even when the game did
            // not produce a confirmed effect.
            fs::write(
                root.join("reload/spell-cast-observation.json"),
                serde_json::to_vec_pretty(&observation)?,
            )?;
            let mana_spent = after_mana["current"]
                .as_u64()
                .is_some_and(|value| value < u64::from(before_mana));
            let action_changed = after_actor.current_action != before_actor.current_action;
            ensure!(
                mana_spent,
                "spell {code} was selected and Return was sent, but mana did not decrease to confirm a cast; actor action changed: {action_changed}; see reload/spell-cast-observation.json"
            );
            Some(json!({
                "observation":observation,
                "mana_decreased":mana_spent,
                "actor_action_changed":action_changed,
                "effect_confirmed_by_mana":mana_spent,
            }))
        } else {
            None
        };
        let equipment = json!({
            "before":selection_before,
            "weapon_draw":weapon_draw,
            "delete_after_draw":cycle_observation,
            "cast":cast_observation,
            "gameplay_status":if active_spell.is_some() { "cast_confirmed" } else { "pending_spell_selection" },
        });
        fs::write(
            root.join("reload/equipment.json"),
            serde_json::to_vec_pretty(&equipment)?,
        )?;
        Ok(json!({
            "inventory":restored,
            "selection_before":selection_before,
            "equipment":equipment,
        }))
    })();
    let second_close = second.close();
    let reloaded = reload_result?;
    second_close?;
    let status = if reloaded["equipment"]["gameplay_status"] == "cast_confirmed" {
        "inventory_reload_and_spell_cast_confirmed"
    } else {
        "inventory_reload_demonstrated"
    };
    fs::write(
        root.join("report.json"),
        serde_json::to_vec_pretty(
            &json!({"status":status,"changed":changed,"fresh_reload":reloaded,"source_records":{"MagicIce:54":"0:10:1560","MagicLightning:74":"0:10:2800"},"source_semantics":"the two record bytes are selected/equipped flags; they are not availability flags"}),
        )?,
    )?;
    println!("Original magic inventory save proof: {}", root.display());
    Ok(())
}
