//! Isolated style-4 presentation and the real source cue's retained dependencies.
use anyhow::{Context, Result, ensure};
use astrolabe::{
    godot,
    model::{
        AssetPath, Selection,
        ai::{
            ActorKeyword, ActorTarget, AiModel, BehaviorGroup, Comparison, Expression, Statement,
        },
        dialogue::DialoguePresentation,
        io::{Content, ContentRoot},
    },
    openspace::level::import_scene,
    rete,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

fn stage_component(project: &Path) -> Result<()> {
    godot::support::initialize(project)?;
    // The lead owns package installation/versioning. Until integration, stage
    // exactly these two new maintained files in this owned component fixture.
    for (path, bytes) in [
        (
            "addons/astrolabe/ui/location_title.gd",
            include_bytes!("../godot/addons/astrolabe/ui/location_title.gd").as_slice(),
        ),
        (
            "addons/astrolabe/ui/location_title.tscn",
            include_bytes!("../godot/addons/astrolabe/ui/location_title.tscn").as_slice(),
        ),
        (
            "check_location_title.gd",
            include_bytes!("check_location_title.gd").as_slice(),
        ),
    ] {
        fs::write(project.join(path), bytes)?;
    }
    Ok(())
}

#[test]
fn native_location_title_resources_layout_animation_and_cancellation() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let project = temp.path().join("godot");
    stage_component(&project)?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    for (mode, budget) in [("--check-only", 10), ("--runtime", 20)] {
        let log = project.join(format!("location-title-{mode}.log"));
        let mut args = vec!["--headless", "--log-file", log.to_str().unwrap()];
        if mode == "--check-only" {
            args.push(mode);
        }
        args.extend(["--script", "res://check_location_title.gd"]);
        let evidence = godot::run_godot(&engine, &project, &args, Duration::from_secs(budget))?;
        let diagnostics = fs::read_to_string(log)?;
        ensure!(
            !diagnostics.contains("SCRIPT ERROR:") && !diagnostics.contains("ERROR:"),
            "native title diagnostics: {diagnostics}"
        );
        if mode != "--check-only" {
            ensure!(
                evidence.contains("LOCATION_TITLE_COMPONENT_OK"),
                "{evidence}"
            );
            println!("{evidence}");
        }
    }
    Ok(())
}

#[test]
#[ignore = "requires local English source and Godot; exports only shared font/stencils"]
fn native_location_title_uses_converted_source_font_and_stencils() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let project = temp.path().join("godot");
    stage_component(&project)?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let log = project.join("source-title-parser.log");
    godot::run_godot(
        &engine,
        &project,
        &[
            "--headless",
            "--log-file",
            log.to_str().unwrap(),
            "--check-only",
            "--script",
            "res://check_location_title.gd",
        ],
        Duration::from_secs(10),
    )?;
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("disc"));
    let selection = Selection::hype("manoir", "en")?;
    let mut imported = astrolabe::openspace::content::import_scene(&game, selection.clone())?;
    let roots = [ContentRoot {
        kind: DialoguePresentation::KIND.into(),
        reference: AssetPath::new("level:/dialogue/presentation/en")?,
    }];
    let manifest = godot::export(&mut imported, &roots, selection, &project, &engine)?;
    // This converter invocation has one resource root, no scene/voice/AI export.
    let resource = manifest
        .files
        .keys()
        .find(|p| p.ends_with("/dialogue/presentation/en.tres"))
        .context("native source presentation missing")?;
    let presentation_arg = format!("--presentation=res://{resource}");
    let log = project.join("source-title-runtime.log");
    let capture_arg = std::env::var_os("ASTROLABE_LOCATION_TITLE_CAPTURE")
        .map(|p| format!("--capture={}", PathBuf::from(p).display()));
    if capture_arg.is_some() {
        ensure!(
            !godot::gameplay_headless()?,
            "title capture requires rendered private-display run"
        );
        ensure!(
            std::env::var_os("DISPLAY").is_some() && std::env::var_os("XAUTHORITY").is_some(),
            "rendered title check requires an explicitly owned display"
        );
    }
    let mut args = vec![
        "--headless",
        "--rendering-method",
        "gl_compatibility",
        "--log-file",
        log.to_str().unwrap(),
        "--script",
        "res://check_location_title.gd",
        "--",
        &presentation_arg,
    ];
    if let Some(ref capture) = capture_arg {
        args.push(capture);
    }
    let evidence = godot::run_godot_gameplay(&engine, &project, &args, Duration::from_secs(20))?;
    ensure!(
        evidence.contains("LOCATION_TITLE_COMPONENT_OK"),
        "{evidence}"
    );
    println!("LOCATION_TITLE_CONVERTED_PRESENTATION_OK {evidence}");
    Ok(())
}

