//! The loot context this world's drops read: the server-wide tables and random
//! sequences one call ticks with, and the two attacker views a damage source
//! carries.
//!
//! Vanilla's `DamageSource` holds `LootContextParams.ATTACKING_ENTITY` and
//! `DIRECT_ATTACKING_ENTITY` as `Entity` objects (`LivingEntity.java:1541-1542`).
//! This server carries their ids, and the one entity no
//! [`EntityStore`](crate::entity::EntityStore) holds is a session player:
//! the held stack a `minecraft:entity_properties` `equipment` term tests and the
//! enchantment levels `EnchantmentHelper.getEnchantmentLevel` answers live in
//! the session, so the level builds that view here and hands it to the damage
//! source the entity crate reads.

use crate::world::LevelId;
pub(in crate::world) mod player_nbt;
pub(in crate::world) use player_nbt::PlayerLootNbtSources;
pub(in crate::world) mod entity_scoreboard;
use entity_scoreboard::LootEntityScoreboard;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::enchantment_facts::EnchantmentSlots;
use crate::entity::block_drops::{
    loot_tool_for_runtime_stack, unresolved_tool_enchantments, BlockDropTables, RecipeDropFacts,
};
use crate::equipment_drop_effects::EquipmentDropEffects;
use crate::loot_runtime::{
    LootEntityAnswer, LootEntityFacts, LootEntityMovement, LootEntityPosition,
    LootEnvironmentAttributeValue, LootOrigin, LootRuntimeReason, LootWeather,
};
use bbb_protocol::equipment_slots::{EquipmentSlot, VANILLA_EQUIPMENT_SLOTS};
use bbb_protocol::game_types::GameType;
use bbb_server_datapack::startup::DatapackStartup;
use uuid::Uuid;

use super::command_storage::ServerCommandStorage;
use super::environment_attributes::{DimensionSkyLights, LootEnvironmentContext, ScalarWeather};
use super::natural_spawn::chunk_generation::OriginalMobWorldInputs;
use super::registry_runtime::EntityNbtReadSnapshot;
use super::registry_runtime::PlayerItemStackResolver;

use super::{ConnectionId, PlayerState, ServerLevel, ServerWorld};
use crate::world::raids::LevelRaids;

pub(crate) struct ValueEffectLevel<'a> {
    pub(crate) weather: LootWeather,
    clocks: Option<&'a super::world_clock::ServerWorldClocks>,
    chunks: Option<&'a super::ChunkStore>,
    entities: Option<&'a crate::entity::EntityStore>,
    attributes: Option<ValueEffectAttributes<'a>>,
}

enum ValueEffectAttributes<'a> {
    Prepared(LootEnvironmentContext<'a>),
    World {
        environment: &'a DimensionSkyLights,
        dimension_type: Option<&'a str>,
        registries: &'a crate::registry_sync::SynchronizedRegistries,
        clocks: &'a super::world_clock::ServerWorldClocks,
        weather: ScalarWeather,
    },
}

impl<'a> ValueEffectAttributes<'a> {
    fn sample(
        &self,
        attribute: &'static bbb_facts::environment_attribute_types::VanillaEnvironmentAttribute,
        chunks: &super::ChunkStore,
        origin: Option<LootOrigin>,
    ) -> Option<LootEnvironmentAttributeValue<'a>> {
        match self {
            Self::Prepared(context) => context.sample(attribute, chunks, origin),
            Self::World {
                environment,
                dimension_type,
                registries,
                clocks,
                weather,
            } => environment
                .loot_environment_context(*dimension_type, registries, clocks, *weather)
                .sample(attribute, chunks, origin),
        }
    }
}

impl<'a> ValueEffectLevel<'a> {
    pub(super) fn for_simulation(
        weather: LootWeather,
        chunks: &'a super::ChunkStore,
        clocks: Option<&'a super::world_clock::ServerWorldClocks>,
        attributes: Option<LootEnvironmentContext<'a>>,
    ) -> Self {
        Self {
            weather,
            clocks,
            attributes: attributes.map(ValueEffectAttributes::Prepared),
            chunks: Some(chunks),
            entities: None,
        }
    }

    fn for_world(
        chunks: &'a super::ChunkStore,
        entities: Option<&'a crate::entity::EntityStore>,
        weather: &super::weather::LevelWeather,
        dimension_type: Option<&'a str>,
        registries: &'a crate::registry_sync::SynchronizedRegistries,
        clocks: &'a super::world_clock::ServerWorldClocks,
        environment: &'a DimensionSkyLights,
    ) -> Self {
        Self {
            weather: weather.loot_weather(),
            clocks: Some(clocks),
            attributes: Some(ValueEffectAttributes::World {
                environment,
                dimension_type,
                registries,
                clocks,
                weather: ScalarWeather::from_level(weather),
            }),
            chunks: Some(chunks),
            entities,
        }
    }

    #[cfg(test)]
    pub(crate) const fn weather_only(weather: LootWeather) -> Self {
        Self {
            weather,
            clocks: None,
            attributes: None,
            chunks: None,
            entities: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_entities(mut self, entities: &'a crate::entity::EntityStore) -> Self {
        self.entities = Some(entities);
        self
    }

    pub(crate) fn entity_for_relations<'s>(
        &'s self,
        entity: crate::loot_runtime::LootEntity<'_>,
        bodies: Option<crate::entity::store::EntityBodyReadView<'s>>,
    ) -> Option<crate::loot_runtime::StoredLootEntity<'s>> {
        let crate::loot_runtime::LootEntity::Facts(facts) = entity else {
            return None;
        };
        if facts.player_uuid.is_some() {
            return None;
        }
        let uuid = facts.entity_uuid?;
        let bodies = bodies.or_else(|| {
            self.entities
                .map(|store| crate::entity::store::EntityBodyReadView::new(store, None, None))
        })?;
        let found = bodies.get_by_uuid(uuid)?;
        if found.uuid() != uuid
            || found.data().entity_resource_id() != facts.entity_type?
            || matches!(found.data(), crate::entity::EntityData::Player)
        {
            return None;
        }
        Some(crate::loot_runtime::StoredLootEntity {
            entity: found,
            bodies: Some(bodies),
            raid_at_position: None,
        })
    }

