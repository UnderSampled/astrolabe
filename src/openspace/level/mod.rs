//! Rooted Hype model import and controlled source reconstruction.
//! Virtual addresses remain private to this adapter. See docs/level-rust-research.md.
mod container;
mod fragment;
mod geometry;
mod import;
mod records;
#[cfg(test)]
mod reference_tests;
mod session;
#[cfg(test)]
mod tests;

pub use container::write_sna_block;
pub use fragment::{ModelFragment, write_model_fragment};
pub use geometry::{EmittedRange, GeometryExport, MaterialTarget, write_geometry};
pub use import::{LevelSummary, ModelImport, import_first_model, inspect_level};

#[cfg(test)]
use crate::model::{
    AssetPath, AssetRef,
    geometry::{Model, Submesh},
};
#[cfg(test)]
use anyhow::Result;
#[cfg(test)]
use container::*;
#[cfg(test)]
use records::*;
#[cfg(test)]
use session::*;
#[cfg(test)]
use std::{collections::BTreeMap, path::Path};
