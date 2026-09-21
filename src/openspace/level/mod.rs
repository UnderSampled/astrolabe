//! Rooted Hype model import and controlled source reconstruction.
//! Virtual addresses remain private to this adapter. See docs/level-rust-research.md.
mod collision;
mod collision_export;
mod container;
mod fix_bindings;
mod fragment;
mod geometry;
pub mod hype_runtime;
mod import;
pub mod live;
mod records;
#[cfg(test)]
mod reference_tests;
mod scene;
mod session;
mod source_view;
#[cfg(test)]
mod tests;

pub use collision_export::write_collision;
pub use container::write_sna_block;
pub use fragment::{ModelFragment, write_model_fragment};
pub use geometry::{EmittedRange, GeometryExport, ReferenceTarget, write_geometry};
pub use import::{LevelSummary, ModelImport, import_first_model, inspect_level};
pub use scene::{SceneImport, import_scene};

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
#[cfg(test)]
mod character_probe;

mod transform;
pub use transform::{read_transform, write_transform};

mod animation_export;
mod character;
mod character_collision;
mod character_collision_export;
pub use character_collision_export::{
    CollisionProfileExport, CollisionProfileTargets, write_character_collisions,
    write_collision_profile,
};
#[cfg(test)]
mod character_collision_probe;
mod configuration_export;
pub use animation_export::{
    AnimationActionsExport, TransformPoolExport, write_animation, write_animation_actions,
    write_transform_pool,
};
pub use configuration_export::write_configuration;
mod family_export;
#[cfg(test)]
mod family_probe;
mod mechanics;
#[cfg(test)]
mod scene_record_probe;
pub use family_export::{FamiliesExport, FamilyDependencies, write_families};
pub use mechanics::{read_mechanics, write_mechanics};
mod sprite;

mod light;
pub use light::{read_light, write_light};
mod lighting;

mod sector;
mod sector_export;
pub use sector_export::write_sectors;

mod sound;

mod environment;
pub use environment::{read_fog, write_fog};
mod environment_export;
pub use environment_export::{EnvironmentDependencies, write_environments};

mod ai;
mod scene_export;
pub use scene_export::{SceneDependencies, SceneGraphExport, write_scene_graph};