    pub(crate) fn clock_total_ticks(&self, clock: &str) -> Result<i64, LootRuntimeReason> {
        self.clocks
            .and_then(|clocks| clocks.total_ticks_for_clock(clock))
            .ok_or_else(|| {
                LootRuntimeReason::Catalog("time condition requires an actual world clock".into())
            })
    }

    pub(crate) fn environment_attribute(
        &self,
        attribute: &'static bbb_facts::environment_attribute_types::VanillaEnvironmentAttribute,
        origin: Option<LootOrigin>,
    ) -> Result<LootEnvironmentAttributeValue<'a>, LootRuntimeReason> {
        self.attributes
            .as_ref()
            .zip(self.chunks)
            .and_then(|(attributes, chunks)| attributes.sample(attribute, chunks, origin))
            .ok_or_else(|| {
                LootRuntimeReason::Catalog(
                    "environment attribute requires the actual level environment".into(),
                )
            })
    }
}

impl ServerWorld {
    pub(in crate::world) fn value_effect_level(&self, dimension: &LevelId) -> ValueEffectLevel<'_> {
        let level = self.level_by_id(dimension);
        ValueEffectLevel::for_world(
            &level.chunks,
            Some(&level.entities),
            &level.weather,
            self.cold
                .level_dimension_types
                .get(dimension)
                .map(String::as_str),
            &self.registries,
            &self.world_clocks,
            &self.cold.dimension_sky_lights,
        )
    }

    pub(in crate::world) fn value_effect_level_and_entities(
        &mut self,
        dimension: &LevelId,
    ) -> (ValueEffectLevel<'_>, &mut crate::entity::EntityStore) {
        let level = self
            .levels
            .get_mut(dimension)
            .expect("every reachable dimension is a level this world runs");
        let reads = ValueEffectLevel::for_world(
            &level.chunks,
            None,
            &level.weather,
            self.cold
                .level_dimension_types
                .get(dimension)
                .map(String::as_str),
            &self.registries,
            &self.world_clocks,
            &self.cold.dimension_sky_lights,
        );
        (reads, &mut level.entities)
    }
}

pub(in crate::world) struct CallerEnchantmentReads<'a> {
    pub dimension: &'a str,
    pub chunks: &'a super::ChunkStore,
    pub weather: crate::loot_runtime::LootWeather,
    pub random: &'a crate::random::GameRandom,
    pub environment_attributes: super::environment_attributes::LootEnvironmentContext<'a>,
    pub sky_darkening: Option<i32>,
}

/// The server-wide facts a loot draw reads that [`BlockDropTables`] cannot
/// carry, because the entity crate owns that struct and these are the world's
/// rather than a level's.
///
/// They travel beside the drop tables for the same reason those do: both are
/// the world's rather than the level's. `ServerLevel.getRaidAt` reads the
/// `Raids` saved data (`ServerLevel.java:1554-1556`), the publication a
/// `minecraft:set_components` value is decoded against is the same
/// [`PlayerItemStackResolver`] the `/give` and `/item` surfaces validate with
/// and the save request encodes through, the enchantment registry is the one
/// [`ServerWorld::enchantment_registry`] resolves a tool's holder ids against,
/// and the datapack generation is the one the loot registries themselves came
/// from.
pub(in crate::world) struct LootLevelFacts<'a> {
    pub(in crate::world) maps: Option<RefCell<&'a mut super::maps::ServerMapStore>>,
    pub(in crate::world) world_seed: i64,
    pub(in crate::world) registries: Option<&'a crate::registry_sync::SynchronizedRegistries>,
    pub(in crate::world) sky_darkening: Option<i32>,
    pub(in crate::world) generate_structures: bool,
    pub(in crate::world) other_levels: Option<super::level_keys::OtherLevels<'a>>,
    pub(in crate::world) entity_nbt: Option<EntityNbtReadSnapshot>,
    pub(in crate::world) command_storage: Option<&'a ServerCommandStorage>,
    pub(in crate::world) dimension: Option<LevelId>,
    pub(in crate::world) dimension_key: Option<Arc<bbb_server_types::save::Java26LevelKey>>,
    pub(in crate::world) scoreboard: Option<&'a super::scoreboard::Scoreboard>,
    pub(in crate::world) clocks: Option<&'a super::world_clock::ServerWorldClocks>,
    pub(in crate::world) environment_attributes:
        Option<super::environment_attributes::LootEnvironmentContext<'a>>,
    pub(in crate::world) level_names: Option<Arc<super::level_keys::LevelNames>>,
    /// `ServerLevel.getRaidAt(BlockPos)` over this world's raid map, which is
    /// the second disjunct of `Raider.hasRaid` (`Raider.java:156-158`).
    pub(in crate::world) raids: Option<&'a LevelRaids>,
    /// The generation the loot registries came from, which is also what
    /// `EnchantWithLevelsFunction.run` reads `context.getLevel().registryAccess()`
    /// for (`EnchantWithLevelsFunction.java:74-78`): the `minecraft:enchantment`
    /// registry elements, not just the ids
    /// [`Self::enchantment_registry`] carries.
    pub(in crate::world) datapacks: Option<&'a DatapackStartup>,
    /// The publication live item stacks resolve against.
    pub(in crate::world) item_stack_resolver: Option<PlayerItemStackResolver>,
    /// `RegistryAccess.lookupOrThrow(Registries.ENCHANTMENT)`: every
    /// `minecraft:enchantment` resource id of the loaded generation, in
    /// registry order, which is the order a stack's `minecraft:enchantments`
    /// holder ids index into. `None` on a slice a caller assembled without the
    /// world's publication, where a term that reads a held stack's
    /// enchantments stays a counted refusal rather than resolving the ids
    /// against a generation that did not issue them.
    pub(in crate::world) enchantment_registry: Option<Arc<Vec<String>>>,
    pub(in crate::world) equipment_drop_effects: Option<Arc<Vec<Option<EquipmentDropEffects>>>>,
    pub(in crate::world) enchantment_slots: Option<Arc<Vec<EnchantmentSlots>>>,
    pub(in crate::world) enchantment_combat_effects:
        Option<Arc<crate::enchantment_combat_effects::EnchantmentCombatTable>>,
    pub(in crate::world) enchantment_item_damage:
        Option<Arc<Vec<Option<crate::item_durability::EnchantmentValueEffects>>>>,
    pub(in crate::world) advancement_tree: Option<&'a super::AdvancementTree>,
}

