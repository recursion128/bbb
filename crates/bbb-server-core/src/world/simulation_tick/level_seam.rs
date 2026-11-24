//! Gameplay context construction for callers outside the tick.

use super::*;
use crate::entity::enchanted_damage::DamageImmunityReader as CallerDamageImmunityReader;
use crate::world::block_interaction::BlockInteractionPolicy;
use crate::world::simulation_level::{LevelCtx, LevelFrame, LevelRest};
use crate::world::LevelId;

impl ServerWorld {
    /// The level seam one stored hurt outside the entity loop runs against,
    /// with the level work it queued applied before this returns.
    pub(in crate::world) fn with_stored_hurt_level<R>(
        &mut self,
        dimension: &LevelId,
        run: impl FnOnce(&mut EntityStore, &mut LevelCtx<'_, '_>) -> R,
    ) -> R {
        // A stored hurt ticks entities outside the level frame that refreshed
        // the projection, so it refreshes the one it reads.
        self.refresh_cross_level_entities_by_id(dimension);
        let players = self.nearby_players_by_key(dimension);
        let block_interaction_policy = self.block_interaction_policy(dimension);
        let mut leftover = PendingEntitySpawns::default();
        let answer = self.with_stored_entity_level(
            dimension,
            &block_interaction_policy,
            None,
            &players,
            &mut leftover,
            |entities, level, _| run(entities, level),
        );
        self.flush_stored_player_kills(dimension);
        self.apply_pending_spawns(dimension, leftover);
        answer
    }

    /// [`Self::with_stored_hurt_level`] that reports a refused spawn
    /// publication, and lends a damage-immunity reader for a `caller` level.
    pub(in crate::world) fn with_stored_hurt_level_checked<R>(
        &mut self,
        dimension: &LevelId,
        caller: Option<&LevelId>,
        run: impl FnOnce(
            &mut EntityStore,
            &mut LevelCtx<'_, '_>,
            Option<&mut CallerDamageImmunityReader<'_>>,
        ) -> R,
    ) -> Option<R> {
        self.refresh_cross_level_entities_by_id(dimension);
        let players = self.nearby_players_by_key(dimension);
        let block_interaction_policy = self.block_interaction_policy(dimension);
        let mut leftover = PendingEntitySpawns::default();
        let answer = self.with_stored_entity_level(
            dimension,
            &block_interaction_policy,
            caller,
            &players,
            &mut leftover,
            run,
        );
        self.flush_stored_player_kills(dimension);
        self.apply_pending_spawns_checked(dimension, leftover)?;
        Some(answer)
    }

