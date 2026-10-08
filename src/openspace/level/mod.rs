//! Rooted Hype model import and controlled source reconstruction.
//! Virtual addresses remain private to this adapter. See notes/level-rust-research.md.
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
mod way;
pub use way::{
    ActorWayInspection, WayInspection, WayLinkInspection, WaypointInspection, inspect_actor_ways,
};
mod save_join;
mod saved_inventory;
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
pub use save_join::{
    SaveAllocationJoin, SaveJoinResult, SaveRecordLocation, join_save_allocations,
};
pub(crate) use saved_inventory::{SavedInventoryItem, read_saved_inventory_item_zero};
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
mod armor;
mod camera;
mod character;
pub use camera::{read_camera_parameters, write_camera_parameters};
#[cfg(test)]
mod camera_probe;
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
pub use armor::import as import_armor;
pub use configuration_export::write_configuration;
mod family_export;
#[cfg(test)]
mod family_probe;
mod mechanics;
#[cfg(test)]
mod scene_record_probe;
pub use family_export::{FamiliesExport, FamilyDependencies, write_families};
pub use mechanics::{read_mechanics, write_mechanics};
mod particle;
mod sprite;
pub use particle::write_settings as write_particle_settings;

mod light;
pub use light::{read_light, write_light};
mod lighting;

mod sector;
mod sector_export;
pub use sector_export::write_sectors;

mod dialogue;
mod dialogue_presentation;
mod hud;
mod inventory;
mod menu;
pub(crate) use hud::import as import_hud_presentation;
pub(crate) use inventory::import as import_inventory_presentation;
pub(crate) use menu::{import_main_menu, import_pause_menu};
mod sound;

mod environment;
pub use environment::{read_fog, read_physical_environment, write_fog, write_physical_environment};
mod environment_export;
pub use environment_export::{EnvironmentDependencies, write_environments};

mod ai;
mod scene_export;
pub use scene_export::{SceneDependencies, SceneGraphExport, write_scene_graph};
