//! The shared cave-vine harvest action for players and mobs.

use super::{DropBlockEntity, DropTable, LevelCtx};
use crate::block_behavior::{growing_plant, SET_BLOCK_UPDATE_CLIENTS};
use crate::level_event::VanillaGameEvent;
use crate::loot_runtime::{LootEntity, LootTool};
use bbb_protocol::packets::BlockPos;

impl LevelCtx<'_, '_> {
    // CaveVines.java:23-46.
    pub(crate) fn harvest_cave_vines(
        &mut self,
        pos: BlockPos,
        source_id: i32,
        source: LootEntity<'_>,
    ) -> bool {
        let Some(state) = self
            .block_state_checked(pos)
            .filter(|state| growing_plant::has_berries(*state))
        else {
            return false;
        };
        let Some(next) = growing_plant::with_berries(state, false) else {
            return false;
        };
        self.drop_block_loot_at(
            pos,
            state,
            DropTable::Named("minecraft:harvest/cave_vine"),
            DropBlockEntity::Live,
            &LootTool::empty(),
            source,
        );
        let pitch = 0.8 + self.level_random().next_float() * 0.4;
        self.play_block_sound_with_volume_pitch(
            pos,
            "minecraft:block.cave_vines.pick_berries",
            1.0,
            pitch,
        );
        self.set_block(
            pos,
            next,
            SET_BLOCK_UPDATE_CLIENTS,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        self.game_event_with_state(
            VanillaGameEvent::BLOCK_CHANGE,
            pos,
            Some(source_id),
            Some(next),
        );
        true
    }
}