    pub(super) fn with_stored_entity_level<R>(
        &mut self,
        dimension: &LevelId,
        block_interaction_policy: &BlockInteractionPolicy,
        caller: Option<&LevelId>,
        players: &[NearbyPlayer],
        leftover: &mut PendingEntitySpawns,
        run: impl FnOnce(
            &mut EntityStore,
            &mut LevelCtx<'_, '_>,
            Option<&mut CallerDamageImmunityReader<'_>>,
        ) -> R,
    ) -> R {
        let game_time = self.game_time();
        let game_rules = self.game_rules;
        let feature_flags = self.feature_flags;
        let (
            level,
            drop_tables,
            loot_level_facts,
            sculk_warning,
            structure_services,
            originals,
            mut caller_reads,
        ) = self.level_drop_tables_and_sculk_warning_with_caller(dimension, caller);
        let caller_loot = drop_tables.tables;
        let caller_tags = drop_tables.damage_type_tags;
        let (mut level, LevelRest { entities, .. }) = LevelCtx::new(
            level,
            LevelFrame {
                block_interaction_policy,
                full_preparation:
                    crate::world::chunk_lifecycle::full_preparation::FullChunkPreparationInputs {
                        originals,
                        entity_nbt: &structure_services.entity_nbt,
                    },
                game_time,
                game_rules,
                feature_flags,
                drop_tables,
                loot_level_facts,
                structure_services: Some(&structure_services),
                players,
                item_spawns: leftover,
            },
        );
        let level_facts = std::rc::Rc::clone(&level.loot_level_facts);
        level.sculk_warning = Some(sculk_warning);
        drain_ground_navigation_updates(entities, &mut level);
        let mut immunity_reader =
            |source_level: &mut LevelCtx<'_, '_>,
             request: crate::entity::enchanted_damage::EnchantedDamageRequest<'_>| {
                let reads = caller_reads
                    .as_mut()
                    .ok_or("hurt caller context unavailable")?;
                let table = level_facts
                    .enchantment_combat_effects
                    .as_ref()
                    .ok_or("hurt caller enchantment generation unavailable")?;
                let blocks = super::super::level_blocks::LoadedBlocks {
                    chunks: reads.chunks,
                };
                let bodies = match request.victim {
                    crate::loot_runtime::LootEntity::Stored(stored) => {
                        stored.bodies.unwrap_or(request.bodies)
                    }
                    _ => request.bodies,
                };
                let resolver = request.bodies.store().item_resolver();
                let mut item_environment = super::super::simulation_level::mob_death_loot::LevelLootEnvironment::for_predicates(reads.chunks, &level_facts, Some(bodies))
                    .with_environment_attributes(reads.environment_attributes);
                if let Some(registries) = level_facts.registries {
                    item_environment = item_environment.with_position_queries(
                        registries,
                        level_facts.world_seed,
                        reads.sky_darkening,
                    );
                }
                let source_weather = crate::loot_runtime::LootWeather {
                    raining: source_level.weather.is_raining(),
                    thundering: source_level.weather.is_thundering(),
                };
                super::super::simulation_level::block_entity_loot::with_predicate_loot_context(
                    source_level.chunks,
                    &source_level.loot_level_facts,
                    source_level.drop_tables.tables,
                    source_weather,
                    source_level.random,
                    Some(bodies),
                    false,
                    Some(resolver),
                    "no loot registries are loaded",
                    |source_refs| {
                        let context = super::super::combat::DamageContextLevel {
                            vehicle_slots: Some(source_refs.vehicle_slots),
                            dimension: Some(reads.dimension),
                            item_environment: Some(&item_environment),
                            loot_tables: caller_loot,
                            damage_type_tags: caller_tags,
                            blocks: &|pos| blocks.loaded_block_state(pos),
                            block_entity_data: &|pos| {
                                reads
                                    .chunks
                                    .block_entity_loot_data(pos, Some(resolver))
                                    .map_err(|error| {
                                        crate::loot_runtime::LootRuntimeReason::Catalog(
                                            error.to_string(),
                                        )
                                    })
                            },
                            sky: &|pos| blocks.loaded_can_see_sky(pos),
                            weather: reads.weather,
                        };
                        super::super::simulation_level::enchanted_damage::with_request_context(
                            table,
                            &context,
                            &|_| Err("Conduit immunity cannot resolve a session attacker".into()),
                            reads.random,
                            request,
                            super::super::combat::enchantments_immune,
                        )
                    },
                )
            };
        let has_caller = caller.is_some_and(|caller| caller != dimension);
        let answer = run(
            entities,
            &mut level,
            has_caller.then_some(&mut immunity_reader as &mut CallerDamageImmunityReader<'_>),
        );
        drain_ground_navigation_updates(entities, &mut level);
        // The frame is over: the events the body raised are posted now, before
        // the next body runs.
        self.post_pending_game_events(dimension);
        answer
    }