impl<'a> LootLevelFacts<'a> {
    /// Every fact absent: a level a caller assembles for a draw that reads
    /// none of them, where each stays a counted refusal rather than a `false`
    /// nothing checked.
    pub(in crate::world) fn absent() -> LootLevelFacts<'a> {
        LootLevelFacts {
            maps: None,
            world_seed: 0,
            registries: None,
            sky_darkening: None,
            generate_structures: false,
            other_levels: None,
            entity_nbt: None,
            command_storage: None,
            dimension: None,
            dimension_key: None,
            scoreboard: None,
            clocks: None,
            environment_attributes: None,
            level_names: None,
            raids: None,
            datapacks: None,
            item_stack_resolver: None,
            enchantment_registry: None,
            equipment_drop_effects: None,
            enchantment_slots: None,
            enchantment_combat_effects: None,
            enchantment_item_damage: None,
            advancement_tree: None,
        }
    }

    /// The online player `uuid` names, in `current` or another level.
    pub(in crate::world) fn player_by_uuid<'s>(
        &'s self,
        current: Option<&'s crate::entity::EntityStore>,
        uuid: Uuid,
    ) -> Option<&'s PlayerState> {
        self.player_and_store_by_uuid(current, uuid)
            .map(|(player, _)| player)
    }

    pub(in crate::world) fn session_loot_view(
        &self,
        bodies: Option<crate::entity::store::EntityBodyReadView<'_>>,
        tags: &crate::entity::EntityRegistryTagView,
        uuid: Uuid,
    ) -> Result<Option<LootEntityFacts>, String> {
        let Some((player, store)) =
            self.player_and_store_by_uuid(bodies.map(|bodies| bodies.store()), uuid)
        else {
            return Ok(None);
        };
        let registry = self
            .enchantment_registry
            .as_ref()
            .ok_or("enchantment registry is unavailable")?;
        let slots = self
            .enchantment_slots
            .as_ref()
            .ok_or("enchantment slots are unavailable")?;
        let mut view = player_loot_facts(player, registry, slots);
        let player_bodies = bodies
            .filter(|bodies| bodies.store().player_by_uuid(uuid).is_some())
            .unwrap_or_else(|| crate::entity::store::EntityBodyReadView::new(store, None, None));
        view.vehicle = player_vehicle_loot_facts(
            player,
            player_bodies,
            self.raids
                .filter(|_| self.dimension == Some(player.dimension)),
        );
        view.movement = ServerWorld::player_known_movement_in_bodies(player, player_bodies, tags)
            .map(|movement| LootEntityMovement {
                known_movement: [movement.x, movement.y, movement.z],
                fall_distance: player.block_contact.fall_distance,
            });
        Ok(Some(view))
    }

    pub(in crate::world) fn player_and_store_by_uuid<'s>(
        &'s self,
        current: Option<&'s crate::entity::EntityStore>,
        uuid: Uuid,
    ) -> Option<(&'s PlayerState, &'s crate::entity::EntityStore)> {
        current
            .and_then(|store| store.player_by_uuid(uuid).map(|player| (player, store)))
            .or_else(|| {
                self.other_levels.into_iter().find_map(|levels| {
                    levels.iter().find_map(|(_, level)| {
                        level
                            .entities
                            .player_by_uuid(uuid)
                            .map(|player| (player, &level.entities))
                    })
                })
            })
    }
}

impl<'a> LootLevelFacts<'a> {
    pub(in crate::world) fn entity_scoreboard<'s>(
        &'s self,
        bodies: Option<crate::entity::store::EntityBodyReadView<'s>>,
    ) -> Option<LootEntityScoreboard<'s>> {
        Some(LootEntityScoreboard {
            scoreboard: self.scoreboard?,
            names: self.level_names.as_deref()?,
            dimension: self.dimension?,
            bodies,
            other_levels: self.other_levels.map_or(
                entity_scoreboard::LootEntityLevels::Stores(&[]),
                entity_scoreboard::LootEntityLevels::Other,
            ),
        })
    }

    /// The same facts with `ServerLevel.raids` added, which only the caller
    /// that destructured that level can lend: [`ServerWorld::level_and_drop_tables`]
    /// hands the level out as `&mut` and so cannot also hold a shared borrow of
    /// one of its fields.
    pub(in crate::world) fn with_raids(mut self, raids: &'a LevelRaids) -> Self {
        self.raids = Some(raids);
        self
    }
}

