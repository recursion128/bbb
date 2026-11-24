//! `FoodOnAStickItem.use` (`FoodOnAStickItem.java:24-39`): the rider steering
//! a pig or strider boosts it and wears the stick, which turns back into a
//! fishing rod when it breaks (`ItemStack.java:499-511`).

use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_STRIDER_ID};
use bbb_protocol::item_types::{
    VANILLA_ITEM_CARROT_ON_A_STICK_ID, VANILLA_ITEM_FISHING_ROD_ID,
    VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID,
};
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::stats::StatKey;

use super::{HeldItem, ItemUseContext, ItemUseOutcome, ItemUseResult};
use crate::world::{ServerWorld, WorldEvent};

pub(super) fn use_item(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    events: &mut Vec<WorldEvent>,
) -> ItemUseOutcome {
    // `Items.java:1036-1041`.
    let (steered_type, damage) = match context.item.item_id {
        Some(VANILLA_ITEM_CARROT_ON_A_STICK_ID) => (VANILLA_ENTITY_TYPE_PIG_ID, 7),
        Some(VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID) => (VANILLA_ENTITY_TYPE_STRIDER_ID, 1),
        _ => return ItemUseResult::Pass.into(),
    };
    let boosted = world
        .external_player_controlled_vehicle(context.connection_id)
        .is_some_and(|vehicle_id| {
            let entities = &mut world.level_mut_by_id(&context.dimension).entities;
            let random = entities.random().clone();
            let mut boosted = false;
            if let Some(vehicle) = entities
                .get_mut(vehicle_id)
                .filter(|vehicle| vehicle.entity_type_id() == steered_type)
            {
                boosted = crate::entity::item_steering::boost(vehicle, &random);
            }
            boosted
        });
    if !boosted {
        if let Some(item_id) = context.item.item_id {
            world.levels[context.connection_id].award_stat_once(
                &mut world.scoreboard,
                StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
            );
        }
        return ItemUseResult::Pass.into();
    }
    if world.hurt_and_break_in_hand(context.connection_id, context.hand, damage, events) {
        return ItemUseResult::SuccessServer.into();
    }
    let mut rod = context.item.clone();
    rod.item_id = Some(VANILLA_ITEM_FISHING_ROD_ID);
    rod.count = 1;
    crate::item_durability::repair_to(&mut rod, 0);
    ItemUseOutcome {
        result: ItemUseResult::SuccessServer,
        transformed: Some(HeldItem::Replaced(RuntimeItemStack::from_summary(rod))),
    }
}
