use anyhow::Result;
use astrolabe_testkit::replay::{Edge, Key, PreparedRecording, boundary_recording};

#[test]
fn complete_recording_preserves_literal_edges_gaps_and_exact_bytes() -> Result<()> {
    let prepared = boundary_recording()?;
    let path = tempfile::NamedTempFile::new()?;
    std::fs::write(path.path(), prepared.bytes())?;
    let reopened = PreparedRecording::read(std::fs::read(path.path())?)?;
    assert_eq!(reopened.sha256(), prepared.sha256());
    assert_eq!(reopened.bytes(), prepared.bytes());
    let tape = reopened.recording();
    assert_eq!(tape.updates.len(), 18);
    let literal: Vec<_> = tape
        .updates
        .iter()
        .flat_map(|u| u.events.iter().map(move |e| (u.ordinal, e.key, e.edge)))
        .collect();
    assert_eq!(
        literal,
        [
            (3, Key::ArrowLeft, Edge::Press),
            (4, Key::ArrowLeft, Edge::Release),
            (5, Key::ArrowLeft, Edge::Press),
            (6, Key::ArrowLeft, Edge::Release),
            (8, Key::LeftShift, Edge::Press),
            (8, Key::ArrowLeft, Edge::Press),
            (13, Key::ArrowLeft, Edge::Release),
            (13, Key::LeftShift, Edge::Release),
        ]
    );
    assert!(tape.updates[13..].iter().all(|u| u.events.is_empty()));
    // Exact byte identity matters even when two JSON documents decode equally.
    let mut changed_bytes = prepared.bytes().to_vec();
    changed_bytes.push(b'\n');
    let changed = PreparedRecording::read(changed_bytes)?;
    assert_eq!(changed.recording(), prepared.recording());
    assert_ne!(changed.sha256(), prepared.sha256());
    Ok(())
}

#[test]
fn recording_rejects_skipped_ticks_repeat_edges_and_unreleased_keys() -> Result<()> {
    let valid = boundary_recording()?;
    let mut gap = valid.recording().clone();
    gap.updates.remove(6);
    assert!(gap.validate().unwrap_err().to_string().contains("ordinal"));
    let mut repeated = valid.recording().clone();
    repeated.updates[3].events[0].edge = Edge::Press;
    assert!(
        repeated
            .validate()
            .unwrap_err()
            .to_string()
            .contains("repeated press")
    );
    let mut held = valid.recording().clone();
    held.updates[12].events.pop();
    assert!(
        held.validate()
            .unwrap_err()
            .to_string()
            .contains("release every")
    );
    let mut impossible = valid.recording().clone();
    let event = impossible.updates[2].events[0].clone();
    impossible.updates[2].events.push(event);
    assert!(
        impossible
            .validate()
            .unwrap_err()
            .to_string()
            .contains("once per")
    );
    let mut unsupported = valid.recording().clone();
    unsupported.updates[2].events[0].keyboard = 1;
    assert!(
        unsupported
            .validate()
            .unwrap_err()
            .to_string()
            .contains("primary")
    );
    Ok(())
}
