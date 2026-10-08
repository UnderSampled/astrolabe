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
                | Statement::SetMagicPointsMaximum(value)
                | Statement::Assign { value, .. }
                | Statement::ActorFieldAssign { value, .. }
                | Statement::MainActorFieldAssign { value, .. }
                | Statement::SubtractAssign { value, .. }
                | Statement::AddAssign { value, .. }
                | Statement::VectorComponentAssign { value, .. }
                | Statement::Wait {
                    milliseconds: value,
                }
                | Statement::WaitFrames(value)
                | Statement::ChangeAction(value)
                | Statement::ExecuteAction(value)
                | Statement::Turn {
                    degrees_per_second: value,
                }
                | Statement::IncrementalTurnToward { frames: value, .. }
                | Statement::SetVectorSpeed(value)
                | Statement::AddWind(value)
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
                Statement::GoToPosition { point, radius, .. } => {
                    self.expressions(child_depth, &[point, radius]);
                }
                Statement::SetHitPointsMaximum { actor, value }
                | Statement::SetInventoryMaximum { item: actor, value }
                | Statement::AddHitPoints { actor, value } => {
                    self.expressions(child_depth, &[actor, value])
                }
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
                } => self.expressions(child_depth, &[target, duration_ms]),
                Statement::ExecuteFade {
                    from,
                    to,
                    duration_seconds,
                } => self.expressions(child_depth, &[from, to, duration_seconds]),
                Statement::SoundFadeAll {
                    target,
                    duration_ms,
                } => self.expressions(child_depth, &[target, duration_ms]),
                Statement::Unsupported(_)
                | Statement::Increment { .. }
                | Statement::Decrement { .. }
                | Statement::ChangeBehavior { .. }
                | Statement::ChangeActorBehavior { .. }
                | Statement::ResetRoute(_)
                | Statement::GoOverRoute(_)
                | Statement::GoToNextWaypoint(_)
                | Statement::GoRelative
                | Statement::KillSelf
                | Statement::KillPerso { .. }
                | Statement::JumpAbsolute
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
                | Statement::RepairArmor
                | Statement::CameraChangeTargetedPerso { .. }
                | Statement::CameraResetTargetedPerso { .. }
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
                | Expression::ActorDistance(value)
                | Expression::ActorHitPointsMaximum(value)
                | Expression::ActorLocalSourceXYDistance(value)
                | Expression::IsValidActor(value)
                | Expression::ApplyArmoredDamage(value)
                | Expression::IsInAction(value)
                | Expression::Not(value)
                | Expression::Negate(value)
                | Expression::InventoryQuantity(value)
                | Expression::MagicRemovePoints(value)
                | Expression::ActorListContains { actor: value, .. }
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
                | Expression::ActorFacingAngle { target, .. } => {
                    self.expressions(child_depth, &[target]);
                }
                Expression::Vector(values) => {
                    self.expressions(child_depth, &[&values[0], &values[1], &values[2]]);
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
                | Expression::RandomInt {
                    lower: left,
                    upper: right,
                }
                | Expression::Compare { left, right, .. }
                | Expression::VectorCos(left, right)
                | Expression::Multiply(left, right)
                | Expression::Plus(left, right)
                | Expression::VectorPlus(left, right)
                | Expression::Minus(left, right)
                | Expression::Divide(left, right)
                | Expression::And(left, right)
                | Expression::Or(left, right)
                | Expression::WaypointSphereContains {
                    actor: left,
                    waypoint: right,
                } => self.expressions(child_depth, &[left, right]),
                Expression::Unsupported(_)
                | Expression::AbsentButton(_)
                | Expression::Nobody
                | Expression::Nowhere
                | Expression::Actor(_)
                | Expression::Configuration(_)
                | Expression::Integer(_)
                | Expression::Real(_)
                | Expression::SelfPosition
                | Expression::ActorPosition(_)
                | Expression::SelfPositionField
                | Expression::WaypointReference(_)
                | Expression::WaypointPosition(_)
                | Expression::DistanceToWaypoint { .. }
                | Expression::GameTime
                | Expression::SeePerso { .. }
                | Expression::Variable(_)
                | Expression::Action(_)
                | Expression::ActionFinished
                | Expression::CurrentAnimationFrame
                | Expression::DialogIsOver
                | Expression::DialogIsOverAndChangeBehavior
                | Expression::CollideWithGround
                | Expression::CollideWithNothing
                | Expression::CollidePersoZdeNoWithTypeZde { .. }
                | Expression::CollideWithSlope
                | Expression::IsTypeOfGmtCollide(_)
                | Expression::InTopOfJump
                | Expression::ActorFieldRead { .. }
                | Expression::SelfFieldRead(_)
                | Expression::MainActorFieldRead(_)
                | Expression::IsInBehavior { .. }
                | Expression::Button { .. }
                | Expression::MainActorDistance
                | Expression::SelfHitPoints
                | Expression::ArmorResistance
                | Expression::ArmorCount
                | Expression::InventoryWeapon
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
