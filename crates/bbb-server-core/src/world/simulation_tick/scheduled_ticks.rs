//! The two dimension-wide schedulers: draining the block and fluid ticks a
//! level armed, and taking back the ticks a mechanism arms while it runs.

use super::*;
use crate::block_behavior::{CopperWeatherState, GolemKind};
use crate::entity::BlockPatternMob;
use crate::world::LevelId;

impl ServerWorld {
    pub(super) fn tick_scheduled_block_ticks(
        &mut self,
        dimension: &LevelId,
        ticking: &[ChunkKey],
        events: &mut Vec<WorldEvent>,
    ) {
        let game_time = self.game_time();
        let (prev_handling, first_tick) = {
            let level = self.level_mut_by_id(dimension);
            let prev_handling = level.block_events.set_handling_tick(true);
            level.block_ticks.begin_tick(
                &mut level.chunks,
                game_time,
                SERVER_LEVEL_MAX_TICKS_PER_TICK,
                ticking,
            );
            let first = level.block_ticks.next_collected_tick();
            if first.is_none() {
                level.block_ticks.finish_tick();
                level.block_events.set_handling_tick(prev_handling);
            }
            (prev_handling, first)
        };
        let Some(first_tick) = first_tick else {
            return;
        };
        let mut shrieker_ticks = Vec::new();
        let infiniburn_blocks = self.level_infiniburn_blocks(dimension);
        let game_rules = self.game_rules;
        let difficulty = self.admin.settings().difficulty;
        let difficulty_id = difficulty.id();
        let spawning_monsters = game_rules.spawning_monsters(difficulty);
        let mut players_cache: Option<Vec<NearbyPlayer>> = None;
        let mut pending_tick = Some(first_tick);
        while let Some(tick) = pending_tick.take() {
            let state = super::super::level_blocks::stored_block_state(
                &self.level_by_id(dimension).chunks,
                tick.pos,
            );
            if BlockTickType::of_block_state(state) != Some(tick.tick_type) {
                pending_tick = self
                    .level_mut_by_id(dimension)
                    .block_ticks
                    .next_collected_tick();
                continue;
            }
            if crate::block_behavior::command_block::is_command_block(state) {
                self.tick_command_block(dimension, tick.pos, state, events);
                players_cache = None;
                pending_tick = self
                    .level_mut_by_id(dimension)
                    .block_ticks
                    .next_collected_tick();
                continue;
            }
            if crate::block_behavior::crafter::is_crafter(state) {
                self.tick_crafter(dimension, tick.pos, state, events);
                players_cache = None;
                pending_tick = self
                    .level_mut_by_id(dimension)
                    .block_ticks
                    .next_collected_tick();
                continue;
            }
            if crate::block_behavior::dispenser::is_dispenser_family(state) {
                self.tick_dispenser(dimension, tick.pos, state);
                players_cache = None;
                pending_tick = self
                    .level_mut_by_id(dimension)
                    .block_ticks
                    .next_collected_tick();
                continue;
            }
            if players_cache.is_none() {
                players_cache = Some(self.nearby_players_by_key(dimension));
            }
            let players = players_cache.take().expect("cached nearby players");
            let mut leftover = PendingEntitySpawns::default();
            let mut deferred = None;
            self.with_level_ctx(dimension, &players, &mut leftover, |slice, rest| {
                let entities = rest.entities;
                let mut current = Some((tick, state));
                while let Some((tick, state)) = current.take() {
                    if liquid_block_family(state).is_some() {
                        let mut level = slice.with_structure_entities(entities, None);
                        tick_liquid_bubble_column(&mut level, tick.pos, state);
                    } else if is_falling_block(state) {
                        falling_block_scheduled_tick(entities, slice, tick.pos, state);
                    } else if fire::is_fire(state) {
                        let environment = fire::FireFacts {
                            infiniburn_blocks: &infiniburn_blocks,
                            players: &players,
                            fire_spread_radius: game_rules.fire_spread_radius_around_player,
                            difficulty_id,
                            spawning_monsters,
                            raining: slice.weather.is_raining(),
                        };
                        let mut level = slice.with_structure_entities(entities, None);
                        let _ = fire::tick_with_facts(&mut level, &environment, tick.pos, state);
                    } else if crate::block_entity::sculk_shrieker::is_shrieking(state) {
                        shrieker_ticks.push((tick.pos, state));
                    } else {
                        let mut level = slice.with_structure_entities(entities, None);
                        tick_block(&mut level, tick.pos, state);
                    }
                    if slice.event_sink.has_pending_game_events() || slice.item_spawns.has_pending()
                    {
                        break;
                    }
                    while let Some(upcoming) = slice.block_ticks.next_collected_tick() {
                        let upcoming_state = super::super::level_blocks::stored_block_state(
                            slice.chunks,
                            upcoming.pos,
                        );
                        if BlockTickType::of_block_state(upcoming_state) != Some(upcoming.tick_type)
                        {
                            continue;
                        }
                        if crate::block_behavior::command_block::is_command_block(upcoming_state)
                            || crate::block_behavior::crafter::is_crafter(upcoming_state)
                            || crate::block_behavior::dispenser::is_dispenser_family(upcoming_state)
                        {
                            deferred = Some(upcoming);
                            break;
                        }
                        current = Some((upcoming, upcoming_state));
                        break;
                    }
                }
            });
            self.post_pending_game_events(dimension);
            let pushed_entities = !leftover.entity_push_ups.is_empty();
            for (old_state, new_state, pos) in std::mem::take(&mut leftover.entity_push_ups) {
                let _ = self.push_live_entities_up(dimension, old_state, new_state, pos, events);
            }
            if !pushed_entities {
                players_cache = Some(players);
            }
            self.apply_pending_spawns(dimension, leftover);
            pending_tick = deferred.or_else(|| {
                self.level_mut_by_id(dimension)
                    .block_ticks
                    .next_collected_tick()
            });
        }
        let level = self.level_mut_by_id(dimension);
        level.block_ticks.finish_tick();
        level.block_events.set_handling_tick(prev_handling);
        for (pos, state) in shrieker_ticks {
            self.tick_sculk_shrieker_scheduled(dimension, pos, state, events);
        }
    }

