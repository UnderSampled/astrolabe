//! Registered style-4 presentation, catalogue and unchanged compiled source cue.
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
    fs::write(
        project.join("check_location_title.gd"),
        include_bytes!("check_location_title.gd"),
    )?;
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
fn astrolabe_title_cue_preserves_source_order_and_catalogue() -> Result<()> {
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
    assert_eq!(**left, Expression::GameState);
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
    for diagnostic in &imported.diagnostics {
        if diagnostic
            .asset
            .as_ref()
            .is_some_and(|a| a.as_str().contains("astrolabe_txt"))
        {
            println!("LOCATION_TITLE_CATALOGUE_DIAGNOSTIC {diagnostic:?}");
        }
    }
    let title = imported
        .dialogues
        .iter()
        .find(|d| d.name == "astrolabe_txt2")
        .context("actual title catalogue missing")?;
    assert_eq!(title.owner.path, owner.path);
    assert_eq!(title.box_style, 4);
    assert_eq!(title.lines.len(), 2);
    assert!(title.voice.is_none());
    let completion = title
        .completion
        .as_ref()
        .context("source title completion absent")?;
    assert_eq!(completion.model.path, brain.model.path);
    assert_eq!(completion.group, BehaviorGroup::Normal);
    println!("LOCATION_TITLE_SOURCE_COMPLETION {completion:?}");
    title.validate()?;
    println!(
        "LOCATION_TITLE_SOURCE_LAYOUT {:?}",
        title
            .lines
            .iter()
            .map(|line| &line.layout)
            .collect::<Vec<_>>()
    );
    let reopened: astrolabe::model::dialogue::Dialogue = rete::decode(&rete::encode(title)?)?;
    assert_eq!(&reopened, title);
    println!(
        "LOCATION_TITLE_SOURCE_TYPED_CATALOGUE_OK {} owner={} lines={} style4",
        title.path,
        title.owner.path,
        title.lines.len()
    );
    Ok(())
}

fn stage_cue(project: &Path, engine: &str) -> Result<()> {
    stage_component(project)?;
    fs::write(
        project.join("check_location_title_cue.gd"),
        include_bytes!("check_location_title_cue.gd"),
    )?;
    let log = project.join("cue-preflight.log");
    godot::run_godot(
        engine,
        project,
        &[
            "--headless",
            "--log-file",
            log.to_str().unwrap(),
            "--check-only",
            "--script",
            "res://check_location_title_cue.gd",
        ],
        Duration::from_secs(10),
    )?;
    Ok(())
}

fn run_cue(project: &Path, engine: &str, arguments: &[String]) -> Result<()> {
    let log = project.join("cue-runtime.log");
    let mut args = vec![
        "--headless",
        "--log-file",
        log.to_str().unwrap(),
        "--script",
        "res://check_location_title_cue.gd",
        "--",
    ];
    args.extend(arguments.iter().map(String::as_str));
    let output = godot::run_godot(engine, project, &args, Duration::from_secs(20))?;
    ensure!(output.contains("LOCATION_TITLE_CUE_OK"), "{output}");
    println!("{output}");
    Ok(())
}

