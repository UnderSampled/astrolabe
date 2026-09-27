//! Explicit, versioned menu gates for exploratory Hype replay. These gates
//! observe captured pixels and sampled phase; they do not prove game input
//! consumption and do not change literal replay.
use crate::session::PlannedKey;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{fs::File, io::BufReader, path::Path, time::Duration};

const GATE_TIMEOUT_MICROS: u128 = 6_000_000;
const FRESH_FRAME_MICROS: u128 = 750_000;

#[derive(Debug, Clone, Copy)]
enum GateKind {
    TitleReady,
    LoadSelected,
    ExistingGames,
}

impl GateKind {
    fn label(self) -> &'static str {
        match self {
            Self::TitleReady => "title_ready",
            Self::LoadSelected => "load_game_selected",
            Self::ExistingGames => "existing_games_open",
        }
    }
}

#[derive(Debug)]
struct Observation {
    elapsed_micros: u128,
    frame: String,
    phase: Option<u64>,
    matched: bool,
    scores: Value,
}

#[derive(Debug)]
struct Gate {
    kind: GateKind,
    before_input: usize,
    wait_start: Option<u128>,
    source_target: u128,
    streak: usize,
    last: Option<Observation>,
}

pub struct NavigationEvent {
    pub kind: &'static str,
    pub data: Value,
}

pub struct GateDecision {
    pub ready: bool,
    pub failure: Option<String>,
    pub events: Vec<NavigationEvent>,
}

pub struct Navigation {
    gates: [Gate; 3],
    current: usize,
    delay_micros: u128,
}

impl Navigation {
    pub fn new(script: &[PlannedKey]) -> Result<Self> {
        // This profile is deliberately narrow. It cannot silently reinterpret
        // an unrelated session's first keyboard edges as menu navigation.
        ensure!(script.len() >= 6, "title/load profile needs the menu route");
        for (index, keysym, edge) in [
            (0, 65364, crate::replay::Edge::Press),
            (1, 65364, crate::replay::Edge::Release),
            (2, 65293, crate::replay::Edge::Press),
            (3, 65293, crate::replay::Edge::Release),
            (4, 65293, crate::replay::Edge::Press),
            (5, 65293, crate::replay::Edge::Release),
        ] {
            ensure!(
                script[index].expected_keysym == Some(keysym) && script[index].edge == edge,
                "title/load profile requires Down, Enter, Enter as the first three key presses"
            );
        }
        let gate = |kind, before_input| Gate {
            kind,
            before_input,
            wait_start: None,
            source_target: script[before_input].at_micros,
            streak: 0,
            last: None,
        };
        Ok(Self {
            gates: [
                gate(GateKind::TitleReady, 0),
                gate(GateKind::LoadSelected, 2),
                gate(GateKind::ExistingGames, 4),
            ],
            current: 0,
            delay_micros: 0,
        })
    }