#[test]
#[ignore = "requires local Astrolabe source; static import/Rete, no game runs"]
fn astrolabe_title_cue_preserves_source_order_and_reports_query_gap() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("disc"));
    let imported = import_scene(&game, &Selection::hype("astrolabe", "en")?)?;
    let owner = imported
        .persos
        .iter()
        .find(|p| p.name == "iDummy_Astrolabe_Texte")
        .context("source title dummy absent")?;
    let brain = owner.brain.as_ref().context("source title Brain absent")?;
    assert_eq!(owner.path.as_str(), "level:/persos/world/0/91");
    assert_eq!(brain.model.path.as_str(), "level:/ai/models/659");
    let normal = brain
        .normal
        .as_ref()
        .context("source normal selection absent")?;
    assert_eq!(normal.current, Some(0));
    assert_eq!(normal.previous, Some(0));
    assert_eq!(normal.initial, Some(0));
    println!(
        "LOCATION_TITLE_SOURCE_OWNER {} normal={:?}",
        owner.path, brain.normal
    );
    let model = imported
        .ai_models
        .iter()
        .find(|m| m.path == brain.model.path)
        .context("source title model absent")?;
    let reopened: AiModel = rete::decode(&rete::encode(model)?)?;
    assert_eq!(&reopened, model);
    let behaviors = &reopened.normal.as_ref().unwrap().behaviors;
    let cue = &behaviors[0].schedule.as_ref().unwrap().statements;
    assert_eq!(cue.len(), 4);
    println!("LOCATION_TITLE_SOURCE_CUE {cue:#?}");
    assert_eq!(
        cue[0],
        Statement::Wait {
            milliseconds: Expression::Integer(200)
        }
    );
    assert_eq!(
        cue[1],
        Statement::PositionDialog {
            x: Expression::Integer(160),
            y: Expression::Integer(70)
        }
    );
    let Statement::If {
        condition,
        then,
        otherwise: None,
    } = &cue[2]
    else {
        anyhow::bail!("source title predicate changed: {:?}", cue[2]);
    };
    let Expression::Compare {
        operation: Comparison::Equal,
        left,
        right,
    } = condition
    else {
        anyhow::bail!("source title comparison changed: {condition:?}");
    };
    assert_eq!(**right, Expression::Integer(2));
    let Expression::Unsupported(query) = left.as_ref() else {
        anyhow::bail!(
            "update this pending-query checkpoint after typed Func195 integration: {left:?}"
        );
    };
    assert!(query.reason.contains("195"), "{query:?}");
    assert_eq!(
        then,
        &[Statement::RunDialog {
            name: "astrolabe_txt2".into(),
            mode: Expression::Integer(1)
        }]
    );
    let Statement::ChangeBehavior {
        target,
        pause_after: false,
    } = &cue[3]
    else {
        anyhow::bail!("source title behavior change changed: {:?}", cue[3]);
    };
    assert_eq!(target.model.path, brain.model.path);
    assert_eq!(target.group, BehaviorGroup::Normal);
    assert_eq!(target.behavior, 1);
    let end = &behaviors[1].schedule.as_ref().unwrap().statements;
    assert_eq!(end.len(), 3);
    assert_eq!(
        end[0],
        Statement::Wait {
            milliseconds: Expression::Integer(3000)
        }
    );
    assert_eq!(end[1], Statement::EndDialog);
    assert_eq!(
        end[2],
        Statement::SetObjectActive {
            actor: ActorTarget::Keyword(ActorKeyword::SelfActor),
            active: false
        }
    );
    // Astrolabe language/catalogue import is currently Manor-only. Do not turn
    // a presentation component pass into a claim of a converted source cue.
    assert!(imported.dialogues.is_empty());
    println!(
        "LOCATION_TITLE_SOURCE_PENDING Func195 + Astrolabe language/style4 admission + shared screen dispatch"
    );
    Ok(())
}
