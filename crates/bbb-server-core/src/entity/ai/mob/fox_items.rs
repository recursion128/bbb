//! Fox navigation to loose items; contact pickup owns the mouth stack.

use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::Vec3d;

use crate::entity::EntityData;

use super::target::TargetView;
use super::{reduced_tick_delay, Goal, MobAiContext};

#[derive(Debug, Clone)]
pub(super) struct FoxSearchItemsGoal;

fn item_position(targets: &TargetView<'_, '_, '_>) -> Option<Vec3d> {
    // Fox.
    let bounds = targets.body_entity().bounding_box().inflate(8.0, 8.0, 8.0);
    targets
        .store()
        .first_entity_in_box(bounds, |body| {
            matches!(body.data(), EntityData::Item(item)
                if body.is_alive() && !item.has_pickup_delay())
        })
        .map(|body| body.position())
}

impl Goal for FoxSearchItemsGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let EntityData::GeneralLiving(data) = targets.body_entity().data() else {
            return false;
        };
        data.living()
            .item_by_slot(EquipmentSlot::MainHand)
            .is_none()
            && context.target.target.is_none()
            && targets.last_hurt_by_mob().is_none()
            && data.fox_retained().is_some_and(|fox| fox.can_move())
            && context.next_int(reduced_tick_delay(10)) == 0
            && item_position(targets).is_some()
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some(position) = item_position(targets) {
            context.navigate_to_position(position, 1.2);
        }
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if targets
            .body_entity()
            .living()
            .is_some_and(|living| living.item_by_slot(EquipmentSlot::MainHand).is_none())
        {
            self.start(context, targets);
        }
    }
}
