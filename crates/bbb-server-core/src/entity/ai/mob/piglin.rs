use super::look_at::{LookAtPlayerGoal, PiglinAttentionMode, PiglinPeer};
use super::random_pos::{land_random_pos_away, RandomPosMob};
use super::stroll::{RandomStrollGoal, StrollPosition};
use super::target::{TargetRef, TargetView};
use super::{Goal, MobAiContext};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID,
};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::geometry::Vec3Ops;

#[derive(Debug, Clone, Copy)]
enum IdleAction {
    Stroll,
    SavedHome,
    AroundHome,
    Piglin,
    Brute,
    LookWalk,
    Wait(i64),
    CelebrateLook,
    Ride,
}

impl IdleAction {
    fn moves(self) -> bool {
        !matches!(self, Self::Wait(_) | Self::CelebrateLook)
    }
}

#[derive(Debug, Clone)]
pub(super) struct PiglinIdleGoal {
    look: LookAtPlayerGoal,
    stroll: RandomStrollGoal,
    saved_home: RandomStrollGoal,
    around_home: RandomStrollGoal,
    action: IdleAction,
}

impl Default for PiglinIdleGoal {
    fn default() -> Self {
        Self {
            look: LookAtPlayerGoal::piglin(),
            stroll: RandomStrollGoal::new(0.6, StrollPosition::Land, 0),
            saved_home: RandomStrollGoal::new(0.6, StrollPosition::SavedHome, 80),
            around_home: RandomStrollGoal::new(0.6, StrollPosition::AroundSavedHome, 180),
            action: IdleAction::Wait(0),
        }
    }
}

impl PiglinIdleGoal {
    fn attention_mode(
        context: &MobAiContext,
        targets: &TargetView<'_, '_, '_>,
    ) -> PiglinAttentionMode {
        if context.species.piglin_ride_target.is_some() {
            PiglinAttentionMode::Ride
        } else if targets.body_entity().entity_type_id() == VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID {
            PiglinAttentionMode::BruteIdle
        } else if crate::entity::piglin::celebration(targets.body_entity()).is_some() {
            PiglinAttentionMode::Celebrate
        } else {
            PiglinAttentionMode::Idle
        }
    }

    fn wait(
        context: &mut MobAiContext,
        targets: &TargetView<'_, '_, '_>,
        celebrating: bool,
    ) -> IdleAction {
        let duration = if celebrating {
            10 + context.next_int(11)
        } else {
            30 + context.next_int(31)
        };
        IdleAction::Wait(targets.game_time() + i64::from(duration))
    }

    fn stroll_goal(&mut self) -> Option<&mut RandomStrollGoal> {
        match self.action {
            IdleAction::Stroll => Some(&mut self.stroll),
            IdleAction::SavedHome => Some(&mut self.saved_home),
            IdleAction::AroundHome => Some(&mut self.around_home),
            _ => None,
        }
    }

    fn approach(
        context: &mut MobAiContext,
        targets: &TargetView<'_, '_, '_>,
        target: TargetRef,
        reach: i32,
    ) {
        if !context.navigation_done() || context.nav.navigation_request.is_some() {
            return;
        }
        if let Some(target) = targets
            .facts_of(target)
            .filter(|target| target.is_local() && target.alive())
        {
            if context.body.position.distance_to_sqr(target.position()) > f64::from(reach * reach) {
                context.try_navigate_to_position(target.position(), reach, 0.6);
            }
        }
    }
}

