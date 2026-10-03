//! Useful content and package-scoped references; no target I/O or virtual addresses.
pub mod reference;
pub use reference::{AssetPath, AssetRef, PackageRole, Selection};
pub mod audio;
pub mod collision;
pub mod content;
pub mod geometry;
pub mod io;
pub mod scene;
pub mod sprite;
pub mod texture;

pub mod ai;
pub mod animation;
pub mod camera;
pub mod character;
pub mod dialogue;
pub mod route;

pub mod light;
pub mod mechanics;

pub mod environment;
