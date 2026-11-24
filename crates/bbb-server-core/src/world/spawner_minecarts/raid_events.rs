//! The `ServerLevel` half of `Raid.tick`: what the raid runtime reads off this
//! level and the wave spawn it publishes through the spawner runtime.

use super::*;
use crate::entity::minecart_spawner::spawn_placement;
use crate::entity::natural_spawn::create_natural_spawn_root;
use crate::entity::raider::{can_join_raid, PendingRaiderRecruit, RaiderRaid};
use crate::entity::{
    apply_raid_buffs, join_raid, set_patrol_leader, set_spawn_on_ground_with_moving_pistons,
};
use crate::world::LevelId;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_RAVAGER_ID;
use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_WORLD_SURFACE_ID;
use bbb_protocol::packets::Vec3d;
use bbb_protocol::RichTextComponent;
use uuid::Uuid;

use super::super::raids::{RaidPlayer, RaidRaider, RaidSpawn, RaidSpawnOutcome, RaidTickOutcome};
use super::super::{level_height, stored_block_state, WorldEvent};

/// One level as `Raid.tick` reads it, over the spawner runtime its wave spawn
/// publishes through.
pub(in crate::world) struct RaidLevelRuntime<'a> {
    runtime: MinecartSpawnerRuntime<'a>,
    pub(in crate::world) level_random: &'a GameRandom,
    simulation_levels: &'a super::super::SimulationChunkLevels,
    world_border: super::super::WorldBorder,
    players: Vec<RaidPlayer>,
    difficulty: Difficulty,
    game_time: i64,
    ravager: crate::entity::SpawnPlacementEntry,
}

impl RaidLevelRuntime<'_> {
    pub(in crate::world) fn difficulty(&self) -> Difficulty {
        self.difficulty
    }

    pub(in crate::world) fn has_chunk_at(&self, pos: BlockPos) -> bool {
        self.runtime.chunks.is_ticking_full(pos.x >> 4, pos.z >> 4)
    }

    /// `ServerLevel.isVillage(pos)` is `isCloseToVillage(pos, 1)`
    /// (`ServerLevel.java`).
    pub(in crate::world) fn is_village(&self, pos: BlockPos) -> Option<bool> {
        self.runtime
            .chunks
            .sections_to_village([pos.x >> 4, pos.y >> 4, pos.z >> 4])
            .map(|sections| sections <= 1)
    }

    pub(in crate::world) fn players(&self) -> &[RaidPlayer] {
        &self.players
    }

    /// `canPositionTick` is the loaded entity section and `inEntityTickingRange`
    /// the ticket level.
    pub(in crate::world) fn is_position_entity_ticking(&self, pos: BlockPos) -> bool {
        let chunk = (pos.x >> 4, pos.z >> 4);
        self.runtime.chunks.is_loaded(chunk.0, chunk.1)
            && self.runtime.chunks.is_ticking_full(chunk.0, chunk.1)
            && self.simulation_levels.is_entity_ticking(chunk)
    }

    pub(in crate::world) fn world_surface_height(&self, x: i32, z: i32) -> Option<i32> {
        level_height(
            self.runtime.chunks,
            VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
            x,
            z,
        )
    }

    pub(in crate::world) fn wave_spawn_block_ok(&self, pos: BlockPos) -> bool {
        let mut environment = super::spawn_environment(
            self.runtime.chunks,
            SpawnPlacementProbe {
                entity_type_id: VANILLA_ENTITY_TYPE_RAVAGER_ID,
                position: pos,
                entry: self.ravager,
            },
        );
        environment.inside_world_border = self.world_border.contains_block_at(pos, self.game_time);
        is_spawn_position_ok(self.ravager.placement, environment)
            || snow_under_air(self.runtime.chunks, pos)
    }

    pub(in crate::world) fn raider(&self, uuid: Uuid) -> Option<RaidRaider> {
        let id = self.runtime.entities.entity_id_for_uuid(uuid)?;
        let entity = self.runtime.entities.get(id)?;
        let living = entity.living()?;
        Some(RaidRaider {
            position: entity.block_position(),
            removed: !entity.is_alive(),
            tick_count: entity.tick_count(),
            no_action_time: living.no_action_time,
            health: living.health(),
        })
    }

    pub(in crate::world) fn recruit_raider(
        &mut self,
        pending: &PendingRaiderRecruit,
        wave: i32,
        raids: &super::super::raids::LevelRaids,
    ) -> Option<(Uuid, f32)> {
        let entity = self.runtime.entities.get_mut(pending.recruit.entity_id)?;
        if entity.uuid() != pending.recruit.uuid || !can_join_raid(entity) {
            return None;
        }
        let crate::entity::EntityData::GeneralLiving(data) = entity.data() else {
            return None;
        };
        if let RaiderRaid::Current(id) = data.raider_retained()?.raid() {
            if raids.raider_raid(id).is_some_and(|raid| raid.active) {
                return None;
            }
        }
        let health = entity.living()?.health();
        join_raid(entity, pending.raid_id, wave).then_some((entity.uuid(), health))
    }

    pub(in crate::world) fn spawn_raider(&mut self, spawn: RaidSpawn) -> RaidSpawnOutcome {
        match self.build_raider(&spawn) {
            Ok((uuid, health)) => RaidSpawnOutcome::Spawned { uuid, health },
            Err(detail) => RaidSpawnOutcome::Refused(format!(
                "wave {} entity type {} at {:?}: {detail}",
                spawn.wave, spawn.entity_type_id, spawn.position
            )),
        }
    }
}

