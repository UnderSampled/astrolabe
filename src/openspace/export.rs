//! Reconstruct supported source files from useful roots and their dependencies.
use super::codec::FormatCodec;
use crate::files::write_atomic;
use crate::model::{
    AssetPath, AssetRef,
    content::{Asset, ContentSource},
    geometry::{Model, ModelInstance},
    io::{Content, ContentRoot, Diagnostic},
    texture::Texture,
};
use anyhow::{Result, ensure};
use serde::Serialize;
use std::{collections::BTreeMap, path::Path};

const REPORT_PATH: &str = "reconstruction-report.json";
/// Explicit coverage accompanies partial reconstruction; a fragment is not a level.
#[derive(Debug, Serialize)]
pub struct ReconstructionReport {
    pub files: Vec<String>,
    pub coverage_and_gaps: Vec<String>,
    pub source_diagnostics: Vec<Diagnostic>,
}
impl ReconstructionReport {
    pub fn write(&self, output: &Path) -> Result<()> {
        write_atomic(&output.join(REPORT_PATH), &serde_json::to_vec_pretty(self)?)
    }
}

/// Track ownership across all roots and dependencies, after target naming.
/// Shared references emit once; distinct assets must never replace each other.
struct ExportFiles<'a> {
    output: &'a Path,
    owners: BTreeMap<String, AssetPath>,
}
impl ExportFiles<'_> {
    fn contains(&self, owner: &AssetPath, name: &str) -> Result<bool> {
        ensure!(
            name != REPORT_PATH,
            "asset {owner} collides with the reconstruction report"
        );
        if let Some(previous) = self.owners.get(name) {
            ensure!(
                previous == owner,
                "OpenSpace output collision at {name}: {previous} and {owner}"
            );
            return Ok(true);
        }
        Ok(false)
    }
    fn write(&mut self, owner: &AssetPath, name: String, bytes: &[u8]) -> Result<()> {
        if self.contains(owner, &name)? {
            return Ok(());
        }
        write_atomic(&self.output.join(&name), bytes)?;
        self.owners.insert(name, owner.clone());
        Ok(())
    }
}

pub fn export<R: ContentSource>(
    source: &mut R,
    roots: &[ContentRoot],
    output: &Path,
) -> Result<ReconstructionReport> {
    let mut files = ExportFiles {
        output,
        owners: BTreeMap::new(),
    };
    let mut gaps = Vec::new();
    for root in roots {
        match Asset::load(source, root)? {
            Asset::TextureSet(set) => {
                let bytes = super::texture::write_cnt(&set, source)?;
                let name = format!("Gamedata/{}", set.name);
                files.write(&root.reference, name, &bytes)?;
                gaps.push("CNT uses derived unencrypted output; original XOR obfuscation is not reproduced".to_string());
            }
            Asset::Texture(value) => {
                let name = texture_path(root.reference.file());
                files.write(&root.reference, name, &super::texture::encode_gf(&value)?)?;
            }
            Asset::Sound(value) => {
                let name = root.reference.file().to_owned();
                files.write(&root.reference, name, &super::audio::write_apm(&value)?)?;
                gaps.push("APM waveform re-encoding is supported; original lossy ADPCM byte identity is not claimed".into());
            }
            Asset::SoundBank(bank) => match super::audio::write_bnm(&bank) {
                Ok(bytes) => {
                    let name = root.reference.file().to_owned();
                    files.write(&root.reference, name, &bytes)?;
                }
                Err(error) => gaps.push(format!("{}: {error:#}", root.reference)),
            },
            Asset::SoundDefinitions(value) => {
                match (super::codec::SoundScript {
                    name: value.name.clone(),
                })
                .write(&value)
                {
                    Ok(bytes) => {
                        let name = root.reference.file().to_owned();
                        files.write(&root.reference, name, &bytes)?;
                    }
                    Err(error) => gaps.push(format!("{}: {error:#}", root.reference)),
                }
            }
            Asset::Model(model) => {
                let instance = ModelInstance::new(model.path.clone(), &model);
                export_instance(source, &model, &instance, &mut files, &mut gaps)?;
            }
            Asset::ModelInstance(instance) => {
                let model = source.load(&instance.model)?;
                export_instance(source, &model, &instance, &mut files, &mut gaps)?;
            }
            Asset::Material(_) => anyhow::bail!(
                "standalone OpenSpace material reconstruction requires a model fragment"
            ),
        }
    }
    let report = ReconstructionReport {
        files: files.owners.into_keys().collect(),
        coverage_and_gaps: gaps,
        source_diagnostics: Vec::new(),
    };
    report.write(output)?;
    Ok(report)
}

fn export_instance(
    source: &mut impl ContentSource,
    model: &Model,
    instance: &ModelInstance,
    files: &mut ExportFiles<'_>,
    gaps: &mut Vec<String>,
) -> Result<()> {
    instance.validate_model(model)?;
    let mut materials = BTreeMap::new();
    for reference in model
        .submeshes
        .iter()
        .map(|s| &s.material)
        .chain(&model.vertex_materials)
    {
        let material = source.load(reference)?;
        material.validate_reference(&reference.path)?;
        materials.insert(reference.path.clone(), (*material).clone());
    }
    for material in materials.values() {
        if let Some(id) = &material.texture {
            let name = texture_path(id.file());
            if files.contains(id, &name)? {
                continue;
            }
            let texture = source.load(&AssetRef::<Texture>::from_path(id.clone()))?;
            files.write(
                id,
                name,
                &super::codec::Gf {
                    usage: texture.usage,
                }
                .write(&texture)?,
            )?;
        }
    }
    let fragment = super::level::write_model_fragment(
        model,
        instance,
        &materials.into_values().collect::<Vec<_>>(),
        0x10000000,
    )?;
    let framed = super::level::write_sna_block(&fragment.bytes, (6, 2), 0x10000000)?;
    for (extension, bytes) in [
        ("sna", framed),
        ("bin", fragment.bytes),
        ("rtb", fragment.rtb),
        ("ptx", fragment.ptx),
        ("rtt", fragment.rtt),
    ] {
        let name = format!("fragments/{}.{}", instance.path.file(), extension);
        files.write(&instance.path, name, &bytes)?;
    }
    gaps.extend(fragment.coverage);
    gaps.push("Model fragment contains generated records/relocations; it is not a complete loadable SNA level".into());
    Ok(())
}

fn texture_path(file: &str) -> String {
    let name = format!("textures/{file}");
    if file.to_ascii_lowercase().ends_with(".gf") {
        name
    } else {
        format!("{name}.gf")
    }
}