    pub fn profile(&self) -> &'static str {
        "hype-title-load-v1"
    }

    pub fn delay_micros(&self) -> u128 {
        self.delay_micros
    }

    pub fn delay(&self) -> Duration {
        Duration::from_micros(u64::try_from(self.delay_micros).unwrap_or(u64::MAX))
    }

    pub fn before_input(
        &mut self,
        index: usize,
        now: u128,
        phase: Option<u64>,
    ) -> Result<GateDecision> {
        let Some(gate) = self.gates.get_mut(self.current) else {
            return Ok(GateDecision {
                ready: true,
                failure: None,
                events: vec![],
            });
        };
        if gate.before_input != index {
            return Ok(GateDecision {
                ready: true,
                failure: None,
                events: vec![],
            });
        }
        let target = gate.source_target + self.delay_micros;
        let mut events = Vec::new();
        if gate.wait_start.is_none() {
            gate.wait_start = Some(now);
            events.push(NavigationEvent {
                kind: "navigation_wait_start",
                data: json!({"profile":"hype-title-load-v1","gate":gate.kind.label(),
                    "source_target_micros":gate.source_target,
                    "adapted_target_micros":target,"phase":phase}),
            });
        }
        let observed = gate.last.as_ref().filter(|observation| {
            observation.matched
                && observation.phase == Some(38)
                && now.saturating_sub(observation.elapsed_micros) <= FRESH_FRAME_MICROS
        });
        if gate.streak >= 2
            && phase == Some(38)
            && let Some(observation) = observed
        {
            let extra = now.saturating_sub(target);
            self.delay_micros += extra;
            events.push(NavigationEvent {
                kind: "navigation_gate_observed",
                data: json!({"profile":"hype-title-load-v1","gate":gate.kind.label(),
                    "frame":observation.frame,"frame_elapsed_micros":observation.elapsed_micros,
                    "phase":phase,"matched":true,"consecutive_matches":gate.streak,
                    "scores":observation.scores}),
            });
            events.push(NavigationEvent {
                kind: "navigation_wait_end",
                data: json!({"profile":"hype-title-load-v1","gate":gate.kind.label(),
                    "source_target_micros":gate.source_target,
                    "adapted_target_micros":target,
                    "wait_start_micros":gate.wait_start,
                    "wait_end_micros":now,"added_delay_micros":extra,
                    "total_added_delay_micros":self.delay_micros,
                    "evidence_frame":observation.frame,
                    "evidence_scores":observation.scores,
                    "phase":phase,"consecutive_matches":gate.streak}),
            });
            self.current += 1;
            return Ok(GateDecision {
                ready: true,
                failure: None,
                events,
            });
        }
        if now.saturating_sub(target) >= GATE_TIMEOUT_MICROS {
            let failure = format!("navigation gate {} timed out", gate.kind.label());
            events.push(NavigationEvent {
                kind: "navigation_wait_timeout",
                data: json!({"profile":"hype-title-load-v1","gate":gate.kind.label(),
                    "source_target_micros":gate.source_target,
                    "adapted_target_micros":target,
                    "wait_start_micros":gate.wait_start,"timeout_micros":now,
                    "phase":phase,"last_frame":gate.last.as_ref().map(|last| &last.frame),
                    "last_scores":gate.last.as_ref().map(|last| &last.scores)}),
            });
            return Ok(GateDecision {
                ready: false,
                failure: Some(failure),
                events,
            });
        }
        Ok(GateDecision {
            ready: false,
            failure: None,
            events,
        })
    }

    pub fn observe_frame(
        &mut self,
        path: &Path,
        relative: &str,
        elapsed_micros: u128,
        phase: Option<u64>,
    ) -> Result<Vec<NavigationEvent>> {
        let Some(gate) = self.gates.get_mut(self.current) else {
            return Ok(vec![]);
        };
        let scores = frame_scores(path)?;
        let matched = phase == Some(38) && scores.matches(gate.kind);
        gate.streak = if matched { gate.streak + 1 } else { 0 };
        let data = scores.as_json();
        gate.last = Some(Observation {
            elapsed_micros,
            frame: relative.to_owned(),
            phase,
            matched,
            scores: data.clone(),
        });
        if gate.wait_start.is_some() {
            Ok(vec![NavigationEvent {
                kind: "navigation_gate_observation",
                data: json!({"profile":"hype-title-load-v1","gate":gate.kind.label(),
                    "frame":relative,"frame_elapsed_micros":elapsed_micros,
                    "phase":phase,"matched":matched,"consecutive_matches":gate.streak,
                    "scores":data}),
            }])
        } else {
            Ok(vec![])
        }
    }
}

#[derive(Debug)]
struct FrameScores {
    width: u32,
    height: u32,
    new_luminance: f64,
    load_luminance: f64,
    icon_fraction: f64,
    logo_blue_fraction: f64,
    row_gold_fraction: f64,
    panel_orange_fraction: f64,
    header_bright_fraction: f64,
    title_identity: bool,
    existing_identity: bool,
}

impl FrameScores {
    fn matches(&self, kind: GateKind) -> bool {
        match kind {
            GateKind::TitleReady => {
                self.title_identity && self.new_luminance > 0.35 && self.load_luminance < 0.35
            }
            GateKind::LoadSelected => {
                self.title_identity && self.load_luminance > 0.35 && self.new_luminance < 0.35
            }
            GateKind::ExistingGames => self.existing_identity,
        }
    }

    fn as_json(&self) -> Value {
        json!({"width":self.width,"height":self.height,
            "new_luminance":self.new_luminance,"load_luminance":self.load_luminance,
            "icon_fraction":self.icon_fraction,
            "logo_blue_fraction":self.logo_blue_fraction,
            "row_gold_fraction":self.row_gold_fraction,
            "panel_orange_fraction":self.panel_orange_fraction,
            "header_bright_fraction":self.header_bright_fraction,
            "title_identity":self.title_identity,"existing_identity":self.existing_identity})
    }
}

