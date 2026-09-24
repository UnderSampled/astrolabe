//! Hype's eight-byte AI nodes map directly into useful typed trees.
//! 005217d0 initializes reserved bytes; 00521780 counts through kind-8 sentinel.
use super::codec::FormatCodec;
use crate::model::{AssetRef, ai::*, audio::SoundEventLink, character::Perso, io::FormatSupport};
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
    pub buttons: BTreeMap<usize, String>,
    pub button_handles: Vec<(String, u32)>,
    pub behaviors: BTreeMap<usize, BehaviorRef>,
    pub behavior_handles: Vec<(BehaviorRef, u32)>,
    pub actors: BTreeMap<usize, AssetRef<Perso>>,
    pub actor_handles: Vec<(AssetRef<Perso>, u32)>,
}
impl FormatCodec for AiScriptCodec {
    type Value = Script;
    fn support(&self) -> FormatSupport {
        FormatSupport {
            format: "Hype AI script",
            reads: true,
            writes: true,
            notes: "Typed scalar/Boolean trees, branches, assignment, wait, main-actor distance and sound requests; unsupported operations fail explicitly",
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
        while self.nodes.get(self.at).is_some_and(|n| n.indent >= indent) {
            let (at, node) = self.take(indent)?;
            let (kind, id) = (node.kind, node.parameter);
            if node.reserved != [0; 2] {
                out.push(Statement::Unsupported(
                    self.unsupported(at, "nonzero AI node reserved bytes".into())?,
                ));
                continue;
            }
            out.push(match (kind, id) {
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
                (4, 214) => Statement::Turn {
                    degrees_per_second: self.expression(indent + 1)?,
                },
                (4, 29) => Statement::SetVectorSpeed(self.expression(indent + 1)?),
                (4, 34) => Statement::GoRelative,
                (4, 226) => {
                    let (_, actor) = self.take(indent + 1)?;
                    if actor.kind == 0 && actor.parameter == 4 {
                        Statement::KillSelf
                    } else {
                        Statement::Unsupported(self.unsupported(
                            at,
                            "KillPerso requires an unsupported actor target".into(),
                        )?)
                    }
                }
                (4, 23) => Statement::SetImpulse(self.expression(indent + 1)?),
                (4, 47) => Statement::JumpAbsolute,
                (4, 48) => Statement::JumpWithoutAddingSpeed,
                (4, 192) => Statement::SetParticleGeneratorEnabled(true),
                (4, 193) => Statement::SetParticleGeneratorEnabled(false),
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
                (4, 422..=426) => Statement::SetHudVisible {
                    indicator: match id {
                        422 => HudIndicator::Life,
                        423 => HudIndicator::Armor,
                        424 => HudIndicator::Magic,
                        425 => HudIndicator::SelectedObject,
                        _ => HudIndicator::Weapon,
                    },
                    visible: self.expression(indent + 1)?,
                },
                (5, 68) => Statement::WaitFrames(self.expression(indent + 1)?),
                (4, 209 | 210) => self.change_behavior(id, indent, false, at)?,
                (4, 6) => {
                    // Hype's locomotion rules pair this one-operation yield with
                    // a behavior request. Normalize the pair into useful flow;
                    // never expose source return flags as a Godot API/property.
                    let (_, value) = self.take(indent + 1)?;
                    ensure!(
                        value.kind == 12,
                        "action-return policy lacks integer at node {at}"
                    );
                    if value.parameter != 1
                        || !self.nodes.get(self.at).is_some_and(|request| {
                            request.indent == indent
                                && request.kind == 4
                                && matches!(request.parameter, 209 | 210)
                        })
                    {
                        out.push(Statement::Unsupported(self.unsupported(
                            at,
                            format!("unsupported action-return policy at node {at}"),
                        )?));
                        continue;
                    }
                    let (_, request) = self.take(indent)?;
                    let id = request.parameter;
                    self.change_behavior(id, indent, true, at)?
                }
                (4, 218) => Statement::ChangeAction(self.expression(indent + 1)?),
                (5, 4) => Statement::ExecuteAction(self.expression(indent + 1)?),
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
                (4, 206) => Statement::Noop,
                _ => Statement::Unsupported(self.unsupported(
                    at,
                    format!("unsupported AI statement category {kind} operation {id} at node {at}"),
                )?),
            });
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
        let group = if id == 209 {
            BehaviorGroup::Normal
        } else {
            BehaviorGroup::Reflex
        };
        ensure!(
            target.group == group,
            "behavior request/owning group differs"
        );
        Ok(Statement::ChangeBehavior {
            target,
            pause_after,
        })
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
            (0, 7) => Expression::Nobody,
            (0, 8) => Expression::Nowhere,
            (12, n) => Expression::Integer(n as i32),
            (13, n) => Expression::Real(f32::from_bits(n)),
            (16, 0) => Expression::Vector(Box::new([
                self.expression(indent + 1)?,
                self.expression(indent + 1)?,
                self.expression(indent + 1)?,
            ])),
            (11, n) => Expression::Variable(n.try_into()?),
            (2, 0) => Expression::Plus(
                Box::new(self.expression(indent + 1)?),
                Box::new(self.expression(indent + 1)?),
            ),
            (2, 1) => Expression::Minus(
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
            (2, 12) => {
                let actor_at = self.at;
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
            (1, 123) => Expression::ActionFinished,
            (1, 31) => Expression::CollideWithGround,
            (1, 34) => Expression::CollideWithSlope,
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
            (2, 24) => {
                let (_, actor) = self.take(indent + 1)?;
                let receiver = match (actor.kind, actor.parameter) {
                    (0, 5) => ActorKeyword::MainActor,
                    _ => {
                        return Ok(Expression::Unsupported(self.unsupported(
                            at,
                            "Ultra requires an unsupported receiver actor".into(),
                        )?));
                    }
                };
                match self.expression(indent + 1)? {
                    Expression::SeePerso { target, .. } => {
                        Expression::SeePerso { receiver, target }
                    }
                    _ => Expression::Unsupported(self.unsupported(
                        at,
                        "Ultra operation is unsupported for this receiver".into(),
                    )?),
                }
            }
            (1, 56 | 57) => {
                let (_, actor) = self.take(indent + 1)?;
                if actor.kind != 0 || actor.parameter != 4 {
                    return Ok(Expression::Unsupported(self.unsupported(
                        at,
                        "behavior query requires an unsupported actor expression".into(),
                    )?));
                }
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
                Expression::IsInBehavior(target)
            }
            (1, 9..=12) => {
                let (operand, child) = self.take(indent + 1)?;
                ensure!(child.kind == 14, "button query lacks logical input operand");
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
                    query: match id {
                        9 => ButtonQuery::Pressed,
                        10 => ButtonQuery::JustPressed,
                        11 => ButtonQuery::Released,
                        _ => ButtonQuery::JustReleased,
                    },
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
            (3, 4) => {
                let (_, actor) = self.take(indent + 1)?;
                if actor.kind != 0 || actor.parameter != 5 {
                    return Ok(Expression::Unsupported(self.unsupported(
                        at,
                        "distance requires an unsupported actor expression".into(),
                    )?));
                }
                Expression::MainActorDistance
            }
            (3, 201) => Expression::InventoryWeapon,
            (3, 208) => Expression::InventoryQuantity(Box::new(self.expression(indent + 1)?)),
            (3, 209) => Expression::InventoryRemove {
                item: Box::new(self.expression(indent + 1)?),
                count: Box::new(self.expression(indent + 1)?),
            },
            (3, 164) => Expression::SelfSighting,
            (3, 0) => Expression::SelfPosition,
            (3, 41) => Expression::GameTime,
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
        for s in values {
            match s {
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
                Statement::Turn { degrees_per_second } => {
                    self.node(4, 214, indent);
                    self.expression(degrees_per_second, indent + 1)?;
                }
                Statement::SetVectorSpeed(value) => {
                    self.node(4, 29, indent);
                    self.expression(value, indent + 1)?;
                }
                Statement::GoRelative => self.node(4, 34, indent),
                Statement::KillSelf => {
                    self.node(4, 226, indent);
                    self.node(0, 4, indent + 1);
                }
                Statement::SetImpulse(value) => {
                    self.node(4, 23, indent);
                    self.expression(value, indent + 1)?;
                }
                Statement::JumpAbsolute => self.node(4, 47, indent),
                Statement::JumpWithoutAddingSpeed => self.node(4, 48, indent),
                Statement::SetParticleGeneratorEnabled(enabled) => {
                    self.node(4, if *enabled { 192 } else { 193 }, indent)
                }
                Statement::SetChannelActive { channel, active } => {
                    self.node(4, if *active { 104 } else { 105 }, indent);
                    self.node(18, *channel, indent + 1);
                }
                Statement::SetHudVisible { indicator, visible } => {
                    self.node(
                        4,
                        match indicator {
                            HudIndicator::Life => 422,
                            HudIndicator::Armor => 423,
                            HudIndicator::Magic => 424,
                            HudIndicator::SelectedObject => 425,
                            HudIndicator::Weapon => 426,
                        },
                        indent,
                    );
                    self.expression(visible, indent + 1)?;
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
                Statement::WaitEndOfAction => self.node(5, 6, indent),
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
                Statement::SoundFadeMusic { arg0, arg1 } => {
                    self.node(4, 408, indent);
                    self.expression(arg0, indent + 1)?;
                    self.expression(arg1, indent + 1)?;
                }
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
            Expression::Unsupported(_) => bail!("unsupported AI fragment cannot be reconstructed"),
            Expression::Nobody => self.node(0, 7, indent),
            Expression::Nowhere => self.node(0, 8, indent),
            Expression::Integer(v) => self.node(12, *v as u32, indent),
            Expression::Real(v) => self.node(13, v.to_bits(), indent),
            Expression::Vector(values) => {
                self.node(16, 0, indent);
                for value in values.iter() {
                    self.expression(value, indent + 1)?;
                }
            }
            Expression::Variable(v) => self.node(11, u32::from(*v), indent),
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
            Expression::ActionFinished => self.node(1, 123, indent),
            Expression::CollideWithGround => self.node(1, 31, indent),
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
            Expression::IsInBehavior(target) => {
                self.node(
                    1,
                    if target.group == BehaviorGroup::Normal {
                        56
                    } else {
                        57
                    },
                    indent,
                );
                self.node(0, 4, indent + 1);
                self.behavior(target, indent + 1)?;
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
            Expression::InventoryQuantity(item) => {
                self.node(3, 208, indent);
                self.expression(item, indent + 1)?;
            }
            Expression::InventoryRemove { item, count } => {
                self.node(3, 209, indent);
                self.expression(item, indent + 1)?;
                self.expression(count, indent + 1)?;
            }
            Expression::SelfSighting => self.node(3, 164, indent),
            Expression::SelfPosition => self.node(3, 0, indent),
            Expression::GameTime => self.node(3, 41, indent),
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
    fn channel_and_hud_requests_decode_and_reconstruct() -> Result<()> {
        let (codec, bytes) = partial(&[
            (4, 105, 1),
            (18, 72, 2),
            (4, 104, 1),
            (18, 28, 2),
            (4, 426, 1),
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