impl LootLevelFacts<'_> {
    /// The publication as the value the entity crate's drop projection and
    /// equipment read take, which is the shape a level holding no world can
    /// carry.
    pub(in crate::world) fn item_stack_publication(
        &self,
    ) -> &'_ dyn bbb_registry_codec::PersistentItemStackHolderResolver {
        (self
            .item_stack_resolver
            .as_ref()
            .map(PlayerItemStackResolver::as_dyn))
        .unwrap_or(&bbb_registry_codec::UnavailableHolderResolver)
    }
}

/// The two session attackers one damage source needs, owned so the caller can
/// lend them to the source it is building.
pub(in crate::world) struct SessionAttackerViews {
    causing: Option<LootEntityFacts>,
    direct: Option<LootEntityFacts>,
}

impl SessionAttackerViews {
    pub(in crate::world) fn new(
        causing: Option<LootEntityFacts>,
        direct: Option<LootEntityFacts>,
    ) -> Self {
        Self { causing, direct }
    }

    pub(in crate::world) fn views(&self) -> crate::entity::StoredAttackerViews<'_> {
        crate::entity::StoredAttackerViews {
            causing: self.causing.as_ref(),
            direct: self.direct.as_ref(),
        }
    }
}

impl ServerWorld {
    pub(in crate::world) fn with_entity_predicate_level(
        &mut self,
        dimension: LevelId,
        param_set: &'static crate::loot_runtime::params::LootContextParamSet,
        run: impl for<'scope> FnOnce(
            crate::entity::store::EntityBodyReadView<'scope>,
            Option<&'scope bbb_server_datapack::loot::LootRegistries>,
            &'scope crate::random::GameRandom,
            crate::loot_runtime::LootParams<'scope>,
        ) -> Result<bool, String>,
    ) -> Result<bool, String> {
        use super::level_blocks::LoadedBlocks;
        use super::simulation_level::mob_death_loot::LevelLootEnvironment;
        use crate::loot_runtime::{LootParams, LootRuntimeReason, LootWeather};
        use entity_scoreboard::LootEntityLevels;

        let startup = self.cold.config.datapacks.as_ref();
        let Some((level, others)) = self.levels.split_mut(&dimension) else {
            return Err("the level is not loaded".to_string());
        };
        let names = others.names();
        let ServerLevel {
            chunks,
            entities,
            random: world_random,
            weather,
            dimension: level_dimension,
            sky_darkening,
            ..
        } = level;
        let entities = &*entities;
        let world_random = &*world_random;
        let weather = &*weather;
        let bodies = crate::entity::store::EntityBodyReadView::new(entities, None, None);
        let player_slots = |uuid, slot| {
            let player = entities
                .player_by_uuid(uuid)
                .or_else(|| {
                    others
                        .iter()
                        .find_map(|(_, level)| level.entities.player_by_uuid(uuid))
                })
                .ok_or_else(|| {
                    LootRuntimeReason::Catalog("player slots need an online player".into())
                })?;
            Ok(player_nbt::read_player_slot(
                player,
                slot,
                &self.datapack_tables.enchantment_slots,
            ))
        };
        let player_nbt = |uuid| {
            let resolver =
                super::registry_runtime::PlayerItemStackResolver::new(startup, &self.registries);
            player_nbt::read_player_nbt_with_levels(
                uuid,
                names,
                &resolver,
                Some(&dimension),
                Some(bodies),
                LootEntityLevels::Other(others),
            )
        };
        let chunks = std::cell::RefCell::new(&mut *chunks);
        let maps = std::cell::RefCell::new(&mut self.maps);
        let actual_weather = LootWeather {
            raining: weather.is_raining(),
            thundering: weather.is_thundering(),
        };
        let item_environment = LevelLootEnvironment::for_world_predicates(
            super::level_blocks::LootChunks::Chunks(&chunks),
            entities.item_resolver(),
            startup.map(|packs| packs.as_ref()),
            self.datapack_tables.enchantment_registry.as_slice(),
            startup.map(|packs| crate::entity::block_drops::RecipeDropFacts {
                recipes: &packs.recipes,
                recipe_index: &packs.recipe_index,
                item_tags: &self.cold.loot_item_tags,
            }),
            &self.world_clocks,
            &self.cold.command_storage,
            &self.registries,
            self.cold.seed,
            sky_darkening.map(i32::from),
            self.cold.dimension_sky_lights.loot_environment_context(
                self.cold
                    .level_dimension_types
                    .get(&dimension)
                    .map(String::as_str),
                &self.registries,
                &self.world_clocks,
                super::environment_attributes::ScalarWeather::from_level(weather),
            ),
            LootEntityScoreboard {
                scoreboard: self.scoreboard.board(),
                names,
                dimension,
                bodies: Some(bodies),
                other_levels: LootEntityLevels::Other(others),
            },
        )
        .with_player_slots(&player_slots)
        .with_player_nbt(&player_nbt)
        .with_advancement_tree(&self.cold.advancement_tree)
        .with_maps(
            &maps,
            self.cold.seed,
            level_dimension.as_str(),
            self.cold.generate_structures,
        );
        let loaded_block_state = |pos| {
            LoadedBlocks {
                chunks: &chunks.borrow(),
            }
            .loaded_block_state(pos)
        };
        let block_entity_data = |pos| {
            chunks
                .borrow()
                .block_entity_loot_data(pos, Some(entities.item_resolver()))
                .map_err(|error| LootRuntimeReason::Catalog(error.to_string()))
        };
        let loaded_sky = |pos| {
            LoadedBlocks {
                chunks: &chunks.borrow(),
            }
            .loaded_can_see_sky(pos)
        };
        // Predicate roots stay in this store; other levels are directory reads.
        let vehicle_slots = |requested: &crate::entity::Entity, selected: &[i32]| {
            let actual = bodies
                .get(requested.id())
                .filter(|actual| actual.uuid() == requested.uuid())
                .ok_or_else(|| {
                    LootRuntimeReason::Catalog(
                        "vehicle slots need the actual source level/body".into(),
                    )
                })?;
            let startup = startup.ok_or_else(|| {
                LootRuntimeReason::Catalog("no loot registries are loaded".into())
            })?;
            super::simulation_level::block_entity_loot::vehicle_slots_with_environment(
                &startup.loot,
                Some(&item_environment),
                Some(level_dimension.as_str()),
                &loaded_block_state,
                &block_entity_data,
                actual_weather,
                world_random,
                Some(bodies),
                actual,
                selected,
            )
        };
        let base = LootParams {
            item_environment: Some(&item_environment),
            vehicle_slots: Some(&vehicle_slots),
            dimension: Some(level_dimension.as_str()),
            level: Some(&loaded_block_state),
            block_entity_data: Some(&block_entity_data),
            sky: Some(&loaded_sky),
            weather: Some(actual_weather),
            ..LootParams::new(param_set)
        };
        run(bodies, startup.map(|packs| &packs.loot), world_random, base)
    }

    pub(in crate::world) fn predicate_loot_environment<'a>(
        &'a self,
        level: &'a ServerLevel,
        dimension: LevelId,
    ) -> super::simulation_level::mob_death_loot::LevelLootEnvironment<'a, 'a, 'a> {
        super::simulation_level::mob_death_loot::LevelLootEnvironment::for_world_predicates(
            super::level_blocks::LootChunks::Shared(&level.chunks),
            level.entities.item_resolver(),
            self.cold.config.datapacks.as_deref(),
            self.datapack_tables.enchantment_registry.as_slice(),
            self.cold
                .config
                .datapacks
                .as_deref()
                .map(|packs| RecipeDropFacts {
                    recipes: &packs.recipes,
                    recipe_index: &packs.recipe_index,
                    item_tags: &self.cold.loot_item_tags,
                }),
            &self.world_clocks,
            &self.cold.command_storage,
            &self.registries,
            self.cold.seed,
            level.sky_darkening.map(i32::from),
            self.cold.dimension_sky_lights.loot_environment_context(
                self.cold
                    .level_dimension_types
                    .get(&dimension)
                    .map(String::as_str),
                &self.registries,
                &self.world_clocks,
                super::environment_attributes::ScalarWeather::from_level(&level.weather),
            ),
            LootEntityScoreboard {
                scoreboard: self.scoreboard.board(),
                names: &self.levels.names,
                dimension,
                bodies: Some(crate::entity::store::EntityBodyReadView::new(
                    &level.entities,
                    None,
                    None,
                )),
                other_levels: entity_scoreboard::LootEntityLevels::World(&self.levels),
            },
        )
        .with_advancement_tree(&self.cold.advancement_tree)
    }

    pub(in crate::world) fn world_vehicle_slots(
        &self,
        player: Option<&PlayerState>,
        requested: &crate::entity::Entity,
        selected: &[i32],
    ) -> Result<
        Vec<bbb_server_types::item_stack::RuntimeItemStack>,
        crate::loot_runtime::LootRuntimeReason,
    > {
        use super::level_blocks::LoadedBlocks;
        use crate::loot_runtime::{LootRuntimeReason, LootWeather};

        let startup =
            self.cold.config.datapacks.as_deref().ok_or_else(|| {
                LootRuntimeReason::Catalog("no loot registries are loaded".into())
            })?;
        let (dimension, level, actual) = self
            .levels
            .iter()
            .find_map(|(dimension, level)| {
                level
                    .entities
                    .get(requested.id())
                    .filter(|actual| actual.uuid() == requested.uuid())
                    .map(|actual| (dimension, level, actual))
            })
            .ok_or_else(|| {
                LootRuntimeReason::Catalog("vehicle slots need the actual source level/body".into())
            })?;
        let blocks = LoadedBlocks {
            chunks: &level.chunks,
        };
        let block_at = |pos| blocks.loaded_block_state(pos);
        let block_entity_data = |pos| {
            blocks
                .chunks
                .block_entity_loot_data(pos, Some(level.entities.item_resolver()))
                .map_err(|error| LootRuntimeReason::Catalog(error.to_string()))
        };
        let player_slots = |uuid, slot| self.player_loot_slot(player, uuid, slot);
        let player_nbt = |uuid| self.player_loot_nbt(uuid);
        let item_environment = self
            .predicate_loot_environment(level, dimension)
            .with_player_slots(&player_slots)
            .with_player_nbt(&player_nbt);
        let weather = LootWeather {
            raining: level.weather.is_raining(),
            thundering: level.weather.is_thundering(),
        };
        let bodies = crate::entity::store::EntityBodyReadView::new(&level.entities, None, None);
        super::simulation_level::block_entity_loot::vehicle_slots_with_environment(
            &startup.loot,
            Some(&item_environment),
            Some(level.dimension.as_str()),
            &block_at,
            &block_entity_data,
            weather,
            &level.random,
            Some(bodies),
            actual,
            selected,
        )
    }

    /// The level one call ticks, borrowed together with the server-wide inputs
    /// `Block.dropResources` needs and the three facts of [`LootLevelFacts`].
    ///
    /// They come out of one call because they have different owners: the level
    /// is one of [`Self::levels`], while the loot registries,
    /// `MinecraftServer.randomSequences` (`MinecraftServer.java:274`, `:343`),
    /// the raid map and the item-stack publication belong to the server and are
    /// shared by every level it runs.
    pub(in crate::world) fn level_and_drop_tables(
        &mut self,
        dimension: &LevelId,
    ) -> (
        &mut ServerLevel,
        BlockDropTables<'_>,
        LootLevelFacts<'_>,
        super::simulation_level::structure_block::StructureBlockServices<'_>,
    ) {
        let (level, tables, facts, _, structures, _) =
            self.level_drop_tables_and_sculk_warning(dimension);
        (level, tables, facts, structures)
    }

    pub(in crate::world) fn level_drop_tables_and_sculk_warning(
        &mut self,
        dimension: &LevelId,
    ) -> (
        &mut ServerLevel,
        BlockDropTables<'_>,
        LootLevelFacts<'_>,
        super::simulation_level::SculkWarningRuntime,
        super::simulation_level::structure_block::StructureBlockServices<'_>,
        OriginalMobWorldInputs<'_>,
    ) {
        let (level, tables, facts, sculk, structures, originals, _) =
            self.level_drop_tables_and_sculk_warning_with_caller(dimension, None);
        (level, tables, facts, sculk, structures, originals)
    }

    pub(in crate::world) fn level_drop_tables_and_sculk_warning_with_caller(
        &mut self,
        dimension: &LevelId,
        caller: Option<&LevelId>,
    ) -> (
        &mut ServerLevel,
        BlockDropTables<'_>,
        LootLevelFacts<'_>,
        super::simulation_level::SculkWarningRuntime,
        super::simulation_level::structure_block::StructureBlockServices<'_>,
        OriginalMobWorldInputs<'_>,
        Option<CallerEnchantmentReads<'_>>,
    ) {
        let entity_nbt = self.entity_nbt_read_snapshot();
        let fast_lava = self.fast_lava_dimension_value(dimension);
        let monsters_burn = self.monsters_burn(dimension);
        let piglins_zombify = self.piglins_zombify(dimension);
        let water_evaporates = self.water_evaporates(dimension);
        let bees_stay_in_hive = self.bees_stay_in_hive_values(dimension);
        let villager_activity = [false, true].map(|baby| {
            self.villager_schedule_activity_in(dimension, "", baby)
                .and_then(crate::entity::ai::Activity::from_resource_id)
        });
        // `Timeline.getPeriodCount` over `day.json`'s 24000-tick period.
        let overworld_day = self
            .cold
            .dimension_sky_lights
            .spawn_context(None, &self.registries, &self.world_clocks)
            .overworld_clock_time()
            .map_or(0, |ticks| ticks / 24_000);
        // `MinecraftServer.getRandomSequence` resolves against
        // `worldGenSettings.options().seed()` (`MinecraftServer.java:1758-1760`).
        let world_seed = self.cold.seed;
        // Read before the level borrow below, so this immutable read of the
        // world does not overlap the mutable one that hands out the level.
        let item_stack_resolver = Some(self.item_stack_resolver());
        // The same generation the loot registries below come from, which is what
        // `context.getLevel().registryAccess()` answers on the death path.
        let datapacks = self.cold.config.datapacks.as_deref();
        // Read with the level borrow below for the same reason: the registry is
        // the world's, and the arc keeps it alive past that borrow.
        let enchantment_registry = Some(Arc::clone(&self.datapack_tables.enchantment_registry));
        let tables = self
            .cold
            .config
            .datapacks
            .as_deref()
            .map(|packs| &packs.loot);
        let item_tags = &self.cold.loot_item_tags;
        let recipes = self
            .cold
            .config
            .datapacks
            .as_deref()
            .map(|packs| RecipeDropFacts {
                recipes: &packs.recipes,
                recipe_index: &packs.recipe_index,
                item_tags,
            });
        // `DamageSourcePredicate`'s `tags` term reads `Holder.is(TagKey)` for a
        // `minecraft:damage_type` tag, and the built tags of the generation the
        // loot registries above came from are the authority for it. A world
        // with no generation evaluates no table at all (the registry is absent),
        // so there is nothing for a missing tag report to answer.
        let damage_type_tags = self.cold.config.datapacks.as_deref().map(|packs| {
            packs
                .tags
                .registry(bbb_server_datapack::damage::DAMAGE_TYPE_REGISTRY)
        });
        let originals = super::natural_spawn::chunk_generation::OriginalMobWorldInputs::from_parts(
            &self.cold.config,
            &self.cold.mob_spawns,
            (
                &self.registries,
                &self.cold.dimension_sky_lights,
                &self.world_clocks,
                &self.cold.level_dimension_types,
            ),
            &self.admin,
            (self.game_rules, self.cold.seed, self.game_time, *dimension),
        );
        let level_names = Arc::clone(&self.levels.names);
        let (level, other_levels) = self
            .levels
            .split_mut(dimension)
            .expect("every reachable dimension is a level this world runs");
        let caller_reads = caller.and_then(|caller| {
            other_levels
                .iter()
                .find(|(key, _)| key == caller)
                .map(|(key, level)| CallerEnchantmentReads {
                    dimension: level.dimension.as_str(),
                    chunks: &level.chunks,
                    weather: crate::loot_runtime::LootWeather {
                        raining: level.weather.is_raining(),
                        thundering: level.weather.is_thundering(),
                    },
                    random: &level.random,
                    environment_attributes: self
                        .cold
                        .dimension_sky_lights
                        .loot_environment_context(
                            self.cold
                                .level_dimension_types
                                .get(&key)
                                .map(String::as_str),
                            &self.registries,
                            &self.world_clocks,
                            super::environment_attributes::ScalarWeather::from_level(
                                &level.weather,
                            ),
                        ),
                    sky_darkening: level.sky_darkening.map(i32::from),
                })
        });
        let environment_weather =
            crate::world::environment_attributes::ScalarWeather::from_level(&level.weather);
        let dimension_key = Arc::clone(&level.dimension);
        let sky_darkening = level.sky_darkening.map(i32::from);
        (
            level,
            BlockDropTables {
                tables,
                recipes,
                damage_type_tags,
            },
            LootLevelFacts {
                maps: Some(RefCell::new(&mut self.maps)),
                world_seed,
                registries: Some(&self.registries),
                sky_darkening,
                generate_structures: self.cold.generate_structures,
                other_levels: Some(other_levels),
                entity_nbt: Some(entity_nbt.clone()),
                command_storage: Some(&self.cold.command_storage),
                dimension: Some(*dimension),
                dimension_key: Some(dimension_key),
                scoreboard: Some(self.scoreboard.board()),
                clocks: Some(&self.world_clocks),
                environment_attributes: Some(
                    self.cold.dimension_sky_lights.loot_environment_context(
                        self.cold
                            .level_dimension_types
                            .get(dimension)
                            .map(String::as_str),
                        &self.registries,
                        &self.world_clocks,
                        environment_weather,
                    ),
                ),
                level_names: Some(level_names),
                raids: None,
                datapacks,
                item_stack_resolver,
                enchantment_registry,
                equipment_drop_effects: Some(Arc::clone(
                    &self.datapack_tables.equipment_drop_effects,
                )),
                enchantment_slots: Some(Arc::clone(&self.datapack_tables.enchantment_slots)),
                enchantment_combat_effects: Some(Arc::clone(
                    &self.datapack_tables.enchantment_combat_effects,
                )),
                enchantment_item_damage: Some(Arc::clone(
                    &self.datapack_tables.enchantment_item_damage,
                )),
                advancement_tree: Some(&self.cold.advancement_tree),
            },
            super::simulation_level::SculkWarningRuntime {
                difficulty: self.admin.settings().difficulty,
            },
            super::simulation_level::structure_block::StructureBlockServices {
                manager: &self.cold.structure_templates,
                entity_nbt,
                fast_lava,
                monsters_burn,
                piglins_zombify,
                water_evaporates,
                bees_stay_in_hive,
                villager_activity,
                overworld_day,
            },
            originals,
            caller_reads,
        )
    }

    /// The two views for a damage source whose entities the given players are:
    /// `LootContextParams.ATTACKING_ENTITY` from `causing`, and
    /// `DIRECT_ATTACKING_ENTITY` — the same player for a melee blow — from
    /// `direct`.
    pub(in crate::world) fn session_attacker_views(
        &self,
        causing: Option<Uuid>,
        direct: Option<Uuid>,
    ) -> SessionAttackerViews {
        SessionAttackerViews {
            causing: causing.and_then(|uuid| self.session_player_loot_view(uuid)),
            direct: direct.and_then(|uuid| self.session_player_loot_view(uuid)),
        }
    }

    /// One session player's facts, or `None` for a uuid no session holds.
    fn session_player_loot_view(&self, uuid: Uuid) -> Option<LootEntityFacts> {
        let connection_id = self.connection_of(uuid)?;
        self.session_loot_view(connection_id)
    }

    /// [`player_loot_facts`] plus the live movement and vehicle reads only the world can
    /// answer.
    pub(in crate::world) fn session_loot_view(
        &self,
        target: ConnectionId,
    ) -> Option<LootEntityFacts> {
        let player = self.levels.player(target)?;
        Some(self.loot_view_for_player(player))
    }

    pub(in crate::world) fn loot_view_for_player(&self, player: &PlayerState) -> LootEntityFacts {
        let level = self.levels.get(&player.dimension);
        let mut view = player_loot_view_in_bodies(
            player,
            level.map(|level| {
                crate::entity::store::EntityBodyReadView::new(&level.entities, None, None)
            }),
            level.map(|level| &level.raids),
            &self.datapack_tables.enchantment_registry,
            &self.datapack_tables.enchantment_slots,
            self.player_known_movement(player.connection_id),
        );
        if view.movement_block.is_none() {
            if let Some(level) = level {
                view.movement_block = player_movement_block_in_chunks(player, &level.chunks);
            }
        }
        view
    }
}