fn frame_scores(path: &Path) -> Result<FrameScores> {
    let mut decoder = png::Decoder::new(BufReader::new(File::open(path)?));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let mut bytes = vec![0; reader.output_buffer_size().context("PNG output buffer")?];
    let info = reader.next_frame(&mut bytes)?;
    ensure!(
        info.color_type == png::ColorType::Rgb,
        "navigation frame is not RGB"
    );
    let bytes = &bytes[..info.buffer_size()];
    let width = info.width;
    let height = info.height;
    let roi = |x0: f64,
               y0: f64,
               x1: f64,
               y1: f64,
               predicate: &dyn Fn(f64, f64, f64, f64) -> f64|
     -> f64 {
        let mut sum = 0.0;
        let mut count = 0u64;
        for y in ((height as f64 * y0) as u32)..((height as f64 * y1) as u32) {
            for x in ((width as f64 * x0) as u32)..((width as f64 * x1) as u32) {
                let at = ((y * width + x) * 3) as usize;
                let r = f64::from(bytes[at]) / 255.0;
                let g = f64::from(bytes[at + 1]) / 255.0;
                let b = f64::from(bytes[at + 2]) / 255.0;
                let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                sum += predicate(r, g, b, lum);
                count += 1;
            }
        }
        if count == 0 { 0.0 } else { sum / count as f64 }
    };
    let luminance = |_: f64, _: f64, _: f64, lum: f64| lum;
    let new_luminance = roi(0.50, 0.43, 0.78, 0.51, &luminance);
    let load_luminance = roi(0.50, 0.51, 0.78, 0.60, &luminance);
    let icon_fraction = roi(0.36, 0.28, 0.405, 0.74, &|r, g, b, lum| {
        if lum > 0.58 && r.max(g).max(b) - r.min(g).min(b) < 0.4 {
            1.0
        } else {
            0.0
        }
    });
    let logo_blue_fraction = roi(0.12, 0.05, 0.84, 0.32, &|r, g, b, _| {
        if b > 0.35 && b > 1.18 * r && b > 1.08 * g && r.max(g).max(b) - r.min(g).min(b) > 0.15 {
            1.0
        } else {
            0.0
        }
    });
    let gold = |r: f64, g: f64, b: f64, _: f64| {
        if r > 0.55 && g > 0.25 && r > 0.9 * g && b < 0.8 * r {
            1.0
        } else {
            0.0
        }
    };
    let row_gold_fraction =
        roi(0.50, 0.43, 0.78, 0.51, &gold).max(roi(0.50, 0.51, 0.78, 0.60, &gold));
    let panel_orange_fraction = roi(0.39, 0.20, 0.96, 0.91, &|r, g, b, _| {
        if r > 0.45 && g > 0.12 && r > 1.15 * g && g > 1.2 * b && r - b > 0.18 {
            1.0
        } else {
            0.0
        }
    });
    let header_bright_fraction = roi(0.30, 0.09, 0.98, 0.29, &|_, _, _, lum| {
        if lum > 0.55 { 1.0 } else { 0.0 }
    });
    // Joint cues were calibrated on all 382 captured source/replay frames,
    // including both window sizes, loading frames, gameplay and save overlays.
    let title_identity = logo_blue_fraction > 0.15 && row_gold_fraction > 0.25;
    let existing_identity =
        panel_orange_fraction > 0.70 && icon_fraction > 0.18 && header_bright_fraction > 0.12;
    Ok(FrameScores {
        width,
        height,
        new_luminance,
        load_luminance,
        icon_fraction,
        logo_blue_fraction,
        row_gold_fraction,
        panel_orange_fraction,
        header_bright_fraction,
        title_identity,
        existing_identity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::Edge;
    use std::io::BufWriter;

    fn menu_script() -> Vec<PlannedKey> {
        [
            (10_000_000, 65364, Edge::Press),
            (10_060_000, 65364, Edge::Release),
            (12_000_000, 65293, Edge::Press),
            (12_100_000, 65293, Edge::Release),
            (16_000_000, 65293, Edge::Press),
            (16_100_000, 65293, Edge::Release),
        ]
        .into_iter()
        .map(|(at_micros, expected_keysym, edge)| PlannedKey {
            at_micros,
            code: Some(36),
            key: None,
            expected_keysym: Some(expected_keysym),
            edge,
            repeat: false,
        })
        .collect()
    }

    #[test]
    fn menu_route_is_explicit() {
        assert!(Navigation::new(&menu_script()).is_ok());
        let mut unrelated = menu_script();
        unrelated[0].expected_keysym = Some(65362);
        assert!(Navigation::new(&unrelated).is_err());
    }

    #[test]
    fn gate_requires_stable_fresh_evidence_and_reports_delay() {
        let mut navigation = Navigation::new(&menu_script()).unwrap();
        let first = navigation.before_input(0, 10_000_000, Some(38)).unwrap();
        assert!(!first.ready);
        assert_eq!(first.events[0].kind, "navigation_wait_start");
        navigation.gates[0].streak = 1;
        navigation.gates[0].last = Some(Observation {
            elapsed_micros: 10_100_000,
            frame: "frames/000001.png".into(),
            phase: Some(38),
            matched: true,
            scores: json!({}),
        });
        assert!(
            !navigation
                .before_input(0, 10_150_000, Some(38))
                .unwrap()
                .ready
        );
        navigation.gates[0].streak = 2;
        let accepted = navigation.before_input(0, 10_200_000, Some(38)).unwrap();
        assert!(accepted.ready);
        assert_eq!(accepted.events[0].kind, "navigation_gate_observed");
        assert_eq!(accepted.events[1].kind, "navigation_wait_end");
        assert_eq!(navigation.delay_micros(), 200_000);
        assert!(
            navigation
                .before_input(1, 10_260_000, Some(38))
                .unwrap()
                .ready
        );
        assert!(
            !navigation
                .before_input(2, 12_200_000, Some(38))
                .unwrap()
                .ready
        );
    }

    #[test]
    fn gate_times_out_without_rewriting_later_inputs() {
        let mut navigation = Navigation::new(&menu_script()).unwrap();
        let decision = navigation.before_input(0, 16_000_000, Some(38)).unwrap();
        assert!(!decision.ready);
        assert!(decision.failure.is_some());
        assert!(
            decision
                .events
                .iter()
                .any(|event| event.kind == "navigation_wait_timeout")
        );
        assert_eq!(navigation.delay_micros(), 0);
    }

    fn synthetic_frame(path: &Path, width: u32, height: u32, screen: &str) {
        let mut pixels = vec![0u8; (width * height * 3) as usize];
        let mut fill = |roi: (f64, f64, f64, f64), color: [u8; 3]| {
            for y in ((height as f64 * roi.1) as u32)..((height as f64 * roi.3) as u32) {
                for x in ((width as f64 * roi.0) as u32)..((width as f64 * roi.2) as u32) {
                    let at = ((y * width + x) * 3) as usize;
                    pixels[at..at + 3].copy_from_slice(&color);
                }
            }
        };
        match screen {
            "new" | "load" => {
                fill((0.12, 0.05, 0.84, 0.32), [0, 0, 255]);
                fill(
                    (0.50, 0.43, 0.78, 0.51),
                    if screen == "new" {
                        [255, 230, 0]
                    } else {
                        [145, 50, 0]
                    },
                );
                fill(
                    (0.50, 0.51, 0.78, 0.60),
                    if screen == "load" {
                        [255, 230, 0]
                    } else {
                        [145, 50, 0]
                    },
                );
            }
            "existing" | "save_overlay" => {
                fill((0.39, 0.20, 0.96, 0.91), [230, 120, 30]);
                fill((0.36, 0.28, 0.405, 0.74), [220, 210, 200]);
                if screen == "existing" {
                    fill((0.30, 0.09, 0.98, 0.29), [230, 220, 200]);
                }
            }
            "white" => fill((0.0, 0.0, 1.0, 1.0), [255, 255, 255]),
            _ => unreachable!(),
        }
        let mut encoder =
            png::Encoder::new(BufWriter::new(File::create(path).unwrap()), width, height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&pixels)
            .unwrap();
    }

    #[test]
    fn visual_gates_normalize_resolution_and_reject_other_screens() {
        let temp = tempfile::tempdir().unwrap();
        for (width, height) in [(780, 600), (1024, 768)] {
            let path = temp.path().join(format!("{width}.png"));
            for screen in ["new", "load", "existing", "save_overlay", "white"] {
                synthetic_frame(&path, width, height, screen);
                let scores = frame_scores(&path).unwrap();
                assert_eq!(
                    scores.matches(GateKind::TitleReady),
                    screen == "new",
                    "{width} {screen}"
                );
                assert_eq!(
                    scores.matches(GateKind::LoadSelected),
                    screen == "load",
                    "{width} {screen}"
                );
                assert_eq!(
                    scores.matches(GateKind::ExistingGames),
                    screen == "existing",
                    "{width} {screen}"
                );
            }
        }
    }
}
