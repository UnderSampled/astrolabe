//! Source-derived native expiry regression, separate from paired gameplay acceptance.
use anyhow::{Context, Result, ensure};
use astrolabe::{
    godot,
    model::{
        AssetPath, Selection,
        io::{Content, ContentRoot},
        scene::Scene,
    },
    openspace::content,
};
use std::{
    fs,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[test]
#[ignore = "requires local Hype assets and Godot; exported component fixture, not original-game parity"]
fn exported_crossbow_free_flight_expires_and_releases_guards() -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let project = temporary.path().join("project");
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let selection = Selection::hype("manoir", "en")?;
    let mut imported = content::import_scene(&game, selection.clone())?;
    godot::support::initialize(&project)?;
    godot::export(
        &mut imported,
        &[ContentRoot {
            kind: Scene::KIND.into(),
            reference: AssetPath::new("level:/scene")?,
        }],
        selection,
        &project,
        &engine,
    )?;
    let templates = fs::read_dir(project.join("generated/hype/levels/manoir/persos"))?
        .map(|entry| -> Result<Option<PathBuf>> {
            let path = entry?.path();
            Ok((path
                .extension()
                .is_some_and(|extension| extension == "tscn")
                && fs::read_to_string(&path)?.contains("/ai/models/733.gd"))
            .then_some(path))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    ensure!(
        templates.len() == 1,
        "expected one actual exported model-733 template"
    );
    let template = format!("res://{}", templates[0].strip_prefix(&project)?.display());
    fs::create_dir_all(project.join("tests"))?;
    fs::write(
        project.join("tests/crossbow_free_flight.gd"),
        include_bytes!("../godot/tests/crossbow_free_flight.gd"),
    )?;
    let evidence = std::env::var_os("ASTROLABE_EVIDENCE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| "TestResults".into());
    fs::create_dir_all(&evidence)?;
    let id = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    for (case, yaw) in [("straight", "0.0"), ("aimed", "0.2")] {
        let report = evidence
            .canonicalize()?
            .join(format!("crossbow-native-free-flight-{case}-{id}.json"));
        let output = godot::run_godot(
            &engine,
            &project,
            &[
                "--headless",
                "--script",
                "res://tests/crossbow_free_flight.gd",
                "--",
                &template,
                report.to_str().context("non-UTF8 evidence path")?,
                yaw,
            ],
            Duration::from_secs(35),
        )?;
        fs::write(
            evidence.join(format!("crossbow-native-free-flight-{case}-{id}.log")),
            &output,
        )?;
        ensure!(output.contains("CROSSBOW_FREE_FLIGHT_OK"), "{output}");
        println!(
            "Native exported crossbow expiry evidence: {}",
            report.display()
        );
    }
    Ok(())
}
