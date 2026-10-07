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
        #[arg(long, value_enum)]
        profile: SaveProfile,
        #[arg(long, default_value = "godot/game/original_save_import.json")]
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
                    "scope": "textures, sounds, materials, models, scenes, collision, approximate lights, sector inspection and sampled characters; development outputs"
                }
            }))?
        ),
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
            output,
            profile,
        } => {
            let report =
                godot::save_import::import_original_slot(&input, &output, profile.as_str())?;
            println!(
                "{}",
                serde_json::json!({
                    "output": output,
                    "slot": report["slot"],
                    "current_level": report["current_level"],
                    "godot_state_available": !report["godot_state"].is_null(),
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
