//! `BucketPickup` (`net/minecraft/world/level/block/BucketPickup.java`), the
//! interface that takes the water back out of a cell.
//!

use super::LevelCtx;
use bbb_protocol::block_types::{vanilla_block_type, VanillaBucketPickupOverride};
use bbb_protocol::item_types::{
    VANILLA_ITEM_LAVA_BUCKET_ID, VANILLA_ITEM_POWDER_SNOW_BUCKET_ID, VANILLA_ITEM_WATER_BUCKET_ID,
};
use bbb_protocol::packets::BlockPos;
use bbb_server_types::fluid::FluidFamily;

use crate::fluid::{liquid_block_family, with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::LEVEL_EVENT_PARTICLES_DESTROY_BLOCK;

use super::{air_block_state_id, block_type_of, bool_property, int_property};

const WATERLOGGED: &str = "waterlogged";

/// `LiquidBlock.LEVEL` (`LiquidBlock.java:38`).
const LEVEL: &str = "level";

/// `UPDATE_NEIGHBORS | UPDATE_CLIENTS | UPDATE_IMMEDIATE` (`Block.java:91`,
/// `:92`, `:94`), the flags the three emptying arms write with.
const EMPTY_FLAGS: i32 = 1 | 2 | 8;

/// `BlockState.getBlock() instanceof BucketPickup` and which body it reaches.
fn owner(block_state_id: i32) -> Option<VanillaBucketPickupOverride> {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)?
        .bucket_pickup_override
}

#[must_use]
pub(super) fn is_liquid_block(block_state_id: i32) -> bool {
    owner(block_state_id) == Some(VanillaBucketPickupOverride::LiquidBlock)
}

/// `BucketPickup.pickupBlock(user, level, pos, state)`: the bucket item that
/// came back, if any. `creative_user` is the one thing a user changes
/// (`BarrierBlock.java:88-90`).
pub(crate) fn pickup_block(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    creative_user: bool,
) -> Option<i32> {
    match owner(block_state_id)? {
        VanillaBucketPickupOverride::BarrierBlock if !creative_user => None,
        VanillaBucketPickupOverride::SimpleWaterloggedBlock
        | VanillaBucketPickupOverride::BarrierBlock => {
            if bool_property(block_state_id, WATERLOGGED) != Some(true) {
                return None;
            }
            let drained = with_property(block_state_id, WATERLOGGED, "false")?;
            level.set_block(
                pos,
                drained,
                SET_BLOCK_UPDATE_ALL,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
            if !super::can_survive_at(block_state_id, pos, level) {
                level.destroy_block(pos, true);
            }
            Some(VANILLA_ITEM_WATER_BUCKET_ID)
        }
        VanillaBucketPickupOverride::LiquidBlock => {
            if int_property(block_state_id, LEVEL) != Some(0) {
                return None;
            }
            level.set_block(
                pos,
                air_block_state_id(),
                EMPTY_FLAGS,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
            Some(match liquid_block_family(block_state_id)? {
                FluidFamily::Water => VANILLA_ITEM_WATER_BUCKET_ID,
                FluidFamily::Lava => VANILLA_ITEM_LAVA_BUCKET_ID,
            })
        }
        VanillaBucketPickupOverride::BubbleColumnBlock => {
            level.set_block(
                pos,
                air_block_state_id(),
                EMPTY_FLAGS,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
            Some(VANILLA_ITEM_WATER_BUCKET_ID)
        }
        VanillaBucketPickupOverride::PowderSnowBlock => {
            level.set_block(
                pos,
                air_block_state_id(),
                EMPTY_FLAGS,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
            level.level_event(LEVEL_EVENT_PARTICLES_DESTROY_BLOCK, pos, block_state_id);
            Some(VANILLA_ITEM_POWDER_SNOW_BUCKET_ID)
        }
    }
}

/// `BucketPickup.getPickupSound` (`LiquidBlock.java:260-262`,
/// `PowderSnowBlock.java:158-160`, `WaterFluid.java:127-129`,
/// `LavaFluid.java:231-233`).
pub(crate) fn pickup_sound(block_state_id: i32) -> Option<&'static str> {
    match owner(block_state_id)? {
        VanillaBucketPickupOverride::PowderSnowBlock => {
            Some("minecraft:item.bucket.fill_powder_snow")
        }
        VanillaBucketPickupOverride::LiquidBlock
            if liquid_block_family(block_state_id) == Some(FluidFamily::Lava) =>
        {
            Some("minecraft:item.bucket.fill_lava")
        }
        _ => Some("minecraft:item.bucket.fill"),
    }
}
