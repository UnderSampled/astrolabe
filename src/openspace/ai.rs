//! Hype's eight-byte AI nodes map directly into useful typed trees.
//! 005217d0 initializes reserved bytes; 00521780 counts through kind-8 sentinel.
use super::codec::FormatCodec;
use crate::model::{
    AssetRef,
    ai::*,
    audio::SoundEventLink,
    character::{ObjectList, Perso},
    geometry::Material,
    io::FormatSupport,
    route::Waypoint,
};
use anyhow::{Context, Result, bail, ensure};
use binrw::{BinRead, BinWrite};
use std::{collections::BTreeMap, io::Cursor};

#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
pub(crate) struct NodeWire {
    pub parameter: u32,
    pub reserved: [u8; 2],
    pub indent: u8,
    pub kind: u8,
}
/// Source operands are bound by the localized SND/RTS files. Encoded handles are
/// supplied only at this adapter boundary, never retained in the model/Rete.
#[derive(Default)]
pub struct AiScriptCodec {
    /// Normalized source operands enable partial decoding; empty keeps strict decoding.
    pub preserved_nodes: Vec<PreservedAiNode>,
    pub sounds: BTreeMap<usize, Option<SoundEventLink>>,
    pub sound_handles: Vec<(Option<SoundEventLink>, u32)>,
    pub actions: BTreeMap<usize, Option<crate::model::character::StateRef>>,
    pub action_handles: Vec<(Option<crate::model::character::StateRef>, u32)>,
    pub strings: BTreeMap<usize, String>,
    pub string_handles: Vec<(String, u32)>,
    pub buttons: BTreeMap<usize, String>,
    pub button_handles: Vec<(String, u32)>,
    pub behaviors: BTreeMap<usize, BehaviorRef>,
    pub behavior_handles: Vec<(BehaviorRef, u32)>,
    pub actors: BTreeMap<usize, AssetRef<Perso>>,
    pub actor_handles: Vec<(AssetRef<Perso>, u32)>,
    /// Category-27 source SuperObjects resolve named scene sector nodes.
    pub sectors: BTreeMap<usize, Option<AssetRef<crate::model::scene::Scene>>>,
    pub sector_handles: Vec<(Option<AssetRef<crate::model::scene::Scene>>, u32)>,
    /// Source category-36 GameMaterialRef operands and write-time handles.
    pub materials: BTreeMap<usize, AssetRef<Material>>,
    pub material_handles: Vec<(AssetRef<Material>, u32)>,
    pub waypoints: BTreeMap<usize, AssetRef<Waypoint>>,
    pub waypoint_handles: Vec<(AssetRef<Waypoint>, u32)>,
    pub configurations: BTreeMap<usize, Option<AssetRef<ObjectList>>>,
    pub configuration_handles: Vec<(Option<AssetRef<ObjectList>>, u32)>,
    pub particle_generators: BTreeMap<usize, AssetRef<crate::model::particle::ParticleGenerator>>,
    pub particle_generator_handles: Vec<(AssetRef<crate::model::particle::ParticleGenerator>, u32)>,
}
impl FormatCodec for AiScriptCodec {
    type Value = Script;
    fn support(&self) -> FormatSupport {
        FormatSupport {
            format: "Hype AI script",
            reads: true,
            writes: true,
            notes: "Typed expressions include category-29 Waypoint values and inclusive 3D Cond_CollisionWP; actor/math calls, armor, branches, assignment, waits, sound requests, and source visual diffuse-material calls remain typed; unsupported operands fail explicitly",
        }
    }
    fn read(&self, bytes: &[u8]) -> Result<Script> {
        ensure!(
            bytes.len().is_multiple_of(8) && bytes.len() / 8 <= 65535,
            "invalid AI node extent"
        );
        let mut cursor = Cursor::new(bytes);
        let mut nodes = Vec::new();
        while cursor.position() < bytes.len() as u64 {
            nodes.push(NodeWire::read(&mut cursor)?);
        }
        let end = nodes.pop().context("missing AI sentinel")?;
        ensure!(
            end.kind == 8 && end.indent == 0 && end.parameter == 0,
            "invalid AI sentinel"
        );
        ensure!(
            end.reserved == [0; 2],
            "nonzero AI sentinel reserved bytes are unsupported"
        );
        ensure!(
            nodes
                .iter()
                .all(|n| n.kind != 8 && n.indent > 0 && n.indent <= 64),
            "invalid AI node indentation/sentinel"
        );
        ensure!(
            nodes.iter().all(|n| n.reserved == [0; 2]) || !self.preserved_nodes.is_empty(),
            "nonzero AI node reserved bytes are unsupported"
        );
        for (i, node) in nodes.iter().enumerate() {
            let previous = i.checked_sub(1).map_or(0, |i| nodes[i].indent);
            ensure!(
                node.indent <= previous + 1,
                "AI indentation jump at node {i}"
            );
        }
        ensure!(
            self.preserved_nodes.is_empty() || self.preserved_nodes.len() == nodes.len() + 1,
            "preserved AI node extent differs"
        );
        let mut parser = Parser {
            nodes: &nodes,
            at: 0,
            codec: self,
        };
        let statements = parser.block(1)?;
        ensure!(parser.at == nodes.len(), "unconsumed AI nodes");
        let value = Script { statements };
        // Hype's designer-variable evaluator reads an unsigned byte index.
        value.validate(255)?;
        Ok(value)
    }
    fn write(&self, value: &Script) -> Result<Vec<u8>> {
        value.validate(255)?;
        ensure!(
            value.unsupported_fragments().is_empty(),
            "unsupported AI script cannot be reconstructed"
        );
        let mut writer = Writer {
            nodes: Vec::new(),
            codec: self,
        };
        writer.block(&value.statements, 1)?;
        writer.node(8, 0, 0);
        ensure!(writer.nodes.len() <= 65535, "too many encoded AI nodes");
        let mut cursor = Cursor::new(Vec::new());
        for node in writer.nodes {
            node.write(&mut cursor)?;
        }
        Ok(cursor.into_inner())
    }
}
struct Parser<'a> {
    nodes: &'a [NodeWire],
    at: usize,
    codec: &'a AiScriptCodec,
}
impl Parser<'_> {
    fn end(&self, at: usize) -> usize {
        let indent = self.nodes[at].indent;
        (at + 1..self.nodes.len())
            .find(|&i| self.nodes[i].indent <= indent)
            .unwrap_or(self.nodes.len())
    }
    fn unsupported(&mut self, at: usize, reason: String) -> Result<UnsupportedFragment> {
        self.unsupported_range(at, self.end(at), reason)
    }
    fn unsupported_range(
        &mut self,
        at: usize,
        end: usize,
        reason: String,
    ) -> Result<UnsupportedFragment> {
        ensure!(!self.codec.preserved_nodes.is_empty(), "{reason}");
        self.at = end;
        Ok(UnsupportedFragment {
            reason,
            start: at as u32,
            nodes: self.codec.preserved_nodes[at..end].to_vec(),
            receiver: None,
        })
    }

    fn take(&mut self, indent: u8) -> Result<(usize, &NodeWire)> {
        let at = self.at;
        let node = self.nodes.get(at).context("short AI tree")?;
        ensure!(
            node.indent == indent,
            "AI child indentation differs at node {at}"
        );
        self.at += 1;
        Ok((at, node))
    }
    fn block(&mut self, indent: u8) -> Result<Vec<Statement>> {
        ensure!(indent <= 64, "AI tree too deep");
        let mut out = Vec::new();
        let mut hold_pending = false;
        while self.nodes.get(self.at).is_some_and(|n| n.indent >= indent) {
            let hold_after = std::mem::take(&mut hold_pending);
            let (at, node) = self.take(indent)?;
            let (kind, id) = (node.kind, node.parameter);
            if node.reserved != [0; 2] {
                out.push(Statement::Unsupported(
                    self.unsupported(at, "nonzero AI node reserved bytes".into())?,
                ));
                continue;
            }
            out.push(match (kind, id) {
                (4, 0) => Statement::PlayerIsDead,
                (3, 13 | 196 | 229) => {
                    self.at -= 1;
                    Statement::Evaluate(self.expression(indent)?)
                }
                (4, 52) => Statement::SetHitPoints {
                    actor: self.expression(indent + 1)?,
                    value: self.expression(indent + 1)?,
                },
                (4, 55) => Statement::AddHitPoints {
                    actor: self.expression(indent + 1)?, value: self.expression(indent + 1)?
                },
                (4, 56) => Statement::SubtractHitPoints {
                    actor: self.expression(indent + 1)?,
                    value: self.expression(indent + 1)?,
                },
                (4, 57) => Statement::SetHitPointsMaximum {
                    actor: self.expression(indent + 1)?, value: self.expression(indent + 1)?
                },
                (4, 434) => Statement::SetMagicPointsMaximum(self.expression(indent + 1)?),
                (4, 418) => Statement::SetInventoryMaximum {
                    item: self.expression(indent + 1)?, value: self.expression(indent + 1)?
                },
                (4, 376) => Statement::ActivateMapName {
                    name: self.expression(indent + 1)?, active: self.expression(indent + 1)?
                },
                (0, 0) => {
                    let condition = self.expression(indent + 1)?;
                    let (_, then) = self.take(indent)?;
                    ensure!(then.kind == 0 && then.parameter == 1, "AI If lacks Then");
                    let then = self.block(indent + 1)?;
                    let otherwise = if self
                        .nodes
                        .get(self.at)
                        .is_some_and(|n| n.indent == indent && n.kind == 0 && n.parameter == 2)
                    {
                        self.at += 1;
                        Some(self.block(indent + 1)?)
                    } else {
                        None
                    };
                    Statement::If {
                        condition,
                        then,
                        otherwise,
                    }
                }
                (2, 11) => {
                    if self
                        .nodes
                        .get(self.at)
                        .is_some_and(|target| target.kind == 2 && target.parameter == 12)
                    {
                        let actor_at = self.at + 1;
                        let receiver = self
                            .nodes
                            .get(actor_at)
                            .map(|node| (node.kind, node.parameter));
                        if !matches!(receiver, Some((0, 5)))
                            && !self.codec.actors.contains_key(&actor_at)
                        {
                            out.push(Statement::Unsupported(self.unsupported(
                                at,
                                format!("unbound actor field target at node {actor_at}"),
                            )?));
                            continue;
                        }
                        let (_, dot) = self.take(indent + 1)?;
                        ensure!(dot.kind == 2 && dot.parameter == 12, "actor dot differs");
                        let (actor_at, actor) = self.take(indent + 2)?;
                        ensure!(
                            actor.kind == 24 || (actor.kind == 0 && actor.parameter == 5),
                            "actor dot lacks actor operand"
                        );
                        let main_actor = actor.kind == 0;
                        let (_, field) = self.take(indent + 2)?;
                        ensure!(field.kind == 11, "actor dot lacks designer field");
                        let variable = field.parameter.try_into()?;
                        let value = self.expression(indent + 1)?;
                        if main_actor {
                            Statement::MainActorFieldAssign { variable, value }
                        } else {
                            Statement::ActorFieldAssign {
                                actor: self.codec.actors[&actor_at].clone(),
                                variable,
                                value,
                            }
                        }
                    } else {
                        let target = self.expression(indent + 1)?;
                        let value = self.expression(indent + 1)?;
                        match target {
                            Expression::Variable(variable) => Statement::Assign { variable, value },
                            Expression::ArrayElement { array, index } => Statement::ArrayAssign {
                                array: *array,
                                index: *index,
                                value,
                            },
                            Expression::VectorComponent { vector, axis } => match *vector {
                                Expression::Variable(vector) => Statement::VectorComponentAssign {
                                    vector,
                                    axis,
                                    value,
                                },
                                _ => Statement::Unsupported(self.unsupported(
                                    at,
                                    format!("unsupported vector component target at node {at}"),
                                )?),
                            },
                            _ => Statement::Unsupported(self.unsupported(
                                at,
                                format!("unsupported AI assignment target at node {at}"),
                            )?),
                        }
                    }
                }
                (5, 37) => Statement::Wait {
                    milliseconds: self.expression(indent + 1)?,
                },
                (5, 40) => Statement::WaitInteger {
                    left: self.expression(indent + 1)?,
                    selector: self.expression(indent + 1)?,
                    right: self.expression(indent + 1)?,
                },
                (2, 7) => {
                    let (_, target) = self.take(indent + 1)?;
                    ensure!(target.kind == 11, "multiply assignment needs designer field");
                    Statement::MultiplyAssign { variable: target.parameter.try_into()?, value: self.expression(indent + 1)? }
                }
                (5, 38) => Statement::WaitBoolean { field: self.expression(indent + 1)?, expected: self.expression(indent + 1)? },
                (4, 420) => Statement::SetStockCount { actor: self.expression(indent + 1)?, count: self.expression(indent + 1)? },
                (4, 421) => Statement::DisplayPickup {
                    item: Box::new(self.expression(indent + 1)?), milliseconds: Box::new(self.expression(indent + 1)?),
                    from: Box::new(self.expression(indent + 1)?), to: Box::new(self.expression(indent + 1)?),
                    x: Box::new(self.expression(indent + 1)?), y: Box::new(self.expression(indent + 1)?),
                    width: Box::new(self.expression(indent + 1)?), height: Box::new(self.expression(indent + 1)?),
                },
                (4, 46) => Statement::Jump,
                (2, 6) => {
                    let (_, target) = self.take(indent + 1)?;
                    ensure!(
                        target.kind == 11,
                        "subtract assignment requires a designer variable at node {at}"
                    );
                    Statement::SubtractAssign {
                        variable: target.parameter.try_into()?,
                        value: self.expression(indent + 1)?,
                    }
                }
                (2, 5) => {
                    if !self
                        .nodes
                        .get(self.at)
                        .is_some_and(|target| target.kind == 11)
                    {
                        out.push(Statement::Unsupported(self.unsupported(
                            at,
                            format!(
                                "add assignment target is not a designer variable at node {at}"
                            ),
                        )?));
                        continue;
                    }
                    let (_, target) = self.take(indent + 1)?;
                    ensure!(
                        target.kind == 11,
                        "add assignment requires a designer variable at node {at}"
                    );
                    Statement::AddAssign {
                        variable: target.parameter.try_into()?,
                        value: self.expression(indent + 1)?,
                    }
                }
                (2, 9 | 10) => {
                    let target = match self.expression(indent + 1)? {
                        Expression::Variable(variable) => FieldTarget::Variable(variable),
                        Expression::ActorFieldRead { actor, variable } => {
                            FieldTarget::ActorField { actor, variable }
                        }
                        Expression::MainActorFieldRead(variable) => {
                            FieldTarget::MainActorField(variable)
                        }
                        _ => {
                            out.push(Statement::Unsupported(self.unsupported(
                                at,
                                format!("unsupported field adjustment target at node {at}"),
                            )?));
                            continue;
                        }
                    };
                    if id == 9 {
                        Statement::Increment { target }
                    } else {
                        Statement::Decrement { target }
                    }
                }
                (4, 214 | 215) => Statement::Turn {
                    per_frame: id == 215,
                    degrees_per_second: self.expression(indent + 1)?,
                },
                (4, 217) => Statement::TurnToward(self.expression(indent + 1)?),
                // Hype Proc_SetPersoAbsolutePosition (253) acts on the
                // executing Perso. The vector is source-space absolute
                // translation, converted at the Godot actor API boundary.
                (4, 253) => Statement::SetSelfAbsolutePosition(self.expression(indent + 1)?),
                (5, 23) => {
                    let (target_at, target) = self.take(indent + 1)?;
                    let target = match (target.kind, target.parameter) {
                        (0, 4) => Some(ActorTarget::Keyword(ActorKeyword::SelfActor)),
                        (0, 5) => Some(ActorTarget::Keyword(ActorKeyword::MainActor)),
                        (11, variable) => Some(ActorTarget::Variable(variable.try_into()?)),
                        (24, _) => self
                            .codec
                            .actors
                            .get(&target_at)
                            .cloned()
                            .map(ActorTarget::Reference),
                        _ => None,
                    };
                    if let Some(target) = target {
                        Statement::IncrementalTurnToward {
                            target,
                            frames: self.expression(indent + 1)?,
                        }
                    } else {
                        Statement::Unsupported(
                            self.unsupported(at, "unbound incremental turn target".into())?,
                        )
                    }
                }
                (5, 19) => Statement::IncrementalTurnPosition {
                    point: self.expression(indent + 1)?,
                    frames: self.expression(indent + 1)?,
                },
                (5, 32 | 33) => Statement::GoToPosition {
                    point: self.expression(indent + 1)?,
                    radius: self.expression(indent + 1)?,
                    horizontal: id == 33,
                },
                (5, 25) => Statement::GoToWaypoint {
                    waypoint: self.expression(indent + 1)?,
                },
                (5, 2) => Statement::PursueActor {
                    actor: self.expression(indent + 1)?,
                    radius: self.expression(indent + 1)?,
                },
                (4, 12) => Statement::ActivateActorAtPosition {
                    actor: self.expression(indent + 1)?,
                    point: self.expression(indent + 1)?,
                },
                (5, 49) => Statement::CameraGoToWaypoint {
                    camera: self.expression(indent + 1)?,
                    waypoint: self.expression(indent + 1)?,
                },
                (5, 36) => Statement::MoveLift {
                    from: self.expression(indent + 1)?,
                    to: self.expression(indent + 1)?,
                    duration_seconds: self.expression(indent + 1)?,
                },
                (4, 29) => Statement::SetVectorSpeed(self.expression(indent + 1)?),
                (4, 30) => Statement::AddWind(self.expression(indent + 1)?),
                (4, 221) => Statement::ResetRouteBack(self.route_slot(indent + 1, at)?),
                (4, 222) => Statement::InitRouteAtWaypoint {
                    slot: self.route_slot(indent + 1, at)?,
                    waypoint: self.expression(indent + 1)?,
                },
                (4, 220) | (5, 27 | 30) => {
                    let slot = self.route_slot(indent + 1, at)?;
                    if kind == 4 {
                        Statement::ResetRoute(slot)
                    } else if id == 30 {
                        Statement::GoToNextWaypoint(slot)
                    } else {
                        Statement::GoOverRoute(slot)
                    }
                }
                (4, 34) => Statement::GoRelative,
                (4, 226) => {
                    let operand = self.at;
                    let (_, actor) = self.take(indent + 1)?;
                    match (actor.kind, actor.parameter) {
                        (0, 4) => Statement::KillSelf,
                        (0, 5) => Statement::KillPerso {
                            target: KillPersoTarget::MainActor,
                        },
                        (24, _) => {
                            if let Some(reference) = self.codec.actors.get(&operand) {
                                Statement::KillPerso {
                                    target: KillPersoTarget::Reference(reference.clone()),
                                }
                            } else if let Some(PreservedAiOperand::Source(Some(identity))) = self
                                .codec
                                .preserved_nodes
                                .get(operand)
                                .map(|node| &node.operand)
                            {
                                Statement::KillPerso {
                                    target: KillPersoTarget::Source(identity.clone()),
                                }
                            } else {
                                Statement::Unsupported(
                                    self.unsupported(
                                        at,
                                        "KillPerso actor operand lacks a resolved source identity"
                                            .into(),
                                    )?,
                                )
                            }
                        }
                        _ => Statement::Unsupported(self.unsupported(
                            at,
                            "KillPerso requires a Perso or actor keyword".into(),
                        )?),
                    }
                }
                (4, 321) => Statement::ExecuteFade {
                    from: self.expression(indent + 1)?,
                    to: self.expression(indent + 1)?,
                    duration_seconds: self.expression(indent + 1)?,
                },
                (4, 312) => Statement::SetGameState(self.expression(indent + 1)?),
                (4, 307) => {
                    let operand = self.at;
                    let (_, name) = self.take(indent + 1)?;
                    ensure!(name.kind == 20, "vignette transition requires string operand");
                    if let Some(name) = self.codec.strings.get(&operand).cloned() {
                        Statement::DisplayVignetteAndReturnToMainMenu {
                            name,
                            milliseconds: self.expression(indent + 1)?,
                            image: None,
                        }
                    } else {
                        Statement::Unsupported(self.unsupported(at, "unbound vignette name".into())?)
                    }
                }
                (4, 317) => Statement::SetFadeValue(self.expression(indent + 1)?),
                (4, 318) => Statement::SetFadeTexture(self.expression(indent + 1)?),
                (4, 319) => Statement::ActivateFade,
                (4, 320) => Statement::DeactivateFade,
                (4, 129) => {
                    let operand = self.at;
                    let (_, sound) = self.take(indent + 1)?;
                    if sound.kind == 34 {
                        if let Some(event) = self.codec.sounds.get(&operand) {
                            Statement::MusicRequest { event: event.clone() }
                        } else {
                            Statement::Unsupported(self.unsupported(at, "unbound source music event".into())?)
                        }
                    } else {
                        Statement::Unsupported(self.unsupported(at, "music request requires a sound event operand".into())?)
                    }
                }
                (4, 382) => Statement::SoundKillMusic,
                (4, 411) => Statement::SoundFadeAll {
                    target: self.expression(indent + 1)?,
                    duration_ms: self.expression(indent + 1)?,
                },
                (4, 23) => Statement::SetImpulse(self.expression(indent + 1)?),
                (4, 47) => Statement::JumpAbsolute,
                (4, 417) => Statement::InventoryUnselectObject,
                (4, 48) => Statement::JumpWithoutAddingSpeed,
                (4, 192) => Statement::SetParticleGeneratorEnabled(true),
                (4, 193) => Statement::SetParticleGeneratorEnabled(false),
                (4, 194) => {
                    let (operand, node) = self.take(indent + 1)?;
                    if node.kind == 37
                        && let Some(generator) = self.codec.particle_generators.get(&operand)
                    {
                        Statement::SetParticleGenerator(generator.clone())
                    } else {
                        Statement::Unsupported(
                            self.unsupported(at, "unbound particle-generator operand".into())?,
                        )
                    }
                }
                (4, 337) => Statement::SetLaserPointerActive(true),
                (4, 338) => Statement::SetLaserPointerActive(false),
                (4, 342) => Statement::SetLaserPointerSniperMode(self.expression(indent + 1)?),
                (4, 343) => Statement::SetLaserPointerFlyMode(self.expression(indent + 1)?),
                (4, 266) => Statement::SetDisplayVisible(false),
                (4, 267) => Statement::SetDisplayVisible(true),
                (4, 187) => Statement::SetTransparency(self.expression(indent + 1)?),
                (4, 185) => Statement::SetTransparentDisplay(true),
                (4, 186) => Statement::SetTransparentDisplay(false),
                (4, 445) => Statement::ChangeSighting(self.expression(indent + 1)?),
                (4, 415) => Statement::SetEconomicActorAbsolutePosition {
                    actor: Expression::Actor(ActorTarget::Keyword(ActorKeyword::SelfActor)),
                    position: self.expression(indent + 1)?,
                },
                (4, 141) => Statement::SelectViewportCamera {
                    viewport: self.expression(indent + 1)?,
                    previous_policy: self.expression(indent + 1)?,
                    placement_policy: self.expression(indent + 1)?,
                    actor: self.expression(indent + 1)?,
                },
                (4, 158) => Statement::ChangeCameraFocal {
                    actor: self.expression(indent + 1)?,
                    selector: self.expression(indent + 1)?,
                    value: self.expression(indent + 1)?,
                },
                (4, 153) => Statement::ChangeCameraAngle {
                    actor: self.expression(indent + 1)?,
                    field: self.expression(indent + 1)?,
                    mode: self.expression(indent + 1)?,
                    value: self.expression(indent + 1)?,
                },
                (4, 143) => Statement::ChangeCameraDistance {
                    actor: self.expression(indent + 1)?,
                    mode: self.expression(indent + 1)?,
                    minimum: self.expression(indent + 1)?,
                    maximum: self.expression(indent + 1)?,
                },
                (4, 171) => Statement::CameraResetVerticalMaximum {
                    actor: self.expression(indent + 1)?,
                },
                (4, 136 | 157 | 160 | 163 | 166) => {
                    let operand = self.at;
                    // The camera is a referenced camera actor or, in a camera
                    // model (fix/437), its own Self keyword (kind 0, parameter 4).
                    let self_camera = self.nodes.get(operand).is_some_and(|child| {
                        child.kind == 0 && child.parameter == 4 && child.indent == indent + 1
                    });
                    if !self_camera
                        && (!self
                            .nodes
                            .get(operand)
                            .is_some_and(|child| child.kind == 24 && child.indent == indent + 1)
                            || !self.codec.actors.contains_key(&operand))
                    {
                        let shape = self
                            .nodes
                            .get(operand)
                            .map(|child| format!("kind {} parameter {}", child.kind, child.parameter))
                            .unwrap_or_default();
                        out.push(Statement::Unsupported(self.unsupported(
                            at,
                            format!("unbound camera actor at node {operand} ({shape})"),
                        )?));
                        continue;
                    }
                    self.take(indent + 1)?;
                    let camera = if self_camera {
                        ActorTarget::Keyword(ActorKeyword::SelfActor)
                    } else {
                        ActorTarget::Reference(self.codec.actors[&operand].clone())
                    };
                    if id == 136 {
                        Statement::CameraReset { camera }
                    } else if id == 166 {
                        Statement::CameraChangeFlag {
                            camera,
                            group: self.expression(indent + 1)?,
                            operation: self.expression(indent + 1)?,
                            selector: self.expression(indent + 1)?,
                        }
                    } else if id == 157 {
                        Statement::CameraChangeState {
                            camera,
                            state: self.expression(indent + 1)?,
                        }
                    } else if id == 163 {
                        Statement::CameraResetTargetedPerso { camera }
                    } else {
                        let (target_at, target) = self.take(indent + 1)?;
                        let target = match (target.kind, target.parameter) {
                            (0, 4) => Some(ActorTarget::Keyword(ActorKeyword::SelfActor)),
                            (0, 5) => Some(ActorTarget::Keyword(ActorKeyword::MainActor)),
                            (24, _) => self
                                .codec
                                .actors
                                .get(&target_at)
                                .cloned()
                                .map(ActorTarget::Reference),
                            _ => None,
                        };
                        if let Some(target) = target {
                            Statement::CameraChangeTargetedPerso { camera, target }
                        } else {
                            Statement::Unsupported(
                                self.unsupported(at, "unbound camera target actor".into())?,
                            )
                        }
                    }
                }
                (2, 24) => {
                    let (receiver_at, receiver) = self.take(indent + 1)?;
                    let receiver_kind = receiver.kind;
                    let receiver_parameter = receiver.parameter;
                    let keyword = match (receiver_kind, receiver_parameter) {
                        (0, 4) => Some(ActorKeyword::SelfActor),
                        (0, 5) => Some(ActorKeyword::MainActor),
                        _ => None,
                    };
                    let actor = self.codec.actors.get(&receiver_at).cloned();
                    let actor_target = match (receiver_kind, receiver_parameter) {
                        (0, 4) => Some(ActorTarget::Keyword(ActorKeyword::SelfActor)),
                        (0, 5) => Some(ActorTarget::Keyword(ActorKeyword::MainActor)),
                        (11, variable) => Some(ActorTarget::Variable(variable.try_into()?)),
                        (24, _) => actor.clone().map(ActorTarget::Reference),
                        _ => None,
                    };
                    let child = self.nodes.get(self.at);
                    if actor_target.is_some()
                        && child.is_some_and(|n| {
                            n.indent == indent + 1 && n.kind == 3 && n.parameter == 13
                        })
                    {
                        // Func13 is a pure distance query even when its result
                        // is discarded by this source Ultra statement.
                        self.at = at;
                        Statement::Evaluate(self.expression(indent)?)
                    } else if let Some(actor_target) = actor_target.clone()
                        && child.is_some_and(|n| n.indent == indent + 1 && n.kind == 4 && n.parameter == 218)
                    {
                        // 00542e60 takes Operator_Ultra's receiver.
                        self.take(indent + 1)?;
                        Statement::ActorChangeAction {
                            actor: actor_target,
                            action: self.expression(indent + 2)?,
                        }
                    } else if let Some(actor_target) = actor_target.clone()
                        && child.is_some_and(|n| n.indent == indent + 1 && n.kind == 4 && matches!(n.parameter, 185..=187))
                    {
                        // 00543130 takes Operator_Ultra's receiver.
                        let (_, operation) = self.take(indent + 1)?;
                        if operation.parameter == 187 {
                            Statement::ActorSetTransparency {
                                actor: actor_target,
                                value: self.expression(indent + 2)?,
                            }
                        } else {
                            Statement::ActorSetTransparentDisplay {
                                actor: actor_target,
                                enabled: operation.parameter == 185,
                            }
                        }
                    } else if let Some(actor_target) = actor_target.clone()
                        && child.is_some_and(|n| n.indent == indent + 1 && n.kind == 4 && n.parameter == 217)
                    {
                        // 00542c80 takes Operator_Ultra's receiver as the turner.
                        self.take(indent + 1)?;
                        Statement::ActorTurnToward {
                            actor: actor_target,
                            target: self.expression(indent + 2)?,
                        }
                    } else if let Some(actor_target) = actor_target.clone()
                        && child.is_some_and(|n| n.indent == indent + 1 && n.kind == 4 && n.parameter == 273)
                    {
                        self.take(indent + 1)?;
                        Statement::SetCustomBit {
                            actor: actor_target,
                            bit: self.expression(indent + 2)?,
                            value: self.expression(indent + 2)?,
                        }
                    } else if let Some(actor_target) = actor_target.clone()
                        && child.is_some_and(|n| n.indent == indent + 1 && n.kind == 4 && n.parameter == 127)
                    {
                        self.take(indent + 1)?;
                        let (operand, sound) = self.take(indent + 2)?;
                        if sound.kind == 34 && let Some(event) = self.codec.sounds.get(&operand) {
                            Statement::SoundOnActor { actor: actor_target, event: event.clone() }
                        } else {
                            {
                                let mut fragment = self.unsupported(at, "actor sound requires a bound sound event operand".into())?;
                                fragment.receiver = Some(actor_target);
                                Statement::Unsupported(fragment)
                            }
                        }
                    } else if let Some(actor_target) = actor_target.clone()
                        && child.is_some_and(|n| {
                            n.indent == indent + 1 && n.kind == 4 && n.parameter == 412
                        })
                    {
                        self.take(indent + 1)?;
                        Statement::SoundFadeObjectAndStop {
                            actor: actor_target,
                            target: self.expression(indent + 2)?,
                            duration_ms: self.expression(indent + 2)?,
                        }
                    } else if let Some(actor_target) = actor_target.clone()
                        && child.is_some_and(|n| n.indent == indent + 1 && n.kind == 4 && n.parameter == 413)
                    {
                        self.take(indent + 1)?;
                        let target = self.expression(indent + 2)?;
                        let duration_ms = self.expression(indent + 2)?;
                        let (operand, sound) = self.take(indent + 2)?;
                        if sound.kind == 34 && let Some(event) = self.codec.sounds.get(&operand) {
                            Statement::SoundFadeInObject { actor: actor_target, target, duration_ms, event: event.clone() }
                        } else {
                            {
                                let mut fragment = self.unsupported(at, "object sound fade requires a bound sound event operand".into())?;
                                fragment.receiver = Some(actor_target);
                                Statement::Unsupported(fragment)
                            }
                        }
                    } else if let Some(actor_target) = actor_target.clone()
                        && child.is_some_and(|n| {
                            n.indent == indent + 1 && n.kind == 4 && matches!(n.parameter, 415 | 445)
                        })
                    {
                        let (_, operation) = self.take(indent + 1)?;
                        let sighting = operation.parameter == 445;
                        let value = self.expression(indent + 2)?;
                        if sighting {
                            Statement::ChangeActorSighting {
                                actor: Expression::Actor(actor_target),
                                direction: value,
                            }
                        } else {
                            Statement::SetEconomicActorAbsolutePosition {
                                actor: Expression::Actor(actor_target),
                                position: value,
                            }
                        }
                    } else if let Some(actor_target) = actor_target.clone()
                        && child.is_some_and(|n| {
                            n.indent == indent + 1
                                && n.kind == 4
                                && matches!(n.parameter, 266 | 267)
                        })
                    {
                        let (_, operation) = self.take(indent + 1)?;
                        Statement::SetActorDisplayVisible {
                            actor: actor_target,
                            visible: operation.parameter == 267,
                        }
                    } else if let Some(actor_target) = actor_target.clone()
                        && child.is_some_and(|n| {
                            n.indent == indent + 1
                                && n.kind == 4
                                && matches!(n.parameter, 104 | 105)
                        })
                    {
                        let (_, operation) = self.take(indent + 1)?;
                        let active = operation.parameter == 104;
                        let (_, channel) = self.take(indent + 2)?;
                        let channel_kind = channel.kind;
                        let channel = channel.parameter;
                        ensure!(
                            channel_kind == 18,
                            "Ultra channel request lacks channel operand at node {at}"
                        );
                        Statement::SetActorChannelActive {
                            actor: actor_target,
                            channel,
                            active,
                        }
                    } else if let Some(actor) = keyword
                        && self.nodes.get(self.at).is_some_and(|n| {
                            n.kind == 4 && n.parameter == 253 && n.indent == indent + 1
                        })
                    {
                        self.take(indent + 1)?;
                        Statement::SetKeywordAbsolutePosition {
                            actor,
                            position: self.expression(indent + 2)?,
                        }
                    } else if receiver_kind == 24
                        && let Some(actor) = actor
                        && self.nodes.get(self.at).is_some_and(|n| {
                            n.kind == 4 && n.parameter == 253 && n.indent == indent + 1
                        })
                    {
                        self.take(indent + 1)?;
                        Statement::SetActorAbsolutePosition {
                            actor,
                            position: self.expression(indent + 2)?,
                        }
                    } else {
                        let operation = self
                            .nodes
                            .get(self.at)
                            .map(|n| format!("category {} operation {}", n.kind, n.parameter));
                        let mut fragment = self.unsupported(
                            at,
                            format!(
                                "Ultra receiver/category {receiver_kind} does not support {}",
                                operation.as_deref().unwrap_or("a missing child operation")
                            ),
                        )?;
                        fragment.receiver = actor_target;
                        Statement::Unsupported(fragment)
                    }
                }
                (4, 69) => {
                    let (_, list) = self.take(indent + 1)?;
                    ensure!(
                        list.kind == 11,
                        "AddPersoInList requires a designer list operand"
                    );
                    let list: u16 = list.parameter.try_into()?;
                    let operand = self.at;
                    let (_, actor) = self.take(indent + 1)?;
                    let actor_kind = actor.kind;
                    if actor_kind == 24 {
                        if let Some(actor) = self.codec.actors.get(&operand) {
                            Statement::ActorListAdd {
                                list,
                                actor: actor.clone(),
                            }
                        } else {
                            Statement::Unsupported(
                                self.unsupported(
                                    at,
                                    format!("unbound list actor at node {operand}"),
                                )?,
                            )
                        }
                    } else {
                        Statement::Unsupported(
                            self.unsupported(at, "AddPersoInList requires a source actor".into())?,
                        )
                    }
                }
                (4, 104 | 105) => {
                    let (_, channel) = self.take(indent + 1)?;
                    ensure!(
                        channel.kind == 18,
                        "channel request lacks channel operand at node {at}"
                    );
                    Statement::SetChannelActive {
                        channel: channel.parameter,
                        active: id == 104,
                    }
                }
                (4, 5) => {
                    let operand = self.at;
                    let (_, name) = self.take(indent + 1)?;
                    ensure!(name.kind == 20, "map change lacks a string operand");
                    if let Some(level) = self.codec.strings.get(&operand).cloned() {
                        Statement::ChangeMap { level }
                    } else {
                        Statement::Unsupported(self.unsupported(at, "unbound map name".into())?)
                    }
                }
                (4, 13) => {
                    let mut names = Vec::with_capacity(3);
                    let mut bound = true;
                    for _ in 0..3 {
                        let operand = self.at;
                        let (_, name) = self.take(indent + 1)?;
                        ensure!(
                            name.kind == 20,
                            "positioned map change lacks a string operand"
                        );
                        if let Some(value) = self.codec.strings.get(&operand) {
                            names.push(value.clone());
                        } else {
                            bound = false;
                        }
                    }
                    if bound {
                        Statement::ChangeMapAtPosition {
                            level: names[0].clone(),
                            entry: names[1].clone(),
                            camera_entry: names[2].clone(),
                        }
                    } else {
                        Statement::Unsupported(
                            self.unsupported(at, "unbound positioned map entry".into())?,
                        )
                    }
                }
                (4, 323) => Statement::ToggleWideScreen,
                (4, 402 | 403) => Statement::SwitchMicrophone {
                    actor: self.expression(indent + 1)?,
                    evaluated_second: if id == 403 { Some(self.expression(indent + 1)?) } else { None },
                },
                (4, 273) => Statement::SetCustomBit {
                    actor: ActorTarget::Keyword(ActorKeyword::SelfActor),
                    bit: self.expression(indent + 1)?,
                    value: self.expression(indent + 1)?,
                },
                (4, 268 | 269) => {
                    let parent = self.expression(indent + 1)?;
                    let (_, channel) = self.take(indent + 1)?;
                    ensure!(channel.kind == 18, "attachment requires a channel operand");
                    let channel = channel.parameter;
                    let child = self.expression(indent + 1)?;
                    Statement::SetChannelParent {
                        parent,
                        channel,
                        child,
                        attached: id == 268,
                    }
                }
                (4, 3 | 4) => {
                    let operand = self.at;
                    let (_, object) = self.take(indent + 1)?;
                    let actor = match (object.kind, object.parameter) {
                        (0, 4) => Some(ActorTarget::Keyword(ActorKeyword::SelfActor)),
                        (0, 5) => Some(ActorTarget::Keyword(ActorKeyword::MainActor)),
                        (24, _) => self.codec.actors.get(&operand).cloned().map(ActorTarget::Reference),
                        _ => None,
                    };
                    match actor {
                        Some(actor) => Statement::SetObjectActive { actor, active: id == 3 },
                        None => Statement::Unsupported(self.unsupported(at, format!("unbound source object at node {operand}"))?),
                    }
                }
                (4, 328) => Statement::PositionDialog {
                    x: self.expression(indent + 1)?,
                    y: self.expression(indent + 1)?,
                },
                (4, 325) => {
                    let operand = self.at;
                    let (_, name) = self.take(indent + 1)?;
                    ensure!(name.kind == 20, "RunDialog requires string operand");
                    if let Some(name) = self.codec.strings.get(&operand).cloned() {
                        Statement::RunDialog {
                            name,
                            mode: self.expression(indent + 1)?,
                        }
                    } else {
                        Statement::Unsupported(
                            self.unsupported(at, "unbound dialogue name".into())?,
                        )
                    }
                }
                (4, 326) => Statement::ScrollDialog(self.expression(indent + 1)?),
                (4, 327) => Statement::SelectDialog,
                (4, 324) => Statement::EndDialog,
                (4, 330) => Statement::ChangeDialogBehavior,
                (4, 422..=426 | 428) => Statement::SetHudVisible {
                    indicator: match id {
                        422 => HudIndicator::Life,
                        423 => HudIndicator::Armor,
                        424 => HudIndicator::Magic,
                        425 => HudIndicator::SelectedObject,
                        426 => HudIndicator::Weapon,
                        _ => HudIndicator::Cash,
                    },
                    visible: self.expression(indent + 1)?,
                },
                // The game handler calls Fight_ChangeArmor (429) with a byte
                // index into the shared armor-definition list. Keep the value
                // expression typed; the target component performs the source
                // byte conversion at the API boundary.
                (4, 429) => Statement::SelectArmor(self.expression(indent + 1)?),
                (4, 430) => Statement::SetArmorResistance(self.expression(indent + 1)?),
                (4, 431) => Statement::RepairArmor,
                (4, 82) => Statement::SwapConfiguredParts {
                    first: self.expression(indent + 1)?,
                    second: self.expression(indent + 1)?,
                },
                (4, 83) => {
                    if self.nodes.get(self.at).is_none_or(|node| node.kind != 35) {
                        Statement::Unsupported(self.unsupported(
                            at,
                            "configuration change requires an ObjectTableRef literal; dynamic configuration dependency is unsupported".into(),
                        )?)
                    } else {
                        let value = self.expression(indent + 1)?;
                        if matches!(value, Expression::Configuration(_)) {
                            Statement::ChangeConfiguration(value)
                        } else {
                            Statement::Unsupported(self.unsupported(at, "unbound configuration literal".into())?)
                        }
                    }
                }
                (5, 68) => Statement::WaitFrames(self.expression(indent + 1)?),
                (4, 207 | 208) => self.change_actor_behavior(id, indent, at)?,
                (4, 209 | 210) => self.change_behavior(id, indent, false, at)?,
                (4, 432) => Statement::FightSubHitPointsEnemy(Expression::FightSubtractAndGetHitPoints(
                    Box::new(Expression::SubtractAndGetHitPoints {
                        actor: Box::new(self.expression(indent + 1)?),
                        value: Box::new(self.expression(indent + 1)?),
                    }),
                )),
                (4, 6) => {
                    // The observed return-zero/camera-update pair proceeds to
                    // the camera handler. Locomotion's return-one/behavior
                    // request pair yields. Keep both as ordinary compiled flow,
                    // without a source return flag in Godot support.
                    let (_, value) = self.take(indent + 1)?;
                    let value_kind = value.kind;
                    let return_policy = value.parameter;
                    ensure!(
                        value_kind == 12,
                        "action-return policy lacks integer at node {at}"
                    );
                    if return_policy == 0
                        && self.nodes.get(self.at).is_some_and(|request| {
                            request.indent == indent
                                && request.kind == 4
                                && request.parameter == 135
                        })
                    {
                        self.take(indent)?;
                        Statement::CameraUpdatePosition {
                            preceded_by_return_zero: true,
                        }
                    } else if return_policy == 1
                        && self.nodes.get(self.at).is_some_and(|request| {
                            request.indent == indent && request.kind == 5
                        })
                    {
                        // A compiled meta action already holds the rule until it ends.
                        continue;
                    } else if return_policy == 0 {
                        // 0053fc30 stores the policy; 00522750 then ORs it into
                        // the next node's control after masking 0x1c, so policy
                        // 0 clears that node's end-of-rule bits and execution
                        // continues to the following statement: plain sequence.
                        continue;
                    } else if return_policy == 1
                        && self.nodes.get(self.at).is_some_and(|request| {
                            request.indent == indent
                                && request.kind == 4
                                && !matches!(request.parameter, 209 | 210)
                        })
                    {
                        // Policy 1 before a procedure (e.g. Proc_Turn): the
                        // procedure runs, then its control is forced to hold.
                        hold_pending = true;
                        continue;
                    } else if return_policy != 1
                        || !self.nodes.get(self.at).is_some_and(|request| {
                            request.indent == indent
                                && request.kind == 4
                                && matches!(request.parameter, 209 | 210)
                        })
                    {
                        out.push(Statement::Unsupported(self.unsupported(
                            at,
                            format!("unsupported action-return policy {return_policy} at node {at}"),
                        )?));
                        continue;
                    } else {
                        let (_, request) = self.take(indent)?;
                        let id = request.parameter;
                        self.change_behavior(id, indent, true, at)?
                    }
                }
                (4, 135) => Statement::CameraUpdatePosition {
                    preceded_by_return_zero: false,
                },
                (4, 218) => Statement::ChangeAction(self.expression(indent + 1)?),
                (5, 4) => Statement::ExecuteAction(self.expression(indent + 1)?),
                // Meta-action 17 is PXXAA: runtime Perso expression, two
                // distance thresholds, then far/near action references.
                // The useful representation preserves these source operands;
                // Godot currently rejects execution until its movement/update
                // semantics are implemented from the remaining source trace.
                (5, 17) => Statement::AdjustPersoOnObject {
                    target: self.expression(indent + 1)?,
                    near_threshold: self.expression(indent + 1)?,
                    far_threshold: self.expression(indent + 1)?,
                    far_action: self.expression(indent + 1)?,
                    near_action: self.expression(indent + 1)?,
                },
                (5, 15) => {
                    let operand = self.at;
                    if !self
                        .nodes
                        .get(operand)
                        .is_some_and(|child| child.kind == 24 && child.indent == indent + 1)
                        || !self.codec.actors.contains_key(&operand)
                    {
                        out.push(Statement::Unsupported(self.unsupported(
                            at,
                            format!("unbound object template at node {operand}"),
                        )?));
                        continue;
                    }
                    self.take(indent + 1)?;
                    Statement::ExecuteActionAndGenerateObject {
                        template: self.codec.actors[&operand].clone(),
                        action: self.expression(indent + 1)?,
                        vector: self.expression(indent + 1)?,
                    }
                }
                (5, 6) => Statement::WaitEndOfAction,
                (5, 61) => Statement::WaitEndOfFade,
                (5, 8) => Statement::WaitEndOfAnimation,
                (4, 127) => {
                    let (operand, child) = self.take(indent + 1)?;
                    ensure!(child.kind == 34, "sound request lacks sound operand");
                    let event = self
                        .codec
                        .sounds
                        .get(&operand)
                        .with_context(|| format!("unbound AI sound at node {operand}"))?
                        .clone();
                    Statement::Sound { event }
                }
                (4, 408) => Statement::SoundFadeMusic {
                    arg0: self.expression(indent + 1)?,
                    arg1: self.expression(indent + 1)?,
                },
                (4, 386) => Statement::SoundSetMyEffectsVolume(self.expression(indent + 1)?),
                (4, 412) => Statement::SoundFadeObjectAndStop {
                    actor: ActorTarget::Keyword(ActorKeyword::SelfActor),
                    target: self.expression(indent + 1)?,
                    duration_ms: self.expression(indent + 1)?,
                },
                (4, 413) => {
                    let target = self.expression(indent + 1)?;
                    let duration_ms = self.expression(indent + 1)?;
                    let (operand, sound) = self.take(indent + 1)?;
                    if sound.kind == 34 && let Some(event) = self.codec.sounds.get(&operand) {
                        Statement::SoundFadeInObject { actor: ActorTarget::Keyword(ActorKeyword::SelfActor), target, duration_ms, event: event.clone() }
                    } else {
                        Statement::Unsupported(self.unsupported(at, "object sound fade requires a bound sound event operand".into())?)
                    }
                }
                (4, 409) => Statement::SoundFadeVoice {
                    target: self.expression(indent + 1)?,
                    duration_ms: self.expression(indent + 1)?,
                },
                (4, 410) => Statement::SoundFadeSound {
                    target: self.expression(indent + 1)?,
                    duration_ms: self.expression(indent + 1)?,
                },
                (4, 237) => {
                    let material_at = self.at;
                    let is_material = self
                        .nodes
                        .get(material_at)
                        .is_some_and(|node| node.indent == indent + 1 && node.kind == 36);
                    let is_bound = self.codec.materials.contains_key(&material_at);
                    let is_designer_variable = self
                        .nodes
                        .get(material_at + 1)
                        .is_some_and(|node| node.indent == indent + 1 && node.kind == 11);
                    if !is_material || !is_bound || !is_designer_variable {
                        out.push(Statement::Unsupported(self.unsupported(
                            at,
                            format!(
                                "Proc_SetVisualGMTDiffuseCoef requires a bound category-36 material and category-11 designer field at node {material_at}"
                            ),
                        )?));
                        continue;
                    }
                    self.take(indent + 1)?;
                    let value = self.expression(indent + 1)?;
                    Statement::SetVisualGMTDiffuseCoef {
                        material: self.codec.materials[&material_at].clone(),
                        value,
                    }
                }
                (4, 206) => Statement::Noop,
                _ => Statement::Unsupported(self.unsupported(
                    at,
                    format!("unsupported AI statement category {kind} operation {id} at node {at}"),
                )?),
            });
            if hold_after {
                out.push(Statement::HoldAfterPrevious);
            }
            ensure!(
                self.nodes.get(self.at).is_none_or(|n| n.indent <= indent),
                "unexpected AI statement child at node {}",
                self.at
            );
        }
        Ok(out)
    }
    fn behavior(&mut self, indent: u8) -> Result<BehaviorRef> {
        let (at, node) = self.take(indent)?;
        ensure!(node.kind == 32, "behavior request lacks a behavior operand");
        self.codec
            .behaviors
            .get(&at)
            .cloned()
            .with_context(|| format!("unbound behavior at node {at}"))
    }
    fn change_behavior(
        &mut self,
        id: u32,
        indent: u8,
        pause_after: bool,
        start: usize,
    ) -> Result<Statement> {
        let operand = self.at;
        let group = if id == 209 {
            BehaviorGroup::Normal
        } else {
            BehaviorGroup::Reflex
        };
        // Field 7 PrevComportIntell / 8 PrevComportReflex (0x005ffd68 table).
        if self.nodes.get(operand).is_some_and(|node| {
            node.kind == 9
                && node.indent == indent + 1
                && node.parameter == if group == BehaviorGroup::Normal { 7 } else { 8 }
        }) {
            self.take(indent + 1)?;
            return Ok(Statement::ChangeBehaviorToPrevious { group, pause_after });
        }
        if let Some(node) = self
            .nodes
            .get(operand)
            .filter(|node| node.kind == 11 && node.indent == indent + 1)
        {
            // A designer variable operand: typed only when the variable holds
            // a behavior (DSG type 13); the emitter checks its model and group.
            let variable = u16::try_from(node.parameter)?;
            self.take(indent + 1)?;
            return Ok(Statement::ChangeBehaviorVariable {
                variable,
                group,
                pause_after,
            });
        }
        if !self
            .nodes
            .get(operand)
            .is_some_and(|node| node.kind == 32 && node.indent == indent + 1)
        {
            return Ok(Statement::Unsupported(self.unsupported_range(
                start,
                self.end(start),
                format!(
                    "behavior request at node {start} has a non-behavior operand at node {operand} ({})",
                    self.nodes
                        .get(operand)
                        .map(|n| format!("kind {} parameter {}", n.kind, n.parameter))
                        .unwrap_or_default()
                ),
            )?));
        }
        if self
            .nodes
            .get(operand)
            .is_some_and(|n| n.kind == 32 && !self.codec.behaviors.contains_key(&operand))
        {
            return Ok(Statement::Unsupported(self.unsupported_range(
                start,
                self.end(operand - 1),
                format!("unbound behavior at node {operand}"),
            )?));
        }
        let target = self.behavior(indent + 1)?;
        ensure!(
            target.group == group,
            "behavior request/owning group differs"
        );
        Ok(Statement::ChangeBehavior {
            target,
            pause_after,
        })
    }
    /// A way operand: kind 23 is a literal way index; a kind-11 designer
    /// variable of type Way holds the same index (0043e370).
    fn route_slot(&mut self, indent: u8, at: usize) -> Result<RouteSlot> {
        let (_, slot) = self.take(indent)?;
        Ok(match slot.kind {
            23 => RouteSlot::Slot(slot.parameter.try_into()?),
            11 => RouteSlot::Variable(slot.parameter.try_into()?),
            _ => bail!("route call lacks a way operand at node {at}"),
        })
    }
    fn change_actor_behavior(&mut self, id: u32, indent: u8, start: usize) -> Result<Statement> {
        let actor = match self.expression(indent + 1)? {
            Expression::Actor(actor) => actor,
            // A designer variable holding the actor, e.g. a wolf's partner.
            Expression::Variable(index) => ActorTarget::Variable(index),
            _ => {
                return Ok(Statement::Unsupported(self.unsupported_range(
                    start,
                    self.end(start),
                    format!(
                        "Proc_ChangeComport actor operand is not a resolved actor at node {start}"
                    ),
                )?));
            }
        };
        let operand = self.at;
        if !self
            .nodes
            .get(operand)
            .is_some_and(|node| node.kind == 32 && node.indent == indent + 1)
        {
            return Ok(Statement::Unsupported(self.unsupported_range(
                start,
                self.end(start),
                format!("Proc_ChangeComport lacks a behavior operand at node {operand}"),
            )?));
        }
        if !self.codec.behaviors.contains_key(&operand) {
            return Ok(Statement::Unsupported(self.unsupported_range(
                start,
                self.end(start),
                format!("Proc_ChangeComport behavior operand is unbound at node {operand}"),
            )?));
        }
        let target = self.behavior(indent + 1)?;
        let group = if id == 207 {
            BehaviorGroup::Normal
        } else {
            BehaviorGroup::Reflex
        };
        if target.group != group {
            return Ok(Statement::Unsupported(self.unsupported_range(
                start,
                self.end(start),
                format!("actor behavior request/owning group differs at node {operand}"),
            )?));
        }
        Ok(Statement::ChangeActorBehavior { actor, target })
    }
    fn expression(&mut self, indent: u8) -> Result<Expression> {
        ensure!(indent <= 64, "AI expression too deep");
        let (at, node) = self.take(indent)?;
        let (kind, id) = (node.kind, node.parameter);
        if node.reserved != [0; 2] {
            return Ok(Expression::Unsupported(
                self.unsupported(at, "nonzero AI node reserved bytes".into())?,
            ));
        }
        let end = self.end(at);
        if kind == 25 && !self.codec.actions.contains_key(&at) {
            return Ok(Expression::Unsupported(
                self.unsupported(at, format!("unbound action at node {at}"))?,
            ));
        }
        let value = match (kind, id) {
            (0, 4) => Expression::Actor(ActorTarget::Keyword(ActorKeyword::SelfActor)),
            (0, 5) => Expression::Actor(ActorTarget::Keyword(ActorKeyword::MainActor)),
            (0, 7) => Expression::Nobody,
            (0, 8) => Expression::Nowhere,
            (24, _) => match self.codec.actors.get(&at) {
                Some(actor) => Expression::Actor(ActorTarget::Reference(actor.clone())),
                None => Expression::Unsupported(
                    self.unsupported(at, format!("unbound actor at node {at}"))?,
                ),
            },
            (27, _) => match self.codec.sectors.get(&at) {
                Some(sector) => Expression::SectorReference(sector.clone()),
                None if id == 0 => Expression::SectorReference(None),
                None => Expression::Unsupported(
                    self.unsupported(at, format!("unbound sector at node {at}"))?,
                ),
            },
            (35, 0 | u32::MAX)
                if self
                    .codec
                    .configurations
                    .get(&at)
                    .is_none_or(Option::is_none) =>
            {
                Expression::Configuration(None)
            }
            (35, _) => match self.codec.configurations.get(&at) {
                Some(reference) => Expression::Configuration(reference.clone()),
                None => Expression::Unsupported(
                    self.unsupported(at, "unbound ObjectTableRef literal".into())?,
                ),
            },
            (12, n) => Expression::Integer(n as i32),
            (13, n) => Expression::Real(f32::from_bits(n)),
            (16, 0) => Expression::Vector(Box::new([
                self.expression(indent + 1)?,
                self.expression(indent + 1)?,
                self.expression(indent + 1)?,
            ])),
            (9, 0) => Expression::SelfPositionField,
            (29, _) => match self.codec.waypoints.get(&at) {
                Some(reference) => Expression::WaypointReference(reference.clone()),
                None => Expression::Unsupported(
                    self.unsupported(at, format!("unbound Waypoint reference at node {at}"))?,
                ),
            },
            (11, n) => Expression::Variable(n.try_into()?),
            (1, 53) => {
                let (_, list) = self.take(indent + 1)?;
                ensure!(
                    list.kind == 11,
                    "IsPersoInList requires a designer list operand"
                );
                let list: u16 = list.parameter.try_into()?;
                Expression::ActorListContains {
                    list,
                    actor: Box::new(self.expression(indent + 1)?),
                }
            }
            (2, 0) => Expression::Plus(
                Box::new(self.expression(indent + 1)?),
                Box::new(self.expression(indent + 1)?),
            ),
            (2, 1) => Expression::Minus(
                Box::new(self.expression(indent + 1)?),
                Box::new(self.expression(indent + 1)?),
            ),
            (3, 189) => Expression::Modulus(
                Box::new(self.expression(indent + 1)?),
                Box::new(self.expression(indent + 1)?),
            ),
            (2, 2) => Expression::Multiply(
                Box::new(self.expression(indent + 1)?),
                Box::new(self.expression(indent + 1)?),
            ),
            (2, 3) => Expression::Divide(
                Box::new(self.expression(indent + 1)?),
                Box::new(self.expression(indent + 1)?),
            ),
            (2, 16) => Expression::VectorPlus(
                Box::new(self.expression(indent + 1)?),
                Box::new(self.expression(indent + 1)?),
            ),
            (2, 17) => Expression::VectorMinus(
                Box::new(self.expression(indent + 1)?),
                Box::new(self.expression(indent + 1)?),
            ),
            (2, 18) => Expression::VectorScale(
                Box::new(self.expression(indent + 1)?),
                Box::new(self.expression(indent + 1)?),
            ),
            (2, 12) => {
                let actor_at = self.at;
                if self.nodes.get(actor_at + 1).is_some_and(|field| {
                    field.indent == indent + 1
                        && field.kind == 9
                        && matches!(field.parameter, 0 | 1)
                }) {
                    let actor = self.expression(indent + 1)?;
                    let (_, field) = self.take(indent + 1)?;
                    if let Expression::Actor(actor) = actor {
                        return Ok(if field.parameter == 0 {
                            Expression::ActorPositionField(actor)
                        } else {
                            Expression::ActorSightingField(actor)
                        });
                    }
                    return Ok(Expression::Unsupported(
                        self.unsupported(at, "position field needs actor".into())?,
                    ));
                }
                if !self.nodes.get(actor_at).is_some_and(|actor| {
                    actor.indent == indent + 1
                        && (actor.kind == 24
                            || (actor.kind == 0 && matches!(actor.parameter, 4 | 5)))
                }) || !self
                    .nodes
                    .get(actor_at + 1)
                    .is_some_and(|field| field.indent == indent + 1 && field.kind == 11)
                {
                    return Ok(Expression::Unsupported(self.unsupported(
                        at,
                        format!("unsupported actor field receiver or field at node {at}"),
                    )?));
                }
                let (_, actor) = self.take(indent + 1)?;
                let receiver = (actor.kind, actor.parameter);
                let (_, field) = self.take(indent + 1)?;
                let variable = field.parameter.try_into()?;
                match receiver {
                    (0, 4) => Expression::SelfFieldRead(variable),
                    (0, 5) => Expression::MainActorFieldRead(variable),
                    (24, _) => {
                        if let Some(actor) = self.codec.actors.get(&actor_at) {
                            Expression::ActorFieldRead {
                                actor: actor.clone(),
                                variable,
                            }
                        } else {
                            Expression::Unsupported(self.unsupported(
                                at,
                                format!("unbound actor field read at node {actor_at}"),
                            )?)
                        }
                    }
                    _ => Expression::Unsupported(self.unsupported(
                        at,
                        format!("unsupported actor field receiver at node {actor_at}"),
                    )?),
                }
            }
            (2, 4) => Expression::Negate(Box::new(self.expression(indent + 1)?)),
            (25, _) => Expression::Action(
                self.codec
                    .actions
                    .get(&at)
                    .with_context(|| format!("unbound action at node {at}"))?
                    .clone(),
            ),
            (1, 59) => Expression::IsInAction(Box::new(self.expression(indent + 1)?)),
            (1, 121) => Expression::DialogIsOver,
            (1, 120) => Expression::DialogIsOverAndChangeBehavior,
            (1, 123) => Expression::ActionFinished(
                if self.nodes.get(self.at).is_some_and(|n| n.indent > indent) {
                    Some(Box::new(self.expression(indent + 1)?))
                } else {
                    None
                },
            ),
            (1, 31) => Expression::CollideWithGround,
            (1, 33) => Expression::CollideWithNothing,
            (1, 32) => Expression::CollideWithWall,
            (1, 65) => Expression::IsValidWaypoint(Box::new(self.expression(indent + 1)?)),
            (1, 34) => Expression::CollideWithSlope,
            (1, 37) => Expression::CollideWithPerso,
            (1, 28) => {
                let (_, actor) = self.take(indent + 1)?;
                let (actor_kind, actor_parameter) = (actor.kind, actor.parameter);
                let (_, zone) = self.take(indent + 1)?;
                let (zone_kind, zone_parameter) = (zone.kind, zone.parameter);
                let (_, limit) = self.take(indent + 1)?;
                match (actor_kind, actor_parameter, zone_kind, limit.kind) {
                    (0, 4, 12, 17) => Expression::CollidePersoZdeNoWithTypeZde {
                        zone: zone_parameter,
                        material_mask: limit.parameter,
                    },
                    _ => return Ok(Expression::Unsupported(self.unsupported_range(
                        at,
                        end,
                        format!(
                            "unsupported Cond_CollidePersoZDENoWithTypeZDE operands at node {at}"
                        ),
                    )?)),
                }
            }
            (1, opcode @ 24..=27) => {
                let first = self.expression(indent + 1)?;
                let mut masks = [0u32; 2];
                let mut mask = |parser: &mut Self, slot: usize| -> Result<bool> {
                    let (_, node) = parser.take(indent + 1)?;
                    masks[slot] = node.parameter;
                    Ok(node.kind == 17)
                };
                let first_ok = !matches!(opcode, 24 | 26) || mask(self, 0)?;
                let second = self.expression(indent + 1)?;
                let second_ok = !matches!(opcode, 24 | 27) || mask(self, 1)?;
                if !first_ok || !second_ok {
                    return Ok(Expression::Unsupported(self.unsupported_range(
                        at,
                        end,
                        format!("Cond{opcode} material mask is not a constant at node {at}"),
                    )?));
                }
                Expression::CollidePersoTypeZdeWithPersoTypeZde {
                    opcode,
                    first: Box::new(first),
                    first_mask: masks[0],
                    second: Box::new(second),
                    second_mask: masks[1],
                }
            }
            (1, 60) => {
                let (_, material_type) = self.take(indent + 1)?;
                ensure!(
                    material_type.kind == 17,
                    "GMT collision query lacks a material type at node {at}"
                );
                Expression::IsTypeOfGmtCollide(material_type.parameter)
            }
            (1, 69) => Expression::InTopOfJump,
            (1, 55) => Expression::TimeElapsed {
                start: Box::new(self.expression(indent + 1)?),
                duration: Box::new(self.expression(indent + 1)?),
            },
            (1, 76) => {
                let (_, actor) = self.take(indent + 1)?;
                let target = match (actor.kind, actor.parameter) {
                    (0, 4) => ActorKeyword::SelfActor,
                    (0, 5) => ActorKeyword::MainActor,
                    _ => {
                        return Ok(Expression::Unsupported(self.unsupported(
                            at,
                            "SeePerso requires an unsupported target actor".into(),
                        )?));
                    }
                };
                Expression::SeePerso {
                    receiver: ActorKeyword::SelfActor,
                    target,
                }
            }
            (1, 107) => Expression::HasCustomBit {
                actor: ActorTarget::Keyword(ActorKeyword::SelfActor),
                bit: Box::new(self.expression(indent + 1)?),
            },
            (1, 108) => Expression::WaypointSphereContains {
                actor: Box::new(self.expression(indent + 1)?),
                waypoint: Box::new(self.expression(indent + 1)?),
            },
            // 00535460 evaluates the actor and captures its current sector
            // before evaluating the second value. It does not select Ultra.
            (1, 124) => Expression::ActorInSector {
                actor: Box::new(self.expression(indent + 1)?),
                sector: Box::new(self.expression(indent + 1)?),
            },
            (2, 24) => {
                let (receiver_at, actor) = self.take(indent + 1)?;
                let receiver = match (actor.kind, actor.parameter) {
                    (0, 4) => ActorTarget::Keyword(ActorKeyword::SelfActor),
                    (0, 5) => ActorTarget::Keyword(ActorKeyword::MainActor),
                    (11, variable) => ActorTarget::Variable(variable.try_into()?),
                    (24, _) => match self.codec.actors.get(&receiver_at).cloned() {
                        Some(actor) => ActorTarget::Reference(actor),
                        None => {
                            return Ok(Expression::Unsupported(self.unsupported(
                                at,
                                format!("unbound Ultra receiver actor at node {receiver_at}"),
                            )?));
                        }
                    },
                    _ => {
                        return Ok(Expression::Unsupported(self.unsupported(
                            at,
                            "Ultra requires an unsupported receiver actor".into(),
                        )?));
                    }
                };
                match self.expression(indent + 1)? {
                    Expression::SelfPosition => Expression::ActorPosition(receiver),
                    Expression::SelfSighting => Expression::ActorSighting(receiver),
                    Expression::HasCustomBit { bit, .. } => Expression::HasCustomBit {
                        actor: receiver,
                        bit,
                    },
                    Expression::ActorLocalSourceXYDistance(target) => {
                        Expression::ActorLocalDistance {
                            receiver,
                            target,
                            vertical: false,
                        }
                    }
                    Expression::ActorLocalDistance {
                        target, vertical, ..
                    } => Expression::ActorLocalDistance {
                        receiver,
                        target,
                        vertical,
                    },
                    Expression::IsAbovePerso {
                        point, threshold, ..
                    } => Expression::IsAbovePerso {
                        receiver,
                        point,
                        threshold,
                    },
                    Expression::ActorDistance(target) => {
                        Expression::ActorDistanceFrom { receiver, target }
                    }
                    Expression::DistanceToPosition {
                        receiver: None,
                        point,
                    } => Expression::DistanceToPosition {
                        receiver: Some(receiver),
                        point,
                    },
                    Expression::ActorRelativeZ { target, .. } => {
                        Expression::ActorRelativeZ { receiver, target }
                    }
                    Expression::ActorFacingAngle { target, .. } => {
                        Expression::ActorFacingAngle { receiver, target }
                    }
                    Expression::MainActorDistance => Expression::ActorDistanceFrom {
                        receiver,
                        target: Box::new(Expression::Actor(ActorTarget::Keyword(
                            ActorKeyword::MainActor,
                        ))),
                    },
                    Expression::DistanceToWaypoint { waypoint, .. } => {
                        Expression::DistanceToWaypoint {
                            actor: receiver,
                            waypoint,
                        }
                    }
                    Expression::SeePerso { target, .. } => match receiver {
                        ActorTarget::Keyword(receiver) => Expression::SeePerso { receiver, target },
                        ActorTarget::Reference(_) | ActorTarget::Variable(_) => {
                            let mut fragment = self.unsupported(
                                at,
                                "Ultra SeePerso actor reference is unsupported".into(),
                            )?;
                            fragment.receiver = Some(receiver);
                            Expression::Unsupported(fragment)
                        }
                    },
                    _ => {
                        let mut fragment = self.unsupported(
                            at,
                            "Ultra operation is unsupported for this receiver".into(),
                        )?;
                        fragment.receiver = Some(receiver);
                        Expression::Unsupported(fragment)
                    }
                }
            }
            (1, 56 | 57) => {
                let (actor_at, actor) = self.take(indent + 1)?;
                let actor = match (actor.kind, actor.parameter) {
                    (0, 4) => ActorTarget::Keyword(ActorKeyword::SelfActor),
                    (0, 5) => ActorTarget::Keyword(ActorKeyword::MainActor),
                    (11, variable) => ActorTarget::Variable(variable.try_into()?),
                    (24, _) if id == 56 => match self.codec.actors.get(&actor_at) {
                        Some(actor) => ActorTarget::Reference(actor.clone()),
                        None => {
                            return Ok(Expression::Unsupported(self.unsupported(
                                at,
                                format!("unbound behavior-query actor at node {actor_at}"),
                            )?));
                        }
                    },
                    (24, _) => {
                        return Ok(Expression::Unsupported(self.unsupported(
                            at,
                            "reflex behavior query actor reference is unsupported".into(),
                        )?));
                    }
                    _ => {
                        return Ok(Expression::Unsupported(self.unsupported(
                            at,
                            "behavior query requires an unsupported actor expression".into(),
                        )?));
                    }
                };
                if !self.codec.behaviors.contains_key(&self.at)
                    && self.nodes.get(self.at).is_some_and(|n| n.kind == 32)
                {
                    return Ok(Expression::Unsupported(self.unsupported(
                        at,
                        format!("unbound behavior at node {}", self.at),
                    )?));
                }
                let target = self.behavior(indent + 1)?;
                ensure!(
                    target.group
                        == if id == 56 {
                            BehaviorGroup::Normal
                        } else {
                            BehaviorGroup::Reflex
                        },
                    "behavior query/owning group differs"
                );
                Expression::IsInBehavior { actor, target }
            }
            (1, 9..=12) => {
                let (operand, child) = self.take(indent + 1)?;
                ensure!(child.kind == 14, "button query lacks logical input operand");
                let query = match id {
                    9 => ButtonQuery::Pressed,
                    10 => ButtonQuery::JustPressed,
                    11 => ButtonQuery::Released,
                    _ => ButtonQuery::JustReleased,
                };
                if matches!(child.parameter, 0 | u32::MAX)
                    && !self.codec.buttons.contains_key(&operand)
                {
                    return Ok(Expression::AbsentButton(query));
                }
                if !self.codec.buttons.contains_key(&operand) {
                    return Ok(Expression::Unsupported(
                        self.unsupported(at, format!("unbound input at node {operand}"))?,
                    ));
                }
                Expression::Button {
                    action: self
                        .codec
                        .buttons
                        .get(&operand)
                        .with_context(|| format!("unbound button at node {operand}"))?
                        .clone(),
                    query,
                }
            }
            (2, 26) => Expression::ArrayElement {
                array: Box::new(self.expression(indent + 1)?),
                index: Box::new(self.expression(indent + 1)?),
            },
            (2, 13..=15) => Expression::VectorComponent {
                vector: Box::new(self.expression(indent + 1)?),
                axis: match id {
                    13 => VectorAxis::X,
                    14 => VectorAxis::Y,
                    _ => VectorAxis::Z,
                },
            },
            (1, 0 | 1) => {
                let a = Box::new(self.expression(indent + 1)?);
                let b = Box::new(self.expression(indent + 1)?);
                if id == 0 {
                    Expression::And(a, b)
                } else {
                    Expression::Or(a, b)
                }
            }
            (1, 2) => Expression::Not(Box::new(self.expression(indent + 1)?)),
            (1, 64) => Expression::IsValidActor(Box::new(self.expression(indent + 1)?)),
            (1, 3..=8) => Expression::Compare {
                operation: match id {
                    3 => Comparison::Equal,
                    4 => Comparison::Different,
                    5 => Comparison::Less,
                    6 => Comparison::Greater,
                    7 => Comparison::LessOrEqual,
                    _ => Comparison::GreaterOrEqual,
                },
                left: Box::new(self.expression(indent + 1)?),
                right: Box::new(self.expression(indent + 1)?),
            },
            // 00537c20 evaluates its P operand with the script owner as caller,
            // even under Ultra, so a designer variable is the owner's actor
            // (notes/research/distance-to-perso-designer-actor.md).
            (3, 4) => {
                let actor = match self.expression(indent + 1)? {
                    Expression::Actor(actor) => actor,
                    Expression::Variable(index) => ActorTarget::Variable(index),
                    _ => {
                        return Ok(Expression::Unsupported(self.unsupported(
                            at,
                            "distance requires an unsupported actor expression".into(),
                        )?));
                    }
                };
                if actor == ActorTarget::Keyword(ActorKeyword::MainActor) {
                    Expression::MainActorDistance
                } else {
                    Expression::ActorDistance(Box::new(Expression::Actor(actor)))
                }
            }
            // Installed Hype PE row 8 is named Func_DistanceXZToPerso, but
            // handler 0x537aa0 passes selector 3 to 0x52ea00; helper 0x52e910
            // selector 3 computes the source-XY magnitude after transforming
            // the displacement into the explicit target actor's local frame.
            (3, 8) => {
                let actor = self.expression(indent + 1)?;
                let Expression::Actor(actor) = actor else {
                    return Ok(Expression::Unsupported(self.unsupported_range(
                        at,
                        end,
                        "installed category-3 ID 8 requires an actor operand".into(),
                    )?));
                };
                Expression::ActorLocalSourceXYDistance(Box::new(Expression::Actor(actor)))
            }
            (3, 7) => Expression::ActorLocalDistance {
                receiver: ActorTarget::Keyword(ActorKeyword::SelfActor),
                target: Box::new(self.expression(indent + 1)?),
                vertical: true,
            },
            (3, 13) => Expression::DistanceToPosition {
                receiver: None,
                point: Box::new(self.expression(indent + 1)?),
            },
            (3, 192) => Expression::ActorRelativeZ {
                receiver: ActorTarget::Keyword(ActorKeyword::SelfActor),
                target: Box::new(self.expression(indent + 1)?),
            },
            (1, 98) => Expression::IsAbovePerso {
                receiver: ActorTarget::Keyword(ActorKeyword::SelfActor),
                point: Box::new(self.expression(indent + 1)?),
                threshold: Box::new(self.expression(indent + 1)?),
            },
            (3, 198) => Expression::InventoryGold,
            (3, 210) => Expression::InventoryMaximum(Box::new(self.expression(indent + 1)?)),
            (3, 23) => Expression::Sine(Box::new(self.expression(indent + 1)?)),
            (3, 24) => Expression::Cosine(Box::new(self.expression(indent + 1)?)),
            (3, 25) => Expression::Square(Box::new(self.expression(indent + 1)?)),
            (3, 22) => Expression::IntegerCast(Box::new(self.expression(indent + 1)?)),
            (3, 33) => Expression::AbsoluteValue(Box::new(self.expression(indent + 1)?)),
            (3, 88) => Expression::VectorNorm(Box::new(self.expression(indent + 1)?)),
            (3, 158) => Expression::VectorNormalize(Box::new(self.expression(indent + 1)?)),
            (3, 224) => Expression::ApplyArmoredDamage(Box::new(self.expression(indent + 1)?)),
            (3, 226) => Expression::ArmorResistance,
            (3, 227) => Expression::ArmorCount,
            (3, 35) => {
                let actor = self.expression(indent + 1)?;
                if matches!(
                    actor,
                    Expression::Unsupported(_) | Expression::Nobody | Expression::Nowhere
                ) {
                    return Ok(Expression::Unsupported(self.unsupported_range(
                        at,
                        end,
                        "installed category-3 ID35 needs a resolved actor operand".into(),
                    )?));
                }
                if actor == Expression::Actor(ActorTarget::Keyword(ActorKeyword::SelfActor)) {
                    Expression::SelfHitPoints
                } else {
                    Expression::ActorHitPoints(Box::new(actor))
                }
            }
            (3, 1) => Expression::ActorFacingAngle {
                receiver: ActorTarget::Keyword(ActorKeyword::SelfActor),
                target: Box::new(self.expression(indent + 1)?),
            },
            (3, 38) => Expression::ActorHitPointsMaximum(Box::new(self.expression(indent + 1)?)),
            (3, 37) => Expression::SubtractAndGetHitPoints {
                actor: Box::new(self.expression(indent + 1)?),
                value: Box::new(self.expression(indent + 1)?),
            },
            (3, 225) => Expression::FightSubtractAndGetHitPoints(Box::new(
                Expression::SubtractAndGetHitPoints {
                    actor: Box::new(self.expression(indent + 1)?),
                    value: Box::new(self.expression(indent + 1)?),
                },
            )),
            // Func184's sibling in 005398b0: distance from an actor to a waypoint.
            (3, 194) => {
                let actor = self.expression(indent + 1)?;
                let waypoint = Box::new(self.expression(indent + 1)?);
                if let Expression::Actor(actor) = actor {
                    Expression::DistanceToWaypoint { actor, waypoint }
                } else {
                    Expression::Unsupported(
                        self.unsupported(at, "DistanceCaracToWP requires an actor operand".into())?,
                    )
                }
            }
            // 00534af0 cases 0x4e, 0x4f: a segment against sector geometry, from
            // the actor's sight point plus offsets.
            (1, 78) => {
                let target = Box::new(self.expression(indent + 1)?);
                let offset = Box::new(self.expression(indent + 1)?);
                Expression::SeePositionWithOffset { target, offset }
            }
            (1, 79) => {
                let offset = Box::new(self.expression(indent + 1)?);
                let target = Box::new(self.expression(indent + 1)?);
                let target_offset = Box::new(self.expression(indent + 1)?);
                Expression::SeePersoWithOffset {
                    offset,
                    target,
                    target_offset,
                }
            }
            (3, getter @ (54 | 55 | 58 | 60 | 70)) => {
                let selector = Box::new(self.expression(indent + 1)?);
                let actor = self.expression(indent + 1)?;
                if let Expression::Actor(receiver) = actor {
                    Expression::CameraValue {
                        getter,
                        selector,
                        receiver,
                    }
                } else {
                    Expression::Unsupported(
                        self.unsupported(
                            at,
                            "Function_Camera_Get* requires a Perso receiver".into(),
                        )?,
                    )
                }
            }
            (3, 59) => {
                let selector = Box::new(self.expression(indent + 1)?);
                let actor = self.expression(indent + 1)?;
                if let Expression::Actor(receiver) = actor {
                    Expression::CameraShiftAlpha { selector, receiver }
                } else {
                    Expression::Unsupported(self.unsupported(
                        at,
                        "Function_Camera_GetShiftAlpha requires a Perso receiver".into(),
                    )?)
                }
            }
            (3, 61) => {
                let selector = Box::new(self.expression(indent + 1)?);
                let actor = self.expression(indent + 1)?;
                if let Expression::Actor(receiver) = actor {
                    Expression::CameraShiftTeta { selector, receiver }
                } else {
                    Expression::Unsupported(self.unsupported(
                        at,
                        "Function_Camera_GetShiftTeta requires a Perso receiver".into(),
                    )?)
                }
            }
            (3, 184) => Expression::CurrentAnimationFrame,
            (3, 201) => Expression::InventoryWeapon,
            (3, 205) => Expression::InventoryObject,
            (3, 206) => Expression::InventorySelectObject(Box::new(self.expression(indent + 1)?)),
            (3, 196) => Expression::InventoryAddObject(Box::new(self.expression(indent + 1)?)),
            (3, 208) => Expression::InventoryQuantity(Box::new(self.expression(indent + 1)?)),
            (3, 209) => Expression::InventoryRemove {
                item: Box::new(self.expression(indent + 1)?),
                count: Box::new(self.expression(indent + 1)?),
            },
            (3, 228) => Expression::MagicPoints,
            (3, 229) => Expression::MagicAddPoints(Box::new(self.expression(indent + 1)?)),
            (3, 231) => Expression::MagicPointsMaximum,
            (3, 230) => Expression::MagicRemovePoints(Box::new(self.expression(indent + 1)?)),
            (3, 232) => Expression::MagicCurrentSpell,
            (3, 211) => Expression::LaserDirection,
            (3, 172) => Expression::VectorCos(
                Box::new(self.expression(indent + 1)?),
                Box::new(self.expression(indent + 1)?),
            ),
            (3, 171) => Expression::VectorAngle {
                first: Box::new(self.expression(indent + 1)?),
                second: Box::new(self.expression(indent + 1)?),
                sign_axis: Box::new(self.expression(indent + 1)?),
            },
            (3, 164) => Expression::SelfSighting,
            (3, 0) => Expression::SelfPosition,
            (3, 3) => {
                let first = self.at;
                let (operand, point) = self.take(indent + 1)?;
                let literal = point.kind == 29;
                if literal && let Some(reference) = self.codec.waypoints.get(&operand) {
                    Expression::WaypointPosition(reference.clone())
                } else if !literal {
                    self.at = first;
                    Expression::WaypointValuePosition(Box::new(self.expression(indent + 1)?))
                } else {
                    Expression::Unsupported(
                        self.unsupported(at, "unbound absolute waypoint position".into())?,
                    )
                }
            }
            (3, 11) => Expression::DistanceToWaypoint {
                actor: ActorTarget::Keyword(ActorKeyword::SelfActor),
                waypoint: Box::new(self.expression(indent + 1)?),
            },
            (3, 12) => {
                let (_, slot) = self.take(indent + 1)?;
                Expression::CloserRouteWaypoint(slot.parameter.try_into()?)
            }
            (1, 68) => Expression::RouteWaypointCount(self.route_slot(indent + 1, at)?),
            (3, 41) => Expression::GameTime,
            (3, 220) => {
                let origin = Box::new(self.expression(indent + 1)?);
                let (array_at, array) = self.take(indent + 1)?;
                // The source reads this node's id byte without evaluating it.
                ensure!(
                    array.kind == 11,
                    "ReduceIndexOfVectorArray needs a designer variable operand at node {array_at}"
                );
                let array = u16::try_from(array.parameter & 0xff)?;
                Expression::FirstVisibleArrayPoint {
                    origin,
                    array,
                    offset: Box::new(self.expression(indent + 1)?),
                    count: Box::new(self.expression(indent + 1)?),
                }
            }
            (1, 30) => Expression::CollideMovingZdm {
                zone: Box::new(self.expression(indent + 1)?),
                offset: Box::new(self.expression(indent + 1)?),
            },
            (3, 153 | 174) => Expression::CollideNormal {
                ground: id == 174,
            },
            (3, 195) => Expression::GameState,
            (3, 27) => Expression::RandomInt {
                lower: Box::new(self.expression(indent + 1)?),
                upper: Box::new(self.expression(indent + 1)?),
            },
            _ => Expression::Unsupported(self.unsupported(
                at,
                format!("unsupported AI expression category {kind} operation {id} at node {at}"),
            )?),
        };
        ensure!(
            self.at == end,
            "unexpected AI expression child at node {}",
            self.at
        );
        Ok(value)
    }
}
struct Writer<'a> {
    nodes: Vec<NodeWire>,
    codec: &'a AiScriptCodec,
}
impl Writer<'_> {
    fn camera_node(&mut self, camera: &ActorTarget, indent: u8) -> Result<()> {
        match camera {
            ActorTarget::Reference(actor) => self.actor_node(actor, indent),
            ActorTarget::Keyword(ActorKeyword::SelfActor) => {
                self.node(0, 4, indent);
                Ok(())
            }
            _ => bail!("camera operand must be a referenced actor or Self"),
        }
    }
    fn actor_node(&mut self, actor: &AssetRef<Perso>, indent: u8) -> Result<()> {
        let handle = self
            .codec
            .actor_handles
            .iter()
            .find(|(r, _)| r == actor)
            .context("missing actor handle")?
            .1;
        self.node(24, handle, indent);
        Ok(())
    }
    fn route_slot(&mut self, slot: &RouteSlot, indent: u8) {
        match slot {
            RouteSlot::Slot(slot) => self.node(23, u32::from(*slot), indent),
            RouteSlot::Variable(variable) => self.node(11, u32::from(*variable), indent),
        }
    }
    fn node(&mut self, kind: u8, parameter: u32, indent: u8) {
        self.nodes.push(NodeWire {
            parameter,
            reserved: [0, 0],
            indent,
            kind,
        });
    }
    fn block(&mut self, values: &[Statement], indent: u8) -> Result<()> {
        ensure!(indent <= 64, "AI tree too deep to encode");
        for (index, s) in values.iter().enumerate() {
            if matches!(values.get(index + 1), Some(Statement::HoldAfterPrevious)) {
                self.node(4, 6, indent);
                self.node(12, 1, indent + 1);
            }
            match s {
                Statement::HoldAfterPrevious => {}
                Statement::FightSubHitPointsEnemy(Expression::FightSubtractAndGetHitPoints(inner)) => {
                    let Expression::SubtractAndGetHitPoints { actor, value } = &**inner else {
                        bail!("Fight_SubHitPointsEnemy lacks its actor and value");
                    };
                    self.node(4, 432, indent);
                    self.expression(actor, indent + 1)?;
                    self.expression(value, indent + 1)?;
                }
                Statement::FightSubHitPointsEnemy(_) => bail!("malformed Fight_SubHitPointsEnemy"),
                Statement::Evaluate(value) => {
                    ensure!(
                        matches!(
                            value,
                            Expression::InventoryAddObject(_)
                                | Expression::MagicAddPoints(_)
                                | Expression::DistanceToPosition { .. }
                        ),
                        "unsupported standalone function"
                    );
                    self.expression(value, indent)?;
                }
                Statement::SetHitPointsMaximum { actor, value }
                | Statement::SetHitPoints { actor, value }
                | Statement::AddHitPoints { actor, value }
                | Statement::SubtractHitPoints { actor, value } => {
                    self.node(
                        4,
                        if matches!(s, Statement::AddHitPoints { .. }) {
                            55
                        } else if matches!(s, Statement::SubtractHitPoints { .. }) {
                            56
                        } else {
                            57
                        },
                        indent,
                    );
                    self.expression(actor, indent + 1)?;
                    self.expression(value, indent + 1)?;
                }
                Statement::PlayerIsDead => self.node(4, 0, indent),
                Statement::SetMagicPointsMaximum(value) => {
                    self.node(4, 434, indent);
                    self.expression(value, indent + 1)?;
                }
                Statement::SetInventoryMaximum { item, value } => {
                    self.node(4, 418, indent);
                    self.expression(item, indent + 1)?;
                    self.expression(value, indent + 1)?;
                }
                Statement::ActivateMapName { name, active } => {
                    self.node(4, 376, indent);
                    self.expression(name, indent + 1)?;
                    self.expression(active, indent + 1)?;
                }
                Statement::Unsupported(_) => {
                    bail!("unsupported AI fragment cannot be reconstructed")
                }
                Statement::If {
                    condition,
                    then,
                    otherwise,
                } => {
                    self.node(0, 0, indent);
                    self.expression(condition, indent + 1)?;
                    self.node(0, 1, indent);
                    self.block(then, indent + 1)?;
                    if let Some(v) = otherwise {
                        self.node(0, 2, indent);
                        self.block(v, indent + 1)?;
                    }
                }
                Statement::Assign { variable, value } => {
                    self.node(2, 11, indent);
                    self.node(11, u32::from(*variable), indent + 1);
                    self.expression(value, indent + 1)?;
                }
                Statement::ActorFieldAssign {
                    actor,
                    variable,
                    value,
                } => {
                    self.node(2, 11, indent);
                    self.node(2, 12, indent + 1);
                    let handle = self
                        .codec
                        .actor_handles
                        .iter()
                        .find(|(reference, _)| reference == actor)
                        .context("missing assigned AI actor handle")?
                        .1;
                    self.node(24, handle, indent + 2);
                    self.node(11, u32::from(*variable), indent + 2);
                    self.expression(value, indent + 1)?;
                }
                Statement::MainActorFieldAssign { variable, value } => {
                    self.node(2, 11, indent);
                    self.node(2, 12, indent + 1);
                    self.node(0, 5, indent + 2);
                    self.node(11, u32::from(*variable), indent + 2);
                    self.expression(value, indent + 1)?;
                }
                Statement::SubtractAssign { variable, value } => {
                    self.node(2, 6, indent);
                    self.node(11, u32::from(*variable), indent + 1);
                    self.expression(value, indent + 1)?;
                }
                Statement::MultiplyAssign { variable, value } => {
                    self.node(2, 7, indent);
                    self.node(11, u32::from(*variable), indent + 1);
                    self.expression(value, indent + 1)?;
                }
                Statement::WaitBoolean { field, expected } => {
                    self.node(5, 38, indent);
                    self.expression(field, indent + 1)?;
                    self.expression(expected, indent + 1)?;
                }
                Statement::SetStockCount { actor, count } => {
                    self.node(4, 420, indent);
                    self.expression(actor, indent + 1)?;
                    self.expression(count, indent + 1)?;
                }
                Statement::DisplayPickup {
                    item,
                    milliseconds,
                    from,
                    to,
                    x,
                    y,
                    width,
                    height,
                } => {
                    self.node(4, 421, indent);
                    for value in [item, milliseconds, from, to, x, y, width, height] {
                        self.expression(value, indent + 1)?;
                    }
                }
                Statement::Jump => self.node(4, 46, indent),
                Statement::AddAssign { variable, value } => {
                    self.node(2, 5, indent);
                    self.node(11, u32::from(*variable), indent + 1);
                    self.expression(value, indent + 1)?;
                }
                Statement::Increment { target } | Statement::Decrement { target } => {
                    self.node(
                        2,
                        if matches!(s, Statement::Increment { .. }) {
                            9
                        } else {
                            10
                        },
                        indent,
                    );
                    match target {
                        FieldTarget::Variable(variable) => {
                            self.node(11, u32::from(*variable), indent + 1);
                        }
                        FieldTarget::ActorField { actor, variable } => {
                            self.node(2, 12, indent + 1);
                            let handle = self
                                .codec
                                .actor_handles
                                .iter()
                                .find(|(reference, _)| reference == actor)
                                .context("missing adjusted AI actor handle")?
                                .1;
                            self.node(24, handle, indent + 2);
                            self.node(11, u32::from(*variable), indent + 2);
                        }
                        FieldTarget::MainActorField(variable) => {
                            self.node(2, 12, indent + 1);
                            self.node(0, 5, indent + 2);
                            self.node(11, u32::from(*variable), indent + 2);
                        }
                    }
                }
                Statement::Turn {
                    degrees_per_second,
                    per_frame,
                } => {
                    self.node(4, if *per_frame { 215 } else { 214 }, indent);
                    self.expression(degrees_per_second, indent + 1)?;
                }
                Statement::TurnToward(target) => {
                    self.node(4, 217, indent);
                    self.expression(target, indent + 1)?;
                }
                Statement::IncrementalTurnToward { target, frames } => {
                    self.node(5, 23, indent);
                    match target {
                        ActorTarget::Keyword(keyword) => self.node(
                            0,
                            match keyword {
                                ActorKeyword::SelfActor => 4,
                                ActorKeyword::MainActor => 5,
                            },
                            indent + 1,
                        ),
                        ActorTarget::Variable(variable) => {
                            self.node(11, u32::from(*variable), indent + 1)
                        }
                        ActorTarget::Reference(actor) => self.actor_node(actor, indent + 1)?,
                    }
                    self.expression(frames, indent + 1)?;
                }
                Statement::IncrementalTurnPosition { point, frames } => {
                    self.node(5, 19, indent);
                    self.expression(point, indent + 1)?;
                    self.expression(frames, indent + 1)?;
                }
                Statement::GoToPosition {
                    point,
                    radius,
                    horizontal,
                } => {
                    self.node(5, if *horizontal { 33 } else { 32 }, indent);
                    self.expression(point, indent + 1)?;
                    self.expression(radius, indent + 1)?;
                }
                Statement::GoToWaypoint { waypoint } => {
                    self.node(5, 25, indent);
                    self.expression(waypoint, indent + 1)?;
                }
                Statement::PursueActor { actor, radius } => {
                    self.node(5, 2, indent);
                    self.expression(actor, indent + 1)?;
                    self.expression(radius, indent + 1)?;
                }
                Statement::ActivateActorAtPosition { actor, point } => {
                    self.node(4, 12, indent);
                    self.expression(actor, indent + 1)?;
                    self.expression(point, indent + 1)?;
                }
                Statement::CameraGoToWaypoint { camera, waypoint } => {
                    self.node(5, 49, indent);
                    self.expression(camera, indent + 1)?;
                    self.expression(waypoint, indent + 1)?;
                }
                Statement::SetVectorSpeed(value) => {
                    self.node(4, 29, indent);
                    self.expression(value, indent + 1)?;
                }
                Statement::MoveLift {
                    from,
                    to,
                    duration_seconds,
                } => {
                    self.node(5, 36, indent);
                    self.expression(from, indent + 1)?;
                    self.expression(to, indent + 1)?;
                    self.expression(duration_seconds, indent + 1)?;
                }
                Statement::AddWind(value) => {
                    self.node(4, 30, indent);
                    self.expression(value, indent + 1)?;
                }
                Statement::ResetRoute(slot)
                | Statement::GoOverRoute(slot)
                | Statement::GoToNextWaypoint(slot) => {
                    if matches!(s, Statement::ResetRoute(_)) {
                        self.node(4, 220, indent);
                    } else if matches!(s, Statement::GoToNextWaypoint(_)) {
                        self.node(5, 30, indent);
                    } else {
                        self.node(5, 27, indent);
                    }
                    self.route_slot(slot, indent + 1);
                }
                Statement::ResetRouteBack(slot) => {
                    self.node(4, 221, indent);
                    self.route_slot(slot, indent + 1);
                }
                Statement::InitRouteAtWaypoint { slot, waypoint } => {
                    self.node(4, 222, indent);
                    self.route_slot(slot, indent + 1);
                    self.expression(waypoint, indent + 1)?;
                }
                Statement::GoRelative => self.node(4, 34, indent),
                Statement::KillSelf => {
                    self.node(4, 226, indent);
                    self.node(0, 4, indent + 1);
                }
                Statement::KillPerso { target } => {
                    self.node(4, 226, indent);
                    match target {
                        KillPersoTarget::SelfActor => self.node(0, 4, indent + 1),
                        KillPersoTarget::MainActor => self.node(0, 5, indent + 1),
                        KillPersoTarget::Reference(actor) => self.actor_node(actor, indent + 1)?,
                        KillPersoTarget::Source(_) => {
                            bail!("KillPerso source identity needs a promoted Perso asset")
                        }
                    }
                }
                Statement::ExecuteFade {
                    from,
                    to,
                    duration_seconds,
                } => {
                    self.node(4, 321, indent);
                    self.expression(from, indent + 1)?;
                    self.expression(to, indent + 1)?;
                    self.expression(duration_seconds, indent + 1)?;
                }
                Statement::SetImpulse(value) => {
                    self.node(4, 23, indent);
                    self.expression(value, indent + 1)?;
                }
                Statement::JumpAbsolute => self.node(4, 47, indent),
                Statement::InventoryUnselectObject => self.node(4, 417, indent),
                Statement::JumpWithoutAddingSpeed => self.node(4, 48, indent),
                Statement::SetParticleGeneratorEnabled(enabled) => {
                    self.node(4, if *enabled { 192 } else { 193 }, indent)
                }
                Statement::SetParticleGenerator(generator) => {
                    let handle = self
                        .codec
                        .particle_generator_handles
                        .iter()
                        .find(|(r, _)| r == generator)
                        .context("missing particle-generator source handle")?
                        .1;
                    self.node(4, 194, indent);
                    self.node(37, handle, indent + 1);
                }
                Statement::SetLaserPointerActive(active) => {
                    self.node(4, if *active { 337 } else { 338 }, indent)
                }
                Statement::SetLaserPointerSniperMode(enabled) => {
                    self.node(4, 342, indent);
                    self.expression(enabled, indent + 1)?;
                }
                Statement::SetLaserPointerFlyMode(enabled) => {
                    self.node(4, 343, indent);
                    self.expression(enabled, indent + 1)?;
                }
                Statement::SetDisplayVisible(visible) => {
                    self.node(4, if *visible { 267 } else { 266 }, indent)
                }
                Statement::SetActorDisplayVisible { actor, visible } => {
                    self.node(2, 24, indent);
                    match actor {
                        ActorTarget::Keyword(keyword) => self.node(
                            0,
                            if *keyword == ActorKeyword::MainActor {
                                5
                            } else {
                                4
                            },
                            indent + 1,
                        ),
                        ActorTarget::Variable(variable) => {
                            self.node(11, u32::from(*variable), indent + 1)
                        }
                        ActorTarget::Reference(actor) => self.actor_node(actor, indent + 1)?,
                    }
                    self.node(4, if *visible { 267 } else { 266 }, indent + 1);
                }
                Statement::SetActorChannelActive {
                    actor,
                    channel,
                    active,
                } => {
                    self.node(2, 24, indent);
                    match actor {
                        ActorTarget::Keyword(keyword) => self.node(
                            0,
                            if *keyword == ActorKeyword::MainActor {
                                5
                            } else {
                                4
                            },
                            indent + 1,
                        ),
                        ActorTarget::Variable(variable) => {
                            self.node(11, u32::from(*variable), indent + 1)
                        }
                        ActorTarget::Reference(actor) => self.actor_node(actor, indent + 1)?,
                    }
                    self.node(4, if *active { 104 } else { 105 }, indent + 1);
                    self.node(18, *channel, indent + 2);
                }
                Statement::SetTransparency(value) => {
                    self.node(4, 187, indent);
                    self.expression(value, indent + 1)?;
                }
                Statement::SetTransparentDisplay(enabled) => {
                    self.node(4, if *enabled { 185 } else { 186 }, indent);
                }
                Statement::ChangeSighting(direction) => {
                    self.node(4, 445, indent);
                    self.expression(direction, indent + 1)?;
                }
                Statement::ChangeActorSighting { actor, direction } => {
                    self.node(2, 24, indent);
                    self.expression(actor, indent + 1)?;
                    self.node(4, 445, indent + 1);
                    self.expression(direction, indent + 2)?;
                }
                Statement::SetEconomicActorAbsolutePosition { actor, position } => {
                    let inner = if *actor
                        == Expression::Actor(ActorTarget::Keyword(ActorKeyword::SelfActor))
                    {
                        indent
                    } else {
                        self.node(2, 24, indent);
                        self.expression(actor, indent + 1)?;
                        indent + 1
                    };
                    self.node(4, 415, inner);
                    self.expression(position, inner + 1)?;
                }
                Statement::CameraChangeTargetedPerso { camera, target } => {
                    self.node(4, 160, indent);
                    self.camera_node(camera, indent + 1)?;
                    match target {
                        ActorTarget::Keyword(keyword) => self.node(
                            0,
                            if *keyword == ActorKeyword::MainActor {
                                5
                            } else {
                                4
                            },
                            indent + 1,
                        ),
                        ActorTarget::Variable(variable) => {
                            self.node(11, u32::from(*variable), indent + 1)
                        }
                        ActorTarget::Reference(actor) => self.actor_node(actor, indent + 1)?,
                    }
                }
                Statement::CameraResetTargetedPerso { camera }
                | Statement::CameraReset { camera } => {
                    self.node(
                        4,
                        if matches!(s, Statement::CameraReset { .. }) {
                            136
                        } else {
                            163
                        },
                        indent,
                    );
                    self.camera_node(camera, indent + 1)?;
                }
                Statement::CameraChangeState { camera, state } => {
                    self.node(4, 157, indent);
                    self.camera_node(camera, indent + 1)?;
                    self.expression(state, indent + 1)?;
                }
                Statement::CameraChangeFlag {
                    camera,
                    group,
                    operation,
                    selector,
                } => {
                    self.node(4, 166, indent);
                    self.camera_node(camera, indent + 1)?;
                    self.expression(group, indent + 1)?;
                    self.expression(operation, indent + 1)?;
                    self.expression(selector, indent + 1)?;
                }
                Statement::SetActorAbsolutePosition { actor, position } => {
                    self.node(2, 24, indent);
                    self.actor_node(actor, indent + 1)?;
                    self.node(4, 253, indent + 1);
                    self.expression(position, indent + 2)?;
                }
                Statement::SetKeywordAbsolutePosition { actor, position } => {
                    self.node(2, 24, indent);
                    self.node(
                        0,
                        if *actor == ActorKeyword::MainActor {
                            5
                        } else {
                            4
                        },
                        indent + 1,
                    );
                    self.node(4, 253, indent + 1);
                    self.expression(position, indent + 2)?;
                }
                Statement::SetSelfAbsolutePosition(position) => {
                    self.node(4, 253, indent);
                    self.expression(position, indent + 1)?;
                }
                Statement::CameraUpdatePosition {
                    preceded_by_return_zero,
                } => {
                    if *preceded_by_return_zero {
                        self.node(4, 6, indent);
                        self.node(12, 0, indent + 1);
                    }
                    self.node(4, 135, indent);
                }
                Statement::ActorListAdd { list, actor } => {
                    let handle = self
                        .codec
                        .actor_handles
                        .iter()
                        .find(|(reference, _)| reference == actor)
                        .context("missing list actor handle")?
                        .1;
                    self.node(4, 69, indent);
                    self.node(11, u32::from(*list), indent + 1);
                    self.node(24, handle, indent + 1);
                }
                Statement::SetChannelActive { channel, active } => {
                    self.node(4, if *active { 104 } else { 105 }, indent);
                    self.node(18, *channel, indent + 1);
                }
                Statement::ChangeMap { level } => {
                    self.node(4, 5, indent);
                    let handle = self
                        .codec
                        .string_handles
                        .iter()
                        .find(|(value, _)| value == level)
                        .context("missing map name handle")?
                        .1;
                    self.node(20, handle, indent + 1);
                }
                Statement::ChangeMapAtPosition {
                    level,
                    entry,
                    camera_entry,
                } => {
                    self.node(4, 13, indent);
                    for value in [level, entry, camera_entry] {
                        let handle = self
                            .codec
                            .string_handles
                            .iter()
                            .find(|(candidate, _)| candidate == value)
                            .with_context(|| {
                                format!("missing map entry string handle for {value:?}")
                            })?
                            .1;
                        self.node(20, handle, indent + 1);
                    }
                }
                Statement::ToggleWideScreen => self.node(4, 323, indent),
                Statement::SetGameState(state) => {
                    self.node(4, 312, indent);
                    self.expression(state, indent + 1)?;
                }
                Statement::SetFadeTexture(texture) => {
                    self.node(4, 318, indent);
                    self.expression(texture, indent + 1)?;
                }
                Statement::SetFadeValue(value) => {
                    self.node(4, 317, indent);
                    self.expression(value, indent + 1)?;
                }
                Statement::ActivateFade => self.node(4, 319, indent),
                Statement::DeactivateFade => self.node(4, 320, indent),
                Statement::ActorChangeAction { actor, action } => {
                    self.node(2, 24, indent);
                    self.expression(&Expression::Actor(actor.clone()), indent + 1)?;
                    self.node(4, 218, indent + 1);
                    self.expression(action, indent + 2)?;
                }
                Statement::ActorSetTransparentDisplay { actor, enabled } => {
                    self.node(2, 24, indent);
                    self.expression(&Expression::Actor(actor.clone()), indent + 1)?;
                    self.node(4, if *enabled { 185 } else { 186 }, indent + 1);
                }
                Statement::ActorSetTransparency { actor, value } => {
                    self.node(2, 24, indent);
                    self.expression(&Expression::Actor(actor.clone()), indent + 1)?;
                    self.node(4, 187, indent + 1);
                    self.expression(value, indent + 2)?;
                }
                Statement::ActorTurnToward { actor, target } => {
                    self.node(2, 24, indent);
                    self.expression(&Expression::Actor(actor.clone()), indent + 1)?;
                    self.node(4, 217, indent + 1);
                    self.expression(target, indent + 2)?;
                }
                Statement::SetCustomBit { actor, bit, value } => {
                    if *actor == ActorTarget::Keyword(ActorKeyword::SelfActor) {
                        self.node(4, 273, indent);
                        self.expression(bit, indent + 1)?;
                        self.expression(value, indent + 1)?;
                    } else {
                        self.node(2, 24, indent);
                        self.expression(&Expression::Actor(actor.clone()), indent + 1)?;
                        self.node(4, 273, indent + 1);
                        self.expression(bit, indent + 2)?;
                        self.expression(value, indent + 2)?;
                    }
                }
                Statement::SetChannelParent {
                    parent,
                    channel,
                    child,
                    attached,
                } => {
                    self.node(4, if *attached { 268 } else { 269 }, indent);
                    self.expression(parent, indent + 1)?;
                    self.node(18, *channel, indent + 1);
                    self.expression(child, indent + 1)?;
                }
                Statement::SelectViewportCamera {
                    viewport,
                    previous_policy,
                    placement_policy,
                    actor,
                } => {
                    self.node(4, 141, indent);
                    for value in [viewport, previous_policy, placement_policy, actor] {
                        self.expression(value, indent + 1)?;
                    }
                }
                Statement::ChangeCameraFocal {
                    actor,
                    selector,
                    value,
                } => {
                    self.node(4, 158, indent);
                    for value in [actor, selector, value] {
                        self.expression(value, indent + 1)?;
                    }
                }
                Statement::ChangeCameraAngle {
                    actor,
                    field,
                    mode,
                    value,
                } => {
                    self.node(4, 153, indent);
                    for value in [actor, field, mode, value] {
                        self.expression(value, indent + 1)?;
                    }
                }
                Statement::ChangeCameraDistance {
                    actor,
                    mode,
                    minimum,
                    maximum,
                } => {
                    self.node(4, 143, indent);
                    for value in [actor, mode, minimum, maximum] {
                        self.expression(value, indent + 1)?;
                    }
                }
                Statement::CameraResetVerticalMaximum { actor } => {
                    self.node(4, 171, indent);
                    self.expression(actor, indent + 1)?;
                }
                Statement::SetObjectActive { actor, active } => {
                    self.node(4, if *active { 3 } else { 4 }, indent);
                    match actor {
                        ActorTarget::Keyword(ActorKeyword::SelfActor) => {
                            self.node(0, 4, indent + 1)
                        }
                        ActorTarget::Keyword(ActorKeyword::MainActor) => {
                            self.node(0, 5, indent + 1)
                        }
                        ActorTarget::Variable(variable) => {
                            self.node(11, u32::from(*variable), indent + 1)
                        }
                        ActorTarget::Reference(actor) => self.actor_node(actor, indent + 1)?,
                    }
                }
                Statement::MusicRequest { event } => {
                    let handle = self
                        .codec
                        .sound_handles
                        .iter()
                        .find(|(candidate, _)| candidate == event)
                        .context("missing music event handle")?
                        .1;
                    self.node(4, 129, indent);
                    self.node(34, handle, indent + 1);
                }
                Statement::SwitchMicrophone {
                    actor,
                    evaluated_second,
                } => {
                    self.node(
                        4,
                        if evaluated_second.is_some() { 403 } else { 402 },
                        indent,
                    );
                    self.expression(actor, indent + 1)?;
                    if let Some(second) = evaluated_second {
                        self.expression(second, indent + 1)?;
                    }
                }
                Statement::PositionDialog { x, y } => {
                    self.node(4, 328, indent);
                    self.expression(x, indent + 1)?;
                    self.expression(y, indent + 1)?;
                }
                Statement::RunDialog { name, mode } => {
                    self.node(4, 325, indent);
                    let handle = self
                        .codec
                        .string_handles
                        .iter()
                        .find(|(v, _)| v == name)
                        .context("missing dialogue name handle")?
                        .1;
                    self.node(20, handle, indent + 1);
                    self.expression(mode, indent + 1)?;
                }
                Statement::DisplayVignetteAndReturnToMainMenu {
                    name, milliseconds, ..
                } => {
                    self.node(4, 307, indent);
                    let handle = self
                        .codec
                        .string_handles
                        .iter()
                        .find(|(candidate, _)| candidate == name)
                        .context("missing vignette name handle")?
                        .1;
                    self.node(20, handle, indent + 1);
                    self.expression(milliseconds, indent + 1)?;
                }
                Statement::ScrollDialog(direction) => {
                    self.node(4, 326, indent);
                    self.expression(direction, indent + 1)?;
                }
                Statement::SelectDialog => self.node(4, 327, indent),
                Statement::EndDialog => self.node(4, 324, indent),
                Statement::ChangeDialogBehavior => self.node(4, 330, indent),
                Statement::SetHudVisible { indicator, visible } => {
                    self.node(
                        4,
                        match indicator {
                            HudIndicator::Life => 422,
                            HudIndicator::Armor => 423,
                            HudIndicator::Magic => 424,
                            HudIndicator::SelectedObject => 425,
                            HudIndicator::Weapon => 426,
                            HudIndicator::Cash => 428,
                        },
                        indent,
                    );
                    self.expression(visible, indent + 1)?;
                }
                Statement::SelectArmor(index) => {
                    self.node(4, 429, indent);
                    self.expression(index, indent + 1)?;
                }
                Statement::SetArmorResistance(rank) => {
                    self.node(4, 430, indent);
                    self.expression(rank, indent + 1)?;
                }
                Statement::RepairArmor => self.node(4, 431, indent),
                Statement::SwapConfiguredParts { first, second } => {
                    self.node(4, 82, indent);
                    self.expression(first, indent + 1)?;
                    self.expression(second, indent + 1)?;
                }
                Statement::ChangeConfiguration(value) => {
                    self.node(4, 83, indent);
                    self.expression(value, indent + 1)?;
                }
                Statement::ArrayAssign {
                    array,
                    index,
                    value,
                } => {
                    self.node(2, 11, indent);
                    self.node(2, 26, indent + 1);
                    self.expression(array, indent + 2)?;
                    self.expression(index, indent + 2)?;
                    self.expression(value, indent + 1)?;
                }
                Statement::VectorComponentAssign {
                    vector,
                    axis,
                    value,
                } => {
                    self.node(2, 11, indent);
                    self.node(
                        2,
                        match axis {
                            VectorAxis::X => 13,
                            VectorAxis::Y => 14,
                            VectorAxis::Z => 15,
                        },
                        indent + 1,
                    );
                    self.node(11, u32::from(*vector), indent + 2);
                    self.expression(value, indent + 1)?;
                }
                Statement::Wait { milliseconds } => {
                    self.node(5, 37, indent);
                    self.expression(milliseconds, indent + 1)?;
                }
                Statement::WaitInteger {
                    left,
                    selector,
                    right,
                } => {
                    self.node(5, 40, indent);
                    self.expression(left, indent + 1)?;
                    self.expression(selector, indent + 1)?;
                    self.expression(right, indent + 1)?;
                }
                Statement::WaitFrames(frames) => {
                    self.node(5, 68, indent);
                    self.expression(frames, indent + 1)?;
                }
                Statement::ChangeBehavior {
                    target,
                    pause_after,
                } => {
                    if *pause_after {
                        self.node(4, 6, indent);
                        self.node(12, 1, indent + 1);
                    }
                    self.node(
                        4,
                        match target.group {
                            BehaviorGroup::Normal => 209,
                            BehaviorGroup::Reflex => 210,
                        },
                        indent,
                    );
                    self.behavior(target, indent + 1)?;
                }
                Statement::ChangeBehaviorToPrevious { group, pause_after } => {
                    if *pause_after {
                        self.node(4, 6, indent);
                        self.node(12, 1, indent + 1);
                    }
                    let (id, field) = match group {
                        BehaviorGroup::Normal => (209, 7),
                        BehaviorGroup::Reflex => (210, 8),
                    };
                    self.node(4, id, indent);
                    self.node(9, field, indent + 1);
                }
                Statement::ChangeBehaviorVariable {
                    variable,
                    group,
                    pause_after,
                } => {
                    if *pause_after {
                        self.node(4, 6, indent);
                        self.node(12, 1, indent + 1);
                    }
                    self.node(
                        4,
                        match group {
                            BehaviorGroup::Normal => 209,
                            BehaviorGroup::Reflex => 210,
                        },
                        indent,
                    );
                    self.node(11, u32::from(*variable), indent + 1);
                }
                Statement::ChangeActorBehavior { actor, target } => {
                    self.node(
                        4,
                        if target.group == BehaviorGroup::Normal {
                            207
                        } else {
                            208
                        },
                        indent,
                    );
                    self.expression(&Expression::Actor(actor.clone()), indent + 1)?;
                    self.behavior(target, indent + 1)?;
                }
                Statement::ChangeAction(action) | Statement::ExecuteAction(action) => {
                    let (kind, id) = if matches!(s, Statement::ChangeAction(_)) {
                        (4, 218)
                    } else {
                        (5, 4)
                    };
                    self.node(kind, id, indent);
                    self.expression(action, indent + 1)?;
                }
                Statement::ExecuteActionAndGenerateObject {
                    template,
                    action,
                    vector,
                } => {
                    self.node(5, 15, indent);
                    let handle = self
                        .codec
                        .actor_handles
                        .iter()
                        .find(|(reference, _)| reference == template)
                        .context("missing AI object template handle")?
                        .1;
                    self.node(24, handle, indent + 1);
                    self.expression(action, indent + 1)?;
                    self.expression(vector, indent + 1)?;
                }
                Statement::AdjustPersoOnObject {
                    target,
                    near_threshold,
                    far_threshold,
                    far_action,
                    near_action,
                } => {
                    self.node(5, 17, indent);
                    self.expression(target, indent + 1)?;
                    self.expression(near_threshold, indent + 1)?;
                    self.expression(far_threshold, indent + 1)?;
                    self.expression(far_action, indent + 1)?;
                    self.expression(near_action, indent + 1)?;
                }
                Statement::WaitEndOfAction => self.node(5, 6, indent),
                Statement::WaitEndOfFade => self.node(5, 61, indent),
                Statement::WaitEndOfAnimation => self.node(5, 8, indent),
                Statement::Sound { event } => {
                    self.node(4, 127, indent);
                    let handle = self
                        .codec
                        .sound_handles
                        .iter()
                        .find(|(e, _)| e == event)
                        .context("missing assigned AI sound handle")?
                        .1;
                    self.node(34, handle, indent + 1);
                }
                Statement::SoundOnActor { actor, event } => {
                    self.node(2, 24, indent);
                    self.expression(&Expression::Actor(actor.clone()), indent + 1)?;
                    self.node(4, 127, indent + 1);
                    let handle = self
                        .codec
                        .sound_handles
                        .iter()
                        .find(|(e, _)| e == event)
                        .context("missing assigned AI sound handle")?
                        .1;
                    self.node(34, handle, indent + 2);
                }
                Statement::SoundFadeMusic { arg0, arg1 } => {
                    self.node(4, 408, indent);
                    self.expression(arg0, indent + 1)?;
                    self.expression(arg1, indent + 1)?;
                }
                Statement::SoundFadeInObject {
                    actor,
                    target,
                    duration_ms,
                    event,
                } => {
                    let is_self = *actor == ActorTarget::Keyword(ActorKeyword::SelfActor);
                    let operation_indent = if is_self { indent } else { indent + 1 };
                    if !is_self {
                        self.node(2, 24, indent);
                        self.expression(&Expression::Actor(actor.clone()), indent + 1)?;
                    }
                    self.node(4, 413, operation_indent);
                    self.expression(target, operation_indent + 1)?;
                    self.expression(duration_ms, operation_indent + 1)?;
                    let handle = self
                        .codec
                        .sound_handles
                        .iter()
                        .find(|(candidate, _)| candidate == event)
                        .context("missing assigned object fade sound handle")?
                        .1;
                    self.node(34, handle, operation_indent + 1);
                }
                Statement::SoundSetMyEffectsVolume(value) => {
                    self.node(4, 386, indent);
                    self.expression(value, indent + 1)?;
                }
                Statement::SoundFadeObjectAndStop {
                    actor,
                    target,
                    duration_ms,
                } => {
                    let is_self = *actor == ActorTarget::Keyword(ActorKeyword::SelfActor);
                    let operation_indent = if is_self { indent } else { indent + 1 };
                    if !is_self {
                        self.node(2, 24, indent);
                        self.expression(&Expression::Actor(actor.clone()), indent + 1)?;
                    }
                    self.node(4, 412, operation_indent);
                    self.expression(target, operation_indent + 1)?;
                    self.expression(duration_ms, operation_indent + 1)?;
                }
                Statement::SoundFadeVoice {
                    target,
                    duration_ms,
                } => {
                    self.node(4, 409, indent);
                    self.expression(target, indent + 1)?;
                    self.expression(duration_ms, indent + 1)?;
                }
                Statement::SoundFadeSound {
                    target,
                    duration_ms,
                } => {
                    self.node(4, 410, indent);
                    self.expression(target, indent + 1)?;
                    self.expression(duration_ms, indent + 1)?;
                }
                Statement::SoundFadeAll {
                    target,
                    duration_ms,
                } => {
                    self.node(4, 411, indent);
                    self.expression(target, indent + 1)?;
                    self.expression(duration_ms, indent + 1)?;
                }
                Statement::SetVisualGMTDiffuseCoef { material, value } => {
                    ensure!(
                        matches!(value, Expression::Variable(_)),
                        "Proc_SetVisualGMTDiffuseCoef writer requires a direct designer variable"
                    );
                    let handle = self
                        .codec
                        .material_handles
                        .iter()
                        .find(|(reference, _)| reference == material)
                        .with_context(|| {
                            format!("missing AI visual material handle for {}", material.path)
                        })?
                        .1;
                    self.node(4, 237, indent);
                    self.node(36, handle, indent + 1);
                    self.expression(value, indent + 1)?;
                }
                Statement::SoundKillMusic => self.node(4, 382, indent),
                Statement::Noop => self.node(4, 206, indent),
            }
        }
        Ok(())
    }
    fn behavior(&mut self, target: &BehaviorRef, indent: u8) -> Result<()> {
        let handle = self
            .codec
            .behavior_handles
            .iter()
            .find(|(r, _)| r == target)
            .context("missing assigned AI behavior handle")?
            .1;
        self.node(32, handle, indent);
        Ok(())
    }
    fn expression(&mut self, e: &Expression, indent: u8) -> Result<()> {
        ensure!(indent <= 64, "AI expression too deep to encode");
        match e {
            Expression::Actor(ActorTarget::Variable(variable)) => {
                self.node(11, u32::from(*variable), indent)
            }
            Expression::Unsupported(_) => bail!("unsupported AI fragment cannot be reconstructed"),
            Expression::Actor(ActorTarget::Keyword(actor)) => self.node(
                0,
                if *actor == ActorKeyword::SelfActor {
                    4
                } else {
                    5
                },
                indent,
            ),
            Expression::Actor(ActorTarget::Reference(actor)) => self.actor_node(actor, indent)?,
            Expression::SectorReference(reference) => {
                let handle = if reference.is_none() {
                    0
                } else {
                    self.codec
                        .sector_handles
                        .iter()
                        .find(|(sector, _)| sector == reference)
                        .context("missing assigned AI sector handle")?
                        .1
                };
                self.node(27, handle, indent);
            }
            Expression::Configuration(reference) => {
                let handle = if reference.is_none() {
                    0
                } else {
                    self.codec
                        .configuration_handles
                        .iter()
                        .find(|(value, _)| value == reference)
                        .context("missing assigned AI ObjectTableRef handle")?
                        .1
                };
                ensure!(
                    reference.is_none() || !matches!(handle, 0 | u32::MAX),
                    "nonnull AI configuration has a null handle"
                );
                self.node(35, handle, indent);
            }
            Expression::Nobody => self.node(0, 7, indent),
            Expression::Nowhere => self.node(0, 8, indent),
            Expression::Integer(v) => self.node(12, *v as u32, indent),
            Expression::Real(v) => self.node(13, v.to_bits(), indent),
            Expression::IntegerCast(value) => {
                self.node(3, 22, indent);
                self.expression(value, indent + 1)?;
            }
            Expression::Square(value) => {
                self.node(3, 25, indent);
                self.expression(value, indent + 1)?;
            }
            Expression::AbsoluteValue(value) => {
                self.node(3, 33, indent);
                self.expression(value, indent + 1)?;
            }
            Expression::VectorNorm(value) => {
                self.node(3, 88, indent);
                self.expression(value, indent + 1)?;
            }
            Expression::VectorNormalize(value) => {
                self.node(3, 158, indent);
                self.expression(value, indent + 1)?;
            }
            Expression::IsValidActor(value) => {
                self.node(1, 64, indent);
                self.expression(value, indent + 1)?;
            }
            Expression::ActorDistance(value) => {
                self.node(3, 4, indent);
                self.expression(value, indent + 1)?;
            }
            Expression::ActorDistanceFrom { receiver, target } => {
                self.node(2, 24, indent);
                match receiver {
                    ActorTarget::Keyword(ActorKeyword::SelfActor) => self.node(0, 4, indent + 1),
                    ActorTarget::Keyword(ActorKeyword::MainActor) => self.node(0, 5, indent + 1),
                    ActorTarget::Variable(variable) => {
                        self.node(11, u32::from(*variable), indent + 1)
                    }
                    ActorTarget::Reference(actor) => self.actor_node(actor, indent + 1)?,
                }
                self.node(3, 4, indent + 1);
                self.expression(target, indent + 2)?;
            }
            Expression::InventoryGold => self.node(3, 198, indent),
            Expression::ActorRelativeZ { receiver, target } => {
                if *receiver != ActorTarget::Keyword(ActorKeyword::SelfActor) {
                    self.node(2, 24, indent);
                    match receiver {
                        ActorTarget::Keyword(ActorKeyword::SelfActor) => unreachable!(),
                        ActorTarget::Keyword(ActorKeyword::MainActor) => {
                            self.node(0, 5, indent + 1)
                        }
                        ActorTarget::Variable(variable) => {
                            self.node(11, u32::from(*variable), indent + 1)
                        }
                        ActorTarget::Reference(actor) => self.actor_node(actor, indent + 1)?,
                    }
                    self.node(3, 192, indent + 1);
                    self.expression(target, indent + 2)?;
                } else {
                    self.node(3, 192, indent);
                    self.expression(target, indent + 1)?;
                }
            }
            Expression::ActorHitPoints(value)
            | Expression::InventoryMaximum(value)
            | Expression::Sine(value)
            | Expression::Cosine(value) => {
                self.node(
                    3,
                    match e {
                        Expression::ActorHitPoints(_) => 35,
                        Expression::InventoryMaximum(_) => 210,
                        Expression::Sine(_) => 23,
                        _ => 24,
                    },
                    indent,
                );
                self.expression(value, indent + 1)?;
            }
            Expression::ActorLocalDistance {
                receiver,
                target,
                vertical,
            } => {
                let inner = if *receiver == ActorTarget::Keyword(ActorKeyword::SelfActor) {
                    indent
                } else {
                    self.node(2, 24, indent);
                    self.expression(&Expression::Actor(receiver.clone()), indent + 1)?;
                    indent + 1
                };
                self.node(3, if *vertical { 7 } else { 8 }, inner);
                self.expression(target, inner + 1)?;
            }
            Expression::IsAbovePerso {
                receiver,
                point,
                threshold,
            } => {
                let inner = if *receiver == ActorTarget::Keyword(ActorKeyword::SelfActor) {
                    indent
                } else {
                    self.node(2, 24, indent);
                    self.expression(&Expression::Actor(receiver.clone()), indent + 1)?;
                    indent + 1
                };
                self.node(1, 98, inner);
                self.expression(point, inner + 1)?;
                self.expression(threshold, inner + 1)?;
            }
            Expression::ActorPositionField(actor) | Expression::ActorSightingField(actor) => {
                self.node(2, 12, indent);
                self.expression(&Expression::Actor(actor.clone()), indent + 1)?;
                self.node(
                    9,
                    u32::from(matches!(e, Expression::ActorSightingField(_))),
                    indent + 1,
                );
            }
            Expression::DistanceToPosition { receiver, point } => {
                let inner = if receiver.is_some() {
                    indent + 1
                } else {
                    indent
                };
                if let Some(receiver) = receiver {
                    self.node(2, 24, indent);
                    self.expression(&Expression::Actor(receiver.clone()), inner)?;
                }
                self.node(3, 13, inner);
                self.expression(point, inner + 1)?;
            }
            Expression::ActorLocalSourceXYDistance(value) => {
                self.node(3, 8, indent);
                self.expression(value, indent + 1)?;
            }
            Expression::ApplyArmoredDamage(value) => {
                self.node(3, 224, indent);
                self.expression(value, indent + 1)?;
            }
            Expression::ArmorResistance => self.node(3, 226, indent),
            Expression::ArmorCount => self.node(3, 227, indent),
            Expression::CurrentAnimationFrame => self.node(3, 184, indent),
            Expression::SelfHitPoints => {
                self.node(3, 35, indent);
                self.node(0, 4, indent + 1);
            }
            Expression::ActorHitPointsMaximum(actor) => {
                self.node(3, 38, indent);
                self.expression(actor, indent + 1)?;
            }
            Expression::FightSubtractAndGetHitPoints(inner) => {
                self.node(3, 225, indent);
                if let Expression::SubtractAndGetHitPoints { actor, value } = &**inner {
                    self.expression(actor, indent + 1)?;
                    self.expression(value, indent + 1)?;
                }
            }
            Expression::SeePositionWithOffset { target, offset } => {
                self.node(1, 78, indent);
                self.expression(target, indent + 1)?;
                self.expression(offset, indent + 1)?;
            }
            Expression::SeePersoWithOffset {
                offset,
                target,
                target_offset,
            } => {
                self.node(1, 79, indent);
                self.expression(offset, indent + 1)?;
                self.expression(target, indent + 1)?;
                self.expression(target_offset, indent + 1)?;
            }
            Expression::SubtractAndGetHitPoints { actor, value } => {
                self.node(3, 37, indent);
                self.expression(actor, indent + 1)?;
                self.expression(value, indent + 1)?;
            }
            Expression::ActorPosition(actor) => {
                self.node(2, 24, indent);
                match actor {
                    ActorTarget::Keyword(ActorKeyword::SelfActor) => self.node(0, 4, indent + 1),
                    ActorTarget::Keyword(ActorKeyword::MainActor) => self.node(0, 5, indent + 1),
                    ActorTarget::Variable(variable) => {
                        self.node(11, u32::from(*variable), indent + 1)
                    }
                    ActorTarget::Reference(actor) => self.actor_node(actor, indent + 1)?,
                }
                self.node(3, 0, indent + 1);
            }
            Expression::ActorFacingAngle { receiver, target } => {
                if *receiver != ActorTarget::Keyword(ActorKeyword::SelfActor) {
                    self.node(2, 24, indent);
                    match receiver {
                        ActorTarget::Keyword(ActorKeyword::MainActor) => {
                            self.node(0, 5, indent + 1)
                        }
                        ActorTarget::Keyword(ActorKeyword::SelfActor) => unreachable!(),
                        ActorTarget::Variable(variable) => {
                            self.node(11, u32::from(*variable), indent + 1)
                        }
                        ActorTarget::Reference(actor) => self.actor_node(actor, indent + 1)?,
                    }
                    self.node(3, 1, indent + 1);
                    self.expression(target, indent + 2)?;
                } else {
                    self.node(3, 1, indent);
                    self.expression(target, indent + 1)?;
                }
            }
            Expression::CameraValue {
                getter,
                selector,
                receiver,
            } => {
                self.node(3, *getter, indent);
                self.expression(selector, indent + 1)?;
                match receiver {
                    ActorTarget::Keyword(ActorKeyword::SelfActor) => self.node(0, 4, indent + 1),
                    ActorTarget::Keyword(ActorKeyword::MainActor) => self.node(0, 5, indent + 1),
                    ActorTarget::Variable(variable) => {
                        self.node(11, u32::from(*variable), indent + 1)
                    }
                    ActorTarget::Reference(actor) => self.actor_node(actor, indent + 1)?,
                }
            }
            Expression::CameraShiftAlpha { selector, receiver } => {
                self.node(3, 59, indent);
                self.expression(selector, indent + 1)?;
                match receiver {
                    ActorTarget::Keyword(ActorKeyword::SelfActor) => self.node(0, 4, indent + 1),
                    ActorTarget::Keyword(ActorKeyword::MainActor) => self.node(0, 5, indent + 1),
                    ActorTarget::Variable(variable) => {
                        self.node(11, u32::from(*variable), indent + 1)
                    }
                    ActorTarget::Reference(actor) => self.actor_node(actor, indent + 1)?,
                }
            }
            Expression::CameraShiftTeta { selector, receiver } => {
                self.node(3, 61, indent);
                self.expression(selector, indent + 1)?;
                match receiver {
                    ActorTarget::Keyword(ActorKeyword::SelfActor) => self.node(0, 4, indent + 1),
                    ActorTarget::Keyword(ActorKeyword::MainActor) => self.node(0, 5, indent + 1),
                    ActorTarget::Variable(variable) => {
                        self.node(11, u32::from(*variable), indent + 1)
                    }
                    ActorTarget::Reference(actor) => self.actor_node(actor, indent + 1)?,
                }
            }
            Expression::Vector(values) => {
                self.node(16, 0, indent);
                for value in values.iter() {
                    self.expression(value, indent + 1)?;
                }
            }
            Expression::Variable(v) => self.node(11, u32::from(*v), indent),
            Expression::ActorListContains { list, actor } => {
                self.node(1, 53, indent);
                self.node(11, u32::from(*list), indent + 1);
                self.expression(actor, indent + 1)?;
            }
            Expression::SeePerso { receiver, target } => {
                let inner = if *receiver == ActorKeyword::MainActor {
                    self.node(2, 24, indent);
                    self.node(0, 5, indent + 1);
                    indent + 1
                } else {
                    indent
                };
                self.node(1, 76, inner);
                self.node(
                    0,
                    if *target == ActorKeyword::SelfActor {
                        4
                    } else {
                        5
                    },
                    inner + 1,
                );
            }
            Expression::Modulus(a, b) => {
                self.node(3, 189, indent);
                self.expression(a, indent + 1)?;
                self.expression(b, indent + 1)?;
            }
            Expression::Plus(a, b) | Expression::Minus(a, b) | Expression::Divide(a, b) => {
                self.node(
                    2,
                    match e {
                        Expression::Plus(..) => 0,
                        Expression::Minus(..) => 1,
                        _ => 3,
                    },
                    indent,
                );
                self.expression(a, indent + 1)?;
                self.expression(b, indent + 1)?;
            }
            Expression::VectorPlus(a, b) | Expression::VectorMinus(a, b) => {
                self.node(
                    2,
                    if matches!(e, Expression::VectorPlus(..)) {
                        16
                    } else {
                        17
                    },
                    indent,
                );
                self.expression(a, indent + 1)?;
                self.expression(b, indent + 1)?;
            }
            Expression::VectorScale(a, b) => {
                self.node(2, 18, indent);
                self.expression(a, indent + 1)?;
                self.expression(b, indent + 1)?;
            }
            Expression::Multiply(a, b) => {
                self.node(2, 2, indent);
                self.expression(a, indent + 1)?;
                self.expression(b, indent + 1)?;
            }
            Expression::ActorFieldRead { actor, variable } => {
                self.node(2, 12, indent);
                let handle = self
                    .codec
                    .actor_handles
                    .iter()
                    .find(|(reference, _)| reference == actor)
                    .context("missing AI actor field read handle")?
                    .1;
                self.node(24, handle, indent + 1);
                self.node(11, u32::from(*variable), indent + 1);
            }
            Expression::SelfFieldRead(variable) | Expression::MainActorFieldRead(variable) => {
                self.node(2, 12, indent);
                self.node(
                    0,
                    if matches!(e, Expression::SelfFieldRead(_)) {
                        4
                    } else {
                        5
                    },
                    indent + 1,
                );
                self.node(11, u32::from(*variable), indent + 1);
            }
            Expression::Negate(v) => {
                self.node(2, 4, indent);
                self.expression(v, indent + 1)?;
            }
            Expression::Action(value) => {
                let handle = self
                    .codec
                    .action_handles
                    .iter()
                    .find(|(r, _)| r == value)
                    .context("missing assigned AI action handle")?
                    .1;
                self.node(25, handle, indent);
            }
            Expression::IsInAction(action) => {
                self.node(1, 59, indent);
                self.expression(action, indent + 1)?;
            }
            Expression::DialogIsOver => self.node(1, 121, indent),
            Expression::DialogIsOverAndChangeBehavior => self.node(1, 120, indent),
            Expression::ActionFinished(actor) => {
                self.node(1, 123, indent);
                if let Some(actor) = actor {
                    self.expression(actor, indent + 1)?;
                }
            }
            Expression::HasCustomBit { actor, bit } => {
                if *actor == ActorTarget::Keyword(ActorKeyword::SelfActor) {
                    self.node(1, 107, indent);
                    self.expression(bit, indent + 1)?;
                } else {
                    self.node(2, 24, indent);
                    self.expression(&Expression::Actor(actor.clone()), indent + 1)?;
                    self.node(1, 107, indent + 1);
                    self.expression(bit, indent + 2)?;
                }
            }
            Expression::CollideWithGround => self.node(1, 31, indent),
            Expression::CollideWithNothing => self.node(1, 33, indent),
            Expression::CollideWithWall => self.node(1, 32, indent),
            Expression::CollideWithPerso => self.node(1, 37, indent),
            Expression::CloserRouteWaypoint(slot) => {
                self.node(3, 12, indent);
                self.node(23, *slot as u32, indent + 1);
            }
            Expression::RouteWaypointCount(slot) => {
                self.node(1, 68, indent);
                self.route_slot(slot, indent + 1);
            }
            Expression::IsValidWaypoint(value) => {
                self.node(1, 65, indent);
                self.expression(value, indent + 1)?;
            }
            Expression::CollidePersoZdeNoWithTypeZde {
                zone,
                material_mask,
            } => {
                self.node(1, 28, indent);
                self.node(0, 4, indent + 1);
                self.node(12, *zone, indent + 1);
                self.node(17, *material_mask, indent + 1);
            }
            Expression::CollideWithSlope => self.node(1, 34, indent),
            Expression::IsTypeOfGmtCollide(kind) => {
                self.node(1, 60, indent);
                self.node(17, *kind, indent + 1);
            }
            Expression::InTopOfJump => self.node(1, 69, indent),
            Expression::TimeElapsed { start, duration } => {
                self.node(1, 55, indent);
                self.expression(start, indent + 1)?;
                self.expression(duration, indent + 1)?;
            }
            Expression::IsInBehavior { actor, target } => {
                self.node(
                    1,
                    if target.group == BehaviorGroup::Normal {
                        56
                    } else {
                        57
                    },
                    indent,
                );
                match actor {
                    ActorTarget::Keyword(ActorKeyword::SelfActor) => self.node(0, 4, indent + 1),
                    ActorTarget::Keyword(ActorKeyword::MainActor) => self.node(0, 5, indent + 1),
                    ActorTarget::Variable(variable) => {
                        self.node(11, u32::from(*variable), indent + 1)
                    }
                    ActorTarget::Reference(actor) if target.group == BehaviorGroup::Normal => {
                        self.actor_node(actor, indent + 1)?
                    }
                    ActorTarget::Reference(_) => {
                        bail!("referenced actor in Reflex behavior query is unsupported")
                    }
                }
                self.behavior(target, indent + 1)?;
            }
            Expression::AbsentButton(query) => {
                self.node(
                    1,
                    match query {
                        ButtonQuery::Pressed => 9,
                        ButtonQuery::JustPressed => 10,
                        ButtonQuery::Released => 11,
                        ButtonQuery::JustReleased => 12,
                    },
                    indent,
                );
                self.node(14, 0, indent + 1);
            }
            Expression::Button { action, query } => {
                self.node(
                    1,
                    match query {
                        ButtonQuery::Pressed => 9,
                        ButtonQuery::JustPressed => 10,
                        ButtonQuery::Released => 11,
                        ButtonQuery::JustReleased => 12,
                    },
                    indent,
                );
                let handle = self
                    .codec
                    .button_handles
                    .iter()
                    .find(|(name, _)| name == action)
                    .context("missing assigned AI button handle")?
                    .1;
                self.node(14, handle, indent + 1);
            }
            Expression::ArrayElement { array, index } => {
                self.node(2, 26, indent);
                self.expression(array, indent + 1)?;
                self.expression(index, indent + 1)?;
            }
            Expression::VectorComponent { vector, axis } => {
                self.node(
                    2,
                    match axis {
                        VectorAxis::X => 13,
                        VectorAxis::Y => 14,
                        VectorAxis::Z => 15,
                    },
                    indent,
                );
                self.expression(vector, indent + 1)?;
            }
            Expression::MainActorDistance => {
                self.node(3, 4, indent);
                self.node(0, 5, indent + 1);
            }
            Expression::InventoryWeapon => self.node(3, 201, indent),
            Expression::InventoryObject => self.node(3, 205, indent),
            Expression::InventorySelectObject(item) => {
                self.node(3, 206, indent);
                self.expression(item, indent + 1)?;
            }
            Expression::InventoryAddObject(actor) => {
                self.node(3, 196, indent);
                self.expression(actor, indent + 1)?;
            }
            Expression::InventoryQuantity(item) => {
                self.node(3, 208, indent);
                self.expression(item, indent + 1)?;
            }
            Expression::InventoryRemove { item, count } => {
                self.node(3, 209, indent);
                self.expression(item, indent + 1)?;
                self.expression(count, indent + 1)?;
            }
            Expression::MagicPoints => self.node(3, 228, indent),
            Expression::MagicPointsMaximum => self.node(3, 231, indent),
            Expression::MagicAddPoints(amount) => {
                self.node(3, 229, indent);
                self.expression(amount, indent + 1)?;
            }
            Expression::MagicRemovePoints(amount) => {
                self.node(3, 230, indent);
                self.expression(amount, indent + 1)?;
            }
            Expression::MagicCurrentSpell => self.node(3, 232, indent),
            Expression::LaserDirection => self.node(3, 211, indent),
            Expression::VectorCos(a, b) => {
                self.node(3, 172, indent);
                self.expression(a, indent + 1)?;
                self.expression(b, indent + 1)?;
            }
            Expression::VectorAngle {
                first,
                second,
                sign_axis,
            } => {
                self.node(3, 171, indent);
                self.expression(first, indent + 1)?;
                self.expression(second, indent + 1)?;
                self.expression(sign_axis, indent + 1)?;
            }
            Expression::SelfSighting => self.node(3, 164, indent),
            Expression::ActorSighting(receiver) => {
                self.node(2, 24, indent);
                self.expression(&Expression::Actor(receiver.clone()), indent + 1)?;
                self.node(3, 164, indent + 1);
            }
            Expression::SelfPosition => self.node(3, 0, indent),
            Expression::SelfPositionField => self.node(9, 0, indent),
            Expression::WaypointReference(reference) => {
                let handle = self
                    .codec
                    .waypoint_handles
                    .iter()
                    .find(|(candidate, _)| candidate == reference)
                    .context("missing Waypoint reference handle")?
                    .1;
                self.node(29, handle, indent);
            }
            Expression::WaypointValuePosition(waypoint) => {
                self.node(3, 3, indent);
                self.expression(waypoint, indent + 1)?;
            }
            Expression::WaypointPosition(reference) => {
                let handle = self
                    .codec
                    .waypoint_handles
                    .iter()
                    .find(|(r, _)| r == reference)
                    .context("missing waypoint handle")?
                    .1;
                self.node(3, 3, indent);
                self.node(29, handle, indent + 1);
            }
            Expression::DistanceToWaypoint { actor, waypoint } => {
                let (function_indent, waypoint_indent) = match actor {
                    ActorTarget::Variable(variable) => {
                        self.node(2, 24, indent);
                        self.node(11, u32::from(*variable), indent + 1);
                        (indent + 1, indent + 2)
                    }
                    ActorTarget::Keyword(ActorKeyword::SelfActor) => (indent, indent + 1),
                    ActorTarget::Keyword(ActorKeyword::MainActor) => {
                        self.node(2, 24, indent);
                        self.node(0, 5, indent + 1);
                        (indent + 1, indent + 2)
                    }
                    ActorTarget::Reference(actor) => {
                        let handle = self
                            .codec
                            .actor_handles
                            .iter()
                            .find(|(reference, _)| reference == actor)
                            .context("missing AI distance receiver handle")?
                            .1;
                        self.node(2, 24, indent);
                        self.node(24, handle, indent + 1);
                        (indent + 1, indent + 2)
                    }
                };
                self.node(3, 11, function_indent);
                self.expression(waypoint, waypoint_indent)?;
            }
            Expression::CollidePersoTypeZdeWithPersoTypeZde {
                opcode,
                first,
                first_mask,
                second,
                second_mask,
            } => {
                self.node(1, *opcode, indent);
                self.expression(first, indent + 1)?;
                if matches!(opcode, 24 | 26) {
                    self.node(17, *first_mask, indent + 1);
                }
                self.expression(second, indent + 1)?;
                if matches!(opcode, 24 | 27) {
                    self.node(17, *second_mask, indent + 1);
                }
            }
            Expression::WaypointSphereContains { actor, waypoint } => {
                self.node(1, 108, indent);
                self.expression(actor, indent + 1)?;
                self.expression(waypoint, indent + 1)?;
            }
            Expression::ActorInSector { actor, sector } => {
                self.node(1, 124, indent);
                self.expression(actor, indent + 1)?;
                self.expression(sector, indent + 1)?;
            }
            Expression::GameTime => self.node(3, 41, indent),
            Expression::FirstVisibleArrayPoint {
                origin,
                array,
                offset,
                count,
            } => {
                self.node(3, 220, indent);
                self.expression(origin, indent + 1)?;
                self.node(11, u32::from(*array), indent + 1);
                self.expression(offset, indent + 1)?;
                self.expression(count, indent + 1)?;
            }
            Expression::CollideMovingZdm { zone, offset } => {
                self.node(1, 30, indent);
                self.expression(zone, indent + 1)?;
                self.expression(offset, indent + 1)?;
            }
            Expression::CollideNormal { ground } => {
                self.node(3, if *ground { 174 } else { 153 }, indent)
            }
            Expression::GameState => self.node(3, 195, indent),
            Expression::RandomInt { lower, upper } => {
                self.node(3, 27, indent);
                self.expression(lower, indent + 1)?;
                self.expression(upper, indent + 1)?;
            }
            Expression::Compare {
                operation,
                left,
                right,
            } => {
                let id = match operation {
                    Comparison::Equal => 3,
                    Comparison::Different => 4,
                    Comparison::Less => 5,
                    Comparison::Greater => 6,
                    Comparison::LessOrEqual => 7,
                    Comparison::GreaterOrEqual => 8,
                };
                self.node(1, id, indent);
                self.expression(left, indent + 1)?;
                self.expression(right, indent + 1)?;
            }
            Expression::And(a, b) | Expression::Or(a, b) => {
                self.node(
                    1,
                    if matches!(e, Expression::And(..)) {
                        0
                    } else {
                        1
                    },
                    indent,
                );
                self.expression(a, indent + 1)?;
                self.expression(b, indent + 1)?;
            }
            Expression::Not(v) => {
                self.node(1, 2, indent);
                self.expression(v, indent + 1)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod partial_tests {
    use super::*;

    #[test]
    fn source_camera_update_return_zero_pair_round_trips() -> Result<()> {
        let (codec, bytes) = partial(&[(4, 6, 1), (12, 0, 2), (4, 135, 1), (8, 0, 0)]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            script.statements.as_slice(),
            [Statement::CameraUpdatePosition {
                preceded_by_return_zero: true
            }]
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn camera_shift_alpha_keeps_selector_and_perso_receiver() -> Result<()> {
        let (codec, bytes) = partial(&[
            (2, 11, 1),
            (11, 0, 2),
            (3, 59, 2),
            (12, 1, 3),
            (0, 4, 3),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            script.statements.as_slice(),
            [Statement::Assign {
                variable: 0,
                value: Expression::CameraShiftAlpha {
                    selector,
                    receiver: ActorTarget::Keyword(ActorKeyword::SelfActor),
                },
            }] if **selector == Expression::Integer(1)
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn camera_shift_teta_keeps_selector_and_perso_receiver() -> Result<()> {
        let (codec, bytes) = partial(&[
            (2, 11, 1),
            (11, 0, 2),
            (3, 61, 2),
            (12, 0, 3),
            (0, 4, 3),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            script.statements.as_slice(),
            [Statement::Assign {
                variable: 0,
                value: Expression::CameraShiftTeta {
                    selector,
                    receiver: ActorTarget::Keyword(ActorKeyword::SelfActor),
                },
            }] if **selector == Expression::Integer(0)
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn source_actor_list_calls_decode_and_reconstruct() -> Result<()> {
        let actor = AssetRef::new("level:/persos/world/0/2")?;
        let (mut codec, bytes) = partial(&[
            (0, 0, 1),
            (1, 53, 2),
            (11, 27, 3),
            (11, 29, 3),
            (0, 1, 1),
            (4, 69, 2),
            (11, 27, 3),
            (24, 0x1234, 3),
            (8, 0, 0),
        ]);
        codec.actors.insert(7, actor.clone());
        codec.actor_handles.push((actor.clone(), 0x1234));
        let script = codec.read(&bytes)?;
        let Statement::If {
            condition, then, ..
        } = &script.statements[0]
        else {
            panic!("lost actor-list condition")
        };
        assert!(matches!(
            condition,
            Expression::ActorListContains { list: 27, actor }
                if **actor == Expression::Variable(29)
        ));
        assert!(matches!(
            &then[0],
            Statement::ActorListAdd { list: 27, actor: target } if target == &actor
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn tunnel_transition_operations_preserve_target_arguments_and_order() -> Result<()> {
        let (mut codec, bytes) = partial(&[
            (4, 321, 1),
            (12, 0, 2),
            (12, 255, 2),
            (13, 2.0_f32.to_bits(), 2),
            (4, 226, 1),
            (24, 0x1234, 2),
            (4, 411, 1),
            (12, 0, 2),
            (12, 2000, 2),
            (5, 37, 1),
            (12, 2000, 2),
            (4, 382, 1),
            (4, 411, 1),
            (12, 127, 2),
            (12, 1, 2),
            (4, 13, 1),
            (20, 0x1234, 2),
            (20, 0x1235, 2),
            (20, 0x1236, 2),
            (8, 0, 0),
        ]);
        let target = SourceObjectIdentity {
            role: SourceRole::Level,
            block: [5, 1],
            allocation: 9217,
        };
        codec.preserved_nodes[5].operand = PreservedAiOperand::Source(Some(target.clone()));
        codec.strings = BTreeMap::from([
            (16, "brigand".into()),
            (17, "i_brigand_dummy_gogood_hype".into()),
            (18, "i_brigand_dummy_gogood_camera".into()),
        ]);
        let script = codec.read(&bytes)?;
        assert!(script.unsupported_fragments().is_empty());
        assert!(matches!(
            script.statements.as_slice(),
            [
                Statement::ExecuteFade {
                    from: Expression::Integer(0),
                    to: Expression::Integer(255),
                    duration_seconds: Expression::Real(2.0),
                },
                Statement::KillPerso {
                    target: KillPersoTarget::Source(source_target),
                },
                Statement::SoundFadeAll {
                    target: Expression::Integer(0),
                    duration_ms: Expression::Integer(2000),
                },
                Statement::Wait {
                    milliseconds: Expression::Integer(2000),
                },
                Statement::SoundKillMusic,
                Statement::SoundFadeAll {
                    target: Expression::Integer(127),
                    duration_ms: Expression::Integer(1),
                },
                Statement::ChangeMapAtPosition {
                    level,
                    entry,
                    camera_entry,
                },
            ] if source_target == &target
                && level == "brigand"
                && entry == "i_brigand_dummy_gogood_hype"
                && camera_entry == "i_brigand_dummy_gogood_camera"
        ));
        Ok(())
    }

    #[test]
    fn func_get_game_state_round_trips_as_a_typed_session_query() -> Result<()> {
        let (codec, bytes) = partial(&[
            (0, 0, 1),
            (1, 3, 2),
            (3, 195, 3),
            (12, 2, 3),
            (0, 1, 1),
            (4, 324, 2),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert!(matches!(script.statements.as_slice(), [Statement::If {
            condition: Expression::Compare { left, .. }, ..
        }] if **left == Expression::GameState));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn proc_set_game_state_preserves_its_long_operand() -> Result<()> {
        let (codec, bytes) = partial(&[(4, 312, 1), (12, 1, 2), (8, 0, 0)]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            script.statements.as_slice(),
            [Statement::SetGameState(Expression::Integer(1))]
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn opening_fade_and_music_calls_round_trip_as_ordered_native_semantics() -> Result<()> {
        let (mut codec, bytes) = partial(&[
            (4, 312, 1),
            (12, 1, 2),
            (4, 318, 1),
            (12, 0, 2),
            (4, 317, 1),
            (12, 255, 2),
            (4, 319, 1),
            (4, 129, 1),
            (34, 5678, 2),
            (8, 0, 0),
        ]);
        let event = SoundEventLink {
            library: AssetRef::new("sound:/Sound")?,
            bank: 36,
            member: 0,
        };
        codec.sounds.insert(8, Some(event.clone()));
        codec.sound_handles.push((Some(event.clone()), 5678));
        let script = codec.read(&bytes)?;
        assert_eq!(
            script.statements,
            vec![
                Statement::SetGameState(Expression::Integer(1)),
                Statement::SetFadeTexture(Expression::Integer(0)),
                Statement::SetFadeValue(Expression::Integer(255)),
                Statement::ActivateFade,
                Statement::MusicRequest { event: Some(event) },
            ]
        );
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn ultra_main_actor_absolute_waypoint_receiver_round_trips() -> Result<()> {
        let (mut codec, bytes) = partial(&[
            (2, 24, 1),
            (0, 5, 2),
            (4, 253, 2),
            (3, 3, 3),
            (29, 9012, 4),
            (8, 0, 0),
        ]);
        let point = AssetRef::<Waypoint>::new("level:/waypoints/17-1/83")?;
        codec.waypoints.insert(4, point.clone());
        codec.waypoint_handles.push((point.clone(), 9012));
        let script = codec.read(&bytes)?;
        assert_eq!(
            script.statements,
            vec![Statement::SetKeywordAbsolutePosition {
                actor: ActorKeyword::MainActor,
                position: Expression::WaypointPosition(point),
            }]
        );
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn source_zde_collision_condition_decodes_and_reconstructs() -> Result<()> {
        let (codec, bytes) = partial(&[
            (0, 0, 1),
            (1, 28, 2),
            (0, 4, 3),
            (12, 1, 3),
            (17, 1, 3),
            (0, 1, 1),
            (4, 206, 2),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            script.statements.as_slice(),
            [Statement::If {
                condition: Expression::CollidePersoZdeNoWithTypeZde {
                    zone: 1,
                    material_mask: 1
                },
                then,
                otherwise: None
            }] if matches!(then.as_slice(), [Statement::Noop])
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    fn partial(nodes: &[(u8, u32, u8)]) -> (AiScriptCodec, Vec<u8>) {
        let mut bytes = Cursor::new(Vec::new());
        let mut preserved_nodes = Vec::new();
        for &(kind, parameter, indent) in nodes {
            NodeWire {
                kind,
                parameter,
                indent,
                reserved: [0; 2],
            }
            .write(&mut bytes)
            .unwrap();
            preserved_nodes.push(PreservedAiNode {
                category: kind,
                indent,
                reserved: [0; 2],
                operand: PreservedAiOperand::NonRelocated(parameter.to_le_bytes()),
            });
        }
        (
            AiScriptCodec {
                preserved_nodes,
                ..Default::default()
            },
            bytes.into_inner(),
        )
    }

    #[test]
    fn dialogue_procedures_and_over_condition_reconstruct_exact_nodes() -> Result<()> {
        let (mut codec, bytes) = partial(&[
            (4, 323, 1),
            (4, 328, 1),
            (12, 0, 2),
            (12, 405, 2),
            (4, 325, 1),
            (20, 1234, 2),
            (12, 1, 2),
            (4, 326, 1),
            (12, u32::MAX, 2),
            (4, 327, 1),
            (0, 0, 1),
            (1, 121, 2),
            (0, 1, 1),
            (4, 330, 2),
            (4, 324, 1),
            (8, 0, 0),
        ]);
        codec.strings.insert(5, "fixture_dialogue".into());
        codec.string_handles.push(("fixture_dialogue".into(), 1234));
        let script = codec.read(&bytes)?;
        assert!(matches!(script.statements[0], Statement::ToggleWideScreen));
        assert!(
            matches!(&script.statements[2], Statement::RunDialog { name, mode } if name == "fixture_dialogue" && *mode == Expression::Integer(1))
        );
        assert!(
            matches!(&script.statements[5], Statement::If { condition: Expression::DialogIsOver, then, .. } if matches!(then.as_slice(), [Statement::ChangeDialogBehavior]))
        );
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn channel_and_hud_requests_decode_and_reconstruct() -> Result<()> {
        let (codec, bytes) = partial(&[
            (4, 105, 1),
            (18, 72, 2),
            (4, 104, 1),
            (18, 28, 2),
            (4, 426, 1),
            (12, 1, 2),
            (4, 428, 1),
            (12, 1, 2),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            script.statements[0],
            Statement::SetChannelActive {
                channel: 72,
                active: false
            }
        ));
        assert!(matches!(
            script.statements[1],
            Statement::SetChannelActive {
                channel: 28,
                active: true
            }
        ));
        assert!(matches!(
            script.statements[2],
            Statement::SetHudVisible {
                indicator: HudIndicator::Weapon,
                visible: Expression::Integer(1)
            }
        ));
        assert!(matches!(
            script.statements[3],
            Statement::SetHudVisible {
                indicator: HudIndicator::Cash,
                visible: Expression::Integer(1)
            }
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn partial_script_retains_branches_and_supported_siblings() -> Result<()> {
        let (codec, bytes) = partial(&[
            (0, 0, 1),
            (12, 0, 2),
            (0, 1, 1),
            (4, 999, 2),
            (12, 4, 3),
            (0, 2, 1),
            (2, 11, 2),
            (11, 0, 3),
            (12, 8, 3),
            (2, 11, 1),
            (11, 0, 2),
            (12, 9, 2),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        let Statement::If {
            then,
            otherwise: Some(otherwise),
            ..
        } = &script.statements[0]
        else {
            panic!("lost branch")
        };
        assert!(
            matches!(&then[0], Statement::Unsupported(fragment) if fragment.start == 3 && fragment.nodes.len() == 2)
        );
        assert!(matches!(
            &otherwise[0],
            Statement::Assign {
                value: Expression::Integer(8),
                ..
            }
        ));
        assert!(matches!(
            &script.statements[1],
            Statement::Assign {
                value: Expression::Integer(9),
                ..
            }
        ));
        assert_eq!(script.unsupported_fragments().len(), 1);
        assert!(codec.write(&script).is_err());
        Ok(())
    }

    #[test]
    fn previous_behavior_field_operand_is_typed_and_reconstructs() -> Result<()> {
        // Proc_ChangeMyComport(PrevComportIntell): field 7 (0x005ffd68 table).
        let (codec, bytes) = partial(&[(4, 209, 1), (9, 7, 2), (8, 0, 0)]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            script.statements[0],
            Statement::ChangeBehaviorToPrevious {
                group: BehaviorGroup::Normal,
                pause_after: false
            }
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn designer_variable_behavior_operand_is_typed_and_reconstructs() -> Result<()> {
        // Proc_ChangeMyComport with a DSG variable operand (a type-13 behavior
        // pointer) is a typed request on that variable, not a fixed behavior.
        let (codec, bytes) = partial(&[(4, 209, 1), (11, 45, 2), (8, 0, 0)]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            script.statements[0],
            Statement::ChangeBehaviorVariable {
                variable: 45,
                group: BehaviorGroup::Normal,
                pause_after: false
            }
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn source_null_reference_keywords_reconstruct_exact_script() -> Result<()> {
        let (codec, bytes) = partial(&[
            (2, 11, 1),
            (11, 176, 2),
            (0, 8, 2),
            (2, 11, 1),
            (11, 175, 2),
            (0, 8, 2),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            &script.statements[..],
            [
                Statement::Assign {
                    variable: 176,
                    value: Expression::Nowhere
                },
                Statement::Assign {
                    variable: 175,
                    value: Expression::Nowhere
                }
            ]
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn vector_components_and_increment_reconstruct_exact_script() -> Result<()> {
        let (codec, bytes) = partial(&[
            (2, 11, 1),
            (11, 168, 2),
            (3, 164, 2),
            (2, 11, 1),
            (11, 153, 2),
            (2, 13, 2),
            (11, 168, 3),
            (2, 11, 1),
            (11, 154, 2),
            (2, 14, 2),
            (11, 168, 3),
            (2, 11, 1),
            (11, 155, 2),
            (2, 15, 2),
            (11, 168, 3),
            (2, 9, 1),
            (11, 132, 2),
            (2, 11, 1),
            (11, 131, 2),
            (3, 27, 2),
            (12, 1800, 3),
            (12, 2500, 3),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            &script.statements[1],
            Statement::Assign {
                variable: 153,
                value: Expression::VectorComponent {
                    axis: VectorAxis::X,
                    ..
                }
            }
        ));
        assert!(matches!(
            &script.statements[4],
            Statement::Increment {
                target: FieldTarget::Variable(132)
            }
        ));
        assert!(matches!(
            &script.statements[5],
            Statement::Assign {
                variable: 131,
                value: Expression::RandomInt { .. }
            }
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn main_actor_field_adjustments_reconstruct_exact_script() -> Result<()> {
        let (codec, bytes) = partial(&[
            (2, 9, 1),
            (2, 12, 2),
            (0, 5, 3),
            (11, 17, 3),
            (2, 10, 1),
            (2, 12, 2),
            (0, 5, 3),
            (11, 17, 3),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            &script.statements[0],
            Statement::Increment {
                target: FieldTarget::MainActorField(17)
            }
        ));
        assert!(matches!(
            &script.statements[1],
            Statement::Decrement {
                target: FieldTarget::MainActorField(17)
            }
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn vector_component_assignments_reconstruct_exact_script() -> Result<()> {
        let (codec, bytes) = partial(&[
            (2, 11, 1),
            (2, 15, 2),
            (11, 2, 3),
            (12, 0, 2),
            (2, 11, 1),
            (2, 15, 2),
            (11, 2, 3),
            (11, 3, 2),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            &script.statements[0],
            Statement::VectorComponentAssign {
                vector: 2,
                axis: VectorAxis::Z,
                value: Expression::Integer(0),
            }
        ));
        assert!(matches!(
            &script.statements[1],
            Statement::VectorComponentAssign {
                vector: 2,
                axis: VectorAxis::Z,
                value: Expression::Variable(3),
            }
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn inventory_functions_reconstruct_exact_script() -> Result<()> {
        let (codec, bytes) = partial(&[
            (2, 11, 1),
            (11, 0, 2),
            (3, 201, 2),
            (2, 11, 1),
            (11, 1, 2),
            (3, 208, 2),
            (12, 80, 3),
            (2, 11, 1),
            (11, 2, 2),
            (3, 209, 2),
            (12, 80, 3),
            (12, 1, 3),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            &script.statements[0],
            Statement::Assign {
                value: Expression::InventoryWeapon,
                ..
            }
        ));
        assert!(matches!(
            &script.statements[1],
            Statement::Assign {
                value: Expression::InventoryQuantity(item),
                ..
            } if **item == Expression::Integer(80)
        ));
        assert!(matches!(
            &script.statements[2],
            Statement::Assign {
                value: Expression::InventoryRemove { item, count },
                ..
            } if **item == Expression::Integer(80) && **count == Expression::Integer(1)
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn magick_functions_reconstruct_exact_script() -> Result<()> {
        let (codec, bytes) = partial(&[
            (2, 11, 1),
            (11, 0, 2),
            (3, 232, 2),
            (2, 11, 1),
            (11, 1, 2),
            (3, 228, 2),
            (2, 11, 1),
            (11, 2, 2),
            (3, 230, 2),
            (12, 10, 3),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            script.statements[0],
            Statement::Assign {
                value: Expression::MagicCurrentSpell,
                ..
            }
        ));
        assert!(matches!(
            script.statements[1],
            Statement::Assign {
                value: Expression::MagicPoints,
                ..
            }
        ));
        assert!(matches!(
            &script.statements[2],
            Statement::Assign {
                value: Expression::MagicRemovePoints(amount),
                ..
            } if **amount == Expression::Integer(10)
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn contact_nothing_condition_reconstructs_exact_script() -> Result<()> {
        let (codec, bytes) = partial(&[(2, 11, 1), (11, 0, 2), (1, 33, 2), (8, 0, 0)]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            script.statements[0],
            Statement::Assign {
                value: Expression::CollideWithNothing,
                ..
            }
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn camera_target_call_preserves_actor_and_main_actor_keyword() -> Result<()> {
        let (mut codec, bytes) = partial(&[(4, 160, 1), (24, 1234, 2), (0, 5, 2), (8, 0, 0)]);
        let camera = AssetRef::<Perso>::new("level:/persos/world/0/2")?;
        codec.actors.insert(1, camera.clone());
        codec.actor_handles.push((camera.clone(), 1234));
        let script = codec.read(&bytes)?;
        assert!(matches!(
            &script.statements[0],
            Statement::CameraChangeTargetedPerso {
                camera: bound,
                target: ActorTarget::Keyword(ActorKeyword::MainActor),
            } if bound == &ActorTarget::Reference(camera.clone())
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn camera_procedure_on_self_keyword_reconstructs() -> Result<()> {
        // fix/437's camera procedures name the camera as Self (kind 0, param 4).
        let (codec, bytes) = partial(&[(4, 136, 1), (0, 4, 2), (8, 0, 0)]);
        let script = codec.read(&bytes)?;
        assert_eq!(
            script.statements,
            vec![Statement::CameraReset {
                camera: ActorTarget::Keyword(ActorKeyword::SelfActor)
            }]
        );
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn camera_reset_target_procedure_reconstructs_and_roundtrips() -> Result<()> {
        let (mut codec, bytes) = partial(&[(4, 163, 1), (24, 1234, 2), (8, 0, 0)]);
        let camera = AssetRef::<Perso>::new("level:/persos/world/0/2")?;
        codec.actors.insert(1, camera.clone());
        codec.actor_handles.push((camera.clone(), 1234));
        let script = codec.read(&bytes)?;
        assert_eq!(
            script.statements,
            vec![Statement::CameraResetTargetedPerso {
                camera: ActorTarget::Reference(camera)
            }]
        );
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn self_absolute_position_procedure_reconstructs_vector_plus_self() -> Result<()> {
        let (codec, bytes) = partial(&[
            (4, 253, 1),
            (2, 16, 2),
            (3, 0, 3),
            (16, 0, 3),
            (13, 2.6_f32.to_bits(), 4),
            (13, 0.0_f32.to_bits(), 4),
            (13, 0.0_f32.to_bits(), 4),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert_eq!(
            script.statements,
            vec![Statement::SetSelfAbsolutePosition(Expression::VectorPlus(
                Box::new(Expression::SelfPosition),
                Box::new(Expression::Vector(Box::new([
                    Expression::Real(2.6),
                    Expression::Real(0.0),
                    Expression::Real(0.0),
                ]))),
            ))]
        );
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn entrance_gate_lift_reconstructs_vector_minus_endpoint() -> Result<()> {
        let (codec, bytes) = partial(&[
            (5, 36, 1),
            (3, 0, 2),
            (2, 17, 2),
            (3, 0, 3),
            (16, 0, 3),
            (13, 0.0_f32.to_bits(), 4),
            (13, 0.0_f32.to_bits(), 4),
            (13, 3.0_f32.to_bits(), 4),
            (12, 5, 2),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert_eq!(
            script.statements,
            vec![Statement::MoveLift {
                from: Expression::SelfPosition,
                to: Expression::VectorMinus(
                    Box::new(Expression::SelfPosition),
                    Box::new(Expression::Vector(Box::new([
                        Expression::Real(0.0),
                        Expression::Real(0.0),
                        Expression::Real(3.0),
                    ]))),
                ),
                duration_seconds: Expression::Integer(5),
            }]
        );
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn action_finished_condition_preserves_its_evaluated_actor_operand() -> Result<()> {
        for keyword in [4, 5] {
            let (codec, bytes) = partial(&[
                (2, 11, 1),
                (11, 0, 2),
                (1, 123, 2),
                (0, keyword, 3),
                (8, 0, 0),
            ]);
            let script = codec.read(&bytes)?;
            let Statement::Assign { value, .. } = &script.statements[0] else {
                panic!()
            };
            assert_eq!(
                value,
                &Expression::ActionFinished(Some(Box::new(Expression::Actor(
                    ActorTarget::Keyword(if keyword == 4 {
                        ActorKeyword::SelfActor
                    } else {
                        ActorKeyword::MainActor
                    })
                ))))
            );
            assert_eq!(codec.write(&script)?, bytes);
        }
        Ok(())
    }

    #[test]
    fn action_finished_condition_without_an_operand_round_trips_as_a_leaf() -> Result<()> {
        // The shape of every Cond123 in the loaded Manor and Fix scripts.
        let (codec, bytes) = partial(&[(2, 11, 1), (11, 0, 2), (1, 123, 2), (8, 0, 0)]);
        let script = codec.read(&bytes)?;
        let Statement::Assign { value, .. } = &script.statements[0] else {
            panic!()
        };
        assert_eq!(value, &Expression::ActionFinished(None));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn camera_shot_commands_reconstruct_bound_actors_waypoint_and_state() -> Result<()> {
        let (mut codec, bytes) = partial(&[
            (4, 160, 1),
            (24, 1234, 2),
            (24, 5678, 2),
            (2, 24, 1),
            (24, 1234, 2),
            (4, 253, 2),
            (3, 3, 3),
            (29, 9012, 4),
            (4, 157, 1),
            (24, 1234, 2),
            (12, 3, 2),
            (5, 37, 1),
            (12, 3600, 2),
            (8, 0, 0),
        ]);
        let camera = AssetRef::<Perso>::new("level:/persos/world/0/2")?;
        let target = AssetRef::<Perso>::new("level:/persos/world/0/33")?;
        let point = AssetRef::<Waypoint>::new("level:/waypoints/17-1/86")?;
        for i in [1, 4, 9] {
            codec.actors.insert(i, camera.clone());
        }
        codec.actors.insert(2, target.clone());
        codec.actor_handles = vec![(camera.clone(), 1234), (target.clone(), 5678)];
        codec.waypoints.insert(7, point.clone());
        codec.waypoint_handles.push((point.clone(), 9012));
        let script = codec.read(&bytes)?;
        assert_eq!(
            script.statements,
            vec![
                Statement::CameraChangeTargetedPerso {
                    camera: ActorTarget::Reference(camera.clone()),
                    target: ActorTarget::Reference(target)
                },
                Statement::SetActorAbsolutePosition {
                    actor: camera.clone(),
                    position: Expression::WaypointPosition(point)
                },
                Statement::CameraChangeState {
                    camera: ActorTarget::Reference(camera),
                    state: Expression::Integer(3)
                },
                Statement::Wait {
                    milliseconds: Expression::Integer(3600)
                },
            ]
        );
        assert_eq!(codec.write(&script)?, bytes);
        assert_eq!(
            rmp_serde::from_slice::<Script>(&rmp_serde::to_vec_named(&script)?)?,
            script
        );
        let old = rmp_serde::to_vec_named(&ActorKeyword::MainActor)?;
        let keyword = ActorTarget::Keyword(ActorKeyword::MainActor);
        assert_eq!(old, rmp_serde::to_vec_named(&keyword)?);
        assert_eq!(rmp_serde::from_slice::<ActorTarget>(&old)?, keyword);
        // An unresolved reference remains preserved; no invented point or actor.
        codec.waypoints.clear();
        assert!(!codec.read(&bytes)?.unsupported_fragments().is_empty());
        // World startup also uses Ultra for NoDisplay. Its exact Perso target
        // must survive as a semantic actor reference, not a guessed position.
        let (mut visible, hidden) = partial(&[(2, 24, 1), (24, 1234, 2), (4, 266, 2), (8, 0, 0)]);
        let hidden_actor = AssetRef::new("level:/persos/world/0/0")?;
        visible.actors.insert(1, hidden_actor.clone());
        visible.actor_handles.push((hidden_actor.clone(), 1234));
        let hidden_script = visible.read(&hidden)?;
        assert_eq!(
            hidden_script.statements,
            vec![Statement::SetActorDisplayVisible {
                actor: ActorTarget::Reference(hidden_actor),
                visible: false,
            }]
        );
        assert_eq!(visible.write(&hidden_script)?, hidden);
        Ok(())
    }

    #[test]
    fn ultra_display_calls_keep_the_resolved_actor_identity() -> Result<()> {
        let (mut codec, bytes) = partial(&[(2, 24, 1), (24, 1234, 2), (4, 266, 2), (8, 0, 0)]);
        let actor = AssetRef::<Perso>::new("level:/persos/world/0/174")?;
        codec.actors.insert(1, actor.clone());
        codec.actor_handles.push((actor.clone(), 1234));

        let script = codec.read(&bytes)?;
        assert_eq!(
            script.statements,
            vec![Statement::SetActorDisplayVisible {
                actor: ActorTarget::Reference(actor),
                visible: false,
            }]
        );
        assert_eq!(codec.write(&script)?, bytes);
        assert_eq!(
            rmp_serde::from_slice::<Script>(&rmp_serde::to_vec_named(&script)?)?,
            script
        );

        let (unbound, unresolved) = partial(&[(2, 24, 1), (24, 1234, 2), (4, 266, 2), (8, 0, 0)]);
        let rejected = unbound.read(&unresolved)?;
        assert!(matches!(
            rejected.statements.as_slice(),
            [Statement::Unsupported(_)]
        ));

        let (mut channel_codec, channel_bytes) = partial(&[
            (2, 24, 1),
            (24, 1234, 2),
            (4, 105, 2),
            (18, 7, 3),
            (8, 0, 0),
        ]);
        let actor = AssetRef::<Perso>::new("level:/persos/world/0/173")?;
        channel_codec.actors.insert(1, actor.clone());
        channel_codec.actor_handles.push((actor.clone(), 1234));
        let channel_script = channel_codec.read(&channel_bytes)?;
        assert_eq!(
            channel_script.statements,
            vec![Statement::SetActorChannelActive {
                actor: ActorTarget::Reference(actor),
                channel: 7,
                active: false,
            }]
        );
        assert_eq!(channel_codec.write(&channel_script)?, channel_bytes);

        let (mut unresolved_codec, unresolved_bytes) = partial(&[
            (2, 24, 1),
            (24, 1234, 2),
            (4, 413, 2),
            (12, 90, 3),
            (12, 80, 3),
            (34, 999, 3),
            (8, 0, 0),
        ]);
        let actor = AssetRef::<Perso>::new("level:/persos/world/0/174")?;
        unresolved_codec.actors.insert(1, actor.clone());
        let unresolved_script = unresolved_codec.read(&unresolved_bytes)?;
        let Statement::Unsupported(fragment) = &unresolved_script.statements[0] else {
            anyhow::bail!("SND call without a bound event was not preserved");
        };
        assert_eq!(
            fragment.receiver,
            Some(ActorTarget::Reference(actor.clone()))
        );
        assert_eq!(
            rmp_serde::from_slice::<Script>(&rmp_serde::to_vec_named(&unresolved_script)?)?,
            unresolved_script
        );
        Ok(())
    }

    #[test]
    fn external_main_actor_behavior_query_reconstructs_exact_script() -> Result<()> {
        let (mut codec, bytes) = partial(&[
            (0, 0, 1),
            (1, 56, 2),
            (0, 5, 3),
            (32, 43_270_544, 3),
            (0, 1, 1),
            (4, 206, 2),
            (8, 0, 0),
        ]);
        let target = BehaviorRef {
            model: AssetRef::new("fix:/ai/models/28")?,
            group: BehaviorGroup::Normal,
            behavior: 94,
        };
        codec.behaviors.insert(3, target.clone());
        codec.behavior_handles.push((target.clone(), 43_270_544));
        let script = codec.read(&bytes)?;
        assert!(matches!(
            &script.statements[0],
            Statement::If {
                condition: Expression::IsInBehavior {
                    actor: ActorTarget::Keyword(ActorKeyword::MainActor),
                    target: bound,
                },
                ..
            } if bound == &target
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn laser_aim_calls_reconstruct_exact_script() -> Result<()> {
        let (codec, bytes) = partial(&[
            (4, 338, 1),
            (2, 11, 1),
            (11, 2, 2),
            (3, 211, 2),
            (2, 11, 1),
            (11, 3, 2),
            (3, 172, 2),
            (11, 2, 3),
            (3, 164, 3),
            (4, 445, 1),
            (11, 2, 2),
            (4, 337, 1),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert_eq!(script.statements.len(), 5);
        assert!(matches!(
            script.statements[0],
            Statement::SetLaserPointerActive(false)
        ));
        assert!(matches!(
            script.statements[1],
            Statement::Assign {
                value: Expression::LaserDirection,
                ..
            }
        ));
        assert!(matches!(
            script.statements[2],
            Statement::Assign {
                value: Expression::VectorCos(..),
                ..
            }
        ));
        assert!(matches!(
            script.statements[3],
            Statement::ChangeSighting(Expression::Variable(2))
        ));
        assert!(matches!(
            script.statements[4],
            Statement::SetLaserPointerActive(true)
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn crossbow_camera_mode_calls_decode_and_reconstruct() -> Result<()> {
        let (codec, bytes) =
            partial(&[(4, 342, 1), (12, 1, 2), (4, 266, 1), (4, 267, 1), (8, 0, 0)]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            script.statements.as_slice(),
            [
                Statement::SetLaserPointerSniperMode(Expression::Integer(1)),
                Statement::SetDisplayVisible(false),
                Statement::SetDisplayVisible(true),
            ]
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn integer_function_reconstructs_crossbow_camera_aim_math() -> Result<()> {
        let (codec, bytes) = partial(&[
            (2, 11, 1),
            (11, 102, 2),
            (3, 22, 2),
            (2, 0, 3),
            (11, 102, 4),
            (12, 5, 4),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            script.statements.as_slice(),
            [Statement::Assign {
                variable: 102,
                value: Expression::IntegerCast(value)
            }] if matches!(value.as_ref(), Expression::Plus(..))
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn jump_motion_requests_reconstruct_source_nodes() -> Result<()> {
        let (codec, bytes) = partial(&[
            (4, 29, 1),
            (16, 0, 2),
            (13, 0, 3),
            (13, 0, 3),
            (13, 0, 3),
            (4, 34, 1),
            (4, 23, 1),
            (16, 0, 2),
            (2, 2, 3),
            (11, 151, 4),
            (13, 1.5_f32.to_bits(), 4),
            (2, 2, 3),
            (11, 152, 4),
            (13, 1.5_f32.to_bits(), 4),
            (13, 9.5_f32.to_bits(), 3),
            (4, 47, 1),
            (4, 48, 1),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            &script.statements[0],
            Statement::SetVectorSpeed(Expression::Vector(_))
        ));
        assert!(matches!(&script.statements[1], Statement::GoRelative));
        assert!(matches!(
            &script.statements[2],
            Statement::SetImpulse(Expression::Vector(_))
        ));
        assert!(matches!(&script.statements[3], Statement::JumpAbsolute));
        assert!(matches!(
            &script.statements[4],
            Statement::JumpWithoutAddingSpeed
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn wind_and_actor_local_route_calls_reconstruct_source_nodes() -> Result<()> {
        let (codec, bytes) = partial(&[
            (4, 30, 1),
            (16, 0, 2),
            (13, 0, 3),
            (13, 0, 3),
            (13, 0, 3),
            (4, 30, 1),
            (16, 0, 2),
            (13, 2.0_f32.to_bits(), 3),
            (13, 0, 3),
            (13, 0, 3),
            (4, 220, 1),
            (23, 0, 2),
            (5, 27, 1),
            (23, 6, 2),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            script.statements.as_slice(),
            [
                Statement::AddWind(Expression::Vector(_)),
                Statement::AddWind(Expression::Vector(_)),
                Statement::ResetRoute(RouteSlot::Slot(0)),
                Statement::GoOverRoute(RouteSlot::Slot(6))
            ]
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn way_procedures_and_way_condition_decode_and_reconstruct() -> Result<()> {
        // The guards (927, 1060): InitWayWithWp(way 0, CloserWP(way 0)),
        // ReInitWayBack and an IsValidWay test.
        let (codec, bytes) = partial(&[
            (4, 222, 1),
            (23, 0, 2),
            (3, 12, 2),
            (23, 0, 3),
            (4, 221, 1),
            (11, 4, 2),
            (0, 0, 1),
            (1, 68, 2),
            (23, 1, 3),
            (0, 1, 1),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert_eq!(
            script.statements,
            vec![
                Statement::InitRouteAtWaypoint {
                    slot: RouteSlot::Slot(0),
                    waypoint: Expression::CloserRouteWaypoint(0),
                },
                Statement::ResetRouteBack(RouteSlot::Variable(4)),
                Statement::If {
                    condition: Expression::RouteWaypointCount(RouteSlot::Slot(1)),
                    then: Vec::new(),
                    otherwise: None,
                },
            ]
        );
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn way_calls_take_a_designer_way_variable() -> Result<()> {
        // The squirrels (816, 1862): ReInitWay and GoOverWay on Way variable 10.
        let (codec, bytes) = partial(&[
            (4, 220, 1),
            (11, 10, 2),
            (5, 27, 1),
            (11, 10, 2),
            (5, 30, 1),
            (23, 1, 2),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert_eq!(
            script.statements,
            vec![
                Statement::ResetRoute(RouteSlot::Variable(10)),
                Statement::GoOverRoute(RouteSlot::Variable(10)),
                Statement::GoToNextWaypoint(RouteSlot::Slot(1)),
            ]
        );
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn incremental_turn_keeps_target_frame_count_and_exact_source_nodes() -> Result<()> {
        for frames in [5, 0, -7] {
            let (mut codec, bytes) =
                partial(&[(5, 23, 1), (24, 1234, 2), (12, frames as u32, 2), (8, 0, 0)]);
            let actor = AssetRef::<Perso>::new("level:/persos/world/0/1")?;
            codec.actors.insert(1, actor.clone());
            codec.actor_handles.push((actor.clone(), 1234));
            let script = codec.read(&bytes)?;
            assert_eq!(
                script.statements,
                vec![Statement::IncrementalTurnToward {
                    target: ActorTarget::Reference(actor),
                    frames: Expression::Integer(frames),
                }]
            );
            assert_eq!(codec.write(&script)?, bytes);
        }
        let (codec, bytes) = partial(&[(5, 23, 1), (0, 5, 2), (12, 5, 2), (8, 0, 0)]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            &script.statements[0],
            Statement::IncrementalTurnToward {
                target: ActorTarget::Keyword(ActorKeyword::MainActor),
                ..
            }
        ));
        assert_eq!(codec.write(&script)?, bytes);
        let (codec, bytes) = partial(&[(5, 23, 1), (24, 1234, 2), (12, 5, 2), (8, 0, 0)]);
        assert!(matches!(
            &codec.read(&bytes)?.statements[0],
            Statement::Unsupported(_)
        ));
        Ok(())
    }

    #[test]
    fn absent_input_queries_preserve_semantics_without_masking_unbound_handles() -> Result<()> {
        for (operation, query) in [
            (9, ButtonQuery::Pressed),
            (10, ButtonQuery::JustPressed),
            (11, ButtonQuery::Released),
            (12, ButtonQuery::JustReleased),
        ] {
            let (codec, bytes) = partial(&[
                (2, 11, 1),
                (11, 0, 2),
                (1, operation, 2),
                (14, 0, 3),
                (8, 0, 0),
            ]);
            let script = codec.read(&bytes)?;
            assert_eq!(
                script.statements,
                vec![Statement::Assign {
                    variable: 0,
                    value: Expression::AbsentButton(query)
                }]
            );
            assert_eq!(codec.write(&script)?, bytes);
        }
        let (codec, bytes) =
            partial(&[(2, 11, 1), (11, 0, 2), (1, 10, 2), (14, 1234, 3), (8, 0, 0)]);
        assert!(matches!(
            &codec.read(&bytes)?.statements[0],
            Statement::Assign {
                value: Expression::Unsupported(_),
                ..
            }
        ));
        Ok(())
    }

    #[test]
    fn jump_queries_and_arithmetic_reconstruct_source_nodes() -> Result<()> {
        let (codec, bytes) = partial(&[
            (2, 11, 1),
            (11, 98, 2),
            (3, 41, 2),
            (2, 11, 1),
            (11, 167, 2),
            (3, 0, 2),
            (2, 11, 1),
            (11, 145, 2),
            (2, 3, 2),
            (2, 1, 3),
            (3, 41, 4),
            (11, 98, 4),
            (13, 1.5_f32.to_bits(), 3),
            (2, 5, 1),
            (11, 141, 2),
            (12, 1, 2),
            (0, 0, 1),
            (1, 60, 2),
            (17, 128, 3),
            (0, 1, 1),
            (4, 206, 2),
            (0, 0, 1),
            (1, 69, 2),
            (0, 1, 1),
            (4, 206, 2),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            &script.statements[0],
            Statement::Assign {
                value: Expression::GameTime,
                ..
            }
        ));
        assert!(matches!(
            &script.statements[1],
            Statement::Assign {
                value: Expression::SelfPosition,
                ..
            }
        ));
        assert!(matches!(
            &script.statements[2],
            Statement::Assign {
                value: Expression::Divide(..),
                ..
            }
        ));
        assert!(matches!(
            &script.statements[3],
            Statement::AddAssign { variable: 141, .. }
        ));
        assert!(matches!(
            &script.statements[4],
            Statement::If {
                condition: Expression::IsTypeOfGmtCollide(128),
                ..
            }
        ));
        assert!(matches!(
            &script.statements[5],
            Statement::If {
                condition: Expression::InTopOfJump,
                ..
            }
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn actor_designer_assignment_reconstructs_dot_target() -> Result<()> {
        let (mut codec, bytes) = partial(&[
            (2, 11, 1),
            (2, 12, 2),
            (24, 1234, 3),
            (11, 17, 3),
            (12, 1, 2),
            (8, 0, 0),
        ]);
        let actor = AssetRef::<Perso>::new("fix:/persos/15")?;
        codec.actors.insert(2, actor.clone());
        codec.actor_handles.push((actor.clone(), 1234));
        let script = codec.read(&bytes)?;
        assert!(matches!(
            &script.statements[0],
            Statement::ActorFieldAssign {
                actor: target,
                variable: 17,
                value: Expression::Integer(1)
            } if *target == actor
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn actor_designer_read_reconstructs_dot_expression() -> Result<()> {
        let (mut codec, bytes) = partial(&[
            (2, 11, 1),
            (11, 0, 2),
            (2, 12, 2),
            (24, 1234, 3),
            (11, 201, 3),
            (8, 0, 0),
        ]);
        let actor = AssetRef::<Perso>::new("fix:/persos/15")?;
        codec.actors.insert(3, actor.clone());
        codec.actor_handles.push((actor.clone(), 1234));
        let script = codec.read(&bytes)?;
        assert!(matches!(
            &script.statements[0],
            Statement::Assign {
                value: Expression::ActorFieldRead { actor: target, variable: 201 },
                ..
            } if *target == actor
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn main_actor_designer_read_reconstructs_dot_expression() -> Result<()> {
        let (codec, bytes) = partial(&[
            (2, 11, 1),
            (11, 0, 2),
            (2, 12, 2),
            (0, 5, 3),
            (11, 56, 3),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            &script.statements[0],
            Statement::Assign {
                value: Expression::MainActorFieldRead(56),
                ..
            }
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn main_actor_designer_assignment_reconstructs_dot_target() -> Result<()> {
        let (codec, bytes) = partial(&[
            (2, 11, 1),
            (2, 12, 2),
            (0, 5, 3),
            (11, 55, 3),
            (12, 1, 2),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            &script.statements[0],
            Statement::MainActorFieldAssign {
                variable: 55,
                value: Expression::Integer(1)
            }
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn kill_self_reconstructs_actor_operand() -> Result<()> {
        let (codec, bytes) = partial(&[(4, 226, 1), (0, 4, 2), (8, 0, 0)]);
        let script = codec.read(&bytes)?;
        assert!(matches!(script.statements[0], Statement::KillSelf));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn ultra_distance_to_perso_preserves_receiver_and_target() -> Result<()> {
        let (mut codec, bytes) = partial(&[
            (0, 0, 1),
            (1, 5, 2),
            (2, 24, 3),
            (0, 5, 4),
            (3, 4, 4),
            (24, 5678, 5),
            (12, 5, 3),
            (0, 1, 1),
            (8, 0, 0),
        ]);
        let target = AssetRef::<Perso>::new("level:/persos/world/0/174")?;
        codec.actors.insert(5, target.clone());
        codec.actor_handles.push((target.clone(), 5678));

        let script = codec.read(&bytes)?;
        assert_eq!(
            script.statements,
            vec![Statement::If {
                condition: Expression::Compare {
                    operation: Comparison::Less,
                    left: Box::new(Expression::ActorDistanceFrom {
                        receiver: ActorTarget::Keyword(ActorKeyword::MainActor),
                        target: Box::new(Expression::Actor(
                            ActorTarget::Reference(target.clone(),)
                        )),
                    }),
                    right: Box::new(Expression::Integer(5)),
                },
                then: Vec::new(),
                otherwise: None,
            }]
        );
        assert_eq!(codec.write(&script)?, bytes);
        assert_eq!(
            rmp_serde::from_slice::<Script>(&rmp_serde::to_vec_named(&script)?)?,
            script
        );
        Ok(())
    }

    #[test]
    fn distance_to_designer_actor_reads_the_owners_variable() -> Result<()> {
        let (codec, bytes) = partial(&[
            (0, 0, 1),
            (1, 0, 2),
            (1, 5, 3),
            (3, 4, 4),
            (11, 7, 5),
            (12, 5, 4),
            (1, 5, 3),
            (2, 24, 4),
            (0, 5, 5),
            (3, 4, 5),
            (11, 11, 6),
            (12, 5, 4),
            (0, 1, 1),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        let compare = |left| Expression::Compare {
            operation: Comparison::Less,
            left: Box::new(left),
            right: Box::new(Expression::Integer(5)),
        };
        assert_eq!(
            script.statements,
            vec![Statement::If {
                condition: Expression::And(
                    Box::new(compare(Expression::ActorDistance(Box::new(Expression::Actor(
                        ActorTarget::Variable(7)
                    ))))),
                    Box::new(compare(Expression::ActorDistanceFrom {
                        receiver: ActorTarget::Keyword(ActorKeyword::MainActor),
                        target: Box::new(Expression::Actor(ActorTarget::Variable(11))),
                    })),
                ),
                then: Vec::new(),
                otherwise: None,
            }]
        );
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn ultra_see_perso_preserves_receiver_and_lexical_target() -> Result<()> {
        let (codec, bytes) = partial(&[
            (2, 11, 1),
            (11, 0, 2),
            (2, 24, 2),
            (0, 5, 3),
            (1, 76, 3),
            (0, 4, 4),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            script.statements[0],
            Statement::Assign {
                variable: 0,
                value: Expression::SeePerso {
                    receiver: ActorKeyword::MainActor,
                    target: ActorKeyword::SelfActor
                }
            }
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn unrelated_dot_receiver_remains_a_partial_expression() -> Result<()> {
        let (codec, bytes) = partial(&[
            (2, 11, 1),
            (11, 0, 2),
            (2, 12, 2),
            (3, 99, 3),
            (11, 56, 3),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        let Statement::Assign {
            value: Expression::Unsupported(fragment),
            ..
        } = &script.statements[0]
        else {
            panic!("unknown Dot receiver was not preserved")
        };
        assert_eq!(fragment.start, 2);
        assert_eq!(fragment.nodes.len(), 3);
        assert!(
            codec
                .write(&script)
                .unwrap_err()
                .to_string()
                .contains("unsupported AI script cannot be reconstructed")
        );
        Ok(())
    }

    #[test]
    fn elapsed_clock_query_reconstructs_source_nodes() -> Result<()> {
        let (codec, bytes) = partial(&[
            (0, 0, 1),
            (1, 55, 2),
            (11, 98, 3),
            (12, 1600, 3),
            (0, 1, 1),
            (4, 206, 2),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            &script.statements[0],
            Statement::If {
                condition: Expression::TimeElapsed { .. },
                ..
            }
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn particle_generator_switch_reconstructs_source_nodes() -> Result<()> {
        let (codec, bytes) = partial(&[(4, 192, 1), (4, 193, 1), (8, 0, 0)]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            &script.statements[..],
            [
                Statement::SetParticleGeneratorEnabled(true),
                Statement::SetParticleGeneratorEnabled(false)
            ]
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn hit_point_queries_preserve_self_main_and_variable_receivers() -> Result<()> {
        let (codec, bytes) = partial(&[(2, 11, 1), (11, 0, 2), (3, 35, 2), (0, 4, 3), (8, 0, 0)]);
        let script = codec.read(&bytes)?;
        assert!(matches!(
            script.statements.as_slice(),
            [Statement::Assign {
                variable: 0,
                value: Expression::SelfHitPoints
            }]
        ));
        assert_eq!(codec.write(&script)?, bytes);
        for (kind, operand, receiver) in [
            (
                0,
                5,
                Expression::Actor(ActorTarget::Keyword(ActorKeyword::MainActor)),
            ),
            (11, 0, Expression::Variable(0)),
        ] {
            let (codec, bytes) = partial(&[
                (2, 11, 1),
                (11, 0, 2),
                (3, 35, 2),
                (kind, operand, 3),
                (8, 0, 0),
            ]);
            let script = codec.read(&bytes)?;
            assert!(matches!(
                script.statements.as_slice(),
                [Statement::Assign {
                    value: Expression::ActorHitPoints(actual),
                    ..
                }] if **actual == receiver
            ));
            assert_eq!(codec.write(&script)?, bytes);
        }
        {
            let (codec, bytes) =
                partial(&[(2, 11, 1), (11, 0, 2), (3, 35, 2), (0, 7, 3), (8, 0, 0)]);
            let script = codec.read(&bytes)?;
            assert!(matches!(
                script.statements.as_slice(),
                [Statement::Assign {
                    value: Expression::Unsupported(_),
                    ..
                }]
            ));
            assert_eq!(script.unsupported_fragments()[0].nodes.len(), 2);
            assert_eq!(
                script.unsupported_fragments()[0].nodes,
                codec.preserved_nodes[2..4]
            );
            assert!(codec.write(&script).is_err());
        }
        Ok(())
    }

    #[test]
    fn object_action_retains_template_action_and_vector() -> Result<()> {
        let (mut codec, bytes) = partial(&[
            (5, 15, 1),
            (24, 1234, 2),
            (25, 5678, 2),
            (16, 0, 2),
            (13, 0, 3),
            (13, 0, 3),
            (13, 0, 3),
            (8, 0, 0),
        ]);
        let template = AssetRef::<Perso>::new("fix:/persos/5-0-24656")?;
        let action = crate::model::character::StateRef {
            family: AssetRef::new("fix:/families/1")?,
            state: 0,
        };
        codec.actors.insert(1, template.clone());
        codec.actor_handles.push((template.clone(), 1234));
        codec.actions.insert(2, Some(action.clone()));
        codec.action_handles.push((Some(action), 5678));
        let script = codec.read(&bytes)?;
        assert!(matches!(
            &script.statements[0],
            Statement::ExecuteActionAndGenerateObject {
                template: target,
                action: Expression::Action(Some(_)),
                vector: Expression::Vector(_),
            } if *target == template
        ));
        assert_eq!(codec.write(&script)?, bytes);
        Ok(())
    }

    #[test]
    fn adjust_perso_on_object_preserves_threshold_values_and_action_order() -> Result<()> {
        let (mut codec, bytes) = partial(&[
            (5, 17, 1),
            (11, 133, 2),
            (13, 1.3_f32.to_bits(), 2),
            (13, 1.4_f32.to_bits(), 2),
            (25, 0x1234, 2),
            (25, 0x5678, 2),
            (8, 0, 0),
        ]);
        let family = AssetRef::new("fix:/families/7")?;
        let far = crate::model::character::StateRef {
            family: family.clone(),
            state: 21,
        };
        let near = crate::model::character::StateRef { family, state: 22 };
        codec.actions.insert(4, Some(far.clone()));
        codec.actions.insert(5, Some(near.clone()));
        codec.action_handles.push((Some(far.clone()), 0x1234));
        codec.action_handles.push((Some(near.clone()), 0x5678));

        let script = codec.read(&bytes)?;
        assert!(matches!(
            &script.statements[..],
            [Statement::AdjustPersoOnObject {
                target: Expression::Variable(133),
                near_threshold: Expression::Real(near_threshold),
                far_threshold: Expression::Real(far_threshold),
                far_action: Expression::Action(Some(actual_far)),
                near_action: Expression::Action(Some(actual_near)),
            }]
                if *near_threshold == 1.3
                    && *far_threshold == 1.4
                    && actual_far == &far
                    && actual_near == &near
        ));
        let action_ids: Vec<_> = script
            .walk()
            .filter_map(|(node, _)| match node {
                crate::model::ai::walk::AstNode::Expression(Expression::Action(Some(action))) => {
                    Some(action.state)
                }
                _ => None,
            })
            .collect();
        assert_eq!(action_ids, [21, 22]);
        assert_eq!(codec.write(&script)?, bytes);

        // A missing relocated action remains an explicit unsupported operand,
        // and the writer refuses to fabricate its missing source handle.
        let (mut missing_action, bytes) = partial(&[
            (5, 17, 1),
            (11, 133, 2),
            (13, 1.3_f32.to_bits(), 2),
            (13, 1.4_f32.to_bits(), 2),
            (25, 0x1234, 2),
            (25, 0x5678, 2),
            (8, 0, 0),
        ]);
        missing_action.actions.insert(4, Some(far));
        let partial_script = missing_action.read(&bytes)?;
        assert!(matches!(
            &partial_script.statements[..],
            [Statement::AdjustPersoOnObject {
                near_action: Expression::Unsupported(fragment),
                ..
            }] if fragment.reason == "unbound action at node 5"
        ));
        assert!(missing_action.write(&partial_script).is_err());
        Ok(())
    }

    #[test]
    fn partial_expression_keeps_short_circuit_structure() -> Result<()> {
        let (codec, bytes) = partial(&[
            (0, 0, 1),
            (1, 0, 2),
            (12, 0, 3),
            (3, 999, 3),
            (12, 7, 4),
            (0, 1, 1),
            (4, 206, 2),
            (8, 0, 0),
        ]);
        let script = codec.read(&bytes)?;
        let Statement::If {
            condition: Expression::And(left, right),
            ..
        } = &script.statements[0]
        else {
            panic!("lost Boolean structure")
        };
        assert!(matches!(**left, Expression::Integer(0)));
        assert!(
            matches!(&**right, Expression::Unsupported(fragment) if fragment.start == 3 && fragment.nodes.len() == 2)
        );
        Ok(())
    }

    #[test]
    fn partial_decode_still_rejects_malformed_known_trees() {
        for nodes in [
            vec![(0, 0, 1), (12, 0, 2), (4, 206, 1), (8, 0, 0)],
            vec![(4, 999, 1), (12, 0, 3), (8, 0, 0)],
            vec![(2, 11, 1), (11, 0, 2), (8, 0, 0)],
            vec![(4, 214, 1), (12, 0, 2), (12, 1, 2), (8, 0, 0)],
        ] {
            let (codec, bytes) = partial(&nodes);
            assert!(codec.read(&bytes).is_err());
        }
    }
}
