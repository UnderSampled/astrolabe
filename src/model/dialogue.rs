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
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueLine {
    pub spans: Vec<DialogueSpan>,
    /// The installed English dialogue uses 3.4e38 as its untimed sentinel.
    pub end_seconds: f32,
    pub flags: u8,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueSpan {
    pub text: String,
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
            matches!(self.box_style, 1 | 3),
            "unsupported dialogue box style"
        );
        ensure!(
            !self.lines.is_empty() && self.lines.len() <= 1024,
            "invalid dialogue line count"
        );
        for line in &self.lines {
            ensure!(
                line.end_seconds.is_finite() && line.end_seconds >= 0.0,
                "invalid dialogue time"
            );
            ensure!(line.spans.len() <= 64, "too many dialogue spans");
            for span in &line.spans {
                ensure!(
                    span.offset.is_finite() && span.text.len() <= 4096,
                    "invalid dialogue span"
                );
            }
        }
        if let Some(voice) = &self.voice {
            voice.validate()?;
        }
        Ok(())
    }
}
