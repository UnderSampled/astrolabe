//! Families are templates; each perso selects states/configuration in its active level session.
use super::{
    AssetPath, AssetRef,
    animation::{AnimationAction, AnimationClip},
    geometry::ModelInstance,
    io::Content,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Family {
    pub path: AssetPath,
    pub name: String,
    /// Identity in the source family-name/object-type table, independent of the asset path.
    pub object_type: u32,
    pub bounding_geometry: Option<AssetRef<super::collision::Collision>>,
    pub states: Vec<CharacterState>,
    pub configurations: Vec<AssetRef<ObjectList>>,
    pub default_configuration: Option<AssetRef<ObjectList>>,
    pub animation_bank: u8,
    /// Unresolved bytes at the end of Hype's 56-byte family record.
    pub unknown: [u8; 3],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterState {
    pub id: u32,
    pub animation: Option<AssetRef<AnimationClip>>,
    pub mechanics: Option<AssetRef<super::mechanics::Mechanics>>,
    pub speed: u8,
    pub next: Option<u32>,
    pub transitions: Vec<StateTransition>,
    pub prohibited: Vec<u32>,
    pub unknown: [u8; 7],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateTransition {
    pub target: u32,
    pub via: u32,
    pub linking_type: u8,
    pub unknown: [u8; 3],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectList {
    pub path: AssetPath,
    pub parts: Vec<ObjectPart>,
    /// Optional saved configuration restored by the original engine's actor reset.
    /// This is a separate table of values, not another runtime configuration asset.
    pub reset_parts: Option<Vec<ObjectPart>>,
    pub unknown: [u8; 2],
}
impl ObjectList {
    pub fn all_parts(&self) -> impl Iterator<Item = &ObjectPart> {
        self.parts.iter().chain(self.reset_parts.iter().flatten())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectPart {
    pub instance: Option<AssetRef<ModelInstance>>,
    pub action: Option<AssetRef<AnimationAction>>,
    pub kind: ObjectPartKind,
    pub scale: Option<[f32; 3]>,
    pub unknown: [u8; 10],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectPartKind {
    Absent,
    Model,
    Action,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Perso {
    pub path: AssetPath,
    pub name: String,
    pub family: AssetRef<Family>,
    pub initial_state: Option<u32>,
    pub current_state: Option<u32>,
    pub secondary_state: Option<u32>,
    pub configuration: Option<AssetRef<ObjectList>>,
    pub initial_configuration: Option<AssetRef<ObjectList>>,
    pub object_types: [u32; 3],
}
impl Content for Family {
    const KIND: &'static str = "family";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        ensure!(self.states.len() <= 65536, "too many states");
        ensure!(
            self.configurations.len() <= 65536,
            "too many family configurations"
        );
        let configurations: std::collections::BTreeSet<_> =
            self.configurations.iter().map(|r| &r.path).collect();
        ensure!(
            configurations.len() == self.configurations.len(),
            "duplicate family configuration"
        );
        let ids: std::collections::BTreeSet<_> = self.states.iter().map(|s| s.id).collect();
        ensure!(ids.len() == self.states.len(), "duplicate state identity");
        for s in &self.states {
            ensure!(
                s.transitions.len() <= 65536 && s.prohibited.len() <= 65536,
                "too many state links"
            );
            ensure!(
                s.next.is_none_or(|n| ids.contains(&n))
                    && s.prohibited.iter().all(|id| ids.contains(id)),
                "state target not in family"
            );
            ensure!(
                s.transitions
                    .iter()
                    .all(|t| ids.contains(&t.target) && ids.contains(&t.via)),
                "transition target not in family"
            );
        }
        Ok(())
    }
}
impl Content for ObjectList {
    const KIND: &'static str = "object-list";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        ensure!(self.parts.len() <= 65535, "too many configured objects");
        ensure!(
            self.reset_parts.is_none() || !self.parts.is_empty(),
            "empty configuration cannot carry a reset snapshot"
        );
        ensure!(
            self.reset_parts
                .as_ref()
                .is_none_or(|p| p.len() == self.parts.len()),
            "configuration reset table length differs"
        );
        ensure!(
            self.all_parts()
                .all(|p| p.scale.is_none_or(|v| v.iter().all(|x| x.is_finite()))),
            "nonfinite configured scale"
        );
        for part in self.all_parts() {
            ensure!(
                match part.kind {
                    ObjectPartKind::Absent =>
                        part.instance.is_none() && part.action.is_none() && part.scale.is_none(),
                    ObjectPartKind::Model => part.action.is_none(),
                    ObjectPartKind::Action =>
                        part.instance.is_none() && part.scale.is_none() && part.action.is_some(),
                },
                "configured part kind and content disagree"
            );
        }
        Ok(())
    }
}
impl Content for Perso {
    const KIND: &'static str = "perso";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}