/// `Entity.getBlockPosBelowThatAffectsMyMovement` (`Entity.java:988-1007`) for
/// a session player against loaded chunks.
fn player_movement_block_in_chunks(
    player: &PlayerState,
    chunks: &crate::chunk::ChunkStore,
) -> Option<[i32; 3]> {
    let mut known = true;
    let pos = crate::entity::body_on_pos(
        player.position(),
        player.block_contact.supporting,
        0.500_001_f32,
        |pos| {
            let state_id = if chunks.is_inside_build_height(pos.y) {
                let Some(state) = chunks.get_block_state(pos.x, pos.y, pos.z) else {
                    known = false;
                    return crate::entity::SupportingBlockFacts {
                        state_id: chunks.air_state_id(),
                        is_fence: false,
                        is_wall: false,
                    };
                };
                state
            } else {
                chunks.air_state_id()
            };
            let block_id =
                bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state_id);
            let tags = chunks.registry_tags();
            let wall =
                block_id.and_then(|id| tags.map(|tags| tags.contains_block("minecraft:walls", id)));
            known &= matches!(wall, Some(true | false));
            crate::entity::SupportingBlockFacts {
                state_id,
                is_fence: false,
                is_wall: wall.unwrap_or(false),
            }
        },
    );
    known.then_some([pos.x, pos.y, pos.z])
}