    pub(super) fn tick_scheduled_fluid_ticks(&mut self, dimension: &LevelId, ticking: &[ChunkKey]) {
        let game_time = self.game_time();
        let (prev_handling, first_tick) = {
            let level = self.level_mut_by_id(dimension);
            level.fluid_ticks.begin_tick(
                &mut level.chunks,
                game_time,
                SERVER_LEVEL_MAX_TICKS_PER_TICK,
                ticking,
            );
            let first = level.fluid_ticks.next_collected_tick();
            if first.is_none() {
                level.fluid_ticks.finish_tick();
                return;
            }
            let prev = level.block_events.set_handling_tick(true);
            (prev, first)
        };
        let Some(first_tick) = first_tick else {
            return;
        };
        let players = self.nearby_players_by_key(dimension);
        let mut leftover = PendingEntitySpawns::default();
        self.with_level_ctx(dimension, &players, &mut leftover, |slice, rest| {
            {
                let mut level = slice.with_structure_entities(rest.entities, None);
                tick_fluid(&mut level, first_tick.pos, first_tick.tick_type);
            }
            while let Some(tick) = slice.fluid_ticks.next_collected_tick() {
                let mut level = slice.with_structure_entities(rest.entities, None);
                tick_fluid(&mut level, tick.pos, tick.tick_type);
            }
            slice.fluid_ticks.finish_tick();
        });
        self.level_mut_by_id(dimension)
            .block_events
            .set_handling_tick(prev_handling);
        // The frame the fluid-tick drain ran on is over.
        self.post_pending_game_events(dimension);
        self.apply_pending_spawns(dimension, leftover);
    }

    /// Adds the entities a level borrow queued once the world is free again.
    pub(in crate::world) fn apply_pending_spawns(
        &mut self,
        dimension: &LevelId,
        leftover: PendingEntitySpawns,
    ) {
        let _ = self.apply_pending_spawns_checked(dimension, leftover);
    }

