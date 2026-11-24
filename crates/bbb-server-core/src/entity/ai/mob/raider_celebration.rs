//! Celebration by surviving members of a lost raid.

use crate::entity::raider::RaiderRaid;
use crate::entity::EntityData;

use super::target::TargetView;
use super::{Goal, MobAiContext};

#[derive(Debug, Clone, Default)]
pub(super) struct RaiderCelebrationGoal;

impl Goal for RaiderCelebrationGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let body = targets.body_entity();
        if !body.is_alive_with_health() || context.target.target.is_some() {
            return false;
        }
        let EntityData::GeneralLiving(data) = body.data() else {
            return false;
        };
        let Some(RaiderRaid::Current(id)) = data.raider_retained().map(|raider| raider.raid())
        else {
            return false;
        };
        targets
            .level()
            .raider_raid(id)
            .is_some_and(|raid| raid.is_loss)
    }

    fn start(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some((entity, store, _)) = targets.world_mut() {
            crate::entity::raider::apply_celebration(entity, store, Some(true), false);
        }
    }

    fn stop(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some((entity, store, _)) = targets.world_mut() {
            crate::entity::raider::apply_celebration(entity, store, Some(false), false);
        }
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if !targets.body_entity().is_silent()
            && context.random.next_int(super::reduced_tick_delay(100)) == 0
        {
            if let Some((entity, store, _)) = targets.world_mut() {
                crate::entity::raider::apply_celebration(entity, store, None, true);
            }
        }
        let body_id = targets.body_entity().id();
        if targets.store().vehicle(body_id).is_none()
            && context.random.next_int(super::reduced_tick_delay(50)) == 0
        {
            context.request_jump();
        }
    }
}
