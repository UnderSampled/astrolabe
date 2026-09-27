use anyhow::Result;
use session_recording::{
    EventWriter, SaveRootKind, capture_saves, inspect_saves, read_events, recording_sha256,
    stage_saves, verify_saves,
};
use std::{fs, fs::FileTimes, time::Duration};

#[test]
fn save_fixture_distinguishes_absent_empty_and_populated_and_prevents_contamination() -> Result<()>
{
    let root = tempfile::tempdir()?;
    let absent = capture_saves(&root.path().join("missing"), &root.path().join("absent"))?;
    assert_eq!(absent.kind, SaveRootKind::Absent);
    assert_eq!(inspect_saves(&root.path().join("missing"))?, absent);
    stage_saves(
        &root.path().join("absent"),
        &absent,
        &root.path().join("absent-run"),
    )?;
    assert!(!root.path().join("absent-run").exists());

    fs::create_dir(root.path().join("empty-source"))?;
    let empty = capture_saves(
        &root.path().join("empty-source"),
        &root.path().join("empty"),
    )?;
    assert_eq!(empty.kind, SaveRootKind::Empty);
    assert_eq!(inspect_saves(&root.path().join("empty-source"))?, empty);
    stage_saves(
        &root.path().join("empty"),
        &empty,
        &root.path().join("empty-run"),
    )?;
    assert!(root.path().join("empty-run").is_dir());

    let source = root.path().join("source");
    fs::create_dir_all(source.join("SaveGame0/empty"))?;
    fs::write(source.join("SaveGame0/General.sav"), b"initial bytes")?;
    let fixture = capture_saves(&source, &root.path().join("fixture"))?;
    assert_eq!(fixture.kind, SaveRootKind::Populated);
    assert_eq!(inspect_saves(&source)?, fixture);
    assert_eq!(fixture.directories, ["SaveGame0", "SaveGame0/empty"]);
    assert!(
        fs::metadata(root.path().join("fixture/SaveGame0/General.sav"))?
            .permissions()
            .readonly()
    );
    let first = root.path().join("first-run");
    stage_saves(&root.path().join("fixture"), &fixture, &first)?;
    assert!(
        !fs::metadata(first.join("SaveGame0/General.sav"))?
            .permissions()
            .readonly()
    );
    fs::write(first.join("SaveGame0/General.sav"), b"changed by first run")?;
    fs::write(first.join("SaveGame0/new.sav"), b"new output")?;
    assert_ne!(inspect_saves(&first)?, fixture);
    let changed = inspect_saves(&first)?;
    let nested_snapshot = root.path().join("snapshots/000000");
    assert_eq!(capture_saves(&first, &nested_snapshot)?, changed);
    verify_saves(&nested_snapshot, &changed)?;
    let second = root.path().join("second-run");
    stage_saves(&root.path().join("fixture"), &fixture, &second)?;
    verify_saves(&second, &fixture)?;
    assert_eq!(
        fs::read(second.join("SaveGame0/General.sav"))?,
        b"initial bytes"
    );
    assert_eq!(
        fs::metadata(second.join("SaveGame0/General.sav"))?.modified()?,
        fs::metadata(source.join("SaveGame0/General.sav"))?.modified()?,
    );
    let changed_time =
        fs::metadata(second.join("SaveGame0/General.sav"))?.modified()? + Duration::from_secs(1);
    fs::File::open(second.join("SaveGame0/General.sav"))?
        .set_times(FileTimes::new().set_modified(changed_time))?;
    assert!(verify_saves(&second, &fixture).is_err());
    assert!(!second.join("SaveGame0/new.sav").exists());
    assert!(stage_saves(&root.path().join("fixture"), &fixture, &first).is_err());
    Ok(())
}

#[test]
fn event_prefix_and_recording_hash_include_frames() -> Result<()> {
    let root = tempfile::tempdir()?;
    let recording = root.path();
    let mut writer = EventWriter::create(&recording.join("events.jsonl"))?;
    writer.append(0, "launch", serde_json::json!({}))?;
    writer.append(15, "frame", serde_json::json!({"path":"frames/0.png"}))?;
    assert!(writer.append(14, "state", serde_json::json!({})).is_err());
    drop(writer);
    fs::write(recording.join("manifest.json"), b"{}")?;
    fs::create_dir(recording.join("frames"))?;
    fs::write(recording.join("frames/0.png"), b"frame one")?;
    let first = recording_sha256(recording)?;
    fs::write(recording.join("frames/0.png"), b"frame two")?;
    assert_ne!(first, recording_sha256(recording)?);
    fs::write(
        recording.join("events.jsonl"),
        b"{\"sequence\":0,\"elapsed_micros\":0,\"kind\":\"launch\",\"data\":{}}\n{broken",
    )?;
    let (events, complete) = read_events(&recording.join("events.jsonl"))?;
    assert_eq!(events.len(), 1);
    assert!(!complete);
    Ok(())
}