impl Goal for PiglinIdleGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if !context.species.piglin_idle_allowed {
            return false;
        }
        if context.species.piglin_ride_target.is_some() {
            self.action = IdleAction::Ride;
            return true;
        }
        let celebrating = crate::entity::piglin::celebration(targets.body_entity()).is_some();
        self.action = if celebrating {
            match context.next_int(3) {
                0 => IdleAction::CelebrateLook,
                1 => IdleAction::Stroll,
                _ => Self::wait(context, targets, true),
            }
        } else if targets.body_entity().entity_type_id() == VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID {
            match context.next_int(11) {
                0 | 1 => IdleAction::Stroll,
                2 | 3 => IdleAction::Piglin,
                4 | 5 => IdleAction::Brute,
                6 | 7 => IdleAction::SavedHome,
                8 | 9 => IdleAction::AroundHome,
                _ => Self::wait(context, targets, false),
            }
        } else {
            match context.next_int(7) {
                0 | 1 => IdleAction::Stroll,
                2 | 3 => IdleAction::Piglin,
                4 | 5 => IdleAction::LookWalk,
                _ => Self::wait(context, targets, false),
            }
        };
        if self.action.moves()
            && (!context.navigation_done() || context.nav.navigation_request.is_some())
        {
            self.action = Self::wait(context, targets, celebrating);
        }
        if matches!(self.action, IdleAction::Stroll) {
            self.stroll = RandomStrollGoal::new(
                0.6,
                if celebrating {
                    StrollPosition::LandNear { xz: 2, y: 1 }
                } else {
                    StrollPosition::Land
                },
                0,
            );
        }
        if self
            .stroll_goal()
            .is_some_and(|stroll| !stroll.can_use(context, targets))
        {
            self.action = Self::wait(context, targets, celebrating);
        }
        true
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if !context.species.piglin_idle_allowed {
            return false;
        }
        if matches!(self.action, IdleAction::Ride) != context.species.piglin_ride_target.is_some() {
            return false;
        }
        if matches!(self.action, IdleAction::Ride) {
            return true;
        }
        if let Some(stroll) = self.stroll_goal() {
            return stroll.can_continue(context, targets);
        }
        match self.action {
            IdleAction::Wait(until) => targets.game_time() < until,
            IdleAction::CelebrateLook => false,
            _ => !context.navigation_done(),
        }
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some(stroll) = self.stroll_goal() {
            stroll.start(context, targets);
            return;
        }
        match self.action {
            IdleAction::Piglin | IdleAction::Brute => {
                let peer = if matches!(self.action, IdleAction::Piglin) {
                    PiglinPeer::Piglin
                } else {
                    PiglinPeer::Brute
                };
                if let Some(target) = self.look.look_at_peer(context, targets, peer) {
                    Self::approach(context, targets, target, 2);
                }
            }
            IdleAction::LookWalk => {
                let mode = Self::attention_mode(context, targets);
                let loved_player = self.look.tick_piglin_attention(context, targets, mode);
                if !loved_player {
                    if let Some(target) = self.look.current_idle_target(context, targets) {
                        Self::approach(context, targets, target, 3);
                    }
                }
            }
            IdleAction::CelebrateLook => {
                let _ = self.look.look_at_peer(context, targets, PiglinPeer::Piglin);
            }
            _ => {}
        }
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        // Higher-priority movement is installed after idle cleanup.
        if let Some(stroll) = self.stroll_goal() {
            stroll.stop(context, targets);
        } else if self.action.moves() {
            context.stop_navigation();
        }
        self.action = IdleAction::Wait(0);
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let mode = Self::attention_mode(context, targets);
        if matches!(self.action, IdleAction::Ride) {
            if targets.store().vehicle(context.entity_id).is_some() {
                self.look.tick_piglin_attention(context, targets, mode);
            } else if let Some(target) = context
                .species
                .piglin_ride_target
                .and_then(|target| targets.facts_of(target))
                .filter(|target| target.is_local() && target.alive())
            {
                let position = target.position();
                context.set_look_at(position.x, target.eye_y(), position.z);
                // Mount. Reach zero lets the actual distance fall below one.
                if context.body.position.distance_to_sqr(position) >= 1.0
                    && context.navigation_done()
                    && context.nav.navigation_request.is_none()
                {
                    context.try_navigate_to_position(position, 0, 0.8);
                }
                self.look.tick_piglin_attention(context, targets, mode);
            }
            return;
        }
        self.look.tick_piglin_attention(context, targets, mode);
        if let Some(stroll) = self.stroll_goal() {
            stroll.tick(context, targets);
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

fn ordinary_idle(context: &MobAiContext, targets: &TargetView<'_, '_, '_>) -> bool {
    targets.body_entity().entity_type_id() == VANILLA_ENTITY_TYPE_PIGLIN_ID
        && context.target.target.is_none()
        && !crate::entity::piglin::is_admiring(targets.body_entity())
}

fn near_repellent(context: &MobAiContext, repellent: Option<BlockPos>) -> Option<Vec3d> {
    let pos = repellent?;
    let pos = Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y),
        z: f64::from(pos.z) + 0.5,
    };
    (context.body.position.distance_to_sqr(pos) < 64.0).then_some(pos)
}

