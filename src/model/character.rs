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
/// An action belongs to a family; the member number alone is not an identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateRef {
    pub family: AssetRef<Family>,
    pub state: u32,
}
impl StateRef {
    pub fn validate_family(&self, family: &Family) -> Result<()> {
        ensure!(
            self.family.path == family.path,
            "action family identity differs: action {} of {} checked against {}",
            self.state,
            self.family.path,
            family.path
        );
        ensure!(
            family.states.iter().any(|s| s.id == self.state),
            "missing action member {} in {}",
            self.state,
            family.path
        );
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterState {
    pub id: u32,
    pub animation: Option<AssetRef<AnimationClip>>,
    pub mechanics: Option<AssetRef<super::mechanics::Mechanics>>,
    pub speed: u8,
    /// Additional animation cycles before the next state or action completion.
    pub repeats: u8,
    /// Blocks unlisted transitions only when exactly one (00435250).
    pub transition_lock: u8,
    pub next: Option<u32>,
    pub transitions: Vec<StateTransition>,
    pub prohibited: Vec<u32>,
    pub unknown: [u8; 5],
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
    /// Active shared object table; Hype's Data3d+16 selection API.
    pub configuration: Option<AssetRef<ObjectList>>,
    /// Initial shared table restored by actor reset, independently of selection.
    /// Hype's Data3d+12; Raymap labels the two object-table fields oppositely.
    pub initial_configuration: Option<AssetRef<ObjectList>>,
    pub object_types: [u32; 3],
    pub camera: Option<super::camera::Camera>,
    /// Effective source dynamics binding used by motion API calls. This is
    /// independent of the selected action's mechanics identity card.
    #[serde(default)]
    pub motion_binding_present: bool,
    /// SuperObject picking geometry: first indexed sphere center, or actor
    /// origin when absent. Template bounds must survive generation as well.
    #[serde(default)]
    pub picking_bounds: Option<AssetRef<super::collision::Collision>>,
    /// Actor-local activation and kill policy; absent on manually authored assets.
    /// Source addresses and component-event dispatch stay in the source adapter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation: Option<ActorActivation>,
    /// Actor-owned StandardGame custom flags, independent of display/activation.
    #[serde(default)]
    pub custom_bits: u32,
    /// Actor-local microphone matrix; composed with the actor rotation by the
    /// native listener. Source allocation details stay in the reader.
    pub microphone: Option<Microphone>,
    /// Source StandardGame bytes read by the life HUD; actor-local mutable state.
    pub hit_points: Option<HitPoints>,
    /// StandardGame +0x14 capabilities dword (00439050..00439080), absent
    /// without a StandardGame. Graph traversal (meta 35) is not ported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<u32>,
    /// Perso3dData shadow fields: SHW_fn_vDrawShadow (004e4e30) reads the kind
    /// at +0x198 and the scales at +0x190 and +0x194.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<ActorShadow>,
    /// The actor's typed +0x38 StockAction edge, when present in the source.
    /// The identities retain its category and item joins for native pickup calls.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory_stock: Option<super::inventory_gameplay::InventoryStock>,
    /// Category-23 AI operands address these local slots, including empty ones.
    pub routes: Vec<super::route::RouteBinding>,
    pub brain: Option<super::ai::Brain>,
    pub collisions: Option<CharacterCollisions>,
    pub particles: Option<super::particle::ParticleComponent>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorActivation {
    pub active: bool,
    /// Nonzero source NeverBack latch suppresses later activation.
    pub return_suppressed: bool,
    /// Source return policy 1 dispatches a full actor reset when killed.
    /// This distinct reset contract must not become a permanent removal.
    pub reset_on_kill: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Microphone {
    pub local_transform: super::geometry::SourceTransform,
    pub eligible: u8,
    pub unknown: [u8; 3],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HitPoints {
    pub current: u8,
    pub maximum: u8,
    pub limit: u8,
}
/// Each actor keeps its active selection; reusable definitions belong to a profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterCollisions {
    /// Source order: ZDD, ZDE, ZDM, ZDR.
    pub groups: [CharacterCollisionGroup; 4],
    pub unknown: [u8; 4],
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CharacterCollisionGroup {
    pub profile: Option<AssetRef<CollisionProfile>>,
    pub active: Option<u32>,
}
/// Reusable collision zones and their state-dependent selections for one purpose.
/// Distinct actors can share a movement profile while using different reaction profiles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollisionProfile {
    pub path: AssetPath,
    pub geometry: Option<CollisionGeometrySlots>,
    pub activations: Option<Vec<CollisionActivation>>,
    pub selections: Vec<CollisionSelection>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollisionGeometrySlots {
    /// Empty slots still have an index and can appear in an activation selection.
    pub slots: Vec<Option<AssetRef<super::collision::Collision>>>,
    pub unknown: [u8; 4],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollisionActivation {
    pub state: u16,
    pub selection: Option<u32>,
    pub unknown: [u8; 2],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollisionSelection {
    pub id: u32,
    pub zones: Vec<CollisionZone>,
    pub unknown: [u8; 4],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollisionZone {
    pub index: u16,
    pub unknown: [u8; 2],
}
impl CharacterCollisions {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.groups
                .iter()
                .all(|g| g.profile.is_some() || g.active.is_none()),
            "active collision selection without a profile"
        );
        Ok(())
    }
}
impl CollisionProfile {
    pub fn selection(&self, id: u32) -> Result<&CollisionSelection> {
        self.selections
            .iter()
            .find(|s| s.id == id)
            .ok_or_else(|| anyhow::anyhow!("missing collision selection {id} in {}", self.path))
    }
}
impl Content for CollisionProfile {
    const KIND: &'static str = "collision-profile";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        let selections: std::collections::BTreeSet<_> =
            self.selections.iter().map(|s| s.id).collect();
        ensure!(
            selections.len() == self.selections.len(),
            "duplicate collision selection"
        );
        let count = self.geometry.as_ref().map_or(0, |g| g.slots.len());
        ensure!(count <= 65536, "too many collision slots");
        // An activation entry may name a zone past the geometry's slots
        // (tour3). It is kept; it enables nothing, since export only activates
        // existing slots (stand-in `zone-past-slots`, stand-ins.md).
        let _ = count;
        if let Some(activations) = &self.activations {
            let states: std::collections::BTreeSet<_> =
                activations.iter().map(|a| a.state).collect();
            ensure!(
                states.len() == activations.len(),
                "duplicate collision activation state"
            );
            for id in activations.iter().filter_map(|a| a.selection) {
                self.selection(id)?;
            }
        }
        Ok(())
    }
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
        ensure!(
            self.picking_bounds
                .as_ref()
                .is_none_or(|r| r.path.member().is_none()),
            "picking bounds must identify an asset"
        );
        if let Some(camera) = &self.camera {
            camera.validate()?;
        }
        if let Some(microphone) = &self.microphone {
            let transform = &microphone.local_transform;
            ensure!(
                transform
                    .translation
                    .iter()
                    .chain(transform.rotation.iter().flatten())
                    .chain(transform.basis.iter().flatten())
                    .all(|v| v.is_finite()),
                "nonfinite microphone transform"
            );
        }
        ensure!(self.routes.len() <= 256, "too many actor-local route slots");
        ensure!(
            self.routes
                .iter()
                .filter_map(|b| b.route.as_ref())
                .all(|r| r.path.member().is_none()),
            "route binding must identify its asset"
        );
        if let Some(collisions) = &self.collisions {
            collisions.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorShadow {
    /// 0 none, 1 fake blob, 2 realistic, 3 and 4 placed from flagged lights.
    pub quality: i16,
    pub scale: [f32; 2],
}
