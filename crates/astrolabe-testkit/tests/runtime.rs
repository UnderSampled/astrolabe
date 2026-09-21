#![cfg(target_os = "linux")]
use anyhow::{Result, ensure};
use astrolabe_testkit::linux::{Consistency, Owner, Process};
use std::{
    fs,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
fn command(output: &std::path::Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_astrolabe-testkit"));
    command
        .arg("fixture")
        .arg(output)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}
fn ready(owner: &mut Owner, output: &std::path::Path) -> Result<(Process, usize)> {
    let started = Instant::now();
    while !output.exists() {
        ensure!(
            started.elapsed() < Duration::from_secs(2),
            "fixture did not start"
        );
        thread::sleep(Duration::from_millis(2));
    }
    let value: serde_json::Value = serde_json::from_slice(&fs::read(output)?)?;
    let pid = value["pid"].as_i64().unwrap() as i32;
    let process = owner
        .processes()?
        .into_iter()
        .find(|p| p.identity().pid == pid)
        .unwrap();
    Ok((process, value["address"].as_u64().unwrap() as usize))
}
fn words(bytes: &[u8]) -> Vec<u64> {
    bytes
        .as_chunks::<8>()
        .0
        .iter()
        .map(|b| u64::from_ne_bytes(*b))
        .collect()
}
#[test]
fn stopped_capture_is_bounded_immutable_and_never_completed_update() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let output = temp.path().join("fixture.json");
    let mut owner = Owner::new()?;
    owner.spawn(command(&output).arg("--partial"))?;
    let (process, address) = ready(&mut owner, &output)?;
    let start = Instant::now();
    while words(&process.read_running(address, 8)?)[0] != 1 {
        ensure!(
            start.elapsed() < Duration::from_secs(2),
            "partial-write fixture not reached"
        );
        thread::sleep(Duration::from_millis(1));
    }
    let stopped = process.pause(Duration::from_secs(1))?;
    assert!(stopped.stopped_threads() >= 2);
    assert!(!stopped.preserved_existing_stop());
    let mut capture = stopped.capture();
    let bytes = capture.read(address, 24)?;
    assert_ne!(
        words(&bytes)[1],
        words(&bytes)[2],
        "fake producer was stopped between related field writes"
    );
    assert_eq!(capture.read(address + 8, 8)?, bytes[8..16]);
    assert!(capture.read(0, 8).is_err());
    assert!(capture.read(usize::MAX - 2, 8).is_err());
    assert!(capture.read(address, 0).is_err());
    assert!(capture.read(address, 1_048_577).is_err());
    let snapshot = capture.finish()?;
    drop(stopped);
    fs::write(output.with_extension("continue"), b"resume fixture writer")?;
    thread::sleep(Duration::from_millis(75));
    assert_eq!(snapshot.consistency, Consistency::StoppedBytes);
    assert_eq!(snapshot.bytes(address, 24)?, bytes);
    assert!(
        snapshot.bytes(address + 24, 8).is_err(),
        "missing pages cannot be fetched after resume"
    );
    assert!(snapshot.require_completed_update().is_err());
    owner.close()?;
    assert!(process.check().is_err());
    assert!(process.read_running(address, 8).is_err());
    Ok(())
}
#[test]
fn pause_preserves_existing_stop_and_unwinds_new_stop() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let output = temp.path().join("fixture.json");
    let mut owner = Owner::new()?;
    owner.spawn(&mut command(&output))?;
    let (process, address) = ready(&mut owner, &output)?;
    let pid = process.identity().pid;
    // SAFETY: this test owns the exact live fixture process and never exposes its PID to a shell.
    assert_eq!(unsafe { libc::kill(pid, libc::SIGSTOP) }, 0);
    let start = Instant::now();
    while !fs::read_to_string(format!("/proc/{pid}/status"))?.contains("State:\tT") {
        ensure!(
            start.elapsed() < Duration::from_secs(1),
            "fixture did not stop"
        );
        thread::sleep(Duration::from_millis(1));
    }
    {
        let guard = process.pause(Duration::from_secs(1))?;
        assert!(guard.preserved_existing_stop());
    }
    assert!(fs::read_to_string(format!("/proc/{pid}/status"))?.contains("State:\tT"));
    // SAFETY: resume the pre-existing stop established by this test.
    assert_eq!(unsafe { libc::kill(pid, libc::SIGCONT) }, 0);
    let caught = std::panic::catch_unwind(|| {
        let _guard = process.pause(Duration::from_secs(1)).unwrap();
        panic!("injected assertion failure");
    });
    assert!(caught.is_err());
    let before = process.read_running(address + 8, 8)?;
    thread::sleep(Duration::from_millis(100));
    assert_ne!(
        before,
        process.read_running(address + 8, 8)?,
        "RAII must resume after assertion failure"
    );
    owner.close()?;
    Ok(())
}
#[test]
fn cleanup_reaps_term_resistant_escaped_descendant_and_preserves_unrelated_child() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let unrelated_path = temp.path().join("unrelated.json");
    let mut unrelated = command(&unrelated_path).spawn()?;
    let result = (|| {
        let output = temp.path().join("owned.json");
        let mut owner = Owner::new()?;
        owner.spawn(command(&output).arg("--escaped"))?;
        let (process, _) = ready(&mut owner, &output)?;
        thread::sleep(Duration::from_millis(150));
        let started = Instant::now();
        owner.close()?;
        assert!(started.elapsed() < Duration::from_secs(4));
        assert!(process.check().is_err());
        assert!(!std::path::Path::new(&format!("/proc/{}", process.identity().pid)).exists());
        assert!(unrelated.try_wait()?.is_none());
        Ok(())
    })();
    unrelated.kill()?;
    unrelated.wait()?;
    result
}
#[test]
fn failed_launch_and_duplicate_owner_do_not_leave_ownership_locked() -> Result<()> {
    {
        let mut owner = Owner::new()?;
        assert!(Owner::new().is_err());
        assert!(
            owner
                .spawn(&mut Command::new("/nonexistent/astrolabe-fixture"))
                .is_err()
        );
        owner.close()?;
    }
    Owner::new()?.close()?;
    Ok(())
}
