//! Localized dialogue meaning; pointers and source allocation layout stay at import.
use super::{
    AssetPath, AssetRef, ai::BehaviorRef, audio::SoundEventLink, character::Perso, io::Content,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dialogue {
    pub path: AssetPath,
    pub name: String,
    pub locale: String,
    pub box_style: u8,
    pub owner: AssetRef<Perso>,
    pub lines: Vec<DialogueLine>,
    pub voice: Option<SoundEventLink>,
    pub completion: Option<BehaviorRef>,
    pub presentation: Option<AssetRef<DialoguePresentation>>,
    /// Choices of a question item (DLG answers with text, 0056dd50). Empty for
    /// the ordinary single-answer item, whose callback is `completion`.
    #[serde(default)]
    pub answers: Vec<DialogueAnswer>,
}

/// One drawn choice: its own text rows, the owner's normal behavior stored as
/// the pending callback when it is selected (0056e3b0), the speech then played
/// on MainActor (answer+0x3c), and AnswerDelete (answer+0x08).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueAnswer {
    pub lines: Vec<DialogueLine>,
    pub completion: Option<BehaviorRef>,
    pub voice: Option<SoundEventLink>,
    pub one_shot: bool,
    /// Answer+0x48: gold added (>= 0) or spent (< 0) on selection (0056e0e0).
    pub gold: i32,
    /// AnswerNext: the item drawing continues with (0056c700), by name.
    pub next: Option<String>,
    /// AnswerBad: items shown instead when the cost check fails, by failure
    /// code (0056e2c0 returns 1 when Hype lacks the gold).
    pub shortfall: Vec<(u8, String)>,
}
/// Shared localized UI inputs. Authored layout and native controls remain in Godot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialoguePresentation {
    pub path: AssetPath,
    pub font_name: String,
    pub atlas: DialogueTexture,
    pub glyphs: Vec<DialogueGlyph>,
    pub tracking: f32,
    pub normal_color: [f32; 4],
    pub emphasis_color: [f32; 4],
    /// Source order: corner, horizontal border, center, vertical side.
    pub frame: [DialogueTexture; 4],
    /// The shared red_arrow stencil used to indicate a completed nonfinal page.
    pub continuation: DialogueTexture,
    /// Third DLG root font (snapshot word 16): the four offset copies drawn
    /// under a selected answer (0056dd50).
    pub selection_color: [f32; 4],
    /// Stencil 5 (`DLG_bullet`), drawn beside each selected answer row.
    pub bullet: DialogueTexture,
    /// `Game.fix^SoundEffects:Dialog` (`DAT_00723b2c`, the eighth DLG Fix GPT
    /// root, 0056bbe0): entry 0 after a paid answer (0056e2c0), entry 2 on an
    /// answer scroll (0056f3a0).
    #[serde(default)]
    pub cash_sound: Option<SoundEventLink>,
    #[serde(default)]
    pub scroll_sound: Option<SoundEventLink>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueTexture {
    pub texture: AssetPath,
    pub size: [u32; 2],
    pub color_key: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueGlyph {
    pub character: char,
    pub width: f32,
    pub half_height: f32,
    pub vertical_offset: f32,
    /// Bottom-left UV origin and extent; empty for the space advance.
    pub uv: Option<[[f32; 2]; 2]>,
}
impl Content for DialoguePresentation {
    const KIND: &'static str = "dialogue-presentation";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            !self.font_name.is_empty() && self.glyphs.len() <= 1024 && !self.glyphs.is_empty(),
            "invalid dialogue font"
        );
        ensure!(
            self.tracking.is_finite() && self.tracking >= 0.0,
            "invalid font tracking"
        );
        ensure!(
            self.atlas.size.iter().all(|&n| n >= 2),
            "font atlas dimensions too small"
        );
        let mut chars = std::collections::BTreeSet::new();
        for glyph in &self.glyphs {
            ensure!(
                chars.insert(glyph.character)
                    && [glyph.width, glyph.half_height, glyph.vertical_offset]
                        .iter()
                        .all(|v| v.is_finite())
                    && glyph.width >= 0.0
                    && glyph.half_height <= 0.0,
                "invalid/duplicate dialogue glyph"
            );
            if let Some(uv) = glyph.uv {
                ensure!(
                    glyph.width > 0.0 && glyph.half_height < 0.0 && uv[1].iter().all(|v| *v > 0.0),
                    "empty textured glyph"
                );
                // Source UVs use n/(dimension-1), including a one-texel edge
                // extrapolation (e.g. J reaches 256/255). Keep that coverage.
                ensure!(
                    uv.iter()
                        .flatten()
                        .all(|v| v.is_finite() && (0.0..=1.0).contains(v)),
                    "invalid dialogue glyph UV"
                );
                ensure!(
                    uv[0][0] + uv[1][0] <= 1.0 + 1.0 / (self.atlas.size[0] - 1) as f32 + 0.00001
                        && uv[0][1] + uv[1][1]
                            <= 1.0 + 1.0 / (self.atlas.size[1] - 1) as f32 + 0.00001,
                    "glyph {} UV {:?} exceeds atlas",
                    glyph.character,
                    uv
                );
            }
        }
        ensure!(
            self.normal_color
                .iter()
                .chain(&self.emphasis_color)
                .chain(&self.selection_color)
                .all(|v| v.is_finite() && (0.0..=1.0).contains(v)),
            "invalid dialogue font color"
        );
        for sound in [&self.cash_sound, &self.scroll_sound].into_iter().flatten() {
            sound.validate()?;
        }
        for texture in std::iter::once(&self.atlas)
            .chain(&self.frame)
            .chain([&self.continuation, &self.bullet])
        {
            ensure!(
                texture.size.iter().all(|&n| n > 0 && n <= 4096),
                "invalid dialogue texture dimensions"
            );
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueLine {
    /// Optional source line drawing anchor, independent of span offsets and
    /// the loader's ignored justification scalar. No pointer survives import.
    pub layout: Option<DialogueLineLayout>,
    pub spans: Vec<DialogueSpan>,
    /// Loader-computed justification. The shipped FON renderer ignores the
    /// forwarded scalar; retain it without changing native glyph advances.
    /// It may be non-finite: the source never reads it as a number.
    pub justification: f32,
    /// The installed English dialogue uses 3.4e38 as its untimed sentinel.
    pub end_seconds: f32,
    pub flags: u8,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueLineLayout {
    /// Horizontal left/center/right and vertical unchanged/half/full-height.
    pub alignment: [u32; 2],
    pub offset: [f32; 2],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueSpan {
    /// Missing text differs from an allocated empty string: the dialogue draw
    /// uses the first span's text presence when replacing the page deadline.
    pub text: Option<String>,
    /// Horizontal layout measured in the source font's coordinate space.
    pub offset: f32,
    pub emphasis: bool,
}
impl Content for Dialogue {
    const KIND: &'static str = "dialogue";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            !self.name.is_empty() && self.name.len() <= 255,
            "invalid dialogue name"
        );
        ensure!(
            self.locale == "en",
            "dialogue extraction currently validates English only"
        );
        ensure!(
            matches!(self.box_style, 1 | 3 | 4 | 5),
            "unsupported dialogue box style"
        );
        ensure!(
            !self.lines.is_empty() && self.lines.len() <= 1024,
            "invalid dialogue line count"
        );
        for line in &self.lines {
            if let Some(layout) = &line.layout {
                ensure!(
                    matches!(self.box_style, 1 | 4)
                        && layout.alignment.iter().all(|v| *v <= 2)
                        && layout.offset.iter().all(|v| v.is_finite()),
                    "unsupported dialogue line layout"
                );
            }
            // The justification word is forwarded unread to the FON draw calls
            // (0056c8e0 -> FON_fn_vDraw3DString's last argument), so the source
            // accepts any bits, including non-finite ones (manoir3).
            ensure!(
                line.end_seconds.is_finite() && line.end_seconds >= 0.0,
                "invalid dialogue time"
            );
            ensure!(line.spans.len() <= 64, "too many dialogue spans");
            for span in &line.spans {
                ensure!(
                    span.offset.is_finite()
                        && span.text.as_ref().is_none_or(|text| text.len() <= 4096),
                    "invalid dialogue span"
                );
            }
        }
        if self.box_style == 4 {
            ensure!(
                self.lines.len() <= 2
                    && self.voice.is_none()
                    && self
                        .lines
                        .iter()
                        .all(|line| line.flags == 0 && line.end_seconds >= 3.0e38),
                "unsupported style4 title semantics"
            );
        }
        if self.box_style == 5 {
            ensure!(
                self.presentation.is_some()
                    && self.voice.is_none()
                    && self.lines.iter().all(|line| {
                        line.flags == 3
                            && line.layout.is_none()
                            && line.end_seconds.to_bits() == 0x7f7fc99e
                    }),
                "unsupported style5 dialogue semantics"
            );
        }
        if let Some(voice) = &self.voice {
            voice.validate()?;
        }
        if !self.answers.is_empty() {
            ensure!(
                self.box_style == 1
                    && (2..=16).contains(&self.answers.len())
                    && self.completion.is_none()
                    && self.presentation.is_some(),
                "unsupported dialogue answer semantics"
            );
            for answer in &self.answers {
                ensure!(
                    !answer.lines.is_empty()
                        && answer.lines.len() <= 16
                        && answer
                            .lines
                            .iter()
                            .all(|line| line.layout.is_none() && line.spans.len() <= 64),
                    "invalid dialogue answer text"
                );
                if let Some(voice) = &answer.voice {
                    voice.validate()?;
                }
            }
        }
        Ok(())
    }
}
