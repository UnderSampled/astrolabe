//! CLI and conversion orchestration. Target encoding rules stay in their owning adapters.
use crate::openspace::codec::{self, FormatCodec};
use crate::{
    files::write_atomic,
    godot,
    model::{
        AssetPath, AssetRef, Selection,
        audio::{Sound, SoundBank, SoundDefinitions},
        content::TextureSet,
        geometry::{Material, Model},
        io::{Content, ContentRoot, Load, Save},
        texture::Texture,
    },
    openspace, rete,
};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Parser)]
#[command(
    version,
    about = "Repeatable Hype/OpenSpace content conversion (first-model development scope)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Clone, Copy, ValueEnum)]
enum Scope {
    Model,
    Textures,
    Audio,
}
#[derive(Subcommand)]
enum Command {
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
        #[arg(long, value_enum, default_value = "model")]
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
        #[arg(long, value_enum, default_value = "model")]
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
    /// Reconstruct supported source assets; level output is an explicit model fragment.
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
        Scope::Model => openspace::content::import_model(game, selection),
        Scope::Textures => openspace::content::import_textures(game, selection),
        Scope::Audio => openspace::content::import_audio(game, selection, files),
    }
}
pub fn run() -> Result<()> {
    let started = Instant::now();
    match Cli::parse().command {
        Command::Formats => println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"openspace":codec::catalogue(),"rete":{"read":true,"write":true,"schema":1},"godot":{"read":false,"write":true,"version":"4.7.2","scope":"textures, sounds, materials, first model; development outputs"}})
            )?
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
                serde_json::json!({"assets":imported.inventory.len(),"roots":imported.roots.len(),"diagnostics":imported.diagnostics,"source_summary":imported.source_summary,"seconds":started.elapsed().as_secs_f64()})
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
            let manifest = export_godot(&mut imported, &roots, selection, &output, &engine)?;
            println!(
                "{}",
                serde_json::json!({"files":manifest.files.len(),"diagnostics":manifest.diagnostics,"source_diagnostics":imported.diagnostics,"source_summary":imported.source_summary,"seconds":started.elapsed().as_secs_f64()})
            );
        }
        Command::ExportGodot {
            input,
            output,
            godot: engine,
        } => {
            let (mut store, package) = rete::Store::open(input)?;
            let manifest = export_godot(
                &mut store,
                &package.roots,
                package.selection,
                &output,
                &engine,
            )?;
            println!(
                "{}",
                serde_json::json!({"files":manifest.files.len(),"diagnostics":manifest.diagnostics,"source_diagnostics":package.diagnostics,"seconds":started.elapsed().as_secs_f64()})
            );
        }
        Command::ExportOpenspace { input, output } => {
            let (mut store, package) = rete::Store::open(input)?;
            let mut report = export_openspace(&mut store, &package.roots, &output)?;
            report["source_diagnostics"] = serde_json::to_value(&package.diagnostics)?;
            write_atomic(
                &output.join("reconstruction-report.json"),
                &serde_json::to_vec_pretty(&report)?,
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::Inspect { input, asset, kind } => {
            let (mut store, package) = rete::Store::open(input)?;
            if let Some(asset) = asset {
                let kind = kind.context("--asset requires --kind")?;
                let entry = ContentRoot {
                    kind,
                    reference: AssetPath::new(asset)?,
                };
                println!("{}", inspect(&mut store, &entry)?);
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
macro_rules! dispatch {
    ($kind:expr,$function:ident,$($arg:expr),*) => {
        match $kind {
            "texture"=>$function::<Texture>($($arg),*),
            "texture-set"=>$function::<TextureSet>($($arg),*),
            "sound"=>$function::<Sound>($($arg),*),
            "sound-bank"=>$function::<SoundBank>($($arg),*),
            "sound-definitions"=>$function::<SoundDefinitions>($($arg),*),
            "model"=>$function::<Model>($($arg),*),
            "material"=>$function::<Material>($($arg),*),
            x=>anyhow::bail!("unsupported useful content kind {x}"),
        }
    };
}
fn copy<T: Content + Serialize + DeserializeOwned>(
    from: &mut impl Load<T>,
    to: &mut impl Save<T>,
    id: &AssetPath,
) -> Result<()> {
    let reference = AssetRef::new(id.to_string())?;
    let value = from.load(&reference)?;
    to.save(&reference, &value)
}
pub fn save_rete(imported: &mut openspace::content::Imported, output: &Path) -> Result<()> {
    let mut store = rete::Store::new(output, imported.selection.clone())?;
    for entry in imported.inventory.clone() {
        persist(imported, &mut store, &entry)?;
    }
    store.save_package(&rete::Package {
        selection: imported.selection.clone(),
        roots: imported.roots.clone(),
        diagnostics: imported.diagnostics.clone(),
    })
}
fn persist(
    from: &mut openspace::content::Imported,
    to: &mut rete::Store,
    entry: &ContentRoot,
) -> Result<()> {
    dispatch!(entry.kind.as_str(), copy, from, to, &entry.reference)
}
fn inspect(store: &mut rete::Store, entry: &ContentRoot) -> Result<String> {
    fn value<T: Content + Serialize + DeserializeOwned>(
        store: &mut rete::Store,
        id: &AssetPath,
    ) -> Result<String> {
        Ok(serde_json::to_string_pretty(&*store.load(&AssetRef::<
            T,
        >::new(
            id.to_string(),
        )?)?)?)
    }
    dispatch!(entry.kind.as_str(), value, store, &entry.reference)
}

/// Both in-memory source sessions and reloaded Rete use this same composition.
pub fn export_godot<R>(
    source: &mut R,
    roots: &[ContentRoot],
    selection: Selection,
    output: &Path,
    engine: &str,
) -> Result<godot::Manifest>
where
    R: Load<Texture>
        + Load<TextureSet>
        + Load<Sound>
        + Load<SoundBank>
        + Load<SoundDefinitions>
        + Load<Model>
        + Load<Material>,
{
    let mut project = godot::Project::new(output, selection)?;
    let mut written = BTreeSet::<AssetPath>::new();
    fn texture<R: Load<Texture>>(
        project: &mut godot::Project,
        source: &mut R,
        id: &AssetPath,
        written: &mut BTreeSet<AssetPath>,
    ) -> Result<()> {
        if written.insert(id.clone()) {
            let reference = AssetRef::<Texture>::new(id.to_string())?;
            let value = source.load(&reference)?;
            project.save(&reference, &value)?;
        }
        Ok(())
    }
    for root in roots {
        match root.kind.as_str() {
            "model" => {
                let model = source.load(&AssetRef::<Model>::new(root.reference.to_string())?)?;
                model.validate_reference(&root.reference)?;
                for part in &model.submeshes {
                    let material = source.load(&part.material)?;
                    material.validate_reference(&part.material.path)?;
                    if written.insert(material.path.clone()) {
                        if let Some(id) = &material.texture {
                            texture(&mut project, source, id, &mut written)?;
                        }
                        project.write_material(&material)?;
                    }
                }
                project.write_model(&model, source)?;
            }
            "material" => {
                let material =
                    source.load(&AssetRef::<Material>::new(root.reference.to_string())?)?;
                material.validate_reference(&root.reference)?;
                if written.insert(material.path.clone()) {
                    if let Some(id) = &material.texture {
                        texture(&mut project, source, id, &mut written)?;
                    }
                    project.write_material(&material)?;
                }
            }
            "texture" => texture(&mut project, source, &root.reference, &mut written)?,
            "texture-set" => {
                let set = source.load(&AssetRef::<TextureSet>::new(root.reference.to_string())?)?;
                for entry in &set.entries {
                    texture(&mut project, source, &entry.texture.path, &mut written)?;
                }
            }
            "sound" => {
                let reference = AssetRef::<Sound>::new(root.reference.to_string())?;
                let value = source.load(&reference)?;
                project.save(&reference, &value)?;
            }
            "sound-bank" => {
                let bank = source.load(&AssetRef::<SoundBank>::new(root.reference.to_string())?)?;
                for (i, sound) in bank.sounds.iter().enumerate() {
                    let id = sound
                        .resource_id
                        .map_or_else(|| format!("sample_{i}"), |id| format!("resource_{id}"));
                    let reference = AssetRef::<Sound>::new(format!("{}/{}", root.reference, id))?;
                    project.save(&reference, sound)?;
                }
                if !bank.external_sounds.is_empty() {
                    project.diagnostic(format!(
                        "{}: {} external sound references require separately selected APM assets",
                        root.reference,
                        bank.external_sounds.len()
                    ));
                }
                for note in &bank.fidelity_notes {
                    project.diagnostic(format!("{}: {note}", root.reference));
                }
            }
            "sound-definitions" => {
                let value = source.load(&AssetRef::<SoundDefinitions>::new(
                    root.reference.to_string(),
                )?)?;
                project.diagnostic(format!(
                    "{}: {} event definitions retained; gameplay routing is milestone 6",
                    root.reference,
                    value.events.len()
                ));
            }
            x => anyhow::bail!("Godot root kind unsupported: {x}"),
        }
    }
    project.finish(engine)
}

pub fn export_openspace<R>(
    source: &mut R,
    roots: &[ContentRoot],
    output: &Path,
) -> Result<serde_json::Value>
where
    R: Load<Texture>
        + Load<TextureSet>
        + Load<Sound>
        + Load<SoundBank>
        + Load<SoundDefinitions>
        + Load<Model>
        + Load<Material>,
{
    let mut files = Vec::new();
    let mut gaps = Vec::new();
    for root in roots {
        match root.kind.as_str() {
            "texture-set" => {
                let value =
                    source.load(&AssetRef::<TextureSet>::new(root.reference.to_string())?)?;
                let bytes = openspace::content::rebuild_texture_set(&value, source)?;
                let name = format!("Gamedata/{}", value.name);
                write_atomic(&output.join(&name), &bytes)?;
                files.push(name);
                gaps.push("CNT uses derived unencrypted output; original XOR obfuscation is not reproduced".to_string());
            }
            "texture" => {
                let value = source.load(&AssetRef::<Texture>::new(root.reference.to_string())?)?;
                let name = format!("textures/{}.gf", root.reference.file());
                write_atomic(&output.join(&name), &openspace::texture::encode_gf(&value)?)?;
                files.push(name);
            }
            "sound" => {
                let value = source.load(&AssetRef::<Sound>::new(root.reference.to_string())?)?;
                let name = root.reference.file().to_owned();
                write_atomic(&output.join(&name), &openspace::audio::write_apm(&value)?)?;
                files.push(name);
                gaps.push("APM waveform re-encoding is supported; original lossy ADPCM byte identity is not claimed".into());
            }
            "sound-bank" => {
                let value =
                    source.load(&AssetRef::<SoundBank>::new(root.reference.to_string())?)?;
                match openspace::audio::write_bnm(&value) {
                    Ok(bytes) => {
                        let name = root.reference.file().to_owned();
                        write_atomic(&output.join(&name), &bytes)?;
                        files.push(name);
                    }
                    Err(error) => gaps.push(format!("{}: {error:#}", root.reference)),
                }
            }
            "sound-definitions" => {
                let value = source.load(&AssetRef::<SoundDefinitions>::new(
                    root.reference.to_string(),
                )?)?;
                match (codec::SoundScript {
                    name: value.name.clone(),
                })
                .write(&value)
                {
                    Ok(bytes) => {
                        let name = root.reference.file().to_owned();
                        write_atomic(&output.join(&name), &bytes)?;
                        files.push(name);
                    }
                    Err(error) => gaps.push(format!("{}: {error:#}", root.reference)),
                }
            }
            "model" => {
                let value = source.load(&AssetRef::<Model>::new(root.reference.to_string())?)?;
                let mut materials = BTreeMap::new();
                for reference in value
                    .submeshes
                    .iter()
                    .map(|s| &s.material)
                    .chain(&value.vertex_materials)
                {
                    materials.insert(reference.path.clone(), (*source.load(reference)?).clone());
                }
                let mut textures = BTreeSet::new();
                for material in materials.values() {
                    if let Some(id) = &material.texture
                        && textures.insert(id.clone())
                    {
                        let texture = source.load(&AssetRef::<Texture>::new(id.to_string())?)?;
                        let name = format!("textures/{}", id.file());
                        let name = if name.to_ascii_lowercase().ends_with(".gf") {
                            name
                        } else {
                            format!("{name}.gf")
                        };
                        write_atomic(
                            &output.join(&name),
                            &codec::Gf {
                                usage: texture.usage,
                            }
                            .write(&texture)?,
                        )?;
                        files.push(name);
                    }
                }
                let fragment = openspace::level::write_model_fragment(
                    &value,
                    &materials.into_values().collect::<Vec<_>>(),
                    0x10000000,
                )?;
                let framed =
                    openspace::level::write_sna_block(&fragment.bytes, (6, 2), 0x10000000)?;
                for (extension, bytes) in [
                    ("sna", framed),
                    ("bin", fragment.bytes),
                    ("rtb", fragment.rtb),
                    ("ptx", fragment.ptx),
                    ("rtt", fragment.rtt),
                ] {
                    let name = format!("fragments/{}.{}", value.path.file(), extension);
                    write_atomic(&output.join(&name), &bytes)?;
                    files.push(name);
                }
                gaps.extend(fragment.coverage);
                gaps.push("Model fragment contains generated records/relocations; it is not a complete loadable SNA level".into());
            }
            x => anyhow::bail!("OpenSpace root kind unsupported: {x}"),
        }
    }
    let report = serde_json::json!({"files":files,"coverage_and_gaps":gaps});
    write_atomic(
        &output.join("reconstruction-report.json"),
        &serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}
