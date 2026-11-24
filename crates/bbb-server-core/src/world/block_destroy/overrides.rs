//! The subclass bodies of `Block.playerWillDestroy`, `Block.playerDestroy` and
//! `BlockBehaviour.attack` a real dig reaches, and the base
//! `playerWillDestroy`'s guarded-by-piglins hook.

use crate::block_behavior::campfire::SOUND_EXTINGUISH_FIRE;
use crate::block_behavior::{bed, decorated_pot, door, piston_head, player_break, tripwire};
use crate::block_destroy::has_correct_tool_for_drops;
use crate::block_entity::BlockEntity;
use crate::connection::ConnectionId;
use crate::entity::block_drops::{loot_tool_for_runtime_stack, loot_tool_for_stack};
use crate::entity::spawn_dropped_item_entity;
use crate::level_event::{PendingLevelEvent, LEVEL_EVENT_PARTICLES_DESTROY_BLOCK};
use crate::world::LevelId;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::vanilla_block_type;
use bbb_protocol::item_types::{vanilla_item_type, VANILLA_ITEM_SHEARS_ID};
use bbb_protocol::packets::{BlockPos, ItemStackSummary, Vec3d};
use bbb_server_types::fluid::air_block_state_id;
use bbb_server_types::stats::custom_stat;

use crate::world::{ServerWorld, WorldEvent};

fn centre(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    }
}

