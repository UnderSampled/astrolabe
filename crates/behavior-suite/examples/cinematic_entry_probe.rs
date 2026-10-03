//! Source route inspection, or bounded New Game entry in an isolated installation.
use anyhow::{Context, Result, ensure};
use orrery::{config::Config, orrery::GameSession, session::prepare_isolated_config};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let root = PathBuf::from(arguments.next().context("evidence directory")?);
    let (source_only, camera_records) = match arguments.next() {
        None => (false, false),
        Some(flag) if flag == "--source-only" => (true, false),
        Some(flag) if flag == "--camera-records" => (false, true),
        Some(_) => anyhow::bail!("expected optional --source-only or --camera-records"),
    };
    ensure!(arguments.next().is_none(), "unexpected extra arguments");
    fs::create_dir_all(&root)?;
    let source_game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let ways = astrolabe::openspace::level::inspect_actor_ways(
        &source_game,
        &astrolabe::model::Selection::hype("manoir", "en")?,
        &astrolabe::model::AssetPath::new("level:/persos/world/0/5")?,
    )?;
    if source_only {
        fs::write(
            root.join("source-routes.json"),
            serde_json::to_vec_pretty(&ways)?,
        )?;
        return Ok(());
    }
    let base = PathBuf::from(
        std::env::var_os("ASTROLABE_HYPE_CONFIG")
            .context("ASTROLABE_HYPE_CONFIG must identify the prepared private installation")?,
    );
    let config = Config::load(&base)?;
    let isolated = prepare_isolated_config(
        &base,
        &config.game.join("GameData/SaveGame"),
        &root.join("isolated"),
    )?;
    let mut game = GameSession::start_session_with_bridge(
        &isolated,
        &root.join("runs"),
        Duration::from_secs(90),
        &std::env::current_dir()?.join("target/validation/orrery"),
    )?;
    let result = (|| -> Result<()> {
        let start = Instant::now();
        while game.state()?["phase"] != 38 {
            ensure!(
                start.elapsed() < Duration::from_secs(35),
                "main menu deadline"
            );
            thread::sleep(Duration::from_millis(100));
        }
        thread::sleep(Duration::from_millis(1000));
        let before = json!({"state":game.state()?,"screenshot":game.screenshot()?});
        let first = game.keys(&["Return".into()], 60, 0)?;
        let mut frames = Vec::new();
        let mut first_scene_screenshot = None;
        for _ in 0..80 {
            let state = game.state()?;
            ensure!(
                state["phase"] != 9 || state["level"] == "manoir",
                "New Game reached an unexpected level: {state}"
            );
            let main_actor = if state["phase"] == 9 {
                Some(
                    match game.inspect_main_actor_with_camera(&[0, 1, 2], Some(&[]), true) {
                        Ok(value) => json!({"inspection":value}),
                        Err(error) => json!({"error":format!("{error:#}")}),
                    },
                )
            } else {
                None
            };
            let camera = if camera_records && state["phase"] == 9 {
                Some(
                    match game.inspect_scene_camera(
                        "manoir",
                        [2, 2, 2],
                        &[24, 26, 28, 29, 30, 31, 38, 39, 40, 41],
                    ) {
                        Ok(value) => json!({"inspection":value}),
                        Err(error) => json!({"error":format!("{error:#}")}),
                    },
                )
            } else {
                None
            };
            let gogoud = if state["phase"] == 9 {
                Some(
                    match game.inspect_scene_actor_by_types(
                        "manoir",
                        ways.object_types,
                        &[0, 1, 23, 24, 26],
                        &[],
                    ) {
                        Ok(value) => json!({"inspection":value}),
                        Err(error) => json!({"error":format!("{error:#}")}),
                    },
                )
            } else {
                None
            };
            if state["phase"] == 9 && first_scene_screenshot.is_none() {
                first_scene_screenshot = Some(game.screenshot()?);
            }
            frames.push(json!({"elapsed_ms":start.elapsed().as_millis(),"state":state,"main_actor":main_actor,"gogoud":gogoud,"camera":camera}));
            thread::sleep(Duration::from_millis(100));
        }
        let camera_count = frames
            .iter()
            .filter(|frame| frame["camera"]["inspection"].is_object())
            .count();
        let camera_errors = frames
            .iter()
            .filter(|frame| frame["camera"]["error"].is_string())
            .count();
        let report = json!({"scope":if camera_records {"original_new_game_opening_camera_records"} else {"original_new_game_first_in_engine_frames"},"video_policy":"prepared base PlayVideo=0 copied unchanged; AVI and full intro path untested","capture_limit":"each selected actor inspection is a separate stopped-byte capture; camera actor/linked records/active viewport share one pause; no synchronized completed-update/render claim","camera_records_requested":camera_records,"camera_capture_count":camera_count,"camera_error_count":camera_errors,"source_routes":ways,"before":before,"first_input":first,"first_scene_screenshot":first_scene_screenshot,"frames":frames,"ending_screenshot":game.screenshot()?});
        fs::write(
            root.join("report.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
        ensure!(
            !camera_records || camera_count > 0 && camera_errors == 0,
            "camera record capture incomplete: {camera_count} captures, {camera_errors} errors; report retained"
        );
        Ok(())
    })();
    let cleanup = game.close();
    result?;
    cleanup?;
    Ok(())
}
