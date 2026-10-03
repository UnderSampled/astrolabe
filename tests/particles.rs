//! Original Hype selection excerpt over reloaded useful assets and native resources.
use anyhow::{Context, Result, ensure};
use astrolabe::{
    godot,
    model::{
        AssetRef, Selection,
        ai::{AiModel, Statement},
        io::{Content, ContentRoot, Load, Save},
        particle::ParticleGenerator,
    },
    openspace, rete,
};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, time::Duration};

#[test]
#[ignore = "requires local game data and Godot; original selection/disable excerpt, active emission pending"]
fn imported_hype_particle_selection_survives_rete_and_uses_native_emitter() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let temporary = tempfile::tempdir()?;
    let evidence = std::env::var_os("ASTROLABE_EVIDENCE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| temporary.path().join("evidence"))
        .join("particle-selection");
    ensure!(!evidence.exists(), "particle evidence already exists");
    fs::create_dir_all(&evidence)?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    // Parse the exact harness and maintained preload closure before source
    // import, Rete or native resource conversion. Never use the live project.
    let syntax = temporary.path().join("syntax");
    godot::support::initialize(&syntax)?;
    fs::write(
        syntax.join("check_particle_selection.gd"),
        include_bytes!("check_particle_selection.gd"),
    )?;
    let parser_log = evidence.join("parser.log");
    godot::run_godot(
        &engine,
        &syntax,
        &[
            "--headless",
            "--log-file",
            parser_log.to_str().context("parser log path")?,
            "--check-only",
            "--script",
            "res://check_particle_selection.gd",
        ],
        Duration::from_secs(10),
    )?;
    ensure!(fs::metadata(&parser_log)?.len() <= 1024 * 1024);
    ensure!(!fs::read_to_string(&parser_log)?.contains("ERROR:"));

    let selection = Selection::hype("manoir", "en")?;
    let mut imported = openspace::content::import_scene(&root.join("disc"), selection.clone())?;
    let model_ref = AssetRef::<AiModel>::new("fix:/ai/models/28")?;
    let full = imported.load(&model_ref)?;
    let schedule = full
        .normal
        .as_ref()
        .context("source Hype normal set absent")?
        .behaviors
        .iter()
        .find(|b| b.id == 0)
        .and_then(|b| b.schedule.as_ref())
        .context("source Hype initialization absent")?;
    let pair = schedule
        .statements
        .windows(2)
        .find(|pair| {
            matches!(
                pair,
                [
                    Statement::SetParticleGenerator(_),
                    Statement::SetParticleGeneratorEnabled(false)
                ]
            )
        })
        .context("source select/disable pair absent")?;
    let Statement::SetParticleGenerator(reference) = &pair[0] else {
        unreachable!()
    };
    let generator = imported.load(reference)?;
    let mut excerpt = (*full).clone();
    let normal = excerpt.normal.as_mut().unwrap();
    normal.behaviors.retain(|b| b.id == 0);
    normal.behaviors[0].rules.clear();
    normal.behaviors[0].schedule.as_mut().unwrap().statements = pair.to_vec();
    let presentation = schedule
        .statements
        .windows(3)
        .find(|part| {
            matches!(
                part,
                [
                    Statement::SetTransparency(astrolabe::model::ai::Expression::Integer(255)),
                    Statement::SetTransparentDisplay(true),
                    Statement::SetDisplayVisible(true)
                ]
            )
        })
        .context("source presentation triple absent")?;
    normal.behaviors[0]
        .schedule
        .as_mut()
        .unwrap()
        .statements
        .extend_from_slice(presentation);
    normal.default_behavior = Some(0);
    excerpt.reflex = None;
    // Designer defaults are unchanged; the fixture removes all other schedules
    // and rules. It is a disclosed excerpt, not an acceptance of full Hype AI.
    fs::write(
        evidence.join("fixture.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "scope":"source nodes14..16 and814..817 select/disable and opacity/display excerpt; component regression, full intro pending",
            "ai_modified":true,"model":full.path,"generator":generator.path,
            "full_model_sha256":format!("{:x}",Sha256::digest(rete::encode(full.as_ref())?)),
            "excerpt_sha256":format!("{:x}",Sha256::digest(rete::encode(&excerpt)?)),
            "source_identity":generator.source_identity,
            "active_emission":"explicitly unsupported"
        }))?,
    )?;
    // Persist the actual sprite/material/texture closure; discard the source
    // resolver before exporting. No original blobs are replayed by Godot.
    let store_path = temporary.path().join("rete");
    let mut store = rete::Store::new(&store_path, selection.clone())?;
    store.save(reference, generator.as_ref())?;
    let sprite = imported.load(&generator.sprite)?;
    store.save(&generator.sprite, sprite.as_ref())?;
    for view in &sprite.views {
        if let Some(reference) = &view.material {
            let material = imported.load(reference)?;
            store.save(reference, material.as_ref())?;
            if let Some(path) = material.texture() {
                let reference =
                    AssetRef::<astrolabe::model::texture::Texture>::from_path(path.clone());
                let texture = imported.load(&reference)?;
                store.save(&reference, texture.as_ref())?;
            }
        }
    }
    store.save(&model_ref, &excerpt)?;
    drop(store);
    drop(imported);
    let mut reloaded = rete::Store::new(store_path, selection.clone())?;
    ensure!(*reloaded.load(reference)? == *generator);
    ensure!(*reloaded.load(&model_ref)? == excerpt);
    let project = evidence.join("project");
    godot::export(
        &mut reloaded,
        &[ContentRoot {
            kind: AiModel::KIND.into(),
            reference: model_ref.path,
        }],
        selection,
        &project,
        &engine,
    )?;
    let scene_path = format!("res://generated/hype/shared/{}.tscn", generator.path.file());
    fs::write(
        project.join("fixture.json"),
        serde_json::to_vec(&serde_json::json!({
            "generator_scene":scene_path,"generator_uri":generator.path,
            "brain_script":"res://generated/hype/levels/manoir/ai/models/28.gd"
        }))?,
    )?;
    fs::write(
        project.join("check_particle_selection.gd"),
        include_bytes!("check_particle_selection.gd"),
    )?;
    let engine_log = evidence.join("engine.log");
    let report = evidence.join("report.json");
    let output = godot::run_godot(
        &engine,
        &project,
        &[
            "--headless",
            "--log-file",
            engine_log.to_str().context("engine log path")?,
            "--script",
            "res://check_particle_selection.gd",
            "--",
            report.to_str().context("report path")?,
        ],
        Duration::from_secs(15),
    )?;
    fs::write(evidence.join("stdout.log"), &output)?;
    ensure!(output.contains("PARTICLE_SELECTION_EXCERPT_OK"));
    let result: serde_json::Value = serde_json::from_slice(&fs::read(report)?)?;
    ensure!(result["status"] == "passed" && result["active_emission_supported"] == false);
    let reloaded: ParticleGenerator = (*reloaded.load(reference)?).clone();
    ensure!(reloaded.capacity == 200);
    println!(
        "Particle selection component evidence: {}",
        evidence.display()
    );
    Ok(())
}