impl RaidLevelRuntime<'_> {
    /// `Raid.joinRaid`'s `exists == false` arm (`Raid.java`) for a
    /// raider `spawnGroup` has just created, with the leader flag and the raid
    /// binding written first because `PatrollingMonster.finalizeSpawn` equips
    /// the banner off one (`PatrollingMonster.java`) and
    /// `Vindicator.populateDefaultEquipmentSlots` reads the other
    /// (`Vindicator.java`).
    fn build_raider(&mut self, spawn: &RaidSpawn) -> Result<(Uuid, f32), String> {
        let Some(mut tree) = create_natural_spawn_root(
            self.runtime.entities,
            spawn.entity_type_id,
            Vec3d::default(),
        ) else {
            return Err("the entity store built no root".into());
        };
        let resource_id = tree.data().entity_resource_id().to_owned();
        if spawn.leader && !set_patrol_leader(&mut tree) {
            self.runtime.entities.discard_fresh(tree);
            return Err("no raider state took the leader flag".into());
        }
        if !join_raid(&mut tree, spawn.raid_id, spawn.wave) {
            self.runtime.entities.discard_fresh(tree);
            return Err("no raider state took the raid binding".into());
        }
        tree.set_pos(Vec3d {
            x: f64::from(spawn.position.x) + 0.5,
            y: f64::from(spawn.position.y) + 1.0,
            z: f64::from(spawn.position.z) + 0.5,
        });
        if finalize_spawn(
            &mut self.runtime,
            &mut tree,
            SpawnReason::Event,
            None,
            self.level_random,
        )
        .is_none()
        {
            self.runtime.entities.discard_fresh(tree);
            return Err("missing moon, biome or difficulty context".into());
        }
        let runtime = &self.runtime;
        let special_multiplier = runtime
            .spawn_environment
            .special_multiplier(
                runtime.chunks,
                spawn.position,
                runtime.difficulty,
                runtime.overworld_clock_time,
            )
            .unwrap_or(0.0);
        let random = self.level_random;
        if !apply_raid_buffs(
            &mut tree,
            random,
            spawn.enchant_odds,
            spawn.enchant_provider,
            |provider, stack| {
                apply_enchantment_provider_to_stack(
                    runtime,
                    provider,
                    stack,
                    special_multiplier,
                    random,
                )
            },
        ) {
            self.runtime.entities.discard_fresh(tree);
            return Err(format!("{resource_id} owns no raid buffs here"));
        }
        let chunks = &*self.runtime.chunks;
        set_spawn_on_ground_with_moving_pistons(
            &mut tree,
            &|pos| collision_boxes(chunks, pos),
            &|pos| super::super::movement_collision::moving_piston_at(chunks, pos).unwrap_or(true),
        );
        if let Some(vehicle) = spawn.rides {
            // `Raid.java`: the rider snaps to the block's bottom centre
            // and mounts, which this runtime publishes it already holding.
            let root = &mut tree;
            root.set_pos(Vec3d {
                x: f64::from(spawn.position.x) + 0.5,
                y: f64::from(spawn.position.y),
                z: f64::from(spawn.position.z) + 0.5,
            });
            root.set_rot(0.0, 0.0);
            root.set_old_pos_and_rot();
            let Some(vehicle_id) = self.runtime.entities.entity_id_for_uuid(vehicle) else {
                self.runtime.entities.discard_fresh(tree);
                return Err("the ravager it rides left the level".into());
            };
            if !self
                .runtime
                .entities
                .fresh_ride(&mut tree, vehicle_id, false)
            {
                self.runtime.entities.discard_fresh(tree);
                return Err("the fresh rider took no vehicle".into());
            }
        }
        let uuid = tree.uuid();
        let health = tree.living().map_or(0.0, |living| living.health());
        self.runtime.entities.add_fresh(tree);
        Ok((uuid, health))
    }
}

