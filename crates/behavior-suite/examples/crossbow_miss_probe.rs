//! Bounded physical-input fixture probe; does not claim an expiry pass after contact.
use anyhow::{Context, Result, ensure};
use behavior_contract::input::{Edge, Event, Key};
use orrery::orrery::NativeController;
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

fn main() -> Result<()> {
    let root = PathBuf::from(std::env::args_os().nth(1).context("evidence directory")?);
    fs::create_dir_all(&root)?;
    let config = std::env::var_os("ASTROLABE_HYPE_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| "/home/stephen/Projects/hype-harness/local/config.json".into());
    let mut controller = NativeController::new(config, root.join("runs"))
        .with_bridge_binary(std::env::current_dir()?.join("target/validation/orrery"));
    controller.start(90)?;
    let result = (|| -> Result<()> {
        let game = controller.game()?;
        game.keys(&["Down".into(), "Return".into(), "Return".into()], 100, 300)?;
        let deadline = Instant::now() + Duration::from_secs(35);
        loop {
            if game.state()?["phase"] == 9
                && game.state()?["level"] == "manoir"
                && game.observe_main_actor()?.current_action == Some(47)
            {
                break;
            }
            ensure!(
                Instant::now() < deadline,
                "manor fixture did not become ready"
            );
            thread::sleep(Duration::from_millis(100));
        }
        thread::sleep(Duration::from_millis(2500));
        let baseline = game.inventory()?;
        let setup = game.prepare_crossbow_quantity()?;
        game.prepare_physical_input()?;
        let origin = Instant::now();
        let mut inputs = Vec::new();
        let deliver = |game: &mut orrery::orrery::GameSession,
                       inputs: &mut Vec<serde_json::Value>,
                       key,
                       edge|
         -> Result<()> {
            let event = Event {
                keyboard: 0,
                key,
                edge,
            };
            game.physical_events(std::slice::from_ref(&event))?;
            inputs.push(json!({"at_ms":origin.elapsed().as_millis(),"event":event}));
            Ok(())
        };
        deliver(game, &mut inputs, Key::Z, Edge::Press)?;
        thread::sleep(Duration::from_millis(55));
        deliver(game, &mut inputs, Key::Z, Edge::Release)?;
        thread::sleep(Duration::from_millis(900));
        ensure!(
            game.observe_main_actor()?.current_action == Some(48),
            "source did not arm"
        );
        deliver(game, &mut inputs, Key::Keypad0, Edge::Press)?;
        deliver(game, &mut inputs, Key::ArrowUp, Edge::Press)?;
        thread::sleep(Duration::from_millis(900));
        deliver(game, &mut inputs, Key::ArrowUp, Edge::Release)?;
        let aiming =
            game.inspect_main_actor_with_camera(&[48, 290, 271], Some(&[58, 17, 120]), true)?;
        fs::write(
            root.join("aiming.json"),
            serde_json::to_vec_pretty(&json!({
                "baseline":baseline,"setup":setup,"inputs":inputs,"aiming":aiming
            }))?,
        )?;
        deliver(game, &mut inputs, Key::Space, Edge::Press)?;
        thread::sleep(Duration::from_millis(80));
        deliver(game, &mut inputs, Key::Space, Edge::Release)?;
        let mut frames = Vec::new();
        for index in 0..140 {
            let observation = match game.observe_latest_projectile() {
                Ok(observation) => observation,
                Err(error) => {
                    fs::write(
                        root.join("report.json"),
                        serde_json::to_vec_pretty(&json!({
                            "scope":"original_crossbow_upward_miss_fixture_probe",
                            "status":"blocked_fixture_left_expected_gameplay",
                            "error":format!("{error:#}"),"index":index,
                            "state":game.state()?,"baseline":baseline,"setup":setup,
                            "inputs":inputs,"aiming":aiming,"frames":frames,
                            "screenshot":game.screenshot()?
                        }))?,
                    )?;
                    return Ok(());
                }
            };
            frames.push(json!({"index":index,"elapsed_ms":origin.elapsed().as_millis(),"observation":observation}));
            if index == 6 {
                deliver(game, &mut inputs, Key::Keypad0, Edge::Release)?;
            }
            thread::sleep(Duration::from_millis(50));
        }
        let terminal = game.observe_magick_behavior(
            &[47, 48, 290, 271],
            &[58, 17, 120],
            Some(2),
            true,
            Some((17, 0, 0x4e324)),
            &[0, 1, 2, 3],
            None,
        )?;
        let inventory = game.inventory()?;
        let evidence = json!({"scope":"original_crossbow_upward_miss_fixture_probe","acceptance":"diagnostic; contact prevents an expiry claim","setup":setup,"inputs":inputs,"aiming":aiming,"frames":frames,"terminal":terminal,"inventory":inventory,"screenshot":game.screenshot()?});
        fs::write(
            root.join("report.json"),
            serde_json::to_vec_pretty(&evidence)?,
        )?;
        Ok(())
    })();
    let close = controller.stop();
    result?;
    close?;
    Ok(())
}
