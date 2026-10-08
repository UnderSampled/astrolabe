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
    let text = normalize_font_properties(&text)?;
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
    // Reserve all initial hashes before probing, so a collision cannot steal
    // the otherwise unchanged ID of a node appearing later in this file.
    let initial_ids = text
        .lines()
        .skip(1)
        .filter(|line| line.starts_with("[node "))
        .map(|line| {
            let name = field(line, "name")?;
            let parent = if line.contains(" parent=\"") {
                field(line, "parent")?
            } else {
                String::new()
            };
            Ok((identity(&format!("{path}:{parent}/{name}")) & i32::MAX as u64).max(1))
        })
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        initial_ids.len() <= i32::MAX as usize,
        "native node identity space exhausted in {path}"
    );
    let reserved_ids: std::collections::BTreeSet<_> = initial_ids.iter().copied().collect();
    let mut initial_ids = initial_ids.into_iter();
    let mut normalized = String::with_capacity(text.len());
    let mut node_ids = std::collections::BTreeSet::new();
    for (i, line) in text.lines().enumerate() {
        if i == 0 {
            let prefix = line
                .strip_suffix(']')
                .context("native resource header missing")?;
            normalized.push_str(&format!("{prefix} uid=\"{}\"]\n", resource_uid(path)));
        } else if line.starts_with("[node ") {
            let initial = initial_ids
                .next()
                .expect("one initial hash per native node");
            let mut id = initial;
            if !node_ids.insert(id) {
                loop {
                    id = if id == i32::MAX as u64 { 1 } else { id + 1 };
                    ensure!(
                        id != initial,
                        "native node identity space exhausted in {path}"
                    );
                    if !reserved_ids.contains(&id) && node_ids.insert(id) {
                        break;
                    }
                }
            }
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

/// A bitmap font must select its size before metadata initializes a native cache.
/// Godot 4.7 ResourceSaver writes font_name first; loading it creates an empty
/// 16px cache before fixed_size is applied. Bitmap has_char()/shaping consult that
/// first cache, even when the later 21px cache contains every source glyph.
/// Preserve all native properties and their order except this prerequisite.
fn normalize_font_properties(text: &str) -> Result<String> {
    if !text.starts_with("[gd_resource type=\"FontFile\"") {
        return Ok(text.to_owned());
    }
    let (prefix, body) = text
        .split_once("\n[resource]\n")
        .context("native FontFile resource section missing")?;
    let Some(fixed_size) = body.lines().find(|line| line.starts_with("fixed_size = ")) else {
        return Ok(text.to_owned());
    };
    if fixed_size[13..].parse::<u32>()? == 0 {
        return Ok(text.to_owned());
    }
    let mut normalized = format!("{prefix}\n[resource]\n{fixed_size}\n");
    for line in body.lines().filter(|line| *line != fixed_size) {
        normalized.push_str(line);
        normalized.push('\n');
    }
    Ok(normalized)
}

/// Remove engine-generated identities and rewrite resource IDs in bounded
/// passes. Each input span is copied once; new IDs never become rewrite input.
pub(super) fn strip_uids(text: &str) -> String {
    let value = strip_native_identity_attributes(text);
    let mut declarations = std::collections::BTreeMap::new();
    let mut index = 0usize;
    for line in value.lines() {
        if let Some((kind, range)) = resource_declaration(line) {
            index += 1;
            // Duplicate IDs are invalid native content, but keeping the first
            // mapping retains the previous normalizer's deterministic choice.
            declarations
                .entry((kind, &line[range]))
                .or_insert_with(|| format!("stable_{index}"));
        }
    }
    let mut normalized = String::with_capacity(value.len());
    for line in value.split_inclusive('\n') {
        let declaration = resource_declaration(line);
        let mut declaration_pending = true;
        let bytes = line.as_bytes();
        let mut copied = 0;
        let mut cursor = 0;
        while cursor < bytes.len() {
            if declaration_pending
                && let Some((kind, range)) = &declaration
                && cursor == range.start
            {
                normalized.push_str(&line[copied..cursor]);
                normalized.push_str(&declarations[&(*kind, &line[range.clone()])]);
                cursor = range.end;
                copied = cursor;
                declaration_pending = false;
                continue;
            }
            let reference = if bytes[cursor..].starts_with(b"ExtResource(\"") {
                Some(("ExtResource", b"ExtResource(\"".len()))
            } else if bytes[cursor..].starts_with(b"SubResource(\"") {
                Some(("SubResource", b"SubResource(\"".len()))
            } else {
                None
            };
            if let Some((kind, prefix)) = reference {
                let start = cursor + prefix;
                let Some(relative_end) = bytes[start..].iter().position(|&b| b == b'"') else {
                    break;
                };
                let end = start + relative_end;
                if bytes.get(end + 1) == Some(&b')') {
                    if let Some(new) = declarations.get(&(kind, &line[start..end])) {
                        normalized.push_str(&line[copied..start]);
                        normalized.push_str(new);
                        copied = end;
                    }
                    cursor = end + 2;
                } else {
                    cursor = end + 1;
                }
            } else {
                cursor += 1;
            }
        }
        normalized.push_str(&line[copied..]);
    }
    normalized
}

fn resource_declaration(line: &str) -> Option<(&'static str, std::ops::Range<usize>)> {
    let kind = if line.starts_with("[ext_resource ") {
        "ExtResource"
    } else if line.starts_with("[sub_resource ") {
        "SubResource"
    } else {
        return None;
    };
    let start = line.find(" id=\"")? + 5;
    let end = line[start..].find('"')? + start;
    Some((kind, start..end))
}

fn strip_native_identity_attributes(text: &str) -> String {
    const UID: &[u8] = b" uid=\"uid://";
    const NODE_ID: &[u8] = b" unique_id=";
    const PARENT_ID: &[u8] = b" parent_id_path=PackedInt32Array(";
    let bytes = text.as_bytes();
    let mut normalized = String::with_capacity(text.len());
    let mut copied = 0;
    let mut cursor = 0;
    let mut scan_uid = true;
    let mut scan_parent = true;
    while cursor < bytes.len() {
        let suffix = &bytes[cursor..];
        let end = if scan_uid && suffix.starts_with(UID) {
            match bytes[cursor + UID.len()..].iter().position(|&b| b == b'"') {
                Some(end) => Some(cursor + UID.len() + end + 1),
                None => {
                    scan_uid = false;
                    None
                }
            }
        } else if suffix.starts_with(NODE_ID) {
            let mut end = cursor + NODE_ID.len();
            while bytes.get(end).is_some_and(u8::is_ascii_digit) {
                end += 1;
            }
            Some(end)
        } else if scan_parent && suffix.starts_with(PARENT_ID) {
            match bytes[cursor + PARENT_ID.len()..]
                .iter()
                .position(|&b| b == b')')
            {
                Some(end) => Some(cursor + PARENT_ID.len() + end + 1),
                None => {
                    scan_parent = false;
                    None
                }
            }
        } else {
            None
        };
        if let Some(end) = end {
            normalized.push_str(&text[copied..cursor]);
            copied = end;
            cursor = end;
        } else {
            cursor += 1;
        }
    }
    normalized.push_str(&text[copied..]);
    normalized
}

/// Whether a gameplay launch should show its rendered window.
pub fn gameplay_headless() -> Result<bool> {
    Ok(match std::env::var("ASTROLABE_GODOT_HEADLESS").as_deref() {
        Ok("0") => false,
        Ok("1") | Err(std::env::VarError::NotPresent) => true,
        _ => anyhow::bail!("ASTROLABE_GODOT_HEADLESS must be 0 or 1"),
    })
}

/// Bounded editor, import, and script validation. These checks do not render gameplay.
pub fn run_godot(godot: &str, project: &Path, args: &[&str], timeout: Duration) -> Result<String> {
    run_godot_with_display(godot, project, args, timeout, false)
}

/// Bounded gameplay launch. `--no-headless` shows the exported scene.
pub fn run_godot_gameplay(
    godot: &str,
    project: &Path,
    args: &[&str],
    timeout: Duration,
) -> Result<String> {
    run_godot_with_display(godot, project, args, timeout, true)
}

fn run_godot_with_display(
    godot: &str,
    project: &Path,
    args: &[&str],
    timeout: Duration,
    gameplay: bool,
) -> Result<String> {
    let headless = !gameplay || gameplay_headless()?;
    let effective_args = args
        .iter()
        .copied()
        .filter(|arg| headless || *arg != "--headless")
        .collect::<Vec<_>>();
    let log = tempfile::tempfile()?;
    let mut child = Command::new(godot)
        .arg("--path")
        .arg(project)
        .args(&effective_args)
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
                "Godot {effective_args:?} in {} exceeded {}s deadline (log capped at 64KiB): {output}",
                project.display(),
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
        "Godot {effective_args:?} in {} reported errors: {output}",
        project.display()
    );
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitmap_font_size_precedes_name_without_changing_native_glyphs() -> Result<()> {
        let input = "[gd_resource type=\"FontFile\" format=4]\n\n[resource]\nfont_name = \"Playmo_Font2_0\"\nsubpixel_positioning = 3\nallow_system_fallback = false\nfixed_size = 21\ncache/0/21/0/ascent = 17.2043\ncache/0/21/0/glyphs/233/advance = Vector2(9.432005, 0)\ncache/0/21/1/glyphs/233/texture_idx = 0\n";
        let normalized = normalize_font_properties(input)?;
        assert!(normalized.contains("[resource]\nfixed_size = 21\nfont_name = "));
        assert_eq!(
            normalized.replace("fixed_size = 21\n", ""),
            input.replace("fixed_size = 21\n", "")
        );
        assert_eq!(normalize_font_properties(&normalized)?, normalized);
        let native = normalize_native("generated/dialogue.font.tres", input)?;
        assert!(native.contains("[resource]\nfixed_size = 21\nfont_name = "));
        assert_eq!(
            normalize_native("generated/dialogue.font.tres", &native)?,
            native
        );
        Ok(())
    }

    #[test]
    fn dynamic_fonts_and_other_resources_keep_property_order() -> Result<()> {
        for input in [
            "[gd_resource type=\"FontFile\" format=4]\n\n[resource]\nfont_name = \"Dynamic\"\nfixed_size = 0\n",
            "[gd_resource type=\"FontFile\" format=4]\n\n[resource]\nfont_name = \"Dynamic\"\n",
            "[gd_resource type=\"Resource\" format=3]\n\n[resource]\nname = \"Other\"\nfixed_size = 21\n",
        ] {
            assert_eq!(normalize_font_properties(input)?, input);
        }
        Ok(())
    }
}