impl ServerWorld {
    pub(super) fn player_will_destroy_override(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> i32 {
        let dimension = dimension.into();
        let Some(player) = self.block_interaction_player(connection_id) else {
            return block_state_id;
        };
        let prevents_block_drops = self.player_has_infinite_materials(connection_id);
        let selected_native = self.levels[connection_id]
            .inventory
            .selected_stack()
            .clone();
        let selected = selected_native.summary().clone();
        if door::is_door(block_state_id) {
            if prevents_block_drops
                || !has_correct_tool_for_drops(
                    &selected,
                    block_state_id,
                    self.registries.registry_tags(),
                )
                .unwrap_or(true)
            {
                self.prevent_drop_from_bottom_part(
                    connection_id,
                    dimension,
                    pos,
                    block_state_id,
                    events,
                );
            }
        } else if player_break::is_double_plant_block(block_state_id) {
            if prevents_block_drops {
                self.prevent_drop_from_bottom_part(
                    connection_id,
                    dimension,
                    pos,
                    block_state_id,
                    events,
                );
            } else {
                self.drop_resources_for_player(
                    connection_id,
                    dimension,
                    pos,
                    block_state_id,
                    &selected_native,
                    None,
                );
            }
        } else if decorated_pot::is_decorated_pot(block_state_id) {
            if self.stack_cracks_decorated_pots(dimension, &selected) {
                return self.with_level(dimension, |level| {
                    player_break::crack_decorated_pot(level, pos, block_state_id)
                });
            }
        } else if bed::is_bed(block_state_id) {
            if prevents_block_drops {
                let removed = self.with_level(dimension, |level| {
                    player_break::remove_creative_bed_head(level, pos, block_state_id)
                });
                if let Some((head_pos, head_state)) = removed {
                    self.level_event_past_breaker(
                        connection_id,
                        dimension,
                        head_pos,
                        head_state,
                        events,
                    );
                }
            }
        } else if piston_head::is_piston_head(block_state_id) {
            if prevents_block_drops {
                self.with_level(dimension, |level| {
                    player_break::destroy_creative_piston_base(level, pos, block_state_id);
                });
            }
        } else if crate::block_behavior::beehive::is_beehive(block_state_id) {
            if prevents_block_drops {
                self.drop_creative_beehive(dimension, pos, block_state_id);
            }
        } else if bbb_server_types::block_queries::is_shulker_box(block_state_id) {
            self.shulker_box_player_will_destroy(
                connection_id,
                dimension,
                pos,
                prevents_block_drops,
                events,
            );
        } else if player_break::is_unstable_tnt(block_state_id) {
            if !prevents_block_drops {
                self.prime_tnt(dimension, pos, None, None, events);
            }
        } else if tripwire::is_tripwire(block_state_id) {
            if selected.item_id == Some(VANILLA_ITEM_SHEARS_ID) && selected.count > 0 {
                self.with_level(dimension, |level| {
                    player_break::disarm_tripwire(level, pos, block_state_id, player);
                });
            }
        } else if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(block_state_id)
            .is_some_and(|id| {
                matches!(
                    id,
                    bbb_protocol::block_types::VANILLA_BLOCK_FIRE_ID
                        | bbb_protocol::block_types::VANILLA_BLOCK_SOUL_FIRE_ID
                )
            })
        {
            self.flush_level_events(&dimension, events);
            self.broadcast_level_event_by_key(
                &dimension,
                None,
                PendingLevelEvent {
                    event_type: SOUND_EXTINGUISH_FIRE,
                    pos,
                    data: 0,
                },
                events,
            );
        }
        block_state_id
    }

    fn prevent_drop_from_bottom_part(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        let replaced = self.with_level(dimension, |level| {
            player_break::prevent_drop_from_bottom_part(level, pos, block_state_id)
        });
        if let Some((bottom_pos, bottom_state)) = replaced {
            // `:129`.
            self.level_event_past_breaker(
                connection_id,
                dimension,
                bottom_pos,
                bottom_state,
                events,
            );
        }
    }

    /// `level.levelEvent(player, 2001, pos, Block.getId(state))`, after the
    /// queued effects of the write that preceded it.
    fn level_event_past_breaker(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        self.flush_level_events(&dimension, events);
        self.broadcast_level_event_by_key(
            &dimension,
            Some(connection_id),
            PendingLevelEvent {
                event_type: LEVEL_EVENT_PARTICLES_DESTROY_BLOCK,
                pos,
                data: block_state_id,
            },
            events,
        );
    }

    pub(super) fn drop_resources_for_player(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        tool_stack: &bbb_server_types::item_stack::RuntimeItemStack,
        block_entity: Option<&BlockEntity>,
    ) {
        let dimension = dimension.into();
        let tool =
            loot_tool_for_runtime_stack(tool_stack, &self.datapack_tables.enchantment_registry);
        let tool_summary = tool_stack.summary();
        // An enchantment this server cannot name reads as absent, which would
        // quietly answer a `match_tool` or an `apply_bonus` with the wrong
        // level, so it is recorded rather than shrugged off.
        for holder_id in crate::entity::block_drops::unresolved_tool_enchantments(
            tool_summary,
            &self.datapack_tables.enchantment_registry,
        ) {
            tracing::warn!("the breaking tool carries enchantment holder id {holder_id}, which the loaded enchantment registry has no row for");
        }
        self.drop_resources_with_tool(
            connection_id,
            dimension,
            pos,
            block_state_id,
            &tool,
            block_entity,
        );
        self.spawn_after_break(dimension, pos, block_state_id, tool_summary, true);
    }

    fn stack_cracks_decorated_pots(
        &mut self,
        dimension: impl Into<LevelId>,
        stack: &ItemStackSummary,
    ) -> bool {
        let dimension = dimension.into();
        let Some(item) = stack
            .item_id
            .filter(|_| stack.count > 0)
            .and_then(vanilla_item_type)
        else {
            return false;
        };
        if !self.live_tag_membership(
            dimension,
            "minecraft:item",
            "minecraft:breaks_decorated_pots",
            item.resource_id,
        ) {
            return false;
        }
        self.stack_has_enchantment_in_tag(
            dimension,
            stack,
            "minecraft:prevents_decorated_pot_shattering",
        ) == Some(false)
    }

    pub(super) fn stack_has_enchantment_in_tag(
        &mut self,
        dimension: impl Into<LevelId>,
        stack: &ItemStackSummary,
        tag: &str,
    ) -> Option<bool> {
        let dimension = dimension.into();
        let unresolved = crate::entity::block_drops::unresolved_tool_enchantments(
            stack,
            &self.datapack_tables.enchantment_registry,
        );
        if !unresolved.is_empty() {
            tracing::warn!("{tag} cannot be asked about an unresolved enchantment holder");
            return None;
        }
        let tool = loot_tool_for_stack(stack, &self.datapack_tables.enchantment_registry);
        for enchantment in tool.enchantments.keys() {
            if self.live_tag_membership(dimension, "minecraft:enchantment", tag, enchantment) {
                return Some(true);
            }
        }
        Some(false)
    }

    fn live_tag_membership(
        &self,
        dimension: impl Into<LevelId>,
        registry: &str,
        tag: &str,
        element: &str,
    ) -> bool {
        let dimension = dimension.into();
        self.level_by_id(&dimension)
            .chunks
            .registry_tags()
            .is_some_and(|tags| tags.contains(registry, tag, element))
    }

    pub(crate) fn player_break_angers_piglins(
        &self,
        dimension: impl Into<LevelId>,
        block_state_id: i32,
    ) -> bool {
        let dimension = dimension.into();
        vanilla_block_state_block_type_id(block_state_id)
            .and_then(vanilla_block_type)
            .is_some_and(|block| {
                self.live_tag_membership(
                    dimension,
                    "minecraft:block",
                    "minecraft:guarded_by_piglins",
                    block.resource_id,
                )
            })
    }

    pub(in crate::world) fn anger_nearby_piglins(
        &mut self,
        connection_id: ConnectionId,
        only_if_they_see_the_player: bool,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let dimension = player.dimension;
        let player_id = player.entity_id();
        let difficulty = self.admin.settings().difficulty;
        let universal_anger = self.game_rule_bool("minecraft:universal_anger", false);
        self.with_level_and_entities(dimension, |level, entities| {
            entities.set_difficulty(difficulty);
            entities.set_target_level_facts(crate::entity::TargetLevelFacts {
                universal_anger,
                ..entities.target_level_facts()
            });
            crate::entity::piglin::anger_nearby(
                entities,
                level,
                player_id,
                only_if_they_see_the_player,
            );
        });
    }

    fn shulker_box_player_will_destroy(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        prevents_block_drops: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        if !prevents_block_drops {
            if let Err(error) = self.unpack_menu_container(connection_id, dimension, pos, events) {
                self.with_level(dimension, |level| level.record_item_failure(pos, error));
            }
            return;
        }
        let unpacked = self.with_level(dimension, |level| {
            match level.unpack_container_loot(pos, None) {
                Ok(()) => true,
                Err(error) => {
                    level.record_item_failure(pos, error);
                    false
                }
            }
        });
        if !unpacked {
            return;
        }
        let resolver = self.item_stack_resolver();
        let Some(block_entity) = self
            .level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(pos)
        else {
            return;
        };
        if block_entity
            .shulker_box()
            .is_none_or(|shulker| shulker.is_empty())
        {
            return;
        }
        let Some(stack) = block_entity.shulker_box_drop_stack(resolver.as_dyn()) else {
            return;
        };
        let position = centre(pos);
        let item = match bbb_server_types::item_stack::RuntimeItemStack::try_read(&stack, &resolver)
        {
            Ok(item) => item,
            Err(error) => {
                tracing::warn!("the creative shulker box drop does not decode: {error}");
                return;
            }
        };
        // `:112-115`: `new ItemEntity(level, x + 0.5, y + 0.5, z + 0.5, stack)`,
        // `setDefaultPickUpDelay` and `addFreshEntity`.
        if spawn_dropped_item_entity(
            &mut self.level_mut_by_id(&dimension).entities,
            position,
            item,
        )
        .is_none()
        {
            tracing::warn!("the entity store refused the shulker box item entity's drawn UUID");
        }
    }

    pub(super) fn player_destroy_override(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        destroyed_with: &bbb_server_types::item_stack::RuntimeItemStack,
        block_entity: Option<&BlockEntity>,
    ) {
        let dimension = dimension.into();
        let mined_block = vanilla_block_state_block_type_id(block_state_id);
        if player_break::is_double_plant_block(block_state_id) {
            self.player_destroy(
                connection_id,
                dimension,
                pos,
                (mined_block, air_block_state_id()),
                destroyed_with,
                block_entity,
            );
            return;
        }
        self.player_destroy(
            connection_id,
            dimension,
            pos,
            (mined_block, block_state_id),
            destroyed_with,
            block_entity,
        );
        if player_break::is_ice_block(block_state_id) {
            let Some(prevents_melting) = self.stack_has_enchantment_in_tag(
                dimension,
                destroyed_with.summary(),
                "minecraft:prevents_ice_melting",
            ) else {
                return;
            };
            self.with_level(dimension, |level| {
                player_break::ice_player_destroy(level, pos, prevents_melting);
            });
        } else if player_break::is_turtle_egg(block_state_id) {
            self.with_level(dimension, |level| {
                player_break::decrease_turtle_eggs(level, pos, block_state_id);
            });
        }
    }

    pub(super) fn attack_block(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
    ) {
        let dimension = dimension.into();
        if player_break::is_dragon_egg(block_state_id) {
            self.use_dragon_egg_without_item(dimension, pos, block_state_id);
            return;
        }
        let Some(player) = self.block_interaction_player(connection_id) else {
            return;
        };
        let played_note = self.with_level(dimension, |level| {
            player_break::attack(level, pos, block_state_id, player)
        });
        if played_note {
            if let Some(player) = self.levels.player_mut(connection_id) {
                player.award_stat_once(
                    &mut self.scoreboard,
                    custom_stat("minecraft:play_noteblock"),
                );
            }
        }
    }
}
