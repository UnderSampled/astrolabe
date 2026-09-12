use super::{AssetPath, AssetRef};
use anyhow::Result;
use std::sync::Arc;

/// A useful kind of content, independent of each target's I/O.
pub trait Content: Send + Sync + 'static {
    const KIND: &'static str;
    fn validate(&self) -> Result<()>;
    /// Content carrying its own identity must agree with the resolver key.
    /// Media values without an embedded identity use the containing reference alone.
    fn identity(&self) -> Option<&AssetPath> {
        None
    }
    fn validate_reference(&self, reference: &AssetPath) -> Result<()> {
        if let Some(identity) = self.identity() {
            anyhow::ensure!(
                identity == reference,
                "content identity {identity} differs from requested {reference}"
            );
        }
        Ok(())
    }
}

/// Load whole useful assets. Implementations own package and target-specific context.
pub trait Load<T: Content> {
    fn load(&mut self, reference: &AssetRef<T>) -> Result<Arc<T>>;
}

/// Save useful content. The adapter derives its target representation.
pub trait Save<T: Content> {
    fn save(&mut self, reference: &AssetRef<T>, value: &T) -> Result<()>;
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FormatSupport {
    pub format: &'static str,
    pub reads: bool,
    pub writes: bool,
    pub notes: &'static str,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ContentRoot {
    pub kind: String,
    pub reference: super::AssetPath,
}

/// Stable severity and code let callers retain source coverage without parsing prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// A semantic conversion observation. Source addresses belong in transient debug
/// summaries, not in content identity or this persisted package coverage report.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub asset: Option<AssetPath>,
    pub severity: Severity,
    pub message: String,
}
impl Diagnostic {
    pub fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            !self.code.is_empty()
                && self.code.bytes().all(|b| b.is_ascii_lowercase()
                    || b.is_ascii_digit()
                    || matches!(b, b'.' | b'_' | b'-')),
            "invalid diagnostic code"
        );
        anyhow::ensure!(!self.message.trim().is_empty(), "empty diagnostic message");
        Ok(())
    }
}
