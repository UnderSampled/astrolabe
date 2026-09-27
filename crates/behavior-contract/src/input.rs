//! Shared physical keyboard event identities for scenario adapters.
use serde::{Deserialize, Serialize};

/// Physical keyboard positions. An adapter translates these to its device codes;
/// it must not translate them into a gameplay action or change bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Key {
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    LeftShift,
    LeftControl,
    X,
    Z,
    Space,
    Enter,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Edge {
    Press,
    Release,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    /// Only the fixture's primary keyboard is supported by this initial profile.
    pub keyboard: u8,
    pub key: Key,
    pub edge: Edge,
}
