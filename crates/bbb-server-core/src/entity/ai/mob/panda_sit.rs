//! Panda food seeking and autonomous sitting.

use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::Vec3d;

use crate::entity::EntityData;

use super::target::TargetView;
use super::{reduced_tick_delay, Goal, MobAiContext};

#[derive(Debug, Clone, Default)]
pub(super) struct PandaSitGoal {
    cooldown: i32,
    next_search: i32,
}

fn holding_item(targets: &TargetView<'_, '_, '_>) -> bool {
    targets
        .body_entity()
        .living()
        .is_some_and(|living| living.item_by_slot(EquipmentSlot::MainHand).is_some())
}

fn food_position(targets: &TargetView<'_, '_, '_>, range: f64) -> Option<Vec3d> {
    let bounds = targets
        .body_entity()
        .bounding_box()
        .inflate(range, range, range);
    targets
        .store()
        .first_entity_in_box(bounds, |body| {
            let EntityData::Item(item) = body.data() else {
                return false;
            };
            body.is_alive()
                && !item.has_pickup_delay()
                && item.item().item_resource_id().is_some_and(|item| {
                    targets.level().registry_tag_membership(
                        "minecraft:item",
                        "minecraft:panda_eats_from_ground",
                        item,
                    )
                })
        })
        .map(|body| body.position())
}

fn try_sitting(context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
    if !targets.body_entity().is_in_water() {
        context.controls.zza = 0.0;
        context.stop_navigation();
        if let Some((entity, _, _)) = targets.world_mut() {
            crate::entity::panda::try_sit(entity);
        }
    }
}

impl Goal for PandaSitGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let body = targets.body_entity();
        let EntityData::GeneralLiving(data) = body.data() else {
            return false;
        };
        let Some(panda) = data.panda_retained() else {
            return false;
        };
        if self.cooldown > context.tick_count
            || data.is_baby()
            || body.is_in_water()
            || !panda.can_perform_action(targets.level().is_thundering())
            || panda.is_unhappy()
        {
            return false;
        }
        if holding_item(targets) {
            return true;
        }
        if context.tick_count < self.next_search {
            return false;
        }
        self.next_search = context.tick_count.saturating_add(20);
        food_position(targets, 6.0).is_some()
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let lazy = matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
            if data.panda_retained().is_some_and(|panda| panda.is_lazy()));
        !targets.body_entity().is_in_water()
            && (lazy || context.next_int(reduced_tick_delay(600)) != 1)
            && context.next_int(reduced_tick_delay(2000)) != 1
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if holding_item(targets) {
            try_sitting(context, targets);
        } else if let Some(position) = food_position(targets, 8.0) {
            context.navigate_to_position(position, 1.2);
        }
        self.cooldown = 0;
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let sitting = matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
            if data.panda_retained().is_some_and(|panda| panda.is_sitting()));
        if !sitting && holding_item(targets) {
            try_sitting(context, targets);
        }
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if holding_item(targets) {
            let lazy = matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
                if data.panda_retained().is_some_and(|panda| panda.is_lazy()));
            let wait = 10 + context.next_int(if lazy { 50 } else { 150 });
            self.cooldown = context.tick_count.saturating_add(wait * 20);
        }
        if let Some((entity, store, level)) = targets.world_mut() {
            if !crate::entity::panda::stop_sitting(entity, store, level) {
                targets.abort();
            }
        }
    }
}
