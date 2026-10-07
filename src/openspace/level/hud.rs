//! Quarantined pending validated Fix reader cursor accounting.
use crate::model::{Selection, hud::HudPresentation};
use anyhow::Result;
use std::path::Path;

/// Returns `None` while the sequential Fix reader is quarantined; the caller
/// records the omission as a stable source diagnostic.
pub(crate) fn import(_game: &Path, _selection: &Selection) -> Result<Option<HudPresentation>> {
    Ok(None)
}
