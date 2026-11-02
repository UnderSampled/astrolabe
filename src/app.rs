//! CLI and conversion orchestration. Target encoding rules stay in their owning adapters.
use crate::openspace::codec;
use crate::{
    files::write_atomic,
    godot,
    model::{AssetPath, Selection, io::ContentRoot},
    openspace, rete,
};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Parser)]
#[command(
    version,
    about = "Repeatable Hype/OpenSpace media, scene and sampled-character conversion"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Clone, Copy, ValueEnum)]
enum Scope {
    Scene,
    Menu,
    PauseMenu,
    Inventory,
    Hud,
    Model,
    Textures,
    Audio,
}

#[derive(Clone, Copy, ValueEnum)]
enum SaveProfile {
    HypePcEnglishSuppliedSave,
}

impl SaveProfile {
    fn as_str(self) -> &'static str {
        match self {
            Self::HypePcEnglishSuppliedSave => godot::save_import::HYPE_PC_ENGLISH_PROFILE,
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// Adopt the source-free Godot starter and compatible support package once.
    InitGodot { output: PathBuf },
    /// Update unchanged installed support without replacing adopted settings/content.
    UpdateGodotSupport { output: PathBuf },
    /// Report supported target operations and important coverage restrictions.
    Formats,
    /// Prepare the manor game, main menu, inventory and pause menu in one command.
    PrepareGame {
        game: PathBuf,
        output: PathBuf,
        #[arg(long, default_value = "manoir")]
        level: String,
        #[arg(long, default_value = "en")]
        locale: String,
        #[arg(long, default_value = "godot")]
        godot: String,
    },
    /// Import useful content into per-asset MessagePack Rete files.
    ImportOpenspace {
        game: PathBuf,
        output: PathBuf,
        #[arg(long, default_value = "astrolabe")]
        level: String,
        #[arg(long, default_value = "en")]
        locale: String,
        #[arg(long, value_enum, default_value = "scene")]
        content: Scope,
        #[arg(long)]
        audio_file: Vec<PathBuf>,
    },
    /// Port directly from source packages to a native Godot development project.
    Port {
        game: PathBuf,
        output: PathBuf,
        #[arg(long, default_value = "astrolabe")]
        level: String,
        #[arg(long, default_value = "en")]
        locale: String,
        #[arg(long, value_enum, default_value = "scene")]
        content: Scope,
        #[arg(long)]
        audio_file: Vec<PathBuf>,
        #[arg(long, default_value = "godot")]
        godot: String,
    },
    ExportGodot {
        input: PathBuf,
        output: PathBuf,
        #[arg(long, default_value = "godot")]
        godot: String,
    },
    /// Reconstruct supported source assets; level output consists of supported asset fragments.
    ExportOpenspace { input: PathBuf, output: PathBuf },
    /// Inspect package roots, useful assets, or rooted source-level summaries.
    Inspect {
        input: PathBuf,
        #[arg(long)]
        asset: Option<String>,
        #[arg(long)]
        kind: Option<String>,
    },
    InspectLevel {
        game: PathBuf,
        #[arg(long, default_value = "astrolabe")]
        level: String,
    },
    /// Import an extracted original Hype SaveGameN slot for the Godot Load Game route.
    ImportSave {
        input: PathBuf,
        /// Installed/extracted Hype source root used to resolve saved fields by source composition.
        #[arg(long)]
        game: PathBuf,
        #[arg(long, value_enum)]
        profile: SaveProfile,
        /// Directory receiving SaveGameN.cfg.
        #[arg(long, default_value = "godot/game/saves")]
        output: PathBuf,
    },
    Lzo {
        #[command(subcommand)]
        operation: LzoCommand,
    },
}
#[derive(Subcommand)]
enum LzoCommand {
    Encode { input: PathBuf, output: PathBuf },
    Decode { input: PathBuf, output: PathBuf },
}
fn import(
    game: &Path,
    selection: Selection,
    scope: Scope,
    files: &[PathBuf],
) -> Result<openspace::content::Imported> {
    match scope {
        Scope::Scene => openspace::content::import_scene(game, selection),
        Scope::Menu => openspace::content::import_menu(game, selection),
        Scope::PauseMenu => openspace::content::import_pause_menu(game, selection),
        Scope::Inventory => openspace::content::import_inventory(game, selection),
        Scope::Hud => openspace::content::import_hud(game, selection),
        Scope::Model => openspace::content::import_model(game, selection),
        Scope::Textures => openspace::content::import_textures(game, selection),
        Scope::Audio => openspace::content::import_audio(game, selection, files),
    }
}

/// Application preparation composes ordinary scope conversions in isolation.
/// Publish their distinct owned closures together after every scope succeeds.
fn prepare_game(game: &Path, output: &Path, selection: Selection, engine: &str) -> Result<()> {
    anyhow::ensure!(
        selection.level == "manoir" && selection.locale == "en",
        "prepare-game currently prepares the authored manor/English application; use port for other level or locale selections"
    );
    let started = Instant::now();
    let prepared = godot::prepare_directory(output)?;
    let scopes = [
        ("scene", Scope::Scene),
        ("menu", Scope::Menu),
        ("inventory", Scope::Inventory),
        ("pause-menu", Scope::PauseMenu),
    ];
    let mut completed = Vec::new();
    for (index, (name, scope)) in scopes.iter().enumerate() {
        let scope_started = Instant::now();
        eprintln!("Preparing {name} ({}/{})", index + 1, scopes.len());
        let mut phase = "import";
        let result = (|| -> Result<serde_json::Value> {
            let mut imported = import(game, selection.clone(), *scope, &[])?;
            let roots = imported.roots.clone();
            let owner = imported.selection.clone();
            phase = "staged-export";
            let manifest = godot::export(
                &mut imported,
                &roots,
                owner.clone(),
                prepared.path(),
                engine,
            )?;
            Ok(serde_json::json!({
                "scope": name,
                "owner": owner.level,
                "status": "complete",
                "files": manifest.selections.get(&owner.level).map_or(0, |files| files.len()),
                "diagnostics": manifest.diagnostics,
                "source_diagnostics": imported.diagnostics,
                "seconds": scope_started.elapsed().as_secs_f64()
            }))
        })();
        match result {
            Ok(report) => completed.push(report),
            Err(error) => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "operation": "prepare-game",
                        "status": "incomplete",
                        "game": game,
                        "output": output,
                        "completed": completed,
                        "failed": { "scope": name, "phase": phase, "error": format!("{error:#}") },
                        "remaining": scopes[index + 1..].iter().map(|(name, _)| name).collect::<Vec<_>>(),
                        "recovery": "Resolve the reported failure and repeat prepare-game. The consumer generated content is unchanged; completed scopes were prepared only in an isolated temporary project.",
                        "seconds": started.elapsed().as_secs_f64()
                    }))?
                );
                return Err(error)
                    .with_context(|| format!("prepare-game stopped at {name} ({phase})"));
            }
        }
    }
    godot::publish_prepared(prepared.path(), output, engine)
        .context("publish completed application preparation")?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "operation": "prepare-game",
            "status": "complete",
            "game": game,
            "output": output,
            "completed": completed,
            "remaining": [],
            "entry_scene": "res://game/bootstrap.tscn",
            "scope": "Prepared supported manor and UI assets; full-game behavior and presentation remain incomplete.",
            "seconds": started.elapsed().as_secs_f64()
        }))?
    );
    Ok(())
}
pub fn run() -> Result<()> {
    let started = Instant::now();
    match Cli::parse().command {
        Command::InitGodot { output } => godot::support::initialize(&output)?,
        Command::UpdateGodotSupport { output } => godot::support::update(&output)?,
        Command::Formats => println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "openspace": codec::catalogue(),
                "rete": { "read": true, "write": true, "schema": rete::SCHEMA },
                "godot": {
                    "read": false,
                    "write": true,
                    "version": "4.7.2",
                    "scope": "native textures, sounds, materials, models, scenes, collision, approximate lights, sampled characters, compiled supported AI, main/pause menus and inventory; partial game conversion",
                    "prepare_game": ["scene", "menu", "inventory", "pause-menu"],
                    "limits": ["authored application selects manor/en", "opening, camera, savegame and UI fidelity remain incomplete", "OpenSpace reconstruction is not complete package assembly"]
                }
            }))?
        ),
        Command::PrepareGame {
            game,
            output,
            level,
            locale,
            godot: engine,
        } => prepare_game(&game, &output, Selection::hype(level, locale)?, &engine)?,
        Command::ImportOpenspace {
            game,
            output,
            level,
            locale,
            content,
            audio_file,
        } => {
            let mut imported =
                import(&game, Selection::hype(level, locale)?, content, &audio_file)?;
            save_rete(&mut imported, &output)?;
            println!(
                "{}",
                serde_json::json!({
                    "assets": imported.inventory.len(),
                    "roots": imported.roots.len(),
                    "diagnostics": imported.diagnostics,
                    "source_summary": imported.source_summary,
                    "seconds": started.elapsed().as_secs_f64()
                })
            );
        }
        Command::Port {
            game,
            output,
            level,
            locale,
            content,
            audio_file,
            godot: engine,
        } => {
            let mut imported =
                import(&game, Selection::hype(level, locale)?, content, &audio_file)?;
            let roots = imported.roots.clone();
            let selection = imported.selection.clone();
            let manifest = godot::export(&mut imported, &roots, selection, &output, &engine)?;
            println!(
                "{}",
                serde_json::json!({
                    "files": manifest.files.len(),
                    "diagnostics": manifest.diagnostics,
                    "source_diagnostics": imported.diagnostics,
                    "source_summary": imported.source_summary,
                    "seconds": started.elapsed().as_secs_f64()
                })
            );
        }
        Command::ExportGodot {
            input,
            output,
            godot: engine,
        } => {
            let (mut store, package) = rete::Store::open(input)?;
            let manifest = godot::export(
                &mut store,
                &package.roots,
                package.selection,
                &output,
                &engine,
            )?;
            println!(
                "{}",
                serde_json::json!({
                    "files": manifest.files.len(),
                    "diagnostics": manifest.diagnostics,
                    "source_diagnostics": package.diagnostics,
                    "seconds": started.elapsed().as_secs_f64()
                })
            );
        }
        Command::ExportOpenspace { input, output } => {
            let (mut store, package) = rete::Store::open(input)?;
            let mut report = openspace::export(&mut store, &package.roots, &output)?;
            report.source_diagnostics = package.diagnostics;
            report.write(&output)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::ImportSave {
            input,
            game,
            output,
            profile,
        } => {
            let report =
                godot::save_import::import_original_slot(&input, &game, &output, profile.as_str())?;
            println!(
                "{}",
                serde_json::json!({
                    "output": output,
                    "slot": report["slot"],
                    "current_level": report["current_level"],
                    "godot_state_available": !report["godot_state"].is_null(),
                    "coverage": report["coverage"],
                    "restored_fields": report["restored_fields"],
                    "not_restored": report["not_restored"],
                })
            );
        }
        Command::Inspect { input, asset, kind } => {
            let (mut store, package) = rete::Store::open(input)?;
            if let Some(asset) = asset {
                let kind = kind.context("--asset requires --kind")?;
                let entry = ContentRoot {
                    kind,
                    reference: AssetPath::new(asset)?,
                };
                println!("{}", store.inspect(&entry)?);
            } else {
                println!("{}", serde_json::to_string_pretty(&package)?);
            }
        }
        Command::InspectLevel { game, level } => println!(
            "{}",
            serde_json::to_string_pretty(&openspace::level::inspect_level(
                &game,
                &Selection::hype(level, "en")?
            )?)?
        ),
        Command::Lzo { operation } => match operation {
            LzoCommand::Encode { input, output } => write_atomic(
                &output,
                &openspace::compression::encode_envelope(&fs::read(input)?)?,
            )?,
            LzoCommand::Decode { input, output } => {
                let bytes = fs::read(input)?;
                let (value, used) = openspace::compression::decode_envelope(
                    &bytes,
                    openspace::compression::DEFAULT_MAX_DECODED,
                )?;
                anyhow::ensure!(used == bytes.len(), "trailing data after envelope");
                write_atomic(&output, &value)?;
            }
        },
    }
    Ok(())
}
/// Compose the imported dependency closure with Rete persistence.
pub fn save_rete(imported: &mut openspace::content::Imported, output: &Path) -> Result<()> {
    let inventory = imported.inventory.clone();
    let package = rete::Package {
        selection: imported.selection.clone(),
        roots: imported.roots.clone(),
        diagnostics: imported.diagnostics.clone(),
    };
    rete::save_package(imported, &inventory, &package, output)
}
