//! OpenSpace, Rete, and Godot adapters around one useful content model.
pub mod app;
pub mod godot;
pub mod model;
pub mod openspace;
pub mod rete;

pub use anyhow::{Error, Result};
pub(crate) mod files;