    pub(in crate::world) fn apply_pending_spawns_checked(
        &mut self,
        dimension: &LevelId,
        leftover: PendingEntitySpawns,
    ) -> Option<()> {
        // The removal overrides that reach the entity store or the session
        // list wait for the tick's own drain, which holds both.
        self.pending_removal_effects.extend(
            leftover
                .removal_effects
                .into_iter()
                .map(|effect| (*dimension, effect)),
        );
        for batch in leftover.worldgen_entities {
            crate::world::chunk_lifecycle::full_preparation::install_worldgen_entities(
                &mut self.level_mut_by_id(dimension).entities,
                batch,
            );
        }
        for mut mob in leftover.structure_mobs {
            let finalized = self.finalize_structure_template_entity(dimension, &mut mob);
            let entities = &mut self.level_mut_by_id(dimension).entities;
            let mob = Box::new(mob);
            if finalized {
                let team_identity =
                    matches!(mob.data(), crate::entity::EntityData::GeneralLiving(_))
                        .then_some((mob.id(), mob.uuid()));
                entities.add_fresh(mob);
                if let Some((entity_id, uuid)) = team_identity {
                    entities.queue_command(crate::entity::EntityCommand::LoadedTeam {
                        entity_id,
                        uuid,
                    });
                }
            } else {
                entities.discard_fresh(mob);
            }
        }
        for pos in leftover.tnt_primes {
            let _ = self.prime_tnt_from_redstone(*dimension, pos);
        }
        // The two drop queues are drained in the order the level filled them:
        // `Containers.dropContents` runs inside `setBlockState`, so it can sit
        // between two `Block.popResource` calls of the same tick, and both take
        // their draws from the entity store's own source.
        let mut container_drops = leftover.container_drops.into_iter().peekable();
        for (index, spawn) in leftover.items.into_iter().enumerate() {
            while container_drops
                .peek()
                .is_some_and(|drop| drop.items_before <= index)
            {
                let drop = container_drops.next().expect("peeked");
                let level = self.level_mut_by_id(dimension);
                if !drop_container_contents(&mut level.entities, drop.position, &drop.slots) {
                    tracing::warn!("container item publication refused");
                    return None;
                }
            }
            let level = self.level_mut_by_id(dimension);
            if crate::entity::item::spawn_item_entity_with_pickup_delay(
                &mut level.entities,
                spawn.position,
                spawn.item,
                spawn.default_pickup_delay,
            )
            .is_none()
            {
                tracing::warn!("the entity store refused the item entity's drawn UUID");
            }
        }
        for drop in container_drops {
            let level = self.level_mut_by_id(dimension);
            if !drop_container_contents(&mut level.entities, drop.position, &drop.slots) {
                tracing::warn!("container item publication refused");
                return None;
            }
        }
        for request in leftover.block_mobs {
            match request {
                BlockMobSpawnRequest::HatchedTurtle { position, home_pos } => {
                    let _ = self
                        .level_mut_by_id(dimension)
                        .entities
                        .spawn_hatched_turtle(position, home_pos);
                }
                BlockMobSpawnRequest::HatchedSniffer { position, y_rot } => {
                    let _ = self
                        .level_mut_by_id(dimension)
                        .entities
                        .spawn_hatched_sniffer(position, y_rot);
                }
                BlockMobSpawnRequest::HydratedHappyGhast { position, y_rot } => {
                    let _ = self
                        .level_mut_by_id(dimension)
                        .entities
                        .spawn_hydrated_happy_ghast(position, y_rot);
                }
                BlockMobSpawnRequest::FrogspawnTadpole { position, y_rot } => {
                    let _ = self
                        .level_mut_by_id(dimension)
                        .entities
                        .spawn_frogspawn_tadpole(position, y_rot);
                }
                BlockMobSpawnRequest::PortalZombifiedPiglin { position, y_rot } => {
                    let _ =
                        self.spawn_finalized_portal_zombified_piglin(*dimension, position, y_rot);
                }
            }
        }
        for request in leftover.pattern_mobs {
            let mob = match request.kind {
                GolemKind::Snow => BlockPatternMob::SnowGolem,
                GolemKind::Iron => BlockPatternMob::IronGolem,
                GolemKind::Copper => BlockPatternMob::CopperGolem {
                    weather_state: match request.copper_weather {
                        Some(CopperWeatherState::Exposed) => "exposed",
                        Some(CopperWeatherState::Weathered) => "weathered",
                        Some(CopperWeatherState::Oxidized) => "oxidized",
                        Some(CopperWeatherState::Unaffected) | None => "unaffected",
                    },
                },
                GolemKind::Wither => BlockPatternMob::Wither {
                    y_rot: request.y_rot,
                },
            };
            let store = &mut self.level_mut_by_id(dimension).entities;
            if let Some(entity_id) = store.spawn_block_pattern_mob(mob, request.pos) {
                // CarvedPumpkinBlock.java:107-116; WitherSkullBlock.java:72-74.
                let range = if matches!(request.kind, GolemKind::Wither) {
                    50.0
                } else {
                    5.0
                };
                store.queue_command(crate::entity::EntityCommand::SummonedEntity {
                    entity_id,
                    range,
                });
            } else {
                tracing::warn!(
                    "the entity store refused the {:?} pattern summon",
                    request.kind
                );
            }
        }
        for request in leftover.falling_blocks {
            let _ = spawn_requested_fallen_block(
                &mut self.level_mut_by_id(dimension).entities,
                request,
            );
        }
        Some(())
    }

    /// A column's parked containers move into the level schedulers here.
    pub(in crate::world) fn register_chunk_tick_containers_by_key(
        &mut self,
        dimension: &LevelId,
        chunk: ChunkKey,
    ) {
        let game_time = self.game_time();
        self.level_mut_by_id(dimension)
            .chunks
            .register_ticks(chunk, game_time);
    }

    /// [`LevelCtx::schedule_block_tick`] for callers that hold no level borrow.
    pub(in crate::world) fn schedule_block_tick(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        block: BlockTickType,
        delay: i32,
        priority: TickPriority,
    ) {
        if !self.is_in_world_bounds_by_key(dimension, pos) {
            return;
        }
        self.register_chunk_tick_containers_by_key(dimension, chunk_of_block(pos));
        let trigger_tick = self.game_time() + i64::from(delay);
        let level = self.level_mut_by_id(dimension);
        let sub_tick_order = level.sub_tick_count;
        level.sub_tick_count += 1;
        level.chunks.schedule_tick(ScheduledTick::with_priority(
            block,
            pos,
            trigger_tick,
            priority,
            sub_tick_order,
        ));
    }

