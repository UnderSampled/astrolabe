//! Continuous, lossless capture of a controller-verified X11 game window.
//! Sparse PNG checkpoints remain separate from this video stream.
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use serde_json::json;
use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const FRAME_RATE: u64 = 30;
const FRAME_INTERVAL_MICROS: u128 = 1_000_000 / FRAME_RATE as u128;

fn parse_epoch_micros(timestamp: &str) -> Result<u128> {
    let (seconds, fraction) = timestamp
        .split_once('.')
        .context("video timestamp lacks a decimal fraction")?;
    ensure!(
        !fraction.is_empty()
            && fraction.len() <= 6
            && fraction.bytes().all(|digit| digit.is_ascii_digit()),
        "video timestamp precision is unsupported"
    );
    let seconds = seconds.parse::<u128>()?;
    let micros = format!("{fraction:0<6}").parse::<u128>()?;
    seconds
        .checked_mul(1_000_000)
        .and_then(|value| value.checked_add(micros))
        .context("video timestamp overflow")
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ClockSample {
    pub monotonic_before_micros: u128,
    pub wall_unix_micros: u128,
    pub monotonic_after_micros: u128,
}

impl ClockSample {
    fn capture(origin: Instant) -> Result<Self> {
        let before = origin.elapsed().as_micros();
        let wall = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros();
        let after = origin.elapsed().as_micros();
        Ok(Self {
            monotonic_before_micros: before,
            wall_unix_micros: wall,
            monotonic_after_micros: after,
        })
    }

    fn elapsed_for_wall(self, unix_micros: u128) -> Result<u128> {
        let midpoint = (self.monotonic_before_micros + self.monotonic_after_micros) / 2;
        if unix_micros >= self.wall_unix_micros {
            Ok(midpoint + (unix_micros - self.wall_unix_micros))
        } else {
            midpoint
                .checked_sub(self.wall_unix_micros - unix_micros)
                .context("video frame precedes session launch")
        }
    }

    fn wall_minus_monotonic(self) -> i128 {
        self.wall_unix_micros as i128
            - ((self.monotonic_before_micros + self.monotonic_after_micros) / 2) as i128
    }
}

#[derive(Debug, Serialize)]
pub struct VideoSummary {
    pub path: String,
    pub frame_index_path: String,
    pub codec: &'static str,
    pub container: &'static str,
    pub capture: &'static str,
    pub requested_frames_per_second: u64,
    pub frames: u64,
    pub bytes: u64,
    pub first_frame_elapsed_micros: u128,
    pub last_frame_elapsed_micros: u128,
    pub width: u32,
    pub height: u32,
    pub timestamp_gaps: u64,
    pub clock_mapping_drift_micros: i128,
    pub clock_start: ClockSample,
    pub clock_stop_requested: ClockSample,
    pub clock_end: ClockSample,
    pub ffmpeg_exit_code: Option<i32>,
}

pub struct VideoCapture {
    child: Option<Child>,
    path: PathBuf,
    index_path: PathBuf,
    log_path: PathBuf,
    relative_path: String,
    relative_index_path: String,
    start_clock: ClockSample,
    origin: Instant,
}

impl VideoCapture {
    pub fn start(
        display: &str,
        window: u32,
        recording_dir: &Path,
        origin: Instant,
        log_path: &Path,
        segment: u32,
    ) -> Result<Self> {
        ensure!(
            !display.trim().is_empty() && window != 0,
            "video capture requires a verified X11 display and owned game window"
        );
        let directory = recording_dir.join("video");
        fs::create_dir_all(&directory)?;
        let relative_path = format!("video/{segment:06}.mkv");
        let relative_index_path = format!("video/{segment:06}.frames.jsonl");
        let path = recording_dir.join(&relative_path);
        let index_path = recording_dir.join(&relative_index_path);
        ensure!(
            !path.exists() && !index_path.exists(),
            "video output already exists"
        );
        let log = File::create_new(log_path)?;
        let start_clock = ClockSample::capture(origin)?;
        let child = Command::new("ffmpeg")
            .args(["-nostdin", "-hide_banner", "-loglevel", "info"])
            .args(["-f", "x11grab", "-framerate", "30", "-draw_mouse", "0"])
            .args(["-window_id", &window.to_string()])
            .args(["-use_wallclock_as_timestamps", "1"])
            .arg("-i")
            .arg(display)
            // Keep a playable, zero-based Matroska timeline while retaining
            // the raw wall-clock grab timestamps in the diagnostic log. The
            // latter are paired with media frames when the segment is sealed.
            .args(["-debug_ts", "-copyts", "-start_at_zero"])
            .args(["-avoid_negative_ts", "disabled"])
            .args(["-an", "-c:v", "ffv1", "-level", "3", "-pix_fmt", "bgr0"])
            .args(["-fps_mode", "passthrough", "-enc_time_base:v", "demux"])
            .args(["-f", "matroska"])
            .arg(&path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(log))
            .spawn()
            .context("start FFmpeg video capture")?;
        Ok(Self {
            child: Some(child),
            path,
            index_path,
            log_path: log_path.to_owned(),
            relative_path,
            relative_index_path,
            start_clock,
            origin,
        })
    }

    pub fn start_clock(&self) -> ClockSample {
        self.start_clock
    }

    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }

    pub fn bytes(&mut self) -> Result<u64> {
        if let Some(child) = self.child.as_mut()
            && let Some(status) = child.try_wait()?
        {
            anyhow::bail!("video encoder exited during capture: {status}");
        }
        match fs::metadata(&self.path) {
            Ok(metadata) => Ok(metadata.len()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
            Err(error) => Err(error.into()),
        }
    }

    pub fn finish(mut self) -> Result<VideoSummary> {
        let stop_requested = ClockSample::capture(self.origin)?;
        let mut child = self.child.take().context("video encoder already stopped")?;
        // FFmpeg finalizes Matroska when interrupted once. It returns 255 for
        // this normal SIGINT path on the pinned local build; probe the output.
        let signaled = unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGINT) } == 0;
        ensure!(
            signaled,
            "signal video encoder for clean finalization failed"
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill()?;
                child.wait()?;
                anyhow::bail!("video encoder did not finalize within ten seconds");
            }
            thread::sleep(Duration::from_millis(25));
        };
        ensure!(
            status.success() || status.code() == Some(255),
            "video encoder failed: {status}"
        );
        let end_clock = ClockSample::capture(self.origin)?;
        self.index_frames(stop_requested, end_clock, status.code())
    }

    fn index_frames(
        &self,
        stop_requested: ClockSample,
        end_clock: ClockSample,
        exit_code: Option<i32>,
    ) -> Result<VideoSummary> {
        let raw_log = File::open(&self.log_path)?;
        let mut raw_lines = BufReader::new(raw_log).lines();
        let mut child = Command::new("ffprobe")
            .args(["-v", "error", "-select_streams", "v:0"])
            .args([
                "-show_entries",
                "frame=best_effort_timestamp_time,width,height",
            ])
            .args(["-of", "csv=p=0"])
            .arg(&self.path)
            .stdout(Stdio::piped())
            .spawn()
            .context("index captured video")?;
        let stdout = child.stdout.take().context("ffprobe stdout missing")?;
        let mut index = File::create_new(&self.index_path)?;
        let mut frames = 0u64;
        let mut first = None;
        let mut last = None;
        let mut dimensions = None;
        let mut timestamp_gaps = 0u64;
        let mut first_raw = None;
        for line in BufReader::new(stdout).lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }
            let fields = line.split(',').collect::<Vec<_>>();
            ensure!(fields.len() == 3, "invalid video frame metadata");
            let media_micros = parse_epoch_micros(fields[0])?;
            let unix_micros = loop {
                let raw = raw_lines
                    .next()
                    .context("video frame has no raw capture timestamp")??;
                if !raw.contains("demuxer ->") || !raw.contains("type:video") {
                    continue;
                }
                let value = raw
                    .split("pkt_pts_time:")
                    .nth(1)
                    .and_then(|tail| tail.split_whitespace().next())
                    .context("raw video packet lacks wall-clock PTS")?;
                break parse_epoch_micros(value)?;
            };
            let raw_origin = *first_raw.get_or_insert(unix_micros);
            ensure!(
                unix_micros >= raw_origin
                    && unix_micros.abs_diff(raw_origin + media_micros) <= 2_000,
                "encoded media frame diverges from its raw wall-clock timestamp"
            );
            let elapsed_micros = self.start_clock.elapsed_for_wall(unix_micros)?;
            let width = fields[1].parse::<u32>()?;
            let height = fields[2].parse::<u32>()?;
            ensure!(width > 0 && height > 0, "empty video frame");
            if let Some(previous) = last {
                ensure!(
                    elapsed_micros > previous,
                    "video timestamps are not increasing"
                );
                if elapsed_micros - previous > FRAME_INTERVAL_MICROS * 3 / 2 {
                    timestamp_gaps += 1;
                }
            }
            if let Some((previous_width, previous_height)) = dimensions {
                ensure!(
                    (width, height) == (previous_width, previous_height),
                    "video dimensions changed without a new segment"
                );
            }
            dimensions = Some((width, height));
            first.get_or_insert(elapsed_micros);
            last = Some(elapsed_micros);
            writeln!(
                index,
                "{}",
                json!({
                    "index":frames,
                "path":self.relative_path,
                    "pts_media_micros":media_micros,
                    "pts_unix_micros":unix_micros,
                    "elapsed_micros":elapsed_micros,
                    "width":width,
                    "height":height,
                    "association":"presented_window_sample; game_render_update_unverified",
                })
            )?;
            frames += 1;
        }
        ensure!(
            child.wait()?.success(),
            "ffprobe failed to index captured video"
        );
        ensure!(frames > 0, "video capture contains no frames");
        index.sync_all()?;
        let (width, height) = dimensions.context("video dimensions missing")?;
        Ok(VideoSummary {
            path: self.relative_path.clone(),
            frame_index_path: self.relative_index_path.clone(),
            codec: "FFV1 level 3 bgr0 lossless",
            container: "Matroska",
            capture: "FFmpeg x11grab owned X11 window, 30 Hz wall-clock PTS",
            requested_frames_per_second: FRAME_RATE,
            frames,
            bytes: fs::metadata(&self.path)?.len(),
            first_frame_elapsed_micros: first.context("first video frame missing")?,
            last_frame_elapsed_micros: last.context("last video frame missing")?,
            width,
            height,
            timestamp_gaps,
            clock_mapping_drift_micros: end_clock.wall_minus_monotonic()
                - self.start_clock.wall_minus_monotonic(),
            clock_start: self.start_clock,
            clock_stop_requested: stop_requested,
            clock_end: end_clock,
            ffmpeg_exit_code: exit_code,
        })
    }
}

impl Drop for VideoCapture {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            // Best effort on capture failure. The session's failed manifest
            // and encoder log preserve why the video was not accepted.
            let _ = unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGINT) };
            let deadline = Instant::now() + Duration::from_secs(2);
            while Instant::now() < deadline {
                if child.try_wait().ok().flatten().is_some() {
                    return;
                }
                thread::sleep(Duration::from_millis(20));
            }
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_epoch_micros;

    #[test]
    fn frame_pts_keeps_epoch_microseconds_exact() -> anyhow::Result<()> {
        assert_eq!(
            parse_epoch_micros("1790514175.267000")?,
            1_790_514_175_267_000
        );
        assert_eq!(parse_epoch_micros("1.5")?, 1_500_000);
        assert!(parse_epoch_micros("1.1234567").is_err());
        Ok(())
    }
}