pub(in crate::world) fn player_loot_view_in_bodies(
    player: &PlayerState,
    bodies: Option<crate::entity::store::EntityBodyReadView<'_>>,
    raids: Option<&LevelRaids>,
    enchantments: &Arc<Vec<String>>,
    slots: &Arc<Vec<EnchantmentSlots>>,
    known_movement: Option<bbb_protocol::packets::Vec3d>,
) -> LootEntityFacts {
    let mut view = player_loot_facts(player, enchantments, slots);
    if let Some(bodies) = bodies {
        view.vehicle = player_vehicle_loot_facts(player, bodies, raids);
    }
    view.movement = known_movement.map(|known| LootEntityMovement {
        known_movement: [known.x, known.y, known.z],
        fall_distance: player.block_contact.fall_distance,
    });
    view
}

pub(in crate::world) fn player_vehicle_loot_facts(
    player: &PlayerState,
    bodies: crate::entity::store::EntityBodyReadView<'_>,
    raids: Option<&LevelRaids>,
) -> Option<Option<Box<LootEntityFacts>>> {
    match bodies.external_player_vehicle(player.entity_id()) {
        None => Some(None),
        Some(id) => bodies.get(id).map(|entity| {
            Some(Box::new(
                crate::loot_runtime::StoredLootEntity {
                    entity,
                    bodies: Some(bodies),
                    raid_at_position: raids.map(|raids| raids.raid_at(entity.block_position())),
                }
                .snapshot_live(bodies.store().item_resolver()),
            ))
        }),
    }
}

