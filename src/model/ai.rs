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
    /// Fight_SubHitPointsEnemy (4-432, 0053dd60): the statement form of
    /// Func225; same 00460b10 subtraction and 0040c4a0 meter refresh. Holds
    /// the FightSubtractAndGetHitPoints expression whose result is dropped.
    FightSubHitPointsEnemy(Expression),
    /// Proc_SetActionReturn(1) before a procedure: the preceding statement's
    /// control becomes "hold" (00522750 ORs in bit 1). In a rule, rules stop
    /// for this frame, the schedule is skipped and the next frame starts at
    /// this rule (00522840 storage+7); in a schedule, it resumes after the
    /// statement next frame (00522930). Encoded before that statement.
    HoldAfterPrevious,
    /// Actor-local life APIs; source byte narrowing happens at the native API.
    SetHitPointsMaximum {
        actor: Expression,
        value: Expression,
    },
    AddHitPoints {
        actor: Expression,
        value: Expression,
    },
    /// Proc_SetHitPoints: clamped by the actor's maximum.
    SetHitPoints {
        actor: Expression,
        value: Expression,
    },
    SubtractHitPoints {
        actor: Expression,
        value: Expression,
    },
    /// Session-wide death request, independent of an actor's life byte.
    PlayerIsDead,
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
    MultiplyAssign {
        variable: u16,
        value: Expression,
    },
    /// Capture the observed Brain and expected value once, then await its
    /// ordinary Boolean designer property to equal that captured value.
    WaitBoolean {
        field: Expression,
        expected: Expression,
    },
    SetStockCount {
        actor: Expression,
        count: Expression,
    },
    DisplayPickup {
        item: Box<Expression>,
        milliseconds: Box<Expression>,
        from: Box<Expression>,
        to: Box<Expression>,
        x: Box<Expression>,
        y: Box<Expression>,
        width: Box<Expression>,
        height: Box<Expression>,
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
    /// Capture the observed Brain/field, comparison DWORD and expected signed
    /// value at entry. Poll only the ordinary observed property. Modes 0..5
    /// resume on ==, !=, <, >, <=, >=; other DWORD modes complete immediately.
    WaitInteger {
        left: Expression,
        selector: Expression,
        right: Expression,
    },
    /// Proc_ChangeMyComport/Reflex (209/210) whose operand is a designer
    /// variable of DSG type 13 (a behavior pointer) rather than a literal.
    /// Proc_ChangeMyComport(PrevComportIntell) / Proc_ChangeMyComportReflex
    /// (PrevComportReflex): fields 7 and 8 read the group's previous behavior,
    /// which 005216e0 saves on every change.
    ChangeBehaviorToPrevious {
        group: BehaviorGroup,
        pause_after: bool,
    },
    ChangeBehaviorVariable {
        variable: u16,
        group: BehaviorGroup,
        pause_after: bool,
    },
    ChangeBehavior {
        target: BehaviorRef,
        /// Yield the calling flow after the request; a change to that same
        /// behavior group consequently enters its new behavior next tick.
        pause_after: bool,
    },
    /// Proc_ChangeComport / Proc_ChangeComportReflex (207/208) switch a resolved
    /// actor to a referenced Normal/Reflex behavior. Cross-actor requests are handled
    /// by that actor's native Brain; the target reference may be cross-model.
    ChangeActorBehavior {
        actor: ActorTarget,
        target: BehaviorRef,
    },
    ChangeAction(Expression),
    /// Proc_Turn (214) and Proc_Turn2 (215) share 00542bb0: 214 takes degrees
    /// per second; 215 takes degrees per frame, divided by the clamped frame
    /// time (DAT_00754af8, 16 to 80 ms) its dynamics then integrates over.
    Turn {
        degrees_per_second: Expression,
        per_frame: bool,
    },
    /// Proc_TurnPerso (217) requests the absolute horizontal heading toward
    /// an actor's origin. This is an immediate mechanics request, not Meta23.
    TurnToward(Expression),
    /// Proc_TransparentDisplay/NotTransparentDisplay/SetTransparency (185 to
    /// 187, 00543130) under Operator_Ultra apply to the receiver.
    ActorSetTransparentDisplay {
        actor: ActorTarget,
        enabled: bool,
    },
    ActorSetTransparency {
        actor: ActorTarget,
        value: Expression,
    },
    /// Proc_ChangeAction (218, 00542e60) under Operator_Ultra: the receiver's
    /// action request; a refusal sets procedure control 2, as for self.
    ActorChangeAction {
        actor: ActorTarget,
        action: Expression,
    },
    /// Proc_TurnPerso under Operator_Ultra: the receiver turns toward the target.
    ActorTurnToward {
        actor: ActorTarget,
        target: Expression,
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
    /// The way operand selects one of the actor's own routes.
    ResetRoute(RouteSlot),
    /// Proc_ReInitWayBack (221): progress at the last point, ready to walk
    /// back (005187c0; notes/research/way-procedures.md).
    ResetRouteBack(RouteSlot),
    /// Proc_InitWayWithWp (222): progress at the first point matching the
    /// waypoint, if any (00518840).
    InitRouteAtWaypoint {
        slot: RouteSlot,
        waypoint: Expression,
    },
    GoOverRoute(RouteSlot),
    /// Meta30 visits the next selected waypoint in one actor-local route slot.
    /// Unlike GoOverRoute, it completes after that point's progress update.
    GoToNextWaypoint(RouteSlot),
    /// Meta32/33 capture a point and radius once. The horizontal variant
    /// refreshes the point's height from the actor before each native update.
    GoToPosition {
        point: Expression,
        radius: Expression,
        horizontal: bool,
    },
    /// Meta25 Action_GoToWP: its update 0054b110 recomputes the waypoint's
    /// location (WP_fnv_WayPoint_ComputeLocation) and reads its radius on every
    /// update, then moves as GoToPosition does (0054a960).
    GoToWaypoint { waypoint: Expression },
    /// Meta2 captures an actor and radius once, then follows that actor's
    /// current origin through the same contact movement as GoToPosition.
    PursueActor {
        actor: Expression,
        radius: Expression,
    },
    /// Proc12 places an actor through its contact reset, then activates it.
    /// This is distinct from an absolute-position assignment.
    ActivateActorAtPosition {
        actor: Expression,
        point: Expression,
    },
    /// Meta49 captures a camera actor and waypoint reference. Each update
    /// supplies the remaining eye displacement until within the waypoint's
    /// radius; the camera's ordinary position API consumes that displacement.
    CameraGoToWaypoint {
        camera: Expression,
        waypoint: Expression,
    },
    /// Capture two absolute points and the source lift's duration parameter.
    /// Native lift movement owns the trajectory and completion state.
    MoveLift {
        from: Expression,
        to: Expression,
        duration_seconds: Expression,
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
    /// Proc307 queues converted vignette artwork, then requests the main menu.
    /// Only a strictly positive signed32 millisecond operand has an effect.
    DisplayVignetteAndReturnToMainMenu {
        name: String,
        milliseconds: Expression,
        /// Resolved by content import, including source language selection and
        /// CNT sharing. A standalone script may leave artwork unbound.
        image: Option<AssetRef<super::texture::Texture>>,
    },
    /// Proc_ActivateObject and Proc_DeactivateObject mutate the source object
    /// active bit on this referenced Perso. This state is separate from draw
    /// visibility and actor AI activation.
    SetObjectActive {
        actor: ActorTarget,
        active: bool,
    },
    /// Proc273 snapshots the receiver's custom flags before either operand.
    /// Bit index and truth use the evaluated raw words, without numeric casts.
    SetCustomBit {
        actor: ActorTarget,
        bit: Expression,
        value: Expression,
    },
    /// Proc268/269 resolves parent, channel, then child on the original caller.
    SetChannelParent {
        parent: Expression,
        channel: u32,
        child: Expression,
        attached: bool,
    },
    /// Proc141 selects a viewport camera after evaluating all four operands.
    SelectViewportCamera {
        viewport: Expression,
        previous_policy: Expression,
        placement_policy: Expression,
        actor: Expression,
    },
    /// Proc158 skips selector/value evaluation when the camera is unregistered.
    ChangeCameraFocal {
        actor: Expression,
        selector: Expression,
        value: Expression,
    },
    /// Proc153 (00547620) writes one camera angle. `field` 0/1 are alpha and
    /// teta, 2..4 the view orientation, 5/6 their shifts. `mode` -1 restores
    /// the initial value, 0 sets and 1 adds; 9 and 10 are those for this frame.
    ChangeCameraAngle {
        actor: Expression,
        field: Expression,
        mode: Expression,
        value: Expression,
    },
    /// Proc143 (00546510) writes the camera distance window. `mode` -3..6
    /// selects which bound changes; the result is clamped to 0 <= min <= max.
    ChangeCameraDistance {
        actor: Expression,
        mode: Expression,
        minimum: Expression,
        maximum: Expression,
    },
    /// Proc171 evaluates one actor and resets only its camera's vertical maximum.
    CameraResetVerticalMaximum {
        actor: Expression,
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
    /// Inv_UnselectSelectedObject (procedure 417).
    InventoryUnselectObject,
    Jump,
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
    /// Proc445 with an explicit Ultra receiver; direction evaluates on caller.
    ChangeActorSighting {
        actor: Expression,
        direction: Expression,
    },
    /// Proc415 updates translation and the native pose without activation or
    /// clearing velocity. The source economic placement API is distinct from 253.
    SetEconomicActorAbsolutePosition {
        actor: Expression,
        position: Expression,
    },
    /// Camera procedure 160 stores a keyword or referenced target actor.
    CameraChangeTargetedPerso {
        /// The camera actor: a referenced camera, or the calling actor (a
        /// camera model's Self keyword, e.g. fix/437).
        camera: ActorTarget,
        target: ActorTarget,
    },
    /// Source camera procedure 163 restores the component target to MainActor.
    CameraResetTargetedPerso {
        /// The camera actor: a referenced camera, or the calling actor (a
        /// camera model's Self keyword, e.g. fix/437).
        camera: ActorTarget,
    },
    /// Proc136 restores the camera's initial framing and local progress,
    /// preserving its selected target and session-owned controller state.
    CameraReset {
        /// The camera actor: a referenced camera, or the calling actor (a
        /// camera model's Self keyword, e.g. fix/437).
        camera: ActorTarget,
    },
    /// Proc166 evaluates camera, flag group, operation and selector in order.
    /// Operation -1 restores a bit from initial framing; 0 clears and 1 sets.
    CameraChangeFlag {
        /// The camera actor: a referenced camera, or the calling actor (a
        /// camera model's Self keyword, e.g. fix/437).
        camera: ActorTarget,
        group: Expression,
        operation: Expression,
        selector: Expression,
    },
    /// Source camera procedure 157 selects a motion mode; -1 restores the
    /// original preset's mode. Unsupported transitions fail at the target API.
    CameraChangeState {
        /// The camera actor: a referenced camera, or the calling actor (a
        /// camera model's Self keyword, e.g. fix/437).
        camera: ActorTarget,
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
    /// Hype Proc_ActivateChannel / Proc_DeactivateChannel request visibility
    /// by logical channel ID. The runtime narrows the source operand to a byte
    /// and retains deactivation even when the current animation has no module.
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
    /// Meta61 waits for the shared fade direction to become idle.
    WaitEndOfFade,
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
    /// Proc386 evaluates one raw scalar DWORD on the caller, then updates
    /// that caller's effects-volume override. Exact 127 removes the override.
    SoundSetMyEffectsVolume(Expression),
    /// Proc412 captures the Ultra/self receiver, evaluates target then duration
    /// on the caller, and fades effects before stopping that object's effects.
    SoundFadeObjectAndStop {
        actor: ActorTarget,
        target: Expression,
        duration_ms: Expression,
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
    /// Category-27 SectorRef is a named sector component in its owning scene.
    /// None retains a source null identity, rather than an unresolved binding.
    SectorReference(Option<AssetRef<super::scene::Scene>>),
    /// SCT_ActorInSector captures the first operand's current sector before
    /// evaluating the second operand on the original caller. No Ultra receiver.
    ActorInSector {
        actor: Box<Expression>,
        sector: Box<Expression>,
    },
    /// Category35 ObjectTableRef, including a null selection.
    Configuration(Option<AssetRef<ObjectList>>),
    Integer(i32),
    Real(f32),
    /// Source Func_Int truncates a real-valued expression to an integer.
    IntegerCast(Box<Expression>),
    /// Func_Square and Func_AbsoluteValue return a rounded source real.
    Square(Box<Expression>),
    AbsoluteValue(Box<Expression>),
    /// Func_GetVectorNorm and Func_MTH_NormalizeVector.
    VectorNorm(Box<Expression>),
    VectorNormalize(Box<Expression>),
    /// Source vector literal (category 16); components remain source-space.
    Vector(Box<[Expression; 3]>),
    /// Current actor's world position in source coordinates.
    SelfPosition,
    /// Ultra Func0 reads the resolved receiver's current position.
    ActorPosition(ActorTarget),
    /// Func164 returns the executing/Ultra actor's world sight direction.
    ActorSighting(ActorTarget),
    /// Cond107 tests the receiver's one-based, wrapping custom bit index.
    HasCustomBit {
        actor: ActorTarget,
        bit: Box<Expression>,
    },
    /// Source Field.Position reads the current receiver's world translation.
    /// Keep this distinct from Func_GetPersoAbsolutePosition for source fidelity.
    SelfPositionField,
    /// A category-29 source operand used as a typed Waypoint value (for
    /// example, a saved designer-array assignment), distinct from its position.
    WaypointReference(AssetRef<Waypoint>),
    /// Func_GetWPAbsolutePosition resolves a meaningful shared waypoint asset.
    WaypointPosition(AssetRef<Waypoint>),
    /// Func_GetWPAbsolutePosition of a waypoint value, such as an array element.
    WaypointValuePosition(Box<Expression>),
    /// Func_DistanceToWP evaluates a source-space waypoint distance on its
    /// receiver (self unless an enclosing Operator_Ultra selects another actor).
    DistanceToWaypoint {
        actor: ActorTarget,
        waypoint: Box<Expression>,
    },
    /// Function_Camera_Get* (category 3, native 0x0053d300): `getter` is the
    /// source id, 54 DistMin, 55 DistMax, 58 Alpha, 60 Teta or 70 State. A zero
    /// selector reads the initial parameters, nonzero the current ones.
    CameraValue {
        getter: u32,
        selector: Box<Expression>,
        receiver: ActorTarget,
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
    /// Func_GetNormalCollideVector (153, 0053b990: report+0x30, consumes but
    /// ignores an Ultra receiver) and Func_GetNormalGroundVector (174,
    /// 0053ba40: report+0xa0) read a normal of the caller's dynamics
    /// collision report (0052ec80); zero without one.
    CollideNormal { ground: bool },
    /// Cond_CollideMovingZDM (1-30, 00533c90 -> 00430930): the caller's
    /// 1-based ZDM zone, moved to the actor matrix with translation M*offset,
    /// collides with its surroundings; also true when the zone is missing.
    /// ReduceIndexOfVectorArray (3-220, 00536e60): the first index i < count
    /// whose array point plus offset is visible from origin plus offset
    /// (segment tests against the actor's sector and neighbours, 00536da0),
    /// else count. The array operand is a raw designer-variable id byte.
    FirstVisibleArrayPoint {
        origin: Box<Expression>,
        array: u16,
        offset: Box<Expression>,
        count: Box<Expression>,
    },
    CollideMovingZdm {
        zone: Box<Expression>,
        offset: Box<Expression>,
    },
    /// Func_GetGameState returns the shared session selector (0..8).
    /// Unrecognized combined masks leave the source result stale; targets must
    /// diagnose that unrepresented case rather than return the raw mask.
    GameState,
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
    /// Cond123 evaluates its actor operand, then tests completion and the
    /// actor's current/requested native action target identity (00436000).
    /// Cond123. The loaded Manor and Fix scripts carry no operand: all 30 nodes
    /// observed in live memory on 2026-11-01 are leaves, meaning the executing
    /// actor. `Some` keeps the evaluated form for scripts that do supply one.
    ActionFinished(Option<Box<Expression>>),
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
    /// Cond_CollideWithPerso: the actor's last move touched another actor.
    CollideWithPerso,
    /// Func_CloserWP: the nearest waypoint of an actor-local route slot.
    CloserRouteWaypoint(u8),
    /// Cond_IsValidWay (1-68, 005344a0): the way's point count, or zero when
    /// the actor has no such way.
    RouteWaypointCount(RouteSlot),
    /// Cond_CollideWithWall: the actor's last move touched a wall.
    CollideWithWall,
    /// Cond_IsValidWayPoint: the operand names a waypoint.
    IsValidWaypoint(Box<Expression>),
    /// Cond28 tests actual contact against all specified low16 material bits.
    /// The source output limit is independently one, not the third operand.
    CollidePersoZdeNoWithTypeZde {
        zone: u32,
        material_mask: u32,
    },
    /// Cond24..27 (005338a0 -> 0042f980): some ZDE or physical collide set of
    /// `first` holding a `first_mask` material intersects an element of
    /// `second` whose material carries every `second_mask` bit. Opcodes 25..27
    /// omit one or both masks; an omitted mask is zero.
    CollidePersoTypeZdeWithPersoTypeZde {
        opcode: u32,
        first: Box<Expression>,
        first_mask: u32,
        second: Box<Expression>,
        second_mask: u32,
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
    /// Func13 measures origin-to-position distance in source world axes.
    /// None uses the executing actor; Some retains an explicit Ultra receiver.
    DistanceToPosition {
        receiver: Option<ActorTarget>,
        point: Box<Expression>,
    },
    /// Signed source-Z position of the target in the receiver's local frame.
    ActorRelativeZ {
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
    ActorLocalDistance {
        receiver: ActorTarget,
        target: Box<Expression>,
        vertical: bool,
    },
    IsAbovePerso {
        receiver: ActorTarget,
        point: Box<Expression>,
        threshold: Box<Expression>,
    },
    ActorPositionField(ActorTarget),
    /// Operator_Dot selecting the source Perso sighting field.
    ActorSightingField(ActorTarget),
    ActorHitPoints(Box<Expression>),
    /// Func37 evaluates/captures its actor before its damage operand, then
    /// subtracts low-byte damage with a zero floor and returns remaining life.
    /// Fight_RemoveAndReadHitPointsEnemy: Func37 and an enemy bar refresh.
    FightSubtractAndGetHitPoints(Box<Expression>),
    /// Cond_SeePositionWithOffset: from the sight point plus offset.
    SeePositionWithOffset {
        target: Box<Expression>,
        offset: Box<Expression>,
    },
    /// Cond_SeePersoWithOffset: offsets are in each actor's own axes.
    SeePersoWithOffset {
        offset: Box<Expression>,
        target: Box<Expression>,
        target_offset: Box<Expression>,
    },
    SubtractAndGetHitPoints {
        actor: Box<Expression>,
        value: Box<Expression>,
    },
    InventoryGold,
    InventoryMaximum(Box<Expression>),
    Sine(Box<Expression>),
    Cosine(Box<Expression>),
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
    /// Selected Object identity; catalogue quantity does not imply ownership.
    InventoryObject,
    /// Select an admitted Object using the source evaluator's raw word payload.
    InventorySelectObject(Box<Expression>),
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
    /// Func171 returns source degrees, signed by the selected cross component.
    VectorAngle {
        first: Box<Expression>,
        second: Box<Expression>,
        sign_axis: Box<Expression>,
    },
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
    /// Hype Operator_VectorMinusVector (category 2, operation 17).
    VectorMinus(Box<Expression>, Box<Expression>),
    /// Hype Operator_VectorTimesReal (category 2, operation 18): a vector scaled
    /// by a real. Operation inferred from the order of 16/17 and from Gogoud's
    /// key placement (Hype's sighting scaled by 0.1); not yet checked in the original.
    VectorScale(Box<Expression>, Box<Expression>),
    Minus(Box<Expression>, Box<Expression>),
    /// MTH_Modulus (category 3, operation 189).
    Modulus(Box<Expression>, Box<Expression>),
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
    /// Actor-valued designer property on the original script owner.
    Variable(u16),
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
    /// DSG type 9: one byte indexing the owner's own way list, the same index
    /// space as a category-23 way operand (005236a0 size 1, reader 00523860;
    /// notes/research/way-designer-operand.md).
    Way(u8),
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
    /// DSG type 13: a behavior (comport) pointer, value kind 0x23 (descriptor
    /// table 0x005fdcc0), resolved to the behavior it addresses.
    Behavior(Option<BehaviorRef>),
    /// Preserved bytes for a source designer field whose value semantics are
    /// not understood. The payload is never traversed as pointers or executed.
    Opaque {
        kind: u32,
        bytes: Vec<u8>,
    },
}
/// A way (`Y`) operand: a category-23 literal or a designer variable of type
/// Way. 00542ff0, 0054b260 and 0054b200 read either as the same one-byte index
/// into the actor's way list (notes/research/way-designer-operand.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteSlot {
    Slot(u8),
    Variable(u16),
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
        for expression in self.expressions() {
            let receiver = match expression {
                Expression::Actor(ActorTarget::Variable(index))
                | Expression::ActorPosition(ActorTarget::Variable(index))
                | Expression::ActorSighting(ActorTarget::Variable(index))
                | Expression::HasCustomBit {
                    actor: ActorTarget::Variable(index),
                    ..
                }
                | Expression::ActorPositionField(ActorTarget::Variable(index))
                | Expression::ActorSightingField(ActorTarget::Variable(index))
                | Expression::DistanceToWaypoint {
                    actor: ActorTarget::Variable(index),
                    ..
                }
                | Expression::ActorDistanceFrom {
                    receiver: ActorTarget::Variable(index),
                    ..
                }
                | Expression::ActorRelativeZ {
                    receiver: ActorTarget::Variable(index),
                    ..
                }
                | Expression::ActorLocalDistance {
                    receiver: ActorTarget::Variable(index),
                    ..
                }
                | Expression::ActorFacingAngle {
                    receiver: ActorTarget::Variable(index),
                    ..
                }
                | Expression::DistanceToPosition {
                    receiver: Some(ActorTarget::Variable(index)),
                    ..
                }
                | Expression::IsAbovePerso {
                    receiver: ActorTarget::Variable(index),
                    ..
                } => Some(*index),
                _ => None,
            };
            if let Some(index) = receiver {
                ensure!(
                    matches!(
                        self.variables[usize::from(index)].default,
                        DesignerValue::Actor(_) | DesignerValue::SourceActor(_)
                    ),
                    "actor receiver requires an actor-valued designer field"
                );
            }
        }
        for script in self.scripts() {
            for (node, _) in script.walk() {
                if let AstNode::Statement(Statement::SoundFadeObjectAndStop {
                    actor: ActorTarget::Variable(index),
                    ..
                }) = node
                {
                    ensure!(
                        matches!(
                            self.variables[usize::from(*index)].default,
                            DesignerValue::Actor(_) | DesignerValue::SourceActor(_)
                        ),
                        "object fade receiver requires an actor-valued designer field"
                    );
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
                | Expression::WaypointReference(reference) => Some(reference),
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
    pub fn sector_references(&self) -> Vec<&AssetRef<super::scene::Scene>> {
        self.expressions()
            .into_iter()
            .filter_map(|expression| match expression {
                Expression::SectorReference(Some(reference)) => Some(reference),
                _ => None,
            })
            .collect()
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
    pub fn vignette_images(&self) -> Vec<&AssetRef<super::texture::Texture>> {
        self.scripts()
            .into_iter()
            .flat_map(Script::walk)
            .filter_map(|(node, _)| match node {
                AstNode::Statement(Statement::DisplayVignetteAndReturnToMainMenu {
                    image: Some(image),
                    ..
                }) => Some(image),
                _ => None,
            })
            .collect()
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
                    if let ActorTarget::Reference(actor) = actor {
                        ensure!(actor.path.member().is_none(), "distance actor is a member");
                    }
                    if let ActorTarget::Variable(index) = actor {
                        ensure!(
                            usize::from(*index) < variables,
                            "distance actor variable out of range"
                        );
                    }
                    expression(waypoint, variables, depth + 1, count)?;
                }
                Expression::ActorLocalDistance {
                    receiver, target, ..
                }
                | Expression::ActorDistanceFrom { receiver, target }
                | Expression::ActorRelativeZ { receiver, target }
                | Expression::ActorFacingAngle { receiver, target } => {
                    if let ActorTarget::Reference(actor) = receiver {
                        ensure!(
                            actor.path.member().is_none(),
                            "distance receiver is a member"
                        );
                    }
                    if let ActorTarget::Variable(variable) = receiver {
                        ensure!(
                            usize::from(*variable) < variables,
                            "AI receiver variable out of range"
                        );
                    }
                    expression(target, variables, depth + 1, count)?;
                }
                Expression::DistanceToPosition { receiver, point } => {
                    if let Some(ActorTarget::Reference(actor)) = receiver {
                        ensure!(
                            actor.path.member().is_none(),
                            "distance receiver is a member"
                        );
                    }
                    if let Some(ActorTarget::Variable(variable)) = receiver {
                        ensure!(
                            usize::from(*variable) < variables,
                            "AI receiver variable out of range"
                        );
                    }
                    expression(point, variables, depth + 1, count)?;
                }
                Expression::IsAbovePerso {
                    receiver,
                    point,
                    threshold,
                } => {
                    if let ActorTarget::Variable(variable) = receiver {
                        ensure!(
                            usize::from(*variable) < variables,
                            "AI receiver variable out of range"
                        );
                    }
                    if let ActorTarget::Reference(actor) = receiver {
                        ensure!(
                            actor.path.member().is_none(),
                            "spatial receiver is a member"
                        );
                    }
                    expression(point, variables, depth + 1, count)?;
                    expression(threshold, variables, depth + 1, count)?;
                }
                Expression::ActorPosition(ActorTarget::Variable(variable))
                | Expression::ActorSighting(ActorTarget::Variable(variable))
                | Expression::ActorPositionField(ActorTarget::Variable(variable))
                | Expression::ActorSightingField(ActorTarget::Variable(variable))
                | Expression::Actor(ActorTarget::Variable(variable)) => ensure!(
                    usize::from(*variable) < variables,
                    "AI actor variable out of range"
                ),
                Expression::ActorPositionField(ActorTarget::Reference(actor))
                | Expression::ActorSightingField(ActorTarget::Reference(actor))
                | Expression::ActorPosition(ActorTarget::Reference(actor)) => {
                    ensure!(actor.path.member().is_none(), "position actor is a member");
                }
                Expression::ActorSighting(ActorTarget::Reference(actor)) => {
                    ensure!(actor.path.member().is_none(), "sighting actor is a member");
                }
                Expression::HasCustomBit { actor, bit } => {
                    if let ActorTarget::Variable(variable) = actor {
                        ensure!(
                            usize::from(*variable) < variables,
                            "custom-bit actor variable out of range"
                        );
                    }
                    if let ActorTarget::Reference(actor) = actor {
                        ensure!(
                            actor.path.member().is_none(),
                            "custom-bit actor is a member"
                        );
                    }
                    expression(bit, variables, depth + 1, count)?;
                }
                Expression::CameraShiftAlpha { selector, receiver }
                | Expression::CameraValue {
                    selector, receiver, ..
                }
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
                Expression::SectorReference(Some(reference)) => {
                    ensure!(
                        reference.path.member().is_some(),
                        "sector reference lacks a named scene member"
                    );
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
                | Expression::VectorMinus(left, right)
                | Expression::VectorScale(left, right)
                | Expression::SeePositionWithOffset {
                    target: left,
                    offset: right,
                }
                | Expression::Minus(left, right)
                | Expression::Modulus(left, right)
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
                | Expression::SubtractAndGetHitPoints {
                    actor: left,
                    value: right,
                }
                | Expression::VectorCos(left, right)
                | Expression::WaypointSphereContains {
                    actor: left,
                    waypoint: right,
                }
                | Expression::CollidePersoTypeZdeWithPersoTypeZde {
                    first: left,
                    second: right,
                    ..
                }
                | Expression::ActorInSector {
                    actor: left,
                    sector: right,
                } => {
                    expression(left, variables, depth + 1, count)?;
                    expression(right, variables, depth + 1, count)?;
                }
                Expression::SeePersoWithOffset {
                    offset: first,
                    target: second,
                    target_offset: sign_axis,
                }
                | Expression::VectorAngle {
                    first,
                    second,
                    sign_axis,
                } => {
                    expression(first, variables, depth + 1, count)?;
                    expression(second, variables, depth + 1, count)?;
                    expression(sign_axis, variables, depth + 1, count)?;
                }
                Expression::Not(value)
                | Expression::Negate(value)
                | Expression::IsInAction(value)
                | Expression::ActionFinished(Some(value))
                | Expression::WaypointValuePosition(value)
                | Expression::Square(value)
                | Expression::AbsoluteValue(value)
                | Expression::IsValidWaypoint(value)
                | Expression::FightSubtractAndGetHitPoints(value)
                | Expression::VectorNorm(value)
                | Expression::VectorNormalize(value)
                | Expression::ActorDistance(value)
                | Expression::ActorHitPointsMaximum(value)
                | Expression::ActorHitPoints(value)
                | Expression::InventoryMaximum(value)
                | Expression::InventorySelectObject(value)
                | Expression::Sine(value)
                | Expression::Cosine(value)
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
                    | Statement::AddAssign { variable, value }
                    | Statement::MultiplyAssign { variable, value } => {
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
                    Statement::DisplayVignetteAndReturnToMainMenu {
                        name,
                        milliseconds,
                        image,
                    } => {
                        ensure!(
                            !name.is_empty() && !name.contains('\0'),
                            "invalid vignette identity"
                        );
                        if let Some(image) = image {
                            ensure!(image.path.member().is_none(), "vignette image is a member");
                            ensure!(
                                image.path.role() == super::PackageRole::Texture,
                                "vignette image is not a texture"
                            );
                        }
                        expression(milliseconds, variables, depth + 1, count)?;
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
                    | Statement::FightSubHitPointsEnemy(index)
                    | Statement::SetMagicPointsMaximum(index)
                    | Statement::SelectArmor(index)
                    | Statement::SetArmorResistance(index) => {
                        expression(index, variables, depth + 1, count)?
                    }
                    Statement::SetHitPointsMaximum { actor, value }
                    | Statement::SetHitPoints { actor, value }
                    | Statement::AddHitPoints { actor, value }
                    | Statement::SubtractHitPoints { actor, value }
                    | Statement::SetInventoryMaximum { item: actor, value } => {
                        expression(actor, variables, depth + 1, count)?;
                        expression(value, variables, depth + 1, count)?;
                    }
                    Statement::ActivateMapName { name, active } => {
                        expression(name, variables, depth + 1, count)?;
                        expression(active, variables, depth + 1, count)?;
                    }
                    Statement::ChangeActorSighting { actor, direction } => {
                        expression(actor, variables, depth + 1, count)?;
                        expression(direction, variables, depth + 1, count)?;
                    }
                    Statement::SetEconomicActorAbsolutePosition { actor, position } => {
                        expression(actor, variables, depth + 1, count)?;
                        expression(position, variables, depth + 1, count)?;
                    }
                    Statement::WaitFrames(frames) => {
                        expression(frames, variables, depth + 1, count)?
                    }
                    Statement::WaitBoolean { field, expected } => {
                        expression(field, variables, depth + 1, count)?;
                        expression(expected, variables, depth + 1, count)?;
                    }
                    Statement::SetStockCount {
                        actor,
                        count: value,
                    } => {
                        expression(actor, variables, depth + 1, count)?;
                        expression(value, variables, depth + 1, count)?;
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
                        for value in [item, milliseconds, from, to, x, y, width, height] {
                            expression(value, variables, depth + 1, count)?;
                        }
                    }
                    Statement::WaitInteger {
                        left,
                        selector,
                        right,
                    } => {
                        expression(left, variables, depth + 1, count)?;
                        expression(selector, variables, depth + 1, count)?;
                        expression(right, variables, depth + 1, count)?;
                    }
                    Statement::Turn { degrees_per_second, .. }
                    | Statement::TurnToward(degrees_per_second)
                    | Statement::ActorTurnToward {
                        target: degrees_per_second,
                        ..
                    } => expression(degrees_per_second, variables, depth + 1, count)?,
                    Statement::IncrementalTurnToward { target, frames } => {
                        if let ActorTarget::Reference(actor) = target {
                            ensure!(actor.path.member().is_none(), "turn target is a member");
                        }
                        if let ActorTarget::Variable(variable) = target {
                            ensure!(
                                usize::from(*variable) < variables,
                                "turn target variable out of range"
                            );
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
                    Statement::PursueActor { actor, radius } => {
                        expression(actor, variables, depth + 1, count)?;
                        expression(radius, variables, depth + 1, count)?;
                    }
                    Statement::GoToWaypoint { waypoint } => {
                        expression(waypoint, variables, depth + 1, count)?;
                    }
                    Statement::ActivateActorAtPosition { actor, point } => {
                        expression(actor, variables, depth + 1, count)?;
                        expression(point, variables, depth + 1, count)?;
                    }
                    Statement::CameraGoToWaypoint { camera, waypoint } => {
                        expression(camera, variables, depth + 1, count)?;
                        expression(waypoint, variables, depth + 1, count)?;
                    }
                    Statement::MoveLift {
                        from,
                        to,
                        duration_seconds,
                    } => {
                        for value in [from, to, duration_seconds] {
                            expression(value, variables, depth + 1, count)?;
                        }
                    }
                    Statement::ChangeActorBehavior { actor, .. } => {
                        if let ActorTarget::Reference(actor) = actor {
                            ensure!(actor.path.member().is_none(), "behavior actor is a member");
                        }
                    }
                    Statement::CameraChangeState { camera, state } => {
                        if let ActorTarget::Reference(camera) = camera {
                            ensure!(camera.path.member().is_none(), "camera is a member");
                        }
                        expression(state, variables, depth + 1, count)?;
                    }
                    Statement::CameraChangeFlag {
                        camera,
                        group,
                        operation,
                        selector,
                    } => {
                        if let ActorTarget::Reference(camera) = camera {
                            ensure!(camera.path.member().is_none(), "camera is a member");
                        }
                        expression(group, variables, depth + 1, count)?;
                        expression(operation, variables, depth + 1, count)?;
                        expression(selector, variables, depth + 1, count)?;
                    }
                    Statement::CameraChangeTargetedPerso { camera, target } => {
                        if let ActorTarget::Reference(camera) = camera {
                            ensure!(camera.path.member().is_none(), "camera is a member");
                        }
                        if let ActorTarget::Reference(actor) = target {
                            ensure!(actor.path.member().is_none(), "camera target is a member");
                        }
                    }
                    Statement::CameraResetTargetedPerso { camera }
                    | Statement::CameraReset { camera } => {
                        if let ActorTarget::Reference(camera) = camera {
                            ensure!(camera.path.member().is_none(), "camera is a member");
                        }
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
                    Statement::SetCustomBit { actor, bit, value } => {
                        if let ActorTarget::Variable(variable) = actor {
                            ensure!(
                                usize::from(*variable) < variables,
                                "custom-bit actor variable out of range"
                            );
                        }
                        if let ActorTarget::Reference(actor) = actor {
                            ensure!(
                                actor.path.member().is_none(),
                                "custom-bit actor is a member"
                            );
                        }
                        expression(bit, variables, depth + 1, count)?;
                        expression(value, variables, depth + 1, count)?;
                    }
                    Statement::SetChannelParent {
                        parent,
                        channel,
                        child,
                        ..
                    } => {
                        ensure!(
                            *channel <= u32::from(u16::MAX),
                            "attachment channel exceeds source width"
                        );
                        expression(parent, variables, depth + 1, count)?;
                        expression(child, variables, depth + 1, count)?;
                    }
                    Statement::SelectViewportCamera {
                        viewport,
                        previous_policy,
                        placement_policy,
                        actor,
                    } => {
                        for value in [viewport, previous_policy, placement_policy, actor] {
                            expression(value, variables, depth + 1, count)?;
                        }
                    }
                    Statement::ChangeCameraFocal {
                        actor,
                        selector,
                        value,
                    } => {
                        for value in [actor, selector, value] {
                            expression(value, variables, depth + 1, count)?;
                        }
                    }
                    Statement::ChangeCameraAngle {
                        actor,
                        field,
                        mode,
                        value,
                    } => {
                        for value in [actor, field, mode, value] {
                            expression(value, variables, depth + 1, count)?;
                        }
                    }
                    Statement::ChangeCameraDistance {
                        actor,
                        mode,
                        minimum,
                        maximum,
                    } => {
                        for value in [actor, mode, minimum, maximum] {
                            expression(value, variables, depth + 1, count)?;
                        }
                    }
                    Statement::CameraResetVerticalMaximum { actor } => {
                        expression(actor, variables, depth + 1, count)?;
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
                    }
                    | Statement::SoundFadeObjectAndStop {
                        actor,
                        target,
                        duration_ms,
                    } => {
                        if let ActorTarget::Reference(actor) = actor {
                            ensure!(
                                actor.path.member().is_none(),
                                "sound fade receiver is a member"
                            );
                        }
                        if let ActorTarget::Variable(variable) = actor {
                            ensure!(
                                usize::from(*variable) < variables,
                                "sound receiver variable out of range"
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
                    Statement::SoundSetMyEffectsVolume(value) => {
                        expression(value, variables, depth + 1, count)?;
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
