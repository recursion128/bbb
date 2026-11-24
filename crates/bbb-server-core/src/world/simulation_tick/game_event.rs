//! World delivery to registered block and entity game-event listeners.

use crate::world::LevelId;
#[cfg(test)]
use bbb_protocol::packets::BlockPos;
use bbb_protocol::packets::Vec3d;

use crate::block_entity::game_event::{
    block_pos_center, block_pos_dist_sqr, containing_block_pos, game_event_context,
    game_event_notification_radius, game_event_section_bounds, DeliveryMode, GameEventContext,
    GameEventListenerEntry, GameEventListenerOwner,
};

use super::vibration::WorldVibrationReads;
use crate::level_event::{PositionedGameEvent, VanillaGameEvent};
#[cfg(test)]
use bbb_server_types::save::Java26BuiltinDimension;

use super::{ServerWorld, WorldEvent};

impl ServerWorld {
    pub(in crate::world) fn post_pending_game_events(&mut self, dimension: &LevelId) {
        self.sync_entity_game_event_listeners(dimension);
        let pending = self
            .level_mut_by_id(dimension)
            .event_sink
            .drain_positioned_game_events();
        if !pending.is_empty() {
            let mut events = std::mem::take(&mut self.deferred_game_event_events);
            for raised in pending {
                self.post_game_event(dimension, raised, &mut events);
            }
            self.deferred_game_event_events = events;
        }
        self.flush_pending_death_experience(dimension);
    }

    fn flush_pending_death_experience(&mut self, dimension: &LevelId) {
        let pending = self
            .level_mut_by_id(dimension)
            .event_sink
            .drain_death_experience();
        for experience in pending {
            if let Some(player) = self
                .level_by_id(dimension)
                .entities
                .player(experience.entity_id)
            {
                let amount = if player.cold.skip_drop_experience {
                    0
                } else {
                    self.player_experience_reward(player)
                };
                let level = self.level_mut_by_id(dimension);
                crate::entity::award_experience(
                    &mut level.entities,
                    &level.random,
                    experience.position,
                    amount,
                );
                continue;
            }
            let consumed = self
                .level_by_id(dimension)
                .entities
                .get(experience.entity_id)
                .and_then(crate::entity::Entity::living)
                .is_some_and(crate::entity::LivingEntityState::was_experience_consumed);
            if consumed {
                continue;
            }
            let Some(amount) = self.mob_experience_reward(
                dimension,
                experience.entity_id,
                experience.killer_entity_id,
            ) else {
                continue;
            };
            let level = self.level_mut_by_id(dimension);
            // `ExperienceOrb.awardWithDirection`'s `while (amount > 0)`
            if amount <= 0 {
                continue;
            }
            crate::entity::award_experience(
                &mut level.entities,
                &level.random,
                experience.position,
                amount,
            );
        }
    }

    /// `LivingEntity.getExperienceReward(level, killer)` (`LivingEntity.java:581-583`).
    pub(in crate::world) fn mob_experience_reward(
        &mut self,
        dimension: &LevelId,
        entity_id: i32,
        killer_entity_id: Option<i32>,
    ) -> Option<i32> {
        let amount = self
            .level_mut_by_id(dimension)
            .entities
            .experience_reward(entity_id)?;
        let Some(killer_id) = killer_entity_id else {
            return Some(amount);
        };
        let table = std::sync::Arc::clone(&self.datapack_tables.enchantment_mob_experience);
        if table.iter().all(Option::is_none) {
            return Some(amount);
        }
        let store = &self.level_by_id(dimension).entities;
        let killed = store.get(entity_id)?;
        let bodies = crate::entity::store::EntityBodyReadView::new(store, None, None);
        let facts;
        let killer = if let Some(player) = store.player(killer_id) {
            facts = crate::world::loot_context::player_loot_facts(
                player,
                &self.datapack_tables.enchantment_registry,
                &self.datapack_tables.enchantment_slots,
            );
            crate::loot_runtime::LootEntity::Facts(&facts)
        } else {
            match store
                .get(killer_id)
                .filter(|killer| killer.living().is_some())
            {
                Some(killer) => {
                    crate::loot_runtime::LootEntity::Stored(crate::loot_runtime::StoredLootEntity {
                        entity: killer,
                        bodies: Some(bodies),
                        raid_at_position: None,
                    })
                }
                None => return Some(amount),
            }
        };
        Some(crate::entity::experience::process_mob_experience(
            &table,
            &self.datapack_tables.enchantment_registry,
            killer,
            crate::item_durability::ValueContext::stored(killed, Some(bodies)),
            amount,
            store.random(),
            &self.value_effect_level(dimension),
        ))
    }

