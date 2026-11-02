//! Private PulseAudio-compatible graph for one Orrery game run.
//!
//! The Wine client sees only this socket and its null sink. No host monitor,
//! microphone or host monitor is involved. Interactive recording may forward
//! this private monitor to the host default playback sink through a PCM pipe.
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const SINK: &str = "orrery_game";

pub struct AudioRoute {
    _runtime_guard: tempfile::TempDir,
    runtime: PathBuf,
    server: String,
    pipewire: Child,
    pulse: Child,
    manager: Child,
    capture: Option<AudioCapture>,
    playback: Option<AudioPlayback>,
}

struct AudioPlayback {
    source: Child,
    speaker: Option<Child>,
}

impl Drop for AudioPlayback {
    fn drop(&mut self) {
        if let Some(speaker) = &mut self.speaker {
            stop(speaker);
        }
        stop(&mut self.source);
    }
}

pub struct AudioCapture {
    child: Child,
    path: PathBuf,
    start: Instant,
}

fn stop(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn private_command(program: &str, runtime: &Path, server: &str) -> Command {
    let mut command = Command::new(program);
    command
        .env("XDG_RUNTIME_DIR", runtime)
        .env("PIPEWIRE_RUNTIME_DIR", runtime)
        .env("PULSE_SERVER", server)
        .env("XDG_CONFIG_HOME", runtime)
        .env("XDG_STATE_HOME", runtime)
        .env("DBUS_SYSTEM_BUS_ADDRESS", "unix:path=/nonexistent")
        .env_remove("DBUS_SESSION_BUS_ADDRESS");
    command
}

/// The private graph holds only a null sink. Without this its session manager
/// also runs the ALSA and Bluetooth monitors against the real hardware, which
/// takes the desktop's speakers and Bluetooth endpoints away from it.
const WIREPLUMBER_NO_HARDWARE: &str = "wireplumber.profiles = {\n  main = {\n    hardware.audio = disabled\n    hardware.bluetooth = disabled\n    hardware.video-capture = disabled\n  }\n}\n";

impl AudioRoute {
    pub fn start(run: &Path, capture_path: Option<&Path>) -> Result<Self> {
        // AF_UNIX socket paths have a 108-byte limit; run evidence may live
        // much deeper than that. TempDir also owns removal after every exit.
        let runtime_guard = tempfile::Builder::new()
            .prefix("orrery-audio-")
            .tempdir_in("/tmp")?;
        let runtime = runtime_guard.path().to_path_buf();
        let server = format!("unix:{}/pulse/native", runtime.display());
        let pipewire_log = File::create(run.join("pipewire.log"))?;
        let pulse_log = File::create(run.join("pipewire-pulse.log"))?;
        let manager_log = File::create(run.join("wireplumber.log"))?;
        let pipewire = private_command("pipewire", &runtime, &server)
            .arg("-c")
            .arg("pipewire.conf")
            .stdout(Stdio::from(pipewire_log.try_clone()?))
            .stderr(Stdio::from(pipewire_log))
            .spawn()
            .context("start private PipeWire core")?;
        let pulse = match private_command("pipewire-pulse", &runtime, &server)
            .stdout(Stdio::from(pulse_log.try_clone()?))
            .stderr(Stdio::from(pulse_log))
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                let mut pipewire = pipewire;
                stop(&mut pipewire);
                return Err(error).context("start private PulseAudio-compatible server");
            }
        };
        let manager_config = runtime.join("wireplumber/wireplumber.conf.d");
        fs::create_dir_all(&manager_config)?;
        fs::write(manager_config.join("orrery.conf"), WIREPLUMBER_NO_HARDWARE)?;
        let manager = match private_command("wireplumber", &runtime, &server)
            .stdout(Stdio::from(manager_log.try_clone()?))
            .stderr(Stdio::from(manager_log))
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                let mut pulse = pulse;
                let mut pipewire = pipewire;
                stop(&mut pulse);
                stop(&mut pipewire);
                return Err(error).context("start private audio session manager");
            }
        };
        let mut route = Self {
            _runtime_guard: runtime_guard,
            runtime,
            server,
            pipewire,
            pulse,
            manager,
            capture: None,
            playback: None,
        };
        let ready_by = Instant::now() + Duration::from_secs(5);
        loop {
            if route.pactl(&["info"]).is_ok() {
                break;
            }
            ensure!(
                route.pipewire.try_wait()?.is_none()
                    && route.pulse.try_wait()?.is_none()
                    && route.manager.try_wait()?.is_none(),
                "private audio server exited; inspect pipewire logs in {}",
                run.display()
            );
            ensure!(
                Instant::now() < ready_by,
                "private audio server did not become ready; inspect pipewire logs in {}",
                run.display()
            );
            thread::sleep(Duration::from_millis(40));
        }
        route.pactl(&[
            "load-module",
            "module-null-sink",
            &format!("sink_name={SINK}"),
        ])?;
        if let Some(path) = capture_path {
            route.start_capture(path)?;
        }
        Ok(route)
    }

    fn pactl(&self, args: &[&str]) -> Result<String> {
        let output = private_command("pactl", &self.runtime, &self.server)
            .args(args)
            .output()
            .context("run pactl against private server")?;
        ensure!(
            output.status.success(),
            "private audio command failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        Ok(String::from_utf8(output.stdout)?)
    }

    pub fn apply_to(&self, command: &mut Command) {
        command
            .env("XDG_RUNTIME_DIR", &self.runtime)
            .env("PIPEWIRE_RUNTIME_DIR", &self.runtime)
            .env("PULSE_SERVER", &self.server)
            .env("PULSE_SINK", SINK);
    }

    /// Read only the game monitor on the private server; the host connection
    /// is playback-only and receives PCM over an owned anonymous pipe.
    pub fn start_playback(&mut self, run: &Path) -> Result<()> {
        ensure!(
            self.playback.is_none(),
            "game audio playback already started"
        );
        // A desktop with no real output would swallow the game silently.
        let default_sink = Command::new("pactl").arg("get-default-sink").output()?;
        ensure!(
            default_sink.status.success()
                && !String::from_utf8_lossy(&default_sink.stdout).contains("auto_null"),
            "the desktop has no audio output; fix it before recording"
        );
        let source_log = File::create_new(run.join("audio-playback-source.log"))?;
        let speaker_log = File::create_new(run.join("audio-playback-speaker.log"))?;
        let source = private_command("parec", &self.runtime, &self.server)
            .args([
                "--record",
                "--raw",
                "--format=s16le",
                "--rate=48000",
                "--channels=2",
                "--device=orrery_game.monitor",
                "--latency-msec=20",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(source_log))
            .spawn()
            .context("start private game-only audio playback source")?;
        let mut playback = AudioPlayback {
            source,
            speaker: None,
        };
        let pcm = playback
            .source
            .stdout
            .take()
            .context("game audio PCM pipe")?;
        playback.speaker = Some(
            Command::new("pacat")
                .args([
                    "--playback",
                    "--raw",
                    "--format=s16le",
                    "--rate=48000",
                    "--channels=2",
                    "--latency-msec=40",
                    "--client-name=Orrery",
                    "--stream-name=Hype interactive game audio",
                ])
                .stdin(Stdio::from(pcm))
                .stdout(Stdio::null())
                .stderr(Stdio::from(speaker_log))
                .spawn()
                .context("start game audio playback on the user's desktop")?,
        );
        self.playback = Some(playback);
        let ready_by = Instant::now() + Duration::from_secs(3);
        loop {
            self.check_playback()?;
            // Full recording already has one private monitor reader. Wait for
            // the additional forwarder to attach before launching the game.
            if self
                .pactl(&["list", "short", "source-outputs"])?
                .lines()
                .count()
                > usize::from(self.capture.is_some())
            {
                break;
            }
            ensure!(
                Instant::now() < ready_by,
                "game audio playback did not attach to the private monitor"
            );
            thread::sleep(Duration::from_millis(25));
        }
        Ok(())
    }

    pub fn check_playback(&mut self) -> Result<()> {
        if let Some(playback) = &mut self.playback {
            ensure!(
                playback.source.try_wait()?.is_none(),
                "private game audio playback source exited"
            );
            ensure!(
                playback
                    .speaker
                    .as_mut()
                    .context("game audio playback sink")?
                    .try_wait()?
                    .is_none(),
                "desktop game audio playback exited; inspect audio-playback-speaker.log"
            );
        }
        Ok(())
    }

    fn start_capture(&mut self, path: &Path) -> Result<()> {
        let log = File::create(path.with_extension("audio.log"))?;
        let start = Instant::now();
        let child = private_command("ffmpeg", &self.runtime, &self.server)
            .args([
                "-nostdin",
                "-hide_banner",
                "-loglevel",
                "warning",
                "-f",
                "pulse",
                "-sample_rate",
                "48000",
                "-channels",
                "2",
                "-i",
                "orrery_game.monitor",
                "-c:a",
                "flac",
                "-ar",
                "48000",
                "-ac",
                "2",
                "-y",
            ])
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::from(log))
            .spawn()
            .context("start private game audio capture")?;
        self.capture = Some(AudioCapture {
            child,
            path: path.to_owned(),
            start,
        });
        // Wait until FFmpeg is actually attached to the private monitor so
        // startup sound cannot outrun an encoder that merely spawned.
        let ready_by = Instant::now() + Duration::from_secs(3);
        loop {
            ensure!(
                self.capture.as_mut().unwrap().child.try_wait()?.is_none(),
                "game audio encoder exited before launch"
            );
            if self
                .pactl(&["list", "short", "source-outputs"])
                .is_ok_and(|outputs| !outputs.trim().is_empty())
            {
                break;
            }
            ensure!(
                Instant::now() < ready_by,
                "game audio encoder did not attach to the private monitor"
            );
            thread::sleep(Duration::from_millis(25));
        }
        Ok(())
    }

    pub fn capture_start(&self) -> Option<Instant> {
        self.capture.as_ref().map(|capture| capture.start)
    }

    pub fn finish_capture(&mut self, launch: Instant) -> Result<Option<Value>> {
        let Some(mut capture) = self.capture.take() else {
            return Ok(None);
        };
        // SIGINT lets FFmpeg write the Matroska index and FLAC trailer.
        // SAFETY: child.id() is the owned FFmpeg process.
        unsafe { libc::kill(capture.child.id() as i32, libc::SIGINT) };
        let deadline = Instant::now() + Duration::from_secs(5);
        let status = loop {
            if let Some(status) = capture.child.try_wait()? {
                break status;
            }
            if Instant::now() >= deadline {
                stop(&mut capture.child);
                bail!("game audio encoder did not finalize in five seconds");
            }
            thread::sleep(Duration::from_millis(25));
        };
        // FFmpeg reports 255 for a clean SIGINT after writing the container.
        ensure!(
            status.success() || status.code() == Some(255),
            "game audio encoder failed: {status}"
        );
        ensure!(
            fs::metadata(&capture.path)?.len() > 1024,
            "game audio capture contains no encoded samples"
        );
        let probe = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_streams",
                "-show_format",
                "-of",
                "json",
            ])
            .arg(&capture.path)
            .output()
            .context("inspect captured game audio")?;
        ensure!(probe.status.success(), "captured game audio is unreadable");
        let value: Value = serde_json::from_slice(&probe.stdout)?;
        let stream = value["streams"]
            .as_array()
            .and_then(|streams| {
                streams
                    .iter()
                    .find(|stream| stream["codec_type"] == "audio")
            })
            .context("captured game audio has no audio stream")?;
        ensure!(
            stream["codec_name"] == "flac",
            "game audio codec is not FLAC"
        );
        let duration = value["format"]["duration"]
            .as_str()
            .unwrap_or("0")
            .parse::<f64>()?;
        ensure!(duration > 0.0, "game audio has zero duration");
        let level = Command::new("ffmpeg")
            .args(["-nostdin", "-hide_banner", "-i"])
            .arg(&capture.path)
            .args(["-af", "volumedetect", "-f", "null", "-"])
            .output()
            .context("measure captured game audio signal")?;
        ensure!(
            level.status.success(),
            "captured game audio cannot be decoded"
        );
        let level_log = String::from_utf8_lossy(&level.stderr);
        let max_volume = level_log
            .lines()
            .find_map(|line| {
                line.split_once("max_volume: ")
                    .map(|(_, value)| value.trim())
            })
            .context("captured game audio signal level is unavailable")?;
        let signal_detected = !max_volume.starts_with("-inf");
        let offset = if capture.start <= launch {
            -((launch - capture.start).as_micros().min(i128::MAX as u128) as i128)
        } else {
            (capture.start - launch).as_micros().min(i128::MAX as u128) as i128
        };
        Ok(Some(json!({
            "path":"audio.mka", "capture":"private_pipewire_pulse_null_sink_monitor",
            "codec":"flac", "sample_rate":stream["sample_rate"],
            "channels":stream["channels"], "duration_seconds":duration,
            "signal_detected":signal_detected, "max_volume":max_volume,
            "capture_start_from_launch_micros":offset,
            "timing_uncertainty":"PulseAudio buffering and FFmpeg startup are not sample-clock calibrated; capture was armed before Wine launch",
            "stream_provenance":"monitor of Orrery's private null sink; only this run's Wine client receives the server socket",
            "desktop_playback_requested":self.playback.is_some(),
        })))
    }
}