#[test]
fn compiled_location_title_cue_drives_shared_screen_lifecycle() -> Result<()> {
    use astrolabe::model::{
        AssetRef,
        ai::{Behavior, BehaviorRef, BehaviorSet, Script},
    };
    let temp = tempfile::tempdir()?;
    let project = temp.path().join("godot");
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    stage_cue(&project, &engine)?;
    let path = AssetPath::new("level:/ai/models/title_fixture")?;
    let model = AiModel {
        path: path.clone(),
        name: "Synthetic title cue".into(),
        normal: Some(BehaviorSet {
            default_behavior: Some(0),
            action_capacity: 0,
            unknown: [0; 3],
            behaviors: vec![
                Behavior {
                    id: 0,
                    rules: vec![],
                    unknown: [0; 3],
                    schedule: Some(Script {
                        statements: vec![
                            Statement::Wait {
                                milliseconds: Expression::Integer(200),
                            },
                            Statement::PositionDialog {
                                x: Expression::Integer(160),
                                y: Expression::Integer(70),
                            },
                            Statement::If {
                                condition: Expression::Compare {
                                    operation: Comparison::Equal,
                                    left: Box::new(Expression::GameState),
                                    right: Box::new(Expression::Integer(2)),
                                },
                                then: vec![Statement::RunDialog {
                                    name: "synthetic_title".into(),
                                    mode: Expression::Integer(1),
                                }],
                                otherwise: None,
                            },
                            Statement::ChangeBehavior {
                                target: BehaviorRef {
                                    model: AssetRef::from_path(path),
                                    group: BehaviorGroup::Normal,
                                    behavior: 1,
                                },
                                pause_after: false,
                            },
                        ],
                    }),
                },
                Behavior {
                    id: 1,
                    rules: vec![],
                    unknown: [0; 3],
                    schedule: Some(Script {
                        statements: vec![
                            Statement::Wait {
                                milliseconds: Expression::Integer(3000),
                            },
                            Statement::EndDialog,
                            Statement::SetObjectActive {
                                actor: ActorTarget::Keyword(ActorKeyword::SelfActor),
                                active: false,
                            },
                        ],
                    }),
                },
            ],
        }),
        reflex: None,
        variables: vec![],
        designer_unknown: [0; 3],
        unknown: [0; 4],
    };
    let reopened: AiModel = rete::decode(&rete::encode(&model)?)?;
    assert_eq!(reopened, model);
    let script = godot::ai::compile(&reopened, &[])?;
    ensure!(
        script.contains("unrepresented stale result") && script.contains("TYPE_INT"),
        "missing unsupported combined-mask guard"
    );
    fs::write(project.join("cue.gd"), script)?;
    run_cue(&project, &engine, &[])
}

#[test]
#[ignore = "requires local Astrolabe English catalogue and Godot; exports only title/AI/font/stencils"]
fn unchanged_astrolabe_title_cue_executes_exported_native_catalogue() -> Result<()> {
    use astrolabe::model::{AssetRef, dialogue::Dialogue, io::Load};
    let temp = tempfile::tempdir()?;
    let project = temp.path().join("godot");
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    stage_cue(&project, &engine)?;
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("disc"));
    let selection = Selection::hype("astrolabe", "en")?;
    let mut imported = astrolabe::openspace::content::import_scene(&game, selection.clone())?;
    let model_ref = AssetRef::<AiModel>::new("level:/ai/models/659")?;
    let title_ref = AssetRef::<Dialogue>::new("level:/dialogue/en/astrolabe_txt2")?;
    // Feed the export with reopened useful Rete values, keeping source addresses
    // and relocations solely inside the importer. This selects no level scene.
    let model = imported.load(&model_ref)?;
    imported
        .assets
        .insert(&model_ref, rete::decode(&rete::encode(model.as_ref())?)?)?;
    let title = imported.load(&title_ref)?;
    imported
        .assets
        .insert(&title_ref, rete::decode(&rete::encode(title.as_ref())?)?)?;
    let roots = [
        ContentRoot {
            kind: Dialogue::KIND.into(),
            reference: title_ref.path,
        },
        ContentRoot {
            kind: AiModel::KIND.into(),
            reference: model_ref.path,
        },
    ];
    let manifest = godot::export(&mut imported, &roots, selection, &project, &engine)?;
    ensure!(
        !manifest.files.keys().any(|p| p.ends_with("/level.tscn")),
        "unexpected full-level export"
    );
    let title = manifest
        .files
        .keys()
        .find(|p| p.ends_with("/dialogue/en/astrolabe_txt2.tres"))
        .context("native title resource missing")?;
    let program = manifest
        .files
        .keys()
        .find(|p| p.ends_with("/ai/models/659.gd"))
        .context("compiled source title program missing")?;
    run_cue(
        &project,
        &engine,
        &[
            format!("--item=res://{title}"),
            format!("--program=res://{program}"),
        ],
    )?;
    println!(
        "LOCATION_TITLE_UNCHANGED_SOURCE_NATIVE_OK actor91/model659 catalogue + Rete + public native export"
    );
    Ok(())
}
