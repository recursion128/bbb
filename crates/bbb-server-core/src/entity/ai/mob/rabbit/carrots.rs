//! Rabbit navigation toward ripe carrots and authoritative consumption requests.

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::VANILLA_BLOCK_CARROTS_ID;
use bbb_protocol::packets::BlockPos;

use super::super::target::{TargetRef, TargetView};
use crate::block_behavior::crop;
use crate::entity::rabbit::PendingCarrotRaid;
use crate::entity::{EntityCommand, EntityData};
use crate::world::simulation_level::LevelCtx;

pub(in crate::entity::ai::mob) fn hungry(targets: &TargetView<'_, '_, '_>) -> bool {
    matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
        if data.rabbit_retained().is_some_and(|state| state.more_carrot_ticks() <= 0))
}

pub(in crate::entity::ai::mob) fn ripe_carrot(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    let state = level.block_state(pos);
    level.has_ticking_column_at(pos)
        && vanilla_block_state_block_type_id(state) == Some(VANILLA_BLOCK_CARROTS_ID)
        && crop::is_max_age(state)
        && level.block_state_in_tag(
            level.block_state(BlockPos {
                y: pos.y - 1,
                ..pos
            }),
            "minecraft:supports_crops",
        )
}

pub(in crate::entity::ai::mob) fn raid_carrot(targets: &mut TargetView<'_, '_, '_>, pos: BlockPos) {
    if targets.level().mob_griefing() && hungry(targets) && ripe_carrot(targets.level(), pos) {
        let rabbit = TargetRef {
            entity_id: targets.body_entity().id(),
            uuid: targets.body_entity().uuid(),
        };
        targets.queue_command(EntityCommand::RabbitRaidCarrot(PendingCarrotRaid {
            rabbit,
            pos,
        }));
    }
}
