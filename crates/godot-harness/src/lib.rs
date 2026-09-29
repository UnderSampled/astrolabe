//! Godot-only execution backends for recorded Hype behavior and sessions.
pub mod behavior;

/// Refuse rendered work unless DISPLAY is backed by a live local Xvfb server.
pub fn require_xvfb() -> anyhow::Result<String> {
    use anyhow::{Context, ensure};
    use std::{ffi::OsStr, fs, os::unix::ffi::OsStrExt, path::Path};

    let display = std::env::var("DISPLAY").context("rendered Godot requires an Xvfb DISPLAY")?;
    let authority =
        std::env::var("XAUTHORITY").context("rendered Godot requires XAUTHORITY from xvfb-run")?;
    ensure!(
        Path::new(&authority).is_file(),
        "XAUTHORITY does not name an existing file"
    );
    let number = display
        .strip_prefix(':')
        .and_then(|value| value.split('.').next())
        .filter(|value| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
        .context("DISPLAY must be a local Xvfb display such as :99")?;
    let canonical = format!(":{number}");
    let live = fs::read_dir("/proc")?
        .filter_map(std::result::Result::ok)
        .any(|entry| {
            if !entry.file_name().as_bytes().iter().all(u8::is_ascii_digit) {
                return false;
            }
            let Ok(bytes) = fs::read(entry.path().join("cmdline")) else {
                return false;
            };
            let args = bytes
                .split(|byte| *byte == 0)
                .filter(|arg| !arg.is_empty())
                .collect::<Vec<_>>();
            args.first()
                .and_then(|arg| Path::new(OsStr::from_bytes(arg)).file_name())
                .is_some_and(|name| name == "Xvfb")
                && args.iter().skip(1).any(|arg| *arg == canonical.as_bytes())
        });
    ensure!(
        live,
        "DISPLAY {display} is not backed by a live Xvfb process"
    );
    Ok(display)
}
