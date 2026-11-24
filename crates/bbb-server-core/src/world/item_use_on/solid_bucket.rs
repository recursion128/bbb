//! `SolidBucketItem.useOn` (`SolidBucketItem.java:26-34`): the powder snow
//! bucket places its block and leaves an empty bucket.

use bbb_protocol::block_types::VANILLA_BLOCK_POWDER_SNOW_ID;
use bbb_protocol::item_types::VANILLA_ITEM_BUCKET_ID;
use bbb_protocol::packets::ItemStackSummary;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::ItemUseOnContext;
use crate::world::{ServerWorld, WorldEvent};

/// `Items.POWDER_SNOW_BUCKET`'s `placeSound` (`Items.java:1321-1325`).
const PLACE_SOUND: &str = "minecraft:item.bucket.empty_powder_snow";

pub(super) fn use_on(
    world: &mut ServerWorld,
    context: &ItemUseOnContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    let held = RuntimeItemStack::from_summary(context.stack.clone());
    if !world.place_default_block_from_item(
        context.connection_id,
        context.dimension,
        context.pos(),
        context.update.hit.direction,
        VANILLA_BLOCK_POWDER_SNOW_ID,
        context.hand(),
        &held,
        Some(PLACE_SOUND),
        events,
    ) {
        return false;
    }
    // `BucketItem.getEmptySuccessItem` (`BucketItem.java:97-99`).
    let player = &mut world.levels[context.connection_id];
    if !player.abilities.instabuild {
        player.inventory.set_item_in_hand(
            context.hand(),
            RuntimeItemStack::from_summary(ItemStackSummary {
                item_id: Some(VANILLA_ITEM_BUCKET_ID),
                count: 1,
                ..ItemStackSummary::empty()
            }),
        );
    }
    true
}
