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
                        if !self.codec.actors.contains_key(&actor_at) {
                            out.push(Statement::Unsupported(self.unsupported(
                                at,
                                format!("unbound actor field target at node {actor_at}"),
                            )?));
                            continue;
                        }
                        let (_, dot) = self.take(indent + 1)?;
                        ensure!(dot.kind == 2 && dot.parameter == 12, "actor dot differs");
                        let (actor_at, actor) = self.take(indent + 2)?;
                        ensure!(actor.kind == 24, "actor dot lacks actor operand");
                        let (_, field) = self.take(indent + 2)?;
                        ensure!(field.kind == 11, "actor dot lacks designer field");
                        let variable = field.parameter.try_into()?;
                        let actor = self.codec.actors[&actor_at].clone();
                        let value = self.expression(indent + 1)?;
                        Statement::ActorFieldAssign {
                            actor,
                            variable,
                            value,
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
                (2, 9) => {
                    if !self
                        .nodes
                        .get(self.at)
                        .is_some_and(|target| target.kind == 11)
                    {
                        out.push(Statement::Unsupported(self.unsupported(
                            at,
                            format!("increment target is not a designer variable at node {at}"),
                        )?));
                        continue;
                    }
                    let (_, target) = self.take(indent + 1)?;
                    ensure!(
                        target.kind == 11,
                        "increment requires a designer variable at node {at}"
                    );
                    Statement::Increment {
                        variable: target.parameter.try_into()?,
                    }
                }
                (4, 214) => Statement::Turn {
                    degrees_per_second: self.expression(indent + 1)?,
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
            (11, n) => Expression::Variable(n.try_into()?),
            (2, 2) => Expression::Multiply(
                Box::new(self.expression(indent + 1)?),
                Box::new(self.expression(indent + 1)?),
            ),
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
            (3, 164) => Expression::SelfSighting,
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
                Statement::SubtractAssign { variable, value } => {
                    self.node(2, 6, indent);
                    self.node(11, u32::from(*variable), indent + 1);
                    self.expression(value, indent + 1)?;
                }
                Statement::Increment { variable } => {
                    self.node(2, 9, indent);
                    self.node(11, u32::from(*variable), indent + 1);
                }
                Statement::Turn { degrees_per_second } => {
                    self.node(4, 214, indent);
                    self.expression(degrees_per_second, indent + 1)?;
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
            Expression::Variable(v) => self.node(11, u32::from(*v), indent),
            Expression::Multiply(a, b) => {
                self.node(2, 2, indent);
                self.expression(a, indent + 1)?;
                self.expression(b, indent + 1)?;
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
            Expression::SelfSighting => self.node(3, 164, indent),
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
            Statement::Increment { variable: 132 }
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
