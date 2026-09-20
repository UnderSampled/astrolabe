//! Bounded Godot execution and deterministic ResourceSaver normalization.
use anyhow::{Context, Result, ensure};
use std::{
    io::Read,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn identity(key: &str) -> u64 {
    let hash = super::digest(format!("astrolabe-godot-v1\0{key}").as_bytes());
    u64::from_str_radix(&hash[..16], 16).expect("SHA256 hex") & i64::MAX as u64
}

pub(super) fn resource_uid(path: &str) -> String {
    // Godot ResourceUID uses base 34 (a-y, 0-8), including its historical
    // off-by-one alphabet. See core/io/resource_uid.cpp, id_to_text.
    let mut id = identity(path);
    let alphabet = b"abcdefghijklmnopqrstuvwxy012345678";
    let mut digits = Vec::new();
    loop {
        digits.push(alphabet[(id % 34) as usize] as char);
        id /= 34;
        if id == 0 {
            break;
        }
    }
    format!("uid://{}", digits.into_iter().rev().collect::<String>())
}

pub(super) fn normalize_native(path: &str, text: &str) -> Result<String> {
    let text = strip_uids(text);
    let field = |line: &str, name: &str| -> Result<String> {
        let rest = line
            .split_once(&format!(" {name}=\""))
            .context("native node field missing")?
            .1;
        Ok(rest
            .split('"')
            .next()
            .context("native node field truncated")?
            .to_string())
    };
    let mut normalized = String::new();
    let mut node_ids = std::collections::BTreeSet::new();
    for (i, line) in text.lines().enumerate() {
        if i == 0 {
            let prefix = line
                .strip_suffix(']')
                .context("native resource header missing")?;
            normalized.push_str(&format!("{prefix} uid=\"{}\"]\n", resource_uid(path)));
        } else if line.starts_with("[node ") {
            let name = field(line, "name")?;
            let parent = if line.contains(" parent=\"") {
                field(line, "parent")?
            } else {
                String::new()
            };
            let id = (identity(&format!("{path}:{parent}/{name}")) & i32::MAX as u64).max(1);
            ensure!(
                node_ids.insert(id),
                "native node identity collision in {path}"
            );
            normalized.push_str(&format!(
                "{} unique_id={id}]\n",
                line.strip_suffix(']')
                    .context("native node header truncated")?
            ));
        } else {
            normalized.push_str(line);
            normalized.push('\n');
        }
    }
    Ok(normalized)
}
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
    // Inherited overrides can also retain ResourceSaver's random parent IDs.
    // They no longer identify the normalized nodes. Godot's text loader accepts
    // the explicit parent path without this optional rename-recovery hint.
    // Keep that path instead of publishing stale references to random IDs.
    while let Some(start) = value.find(" parent_id_path=PackedInt32Array(") {
        let Some(end) = value[start..].find(')') else {
            break;
        };
        value.replace_range(start..start + end + 1, "");
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
        let excessive_output = log.metadata()?.len() > 8 * 1024 * 1024;
        if started.elapsed() >= timeout || excessive_output {
            child.kill()?;
            child.wait()?;
            let mut log = log;
            std::io::Seek::rewind(&mut log)?;
            let mut output = String::new();
            log.take(64 * 1024).read_to_string(&mut output)?;
            if excessive_output {
                anyhow::bail!("Godot exceeded 8MiB diagnostic limit (truncated): {output}");
            }
            anyhow::bail!(
                "Godot exceeded {}s deadline (log capped at 64KiB): {output}",
                timeout.as_secs()
            );
        }
        thread::sleep(Duration::from_millis(50));
    };
    let mut log = log;
    std::io::Seek::rewind(&mut log)?;
    let mut output = String::new();
    log.take(8 * 1024 * 1024).read_to_string(&mut output)?;
    ensure!(status.success(), "Godot failed ({status}): {output}");
    ensure!(
        !output.contains("SCRIPT ERROR:") && !output.contains("ERROR:"),
        "Godot reported errors: {output}"
    );
    Ok(output)
}
