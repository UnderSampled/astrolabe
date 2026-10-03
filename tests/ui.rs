//! Maintained presentation contracts and the evidenced manor localization slice.
use anyhow::Result;
use astrolabe::{
    godot,
    model::{Selection, ai::BehaviorGroup, dialogue::Dialogue, io::Content},
    openspace::level::import_scene,
    rete,
};
use std::{fs, path::PathBuf, time::Duration};

#[test]
#[ignore = "requires local English manor data; source-only, no native sessions"]
fn manor_dialogue_localization_voice_and_callback_survive_rete() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("disc"));
    let imported = import_scene(&game, &Selection::hype("manoir", "en")?)?;
    assert_eq!(imported.dialogues.len(), 2);
    for (name, box_style, count, member, model, normal) in [
        ("gogood_texte1_manoir", 3, 3, 0, "level:/ai/models/210", 4),
        (
            "gogood_master_whatif01",
            1,
            21,
            12,
            "level:/ai/models/991",
            17,
        ),
    ] {
        let value = imported.dialogues.iter().find(|d| d.name == name).unwrap();
        value.validate()?;
        assert_eq!((value.box_style, value.lines.len()), (box_style, count));
        let voice = value.voice.as_ref().unwrap();
        assert_eq!((voice.bank, voice.member), (142, member));
        let callback = value.completion.as_ref().unwrap();
        assert_eq!(callback.model.path.as_str(), model);
        assert_eq!(
            (callback.group, callback.behavior),
            (BehaviorGroup::Normal, normal)
        );
        let reopened: Dialogue = rete::decode(&rete::encode(value)?)?;
        assert_eq!(*value, reopened);
        assert!(value.lines.iter().all(|l| l.flags == 0));
    }
    let intro = imported
        .dialogues
        .iter()
        .find(|d| d.box_style == 3)
        .unwrap();
    assert_eq!(
        intro
            .lines
            .iter()
            .map(|l| l.end_seconds)
            .collect::<Vec<_>>(),
        [3.4e38, 1.4, 4.14]
    );
    let ordinary = imported
        .dialogues
        .iter()
        .find(|d| d.box_style == 1)
        .unwrap();
    assert_eq!(ordinary.lines[3].end_seconds, 3.75);
    assert_eq!(ordinary.lines[8].end_seconds, 8.45);
    assert_eq!(ordinary.lines[15].end_seconds, 16.25);
    assert_eq!(ordinary.lines.last().unwrap().end_seconds, 3.4e38);
    assert!(
        ordinary
            .lines
            .iter()
            .any(|l| l.spans.iter().any(|s| s.emphasis))
    );
    Ok(())
}

#[test]
fn native_ui_timing_layers_callbacks_and_adoption() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let project = temp.path().join("godot");
    godot::support::initialize(&project)?;
    fs::write(project.join("check_ui.gd"), include_bytes!("check_ui.gd"))?;
    fs::write(
        project.join("check_ui_brain.gd"),
        include_bytes!("check_ui_brain.gd"),
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let log = project.join("ui-parser.log");
    godot::run_godot(
        &engine,
        &project,
        &[
            "--headless",
            "--log-file",
            log.to_str().unwrap(),
            "--check-only",
            "--script",
            "res://check_ui.gd",
        ],
        Duration::from_secs(10),
    )?;
    let log = project.join("ui-runtime.log");
    let evidence = godot::run_godot(
        &engine,
        &project,
        &[
            "--headless",
            "--log-file",
            log.to_str().unwrap(),
            "--script",
            "res://check_ui.gd",
        ],
        Duration::from_secs(20),
    )?;
    assert!(evidence.contains("UI_CONTRACT_OK"), "{evidence}");
    Ok(())
}

#[test]
#[ignore = "requires local manor data and Godot; serialized native resource export"]
fn manor_dialogue_exports_native_translation_and_voice_resources() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let project = temp.path().join("godot");
    godot::support::initialize(&project)?;
    fs::write(
        project.join("check_ui_resources.gd"),
        include_bytes!("check_ui_resources.gd"),
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let log = project.join("ui-resource-parser.log");
    godot::run_godot(
        &engine,
        &project,
        &[
            "--headless",
            "--log-file",
            log.to_str().unwrap(),
            "--check-only",
            "--script",
            "res://check_ui_resources.gd",
        ],
        Duration::from_secs(10),
    )?;
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("disc"));
    let selection = Selection::hype("manoir", "en")?;
    let mut imported = astrolabe::openspace::content::import_scene(&game, selection.clone())?;
    let roots = imported
        .roots
        .iter()
        .filter(|r| r.kind == Dialogue::KIND)
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(roots.len(), 2);
    godot::export(&mut imported, &roots, selection, &project, &engine)?;
    let log = project.join("ui-resource-runtime.log");
    let evidence = godot::run_godot(
        &engine,
        &project,
        &[
            "--headless",
            "--log-file",
            log.to_str().unwrap(),
            "--script",
            "res://check_ui_resources.gd",
        ],
        Duration::from_secs(20),
    )?;
    assert!(evidence.contains("UI_MANOR_RESOURCES_OK"), "{evidence}");
    Ok(())
}
