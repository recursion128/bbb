//! `BeehiveBlock` (`BeehiveBlock.java`): the honeycomb shears take and the
//! honey bottle a glass bottle fills at full honey.

use crate::world::LevelId;
use bbb_protocol::packets::{
    BlockPos, DataComponentPatchSummary, InteractionHand, ItemStackSummary, SoundEvent,
    SoundEventHolder, SoundSource,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::stats::StatKey;

use super::BlockUseContext;
use crate::block_behavior::beehive as beehive_block;
use crate::block_behavior::BlockUseResult;
use crate::connection::ConnectionId;
use crate::entity::block_drops::loot_tool_for_runtime_stack;
use crate::level_event::VanillaGameEvent;
use crate::loot_runtime::LootEntity;
use crate::world::simulation_level::block_entity_loot::{DropBlockEntity, DropTable};
use crate::world::{ServerWorld, WorldEvent};

/// The `item_on:` column of the `block_use_dispatch!` row:
/// [`ServerWorld::use_beehive_item_on`].
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> BlockUseResult {
    world.use_beehive_item_on(context, events)
}

impl ServerWorld {
    /// `BeehiveBlock.useItemOn` (`BeehiveBlock.java:150-202`).
    pub(super) fn use_beehive_item_on(
        &mut self,
        context: &BlockUseContext<'_>,
        events: &mut Vec<WorldEvent>,
    ) -> BlockUseResult {
        let connection_id = context.connection_id;
        let dimension = context.dimension;
        let pos = context.update.hit.pos;
        let block_state_id = context.block_state_id;
        let selected_item = &context.selected_item;
        let hand = context.update.hand;
        if beehive_block::honey_level(block_state_id) < beehive_block::MAX_HONEY_LEVELS {
            return BlockUseResult::TryWithEmptyHand;
        }
        if beehive_block::item_is_shears(selected_item.item_id()) {
            if !self.harvest_beehive_with_shears(context, events) {
                return BlockUseResult::Pass;
            }
            self.award_beehive_item_used(connection_id, selected_item);
            return BlockUseResult::Consumed;
        }
        if beehive_block::item_is_glass_bottle(selected_item.item_id()) {
            if !self.harvest_beehive_with_bottle(
                connection_id,
                dimension,
                pos,
                block_state_id,
                hand,
                events,
            ) {
                return BlockUseResult::TryWithEmptyHand;
            }
            self.award_beehive_item_used(connection_id, selected_item);
            return BlockUseResult::Consumed;
        }
        BlockUseResult::TryWithEmptyHand
    }

    pub(super) fn harvest_beehive_with_shears(
        &mut self,
        context: &BlockUseContext<'_>,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let connection_id = context.connection_id;
        let dimension = context.dimension;
        let pos = context.update.hit.pos;
        let block_state_id = context.block_state_id;
        let selected_item = &context.selected_item;
        let hand = context.update.hand;
        let tool =
            loot_tool_for_runtime_stack(selected_item, &self.datapack_tables.enchantment_registry);
        let interacting = self.loot_view_for_player(self.levels.player_at(connection_id));
        self.with_level(dimension, |level| {
            level.drop_block_loot_at(
                pos,
                block_state_id,
                DropTable::Named(beehive_block::HARVEST_LOOT_TABLE),
                DropBlockEntity::Live,
                &tool,
                LootEntity::Facts(&interacting),
            );
        });
        self.play_beehive_sound(
            connection_id,
            dimension,
            beehive_block::SHEAR_SOUND,
            None,
            events,
        );
        // `BeehiveBlock.java:166`.
        self.hurt_and_break_in_hand(connection_id, hand, 1, events);
        // `BeehiveBlock.java:168`.
        self.beehive_player_event(connection_id, dimension, pos, VanillaGameEvent::SHEAR);
        self.finish_beehive_harvest(connection_id, dimension, pos, block_state_id, events);
        true
    }

    pub(super) fn harvest_beehive_with_bottle(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        hand: InteractionHand,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let resolver = self.item_stack_resolver();
        let dimension = dimension.into();
        let honey = ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_HONEY_BOTTLE_ID),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        };
        let result = (|| {
            let mut honey = bbb_server_types::item_stack::RuntimeItemStack::from_summary(honey);
            let player = self
                .levels
                .player_mut(connection_id)
                .expect("connection presence was checked");
            let instabuild = player.abilities.instabuild;
            let inventory = &mut player.inventory;
            inventory.consume_in_hand(hand, 1, false);
            if inventory.item_in_hand_mut(hand).is_empty() {
                inventory.set_item_in_hand(hand, honey);
                honey = bbb_server_types::item_stack::RuntimeItemStack::empty();
            } else {
                inventory.add_anywhere(&resolver, &mut honey, instabuild)?;
            }
            Ok::<_, bbb_server_types::ServerWorldError>(honey)
        })();
        let leftover = match result {
            Ok(leftover) => leftover,
            Err(error) => {
                tracing::warn!("beehive bottle output: {error}");
                return false;
            }
        };
        self.play_beehive_sound(
            connection_id,
            dimension,
            beehive_block::BOTTLE_FILL_SOUND,
            Some(connection_id),
            events,
        );
        if !leftover.is_empty() {
            self.drop_player_stack(connection_id, &leftover, false);
        }
        // `BeehiveBlock.java:179`.
        self.beehive_player_event(
            connection_id,
            dimension,
            pos,
            VanillaGameEvent::FLUID_PICKUP,
        );
        self.finish_beehive_harvest(connection_id, dimension, pos, block_state_id, events);
        true
    }

    /// `BeehiveBlock.java:182-184`.
    fn award_beehive_item_used(
        &mut self,
        connection_id: ConnectionId,
        item: &bbb_server_types::item_stack::RuntimeItemStack,
    ) {
        if let Some(item_id) = item.item_id() {
            self.levels[connection_id].award_stat_once(
                &mut self.scoreboard,
                StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
            );
        }
    }

    fn finish_beehive_harvest(
        &mut self,
        player: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        state: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        let smokey = self.is_beehive_smokey(dimension, pos);
        let chunks = &self.level_by_id(&dimension).chunks;
        if !smokey {
            if chunks
                .block_entities()
                .get(pos)
                .and_then(crate::block_entity::BlockEntity::beehive)
                .is_some_and(|hive| !hive.is_empty())
            {
                self.anger_bees_near_hive(dimension, pos);
            }
            self.release_beehive_emergency_occupants(
                dimension,
                pos,
                state,
                Some(player),
                false,
                events,
            );
        }
        self.reset_beehive_honey(dimension, pos, state);
    }

    pub(in crate::world) fn is_beehive_smokey(
        &self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
    ) -> bool {
        let dimension = dimension.into();
        let chunks = &self.level_by_id(&dimension).chunks;
        let tags = chunks.registry_tags_shared().map_or(
            crate::entity::EntityRegistryTagView::Builtin26_1,
            crate::entity::EntityRegistryTagView::Frozen,
        );
        crate::block_behavior::campfire::is_smokey_pos::<std::convert::Infallible>(
            pos,
            |at| Ok(super::super::level_blocks::stored_block_state(chunks, at)),
            |state, _| {
                // `CampfireBlock.java:268` queries the lower state at the hive's position.
                Ok(
                    super::super::movement_collision::block_state_collision_shape_checked(
                        chunks, state, pos, None,
                    )
                    .is_none_or(|boxes| {
                        boxes
                            .into_iter()
                            .any(crate::block_behavior::campfire::smoke_post_intersects)
                    }),
                )
            },
            |name| Ok(tags.contains("minecraft:block", "minecraft:campfires", name)),
        )
        .expect("infallible level query")
    }

    /// `BeehiveBlock.angerNearbyBees` (`BeehiveBlock.java:118-133`).
    /// Default entity queries exclude spectators (`EntityGetter.java:50-51`).
    pub(in crate::world) fn anger_bees_near_hive(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
    ) {
        let dimension = dimension.into();
        use crate::entity::ai::mob::target::TargetRef;
        let area = bbb_server_types::geometry::Aabb::of_block(pos).inflate(8.0, 6.0, 8.0);
        let entities = &self.level_by_id(&dimension).entities;
        let players: Vec<_> = entities
            .players()
            .filter(|player| {
                !player.is_spectator()
                    && player
                        .bounding_box()
                        .is_ok_and(|bounds| bounds.intersects(area))
            })
            .map(|player| TargetRef {
                entity_id: player.entity_id(),
                uuid: player.uuid(),
            })
            .collect();
        if players.is_empty() {
            return;
        }
        let bees: Vec<_> = entities
            .ids_in_box_sections(area)
            .into_iter()
            .filter(|id| {
                entities.get(*id).is_some_and(|entity| {
                    entity.entity_type_id()
                        == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BEE_ID
                        && entity.bounding_box().intersects(area)
                }) && entities
                    .mob_ai(*id)
                    .is_some_and(|runtime| runtime.target_ref().is_none())
            })
            .collect();
        for bee in bees {
            let target = players[self.random.next_int(players.len() as i32) as usize];
            self.set_beehive_player_target(dimension, bee, target);
        }
    }

    fn beehive_player_event(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        game_event: VanillaGameEvent,
    ) {
        let dimension = dimension.into();
        let source_entity_id = self
            .levels
            .player(connection_id)
            .map(|player| player.entity_id());
        self.level_mut_by_id(&dimension)
            .event_sink
            .game_event_with_source(game_event, pos, source_entity_id, None);
    }

    pub(super) fn reset_beehive_honey(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
    ) {
        let dimension = dimension.into();
        let Some(next) = beehive_block::reset_honey_state(block_state_id) else {
            return;
        };
        self.set_block(dimension, pos, next, crate::fluid::SET_BLOCK_UPDATE_ALL);
    }

    pub(super) fn play_beehive_sound(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        sound_resource_id: &str,
        except: Option<ConnectionId>,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        let Some(registry_id) = vanilla_sound_event_registry_id(sound_resource_id) else {
            return;
        };
        let position = self
            .levels
            .player(connection_id)
            .map(|player| player.position())
            .unwrap_or_default();
        let sound = SoundEvent {
            sound: SoundEventHolder::Reference { registry_id },
            source: SoundSource::Blocks,
            position,
            volume: 1.0,
            pitch: 1.0,
            seed: self.random.next_long(),
        };
        self.broadcast_level_sound_by_key(&dimension, &sound, except, events);
    }
}

#[cfg(test)]
mod tests;