fn collision_boxes(chunks: &ChunkStore, pos: BlockPos) -> Vec<Aabb> {
    super::super::movement_collision::static_collision_shape_with_live_piston(
        chunks,
        pos,
        stored_block_state(chunks, pos),
    )
}

/// `Raid.java`: a snow layer under an air block also takes a wave.
fn snow_under_air(chunks: &ChunkStore, pos: BlockPos) -> bool {
    let below = BlockPos {
        y: pos.y - 1,
        ..pos
    };
    vanilla_block_state_block_type_id(stored_block_state(chunks, below))
        == Some(bbb_protocol::block_types::VANILLA_BLOCK_SNOW_ID)
        && bbb_server_types::fluid::block_state_is_air(stored_block_state(chunks, pos))
}

impl ServerWorld {
    /// `ServerLevel.tick`'s raid arm (`ServerLevel.java`).
    pub(in super::super) fn tick_level_raids(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        if let Err(detail) = self.advance_level_raids(dimension, events) {
            events.push(WorldEvent::ConsoleMessage(RichTextComponent::plain(
                format!(
                    "raids in {}: {detail}",
                    self.level_key(*dimension).resource_id()
                ),
            )));
        }
    }

    pub(in crate::world) fn apply_raider_recruit(
        &mut self,
        dimension: &LevelId,
        pending: PendingRaiderRecruit,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let outcome =
            self.with_raid_level(dimension, |raids, view| raids.recruit_raider(pending, view))?;
        self.publish_raid_outcome(dimension, outcome, events);
        Ok(())
    }

    pub(in crate::world) fn apply_raider_hurt(
        &mut self,
        dimension: &LevelId,
        raid_id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let outcome =
            self.with_raid_level(dimension, |raids, view| raids.raider_hurt(raid_id, view))?;
        self.publish_raid_outcome(dimension, outcome, events);
        Ok(())
    }

    fn advance_level_raids(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let raids_rule = self.game_rules.raids;
        let outcome =
            self.with_raid_level(dimension, |raids, view| raids.tick(raids_rule, view))?;
        self.publish_raid_outcome(dimension, outcome, events);
        Ok(())
    }

