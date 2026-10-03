//! Maintained presentation contracts and the evidenced manor localization slice.
use anyhow::Result;
use astrolabe::{
    godot,
    model::{
        Selection,
        ai::BehaviorGroup,
        dialogue::{Dialogue, DialoguePresentation},
        io::Content,
    },
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
    assert_eq!(imported.dialogues.len(), 9);
    let style = imported.dialogue_presentation.as_ref().unwrap();
    style.validate()?;
    assert_eq!(style.font_name, "Playmo_Font2_0");
    assert_eq!(style.glyphs.len(), 133);
    assert_eq!(style.atlas.size, [256, 256]);
    assert!(!style.atlas.color_key);
    assert_eq!(style.normal_color, [1.0; 4]);
    assert_eq!(style.emphasis_color, [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(style.tracking, 0.01);
    let a = style.glyphs.iter().find(|g| g.character == 'A').unwrap();
    assert_eq!(
        (a.width, a.half_height, a.vertical_offset),
        (0.22588237, -0.12705882, -0.01)
    );
    let space = style.glyphs.iter().find(|g| g.character == ' ').unwrap();
    assert_eq!(
        (space.width, space.half_height, space.uv),
        (0.1, -0.15, None)
    );
    let reopened: DialoguePresentation = rete::decode(&rete::encode(style)?)?;
    assert_eq!(*style, reopened);
    for (t, name) in style.frame.iter().zip(["coin", "top", "middle", "side"]) {
        assert_eq!(
            t.texture.as_str(),
            format!("texture:/fixtex/stencil/{name}.tga")
        );
        assert_eq!(t.size, [32, 32]);
    }
    println!(
        "MANOR_UI_SOURCE glyphs={} atlas={} frame={:?}",
        style.glyphs.len(),
        style.atlas.texture,
        style.frame
    );

    let library = &imported
        .sound_definitions
        .iter()
        .find(|(path, _)| path.as_str() == "sound:/Sound/playmo_b.lcb")
        .unwrap()
        .1;
    for (name, box_style, count, member, event, model, normal) in [
        (
            "gogood_texte1_manoir",
            3,
            3,
            0,
            25,
            "level:/ai/models/210",
            4,
        ),
        (
            "gogood_texte1a_manoir",
            3,
            3,
            1,
            2,
            "level:/ai/models/210",
            5,
        ),
        (
            "gogood_texte1b_manoir",
            3,
            4,
            2,
            3,
            "level:/ai/models/210",
            6,
        ),
        (
            "gogood_texte1c_manoir",
            3,
            4,
            3,
            4,
            "level:/ai/models/210",
            1,
        ),
        (
            "gogood_texte2_manoir",
            3,
            4,
            4,
            5,
            "level:/ai/models/210",
            11,
        ),
        (
            "gogood_texte2a_manoir",
            3,
            9,
            5,
            6,
            "level:/ai/models/210",
            12,
        ),
        ("hype_texte1_manoir", 3, 2, 6, 26, "level:/ai/models/294", 5),
        ("hype_texte2_manoir", 3, 8, 7, 27, "level:/ai/models/294", 3),
        (
            "gogood_master_whatif01",
            1,
            21,
            12,
            20,
            "level:/ai/models/991",
            17,
        ),
    ] {
        let value = imported.dialogues.iter().find(|d| d.name == name).unwrap();
        value.validate()?;
        assert!(
            value
                .lines
                .iter()
                .flat_map(|l| &l.spans)
                .flat_map(|s| s.text.chars())
                .all(|c| style.glyphs.iter().any(|g| g.character == c)),
            "selected source text has a missing native glyph"
        );
        assert_eq!(value.presentation.as_ref().unwrap().path, style.path);
        assert_eq!((value.box_style, value.lines.len()), (box_style, count));
        let voice = value.voice.as_ref().unwrap();
        assert_eq!((voice.bank, voice.member), (142, member));
        assert_eq!(voice.binding(library)?.target.member().id, event);
        let callback = value.completion.as_ref().unwrap();
        assert_eq!(callback.model.path.as_str(), model);
        assert_eq!(
            (callback.group, callback.behavior),
            (BehaviorGroup::Normal, normal)
        );
        let reopened: Dialogue = rete::decode(&rete::encode(value)?)?;
        assert_eq!(*value, reopened);
        assert!(value.lines.iter().all(|l| l.flags == 0));
        println!(
            "MANOR_DIALOGUE_SOURCE {}",
            serde_json::json!({
                "name":value.name, "box":value.box_style,
                "lines":value.lines.len(), "owner":value.owner.path,
                "voice":value.voice, "completion":value.completion,
                "resolved_voice":voice.binding(library)?.target.member(),
                "deadlines":value.lines.iter().map(|l|l.end_seconds).collect::<Vec<_>>()
            })
        );
    }
    let intro = imported
        .dialogues
        .iter()
        .find(|d| d.name == "gogood_texte1_manoir")
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
    println!("{evidence}");
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
    let log = project.join("ui-font-api.log");
    let api = godot::run_godot(
        &engine,
        &project,
        &[
            "--headless",
            "--log-file",
            log.to_str().unwrap(),
            "--script",
            "res://check_ui_resources.gd",
            "--",
            "--font-api-only",
        ],
        Duration::from_secs(5),
    )?;
    assert!(api.contains("UI_FONT_API_OK"), "{api}");
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
    assert_eq!(roots.len(), 9);
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
    println!("{evidence}");
    Ok(())
}