    #[cfg(test)]
    pub(crate) fn level_light_facts_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        pos: BlockPos,
    ) -> (i32, i32, i32, bool) {
        self.with_level(dimension, |level| {
            (
                LevelCtx::brightness(level, crate::block_behavior::LightLayer::Block, pos),
                LevelCtx::brightness(level, crate::block_behavior::LightLayer::Sky, pos),
                LevelCtx::raw_brightness(level, pos, 0),
                LevelCtx::can_see_sky(level, pos),
            )
        })
    }

    /// `Level.setBlock` for callers without a level borrow: the same body the
    /// mechanisms reach, so a placed liquid arms its
    /// first spread, a placed falling block arms its fall, and the neighbours
    /// of either are notified, exactly as they are when the world writes the
    /// block itself.
    pub(crate) fn set_block(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        flags: i32,
    ) -> bool {
        let dimension = &dimension.into();
        if !self.is_in_world_bounds_by_key(dimension, pos) {
            return false;
        }
        self.with_level(dimension, |level| {
            level.set_block(pos, block_state_id, flags, UPDATE_LIMIT)
        })
    }

    /// Borrow one dimension and apply its queued work before returning.
    pub(in crate::world) fn with_level<R>(
        &mut self,
        dimension: impl Into<LevelId>,
        body: impl FnOnce(&mut LevelCtx<'_, '_>) -> R,
    ) -> R {
        let dimension = dimension.into();
        let players = self.nearby_players_by_key(&dimension);
        self.with_level_frame(&dimension, &players, true, true, body)
    }

    pub(in crate::world) fn with_explosion_block_loot_level<R>(
        &mut self,
        dimension: impl Into<LevelId>,
        body: impl FnOnce(&mut LevelCtx<'_, '_>) -> R,
    ) -> R {
        let dimension = dimension.into();
        let players = self.nearby_players_by_key(&dimension);
        self.with_level_frame(&dimension, &players, false, true, body)
    }

    pub(super) fn vibration_back_signal(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        direction: bbb_protocol::packets::Direction,
    ) -> Option<i32> {
        let players = self.nearby_players_by_key(dimension);
        // A listener is temporarily outside the store; a signal read must not
        // dispatch pending events or run navigation callbacks in that window.
        self.with_level_frame(dimension, &players, false, false, |level| {
            Some(LevelCtx::get_signal(level, pos, direction))
        })
    }

    /// Borrows one level with the world's services; the caller applies
    /// `item_spawns` once the borrow ends.
    pub(in crate::world) fn with_level_ctx<R>(
        &mut self,
        dimension: &LevelId,
        players: &[NearbyPlayer],
        item_spawns: &mut PendingEntitySpawns,
        body: impl FnOnce(&mut LevelCtx<'_, '_>, LevelRest<'_>) -> R,
    ) -> R {
        let game_time = self.game_time();
        let game_rules = self.game_rules;
        let feature_flags = self.feature_flags;
        let block_interaction_policy = self.block_interaction_policy(dimension);
        let (level, drop_tables, loot_level_facts, sculk_warning, structure_services, originals) =
            self.level_drop_tables_and_sculk_warning(dimension);
        let (mut level, rest) = LevelCtx::new(
            level,
            LevelFrame {
                block_interaction_policy: &block_interaction_policy,
                full_preparation:
                    crate::world::chunk_lifecycle::full_preparation::FullChunkPreparationInputs {
                        originals,
                        entity_nbt: &structure_services.entity_nbt,
                    },
                game_time,
                game_rules,
                feature_flags,
                drop_tables,
                loot_level_facts,
                structure_services: Some(&structure_services),
                players,
                item_spawns,
            },
        );
        level.sculk_warning = Some(sculk_warning);
        body(&mut level, rest)
    }

    fn with_level_frame<R>(
        &mut self,
        dimension: &LevelId,
        players: &[NearbyPlayer],
        run_callbacks: bool,
        post_events: bool,
        body: impl FnOnce(&mut LevelCtx<'_, '_>) -> R,
    ) -> R {
        let mut leftover = PendingEntitySpawns::default();
        let answer = self.with_level_ctx(dimension, players, &mut leftover, |level, rest| {
            if run_callbacks {
                drain_ground_navigation_updates(rest.entities, level);
            }
            let answer = body(&mut level.with_structure_entities(rest.entities, None));
            if run_callbacks {
                drain_ground_navigation_updates(rest.entities, level);
            }
            answer
        });
        // The frame is over: the events the body raised are posted now, before
        // the next body runs.
        self.reconcile_saved_column_accesses(dimension);
        if post_events {
            self.post_pending_game_events(dimension);
        }
        self.apply_pending_spawns(dimension, leftover);
        answer
    }
}
