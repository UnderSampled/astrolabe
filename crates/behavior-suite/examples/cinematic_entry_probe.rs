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
    let source_only = match arguments.next() {
        None => false,
        Some(flag) if flag == "--source-only" => true,
        Some(_) => anyhow::bail!("expected optional --source-only"),
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
            frames.push(json!({"elapsed_ms":start.elapsed().as_millis(),"state":state,"main_actor":main_actor,"gogoud":gogoud}));
            thread::sleep(Duration::from_millis(100));
        }
        let report = json!({"scope":"original_new_game_first_in_engine_frames","video_policy":"prepared base PlayVideo=0 copied unchanged; AVI and full intro path untested","capture_limit":"actor and viewport inspections are separate stopped-byte observations; no synchronized completed-update claim","source_routes":ways,"before":before,"first_input":first,"first_scene_screenshot":first_scene_screenshot,"frames":frames,"ending_screenshot":game.screenshot()?});
        fs::write(
            root.join("report.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
        Ok(())
    })();
    let cleanup = game.close();
    result?;
    cleanup?;
    Ok(())
}
