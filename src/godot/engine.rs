//! Bounded Godot execution and deterministic ResourceSaver normalization.
use anyhow::{Context, Result, ensure};
use std::{
    io::Read,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
pub(super) fn strip_uids(text: &str) -> String {
    let mut value = text.to_owned();
    while let Some(start) = value.find(" uid=\"uid://") {
        if let Some(end) = value[start + 6..].find('"') {
            value.replace_range(start..start + 6 + end + 1, "");
        } else {
            break;
        }
    }
    // Godot 4.7 assigns random editor node identities on PackedScene.save.
    // These provisional standalone assets use node paths, not persistent editor IDs.
    while let Some(start) = value.find(" unique_id=") {
        let end = value[start + 11..]
            .find(|c: char| !c.is_ascii_digit())
            .map_or(value.len(), |n| start + 11 + n);
        value.replace_range(start..end, "");
    }
    // ResourceSaver also chooses opaque IDs. File-local IDs only connect references;
    // name them by declaration order so independent engine processes agree.
    let declarations = value
        .lines()
        .filter_map(|line| {
            let kind = if line.starts_with("[ext_resource ") {
                "ExtResource"
            } else if line.starts_with("[sub_resource ") {
                "SubResource"
            } else {
                return None;
            };
            let start = line.find(" id=\"")? + 5;
            let end = line[start..].find('"')? + start;
            Some((kind, line[start..end].to_owned()))
        })
        .collect::<Vec<_>>();
    for (index, (kind, old)) in declarations.iter().enumerate() {
        let new = format!("stable_{}", index + 1);
        value = value.replace(&format!(" id=\"{old}\""), &format!(" id=\"{new}\""));
        value = value.replace(&format!("{kind}(\"{old}\")"), &format!("{kind}(\"{new}\")"));
    }
    value
}

/// Bounded engine process. Redirected output avoids pipe deadlocks for verbose imports.
pub fn run_godot(godot: &str, project: &Path, args: &[&str], timeout: Duration) -> Result<String> {
    let log = tempfile::tempfile()?;
    let mut child = Command::new(godot)
        .arg("--path")
        .arg(project)
        .args(args)
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log.try_clone()?))
        .spawn()
        .with_context(|| format!("start {godot}"))?;
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() >= timeout {
            child.kill()?;
            child.wait()?;
            let mut log = log;
            std::io::Seek::rewind(&mut log)?;
            let mut output = String::new();
            log.read_to_string(&mut output)?;
            anyhow::bail!("Godot exceeded {}s deadline: {output}", timeout.as_secs());
        }
        thread::sleep(Duration::from_millis(50));
    };
    let mut log = log;
    std::io::Seek::rewind(&mut log)?;
    let mut output = String::new();
    log.read_to_string(&mut output)?;
    ensure!(status.success(), "Godot failed ({status}): {output}");
    ensure!(
        !output.contains("SCRIPT ERROR:") && !output.contains("ERROR:"),
        "Godot reported errors: {output}"
    );
    Ok(output)
}
