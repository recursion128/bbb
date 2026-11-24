//! `TntBlock` (`TntBlock.java`): lighting a TNT block with flint and steel or a
//! fire charge, which primes it into a [`crate::entity::tnt`] entity.

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::VANILLA_BLOCK_TNT_ID;
use bbb_protocol::item_types::{VANILLA_ITEM_FIRE_CHARGE_ID, VANILLA_ITEM_FLINT_AND_STEEL_ID};
use bbb_protocol::packets::{BlockPos, SoundEvent, SoundEventHolder, SoundSource, Vec3d};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use uuid::Uuid;

use super::BlockUseContext;
use crate::block_behavior::BlockUseResult;
use crate::entity::spawn_tnt;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use crate::neighbor_updater::UPDATE_LIMIT;
use crate::world::{LevelId, ServerWorld, WorldEvent};
use bbb_server_types::stats::StatKey;

/// `TntBlock.prime` plays `SoundEvents.TNT_PRIMED` (`TntBlock.java:94`).
const TNT_PRIMED_SOUND: &str = "minecraft:entity.tnt.primed";

/// `Block.UPDATE_ALL_IMMEDIATE` (`Block.java:106`), the flags `TntBlock.useItemOn`
/// removes the block with (`TntBlock.java:115`).
const SET_BLOCK_UPDATE_ALL_IMMEDIATE: i32 = 11;

/// The `matches:` column of the `block_use_dispatch!` row: `Blocks.TNT`.
pub(super) fn is_tnt(block_state_id: i32) -> bool {
    vanilla_block_state_block_type_id(block_state_id) == Some(VANILLA_BLOCK_TNT_ID)
}

/// The `item_on:` column: [`ServerWorld::use_tnt_item_on`].
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> BlockUseResult {
    world.use_tnt_item_on(context, events)
}

impl ServerWorld {
    pub(in crate::world) fn spawn_primed_tnt(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        source: Option<Uuid>,
    ) -> Option<Vec3d> {
        let dimension = dimension.into();
        if !self.game_rule_bool("minecraft:tnt_explodes", true) {
            return None;
        }
        let position = Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y),
            z: f64::from(pos.z) + 0.5,
        };
        let level = self.level_mut_by_id(&dimension);
        spawn_tnt(&mut level.entities, &level.random, position, source)?;
        Some(position)
    }

    /// `TntBlock.useItemOn` (`TntBlock.java:101-130`).
    pub(super) fn use_tnt_item_on(
        &mut self,
        context: &BlockUseContext<'_>,
        events: &mut Vec<WorldEvent>,
    ) -> BlockUseResult {
        let item_id = context.selected_item.item_id();
        let is_flint_and_steel = item_id == Some(VANILLA_ITEM_FLINT_AND_STEEL_ID);
        let is_fire_charge = item_id == Some(VANILLA_ITEM_FIRE_CHARGE_ID);
        // `:110-112`: anything else falls to `super.useItemOn`, whose default
        // is `TRY_WITH_EMPTY_HAND` (`BlockBehaviour.java:788`).
        if !is_flint_and_steel && !is_fire_charge {
            return BlockUseResult::TryWithEmptyHand;
        }

        let connection_id = context.connection_id;
        let dimension = context.dimension;
        let pos = context.update.hit.pos;
        let hand = context.update.hand;
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        let source_entity_id = player.entity_id();
        let source = player.profile.uuid;

        // `:114`: `prime(level, pos, player)`.
        if self.prime_tnt(dimension, pos, Some(source), Some(source_entity_id), events) {
            // `:115`: `setBlock(pos, AIR, 11)`.
            let air = self.level_by_id(&dimension).chunks.air_state_id();
            self.with_level(dimension, |level| {
                level.set_block(pos, air, SET_BLOCK_UPDATE_ALL_IMMEDIATE, UPDATE_LIMIT);
            });
            // `:116-121`: flint and steel takes a durability point, a fire
            // charge is consumed.
            let used_item_id = item_id.expect("the branch already read the item id");
            if is_flint_and_steel {
                self.hurt_and_break_in_hand(connection_id, hand, 1, events);
            } else if let Some(player) = self.levels.player_mut(connection_id) {
                player
                    .inventory
                    .consume_in_hand(hand, 1, player.abilities.instabuild);
            }
            // `:122`: `awardStat(Stats.ITEM_USED.get(item))`.
            if let Some(player) = self.levels.player_mut(connection_id) {
                player.award_stat_once(
                    &mut self.scoreboard,
                    StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, used_item_id),
                );
            }
            return BlockUseResult::Consumed;
        }

        // `:123-126`.
        if !self.game_rule_bool("minecraft:tnt_explodes", true) {
            self.sleep_problem(connection_id, "block.minecraft.tnt.disabled", events);
        }
        BlockUseResult::Pass
    }

    /// `TntBlock.prime(level, pos, source)` (`TntBlock.java:84-98`): spawns a
    /// primed TNT, plays its fuse sound and raises `PRIME_FUSE`, when
    /// `TNT_EXPLODES` is on. `source` is the lighting entity, resolved to a
    /// game-event source id for the world half.
    pub(in crate::world) fn prime_tnt(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        source: Option<Uuid>,
        source_entity_id: Option<i32>,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        // `:85-87`: the rule and entity publication precede every effect.
        let Some(position) = self.spawn_primed_tnt(dimension, pos, source) else {
            return false;
        };
        // `:88`: `level.playSound(null, tnt.getX(), tnt.getY(), tnt.getZ(),
        // SoundEvents.TNT_PRIMED, SoundSource.BLOCKS, 1.0F, 1.0F)`.
        if let Some(registry_id) = vanilla_sound_event_registry_id(TNT_PRIMED_SOUND) {
            let seed = self.random.next_long();
            let sound = SoundEvent {
                sound: SoundEventHolder::Reference { registry_id },
                source: SoundSource::Blocks,
                position,
                volume: 1.0,
                pitch: 1.0,
                seed,
            };
            self.broadcast_level_sound_by_key(&dimension, &sound, None, events);
        }
        // `:89`: `level.gameEvent(source, GameEvent.PRIME_FUSE, pos)`.
        self.level_mut_by_id(&dimension)
            .event_sink
            .game_event_with_source(VanillaGameEvent::PRIME_FUSE, pos, source_entity_id, None);
        true
    }

    /// Redstone's null-source `prime` followed by its caller's
    /// `removeBlock(pos, false)` (`TntBlock.java:48-63`, `:84-98`).
    pub(crate) fn prime_tnt_from_redstone(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
    ) -> bool {
        let dimension = dimension.into();
        if !self.block_state(dimension, pos).is_some_and(is_tnt) {
            return false;
        }
        let Some(_position) = self.spawn_primed_tnt(dimension, pos, None) else {
            return false;
        };
        {
            let level = self.level_mut_by_id(&dimension);
            level
                .event_sink
                .play_block_sound_with_volume_pitch(pos, TNT_PRIMED_SOUND, 1.0, 1.0);
            level
                .event_sink
                .game_event(VanillaGameEvent::PRIME_FUSE, pos, None);
        }
        let air = self.level_by_id(&dimension).chunks.air_state_id();
        self.set_block(dimension, pos, air, SET_BLOCK_UPDATE_ALL)
    }
}

#[cfg(test)]
#[path = "tnt_tests.rs"]
mod tests;
