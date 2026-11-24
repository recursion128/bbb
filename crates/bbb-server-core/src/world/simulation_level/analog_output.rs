//! Exhaustive `BlockState.getAnalogOutputSignal` (`BlockBehaviour.java:635-637`)
//! dispatch over every [`VanillaBlockAnalogOutputOverride`] owner.

use super::*;

pub(super) fn analog_output_signal(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    direction: Direction,
) -> i32 {
    let state = level.block_state(pos);
    let Some(owner) = bbb_server_types::block_queries::block_type_of(state)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.analog_output_override)
    else {
        return 0;
    };
    if let Some(output) = redstone::state_only_analog_output(state) {
        return output;
    }
    match owner {
        VanillaBlockAnalogOutputOverride::BlockBehaviour
        | VanillaBlockAnalogOutputOverride::LayeredCauldronBlock
        | VanillaBlockAnalogOutputOverride::LavaCauldronBlock
        | VanillaBlockAnalogOutputOverride::CandleCakeBlock
        | VanillaBlockAnalogOutputOverride::EndPortalFrameBlock
        | VanillaBlockAnalogOutputOverride::RespawnAnchorBlock
        | VanillaBlockAnalogOutputOverride::CakeBlock
        | VanillaBlockAnalogOutputOverride::ComposterBlock
        | VanillaBlockAnalogOutputOverride::CopperBulbBlock
        | VanillaBlockAnalogOutputOverride::BeehiveBlock
        | VanillaBlockAnalogOutputOverride::CopperGolemStatueBlock => 0,
        VanillaBlockAnalogOutputOverride::LecternBlock => {
            if !lectern_block::has_book(state) {
                return 0;
            }
            let Some(lectern) = level
                .chunks
                .block_entities()
                .get(pos)
                .and_then(BlockEntity::lectern)
            else {
                return 0;
            };
            match lectern.redstone_signal() {
                Ok(signal) => signal,
                Err(error) => {
                    level.record_item_failure(pos, error);
                    0
                }
            }
        }
        VanillaBlockAnalogOutputOverride::ChiseledBookShelfBlock => level
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::chiseled_bookshelf)
            .map_or(
                0,
                chiseled_bookshelf::ChiseledBookShelfBlockEntity::redstone_signal,
            ),
        VanillaBlockAnalogOutputOverride::ShelfBlock => level
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::shelf)
            .map_or(0, |shelf| {
                crate::block_behavior::shelf::analog_output(
                    state,
                    direction,
                    shelf.original_items(),
                )
            }),
        // `CrafterBlock.getAnalogOutputSignal` (`CrafterBlock.java:68-70`).
        VanillaBlockAnalogOutputOverride::CrafterBlock => level
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::crafter)
            .map_or(0, crafter::CrafterBlockEntity::redstone_signal),
        // `CommandBlock.getAnalogOutputSignal` (`CommandBlock.java:146-149`).
        VanillaBlockAnalogOutputOverride::CommandBlock => level
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::command_block)
            .map_or(0, command_block::CommandBlockEntity::success_count),
        // `JukeboxBlock.getAnalogOutputSignal` (`JukeboxBlock.java:110-112`)
        // over `JukeboxBlockEntity.getComparatorOutput` (`:68-70`).
        VanillaBlockAnalogOutputOverride::JukeboxBlock => level
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::jukebox)
            .and_then(|jukebox| jukebox.comparator_output(level.item_stack_resolver()).ok())
            .unwrap_or(0),
        // `SculkSensorBlock.getAnalogOutputSignal`
        // (`SculkSensorBlock.java:269-275`): an inactive sensor answers
        // zero without reading its frequency.
        VanillaBlockAnalogOutputOverride::SculkSensorBlock => {
            if sculk_sensor_block::is_active(state) {
                level
                    .chunks
                    .block_entities()
                    .get(pos)
                    .and_then(BlockEntity::sculk_sensor)
                    .map_or(0, |sensor| sensor.last_vibration_frequency())
            } else {
                0
            }
        }
        VanillaBlockAnalogOutputOverride::HopperBlock
        | VanillaBlockAnalogOutputOverride::BarrelBlock
        | VanillaBlockAnalogOutputOverride::DispenserBlock
        | VanillaBlockAnalogOutputOverride::ShulkerBoxBlock
        | VanillaBlockAnalogOutputOverride::DecoratedPotBlock
        | VanillaBlockAnalogOutputOverride::ChestBlock
        | VanillaBlockAnalogOutputOverride::AbstractFurnaceBlock
        | VanillaBlockAnalogOutputOverride::BrewingStandBlock => level.container_analog_output(pos),
        VanillaBlockAnalogOutputOverride::DetectorRailBlock => {
            crate::block_behavior::rail::detector_analog_output(level, pos, state)
        }
        VanillaBlockAnalogOutputOverride::CreakingHeartBlock => level
            .creaking_heart_analog(pos)
            .map_or(0, |heart| heart.output_signal),
    }
}
