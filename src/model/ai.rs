//! Useful behavior trees and per-instance memory, independent of source opcodes.
use super::{
    AssetPath, AssetRef,
    audio::SoundEventLink,
    character::{Perso, StateRef},
    io::Content,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiModel {
    pub path: AssetPath,
    pub name: String,
    pub normal: Option<BehaviorSet>,
    pub reflex: Option<BehaviorSet>,
    pub variables: Vec<DesignerVariable>,
    pub designer_unknown: [u8; 3],
    pub unknown: [u8; 4],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BehaviorSet {
    pub behaviors: Vec<Behavior>,
    pub default_behavior: Option<u32>,
    pub action_capacity: u8,
    pub unknown: [u8; 3],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Behavior {
    pub id: u32,
    pub rules: Vec<Option<Script>>,
    pub schedule: Option<Script>,
    pub unknown: [u8; 3],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Script {
    pub statements: Vec<Statement>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BehaviorGroup {
    Normal,
    Reflex,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BehaviorRef {
    pub model: AssetRef<AiModel>,
    pub group: BehaviorGroup,
    pub behavior: u32,
}
impl BehaviorRef {
    pub fn validate_model(&self, model: &AiModel) -> Result<()> {
        ensure!(
            self.model.path == model.path,
            "behavior model identity differs"
        );
        let set = match self.group {
            BehaviorGroup::Normal => &model.normal,
            BehaviorGroup::Reflex => &model.reflex,
        };
        ensure!(
            set.as_ref()
                .is_some_and(|s| s.behaviors.iter().any(|b| b.id == self.behavior)),
            "missing behavior member {}",
            self.behavior
        );
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    If {
        condition: Expression,
        then: Vec<Statement>,
        otherwise: Option<Vec<Statement>>,
    },
    Assign {
        variable: u16,
        value: Expression,
    },
    SubtractAssign {
        variable: u16,
        value: Expression,
    },
    ArrayAssign {
        array: Expression,
        index: Expression,
        value: Expression,
    },
    Wait {
        milliseconds: Expression,
    },
    WaitFrames(Expression),
    ChangeBehavior {
        target: BehaviorRef,
        /// Yield the calling flow after the request; a change to that same
        /// behavior group consequently enters its new behavior next tick.
        pause_after: bool,
    },
    ChangeAction(Expression),
    Turn {
        degrees_per_second: Expression,
    },
    ExecuteAction(Expression),
    WaitEndOfAction,
    WaitEndOfAnimation,
    Sound {
        event: Option<SoundEventLink>,
    },
    Noop,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    Integer(i32),
    Real(f32),
    Variable(u16),
    Action(Option<StateRef>),
    IsInAction(Box<Expression>),
    ActionFinished,
    IsInBehavior(BehaviorRef),
    Button {
        action: String,
        query: ButtonQuery,
    },
    ArrayElement {
        array: Box<Expression>,
        index: Box<Expression>,
    },
    MainActorDistance,
    Multiply(Box<Expression>, Box<Expression>),
    Negate(Box<Expression>),
    Compare {
        operation: Comparison,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
    Not(Box<Expression>),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtonQuery {
    JustPressed,
    Pressed,
    Released,
    JustReleased,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Comparison {
    Equal,
    Different,
    Less,
    Greater,
    LessOrEqual,
    GreaterOrEqual,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignerVariable {
    pub default: DesignerValue,
    pub save: u32,
    pub initialize: u32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DesignerValue {
    Boolean(bool),
    Integer(i32),
    Real(f32),
    Byte(i8),
    UnsignedByte(u8),
    Short(i16),
    UnsignedShort(u16),
    UnsignedInteger(u32),
    Vector([f32; 3]),
    IntegerArray(DesignerArray<i32>),
    RealArray(DesignerArray<f32>),
    VectorArray(DesignerArray<[f32; 3]>),
    Action(Option<StateRef>),
    ActionArray(DesignerArray<Option<StateRef>>),
    Actor(Option<AssetRef<Perso>>),
}
/// A fixed-size designer array. Unresolved source fields are preserved explicitly;
/// they have no effect on the array's useful values or Godot's native arrays.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignerArray<T> {
    pub values: Vec<T>,
    pub unknown: [u8; 3],
    /// Only nonzero unused scalar-slot bytes are retained. Vector slots use all
    /// three components and cannot carry these bytes.
    pub element_unknown: std::collections::BTreeMap<u8, [u8; 8]>,
}
impl<T> DesignerArray<T> {
    pub fn new(values: Vec<T>) -> Self {
        Self {
            values,
            unknown: [0; 3],
            element_unknown: Default::default(),
        }
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(self.values.len() <= 255, "too many designer array elements");
        ensure!(
            self.element_unknown
                .iter()
                .all(|(&i, bytes)| usize::from(i) < self.values.len() && *bytes != [0; 8]),
            "invalid designer array unknown fields"
        );
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Brain {
    pub model: AssetRef<AiModel>,
    pub normal: Option<BehaviorSelection>,
    pub reflex: Option<BehaviorSelection>,
    pub variables: Vec<DesignerValue>,
    pub initial_variables: Option<Vec<DesignerValue>>,
    pub unknown: u32,
    pub mind_unknown: [u8; 4],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BehaviorSelection {
    pub current: Option<u32>,
    pub previous: Option<u32>,
    pub initial: Option<u32>,
}
impl Content for AiModel {
    const KIND: &'static str = "ai-model";
    fn identity(&self) -> Option<&AssetPath> {
        Some(&self.path)
    }
    fn validate(&self) -> Result<()> {
        ensure!(self.variables.len() <= 255, "too many designer variables");
        for variable in &self.variables {
            variable.default.validate()?;
        }
        for set in [&self.normal, &self.reflex].into_iter().flatten() {
            ensure!(set.behaviors.len() <= 65535, "too many behaviors");
            let ids: std::collections::BTreeSet<_> = set.behaviors.iter().map(|b| b.id).collect();
            ensure!(
                ids.len() == set.behaviors.len(),
                "duplicate behavior identity"
            );
            ensure!(
                set.default_behavior.is_none_or(|v| ids.contains(&v)),
                "missing default behavior"
            );
            for behavior in &set.behaviors {
                ensure!(behavior.rules.len() <= 255, "too many behavior rules");
                for script in behavior
                    .rules
                    .iter()
                    .flatten()
                    .chain(behavior.schedule.iter())
                {
                    script.validate(self.variables.len())?;
                }
            }
        }
        for reference in self.behavior_references() {
            reference.validate_model(self)?;
        }
        Ok(())
    }
}
impl AiModel {
    pub fn expressions(&self) -> Vec<&Expression> {
        fn expression<'a>(e: &'a Expression, out: &mut Vec<&'a Expression>) {
            out.push(e);
            match e {
                Expression::Compare { left, right, .. }
                | Expression::And(left, right)
                | Expression::Or(left, right)
                | Expression::Multiply(left, right) => {
                    expression(left, out);
                    expression(right, out);
                }
                Expression::Not(v) | Expression::Negate(v) | Expression::IsInAction(v) => {
                    expression(v, out)
                }
                Expression::ArrayElement { array, index } => {
                    expression(array, out);
                    expression(index, out);
                }
                _ => {}
            }
        }
        fn statements<'a>(values: &'a [Statement], out: &mut Vec<&'a Expression>) {
            for s in values {
                match s {
                    Statement::If {
                        condition,
                        then,
                        otherwise,
                    } => {
                        expression(condition, out);
                        statements(then, out);
                        if let Some(v) = otherwise {
                            statements(v, out);
                        }
                    }
                    Statement::Assign { value, .. }
                    | Statement::SubtractAssign { value, .. }
                    | Statement::Turn {
                        degrees_per_second: value,
                    } => expression(value, out),
                    Statement::ArrayAssign {
                        array,
                        index,
                        value,
                    } => {
                        expression(array, out);
                        expression(index, out);
                        expression(value, out);
                    }
                    Statement::Wait { milliseconds } => expression(milliseconds, out),
                    Statement::WaitFrames(frames) => expression(frames, out),
                    Statement::ChangeAction(a) | Statement::ExecuteAction(a) => expression(a, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        for set in [&self.normal, &self.reflex].into_iter().flatten() {
            for behavior in &set.behaviors {
                for script in behavior
                    .rules
                    .iter()
                    .flatten()
                    .chain(behavior.schedule.iter())
                {
                    statements(&script.statements, &mut out);
                }
            }
        }
        out
    }
    pub fn state_references(&self) -> Vec<&StateRef> {
        let mut out: Vec<_> = self
            .variables
            .iter()
            .flat_map(|v| v.default.state_references())
            .collect();
        out.extend(self.expressions().into_iter().filter_map(|e| {
            if let Expression::Action(Some(r)) = e {
                Some(r)
            } else {
                None
            }
        }));
        out
    }
    pub fn behavior_references(&self) -> Vec<&BehaviorRef> {
        fn visit<'a>(statements: &'a [Statement], out: &mut Vec<&'a BehaviorRef>) {
            for statement in statements {
                match statement {
                    Statement::ChangeBehavior { target, .. } => out.push(target),
                    Statement::If {
                        then, otherwise, ..
                    } => {
                        visit(then, out);
                        if let Some(otherwise) = otherwise {
                            visit(otherwise, out);
                        }
                    }
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        for set in [&self.normal, &self.reflex].into_iter().flatten() {
            for behavior in &set.behaviors {
                for script in behavior
                    .rules
                    .iter()
                    .flatten()
                    .chain(behavior.schedule.iter())
                {
                    visit(&script.statements, &mut out);
                }
            }
        }
        out.extend(self.expressions().into_iter().filter_map(|e| match e {
            Expression::IsInBehavior(r) => Some(r),
            _ => None,
        }));
        out
    }
    pub fn sound_events(&self) -> Vec<&SoundEventLink> {
        fn visit<'a>(statements: &'a [Statement], out: &mut Vec<&'a SoundEventLink>) {
            for statement in statements {
                match statement {
                    Statement::Sound { event: Some(event) } => out.push(event),
                    Statement::If {
                        then, otherwise, ..
                    } => {
                        visit(then, out);
                        if let Some(otherwise) = otherwise {
                            visit(otherwise, out);
                        }
                    }
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        for set in [&self.normal, &self.reflex].into_iter().flatten() {
            for behavior in &set.behaviors {
                for script in behavior
                    .rules
                    .iter()
                    .flatten()
                    .chain(behavior.schedule.iter())
                {
                    visit(&script.statements, &mut out);
                }
            }
        }
        out
    }
}
impl DesignerValue {
    pub fn state_references(&self) -> Vec<&StateRef> {
        match self {
            Self::Action(r) => r.iter().collect(),
            Self::ActionArray(v) => v.values.iter().flatten().collect(),
            _ => Vec::new(),
        }
    }
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Real(v) => ensure!(v.is_finite(), "nonfinite designer value"),
            Self::Vector(v) => {
                ensure!(v.iter().all(|v| v.is_finite()), "nonfinite designer vector")
            }
            Self::IntegerArray(v) => v.validate()?,
            Self::ActionArray(v) => v.validate()?,
            Self::RealArray(v) => {
                v.validate()?;
                ensure!(
                    v.values.iter().all(|v| v.is_finite()),
                    "nonfinite designer array value"
                );
            }
            Self::VectorArray(v) => {
                v.validate()?;
                ensure!(
                    v.element_unknown.is_empty(),
                    "vector array has no unused element fields"
                );
                ensure!(
                    v.values.iter().flatten().all(|v| v.is_finite()),
                    "nonfinite designer array vector"
                );
            }
            _ => {}
        }
        Ok(())
    }
    pub fn same_type(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::IntegerArray(a), Self::IntegerArray(b)) => a.values.len() == b.values.len(),
            (Self::RealArray(a), Self::RealArray(b)) => a.values.len() == b.values.len(),
            (Self::VectorArray(a), Self::VectorArray(b)) => a.values.len() == b.values.len(),
            (Self::ActionArray(a), Self::ActionArray(b)) => a.values.len() == b.values.len(),
            _ => std::mem::discriminant(self) == std::mem::discriminant(other),
        }
    }
}
impl Script {
    pub fn validate(&self, variables: usize) -> Result<()> {
        fn expression(
            e: &Expression,
            variables: usize,
            depth: usize,
            count: &mut usize,
        ) -> Result<()> {
            *count += 1;
            ensure!(depth <= 64 && *count <= 65535, "excessive AI tree");
            match e {
                Expression::Real(v) => ensure!(v.is_finite(), "nonfinite AI real"),
                Expression::Variable(id) => {
                    ensure!(usize::from(*id) < variables, "AI variable out of range")
                }
                Expression::Compare { left, right, .. }
                | Expression::And(left, right)
                | Expression::Or(left, right)
                | Expression::Multiply(left, right) => {
                    expression(left, variables, depth + 1, count)?;
                    expression(right, variables, depth + 1, count)?;
                }
                Expression::Not(value)
                | Expression::Negate(value)
                | Expression::IsInAction(value) => expression(value, variables, depth + 1, count)?,
                Expression::ArrayElement { array, index } => {
                    expression(array, variables, depth + 1, count)?;
                    expression(index, variables, depth + 1, count)?;
                }
                Expression::Button { action, .. } => ensure!(
                    !action.is_empty()
                        && action.len() <= 1024
                        && !action.chars().any(char::is_control),
                    "invalid logical button name"
                ),
                _ => {}
            }
            Ok(())
        }
        fn statements(
            values: &[Statement],
            variables: usize,
            depth: usize,
            count: &mut usize,
        ) -> Result<()> {
            ensure!(depth <= 64, "excessive AI tree");
            for s in values {
                *count += 1;
                ensure!(*count <= 65535, "excessive AI tree");
                match s {
                    Statement::If {
                        condition,
                        then,
                        otherwise,
                    } => {
                        expression(condition, variables, depth + 1, count)?;
                        statements(then, variables, depth + 1, count)?;
                        if let Some(v) = otherwise {
                            statements(v, variables, depth + 1, count)?;
                        }
                    }
                    Statement::Assign { variable, value }
                    | Statement::SubtractAssign { variable, value } => {
                        ensure!(
                            usize::from(*variable) < variables,
                            "AI assignment out of range"
                        );
                        expression(value, variables, depth + 1, count)?;
                    }
                    Statement::Wait { milliseconds } => {
                        expression(milliseconds, variables, depth + 1, count)?
                    }
                    Statement::WaitFrames(frames) => {
                        expression(frames, variables, depth + 1, count)?
                    }
                    Statement::Turn { degrees_per_second } => {
                        expression(degrees_per_second, variables, depth + 1, count)?
                    }
                    Statement::ArrayAssign {
                        array,
                        index,
                        value,
                    } => {
                        expression(array, variables, depth + 1, count)?;
                        expression(index, variables, depth + 1, count)?;
                        expression(value, variables, depth + 1, count)?;
                    }
                    Statement::ChangeAction(action) | Statement::ExecuteAction(action) => {
                        expression(action, variables, depth + 1, count)?
                    }
                    _ => {}
                }
            }
            Ok(())
        }
        statements(&self.statements, variables, 0, &mut 0)
    }
}
impl Brain {
    pub fn validate_model(&self, model: &AiModel) -> Result<()> {
        ensure!(
            self.model.path == model.path,
            "brain/model identity differs"
        );
        for values in std::iter::once(&self.variables).chain(self.initial_variables.iter()) {
            ensure!(
                values.len() == model.variables.len(),
                "designer memory length differs"
            );
            for (v, d) in values.iter().zip(&model.variables) {
                v.validate()?;
                ensure!(v.same_type(&d.default), "designer memory type differs");
            }
        }
        for (selection, set) in [(&self.normal, &model.normal), (&self.reflex, &model.reflex)] {
            if let Some(selection) = selection {
                let set = set
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("brain selects absent behavior set"))?;
                for id in [selection.current, selection.previous, selection.initial]
                    .into_iter()
                    .flatten()
                {
                    ensure!(
                        set.behaviors.iter().any(|b| b.id == id),
                        "brain selects missing behavior"
                    );
                }
            }
        }
        Ok(())
    }
}
