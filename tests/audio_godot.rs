//! Local real-media values through persisted Rete and the actual Godot audio mixer.
use anyhow::{Context, Result, ensure};
use astrolabe::{
    godot::{Project, run_godot},
    model::{
        AssetRef, Selection,
        audio::Sound,
        io::{Load, Save},
    },
    openspace::codec::{Apm, Bnm, FormatCodec},
    rete::{Package, Root, Store},
};
use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

#[test]
#[ignore = "requires local game corpus and Godot; use tools/test.py --local"]
fn real_media_rete_and_godot_mixer() -> Result<()> {
    let start = Instant::now();
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("disc"));
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let mut sounds = Vec::new();
    for name in ["Achat.apm", "AmbCiel.apm"] {
        sounds.push(
            Apm { name: name.into() }
                .read(&fs::read(game.join("Gamedata/World/Sound").join(name))?)?,
        );
    }
    let bank = Bnm {
        name: "English/Bnk_155.bnm".into(),
    }
    .read(&fs::read(
        game.join("Gamedata/World/Sound/English/Bnk_155.bnm"),
    )?)?;
    sounds.push(
        bank.sounds
            .into_iter()
            .next()
            .context("voice fixture empty")?,
    );
    // M_Astrolabe.csb Play_Rajintro → theme parts → resource9 URAJO11;
    // playmo_b.lcb maps that resource to bank103 slot1.
    let music = Bnm {
        name: "Bnk_103.bnm".into(),
    }
    .read(&fs::read(game.join("Gamedata/World/Sound/Bnk_103.bnm"))?)?;
    let part = music
        .sounds
        .first()
        .context("music bank lacks first embedded theme part")?;
    sounds.push(part.clone());
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("rete");
    let selection = Selection::hype("astrolabe", "en")?;
    let mut store = Store::new(&root, selection.clone())?;
    let references = (0..sounds.len())
        .map(|n| AssetRef::<Sound>::new(format!("sound:/probe/{n}")))
        .collect::<Result<Vec<_>>>()?;
    for (reference, sound) in references.iter().zip(&sounds) {
        store.save(reference, sound)?;
    }
    store.save_package(&Package {
        diagnostics: vec![],
        selection: selection.clone(),
        roots: references
            .iter()
            .map(|id| Root {
                kind: "sound".into(),
                reference: id.path.clone(),
            })
            .collect(),
    })?;
    drop(store);
    let (mut store, _) = Store::open(root)?;
    let output = temp.path().join("godot");
    let mut project = Project::new(&output, selection)?;
    for (reference, expected) in references.iter().zip(&sounds) {
        let restored = store.load(reference)?;
        ensure!(
            *restored == *expected,
            "Rete changed sample values/loops/channels"
        );
        project.save(reference, &restored)?;
    }
    project.finish(&engine)?;
    for (reference, expected) in references.iter().zip(&sounds) {
        let wav = output.join(format!(
            "generated/hype/sounds/{}.wav",
            reference.path.file()
        ));
        let mut reader = hound::WavReader::open(wav)?;
        ensure!(
            reader.spec().channels == expected.channels
                && reader.spec().sample_rate == expected.sample_rate,
            "target WAV format differs"
        );
        ensure!(
            reader.samples::<i16>().collect::<Result<Vec<_>, _>>()? == expected.interleaved_i16(),
            "target WAV values differ from explicit PCM projection"
        );
    }
    let report = temp.path().join("playback.json");
    run_godot(
        &engine,
        &output,
        &[
            "--headless",
            "--",
            "--audio-check",
            report.to_str().unwrap(),
        ],
        Duration::from_secs(25),
    )?;
    let evidence: serde_json::Value = serde_json::from_slice(&fs::read(report)?)?;
    let rows = evidence.as_array().context("audio evidence array")?;
    ensure!(rows.len() == sounds.len(), "missing playback evidence");
    for (row, expected) in rows.iter().zip(&sounds) {
        ensure!(
            row["captured_frames"].as_u64().unwrap_or(0) > 0
                && row["peak"].as_f64().unwrap_or(0.) > 0.,
            "Godot mixer produced no sound"
        );
        ensure!(
            (row["duration"].as_f64().unwrap() - expected.duration_seconds()).abs()
                < 1. / f64::from(expected.sample_rate),
            "duration differs"
        );
        ensure!(
            row["rate"].as_u64() == Some(u64::from(expected.sample_rate))
                && row["stereo"].as_bool() == Some(expected.channels == 2),
            "Godot channels/rate differ"
        );
        if let Some(loop_region) = &expected.loop_region {
            ensure!(
                row["loop_ok"] == true
                    && row["loop_begin"].as_u64() == Some(loop_region.start_frame)
                    && row["loop_end"].as_u64() == Some(loop_region.end_frame),
                "loop did not wrap at expected samples"
            );
        }
    }
    println!(
        "{} real sounds survived Rete and Godot mixer in {:.2}s",
        sounds.len(),
        start.elapsed().as_secs_f64()
    );
    Ok(())
}
