//! One exhaustive, ordered traversal for inspection, dependencies and validation.
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
    /// Preorder traversal in source evaluation order, with structural depth.
    /// An explicit stack also lets validation reject deep authored trees before
    /// recursive codec or compiler operations see them.
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
                Statement::Assign { value, .. }
                | Statement::SelfFieldAssign { value, .. }
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
                | Statement::SetTransparency(value)
                | Statement::ActorTransparency { opacity: value, .. }
                | Statement::ChangeSighting(value)
                | Statement::ChangeConfiguration(value)
                | Statement::CameraChangeState { state: value, .. }
                | Statement::SetActorAbsolutePosition {
                    position: value, ..
                }
                | Statement::RunDialog { mode: value, .. }
                | Statement::ScrollDialog(value)
                | Statement::SetHudVisible { visible: value, .. } => {
                    self.expressions(child_depth, &[value])
                }
                Statement::ChangeActorBehavior { actor: value, .. }
                | Statement::ChangeArmor(value)
                | Statement::SetArmorResistance(value) => self.expressions(child_depth, &[value]),
                Statement::WaitBoolean { field, value } => {
                    self.expressions(child_depth, &[field, value])
                }
                Statement::SwapConfiguredParts { first, second } => {
                    self.expressions(child_depth, &[first, second])
                }
                Statement::ArrayAssign {
                    array,
                    index,
                    value,
                } => self.expressions(child_depth, &[array, index, value]),
                Statement::PositionDialog { x, y } => self.expressions(child_depth, &[x, y]),
                Statement::ExecuteActionAndGenerateObject { action, vector, .. } => {
                    self.expressions(child_depth, &[action, vector])
                }
                Statement::SoundFadeMusic { arg0, arg1 } => {
                    self.expressions(child_depth, &[arg0, arg1])
                }
                Statement::Unsupported(_)
                | Statement::Increment { .. }
                | Statement::Decrement { .. }
                | Statement::ChangeBehavior { .. }
                | Statement::ResetRoute(_)
                | Statement::GoOverRoute(_)
                | Statement::GoRelative
                | Statement::KillSelf
                | Statement::JumpAbsolute
                | Statement::JumpWithoutAddingSpeed
                | Statement::SetParticleGeneratorEnabled(_)
                | Statement::SetParticleGenerator(_)
                | Statement::SetLaserPointerActive(_)
                | Statement::SetDisplayVisible(_)
                | Statement::SetTransparentDisplay(_)
                | Statement::ActorDisplayVisible { .. }
                | Statement::ActorTransparentDisplay { .. }
                | Statement::CameraChangeTargetedPerso { .. }
                | Statement::CameraUpdatePosition { .. }
                | Statement::ActorListAdd { .. }
                | Statement::SetChannelActive { .. }
                | Statement::ToggleWideScreen
                | Statement::SelectDialog
                | Statement::EndDialog
                | Statement::ChangeDialogBehavior
                | Statement::WaitEndOfAction
                | Statement::WaitEndOfAnimation
                | Statement::Sound { .. }
                | Statement::Noop => {}
                Statement::RepairArmor => {}
            },
            AstNode::Expression(expression) => match expression {
                Expression::IntegerCast(value)
                | Expression::Square(value)
                | Expression::AbsoluteValue(value)
                | Expression::ApplyArmoredDamage(value)
                | Expression::IsValidActor(value)
                | Expression::ActorDistance(value)
                | Expression::IsInAction(value)
                | Expression::Not(value)
                | Expression::Negate(value)
                | Expression::InventoryQuantity(value)
                | Expression::InventoryAddGold(value)
                | Expression::InventorySubtractGold(value)
                | Expression::InventorySelectObject(value)
                | Expression::InventoryAddActorObject(value)
                | Expression::InventoryChangeWeapon(value)
                | Expression::InventoryMaximumCapacity(value)
                | Expression::MagicRemovePoints(value)
                | Expression::ActorListContains { actor: value, .. }
                | Expression::VectorComponent { vector: value, .. } => {
                    self.expressions(child_depth, &[value])
                }
                Expression::Vector(values) => {
                    self.expressions(child_depth, &[&values[0], &values[1], &values[2]])
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
                | Expression::Minus(left, right)
                | Expression::Divide(left, right)
                | Expression::And(left, right)
                | Expression::Or(left, right) => self.expressions(child_depth, &[left, right]),
                Expression::Unsupported(_)
                | Expression::AbsentButton(_)
                | Expression::Nobody
                | Expression::Nowhere
                | Expression::Integer(_)
                | Expression::Real(_)
                | Expression::Actor(_)
                | Expression::Configuration(_)
                | Expression::SelfPosition
                | Expression::WaypointPosition(_)
                | Expression::GameTime
                | Expression::SeePerso { .. }
                | Expression::Variable(_)
                | Expression::Action(_)
                | Expression::ActionFinished
                | Expression::DialogIsOver
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
                | Expression::InventoryWeapon
                | Expression::InventoryGold
                | Expression::InventoryObject
                | Expression::ArmorResistance
                | Expression::ArmorCount
                | Expression::MagicCurrentSpell
                | Expression::MagicPoints
                | Expression::LaserDirection
                | Expression::SelfSighting => {}
            },
        }
        Some((node, depth))
    }
}