impl Drop for AudioRoute {
    fn drop(&mut self) {
        self.playback.take();
        if let Some(capture) = &mut self.capture {
            stop(&mut capture.child);
        }
        stop(&mut self.pulse);
        stop(&mut self.manager);
        stop(&mut self.pipewire);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_client_environment_is_scoped() {
        let command = private_command(
            "pactl",
            Path::new("/tmp/orrery-example"),
            "unix:/tmp/orrery-example/pulse/native",
        );
        let vars = command.get_envs().collect::<Vec<_>>();
        assert!(vars.iter().any(|(key, value)| {
            *key == "PULSE_SERVER"
                && value.is_some_and(|value| value == "unix:/tmp/orrery-example/pulse/native")
        }));
        assert!(vars.iter().any(|(key, value)| {
            *key == "XDG_RUNTIME_DIR" && value.is_some_and(|value| value == "/tmp/orrery-example")
        }));
    }

    #[test]
    #[ignore = "requires private Unix sockets and local PipeWire/FFmpeg; selected for audio integration"]
    fn private_tone_is_captured_and_resources_are_removed() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let recording = temp.path().join("recording");
        fs::create_dir(&recording)?;
        let audio_path = recording.join("audio.mka");
        let mut route = AudioRoute::start(temp.path(), Some(&audio_path))?;
        let launch = Instant::now();
        let mut tone = Command::new("ffmpeg");
        route.apply_to(&mut tone);
        let tone = tone
            .args([
                "-nostdin",
                "-hide_banner",
                "-loglevel",
                "error",
                "-re",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=1",
                "-f",
                "pulse",
                "-device",
                SINK,
                "-y",
                "tone",
            ])
            .status()?;
        ensure!(
            tone.success(),
            "synthetic playback to private sink failed: {}\n{}",
            fs::read_to_string(temp.path().join("pipewire.log")).unwrap_or_default(),
            fs::read_to_string(temp.path().join("pipewire-pulse.log")).unwrap_or_default(),
        );
        thread::sleep(Duration::from_millis(200));
        let summary = route.finish_capture(launch)?.context("capture missing")?;
        assert_eq!(summary["codec"], "flac");
        assert_eq!(summary["sample_rate"], "48000");
        assert_eq!(summary["channels"], 2);
        assert_eq!(summary["signal_detected"], true);
        assert!(
            summary["capture_start_from_launch_micros"]
                .as_i64()
                .unwrap()
                < 0
        );
        assert!(summary["duration_seconds"].as_f64().unwrap() >= 0.8);
        let signal = Command::new("ffmpeg")
            .args(["-hide_banner", "-i"])
            .arg(&audio_path)
            .args(["-af", "volumedetect", "-f", "null", "-"])
            .output()?;
        ensure!(signal.status.success(), "captured FLAC is not decodable");
        let stderr = String::from_utf8_lossy(&signal.stderr);
        ensure!(
            stderr.contains("max_volume:") && !stderr.contains("max_volume: -inf"),
            "private capture contains only silence"
        );
        let runtime = route.runtime.clone();
        drop(route);
        assert!(!runtime.exists());
        Ok(())
    }

    #[test]
    #[ignore = "requires private Unix sockets and local PipeWire/FFmpeg; selected for audio integration"]
    fn failed_capture_start_reaps_private_server() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let missing_parent = temp.path().join("missing/audio.mka");
        let before = fs::read_dir("/tmp")?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("orrery-audio-")
            })
            .count();
        assert!(AudioRoute::start(temp.path(), Some(&missing_parent)).is_err());
        let after = fs::read_dir("/tmp")?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("orrery-audio-")
            })
            .count();
        assert_eq!(before, after);
        Ok(())
    }
}