/// One session player as the entity a loot table reads.
pub(in crate::world) fn player_loot_facts(
    player: &PlayerState,
    enchantment_registry: &Arc<Vec<String>>,
    enchantment_slots: &Arc<Vec<EnchantmentSlots>>,
) -> LootEntityFacts {
    let mut equipment = BTreeMap::new();
    let mut resolved = true;
    for (slot, stack) in player.equipment_slots() {
        if !unresolved_tool_enchantments(stack.summary(), enchantment_registry).is_empty() {
            resolved = false;
        }
        if stack.is_empty() {
            continue;
        }
        equipment.insert(
            slot,
            loot_tool_for_runtime_stack(stack, enchantment_registry),
        );
    }
    LootEntityFacts {
        entity_type: Some("minecraft:player"),
        living: Some(true),
        effects: Some(player.effects.clone()),
        position: Some(LootEntityPosition {
            x: player.position().x,
            y: player.position().y,
            z: player.position().z,
        }),
        on_pos: Some({
            let pos = crate::entity::supporting_block::body_get_on_pos(
                player.position(),
                player.block_contact.supporting,
            );
            [pos.x, pos.y, pos.z]
        }),
        supporting_block: player
            .block_contact
            .supporting
            .map(|pos| [pos.x, pos.y, pos.z]),
        // `Entity.java:988-1007`: without a supporting block, the 0.500001F offset
        // needs no block-state lookup.
        movement_block: player.block_contact.supporting.is_none().then(|| {
            let pos = crate::entity::body_on_pos(player.position(), None, 0.500_001_f32, |_| {
                unreachable!("no supporting block never reads block state")
            });
            [pos.x, pos.y, pos.z]
        }),
        flags: crate::world::advancement_triggers::session_loot_entity_flags(player),
        tick_count: Some(player.tick_count),
        slime_size: Some(LootEntityAnswer::NotApplicable),
        raider_flags: Some(LootEntityAnswer::NotApplicable),
        player: Some(player.loot_player_facts()),
        player_uuid: Some(player.profile.uuid),
        custom_name: Some(player.cold.custom_name.clone()),
        custom_data: Some(player.cold.custom_data.clone()),
        profile: Some(LootEntityAnswer::Value(
            super::command_execution::profile_nbt(&player.profile),
        )),
        equipment: resolved.then_some(equipment),
        enchantment_slots: Some(Arc::clone(enchantment_slots)),
        ..LootEntityFacts::default()
    }
}