fn celebration_approach(targets: &TargetView<'_, '_, '_>) -> Option<(BlockPos, i32, f64)> {
    let celebration = crate::entity::piglin::celebration(targets.body_entity())?;
    let (reach, speed) = if celebration.dancing {
        (4, 0.6)
    } else {
        (2, 1.0)
    };
    let body = targets.body_entity().block_position();
    let location = celebration.location;
    let dx = f64::from(body.x) - f64::from(location.x);
    let dy = f64::from(body.y) - f64::from(location.y);
    let dz = f64::from(body.z) - f64::from(location.z);
    (dx * dx + dy * dy + dz * dz >= f64::from(reach * reach)).then_some((location, reach, speed))
}

pub(super) fn idle_eligible(
    context: &MobAiContext,
    targets: &TargetView<'_, '_, '_>,
    avoid: Option<TargetRef>,
    repellent: Option<BlockPos>,
) -> bool {
    if targets.body_entity().entity_type_id() == VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID {
        return context.target.target.is_none();
    }
    ordinary_idle(context, targets)
        && avoid.is_none()
        && (context.species.piglin_ride_target.is_some()
            || (!context.navigation_done()
                || context.nav.navigation_request.is_some()
                || (near_repellent(context, repellent).is_none()
                    && celebration_approach(targets).is_none())))
}

pub(super) fn avoid_threat(
    context: &mut MobAiContext,
    targets: &TargetView<'_, '_, '_>,
    avoid: Option<TargetRef>,
    repellent: Option<BlockPos>,
) -> bool {
    if !ordinary_idle(context, targets) || context.species.piglin_ride_target.is_some() {
        return false;
    }
    let avoid = if let Some(avoid) = avoid {
        let Some(avoid) = targets
            .facts_of(avoid)
            .filter(|target| target.is_local() && target.alive())
        else {
            return false;
        };
        if context.body.position.distance_to_sqr(avoid.position()) >= 144.0 {
            return false;
        }
        avoid.position()
    } else {
        if !context.nav.navigation_done || context.nav.navigation_request.is_some() {
            return false;
        }
        let Some(pos) = near_repellent(context, repellent) else {
            return false;
        };
        pos
    };
    let dx = avoid.x - context.body.position.x;
    let dy = avoid.y - context.body.position.y;
    let dz = avoid.z - context.body.position.z;
    if !context.nav.navigation_done
        && context.nav.navigation_target.is_some_and(|target| {
            (f64::from(target.x) + 0.5 - context.body.position.x) * dx
                + (f64::from(target.y) - context.body.position.y) * dy
                + (f64::from(target.z) + 0.5 - context.body.position.z) * dz
                < 0.0
        })
    {
        context.set_navigation_speed(1.0);
        return true;
    }
    let mob = RandomPosMob::of(targets.body_entity(), context.nav.malus);
    if let Some(Some(destination)) =
        land_random_pos_away(targets.level(), &context.random, mob, 0.0, 16.0, 7, avoid)
    {
        context.try_navigate_to_position(destination, 0, 1.0);
        return true;
    }
    false
}

pub(super) fn approach_celebration(
    context: &mut MobAiContext,
    targets: &TargetView<'_, '_, '_>,
    avoid: Option<TargetRef>,
) -> bool {
    if !ordinary_idle(context, targets)
        || avoid.is_some()
        || !context.nav.navigation_done
        || context.nav.navigation_request.is_some()
    {
        return false;
    }
    let Some((location, reach, speed)) = celebration_approach(targets) else {
        return false;
    };
    let destination = Vec3d {
        x: f64::from(location.x) + f64::from(context.next_int(3) - 1) + 0.5,
        y: f64::from(location.y) + 0.5,
        z: f64::from(location.z) + f64::from(context.next_int(3) - 1) + 0.5,
    };
    context.set_look_at(destination.x, destination.y, destination.z);
    context.try_navigate_to_position(destination, reach, speed);
    true
}
