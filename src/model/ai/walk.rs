//! One exhaustive ordered traversal for inspection, dependency collection, and validation.
use super::{Expression, Script, Statement};

#[derive(Clone, Copy)]
pub enum AstNode<'a> {
    Statement(&'a Statement),
    Expression(&'a Expression),
}

pub struct AstWalk<'a> {
    pending: Vec<(AstNode<'a>, usize)>,
}

impl Script {
    /// Preorder traversal in source evaluation order with structural depth.
    /// The explicit stack lets validation reject deep trees before recursive
    /// codec/compiler paths inspect them.
    pub fn walk(&self) -> AstWalk<'_> {
        AstWalk {
            pending: self
                .statements
                .iter()
                .rev()
                .map(|statement| (AstNode::Statement(statement), 0))
                .collect(),
        }
    }
}

impl<'a> AstWalk<'a> {
    fn expressions(&mut self, depth: usize, values: &[&'a Expression]) {
        self.pending.extend(
            values
                .iter()
                .rev()
                .map(|value| (AstNode::Expression(value), depth)),
        );
    }
}

impl<'a> Iterator for AstWalk<'a> {
    type Item = (AstNode<'a>, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let (node, depth) = self.pending.pop()?;
        let child_depth = depth + 1;
        match node {
            AstNode::Statement(statement) => match statement {
                Statement::If {
                    condition,
                    then,
                    otherwise,
                } => {
                    if let Some(otherwise) = otherwise {
                        self.pending.extend(
                            otherwise
                                .iter()
                                .rev()
                                .map(|statement| (AstNode::Statement(statement), child_depth)),
                        );
                    }
                    self.pending.extend(
                        then.iter()
                            .rev()
                            .map(|statement| (AstNode::Statement(statement), child_depth)),
                    );
                    self.pending
                        .push((AstNode::Expression(condition), child_depth));
                }
                Statement::Evaluate(value)
                | Statement::FightSubHitPointsEnemy(value)
                | Statement::SoundSetMyEffectsVolume(value)
                | Statement::SetMagicPointsMaximum(value)
                | Statement::Assign { value, .. }
                | Statement::ActorFieldAssign { value, .. }
                | Statement::MainActorFieldAssign { value, .. }
                | Statement::SubtractAssign { value, .. }
                | Statement::AddAssign { value, .. }
                | Statement::MultiplyAssign { value, .. }
                | Statement::VectorComponentAssign { value, .. }
                | Statement::Wait {
                    milliseconds: value,
                }
                | Statement::WaitFrames(value)
                | Statement::ChangeAction(value)
                | Statement::ExecuteAction(value)
                | Statement::Turn {
                    degrees_per_second: value,
                    ..
                }
                | Statement::TurnToward(value)
                | Statement::ActorTurnToward { target: value, .. }
                | Statement::ActorSetTransparency { value, .. }
                | Statement::ActorChangeAction { action: value, .. }
                | Statement::IncrementalTurnToward { frames: value, .. }
                | Statement::SetVectorSpeed(value)
                | Statement::AddWind(value)
                | Statement::InitRouteAtWaypoint {
                    waypoint: value, ..
                }
                | Statement::SetImpulse(value)
                | Statement::SetLaserPointerSniperMode(value)
                | Statement::SetLaserPointerFlyMode(value)
                | Statement::SetTransparency(value)
                | Statement::SetGameState(value)
                | Statement::SetFadeTexture(value)
                | Statement::SetFadeValue(value)
                | Statement::SelectArmor(value)
                | Statement::SetArmorResistance(value)
                | Statement::ChangeSighting(value)
                | Statement::ChangeConfiguration(value)
                | Statement::CameraChangeState { state: value, .. }
                | Statement::SetActorAbsolutePosition {
                    position: value, ..
                }
                | Statement::SetKeywordAbsolutePosition {
                    position: value, ..
                }
                | Statement::SetSelfAbsolutePosition(value)
                | Statement::RunDialog { mode: value, .. }
                | Statement::DisplayVignetteAndReturnToMainMenu {
                    milliseconds: value,
                    ..
                }
                | Statement::ScrollDialog(value)
                | Statement::SetHudVisible { visible: value, .. }
                | Statement::SetVisualGMTDiffuseCoef { value, .. } => {
                    self.expressions(child_depth, &[value]);
                }
                Statement::ArrayAssign {
                    array,
                    index,
                    value,
                } => self.expressions(child_depth, &[array, index, value]),
                Statement::SwapConfiguredParts { first, second } => {
                    self.expressions(child_depth, &[first, second]);
                }
                Statement::IncrementalTurnPosition { point, frames } => {
                    self.expressions(child_depth, &[point, frames]);
                }
                Statement::PursueActor { actor, radius } => {
                    self.expressions(child_depth, &[actor, radius]);
                }
                Statement::GoToWaypoint { waypoint } => {
                    self.expressions(child_depth, &[waypoint]);
                }
                Statement::ActivateActorAtPosition { actor, point } => {
                    self.expressions(child_depth, &[actor, point]);
                }
                Statement::GoToPosition { point, radius, .. } => {
                    self.expressions(child_depth, &[point, radius]);
                }
                Statement::CameraGoToWaypoint { camera, waypoint } => {
                    self.expressions(child_depth, &[camera, waypoint]);
                }
                Statement::MoveLift {
                    from,
                    to,
                    duration_seconds,
                } => self.expressions(child_depth, &[from, to, duration_seconds]),
                Statement::SetHitPointsMaximum { actor, value }
                | Statement::SetInventoryMaximum { item: actor, value }
                | Statement::SetHitPoints { actor, value }
                | Statement::AddHitPoints { actor, value }
                | Statement::SubtractHitPoints { actor, value } => {
                    self.expressions(child_depth, &[actor, value])
                }
                Statement::ChangeActorSighting { actor, direction } => {
                    self.expressions(child_depth, &[actor, direction]);
                }
                Statement::SetEconomicActorAbsolutePosition { actor, position } => {
                    self.expressions(child_depth, &[actor, position]);
                }
                Statement::WaitBoolean {
                    field, expected, ..
                } => self.expressions(child_depth, &[field, expected]),
                Statement::SetStockCount { actor, count } => {
                    self.expressions(child_depth, &[actor, count])
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
                } => self.expressions(
                    child_depth,
                    &[item, milliseconds, from, to, x, y, width, height],
                ),
                Statement::PositionDialog { x, y } => self.expressions(child_depth, &[x, y]),
                Statement::ActivateMapName { name, active } => {
                    self.expressions(child_depth, &[name, active])
                }
                Statement::ExecuteActionAndGenerateObject { action, vector, .. } => {
                    self.expressions(child_depth, &[action, vector]);
                }
                Statement::AdjustPersoOnObject {
                    target,
                    near_threshold,
                    far_threshold,
                    far_action,
                    near_action,
                } => self.expressions(
                    child_depth,
                    &[
                        target,
                        near_threshold,
                        far_threshold,
                        far_action,
                        near_action,
                    ],
                ),
                Statement::SoundFadeMusic { arg0, arg1 } => {
                    self.expressions(child_depth, &[arg0, arg1]);
                }
                Statement::SetCustomBit { bit, value, .. } => {
                    self.expressions(child_depth, &[bit, value]);
                }
                Statement::SetChannelParent { parent, child, .. } => {
                    self.expressions(child_depth, &[parent, child]);
                }
                Statement::SelectViewportCamera {
                    viewport,
                    previous_policy,
                    placement_policy,
                    actor,
                } => {
                    self.expressions(
                        child_depth,
                        &[viewport, previous_policy, placement_policy, actor],
                    );
                }
                Statement::ChangeCameraFocal {
                    actor,
                    selector,
                    value,
                } => {
                    self.expressions(child_depth, &[actor, selector, value]);
                }
                Statement::ChangeCameraAngle {
                    actor,
                    field,
                    mode,
                    value,
                } => {
                    self.expressions(child_depth, &[actor, field, mode, value]);
                }
                Statement::ChangeCameraDistance {
                    actor,
                    mode,
                    minimum,
                    maximum,
                } => {
                    self.expressions(child_depth, &[actor, mode, minimum, maximum]);
                }
                Statement::CameraResetVerticalMaximum { actor } => {
                    self.expressions(child_depth, &[actor]);
                }
                Statement::SwitchMicrophone {
                    actor,
                    evaluated_second,
                } => {
                    if let Some(second) = evaluated_second {
                        self.expressions(child_depth, &[actor, second]);
                    } else {
                        self.expressions(child_depth, &[actor]);
                    }
                }
                Statement::SoundFadeVoice {
                    target,
                    duration_ms,
                }
                | Statement::SoundFadeSound {
                    target,
                    duration_ms,
                }
                | Statement::SoundFadeInObject {
                    target,
                    duration_ms,
                    ..
                }
                | Statement::SoundFadeObjectAndStop {
                    target,
                    duration_ms,
                    ..
                } => self.expressions(child_depth, &[target, duration_ms]),
                Statement::WaitInteger {
                    left,
                    selector,
                    right,
                } => {
                    self.expressions(child_depth, &[left, selector, right]);
                }
                Statement::ExecuteFade {
                    from,
                    to,
                    duration_seconds,
                } => self.expressions(child_depth, &[from, to, duration_seconds]),
                Statement::CameraChangeFlag {
                    group,
                    operation,
                    selector,
                    ..
                } => self.expressions(child_depth, &[group, operation, selector]),
                Statement::SoundFadeAll {
                    target,
                    duration_ms,
                } => self.expressions(child_depth, &[target, duration_ms]),
                Statement::Unsupported(_)
                | Statement::Increment { .. }
                | Statement::Decrement { .. }
                | Statement::ChangeBehavior { .. }
                | Statement::ChangeBehaviorVariable { .. }
                | Statement::ChangeBehaviorToPrevious { .. }
                | Statement::ChangeActorBehavior { .. }
                | Statement::ResetRoute(_)
                | Statement::ResetRouteBack(_)
                | Statement::GoOverRoute(_)
                | Statement::GoToNextWaypoint(_)
                | Statement::GoRelative
                | Statement::KillSelf
                | Statement::PlayerIsDead
                | Statement::HoldAfterPrevious
                | Statement::KillPerso { .. }
                | Statement::JumpAbsolute
                | Statement::InventoryUnselectObject
                | Statement::Jump
                | Statement::JumpWithoutAddingSpeed
                | Statement::SetParticleGeneratorEnabled(_)
                | Statement::SetParticleGenerator(_)
                | Statement::SetLaserPointerActive(_)
                | Statement::SetObjectActive { .. }
                | Statement::ActivateFade
                | Statement::DeactivateFade
                | Statement::SetDisplayVisible(_)
                | Statement::SetActorDisplayVisible { .. }
                | Statement::SetActorChannelActive { .. }
                | Statement::SetTransparentDisplay(_)
                | Statement::ActorSetTransparentDisplay { .. }
                | Statement::RepairArmor
                | Statement::CameraChangeTargetedPerso { .. }
                | Statement::CameraResetTargetedPerso { .. }
                | Statement::CameraReset { .. }
                | Statement::CameraUpdatePosition { .. }
                | Statement::ActorListAdd { .. }
                | Statement::SetChannelActive { .. }
                | Statement::ChangeMap { .. }
                | Statement::ChangeMapAtPosition { .. }
                | Statement::ToggleWideScreen
                | Statement::SelectDialog
                | Statement::EndDialog
                | Statement::ChangeDialogBehavior
                | Statement::WaitEndOfAction
                | Statement::WaitEndOfFade
                | Statement::WaitEndOfAnimation
                | Statement::Sound { .. }
                | Statement::SoundOnActor { .. }
                | Statement::MusicRequest { .. }
                | Statement::SoundKillMusic
                | Statement::Noop => {}
            },
            AstNode::Expression(expression) => match expression {
                Expression::InventoryAddObject(value)
                | Expression::MagicAddPoints(value)
                | Expression::IntegerCast(value)
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
                | Expression::IsInAction(value)
                | Expression::ActionFinished(Some(value))
                | Expression::WaypointValuePosition(value)
                | Expression::Not(value)
                | Expression::Negate(value)
                | Expression::InventoryQuantity(value)
                | Expression::MagicRemovePoints(value)
                | Expression::ActorListContains { actor: value, .. }
                | Expression::CameraValue {
                    selector: value, ..
                }
                | Expression::CameraShiftAlpha {
                    selector: value, ..
                }
                | Expression::CameraShiftTeta {
                    selector: value, ..
                }
                | Expression::VectorComponent { vector: value, .. } => {
                    self.expressions(child_depth, &[value]);
                }
                Expression::ActorDistanceFrom { target, .. }
                | Expression::HasCustomBit { bit: target, .. }
                | Expression::ActorRelativeZ { target, .. }
                | Expression::ActorFacingAngle { target, .. }
                | Expression::DistanceToPosition { point: target, .. }
                | Expression::ActorLocalDistance { target, .. } => {
                    self.expressions(child_depth, &[target]);
                }
                Expression::DistanceToWaypoint { waypoint, .. } => {
                    self.expressions(child_depth, &[waypoint]);
                }
                Expression::IsAbovePerso {
                    point, threshold, ..
                } => self.expressions(child_depth, &[point, threshold]),
                Expression::Vector(values) => {
                    self.expressions(child_depth, &[&values[0], &values[1], &values[2]]);
                }
                Expression::VectorAngle {
                    first,
                    second,
                    sign_axis,
                } => {
                    self.expressions(child_depth, &[first, second, sign_axis]);
                }
                Expression::SeePersoWithOffset {
                    offset,
                    target,
                    target_offset,
                } => {
                    self.expressions(child_depth, &[offset, target, target_offset]);
                }
                Expression::TimeElapsed {
                    start: left,
                    duration: right,
                }
                | Expression::ArrayElement {
                    array: left,
                    index: right,
                }
                | Expression::InventoryRemove {
                    item: left,
                    count: right,
                }
                | Expression::SubtractAndGetHitPoints {
                    actor: left,
                    value: right,
                }
                | Expression::RandomInt {
                    lower: left,
                    upper: right,
                }
                | Expression::Compare { left, right, .. }
                | Expression::VectorCos(left, right)
                | Expression::Multiply(left, right)
                | Expression::Plus(left, right)
                | Expression::VectorPlus(left, right)
                | Expression::CollideMovingZdm {
                    zone: left,
                    offset: right,
                }
                | Expression::FirstVisibleArrayPoint {
                    origin: left,
                    offset: right,
                    ..
                }
                | Expression::VectorMinus(left, right)
                | Expression::VectorScale(left, right)
                | Expression::SeePositionWithOffset {
                    target: left,
                    offset: right,
                }
                | Expression::Minus(left, right)
                | Expression::Modulus(left, right)
                | Expression::Divide(left, right)
                | Expression::And(left, right)
                | Expression::Or(left, right)
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
                } => self.expressions(child_depth, &[left, right]),
                Expression::Unsupported(_)
                | Expression::AbsentButton(_)
                | Expression::Nobody
                | Expression::Nowhere
                | Expression::Actor(_)
                | Expression::SectorReference(_)
                | Expression::Configuration(_)
                | Expression::Integer(_)
                | Expression::Real(_)
                | Expression::SelfPosition
                | Expression::ActorPosition(_)
                | Expression::ActorSighting(_)
                | Expression::ActorPositionField(_)
                | Expression::ActorSightingField(_)
                | Expression::InventoryGold
                | Expression::SelfPositionField
                | Expression::WaypointReference(_)
                | Expression::WaypointPosition(_)
                | Expression::GameTime
                | Expression::CollideNormal { .. }
                | Expression::GameState
                | Expression::SeePerso { .. }
                | Expression::Variable(_)
                | Expression::Action(_)
                | Expression::ActionFinished(None)
                | Expression::CurrentAnimationFrame
                | Expression::DialogIsOver
                | Expression::DialogIsOverAndChangeBehavior
                | Expression::CollideWithGround
                | Expression::CollideWithNothing
                | Expression::CollideWithWall
                | Expression::CollideWithPerso
                | Expression::CloserRouteWaypoint(_)
                | Expression::RouteWaypointCount(_)
                | Expression::CollidePersoZdeNoWithTypeZde { .. }
                | Expression::CollideWithSlope
                | Expression::IsTypeOfGmtCollide(_)
                | Expression::InTopOfJump
                | Expression::ActorFieldRead { .. }
                | Expression::SelfFieldRead(_)
                | Expression::BehaviorLiteral(_)
                | Expression::MainActorFieldRead(_)
                | Expression::IsInBehavior { .. }
                | Expression::Button { .. }
                | Expression::MainActorDistance
                | Expression::SelfHitPoints
                | Expression::ArmorResistance
                | Expression::ArmorCount
                | Expression::InventoryWeapon
                | Expression::InventoryObject
                | Expression::MagicCurrentSpell
                | Expression::MagicPoints
                | Expression::MagicPointsMaximum
                | Expression::LaserDirection
                | Expression::SelfSighting => {}
            },
        }
        Some((node, depth))
    }
}
