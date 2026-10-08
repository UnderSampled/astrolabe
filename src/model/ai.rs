//! Useful behavior trees and per-instance memory, independent of source opcodes.
use super::{
    AssetPath, AssetRef,
    audio::SoundEventLink,
    character::{ObjectList, Perso, StateRef},
    geometry::Material,
    io::Content,
    route::Waypoint,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

pub mod walk;
use walk::AstNode;

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
    /// Semantic Ultra receiver, when the preserved fragment was a call on a
    /// resolved Perso. The raw nodes remain authoritative for the call body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receiver: Option<ActorTarget>,
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
    String(String),
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
    /// Evaluate a source function for its side effects, discarding its result.
    Evaluate(Expression),
    /// Actor-local life APIs; source byte narrowing happens at the native API.
    SetHitPointsMaximum {
        actor: Expression,
        value: Expression,
    },
    AddHitPoints {
        actor: Expression,
        value: Expression,
    },
    /// Shared magick maximum; changing it does not refill/clamp current points.
    SetMagicPointsMaximum(Expression),
    /// Changes the first matching inventory definition, without changing quantity.
    SetInventoryMaximum {
        item: Expression,
        value: Expression,
    },
    ActivateMapName {
        name: Expression,
        active: Expression,
    },
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
    /// Proc_ChangeComport (category 4, operation 207) switches a resolved
    /// actor to a referenced Normal behavior. Cross-actor requests are handled
    /// by that actor's native Brain; the target reference may be cross-model.
    ChangeActorBehavior {
        actor: ActorTarget,
        target: BehaviorRef,
    },
    ChangeAction(Expression),
    Turn {
        degrees_per_second: Expression,
    },
    /// Action_IncrementalTurnPerso snapshots the shortest horizontal turn and
    /// applies equal steps over a source frame count; negative counts mean one.
    IncrementalTurnToward {
        target: ActorTarget,
        frames: Expression,
    },
    /// Meta19 captures a point and heading once; nonpositive signed counts
    /// complete without a turn, unlike Meta23's actor-target variant.
    IncrementalTurnPosition {
        point: Expression,
        frames: Expression,
    },
    /// Source motion requests retain their order in the AI schedule. The
    /// character component owns their runtime effects.
    SetVectorSpeed(Expression),
    /// Proc_AddWind is retained as typed source data; the native target accepts
    /// it as an explicitly deferred no-op until source wind parity is ported.
    AddWind(Expression),
    /// Hype category-23 operands are actor-local route slot indices.
    ResetRoute(u8),
    GoOverRoute(u8),
    /// Meta30 visits the next selected waypoint in one actor-local route slot.
    /// Unlike GoOverRoute, it completes after that point's progress update.
    GoToNextWaypoint(u8),
    /// Meta32/33 capture a point and radius once. The horizontal variant
    /// refreshes the point's height from the actor before each native update.
    GoToPosition {
        point: Expression,
        radius: Expression,
        horizontal: bool,
    },
    GoRelative,
    /// Proc_KillPerso(Me): remove the current generated Perso.
    KillSelf,
    /// Hype's Proc_KillPerso takes a Perso operand. Preserve non-self targets
    /// by identity even when they are outside the rooted actor graph.
    KillPerso {
        target: KillPersoTarget,
    },
    /// Proc_ExecuteFade's source alpha range and duration in seconds.
    ExecuteFade {
        from: Expression,
        to: Expression,
        duration_seconds: Expression,
    },
    /// Proc_SetGameState writes one selector into the game-wide source byte.
    /// The native session owns the resulting mask across level changes.
    SetGameState(Expression),
    /// Proc_ActivateObject and Proc_DeactivateObject mutate the source object
    /// active bit on this referenced Perso. This state is separate from draw
    /// visibility and actor AI activation.
    SetObjectActive {
        actor: ActorTarget,
        active: bool,
    },
    /// Source screen-fade material selection and value remain presentation API
    /// calls owned by the active screen.
    SetFadeTexture(Expression),
    SetFadeValue(Expression),
    ActivateFade,
    DeactivateFade,
    /// Proc_SetVisualGMTDiffuseCoef (category 4, operation 237) overwrites
    /// the four source diffuse coefficients on a referenced GMT visual
    /// material from the packed bytes of one live designer variable. The
    /// target keeps the shared source Material identity; the renderer owns
    /// the material-wide state and its lifetime.
    SetVisualGMTDiffuseCoef {
        material: AssetRef<Material>,
        value: Expression,
    },
    /// Meta-action 17 selects an action from the receiver's distance to a
    /// runtime Perso operand, then repeatedly adjusts its position toward that
    /// Perso. Source init checks strict `distance < arg1` first and selects
    /// `near_action` (operand 4); only otherwise it checks strict
    /// `distance > arg2` and selects `far_action` (operand 3). If neither
    /// branch matches it selects no action. The update routine's
    /// movement/completion contract is not yet ported.
    AdjustPersoOnObject {
        target: Expression,
        near_threshold: Expression,
        far_threshold: Expression,
        far_action: Expression,
        near_action: Expression,
    },
    SetImpulse(Expression),
    JumpAbsolute,
    JumpWithoutAddingSpeed,
    SetParticleGeneratorEnabled(bool),
    SetParticleGenerator(AssetRef<super::particle::ParticleGenerator>),
    /// Original laser-pointer presentation is global; aiming reads its last
    /// target independently of whether the pointer is visible.
    SetLaserPointerActive(bool),
    /// Proc_SetLaserPointerSniperMode changes the native pointer's aiming mode.
    SetLaserPointerSniperMode(Expression),
    /// Proc_SetLaserPointerFlyMode stores its normalized mode bit, clears sniper
    /// mode when enabled, and selects the source pointer scale (3.0 or 1.0).
    SetLaserPointerFlyMode(Expression),
    /// Proc_InvisibleDisplay / Proc_VisibleDisplay toggle rendered visibility
    /// while leaving the actor's native physics and AI active.
    SetDisplayVisible(bool),
    /// Proc_InvisibleDisplay / Proc_VisibleDisplay executed on a source Ultra
    /// receiver. Keep the receiver as a keyword or semantic Perso reference.
    SetActorDisplayVisible {
        actor: ActorTarget,
        visible: bool,
    },
    /// Proc_ActivateChannel / Proc_DeactivateChannel on an Ultra receiver.
    SetActorChannelActive {
        actor: ActorTarget,
        channel: u32,
        active: bool,
    },
    /// Source opacity in 0..255, clamped by the actor API independently of mode.
    SetTransparency(Expression),
    /// TransparentDisplay enables actor opacity; DefaultDisplay disables it.
    SetTransparentDisplay(bool),
    /// Fight_ChangeArmor selects the shared source armor definition by byte index.
    /// Each active level binds the Fix-owned catalog through shared state.
    SelectArmor(Expression),
    /// Fight_ChangeArmorResistance changes the selected Fix definition's rank.
    SetArmorResistance(Expression),
    /// Fight_RepairArmor clears piece damage while retaining the current piece.
    RepairArmor,
    /// Proc82 swaps complete parts in the actor's selected configuration.
    /// The original one-based indices are preserved as source operands.
    SwapConfiguredParts {
        first: Expression,
        second: Expression,
    },
    /// Proc83 replaces the actor's configuration selection without resetting it.
    ChangeConfiguration(Expression),
    ChangeSighting(Expression),
    /// Camera procedure 160 stores a keyword or referenced target actor.
    CameraChangeTargetedPerso {
        camera: AssetRef<Perso>,
        target: ActorTarget,
    },
    /// Source camera procedure 163 restores the component target to MainActor.
    CameraResetTargetedPerso {
        camera: AssetRef<Perso>,
    },
    /// Source camera procedure 157 selects a motion mode; -1 restores the
    /// original preset's mode. Unsupported transitions fail at the target API.
    CameraChangeState {
        camera: AssetRef<Perso>,
        state: Expression,
    },
    /// Proc_SetPersoAbsolutePosition evaluated on a referenced Ultra receiver.
    SetActorAbsolutePosition {
        actor: AssetRef<Perso>,
        position: Expression,
    },
    /// Source Ultra receiver keyword for an embedded absolute-position call.
    SetKeywordAbsolutePosition {
        actor: ActorKeyword,
        position: Expression,
    },
    /// Source procedure 253 sets the executing Perso's absolute world position.
    SetSelfAbsolutePosition(Expression),
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
    /// Source Proc_ChangeMap loads a named map and uses its authored main
    /// actor placement when no named entry is supplied.
    ChangeMap {
        level: String,
    },
    /// Source Proc_ChangeMapAtPosition selects authored Hype and camera entry
    /// Persos by their source names in the destination level.
    ChangeMapAtPosition {
        level: String,
        entry: String,
        camera_entry: String,
    },
    ToggleWideScreen,
    PositionDialog {
        x: Expression,
        y: Expression,
    },
    RunDialog {
        name: String,
        mode: Expression,
    },
    ScrollDialog(Expression),
    SelectDialog,
    EndDialog,
    ChangeDialogBehavior,
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
    /// Ultra Proc127 dispatches on the resolved actor while operands use the caller.
    SoundOnActor {
        actor: ActorTarget,
        event: Option<SoundEventLink>,
    },
    /// Proc_SendMusicRequest asks the shared source music channel to play an
    /// event from the sound library.
    MusicRequest {
        event: Option<SoundEventLink>,
    },
    /// Proc402/403 select the microphone from the first original-caller
    /// operand. Proc403 evaluates and discards its second actor operand.
    SwitchMicrophone {
        actor: Expression,
        evaluated_second: Option<Expression>,
    },
    /// Proc413 sets the receiver's effects volume to zero, sends the event,
    /// then starts a nonblocking fade. Operands use the original caller.
    SoundFadeInObject {
        actor: ActorTarget,
        target: Expression,
        duration_ms: Expression,
        event: Option<SoundEventLink>,
    },
    /// Hype Proc_SND_FadeMusic: arg0 is target level 0..127 and arg1 is
    /// duration in milliseconds, verified at the native mixer helper.
    SoundFadeMusic {
        arg0: Expression,
        arg1: Expression,
    },
    /// Hype Proc_SND_FadeVoice uses the global voice channel. The target is
    /// clamped to 0..127 and the duration shares FadeMusic's millisecond clock.
    SoundFadeVoice {
        target: Expression,
        duration_ms: Expression,
    },
    /// Hype Proc_SND_FadeSound uses the global sound/effects channel. The target
    /// is clamped to 0..127 and the duration shares FadeMusic's millisecond clock.
    SoundFadeSound {
        target: Expression,
        duration_ms: Expression,
    },
    /// Proc_SND_FadeAll applies one target level (0..127) and duration
    /// (milliseconds) to music, effects and voice channels.
    SoundFadeAll {
        target: Expression,
        duration_ms: Expression,
    },
    /// Proc_SND_KillMusic stops the current music channel.
    SoundKillMusic,
    Noop,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HudIndicator {
    Life,
    Armor,
    Magic,
    SelectedObject,
    Weapon,
    /// Source Stats_DisplayCash toggles the coin icon/count in the shared HUD.
    Cash,
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
    /// A source input operand with no logical entry. Pressed/edge queries are
    /// false, while Released is true; this is distinct from an unbound handle.
    AbsentButton(ButtonQuery),
    /// Source Keyword_Nobody: an absent actor reference.
    Nobody,
    /// Source Keyword_Nowhere: an absent waypoint reference.
    Nowhere,
    /// A source actor handle or one of the two AI actor keywords.
    Actor(ActorTarget),
    /// Category35 ObjectTableRef, including a null selection.
    Configuration(Option<AssetRef<ObjectList>>),
    Integer(i32),
    Real(f32),
    /// Source Func_Int truncates a real-valued expression to an integer.
    IntegerCast(Box<Expression>),
    /// Func_Square and Func_AbsoluteValue return a rounded source real.
    Square(Box<Expression>),
    AbsoluteValue(Box<Expression>),
    /// Source vector literal (category 16); components remain source-space.
    Vector(Box<[Expression; 3]>),
    /// Current actor's world position in source coordinates.
    SelfPosition,
    /// Ultra Func0 reads the resolved receiver's current position.
    ActorPosition(ActorTarget),
    /// Source Field.Position reads the current receiver's world translation.
    /// Keep this distinct from Func_GetPersoAbsolutePosition for source fidelity.
    SelfPositionField,
    /// A category-29 source operand used as a typed Waypoint value (for
    /// example, a saved designer-array assignment), distinct from its position.
    WaypointReference(AssetRef<Waypoint>),
    /// Func_GetWPAbsolutePosition resolves a meaningful shared waypoint asset.
    WaypointPosition(AssetRef<Waypoint>),
    /// Func_DistanceToWP evaluates a source-space waypoint distance on its
    /// receiver (self unless an enclosing Operator_Ultra selects another actor).
    DistanceToWaypoint {
        actor: ActorTarget,
        waypoint: AssetRef<Waypoint>,
    },
    /// Function_Camera_GetShiftAlpha (category 3/id 59, native 0x0053d300)
    /// takes a typed integer selector and a Perso receiver. The observed
    /// camera-model calls pass selectors 1 and 0 with SelfActor; its float
    /// result is the receiver's camera-parameter +0x44 value, unchanged.
    CameraShiftAlpha {
        selector: Box<Expression>,
        receiver: ActorTarget,
    },
    /// Function_Camera_GetShiftTeta (category 3/id 61, native 0x0053d300)
    /// takes a typed integer selector and a Perso receiver. The observed
    /// camera-model calls pass selectors 1 and 0 with SelfActor; the native
    /// accessor returns camera-parameter +0x4c unchanged.
    CameraShiftTeta {
        selector: Box<Expression>,
        receiver: ActorTarget,
    },
    /// Cond_CollisionWP (category 1/op 108, 005356c0) tests 0052e810's full
    /// 3D distance minus 005169e0's radius <= 0. 005173f0 resolves an
    /// actor-relative waypoint location each evaluation. Expressions retain
    /// dynamic ArrayElement selection and are evaluated on every check.
    WaypointSphereContains {
        actor: Box<Expression>,
        waypoint: Box<Expression>,
    },
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
    /// Source Func_GetCurrectAnimFrame reads the active animation's discrete
    /// frame cursor. Godot lowering requires a bound native animation clip.
    CurrentAnimationFrame,
    DialogIsOver,
    /// Cond120 has Cond121's MainActor speech gate and consumes the pending
    /// dialogue behavior callback when the condition succeeds.
    DialogIsOverAndChangeBehavior,
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
        actor: ActorTarget,
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
    /// Func_DistanceToPerso resolves a typed actor operand from the active scene.
    ActorDistance(Box<Expression>),
    /// Func_DistanceToPerso evaluated by an Ultra receiver other than the
    /// executing actor. Both receiver and target retain source actor identity.
    ActorDistanceFrom {
        receiver: ActorTarget,
        target: Box<Expression>,
    },
    /// Func1 measures the signed shortest horizontal angle to its actor
    /// operand, in source degrees, after resolving an optional Ultra receiver.
    ActorFacingAngle {
        receiver: ActorTarget,
        target: Box<Expression>,
    },
    /// The installed Hype category-3/raw-ID-8 handler returns the source-XY
    /// magnitude between its current actor and the actor operand in that
    /// operand's local frame. The installed table names row 8
    /// `Func_DistanceXZToPerso`, while the handler passes selector 3 to the
    /// helper, whose selector 3 computes XY. Keep this behavior named from the
    /// handler and preserve the API-name mismatch in source evidence.
    ActorLocalSourceXYDistance(Box<Expression>),
    /// Cond_IsValidObject observes whether its actor handle is non-null/valid.
    IsValidActor(Box<Expression>),
    /// Func_ApplyArmoredDamage mutates shared armor and returns remaining main life.
    ApplyArmoredDamage(Box<Expression>),
    /// Source functions read selected armor state and the shared definition count.
    ArmorResistance,
    ArmorCount,
    /// Func_GetHitPoints(Me): read the current actor's mutable life value.
    SelfHitPoints,
    /// Func38 evaluates its actor operand on the original caller and reads the
    /// actor-owned maximum life byte.
    ActorHitPointsMaximum(Box<Expression>),
    /// Source inventory functions operate on the game inventory, not on a
    /// projectile's own designer fields.
    InventoryWeapon,
    InventoryAddObject(Box<Expression>),
    InventoryQuantity(Box<Expression>),
    InventoryRemove {
        item: Box<Expression>,
        count: Box<Expression>,
    },
    /// Source magick functions read the shared selected spell and mana pool.
    MagicCurrentSpell,
    MagicPoints,
    MagicPointsMaximum,
    MagicAddPoints(Box<Expression>),
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
    /// Hype Operator_VectorPlusVector (category 2, operation 16).
    VectorPlus(Box<Expression>, Box<Expression>),
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
/// Preserve existing keyword serialization while allowing a rooted Perso link.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ActorTarget {
    Keyword(ActorKeyword),
    Reference(AssetRef<Perso>),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KillPersoTarget {
    SelfActor,
    MainActor,
    Reference(AssetRef<Perso>),
    /// Source actor operand not yet promoted into the active useful Perso graph.
    Source(SourceObjectIdentity),
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
    /// A resolved reference to the existing shared waypoint asset model.
    Waypoint(Option<AssetRef<Waypoint>>),
    /// Resolved waypoint assets selected through a source array operation.
    WaypointArray(DesignerArray<Option<AssetRef<Waypoint>>>),
    /// Preserved bytes for a source designer field whose value semantics are
    /// not understood. The payload is never traversed as pointers or executed.
    Opaque {
        kind: u32,
        bytes: Vec<u8>,
    },
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
    fn scripts(&self) -> Vec<&Script> {
        [&self.normal, &self.reflex]
            .into_iter()
            .flatten()
            .flat_map(|set| &set.behaviors)
            .flat_map(|behavior| {
                behavior
                    .rules
                    .iter()
                    .flatten()
                    .chain(behavior.schedule.iter())
            })
            .collect()
    }

    pub fn particle_generators(&self) -> Vec<AssetRef<super::particle::ParticleGenerator>> {
        let mut out = Vec::new();
        for script in self.scripts() {
            for (node, _) in script.walk() {
                if let AstNode::Statement(Statement::SetParticleGenerator(reference)) = node
                    && !out.contains(reference)
                {
                    out.push(reference.clone());
                }
            }
        }
        out
    }
    pub fn object_templates(&self) -> Vec<AssetRef<Perso>> {
        let mut out = Vec::new();
        for script in self.scripts() {
            for (node, _) in script.walk() {
                if let AstNode::Statement(Statement::ExecuteActionAndGenerateObject {
                    template,
                    ..
                }) = node
                    && !out.contains(template)
                {
                    out.push(template.clone());
                }
            }
        }
        out
    }
    pub fn actor_field_writes(&self) -> Vec<(AssetRef<Perso>, u16)> {
        let mut out = Vec::new();
        for script in self.scripts() {
            for (node, _) in script.walk() {
                let entry = match node {
                    AstNode::Statement(Statement::ActorFieldAssign {
                        actor, variable, ..
                    })
                    | AstNode::Statement(Statement::Increment {
                        target: FieldTarget::ActorField { actor, variable },
                    })
                    | AstNode::Statement(Statement::Decrement {
                        target: FieldTarget::ActorField { actor, variable },
                    }) => Some((actor.clone(), *variable)),
                    _ => None,
                };
                if let Some(entry) = entry
                    && !out.contains(&entry)
                {
                    out.push(entry);
                }
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
    pub fn actor_field_accesses(&self) -> Vec<(AssetRef<Perso>, u16)> {
        let mut out = self.actor_field_writes();
        for entry in self.actor_field_reads() {
            if !out.contains(&entry) {
                out.push(entry);
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
    pub fn expressions(&self) -> Vec<&Expression> {
        self.scripts()
            .into_iter()
            .flat_map(Script::walk)
            .filter_map(|(node, _)| match node {
                AstNode::Expression(expression) => Some(expression),
                AstNode::Statement(_) => None,
            })
            .collect()
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
    pub fn waypoint_references(&self) -> Vec<&AssetRef<Waypoint>> {
        let mut out: Vec<_> = self
            .expressions()
            .into_iter()
            .filter_map(|e| match e {
                Expression::WaypointPosition(reference)
                | Expression::WaypointReference(reference)
                | Expression::DistanceToWaypoint {
                    waypoint: reference,
                    ..
                } => Some(reference),
                _ => None,
            })
            .collect();
        for variable in &self.variables {
            match &variable.default {
                DesignerValue::Waypoint(Some(reference)) => out.push(reference),
                DesignerValue::WaypointArray(array) => {
                    out.extend(array.values.iter().flatten());
                }
                _ => {}
            }
        }
        out
    }
    pub fn configuration_references(&self) -> Vec<&AssetRef<ObjectList>> {
        let mut out = Vec::new();
        for expression in self.expressions() {
            if let Expression::Configuration(Some(reference)) = expression
                && !out.contains(&reference)
            {
                out.push(reference);
            }
        }
        out
    }
    pub fn behavior_references(&self) -> Vec<&BehaviorRef> {
        self.behavior_references_with_owner()
            .into_iter()
            .map(|(reference, _)| reference)
            .collect()
    }
    fn behavior_references_with_owner(&self) -> Vec<(&BehaviorRef, bool)> {
        let mut out = Vec::new();
        let mut queries = Vec::new();
        for script in self.scripts() {
            for (node, _) in script.walk() {
                match node {
                    AstNode::Statement(Statement::ChangeBehavior { target, .. }) => {
                        out.push((target, true));
                    }
                    AstNode::Statement(Statement::ChangeActorBehavior { target, .. }) => {
                        out.push((target, false));
                    }
                    AstNode::Expression(Expression::IsInBehavior { target, .. }) => {
                        queries.push((target, false));
                    }
                    _ => {}
                }
            }
        }
        out.extend(queries);
        out
    }
    pub fn sound_events(&self) -> Vec<&SoundEventLink> {
        let mut out = Vec::new();
        for script in self.scripts() {
            for (node, _) in script.walk() {
                if let AstNode::Statement(
                    Statement::Sound { event: Some(event) }
                    | Statement::SoundOnActor {
                        event: Some(event), ..
                    }
                    | Statement::MusicRequest { event: Some(event) }
                    | Statement::SoundFadeInObject {
                        event: Some(event), ..
                    },
                ) = node
                {
                    out.push(event);
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
            Self::Waypoint(Some(reference)) => ensure!(
                reference.path.member().is_none(),
                "designer waypoint must identify its asset"
            ),
            Self::WaypointArray(v) => {
                v.validate()?;
                ensure!(
                    v.values
                        .iter()
                        .flatten()
                        .all(|reference| reference.path.member().is_none()),
                    "designer waypoint array entry must identify its asset"
                );
            }
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
            Self::Opaque { bytes, .. } => {
                ensure!(!bytes.is_empty(), "empty opaque designer field");
            }
            _ => {}
        }
        Ok(())
    }
    pub fn same_type(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Actor(_), Self::SourceActor(_)) | (Self::SourceActor(_), Self::Actor(_)) => true,
            (Self::Waypoint(_), Self::SourceWaypoint(_))
            | (Self::SourceWaypoint(_), Self::Waypoint(_)) => true,
            (Self::IntegerArray(a), Self::IntegerArray(b)) => a.values.len() == b.values.len(),
            (Self::RealArray(a), Self::RealArray(b)) => a.values.len() == b.values.len(),
            (Self::VectorArray(a), Self::VectorArray(b)) => a.values.len() == b.values.len(),
            (Self::ActionArray(a), Self::ActionArray(b)) => a.values.len() == b.values.len(),
            (Self::SourceWaypointArray(a), Self::SourceWaypointArray(b)) => {
                a.values.len() == b.values.len()
            }
            (Self::WaypointArray(a), Self::SourceWaypointArray(b))
            | (Self::SourceWaypointArray(b), Self::WaypointArray(a)) => {
                a.values.len() == b.values.len()
            }
            (Self::WaypointArray(a), Self::WaypointArray(b)) => a.values.len() == b.values.len(),
            (Self::ActorList(a), Self::ActorList(b)) => a.slots.len() == b.slots.len(),
            (
                Self::Opaque {
                    kind: a_kind,
                    bytes: a,
                },
                Self::Opaque {
                    kind: b_kind,
                    bytes: b,
                },
            ) => a_kind == b_kind && a.len() == b.len(),
            _ => std::mem::discriminant(self) == std::mem::discriminant(other),
        }
    }
}
impl Script {
    pub fn unsupported_fragments(&self) -> Vec<&UnsupportedFragment> {
        self.walk()
            .filter_map(|(node, _)| match node {
                AstNode::Statement(Statement::Unsupported(fragment))
                | AstNode::Expression(Expression::Unsupported(fragment)) => Some(fragment),
                _ => None,
            })
            .collect()
    }
    pub fn validate(&self, variables: usize) -> Result<()> {
        let mut walked = 0usize;
        for (node, depth) in self.walk() {
            walked += 1;
            ensure!(depth <= 64 && walked <= 65535, "excessive AI tree");
            match node {
                AstNode::Statement(Statement::Unsupported(fragment))
                | AstNode::Expression(Expression::Unsupported(fragment)) => fragment.validate()?,
                _ => {}
            }
        }
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
                Expression::WaypointPosition(r) | Expression::WaypointReference(r) => {
                    ensure!(r.path.member().is_none(), "waypoint is a member")
                }
                Expression::DistanceToWaypoint { actor, waypoint } => {
                    ensure!(
                        waypoint.path.member().is_none(),
                        "distance waypoint is a member"
                    );
                    if let ActorTarget::Reference(actor) = actor {
                        ensure!(actor.path.member().is_none(), "distance actor is a member");
                    }
                }
                Expression::ActorDistanceFrom { receiver, target }
                | Expression::ActorFacingAngle { receiver, target } => {
                    if let ActorTarget::Reference(actor) = receiver {
                        ensure!(
                            actor.path.member().is_none(),
                            "distance receiver is a member"
                        );
                    }
                    expression(target, variables, depth + 1, count)?;
                }
                Expression::ActorPosition(ActorTarget::Reference(actor)) => {
                    ensure!(actor.path.member().is_none(), "position actor is a member");
                }
                Expression::CameraShiftAlpha { selector, receiver }
                | Expression::CameraShiftTeta { selector, receiver } => {
                    if let ActorTarget::Reference(actor) = receiver {
                        ensure!(actor.path.member().is_none(), "camera receiver is a member");
                    }
                    expression(selector, variables, depth + 1, count)?;
                }
                Expression::IsInBehavior {
                    actor: ActorTarget::Reference(actor),
                    target,
                } => {
                    ensure!(
                        actor.path.member().is_none(),
                        "behavior query actor is a member"
                    );
                    ensure!(
                        target.group == BehaviorGroup::Normal,
                        "referenced actors in Reflex behavior queries are unsupported"
                    );
                }
                Expression::Actor(ActorTarget::Reference(actor)) => {
                    ensure!(
                        actor.path.member().is_none(),
                        "actor expression is a member"
                    )
                }
                Expression::Configuration(Some(reference)) => {
                    ensure!(
                        reference.path.member().is_none(),
                        "configuration is a member"
                    )
                }
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
                | Expression::VectorPlus(left, right)
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
                | Expression::VectorCos(left, right)
                | Expression::WaypointSphereContains {
                    actor: left,
                    waypoint: right,
                } => {
                    expression(left, variables, depth + 1, count)?;
                    expression(right, variables, depth + 1, count)?;
                }
                Expression::Not(value)
                | Expression::Negate(value)
                | Expression::IsInAction(value)
                | Expression::Square(value)
                | Expression::AbsoluteValue(value)
                | Expression::ActorDistance(value)
                | Expression::ActorHitPointsMaximum(value)
                | Expression::ActorLocalSourceXYDistance(value)
                | Expression::IsValidActor(value)
                | Expression::ApplyArmoredDamage(value)
                | Expression::InventoryQuantity(value)
                | Expression::InventoryAddObject(value)
                | Expression::MagicAddPoints(value)
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
                    Statement::RunDialog { name, mode } => {
                        ensure!(
                            !name.is_empty() && name.len() <= 255,
                            "invalid dialogue identity"
                        );
                        expression(mode, variables, depth + 1, count)?;
                    }
                    Statement::ChangeConfiguration(value) => {
                        ensure!(
                            matches!(value, Expression::Configuration(_)),
                            "configuration change requires a literal reference"
                        );
                        expression(value, variables, depth + 1, count)?;
                    }
                    Statement::SwapConfiguredParts { first, second } => {
                        expression(first, variables, depth + 1, count)?;
                        expression(second, variables, depth + 1, count)?;
                    }
                    Statement::ScrollDialog(value) => {
                        expression(value, variables, depth + 1, count)?
                    }
                    Statement::PositionDialog { x, y } => {
                        expression(x, variables, depth + 1, count)?;
                        expression(y, variables, depth + 1, count)?;
                    }
                    Statement::Wait { milliseconds } => {
                        expression(milliseconds, variables, depth + 1, count)?
                    }
                    Statement::Evaluate(index)
                    | Statement::SetMagicPointsMaximum(index)
                    | Statement::SelectArmor(index)
                    | Statement::SetArmorResistance(index) => {
                        expression(index, variables, depth + 1, count)?
                    }
                    Statement::SetHitPointsMaximum { actor, value }
                    | Statement::AddHitPoints { actor, value }
                    | Statement::SetInventoryMaximum { item: actor, value } => {
                        expression(actor, variables, depth + 1, count)?;
                        expression(value, variables, depth + 1, count)?;
                    }
                    Statement::ActivateMapName { name, active } => {
                        expression(name, variables, depth + 1, count)?;
                        expression(active, variables, depth + 1, count)?;
                    }
                    Statement::WaitFrames(frames) => {
                        expression(frames, variables, depth + 1, count)?
                    }
                    Statement::Turn { degrees_per_second } => {
                        expression(degrees_per_second, variables, depth + 1, count)?
                    }
                    Statement::IncrementalTurnToward { target, frames } => {
                        if let ActorTarget::Reference(actor) = target {
                            ensure!(actor.path.member().is_none(), "turn target is a member");
                        }
                        expression(frames, variables, depth + 1, count)?;
                    }
                    Statement::IncrementalTurnPosition { point, frames } => {
                        expression(point, variables, depth + 1, count)?;
                        expression(frames, variables, depth + 1, count)?;
                    }
                    Statement::GoToPosition { point, radius, .. } => {
                        expression(point, variables, depth + 1, count)?;
                        expression(radius, variables, depth + 1, count)?;
                    }
                    Statement::ChangeActorBehavior { actor, target } => {
                        if let ActorTarget::Reference(actor) = actor {
                            ensure!(actor.path.member().is_none(), "behavior actor is a member");
                        }
                        ensure!(
                            target.group == BehaviorGroup::Normal,
                            "Proc_ChangeComport requires a Normal behavior"
                        );
                    }
                    Statement::CameraChangeState { camera, state } => {
                        ensure!(camera.path.member().is_none(), "camera is a member");
                        expression(state, variables, depth + 1, count)?;
                    }
                    Statement::CameraChangeTargetedPerso { camera, target } => {
                        ensure!(camera.path.member().is_none(), "camera is a member");
                        if let ActorTarget::Reference(actor) = target {
                            ensure!(actor.path.member().is_none(), "camera target is a member");
                        }
                    }
                    Statement::CameraResetTargetedPerso { camera } => {
                        ensure!(camera.path.member().is_none(), "camera is a member");
                    }
                    Statement::SetActorAbsolutePosition { actor, position } => {
                        ensure!(
                            actor.path.member().is_none(),
                            "position receiver is a member"
                        );
                        expression(position, variables, depth + 1, count)?;
                    }
                    Statement::SetActorDisplayVisible {
                        actor: ActorTarget::Reference(actor),
                        ..
                    } => {
                        ensure!(
                            actor.path.member().is_none(),
                            "display receiver is a member"
                        );
                    }
                    Statement::SetActorChannelActive {
                        actor: ActorTarget::Reference(actor),
                        ..
                    } => {
                        ensure!(
                            actor.path.member().is_none(),
                            "channel receiver is a member"
                        );
                    }
                    Statement::SetKeywordAbsolutePosition { position, .. }
                    | Statement::SetGameState(position)
                    | Statement::SetFadeTexture(position)
                    | Statement::SetFadeValue(position) => {
                        expression(position, variables, depth + 1, count)?;
                    }
                    Statement::SetObjectActive {
                        actor: ActorTarget::Reference(actor),
                        ..
                    } => {
                        ensure!(actor.path.member().is_none(), "source object is a member");
                    }
                    Statement::SetSelfAbsolutePosition(position) => {
                        expression(position, variables, depth + 1, count)?;
                    }
                    Statement::SetVectorSpeed(value)
                    | Statement::AddWind(value)
                    | Statement::SetImpulse(value)
                    | Statement::ChangeSighting(value)
                    | Statement::SetTransparency(value) => {
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
                    Statement::SoundOnActor {
                        actor: ActorTarget::Reference(actor),
                        ..
                    } => {
                        ensure!(actor.path.member().is_none(), "sound receiver is a member");
                    }
                    Statement::SoundFadeInObject {
                        actor,
                        target,
                        duration_ms,
                        ..
                    } => {
                        if let ActorTarget::Reference(actor) = actor {
                            ensure!(
                                actor.path.member().is_none(),
                                "sound fade receiver is a member"
                            );
                        }
                        expression(target, variables, depth + 1, count)?;
                        expression(duration_ms, variables, depth + 1, count)?;
                    }
                    Statement::SwitchMicrophone {
                        actor,
                        evaluated_second,
                    } => {
                        expression(actor, variables, depth + 1, count)?;
                        if let Some(second) = evaluated_second {
                            expression(second, variables, depth + 1, count)?;
                        }
                    }
                    Statement::SoundFadeVoice {
                        target,
                        duration_ms,
                    }
                    | Statement::SoundFadeSound {
                        target,
                        duration_ms,
                    } => {
                        expression(target, variables, depth + 1, count)?;
                        expression(duration_ms, variables, depth + 1, count)?;
                    }
                    Statement::ExecuteFade {
                        from,
                        to,
                        duration_seconds,
                    } => {
                        expression(from, variables, depth + 1, count)?;
                        expression(to, variables, depth + 1, count)?;
                        expression(duration_seconds, variables, depth + 1, count)?;
                    }
                    Statement::SetVisualGMTDiffuseCoef { material, value } => {
                        ensure!(
                            material.path.member().is_none(),
                            "visual material target is a member"
                        );
                        ensure!(
                            matches!(value, Expression::Variable(_)),
                            "visual diffuse coefficient requires a direct designer variable"
                        );
                        expression(value, variables, depth + 1, count)?;
                    }
                    Statement::SoundFadeAll {
                        target,
                        duration_ms,
                    } => {
                        expression(target, variables, depth + 1, count)?;
                        expression(duration_ms, variables, depth + 1, count)?;
                    }
                    Statement::ChangeMap { level } => {
                        ensure!(!level.is_empty() && level.len() <= 255, "invalid map name")
                    }
                    Statement::ChangeMapAtPosition {
                        level,
                        entry,
                        camera_entry,
                    } => ensure!(
                        [level, entry, camera_entry]
                            .iter()
                            .all(|value| !value.is_empty() && value.len() <= 255),
                        "invalid positioned map entry"
                    ),
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
        if let Some(ActorTarget::Reference(actor)) = &self.receiver {
            ensure!(
                actor.path.member().is_none(),
                "unsupported receiver is a member"
            );
        }
        Ok(())
    }
}
