//! Quarantined Godot HUD projection pending a validated source reader.
use super::Project;
use crate::model::{AssetPath, content::ContentSource, hud::HudPresentation};
use anyhow::{Result, bail};
use std::collections::BTreeSet;

pub(super) fn export(
    _project: &mut Project,
    _source: &mut impl ContentSource,
    _value: &HudPresentation,
    _written: &mut BTreeSet<AssetPath>,
) -> Result<()> {
    bail!(
        "Godot HUD export is quarantined until the Fix HUD reader cursor and serialized layout are validated"
    )
}
