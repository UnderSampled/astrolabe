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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnsupportedFragment {
    pub reason: String,
    /// Ordinal within the source script, never a virtual address.
    pub start: u32,
    pub nodes: Vec<PreservedAiNode>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreservedAiNode {
    pub category: u8,
    pub indent: u8,
    pub reserved: [u8; 2],
    pub operand: PreservedAiOperand,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreservedAiOperand {
    /// The source relocation map has no entry and this word is outside all
    /// source virtual-address ranges. Its meaning remains unclassified.
    NonRelocated([u8; 4]),
    Input(String),
    Action(Option<StateRef>),
    Behavior(BehaviorRef),
    Sound(Option<SoundEventLink>),
    Source(Option<SourceObjectIdentity>),
    /// An unclassified operand that cannot safely be persisted as raw bits.
    /// The source writer must reject this script until its meaning is known.
    Incomplete(String),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceObjectIdentity {
    pub role: SourceRole,
    pub block: [u8; 2],
    pub allocation: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceRole {
    Fix,
    Level,
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
    Unsupported(UnsupportedFragment),
    If {
        condition: Expression,
        then: Vec<Statement>,
        otherwise: Option<Vec<Statement>>,
    },
    Assign {
        variable: u16,
        value: Expression,
    },
    /// Assignment through a typed actor reference, such as Hype's shared
    /// Fix-owned Global actor. The destination is ordinary actor Brain state.
    ActorFieldAssign {
        actor: AssetRef<Perso>,
        variable: u16,
        value: Expression,
    },
    /// Source Keyword_MainActor is resolved from the active scene at runtime.
    MainActorFieldAssign {
        variable: u16,
        value: Expression,
    },
    SubtractAssign {
        variable: u16,
        value: Expression,
    },
    AddAssign {
        variable: u16,
        value: Expression,
    },
    Increment {
        target: FieldTarget,
    },
    Decrement {
        target: FieldTarget,
    },
    ArrayAssign {
        array: Expression,
        index: Expression,
        value: Expression,
    },
    VectorComponentAssign {
        vector: u16,
        axis: VectorAxis,
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
    /// Source motion requests retain their order in the AI schedule. The
    /// character component owns their runtime effects.
    SetVectorSpeed(Expression),
    GoRelative,
    /// Proc_KillPerso(Me): remove the current generated Perso.
    KillSelf,
    SetImpulse(Expression),
    JumpAbsolute,
    JumpWithoutAddingSpeed,
    SetParticleGeneratorEnabled(bool),
    /// Original laser-pointer presentation is global; aiming reads its last
    /// target independently of whether the pointer is visible.
    SetLaserPointerActive(bool),
    ChangeSighting(Expression),
    /// Camera procedure 160 receives an AI-referenced camera Perso and the
    /// source MainActor keyword. The native API stores the target actor.
    CameraChangeTargetedPerso {
        camera: AssetRef<Perso>,
        target: ActorKeyword,
    },
    /// Source camera procedure 135 requests a position update. The preceding
    /// return-zero form is retained for source reconstruction only; Godot
    /// receives one native camera API call.
    CameraUpdatePosition {
        preceded_by_return_zero: bool,
    },
    /// Source Proc_AddPersoInList appends the referenced Perso to a bounded
    /// designer list. The script decides when to call it.
    ActorListAdd {
        list: u16,
        actor: AssetRef<Perso>,
    },
    /// Hype Proc_ActivateChannel / Proc_DeactivateChannel select an existing
    /// character channel by its source channel number.
    SetChannelActive {
        channel: u32,
        active: bool,
    },
    /// Source HUD visibility request; Godot owns the presentation component.
    SetHudVisible {
        indicator: HudIndicator,
        visible: Expression,
    },
    ExecuteAction(Expression),
    /// Play an action while requesting an independently stored Perso template.
    /// Runtime generation/timing is owned by the target character component.
    ExecuteActionAndGenerateObject {
        template: AssetRef<Perso>,
        action: Expression,
        vector: Expression,
    },
    WaitEndOfAction,
    WaitEndOfAnimation,
    Sound {
        event: Option<SoundEventLink>,
    },
    /// Hype Proc_SND_FadeMusic: arg0 is target level 0..127 and arg1 is
    /// duration in milliseconds, verified at the native mixer helper.
    SoundFadeMusic {
        arg0: Expression,
        arg1: Expression,
    },
    Noop,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HudIndicator {
    Life,
    Armor,
    Magic,
    SelectedObject,
    Weapon,
}
/// Writable designer field selected by a source AI arithmetic operator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldTarget {
    Variable(u16),
    ActorField {
        actor: AssetRef<Perso>,
        variable: u16,
    },
    MainActorField(u16),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    Unsupported(UnsupportedFragment),
    /// Source Keyword_Nobody: an absent actor reference.
    Nobody,
    /// Source Keyword_Nowhere: an absent waypoint reference.
    Nowhere,
    Integer(i32),
    Real(f32),
    /// Source vector literal (category 16); components remain source-space.
    Vector(Box<[Expression; 3]>),
    /// Current actor's world position in source coordinates.
    SelfPosition,
    /// Source gameplay clock value; its target implementation is owned by Godot.
    GameTime,
    /// Cond_IsTimeElapsed checks the shared gameplay clock against a saved
    /// timestamp and duration (the reached Hype case uses 1600 ms).
    TimeElapsed {
        start: Box<Expression>,
        duration: Box<Expression>,
    },
    /// Source Cond_SeePerso, optionally evaluated on another receiver through
    /// Operator_Ultra. The target keyword remains relative to the script owner.
    SeePerso {
        receiver: ActorKeyword,
        target: ActorKeyword,
    },
    Variable(u16),
    /// Source Cond_IsPersoInList scans the active prefix of a designer list.
    ActorListContains {
        list: u16,
        actor: Box<Expression>,
    },
    Action(Option<StateRef>),
    IsInAction(Box<Expression>),
    ActionFinished,
    /// Current actor's native floor contact.
    CollideWithGround,
    /// Source Cond_CollideWithNothing reads the actor contact result. The
    /// native contact-category mapping remains under investigation.
    CollideWithNothing,
    /// Source Cond_CollidePersoZDENoWithTypeZDE queries intersections for a
    /// selected actor ZDE zone, bounded by its source hit-count operand.
    CollidePersoZdeNoWithTypeZde {
        zone: u32,
        maximum: u32,
    },
    /// Source collision-category bit 0x4; Godot currently maps it to native
    /// inclined floor contact while the full source classification is studied.
    CollideWithSlope,
    IsTypeOfGmtCollide(u32),
    InTopOfJump,
    /// Read a designer field from a bound actor Brain (Operator_Dot).
    ActorFieldRead {
        actor: AssetRef<Perso>,
        variable: u16,
    },
    SelfFieldRead(u16),
    MainActorFieldRead(u16),
    IsInBehavior {
        actor: ActorKeyword,
        target: BehaviorRef,
    },
    Button {
        action: String,
        query: ButtonQuery,
    },
    ArrayElement {
        array: Box<Expression>,
        index: Box<Expression>,
    },
    VectorComponent {
        vector: Box<Expression>,
        axis: VectorAxis,
    },
    MainActorDistance,
    /// Source inventory functions operate on the game inventory, not on a
    /// projectile's own designer fields.
    InventoryWeapon,
    InventoryQuantity(Box<Expression>),
    InventoryRemove {
        item: Box<Expression>,
        count: Box<Expression>,
    },
    /// Source magick functions read the shared selected spell and mana pool.
    MagicCurrentSpell,
    MagicPoints,
    MagicRemovePoints(Box<Expression>),
    LaserDirection,
    VectorCos(Box<Expression>, Box<Expression>),
    /// Func_GetPersoSighting: current actor's source-coordinate sighting
    /// direction, confirmed from Hype's own perso context.
    SelfSighting,
    RandomInt {
        lower: Box<Expression>,
        upper: Box<Expression>,
    },
    Multiply(Box<Expression>, Box<Expression>),
    Plus(Box<Expression>, Box<Expression>),
    Minus(Box<Expression>, Box<Expression>),
    Divide(Box<Expression>, Box<Expression>),
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
pub enum ActorKeyword {
    SelfActor,
    MainActor,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtonQuery {
    JustPressed,
    Pressed,
    Released,
    JustReleased,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VectorAxis {
    X,
    Y,
    Z,
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
    /// A bounded source list whose verified camera use stores Perso identities.
    /// Inactive slots remain present because source removal can leave stale data.
    ActorList(DesignerActorList),
    Action(Option<StateRef>),
    ActionArray(DesignerArray<Option<StateRef>>),
    Actor(Option<AssetRef<Perso>>),
    /// Actor reference outside the imported scene's useful perso graph.
    SourceActor(Option<SourceObjectIdentity>),
    /// Source waypoint identity until the route/waypoint model is promoted.
    SourceWaypoint(Option<SourceObjectIdentity>),
    SourceWaypointArray(DesignerArray<Option<SourceObjectIdentity>>),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignerActorList {
    pub active_len: u8,
    pub header_unknown: [u8; 2],
    pub slots: Vec<Option<DesignerActorListEntry>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DesignerActorListEntry {
    Actor(AssetRef<Perso>),
    /// Preserved source identity when an entry is outside the imported actor graph.
    Source(SourceObjectIdentity),
}
impl DesignerActorList {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.slots.len() <= 255,
            "designer list capacity exceeds byte range"
        );
        ensure!(
            usize::from(self.active_len) <= self.slots.len(),
            "designer list active length exceeds capacity"
        );
        Ok(())
    }
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
        for (reference, requires_own_model) in self.behavior_references_with_owner() {
            // A query may name a behavior in another actor's AI model. Its
            // owning model is resolved by the content adapter, not by `self`.
            if requires_own_model || reference.model.path == self.path {
                reference.validate_model(self)?;
            }
        }
        Ok(())
    }
}
impl AiModel {
    pub fn object_templates(&self) -> Vec<AssetRef<Perso>> {
        fn visit(statements: &[Statement], out: &mut Vec<AssetRef<Perso>>) {
            for statement in statements {
                match statement {
                    Statement::ExecuteActionAndGenerateObject { template, .. } => {
                        if !out.contains(template) {
                            out.push(template.clone());
                        }
                    }
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
    pub fn actor_field_writes(&self) -> Vec<(AssetRef<Perso>, u16)> {
        fn visit(statements: &[Statement], out: &mut Vec<(AssetRef<Perso>, u16)>) {
            for statement in statements {
                match statement {
                    Statement::ActorFieldAssign {
                        actor, variable, ..
                    } => {
                        let entry = (actor.clone(), *variable);
                        if !out.contains(&entry) {
                            out.push(entry);
                        }
                    }
                    Statement::Increment {
                        target: FieldTarget::ActorField { actor, variable },
                    }
                    | Statement::Decrement {
                        target: FieldTarget::ActorField { actor, variable },
                    } => {
                        let entry = (actor.clone(), *variable);
                        if !out.contains(&entry) {
                            out.push(entry);
                        }
                    }
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
    pub fn actor_field_targets(&self) -> Vec<AssetRef<Perso>> {
        let mut out = Vec::new();
        for (actor, _) in self.actor_field_accesses() {
            if !out.contains(&actor) {
                out.push(actor);
            }
        }
        out
    }
    pub fn actor_field_accesses(&self) -> Vec<(AssetRef<Perso>, u16)> {
        let mut out = self.actor_field_writes();
        for entry in self.actor_field_reads() {
            if !out.contains(&entry) {
                out.push(entry);
            }
        }
        out
    }
    pub fn actor_field_reads(&self) -> Vec<(AssetRef<Perso>, u16)> {
        let mut out = Vec::new();
        for expression in self.expressions() {
            if let Expression::ActorFieldRead { actor, variable } = expression {
                let entry = (actor.clone(), *variable);
                if !out.contains(&entry) {
                    out.push(entry);
                }
            }
        }
        out
    }
    pub fn expressions(&self) -> Vec<&Expression> {
        fn expression<'a>(e: &'a Expression, out: &mut Vec<&'a Expression>) {
            out.push(e);
            match e {
                Expression::Compare { left, right, .. }
                | Expression::And(left, right)
                | Expression::Or(left, right)
                | Expression::Multiply(left, right)
                | Expression::Plus(left, right)
                | Expression::Minus(left, right)
                | Expression::Divide(left, right)
                | Expression::TimeElapsed {
                    start: left,
                    duration: right,
                }
                | Expression::RandomInt {
                    lower: left,
                    upper: right,
                }
                | Expression::InventoryRemove {
                    item: left,
                    count: right,
                }
                | Expression::VectorCos(left, right) => {
                    expression(left, out);
                    expression(right, out);
                }
                Expression::Not(v)
                | Expression::Negate(v)
                | Expression::IsInAction(v)
                | Expression::InventoryQuantity(v)
                | Expression::MagicRemovePoints(v)
                | Expression::ActorListContains { actor: v, .. } => expression(v, out),
                Expression::Vector(values) => {
                    for value in values.iter() {
                        expression(value, out);
                    }
                }
                Expression::VectorComponent { vector, .. } => expression(vector, out),
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
                    | Statement::ActorFieldAssign { value, .. }
                    | Statement::MainActorFieldAssign { value, .. }
                    | Statement::VectorComponentAssign { value, .. }
                    | Statement::SubtractAssign { value, .. }
                    | Statement::AddAssign { value, .. }
                    | Statement::Turn {
                        degrees_per_second: value,
                    }
                    | Statement::ChangeSighting(value)
                    | Statement::SetVectorSpeed(value)
                    | Statement::SetImpulse(value) => expression(value, out),
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
                    Statement::ExecuteActionAndGenerateObject { action, vector, .. } => {
                        expression(action, out);
                        expression(vector, out);
                    }
                    Statement::SoundFadeMusic { arg0, arg1 } => {
                        expression(arg0, out);
                        expression(arg1, out);
                    }
                    Statement::SetHudVisible { visible, .. } => expression(visible, out),
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
        self.behavior_references_with_owner()
            .into_iter()
            .map(|(reference, _)| reference)
            .collect()
    }
    fn behavior_references_with_owner(&self) -> Vec<(&BehaviorRef, bool)> {
        fn visit<'a>(statements: &'a [Statement], out: &mut Vec<(&'a BehaviorRef, bool)>) {
            for statement in statements {
                match statement {
                    Statement::ChangeBehavior { target, .. } => out.push((target, true)),
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
            Expression::IsInBehavior { target, .. } => Some((target, false)),
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
            Self::SourceWaypointArray(v) => v.validate()?,
            Self::ActorList(v) => v.validate()?,
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
            (Self::Actor(_), Self::SourceActor(_)) | (Self::SourceActor(_), Self::Actor(_)) => true,
            (Self::IntegerArray(a), Self::IntegerArray(b)) => a.values.len() == b.values.len(),
            (Self::RealArray(a), Self::RealArray(b)) => a.values.len() == b.values.len(),
            (Self::VectorArray(a), Self::VectorArray(b)) => a.values.len() == b.values.len(),
            (Self::ActionArray(a), Self::ActionArray(b)) => a.values.len() == b.values.len(),
            (Self::SourceWaypointArray(a), Self::SourceWaypointArray(b)) => {
                a.values.len() == b.values.len()
            }
            (Self::ActorList(a), Self::ActorList(b)) => a.slots.len() == b.slots.len(),
            _ => std::mem::discriminant(self) == std::mem::discriminant(other),
        }
    }
}
impl Script {
    pub fn unsupported_fragments(&self) -> Vec<&UnsupportedFragment> {
        fn expression<'a>(e: &'a Expression, out: &mut Vec<&'a UnsupportedFragment>) {
            match e {
                Expression::Unsupported(f) => out.push(f),
                Expression::Compare { left, right, .. }
                | Expression::And(left, right)
                | Expression::Or(left, right)
                | Expression::Multiply(left, right)
                | Expression::Plus(left, right)
                | Expression::Minus(left, right)
                | Expression::Divide(left, right)
                | Expression::TimeElapsed {
                    start: left,
                    duration: right,
                }
                | Expression::RandomInt {
                    lower: left,
                    upper: right,
                }
                | Expression::InventoryRemove {
                    item: left,
                    count: right,
                }
                | Expression::VectorCos(left, right) => {
                    expression(left, out);
                    expression(right, out);
                }
                Expression::Not(v)
                | Expression::Negate(v)
                | Expression::IsInAction(v)
                | Expression::MagicRemovePoints(v)
                | Expression::ActorListContains { actor: v, .. } => expression(v, out),
                Expression::Vector(values) => {
                    for value in values.iter() {
                        expression(value, out);
                    }
                }
                Expression::VectorComponent { vector, .. } => expression(vector, out),
                Expression::ArrayElement { array, index } => {
                    expression(array, out);
                    expression(index, out);
                }
                _ => {}
            }
        }
        fn block<'a>(statements: &'a [Statement], out: &mut Vec<&'a UnsupportedFragment>) {
            for statement in statements {
                match statement {
                    Statement::Unsupported(f) => out.push(f),
                    Statement::If {
                        condition,
                        then,
                        otherwise,
                    } => {
                        expression(condition, out);
                        block(then, out);
                        if let Some(otherwise) = otherwise {
                            block(otherwise, out);
                        }
                    }
                    Statement::Assign { value, .. }
                    | Statement::ActorFieldAssign { value, .. }
                    | Statement::MainActorFieldAssign { value, .. }
                    | Statement::SubtractAssign { value, .. }
                    | Statement::AddAssign { value, .. }
                    | Statement::ChangeAction(value)
                    | Statement::ExecuteAction(value)
                    | Statement::WaitFrames(value)
                    | Statement::Turn {
                        degrees_per_second: value,
                    }
                    | Statement::ChangeSighting(value)
                    | Statement::SetVectorSpeed(value)
                    | Statement::SetImpulse(value)
                    | Statement::Wait {
                        milliseconds: value,
                    } => expression(value, out),
                    Statement::ExecuteActionAndGenerateObject { action, vector, .. } => {
                        expression(action, out);
                        expression(vector, out);
                    }
                    Statement::ArrayAssign {
                        array,
                        index,
                        value,
                    } => {
                        expression(array, out);
                        expression(index, out);
                        expression(value, out);
                    }
                    Statement::SoundFadeMusic { arg0, arg1 } => {
                        expression(arg0, out);
                        expression(arg1, out);
                    }
                    Statement::SetHudVisible { visible, .. } => expression(visible, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        block(&self.statements, &mut out);
        out
    }
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
                Expression::Unsupported(fragment) => fragment.validate()?,
                Expression::Real(v) => ensure!(v.is_finite(), "nonfinite AI real"),
                Expression::Variable(id) => {
                    ensure!(usize::from(*id) < variables, "AI variable out of range")
                }
                Expression::ActorListContains { list, actor } => {
                    ensure!(usize::from(*list) < variables, "AI list out of range");
                    expression(actor, variables, depth + 1, count)?;
                }
                Expression::ActorFieldRead { actor, variable } => {
                    ensure!(
                        actor.path.member().is_none(),
                        "actor field read is a member"
                    );
                    ensure!(*variable < 255, "actor designer field out of range");
                }
                Expression::SelfFieldRead(variable) => {
                    ensure!(
                        usize::from(*variable) < variables,
                        "own designer field out of range"
                    );
                }
                Expression::MainActorFieldRead(variable) => {
                    ensure!(*variable < 255, "main actor designer field out of range");
                }
                Expression::Compare { left, right, .. }
                | Expression::And(left, right)
                | Expression::Or(left, right)
                | Expression::Multiply(left, right)
                | Expression::Plus(left, right)
                | Expression::Minus(left, right)
                | Expression::Divide(left, right)
                | Expression::TimeElapsed {
                    start: left,
                    duration: right,
                }
                | Expression::RandomInt {
                    lower: left,
                    upper: right,
                }
                | Expression::InventoryRemove {
                    item: left,
                    count: right,
                }
                | Expression::VectorCos(left, right) => {
                    expression(left, variables, depth + 1, count)?;
                    expression(right, variables, depth + 1, count)?;
                }
                Expression::Not(value)
                | Expression::Negate(value)
                | Expression::IsInAction(value)
                | Expression::InventoryQuantity(value)
                | Expression::MagicRemovePoints(value)
                | Expression::VectorComponent { vector: value, .. } => {
                    expression(value, variables, depth + 1, count)?
                }
                Expression::Vector(values) => {
                    for value in values.iter() {
                        expression(value, variables, depth + 1, count)?;
                    }
                }
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
                    Statement::Unsupported(fragment) => fragment.validate()?,
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
                    | Statement::SubtractAssign { variable, value }
                    | Statement::AddAssign { variable, value } => {
                        ensure!(
                            usize::from(*variable) < variables,
                            "AI assignment out of range"
                        );
                        expression(value, variables, depth + 1, count)?;
                    }
                    Statement::VectorComponentAssign { vector, value, .. } => {
                        ensure!(
                            usize::from(*vector) < variables,
                            "AI vector assignment out of range"
                        );
                        expression(value, variables, depth + 1, count)?;
                    }
                    Statement::ActorFieldAssign {
                        actor,
                        variable,
                        value,
                    } => {
                        ensure!(
                            actor.path.member().is_none(),
                            "actor field target is a member"
                        );
                        ensure!(*variable < 255, "actor designer field out of range");
                        expression(value, variables, depth + 1, count)?;
                    }
                    Statement::MainActorFieldAssign { variable, value } => {
                        ensure!(*variable < 255, "main actor designer field out of range");
                        expression(value, variables, depth + 1, count)?;
                    }
                    Statement::ActorListAdd { list, actor } => {
                        ensure!(usize::from(*list) < variables, "AI list out of range");
                        ensure!(actor.path.member().is_none(), "list actor is a member");
                    }
                    Statement::Increment { target } | Statement::Decrement { target } => {
                        match target {
                            FieldTarget::Variable(variable) => ensure!(
                                usize::from(*variable) < variables,
                                "AI field adjustment out of range"
                            ),
                            FieldTarget::ActorField { actor, variable } => {
                                ensure!(
                                    actor.path.member().is_none(),
                                    "actor field target is a member"
                                );
                                ensure!(*variable < 255, "actor designer field out of range");
                            }
                            FieldTarget::MainActorField(variable) => {
                                ensure!(*variable < 255, "main actor designer field out of range");
                            }
                        }
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
                    Statement::SetVectorSpeed(value)
                    | Statement::SetImpulse(value)
                    | Statement::ChangeSighting(value) => {
                        expression(value, variables, depth + 1, count)?
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
                    Statement::ExecuteActionAndGenerateObject {
                        template,
                        action,
                        vector,
                    } => {
                        ensure!(
                            template.path.member().is_none(),
                            "object template is a member"
                        );
                        expression(action, variables, depth + 1, count)?;
                        expression(vector, variables, depth + 1, count)?;
                    }
                    Statement::SoundFadeMusic { arg0, arg1 } => {
                        expression(arg0, variables, depth + 1, count)?;
                        expression(arg1, variables, depth + 1, count)?;
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

impl UnsupportedFragment {
    fn validate(&self) -> Result<()> {
        ensure!(
            !self.reason.is_empty(),
            "unsupported AI fragment lacks a reason"
        );
        ensure!(
            !self.nodes.is_empty() && self.nodes.len() <= 65535,
            "unsupported AI fragment extent differs"
        );
        ensure!(
            self.start as usize + self.nodes.len() <= 65535,
            "unsupported AI fragment source range differs"
        );
        Ok(())
    }
}