    /// One level's raid manager beside the view of that level it reads and
    /// spawns through, which share the borrows the world hands them.
    fn with_raid_level<T>(
        &mut self,
        dimension: &LevelId,
        body: impl FnOnce(&mut super::super::raids::LevelRaids, &mut RaidLevelRuntime) -> T,
    ) -> Result<T, String> {
        let difficulty = self.admin.settings().difficulty;
        // `Entity.blockPosition` floors each coordinate of the session
        // `validPlayer` reads (`Raid.java`).
        let players: Vec<RaidPlayer> = self.levels[dimension]
            .player_order
            .iter()
            .filter_map(|connection| Some((*connection, self.levels.player(*connection)?)))
            .map(|(connection, player)| RaidPlayer {
                connection,
                position: BlockPos {
                    x: player.position().x.floor() as i32,
                    y: player.position().y.floor() as i32,
                    z: player.position().z.floor() as i32,
                },
                precise: player.position(),
                alive: !player.avatar_removed && !player.damage.is_dead_or_dying(),
            })
            .collect();
        let game_time = self.game_time;
        let ravager = spawn_placement("minecraft:ravager");
        let dimension_key = self.level_key(*dimension).clone();
        let clock = self
            .registries
            .registry_element_index("minecraft:world_clock", "minecraft:overworld")
            .and_then(|index| usize::try_from(index).ok())
            .and_then(|index| self.world_clock_total_ticks(index))
            .unwrap_or(0);
        let dimension_type = self.cold.level_dimension_types.get(dimension).cloned();
        let facts = {
            let level = self.levels.get(dimension).ok_or("missing level")?;
            let mut facts = self
                .level_spawn_dimension_facts(dimension, level.weather.is_thundering())
                .ok_or("missing dimension spawn facts")?;
            facts.sky_darken = level
                .sky_darkening
                .ok_or("missing committed sky darkening")?;
            facts
        };
        {
            let ServerWorld {
                cold,
                levels,
                maps,
                registries,
                datapack_tables,
                world_clocks,
                scoreboard,
                ..
            } = self;
            let super::super::WorldCold {
                loot_item_tags,
                config,
                seed,
                generate_structures,
                dimension_sky_lights,
                command_storage,
                ..
            } = &mut **cold;
            let enchantment_registry = &datapack_tables.enchantment_registry;
            let spawn_environment = dimension_sky_lights.spawn_context(
                dimension_type.as_deref(),
                registries,
                world_clocks,
            );
            let level_names = Arc::clone(&levels.names);
            let (level, others) = levels.split_mut(dimension).ok_or("missing level")?;
            let ServerLevel {
                raids,
                chunks,
                entities,
                random: world_random,
                simulation_levels,
                world_border,
                weather,
                sky_darkening,
                ..
            } = level;
            let mut view = RaidLevelRuntime {
                runtime: MinecartSpawnerRuntime::new(
                    chunks,
                    maps,
                    *generate_structures,
                    entities,
                    &[],
                    difficulty,
                    dimension_key,
                    registries,
                    config.datapacks.as_deref(),
                    enchantment_registry,
                    loot_item_tags,
                    config.datapacks.as_deref().map(|packs| &packs.loot),
                    *seed,
                    game_time,
                    clock,
                    false,
                    Some(facts),
                    spawn_environment,
                    scoreboard,
                    level_names,
                    LootEntityLevels::Other(others),
                    world_clocks,
                    dimension_sky_lights.loot_environment_context(
                        dimension_type.as_deref(),
                        registries,
                        world_clocks,
                        ScalarWeather::from_level(weather),
                    ),
                    (*sky_darkening).map(i32::from),
                    LootWeather {
                        raining: weather.is_raining(),
                        thundering: weather.is_thundering(),
                    },
                    command_storage,
                ),
                level_random: world_random,
                simulation_levels,
                world_border: *world_border,
                players,
                difficulty,
                game_time,
                ravager,
            };
            Ok(body(raids, &mut view))
        }
    }

    /// The wave spawn's own publication, for a test that wants one raider
    /// rather than a whole pass.
    #[cfg(test)]
    fn spawn_one_raider(
        &mut self,
        dimension: &LevelId,
        spawn: RaidSpawn,
    ) -> Result<RaidSpawnOutcome, String> {
        self.with_raid_level(dimension, |_, view| view.spawn_raider(spawn))
    }

    fn publish_raid_outcome(
        &mut self,
        dimension: &LevelId,
        outcome: RaidTickOutcome,
        events: &mut Vec<WorldEvent>,
    ) {
        for (connection_id, packet) in outcome.boss_packets.into_iter().chain(outcome.sounds) {
            events.push(WorldEvent::Send {
                connection_id,
                packet,
            });
        }
        let mut refusal = |detail: String| {
            events.push(WorldEvent::ConsoleMessage(RichTextComponent::plain(detail)));
        };
        for id in outcome.unanswered_village {
            refusal(format!(
                "raid {id}: no village answer for its centre in {}",
                self.level_key(*dimension).resource_id()
            ));
        }
        for id in outcome.unanswered_wave_spawn {
            refusal(format!(
                "raid {id}: no surface answer for its wave spawn search in {}",
                self.level_key(*dimension).resource_id()
            ));
        }
        for (id, detail) in outcome.refused_raiders {
            refusal(format!(
                "raid {id} in {}: {detail}",
                self.level_key(*dimension).resource_id()
            ));
        }
        let level = self.level_mut_by_id(dimension);
        for uuid in outcome.unbound {
            let Some(id) = level.entities.entity_id_for_uuid(uuid) else {
                continue;
            };
            if let Some(entity) = level.entities.get_mut(id) {
                crate::entity::clear_raider_raid(entity);
            }
        }
        self.award_raid_heroes(dimension, outcome.heroes, events);
    }
}

#[cfg(test)]
mod tests;