    fn post_game_event(
        &mut self,
        dimension: &LevelId,
        raised: PositionedGameEvent,
        events: &mut Vec<WorldEvent>,
    ) {
        let recorded = raised.recorded;
        let game_event = recorded.game_event;
        let position = raised.position;
        let context = game_event_context(recorded);
        let source = containing_block_pos(position);
        let bounds = game_event_section_bounds(
            source,
            game_event_notification_radius(game_event.resource_id()),
        );
        let mut by_distance: Vec<(GameEventListenerEntry, f64)> = Vec::new();
        let mut applicable = false;
        for chunk_x in bounds.min_x..=bounds.max_x {
            for chunk_z in bounds.min_z..=bounds.max_z {
                // column that is not loaded at FULL contributes nothing.
                if !self
                    .level_by_id(dimension)
                    .chunks
                    .is_ticking_full(chunk_x, chunk_z)
                {
                    continue;
                }
                for section_y in bounds.min_y..=bounds.max_y {
                    let Some(section) = self
                        .level_mut_by_id(dimension)
                        .chunks
                        .block_entities_mut()
                        .game_event_listeners_mut()
                        .take_section((chunk_x, chunk_z), section_y)
                    else {
                        continue;
                    };
                    for &listener in section.listeners() {
                        let listener_position = match listener.owner {
                            GameEventListenerOwner::BlockEntity => {
                                Some(block_pos_center(listener.pos))
                            }
                            GameEventListenerOwner::Entity { entity_id, .. } => {
                                self.entity_listener_position(dimension, entity_id)
                            }
                        };
                        let Some(listener_position) = listener_position else {
                            continue;
                        };
                        let radius = f64::from(listener.radius);
                        let reaches =
                            block_pos_dist_sqr(containing_block_pos(listener_position), source)
                                <= radius * radius;
                        if !reaches {
                            continue;
                        }
                        applicable = true;
                        if listener.delivery_mode == DeliveryMode::ByDistance {
                            by_distance
                                .push((listener, distance_to_sqr(position, listener_position)));
                        } else {
                            self.deliver_game_event(
                                dimension, listener, game_event, context, position, events,
                            );
                        }
                    }
                    self.level_mut_by_id(dimension)
                        .chunks
                        .block_entities_mut()
                        .game_event_listeners_mut()
                        .restore_section((chunk_x, chunk_z), section_y, section);
                }
            }
        }

        by_distance.sort_by(|left, right| left.1.total_cmp(&right.1));
        for (listener, _) in by_distance {
            self.deliver_game_event(dimension, listener, game_event, context, position, events);
        }

        // the `GAME_EVENTS` subscribers tracking the event's column.
        if applicable {
            self.broadcast_game_event_debug(dimension, game_event, position, events);
        }
    }

    fn deliver_game_event(
        &mut self,
        dimension: &LevelId,
        listener: GameEventListenerEntry,
        game_event: VanillaGameEvent,
        context: GameEventContext,
        position: Vec3d,
        events: &mut Vec<WorldEvent>,
    ) {
        if let GameEventListenerOwner::Entity { entity_id, kind } = listener.owner {
            self.deliver_entity_game_event(
                dimension, listener, entity_id, kind, game_event, context, position, events,
            );
            return;
        }
        let Some(mut block_entity) = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .take(listener.pos)
        else {
            return;
        };
        {
            let mut level = WorldVibrationReads {
                world: self,
                dimension,
                listener_pos: listener.pos,
                events,
            };
            if let Some(mut view) = block_entity.game_event_listener_mut() {
                view.handle_game_event(&mut level, game_event, context, position);
            }
        }
        self.put_back_block_entity(dimension, block_entity);
    }

    /// The listeners one position's section holds, for the tests that drive
    /// registration through a real block write.
    #[cfg(test)]
    pub(crate) fn game_event_listener_section_for_test(
        &self,
        dimension: Java26BuiltinDimension,
        pos: BlockPos,
    ) -> Option<Vec<GameEventListenerEntry>> {
        self.level(dimension)
            .chunks
            .block_entities()
            .game_event_listeners()
            .section((pos.x >> 4, pos.z >> 4), pos.y >> 4)
            .map(|section| section.listeners().to_vec())
    }

    #[cfg(test)]
    pub(crate) fn positioned_game_events_for_test(
        &self,
        dimension: Java26BuiltinDimension,
    ) -> Vec<PositionedGameEvent> {
        self.level(dimension)
            .event_sink
            .recent_positioned_game_events()
            .copied()
            .collect()
    }

    /// Stands a block entity up without going through a block write, which is
    /// the only way a test can put one in a column that is not loaded.
    #[cfg(test)]
    pub(crate) fn add_block_entity_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        pos: BlockPos,
        block_state_id: i32,
    ) {
        let Some(block_entity) = crate::block_entity::new_block_entity(pos, block_state_id) else {
            return;
        };
        self.level_mut(dimension)
            .chunks
            .block_entities_mut()
            .add_and_register(block_entity);
    }

    #[cfg(test)]
    pub(crate) fn record_game_event_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        game_event: VanillaGameEvent,
        pos: BlockPos,
        source_entity_id: Option<i32>,
        affected_state: Option<i32>,
    ) {
        self.level_mut(dimension).event_sink.game_event_with_source(
            game_event,
            pos,
            source_entity_id,
            affected_state,
        );
    }
}

/// `Vec3.distanceToSqr` between the event's position and the listener's.
fn distance_to_sqr(position: Vec3d, listener: Vec3d) -> f64 {
    let dx = position.x - listener.x;
    let dy = position.y - listener.y;
    let dz = position.z - listener.z;
    dx * dx + dy * dy + dz * dz
}
