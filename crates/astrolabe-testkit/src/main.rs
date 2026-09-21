use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
#[derive(Parser)]
#[command(about = "Optional Astrolabe native runtime and comparison tools")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    /// Write the exact provisional physical-input recording shared by adapters.
    RecordBoundary {
        #[arg(long)]
        output: PathBuf,
    },
    /// Inspect the supplied manor fixture using the native private-game runtime.
    InspectOriginal {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, hide = true)]
        fail_stopped_worker: bool,
    },
    /// Replay the shared 18-update physical-input recording in Hype.
    ReplayOriginal {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    #[command(hide = true)]
    OriginalWorker {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        host_display: String,
        #[arg(long, hide = true)]
        fail_stopped_worker: bool,
        #[arg(long, hide = true)]
        replay: bool,
    },
    /// Synthetic process used by lifecycle acceptance tests; no game data.
    #[command(hide = true)]
    Fixture {
        output: PathBuf,
        #[arg(long)]
        escaped: bool,
        #[arg(long)]
        leaf: bool,
        #[arg(long)]
        partial: bool,
    },
}
#[cfg(target_os = "linux")]
fn fixture(output: PathBuf, escaped: bool, leaf: bool, partial: bool) -> Result<()> {
    use std::{
        sync::atomic::{AtomicU64, Ordering},
        thread,
        time::Duration,
    };
    if escaped {
        use std::os::unix::process::CommandExt;
        let mut command = std::process::Command::new(std::env::current_exe()?);
        command.args([
            "fixture",
            output
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("output path"))?,
            "--leaf",
        ]);
        // SAFETY: setsid only changes the new child before exec.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() < 0 {
                    Err(std::io::Error::last_os_error())
                } else {
                    Ok(())
                }
            });
        }
        let mut child = command.spawn()?;
        // The test requires the child to outlive this parent. Bound failed setup.
        let start = std::time::Instant::now();
        while !output.exists() {
            if start.elapsed() > Duration::from_secs(2) {
                child.kill()?;
                child.wait()?;
                bail!("fixture child did not start");
            }
            thread::sleep(Duration::from_millis(1));
        }
        thread::sleep(Duration::from_millis(100));
        return Ok(());
    }
    if leaf {
        // SAFETY: SIG_IGN is the documented special handler value; only this fixture process is changed.
        unsafe {
            libc::signal(libc::SIGTERM, libc::SIG_IGN);
        }
    }
    let data = Box::leak(Box::new([
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
    ]));
    let address = data.as_ptr() as usize;
    let continue_path = output.with_extension("continue");
    thread::spawn(move || {
        let mut tick = 0;
        loop {
            tick += 1;
            data[1].store(tick, Ordering::SeqCst);
            data[0].store(1, Ordering::SeqCst);
            while partial && !continue_path.exists() {
                thread::sleep(Duration::from_millis(1));
            }
            thread::sleep(Duration::from_millis(20));
            data[2].store(tick, Ordering::SeqCst);
            data[0].store(2, Ordering::SeqCst);
            thread::sleep(Duration::from_millis(30));
        }
    });
    let tmp = output.with_extension("tmp");
    std::fs::write(
        &tmp,
        serde_json::to_vec(
            &serde_json::json!({"pid": std::process::id(), "address": address, "size": 24}),
        )?,
    )?;
    std::fs::rename(tmp, output)?;
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}
fn main() -> Result<()> {
    match Cli::parse().command {
        Commands::RecordBoundary { output } => {
            use std::io::Write;
            let prepared = astrolabe_testkit::replay::boundary_recording()?;
            let mut file = std::fs::File::create_new(&output)?;
            file.write_all(prepared.bytes())?;
            file.sync_all()?;
            println!(
                "{}",
                serde_json::json!({
                    "output": output,
                    "sha256": prepared.sha256(),
                    "updates": prepared.recording().updates.len(),
                })
            );
            Ok(())
        }
        Commands::InspectOriginal {
            config,
            output,
            fail_stopped_worker,
        } => {
            #[cfg(target_os = "linux")]
            return astrolabe_testkit::original::inspect(&config, &output, fail_stopped_worker);
            #[cfg(not(target_os = "linux"))]
            {
                let _ = (config, output, fail_stopped_worker);
                bail!("native runtime currently requires Linux");
            }
        }
        Commands::ReplayOriginal { config, output } => {
            #[cfg(target_os = "linux")]
            return astrolabe_testkit::original::replay(&config, &output);
            #[cfg(not(target_os = "linux"))]
            {
                let _ = (config, output);
                bail!("native runtime currently requires Linux");
            }
        }
        Commands::OriginalWorker {
            config,
            output,
            host_display,
            fail_stopped_worker,
            replay,
        } => {
            #[cfg(target_os = "linux")]
            return astrolabe_testkit::original::worker(
                &config,
                &output,
                &host_display,
                fail_stopped_worker,
                replay,
            );
            #[cfg(not(target_os = "linux"))]
            {
                let _ = (config, output, host_display, fail_stopped_worker, replay);
                bail!("native runtime currently requires Linux");
            }
        }
        Commands::Fixture {
            output,
            escaped,
            leaf,
            partial,
        } => {
            #[cfg(target_os = "linux")]
            return fixture(output, escaped, leaf, partial);
            #[cfg(not(target_os = "linux"))]
            {
                let _ = (output, escaped, leaf, partial);
                bail!("native runtime currently requires Linux");
            }
        }
    }
}