impl PlayerState {
    pub(in crate::world) fn equipment_slots(
        &self,
    ) -> impl Iterator<
        Item = (
            EquipmentSlot,
            &bbb_server_types::item_stack::RuntimeItemStack,
        ),
    > {
        VANILLA_EQUIPMENT_SLOTS.iter().filter_map(move |entry| {
            let stack = match entry.slot {
                EquipmentSlot::MainHand => Some(self.inventory.selected_stack()),
                _ => entry
                    .player_inventory_slot
                    .and_then(|index| usize::try_from(index).ok())
                    .and_then(|index| self.inventory.slot(index)),
            };
            stack.map(|stack| (entry.slot, stack))
        })
    }
}

impl PlayerState {
    // ServerPlayer.java:1586-1625 does not copy Entity names, custom data or these flags.
    pub(in crate::world) fn reset_respawn_entity_components(&mut self) {
        self.has_visual_fire = false;
        self.cold.custom_name = None;
        self.cold.custom_data.clear();
        self.set_silent(false);
        self.silent_dirty = false;
        self.environment_first_tick = true;
        self.was_touching_water = false;
        self.remaining_fire_ticks = 0;
        self.set_air_supply(crate::entity::living::MAX_AIR_SUPPLY);
        self.set_ticks_frozen(0);
    }
}

impl PlayerState {
    // Mob.java:234-239; Player.java:723-725: recheck the actual player on each read.
    pub(crate) fn can_be_seen_as_mob_target(&self, peaceful: bool) -> bool {
        crate::entity::ai::mob::target::conditions::can_be_seen_as_enemy(
            peaceful,
            matches!(self.game_type, GameType::Creative | GameType::Spectator),
            false,
            self.abilities.invulnerable || self.entity_invulnerable,
            self.is_alive(),
        )
    }

    pub(crate) fn loot_player_facts(&self) -> crate::loot_runtime::LootPlayerFacts {
        crate::loot_runtime::LootPlayerFacts {
            experience_level: self.experience.level(),
            food_level: self.food.food_level(),
            saturation_level: self.food.saturation_level(),
            game_mode: self.game_type.serialized_name(),
        }
    }
}
