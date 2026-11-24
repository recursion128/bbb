//! Wandering trader travel to its retained destination.

use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::geometry::Vec3Ops;

use super::target::TargetView;
use super::{Goal, MobAiContext};
use crate::entity::wandering_trader::TraderDrinkKind;

#[derive(Debug, Clone)]
pub(super) struct TraderUseItemGoal {
    kind: TraderDrinkKind,
}

impl TraderUseItemGoal {
    pub(super) const fn new(kind: TraderDrinkKind) -> Self {
        Self { kind }
    }
}

impl Goal for TraderUseItemGoal {
    fn can_use(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let invisible = targets.body_entity().is_invisible();
        match self.kind {
            TraderDrinkKind::Invisibility => {
                targets.level().is_dark_outside() == Some(true) && !invisible
            }
            TraderDrinkKind::Milk => targets.bright_outside() == Some(true) && invisible,
        }
    }

    fn can_continue(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        targets
            .body_entity()
            .living()
            .is_some_and(|living| living.living_entity_flags() & 1 != 0)
    }

    fn start(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some((entity, store, level)) = targets.world_mut() {
            crate::entity::wandering_trader::apply_drink_change(
                entity,
                store,
                level,
                None,
                Some(self.kind),
            );
        }
    }

    fn stop(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some((entity, store, level)) = targets.world_mut() {
            crate::entity::wandering_trader::apply_drink_change(
                entity,
                store,
                level,
                Some(self.kind),
                None,
            );
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct WanderToPositionGoal;

impl Goal for WanderToPositionGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        targets
            .body_entity()
            .wandering_trader_target()
            .is_some_and(|target| center_distance_squared(context.position(), target) >= 4.0)
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some((entity, _, _)) = targets.world_mut() {
            entity.set_wandering_trader_target(None);
        }
        context.stop_navigation();
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if !context.navigation_done() {
            return;
        }
        let Some(target) = targets.body_entity().wandering_trader_target() else {
            return;
        };
        let position = context.position();
        let wanted = Vec3d {
            x: f64::from(target.x),
            y: f64::from(target.y),
            z: f64::from(target.z),
        };
        let wanted = if center_distance_squared(position, target) >= 100.0 {
            Vec3d {
                x: wanted.x - position.x,
                y: wanted.y - position.y,
                z: wanted.z - position.z,
            }
            .normalize()
            .scale(10.0)
            .plus(position)
        } else {
            wanted
        };
        context.navigate_to_position(wanted, 0.35);
    }
}

fn center_distance_squared(position: Vec3d, target: BlockPos) -> f64 {
    position.distance_to_sqr(Vec3d {
        x: f64::from(target.x) + 0.5,
        y: f64::from(target.y) + 0.5,
        z: f64::from(target.z) + 0.5,
    })
}