    #[cfg(test)]
    pub(crate) fn apply_block_mob_spawn_request_for_test(
        &mut self,
        dimension: impl Into<LevelId>,
        request: BlockMobSpawnRequest,
    ) -> Option<i32> {
        let dimension = dimension.into();
        let portal_zombified_piglin =
            matches!(&request, BlockMobSpawnRequest::PortalZombifiedPiglin { .. });
        let before = self
            .level_by_id(&dimension)
            .entities
            .ids()
            .into_iter()
            .collect::<std::collections::HashSet<_>>();
        let mut leftover = PendingEntitySpawns::default();
        leftover.block_mobs.push(request);
        self.apply_pending_spawns(&dimension, leftover);
        self.level_by_id(&dimension)
            .entities
            .ids()
            .into_iter()
            .find(|id| {
                !before.contains(id)
                    && (!portal_zombified_piglin
                        || self
                            .level_by_id(&dimension)
                            .entities
                            .get(*id)
                            .is_some_and(|entity| {
                                entity.data().entity_resource_id() == "minecraft:zombified_piglin"
                            }))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::{
        VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_REDSTONE_BLOCK_ID, VANILLA_BLOCK_STONE_ID,
        VANILLA_BLOCK_TNT_ID,
    };
    use bbb_protocol::packets::Vec3d;
    use bbb_server_types::save::{Java26BuiltinDimension, Java26LevelKey};

    fn custom_dimension_world() -> (ServerWorld, LevelId) {
        let mut candidate = crate::tests::restore_candidate(0, None, Vec::new());
        let custom = Java26LevelKey::parse("example:moon").unwrap();
        let overworld = Java26LevelKey::from(Java26BuiltinDimension::Overworld);
        let stem = candidate.world_gen_settings.dimensions[&overworld].clone();
        candidate
            .world_gen_settings
            .dimensions
            .insert(custom.clone(), stem);
        candidate.raids.insert(custom.clone(), Default::default());
        candidate.world_borders.insert(
            custom.clone(),
            bbb_server_persistence::Java26WorldBorderSettings::default(),
        );
        let mut world = ServerWorld::restore(crate::tests::datapack_config(), &candidate).unwrap();
        let level_id = world.require_level_id(&custom);
        world.level_mut_by_id(&level_id).chunks.load_column(0, 0);
        (world, level_id)
    }

    #[test]
    fn redstone_primes_tnt_in_custom_datapack_dimension() {
        let (mut world, custom) = custom_dimension_world();
        let tnt_pos = BlockPos { x: 4, y: 64, z: 4 };
        let power_pos = BlockPos { x: 5, y: 64, z: 4 };
        let tnt_state = vanilla_block_default_state_id(VANILLA_BLOCK_TNT_ID).unwrap();
        let redstone_block =
            vanilla_block_default_state_id(VANILLA_BLOCK_REDSTONE_BLOCK_ID).unwrap();
        let air = vanilla_block_default_state_id(VANILLA_BLOCK_AIR_ID).unwrap();

        assert!(world.set_block(custom, tnt_pos, tnt_state, SET_BLOCK_UPDATE_ALL));
        assert!(world.set_block(custom, power_pos, redstone_block, SET_BLOCK_UPDATE_ALL));

        assert_eq!(world.block_state(custom, tnt_pos), Some(air));
        let level = world.level_by_id(&custom);
        let primed_tnt_count = level
            .entities
            .ids()
            .into_iter()
            .filter(|id| {
                level
                    .entities
                    .get(*id)
                    .is_some_and(|entity| entity.data().entity_resource_id() == "minecraft:tnt")
            })
            .count();
        assert_eq!(primed_tnt_count, 1);
    }

    #[test]
    fn portal_zombified_piglin_spawns_in_custom_datapack_dimension() {
        let (mut world, custom) = custom_dimension_world();
        let floor = BlockPos { x: 4, y: 64, z: 4 };
        let stone = vanilla_block_default_state_id(VANILLA_BLOCK_STONE_ID).unwrap();
        assert!(world.set_block(custom, floor, stone, SET_BLOCK_UPDATE_ALL));

        let spawned = world.apply_block_mob_spawn_request_for_test(
            custom,
            BlockMobSpawnRequest::PortalZombifiedPiglin {
                position: Vec3d {
                    x: 4.5,
                    y: 65.0,
                    z: 4.5,
                },
                y_rot: 0.0,
            },
        );
        assert!(spawned.is_some());
    }
}
