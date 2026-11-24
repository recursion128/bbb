//! The filled-bucket and empty-bucket behaviours over `BucketItem`,
//! `MobBucketItem`, `SolidBucketItem` and `BucketPickup`.

use bbb_protocol::block_types::VANILLA_BLOCK_POWDER_SNOW_ID;
use bbb_protocol::item_types::{
    vanilla_item_type, VanillaBucketFluid, VANILLA_ITEM_BUCKET_ID,
    VANILLA_ITEM_POWDER_SNOW_BUCKET_ID,
};
use bbb_protocol::packets::ItemStackSummary;
use bbb_server_types::fluid::{block_state_is_air, FluidFamily};
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result as CoreResult;

use super::{BlockSource, Dispensed, DISPENSE_SOUND};
use crate::block_behavior::bucket_pickup::pickup_block;
use crate::block_behavior::default_block_state_id;
use crate::fluid::{empty_bucket, BucketEmptied, BucketUser};
use crate::level_event::VanillaGameEvent;
use crate::world::item_use::bucket::spawn_bucketed_mob;
use crate::world::ServerWorld;

/// The filled-bucket behaviour: `emptyContents` in front, then
/// `checkExtraContent`, leaving an empty bucket.
pub(super) fn empty(
    world: &mut ServerWorld,
    source: &BlockSource,
    slot: usize,
    stack: RuntimeItemStack,
) -> CoreResult<Dispensed> {
    let emptied = if stack.item_id() == Some(VANILLA_ITEM_POWDER_SNOW_BUCKET_ID) {
        place_powder_snow(world, source)
    } else {
        empty_fluid(world, source, stack.summary())
    };
    if !emptied {
        return world.default_dispense(source, stack);
    }
    let bucket = RuntimeItemStack::from_summary(single_item(VANILLA_ITEM_BUCKET_ID));
    let stack = world.consume_with_remainder(source, slot, stack, bucket)?;
    Ok(Dispensed::new(stack, DISPENSE_SOUND))
}

/// `BucketItem.emptyContents(null, level, target, null)` and
/// `MobBucketItem.checkExtraContent`.
fn empty_fluid(world: &mut ServerWorld, source: &BlockSource, item: &ItemStackSummary) -> bool {
    let Some(behavior) = item
        .item_id
        .and_then(vanilla_item_type)
        .and_then(|entry| entry.bucket_behavior)
    else {
        return false;
    };
    let fluid = match behavior.fluid {
        VanillaBucketFluid::Water => FluidFamily::Water,
        VanillaBucketFluid::Lava => FluidFamily::Lava,
        VanillaBucketFluid::Empty => return false,
    };
    let target = source.target();
    let user = BucketUser {
        shift_key_down: false,
        creative: false,
    };
    let Some((at, emptied)) = world.with_level(source.level, |level| {
        empty_bucket(level, target, fluid, user, None)
    }) else {
        return false;
    };
    let pitch = if matches!(emptied, BucketEmptied::Evaporated) {
        2.6 + (world.random.next_float() - world.random.next_float()) * 0.8
    } else {
        1.0
    };
    let sink = &mut world.level_mut_by_id(&source.level).event_sink;
    match (emptied, behavior.mob_empty_sound_resource_id) {
        (BucketEmptied::Evaporated, _) => {
            sink.play_block_sound_with_volume_pitch(
                at,
                "minecraft:block.fire.extinguish",
                0.5,
                pitch,
            );
        }
        (BucketEmptied::Placed, Some(sound)) => sink.play_block_sound(at, sound),
        (BucketEmptied::Placed, None) => {
            sink.play_block_sound(
                at,
                match fluid {
                    FluidFamily::Water => "minecraft:item.bucket.empty",
                    FluidFamily::Lava => "minecraft:item.bucket.empty_lava",
                },
            );
            sink.game_event(VanillaGameEvent::FLUID_PLACE, at, None);
        }
    }
    if behavior.mob_empty_sound_resource_id.is_some() {
        spawn_bucketed_mob(world, None, source.level, item, target);
        world.level_mut_by_id(&source.level).event_sink.game_event(
            VanillaGameEvent::ENTITY_PLACE,
            target,
            None,
        );
    }
    true
}

/// `SolidBucketItem.emptyContents`: powder snow into an empty cell.
fn place_powder_snow(world: &mut ServerWorld, source: &BlockSource) -> bool {
    let target = source.target();
    let placed = world.with_level(source.level, |level| {
        block_state_is_air(level.block_state(target))
            && level.set_block(
                target,
                default_block_state_id(VANILLA_BLOCK_POWDER_SNOW_ID),
                crate::fluid::SET_BLOCK_UPDATE_ALL,
                crate::neighbor_updater::UPDATE_LIMIT,
            )
    });
    if placed {
        let sink = &mut world.level_mut_by_id(&source.level).event_sink;
        sink.game_event(VanillaGameEvent::FLUID_PLACE, target, None);
        sink.play_block_sound(target, "minecraft:item.bucket.empty_powder_snow");
    }
    placed
}

/// The empty-bucket behaviour: `BucketPickup.pickupBlock` in front.
pub(super) fn fill(
    world: &mut ServerWorld,
    source: &BlockSource,
    slot: usize,
    stack: RuntimeItemStack,
) -> CoreResult<Dispensed> {
    let target = source.target();
    let Some(taken) = world.with_level(source.level, |level| {
        let state = level.block_state(target);
        pickup_block(level, target, state, false)
    }) else {
        return world.default_dispense(source, stack);
    };
    world.level_mut_by_id(&source.level).event_sink.game_event(
        VanillaGameEvent::FLUID_PICKUP,
        target,
        None,
    );
    let filled = RuntimeItemStack::from_summary(single_item(taken));
    let stack = world.consume_with_remainder(source, slot, stack, filled)?;
    Ok(Dispensed::new(stack, DISPENSE_SOUND))
}

/// `new ItemStack(item)`.
pub(super) fn single_item(item_id: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        ..ItemStackSummary::empty()
    }
}
