//! Loot table evaluation for stored mob deaths.

use crate::world::level_blocks::{LoadedBlocks, LootChunks};

use crate::entity::block_drops::{loot_tool_for_stack, unresolved_tool_enchantments};
use crate::entity::mob_death_loot::{MobLootRequest, MobLootRoll};
use crate::loot_runtime::{
    loot_table_by_id, loot_table_drops, matches_item_predicate, param_sets, LootDamageSource,
    LootDamageSourceFacts, LootEntities, LootEntity, LootEnvironmentAttributeValue,
    LootItemEnvironment, LootOrigin, LootParams, LootRuntimeReason, LootStack, LootWeather,
    StoredLootEntity,
};
use crate::random::GameRandom;
use bbb_facts::environment_attribute_types::VanillaEnvironmentAttribute;
use bbb_protocol::packets::ItemStackSummary;
use bbb_protocol::NbtSummaryValue;
use bbb_server_datapack::recipe::CookingKind;
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_types::item_stack::RuntimeItemStack;
use std::cell::RefCell;
use std::sync::Arc;

use crate::world::command_storage::ServerCommandStorage;
use crate::world::enchant_with_levels::{enchant_item, EnchantWithLevelsItemWorld};
use crate::world::loot_context::LootLevelFacts;
use crate::world::registry_runtime::EntityNbtReadSnapshot;
use crate::world::registry_runtime::PlayerItemStackResolver;
use crate::world::spawner_minecarts::SpawnEnchantmentProviderEnvironment;

impl super::LevelCtx<'_, '_> {
    /// `EnderMan.dropCustomDeathLoot` (`EnderMan.java:320-340`).
    pub(crate) fn enderman_block_loot_drops(
        &mut self,
        bodies: crate::entity::store::EntityBodyReadView<'_>,
        entity: &crate::entity::Entity,
        block_state_id: i32,
    ) -> Option<Vec<LootStack>> {
        let special = self
            .full_preparation
            .originals
            .local_special_multiplier(self.chunks, entity.block_position())?;
        let raid_at_position = self.raid_at(entity.block_position());
        let bodies = bodies.with_ancestors(self.detached_entities);
        let tables = &self.drop_tables;
        let registries = tables.tables?;
        let Some(table_id) = crate::loot_runtime::block_loot_table_id_with_registry(
            block_state_id,
            self.chunks.content_registry(),
        ) else {
            return Some(Vec::new());
        };
        if loot_table_by_id(registries, &table_id).is_none() {
            return Some(Vec::new());
        }
        let chunks = RefCell::new(&mut *self.chunks);
        let environment = LevelLootEnvironment::for_level(
            tables.recipes,
            LootChunks::Chunks(&chunks),
            &self.loot_level_facts,
            Some(bodies),
        )?;
        let axe = crate::entity::item::plain_stack("minecraft:diamond_axe", 1)?;
        let axe = crate::world::spawner_minecarts::apply_enchantment_provider_to_stack(
            &environment,
            "minecraft:enderman_loot_drop",
            &axe,
            special,
            self.random,
        )
        .unwrap_or(axe);
        let tool = crate::entity::block_drops::loot_tool_for_runtime_stack(
            &axe,
            self.loot_level_facts.enchantment_registry.as_deref()?,
        );
        let position = entity.position();
        let params = LootParams {
            dimension: self
                .loot_level_facts
                .dimension_key
                .as_deref()
                .map(|key| key.as_str()),
            origin: Some(LootOrigin {
                x: position.x,
                y: position.y,
                z: position.z,
            }),
            block_state_id: Some(block_state_id),
            tool: Some(&tool),
            entities: LootEntities {
                this_entity: LootEntity::Stored(StoredLootEntity {
                    entity,
                    bodies: Some(bodies),
                    raid_at_position,
                }),
                ..LootEntities::default()
            },
            item_environment: Some(&environment),
            level: Some(&|pos| {
                LoadedBlocks {
                    chunks: &chunks.borrow(),
                }
                .loaded_block_state(pos)
            }),
            block_entity_data: Some(&|pos| {
                let resolver = self
                    .loot_level_facts
                    .item_stack_resolver
                    .as_ref()
                    .ok_or_else(|| {
                        LootRuntimeReason::Catalog(
                            "block entity NBT publication is unavailable".into(),
                        )
                    })?;
                chunks
                    .borrow()
                    .block_entity_loot_data(pos, Some(resolver))
                    .map_err(|error| LootRuntimeReason::Catalog(error.to_string()))
            }),
            weather: Some(LootWeather {
                raining: self.weather.is_raining(),
                thundering: self.weather.is_thundering(),
            }),
            ..LootParams::new(param_sets::BLOCK)
        };
        loot_table_drops(registries, &table_id, &params, self.random).ok()
    }
}

pub(super) fn mob_loot_drops(
    level: &mut super::LevelCtx<'_, '_>,
    bodies: Option<crate::entity::store::EntityBodyReadView<'_>>,
    request: &MobLootRequest<'_>,
) -> MobLootRoll {
    let super::LevelCtx {
        drop_tables: tables,
        loot_level_facts,
        random: level_random,
        chunks,
        weather,
        ..
    } = level;
    let chunks = &mut **chunks;
    let level_random = &**level_random;
    let Some(registries) = tables.tables else {
        return MobLootRoll::NoRegistries;
    };
    // `ReloadableServerRegistries.getLootTable` answers `LootTable.EMPTY` for a
    // key the registry does not hold (`:119-121`), whose draw is nothing.
    if loot_table_by_id(registries, request.table_id).is_none() {
        return MobLootRoll::Drawn(Vec::new());
    }
    let tags = chunks.registry_tags_shared().map_or(
        crate::entity::EntityRegistryTagView::Builtin26_1,
        crate::entity::EntityRegistryTagView::Frozen,
    );
    let session_view = |uuid| loot_level_facts.session_loot_view(bodies, &tags, uuid);
    let resolve_attacker = |attacker| match attacker {
        crate::entity::mob_death_loot::MobLootAttacker::SessionPlayer { uuid } => {
            session_view(uuid)
        }
        _ => Ok(None),
    };
    let views = (|| -> Result<_, String> {
        Ok((
            resolve_attacker(request.attacking_entity)?,
            resolve_attacker(request.direct_attacking_entity)?,
            request
                .killer_player
                .map(session_view)
                .transpose()?
                .flatten(),
        ))
    })();
    let (attacking_view, direct_view, killer_view) = match views {
        Ok(views) => views,
        Err(reason) => return MobLootRoll::Refused(reason),
    };
    let causing = bound_attacker(request.attacking_entity, attacking_view.as_ref());
    let direct = bound_attacker(request.direct_attacking_entity, direct_view.as_ref());
    let luck = request.killer_player.and_then(|uuid| {
        loot_level_facts
            .player_by_uuid(bodies.map(|bodies| bodies.store()), uuid)
            .map(player_luck)
    });
    let chunks = RefCell::new(chunks);

    let environment = LevelLootEnvironment::for_level(
        tables.recipes,
        LootChunks::Chunks(&chunks),
        loot_level_facts,
        bodies,
    );
    let environment = environment
        .as_ref()
        .map(|environment| environment as &dyn LootItemEnvironment);
    let dimension = loot_level_facts
        .dimension_key
        .as_deref()
        .map(|key| key.as_str());
    let weather = LootWeather {
        raining: weather.is_raining(),
        thundering: weather.is_thundering(),
    };
    let blocks = |pos| {
        LoadedBlocks {
            chunks: &chunks.borrow(),
        }
        .loaded_block_state(pos)
    };
    let block_entity_data = |pos| {
        let resolver = loot_level_facts
            .item_stack_resolver
            .as_ref()
            .ok_or_else(|| {
                LootRuntimeReason::Catalog("block entity NBT publication is unavailable".into())
            })?;
        chunks
            .borrow()
            .block_entity_loot_data(pos, Some(resolver))
            .map_err(|error| LootRuntimeReason::Catalog(error.to_string()))
    };
    let vehicle_slots = |requested: &crate::entity::Entity, selected: &[i32]| {
        super::block_entity_loot::vehicle_slots_with_environment(
            registries,
            environment,
            dimension,
            &blocks,
            &block_entity_data,
            weather,
            level_random,
            bodies,
            requested,
            selected,
        )
    };
    let params = LootParams {
        dimension,
        origin: Some(request.origin),
        entities: LootEntities {
            attacking_entity: causing,
            direct_attacking_entity: direct,
            last_damage_player: killer_view
                .as_ref()
                .map_or(LootEntity::Absent, LootEntity::Facts),
            ..request.entities
        },
        damage_source: LootDamageSource::Present(LootDamageSourceFacts {
            damage_type: request.damage_source.damage_type,
            is_direct: Some(request.damage_source.is_direct),
            tags: tables.damage_type_tags,
            causing,
            direct: Some(direct),
        }),
        item_environment: environment,
        level: Some(&blocks),
        block_entity_data: Some(&block_entity_data),
        vehicle_slots: Some(&vehicle_slots),
        luck: luck.unwrap_or(0.0),
        weather: Some(weather),
        ..LootParams::new(param_sets::ENTITY)
    };
    // A saved `DeathLootTableSeed` makes the roll repeatable.
    let seeded;
    let random = if request.seed != 0 {
        seeded = GameRandom::with_seed(request.seed);
        &seeded
    } else {
        level_random
    };
    match loot_table_drops(registries, request.table_id, &params, random) {
        Ok(stacks) => MobLootRoll::Drawn(stacks),
        Err(error) => MobLootRoll::Refused(error.to_string()),
    }
}

/// `LivingEntity.dropFromGiftLootTable` (`LivingEntity.java:1570-1579`): the
/// table drawn under `GIFT` with the entity's origin and itself as `this`.
pub(super) fn gift_loot_drops(
    level: &mut super::LevelCtx<'_, '_>,
    bodies: crate::entity::store::EntityBodyReadView<'_>,
    entity: &crate::entity::Entity,
    table_id: &str,
) -> Vec<LootStack> {
    entity_loot_drops(level, bodies, entity, table_id, param_sets::GIFT)
}

pub(super) fn piglin_barter_loot_drops(
    level: &mut super::LevelCtx<'_, '_>,
    bodies: crate::entity::store::EntityBodyReadView<'_>,
    entity: &crate::entity::Entity,
) -> Vec<LootStack> {
    entity_loot_drops(
        level,
        bodies,
        entity,
        "minecraft:gameplay/piglin_bartering",
        param_sets::PIGLIN_BARTER,
    )
}

fn entity_loot_drops(
    level: &mut super::LevelCtx<'_, '_>,
    bodies: crate::entity::store::EntityBodyReadView<'_>,
    entity: &crate::entity::Entity,
    table_id: &str,
    param_set: &'static crate::loot_runtime::LootContextParamSet,
) -> Vec<LootStack> {
    let raid_at_position = level.raid_at(entity.block_position());
    let super::LevelCtx {
        drop_tables: tables,
        loot_level_facts,
        random,
        chunks,
        weather,
        ..
    } = level;
    let Some(registries) = tables.tables else {
        return Vec::new();
    };
    if loot_table_by_id(registries, table_id).is_none() {
        return Vec::new();
    }
    let chunks = RefCell::new(&mut **chunks);
    let environment = LevelLootEnvironment::for_level(
        tables.recipes,
        LootChunks::Chunks(&chunks),
        loot_level_facts,
        Some(bodies),
    );
    let environment = environment
        .as_ref()
        .map(|environment| environment as &dyn LootItemEnvironment);
    let dimension = loot_level_facts
        .dimension_key
        .as_deref()
        .map(|key| key.as_str());
    let weather = LootWeather {
        raining: weather.is_raining(),
        thundering: weather.is_thundering(),
    };
    let blocks = |pos| {
        LoadedBlocks {
            chunks: &chunks.borrow(),
        }
        .loaded_block_state(pos)
    };
    let block_entity_data = |pos| {
        let resolver = loot_level_facts
            .item_stack_resolver
            .as_ref()
            .ok_or_else(|| {
                LootRuntimeReason::Catalog("block entity NBT publication is unavailable".into())
            })?;
        chunks
            .borrow()
            .block_entity_loot_data(pos, Some(resolver))
            .map_err(|error| LootRuntimeReason::Catalog(error.to_string()))
    };
    let vehicle_slots = |requested: &crate::entity::Entity, selected: &[i32]| {
        super::block_entity_loot::vehicle_slots_with_environment(
            registries,
            environment,
            dimension,
            &blocks,
            &block_entity_data,
            weather,
            random,
            Some(bodies),
            requested,
            selected,
        )
    };
    let position = entity.position();
    let params = LootParams {
        dimension,
        origin: (param_set != param_sets::PIGLIN_BARTER).then_some(LootOrigin {
            x: position.x,
            y: position.y,
            z: position.z,
        }),
        entities: LootEntities {
            this_entity: LootEntity::Stored(StoredLootEntity {
                entity,
                bodies: Some(bodies),
                raid_at_position,
            }),
            ..LootEntities::default()
        },
        item_environment: environment,
        level: Some(&blocks),
        block_entity_data: Some(&block_entity_data),
        vehicle_slots: Some(&vehicle_slots),
        weather: Some(weather),
        ..LootParams::new(param_set)
    };
    loot_table_drops(registries, table_id, &params, random).unwrap_or_default()
}

/// `FishingHook.retrieve` (`FishingHook.java:446-453`): `gameplay/fishing`
/// under `FISHING` with the hook as `this`, the rod as the tool and the
/// hook's and owner's luck.
pub(super) fn fishing_loot_drops(
    level: &mut super::LevelCtx<'_, '_>,
    bodies: crate::entity::store::EntityBodyReadView<'_>,
    hook: &crate::entity::Entity,
    rod: &crate::loot_runtime::LootTool,
    luck: f32,
    biome_at: &crate::loot_runtime::LootBiomeAt<'_>,
) -> Vec<LootStack> {
    const FISHING: &str = "minecraft:gameplay/fishing";
    let super::LevelCtx {
        drop_tables: tables,
        loot_level_facts,
        random,
        chunks,
        weather,
        ..
    } = level;
    let Some(registries) = tables.tables else {
        return Vec::new();
    };
    let chunks = RefCell::new(&mut **chunks);
    let environment = LevelLootEnvironment::for_level(
        tables.recipes,
        LootChunks::Chunks(&chunks),
        loot_level_facts,
        Some(bodies),
    );
    let position = hook.position();
    let params = LootParams {
        dimension: loot_level_facts
            .dimension_key
            .as_deref()
            .map(|key| key.as_str()),
        origin: Some(LootOrigin {
            x: position.x,
            y: position.y,
            z: position.z,
        }),
        tool: Some(rod),
        entities: LootEntities {
            this_entity: LootEntity::Stored(StoredLootEntity {
                entity: hook,
                bodies: Some(bodies),
                raid_at_position: None,
            }),
            ..LootEntities::default()
        },
        item_environment: environment
            .as_ref()
            .map(|environment| environment as &dyn LootItemEnvironment),
        level: Some(&|pos| {
            LoadedBlocks {
                chunks: &chunks.borrow(),
            }
            .loaded_block_state(pos)
        }),
        block_entity_data: Some(&|pos| {
            let resolver = loot_level_facts
                .item_stack_resolver
                .as_ref()
                .ok_or_else(|| {
                    LootRuntimeReason::Catalog("block entity NBT publication is unavailable".into())
                })?;
            chunks
                .borrow()
                .block_entity_loot_data(pos, Some(resolver))
                .map_err(|error| LootRuntimeReason::Catalog(error.to_string()))
        }),
        biome: Some(biome_at),
        luck,
        weather: Some(LootWeather {
            raining: weather.is_raining(),
            thundering: weather.is_thundering(),
        }),
        ..LootParams::new(param_sets::FISHING)
    };
    loot_table_drops(registries, FISHING, &params, random).unwrap_or_default()
}

fn bound_attacker<'a>(
    attacker: crate::entity::mob_death_loot::MobLootAttacker<'a>,
    view: Option<&'a crate::loot_runtime::LootEntityFacts>,
) -> LootEntity<'a> {
    match attacker {
        crate::entity::mob_death_loot::MobLootAttacker::SessionPlayer { .. } => {
            view.map_or(LootEntity::Absent, LootEntity::Facts)
        }
        other => other.to_loot_entity(),
    }
}

pub(crate) fn player_luck(player: &crate::world::PlayerState) -> f32 {
    player
        .attributes
        .value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_LUCK_ID)
        .unwrap_or(0.0) as f32
}

/// [`LootLevelFacts::item_stack_resolver`] is *the same* publication the
/// `/give` and `/item` surfaces validate a `<item>[components]` argument with
/// and the save request encodes through, so a stack a death writes is the
/// stack a reload reads back. An environment without it refuses
/// `minecraft:set_components` and `minecraft:set_ominous_bottle_amplifier`
/// rather than writing a patch nothing validated.
///
/// `enchant_with_levels` reads the same two facts the merchant trade
/// environment lends it: the level's [`ChunkStore`], which is where a
/// `minecraft:enchantment` tag resolves, and [`LootLevelFacts::datapacks`],
/// which carries the registry elements `EnchantmentHelper.enchantItem`
/// selects by.
type LootPlayerSlotReadFn<'a> =
    &'a dyn Fn(uuid::Uuid, i32) -> Result<Option<crate::loot_runtime::LootTool>, LootRuntimeReason>;
type LootBlockEntitySlotsReadFn<'a> =
    &'a dyn Fn(
        &dyn LootItemEnvironment,
        &crate::block_entity::BlockEntity,
        &[i32],
    ) -> Result<Vec<RuntimeItemStack>, LootRuntimeReason>;
type LootEntityTeamReadFn<'a> = &'a dyn Fn(
    crate::loot_runtime::LootEntity<'_>,
    &bbb_protocol::JavaString,
) -> Result<bool, LootRuntimeReason>;
type LootEntityScoreReadFn<'a> = &'a dyn Fn(
    LootEntity<'_>,
    &bbb_protocol::JavaString,
    Option<crate::entity::store::EntityBodyReadView<'_>>,
) -> Result<Option<i32>, LootRuntimeReason>;

pub(in crate::world) struct LevelLootEnvironment<'a, 'm, 'c> {
    clocks: Option<&'a crate::world::world_clock::ServerWorldClocks>,
    environment_attributes:
        Option<crate::world::environment_attributes::LootEnvironmentContext<'a>>,
    entity_nbt: Option<EntityNbtReadSnapshot>,
    command_storage: Option<&'a ServerCommandStorage>,
    player_sources: Option<crate::world::loot_context::PlayerLootNbtSources<'a>>,
    player_slots: Option<LootPlayerSlotReadFn<'a>>,
    block_entity_slots: Option<LootBlockEntitySlotsReadFn<'a>>,
    player_nbt: Option<&'a dyn Fn(uuid::Uuid) -> Result<NbtSummaryValue, LootRuntimeReason>>,
    /// What `PlayerPredicate`'s advancement terms read.
    advancement_tree: Option<&'a crate::world::AdvancementTree>,
    entity_team: Option<LootEntityTeamReadFn<'a>>,
    entity_score_read: Option<LootEntityScoreReadFn<'a>>,
    entity_scoreboard:
        Option<crate::world::loot_context::entity_scoreboard::LootEntityScoreboard<'a>>,
    chunks: LootChunks<'a, 'c, 'm>,
    registry_tags: Option<Arc<crate::registry_tags::RegistryTagSnapshot>>,
    maps: Option<&'a RefCell<&'m mut crate::world::maps::ServerMapStore>>,
    world_seed: i64,
    registries: Option<&'a crate::registry_sync::SynchronizedRegistries>,
    sky_darkening: Option<i32>,
    dimension: Option<&'a str>,
    generate_structures: bool,
    datapacks: Option<&'a DatapackStartup>,
    recipes: Option<crate::entity::block_drops::RecipeDropFacts<'a>>,
    enchantment_registry: Option<&'a [String]>,
    item_stack_resolver: Option<&'a dyn bbb_registry_codec::PersistentItemStackHolderResolver>,
}

impl<'a, 'm, 'c> LevelLootEnvironment<'a, 'm, 'c> {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::world) fn for_world_predicates(
        chunks: LootChunks<'a, 'c, 'm>,
        resolver: &'a dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        datapacks: Option<&'a DatapackStartup>,
        enchantment_registry: &'a [String],
        recipes: Option<crate::entity::block_drops::RecipeDropFacts<'a>>,
        clocks: &'a crate::world::world_clock::ServerWorldClocks,
        storage: &'a ServerCommandStorage,
        registries: &'a crate::registry_sync::SynchronizedRegistries,
        seed: i64,
        sky_darkening: Option<i32>,
        attributes: crate::world::environment_attributes::LootEnvironmentContext<'a>,
        scoreboard: crate::world::loot_context::entity_scoreboard::LootEntityScoreboard<'a>,
    ) -> Self {
        Self::empty(chunks, Some(resolver))
            .with_enchantment_sources(datapacks, Some(enchantment_registry))
            .with_recipes(recipes)
            .with_clocks(clocks)
            .with_command_storage(storage)
            .with_position_queries(registries, seed, sky_darkening)
            .with_environment_attributes(attributes)
            .with_entity_scoreboard(scoreboard)
    }

    /// The environment of one level, or `None` for a generation the runtime
    /// wired no recipe manager into.
    pub(in crate::world) fn for_level(
        recipes: Option<crate::entity::block_drops::RecipeDropFacts<'a>>,
        chunks: LootChunks<'a, 'c, 'm>,
        facts: &'a LootLevelFacts<'m>,
        bodies: Option<crate::entity::store::EntityBodyReadView<'a>>,
    ) -> Option<Self> {
        recipes.map(|recipes| {
            let mut environment = Self::new(
                recipes,
                chunks,
                facts.datapacks,
                facts.enchantment_registry.as_deref().map(Vec::as_slice),
                facts.item_stack_resolver.as_ref(),
            );
            environment.maps = facts.maps.as_ref();
            environment.clocks = facts.clocks;
            environment.environment_attributes = facts.environment_attributes;
            environment.world_seed = facts.world_seed;
            environment.registries = facts.registries;
            environment.sky_darkening = facts.sky_darkening;
            environment.dimension = facts.dimension_key.as_deref().map(|key| key.as_str());
            environment.generate_structures = facts.generate_structures;
            environment = environment.with_level_player_sources(facts, bodies);
            environment.entity_scoreboard = facts.entity_scoreboard(bodies);
            environment.advancement_tree = facts.advancement_tree;
            match (facts.entity_nbt.as_ref(), facts.command_storage) {
                (Some(snapshot), Some(storage)) => {
                    environment.with_nbt_sources(snapshot.clone(), storage)
                }
                _ => environment,
            }
        })
    }

    pub(in crate::world) fn new(
        facts: crate::entity::block_drops::RecipeDropFacts<'a>,
        chunks: LootChunks<'a, 'c, 'm>,
        datapacks: Option<&'a DatapackStartup>,
        enchantment_registry: Option<&'a [String]>,
        item_stack_resolver: Option<&'a PlayerItemStackResolver>,
    ) -> Self {
        Self::empty(
            chunks,
            item_stack_resolver.map(PlayerItemStackResolver::as_dyn),
        )
        .with_enchantment_sources(datapacks, enchantment_registry)
        .with_recipes(Some(facts))
    }

    pub(in crate::world) fn with_recipes(
        mut self,
        recipes: Option<crate::entity::block_drops::RecipeDropFacts<'a>>,
    ) -> Self {
        self.recipes = recipes;
        self
    }

    pub(in crate::world) fn with_enchantment_sources(
        mut self,
        datapacks: Option<&'a DatapackStartup>,
        enchantment_registry: Option<&'a [String]>,
    ) -> Self {
        self.datapacks = datapacks;
        self.enchantment_registry = enchantment_registry;
        self.registry_tags = self.chunks.borrow().registry_tags_shared();
        self
    }

    pub(in crate::world) fn read_only(
        chunks: &'a crate::chunk::ChunkStore,
        resolver: &'a dyn bbb_registry_codec::PersistentItemStackHolderResolver,
    ) -> Self {
        Self::empty(LootChunks::Shared(chunks), Some(resolver))
    }

    pub(in crate::world) fn for_predicates(
        chunks: &'a crate::chunk::ChunkStore,
        facts: &'a LootLevelFacts<'m>,
        bodies: Option<crate::entity::store::EntityBodyReadView<'a>>,
    ) -> Self {
        let mut environment = Self::read_only(chunks, facts.item_stack_publication());
        environment.clocks = facts.clocks;
        environment.environment_attributes = facts.environment_attributes;
        environment.entity_scoreboard = facts.entity_scoreboard(bodies);
        environment.advancement_tree = facts.advancement_tree;
        environment.registries = facts.registries;
        environment.world_seed = facts.world_seed;
        environment.sky_darkening = facts.sky_darkening;
        environment.command_storage = facts.command_storage;
        environment.with_level_player_sources(facts, bodies)
    }

    fn with_level_player_sources(
        mut self,
        facts: &'a LootLevelFacts<'m>,
        bodies: Option<crate::entity::store::EntityBodyReadView<'a>>,
    ) -> Self {
        self.player_sources = Some(crate::world::loot_context::PlayerLootNbtSources {
            snapshot: facts.entity_nbt.as_ref(),
            resolver: facts.item_stack_resolver.as_ref(),
            dimension: facts.dimension.as_ref(),
            bodies,
            other_levels: facts.other_levels.map_or(
                crate::world::loot_context::entity_scoreboard::LootEntityLevels::Stores(&[]),
                crate::world::loot_context::entity_scoreboard::LootEntityLevels::Other,
            ),
        });
        self
    }

    pub(in crate::world) fn empty(
        chunks: LootChunks<'a, 'c, 'm>,
        item_stack_resolver: Option<&'a dyn bbb_registry_codec::PersistentItemStackHolderResolver>,
    ) -> Self {
        Self {
            clocks: None,
            environment_attributes: None,
            entity_nbt: None,
            command_storage: None,
            player_nbt: None,
            advancement_tree: None,
            entity_team: None,
            entity_score_read: None,
            entity_scoreboard: None,
            player_slots: None,
            block_entity_slots: None,
            player_sources: None,
            chunks,
            registry_tags: None,
            maps: None,
            world_seed: 0,
            registries: None,
            sky_darkening: None,
            dimension: None,
            generate_structures: false,
            datapacks: None,
            recipes: None,
            enchantment_registry: None,
            item_stack_resolver,
        }
    }

    pub(in crate::world) fn with_position_queries(
        mut self,
        registries: &'a crate::registry_sync::SynchronizedRegistries,
        world_seed: i64,
        sky_darkening: Option<i32>,
    ) -> Self {
        self.registries = Some(registries);
        self.world_seed = world_seed;
        self.sky_darkening = sky_darkening;
        self
    }

    pub(in crate::world) fn with_environment_attributes(
        mut self,
        environment: crate::world::environment_attributes::LootEnvironmentContext<'a>,
    ) -> Self {
        self.environment_attributes = Some(environment);
        self
    }

    pub(in crate::world) fn with_clocks(
        mut self,
        clocks: &'a crate::world::world_clock::ServerWorldClocks,
    ) -> Self {
        self.clocks = Some(clocks);
        self
    }

    pub(in crate::world) fn with_maps(
        mut self,
        maps: &'a RefCell<&'m mut crate::world::maps::ServerMapStore>,
        world_seed: i64,
        dimension: &'a str,
        generate_structures: bool,
    ) -> Self {
        self.maps = Some(maps);
        self.world_seed = world_seed;
        self.dimension = Some(dimension);
        self.generate_structures = generate_structures;
        self
    }

    pub(in crate::world) fn with_command_storage(
        mut self,
        storage: &'a ServerCommandStorage,
    ) -> Self {
        self.command_storage = Some(storage);
        self
    }

    pub(in crate::world) fn with_nbt_sources(
        mut self,
        snapshot: EntityNbtReadSnapshot,
        storage: &'a ServerCommandStorage,
    ) -> Self {
        self.entity_nbt = Some(snapshot);
        self.command_storage = Some(storage);
        self
    }

    pub(in crate::world) fn with_player_slots(
        mut self,
        read: &'a dyn Fn(
            uuid::Uuid,
            i32,
        )
            -> Result<Option<crate::loot_runtime::LootTool>, LootRuntimeReason>,
    ) -> Self {
        self.player_slots = Some(read);
        self
    }

    pub(in crate::world) fn with_block_entity_slots(
        mut self,
        read: LootBlockEntitySlotsReadFn<'a>,
    ) -> Self {
        self.block_entity_slots = Some(read);
        self
    }

    pub(in crate::world) fn with_advancement_tree(
        mut self,
        tree: &'a crate::world::AdvancementTree,
    ) -> Self {
        self.advancement_tree = Some(tree);
        self
    }

    pub(in crate::world) fn with_player_nbt(
        mut self,
        read: &'a dyn Fn(uuid::Uuid) -> Result<NbtSummaryValue, LootRuntimeReason>,
    ) -> Self {
        self.player_nbt = Some(read);
        self
    }

    pub(in crate::world) fn with_entity_team(mut self, read: LootEntityTeamReadFn<'a>) -> Self {
        self.entity_team = Some(read);
        self
    }

    pub(in crate::world) fn with_entity_scoreboard(
        mut self,
        scoreboard: crate::world::loot_context::entity_scoreboard::LootEntityScoreboard<'a>,
    ) -> Self {
        self.entity_scoreboard = Some(scoreboard);
        self
    }

    pub(in crate::world) fn with_entity_score(mut self, read: LootEntityScoreReadFn<'a>) -> Self {
        self.entity_score_read = Some(read);
        self
    }
}

/// The two facts `select_spawner_enchantments` selects over, on the level the
/// death happens in: its chunk store, where a `minecraft:enchantment` tag
/// resolves, and the datapack generation the loot registries came from.
impl SpawnEnchantmentProviderEnvironment for LevelLootEnvironment<'_, '_, '_> {
    fn registry_tags(&self) -> Option<&crate::registry_tags::RegistryTagSnapshot> {
        self.registry_tags.as_deref()
    }

    fn datapacks(&self) -> Option<&DatapackStartup> {
        self.datapacks
    }

    fn item_holders(&self) -> &dyn bbb_registry_codec::PersistentItemStackHolderResolver {
        LootItemEnvironment::item_resolver(self)
    }
}

impl EnchantWithLevelsItemWorld for LevelLootEnvironment<'_, '_, '_> {
    fn enchantments(&self) -> Option<&[String]> {
        self.enchantment_registry
    }
}

impl LootItemEnvironment for LevelLootEnvironment<'_, '_, '_> {
    fn biome_at(
        &self,
        pos: bbb_protocol::packets::BlockPos,
    ) -> Result<Option<&str>, LootRuntimeReason> {
        let registries = self.registries.ok_or_else(|| not_here("biome registry"))?;
        Ok(crate::world::natural_spawn::fuzzy_biome_resource_id(
            registries,
            &self.chunks.borrow(),
            bbb_server_worldgen::noise::biome::biome_zoom_seed(self.world_seed),
            pos,
        ))
    }

    fn light_at(
        &self,
        pos: bbb_protocol::packets::BlockPos,
    ) -> Result<Option<i32>, LootRuntimeReason> {
        let darkening = self
            .sky_darkening
            .ok_or_else(|| not_here("sky darkening"))?;
        Ok(Some(super::max_local_raw_brightness(
            &self.chunks.borrow(),
            pos,
            darkening,
        )))
    }

    fn structure_matches(
        &self,
        pos: bbb_protocol::packets::BlockPos,
        expected: &bbb_server_datapack::loot::HolderSet,
    ) -> Result<Option<bool>, LootRuntimeReason> {
        Ok(LoadedBlocks {
            chunks: &self.chunks.borrow(),
        }
        .loaded_structure_matches(pos, expected, self.item_resolver()))
    }

    fn sky_at(
        &self,
        pos: bbb_protocol::packets::BlockPos,
    ) -> Result<Option<bool>, LootRuntimeReason> {
        Ok(LoadedBlocks {
            chunks: &self.chunks.borrow(),
        }
        .loaded_can_see_sky(pos))
    }

    fn environment_attribute(
        &self,
        attribute: &'static VanillaEnvironmentAttribute,
        origin: Option<crate::loot_runtime::LootOrigin>,
    ) -> Result<LootEnvironmentAttributeValue<'_>, LootRuntimeReason> {
        self.environment_attributes
            .and_then(|environment| environment.sample(attribute, &self.chunks.borrow(), origin))
            .ok_or_else(|| not_here("environment attribute"))
    }

    fn score(
        &self,
        name: &bbb_protocol::JavaString,
        objective: &bbb_protocol::JavaString,
    ) -> Result<Option<i32>, LootRuntimeReason> {
        Ok(self
            .entity_scoreboard
            .as_ref()
            .ok_or_else(|| {
                LootRuntimeReason::Catalog("score requires the actual scoreboard".into())
            })?
            .score(name, objective))
    }

    fn entity_score(
        &self,
        entity: LootEntity<'_>,
        objective: &bbb_protocol::JavaString,
        bodies: Option<crate::entity::store::EntityBodyReadView<'_>>,
    ) -> Result<Option<i32>, LootRuntimeReason> {
        if let Some(read) = self.entity_score_read {
            return read(entity, objective, bodies);
        }
        self.entity_scoreboard
            .as_ref()
            .ok_or_else(|| {
                LootRuntimeReason::Catalog("score requires the actual scoreboard".into())
            })?
            .entity_score(entity, objective, bodies)
    }

    fn clock_total_ticks(&self, clock: &str) -> Result<i64, LootRuntimeReason> {
        self.clocks
            .and_then(|clocks| clocks.total_ticks_for_clock(clock))
            .ok_or_else(|| {
                LootRuntimeReason::Catalog("time condition requires an actual world clock".into())
            })
    }

    fn entity_team_matches(
        &self,
        entity: crate::loot_runtime::LootEntity<'_>,
        expected: &bbb_protocol::JavaString,
        bodies: Option<crate::entity::store::EntityBodyReadView<'_>>,
    ) -> Result<bool, LootRuntimeReason> {
        if let Some(read) = self.entity_team {
            return read(entity, expected);
        }
        self.entity_scoreboard
            .as_ref()
            .ok_or_else(|| {
                LootRuntimeReason::Catalog("entity team requires the actual scoreboard".into())
            })?
            .matches(entity, expected, bodies)
    }

    fn entity_for_relations<'s>(
        &'s self,
        entity: LootEntity<'_>,
        bodies: Option<crate::entity::store::EntityBodyReadView<'s>>,
    ) -> Option<StoredLootEntity<'s>> {
        self.entity_scoreboard
            .as_ref()?
            .entity_for_relations(entity, bodies)
    }

    fn player_terms_match(
        &self,
        uuid: uuid::Uuid,
        terms: &bbb_server_datapack::loot::PlayerTerms,
    ) -> Result<bool, LootRuntimeReason> {
        if !terms.advancements.is_empty() && self.advancement_tree.is_none() {
            return Err(LootRuntimeReason::Catalog(
                "player advancement terms need the advancement tree".into(),
            ));
        }
        // An entity whose player left answers no term.
        Ok(self
            .entity_scoreboard
            .as_ref()
            .and_then(|scoreboard| scoreboard.online_player(uuid))
            .is_some_and(|(player, ..)| {
                crate::world::player_predicate::player_terms_match(
                    player,
                    self.advancement_tree,
                    terms,
                )
            }))
    }

    fn player_looking_at<'s>(
        &'s self,
        uuid: uuid::Uuid,
    ) -> Result<Option<StoredLootEntity<'s>>, LootRuntimeReason> {
        let Some((player, store, here)) = self
            .entity_scoreboard
            .as_ref()
            .and_then(|scoreboard| scoreboard.online_player(uuid))
        else {
            return Ok(None);
        };
        if !here {
            return Err(LootRuntimeReason::Catalog(
                "player looking_at needs the player's own level".into(),
            ));
        }
        let entity =
            crate::world::player_predicate::player_looking_at(player, store, &self.chunks.borrow());
        Ok(entity.map(|entity| StoredLootEntity {
            entity,
            bodies: Some(crate::entity::store::EntityBodyReadView::new(
                store, None, None,
            )),
            raid_at_position: None,
        }))
    }

    fn player_item_slot(
        &self,
        uuid: uuid::Uuid,
        slot: i32,
    ) -> Result<Option<crate::loot_runtime::LootTool>, LootRuntimeReason> {
        if let Some(read) = self.player_slots {
            return read(uuid, slot);
        }
        self.player_sources
            .as_ref()
            .ok_or_else(|| {
                LootRuntimeReason::Catalog(
                    "player inventory source is unavailable for this loot path".into(),
                )
            })?
            .slot(uuid, slot)
    }

    fn block_entity_slots(
        &self,
        original: &crate::block_entity::BlockEntity,
        slots: &[i32],
    ) -> Result<Vec<RuntimeItemStack>, LootRuntimeReason> {
        self.block_entity_slots.ok_or_else(|| {
            LootRuntimeReason::Catalog(
                "block container source is unavailable for this loot path".into(),
            )
        })?(self, original, slots)
    }

    fn exploration_map(
        &self,
        origin: crate::loot_runtime::LootOrigin,
        options: &crate::loot_runtime::LootExplorationMap,
    ) -> Result<Option<bbb_server_types::item_stack::RuntimeItemStack>, LootRuntimeReason> {
        if !self.generate_structures {
            return Ok(None);
        }
        let wanted: Vec<String> = self
            .registry_tags
            .as_ref()
            .map(|tags| {
                tags.members("minecraft:worldgen/structure", &options.destination)
                    .iter()
                    .map(|member| member.to_string())
                    .collect()
            })
            .unwrap_or_default();
        if wanted.is_empty() {
            return Ok(None);
        }
        let mut chunks = self
            .chunks
            .borrow_mut()
            .ok_or_else(|| not_here("map chunk writes"))?;
        let target = chunks
            .find_nearest_map_structure(
                bbb_protocol::packets::BlockPos {
                    x: origin.x.floor() as i32,
                    y: origin.y.floor() as i32,
                    z: origin.z.floor() as i32,
                },
                &wanted,
                options.search_radius,
                options.skip_known_structures,
            )
            .map_err(|error| refusal(error.to_string()))?;
        let Some((target, _)) = target else {
            return Ok(None);
        };
        let maps = self.maps.ok_or_else(|| not_here("map saved data"))?;
        let dimension = self.dimension.ok_or_else(|| not_here("map dimension"))?;
        let resolver = self
            .item_stack_resolver
            .ok_or_else(|| not_here("map item publication"))?;
        maps.borrow_mut()
            .create_exploration_map(
                &chunks,
                self.world_seed,
                crate::world::maps::ExplorationMapSpec {
                    dimension,
                    target,
                    zoom: options.zoom,
                    decoration: &options.decoration,
                },
                resolver,
            )
            .map(Some)
            .map_err(refusal)
    }
    fn nbt_storage(&self, id: &str) -> Result<NbtSummaryValue, LootRuntimeReason> {
        self.command_storage
            .ok_or_else(|| {
                LootRuntimeReason::Catalog("loot command storage is unavailable".into())
            })?
            .get(id)
            .map_err(|error| LootRuntimeReason::Catalog(format!("loot command storage: {error:?}")))
    }

    fn nbt_entity(&self, view: LootEntity<'_>) -> Result<NbtSummaryValue, LootRuntimeReason> {
        if let Some(uuid) = view.player_uuid() {
            if let Some(read) = self.player_nbt {
                return read(uuid);
            }
            return self
                .player_sources
                .as_ref()
                .ok_or_else(|| {
                    LootRuntimeReason::Catalog(
                        "complete player NBT source is unavailable for this loot path".into(),
                    )
                })?
                .read(uuid);
        }
        view.nbt_data()
    }

    fn nbt_block_entity(
        &self,
        entity: &crate::block_entity::BlockEntity,
    ) -> Result<NbtSummaryValue, LootRuntimeReason> {
        let resolver = self.item_stack_resolver.ok_or_else(|| {
            LootRuntimeReason::Catalog("block entity NBT publication is unavailable".into())
        })?;
        entity
            .command_data_with_resolver(resolver)
            .map_err(|error| LootRuntimeReason::Catalog(error.to_string()))
    }

    fn mob_effect_instantaneous(&self, id: &str) -> Result<bool, LootRuntimeReason> {
        Ok(crate::entity::living::mob_effects::is_instantaneous(id))
    }

    fn item_resolver(&self) -> &dyn bbb_registry_codec::PersistentItemStackHolderResolver {
        match self.item_stack_resolver {
            Some(resolver) => resolver,
            None => &bbb_registry_codec::UnavailableHolderResolver,
        }
    }

    fn random_enchantment_candidates(
        &self,
        stack: &ItemStackSummary,
        options: Option<&bbb_server_datapack::loot::HolderSet>,
        only_compatible: bool,
    ) -> std::result::Result<Vec<crate::loot_runtime::LootRandomEnchantment>, LootRuntimeReason>
    {
        crate::world::enchant_with_levels::random_enchantment_candidates(
            self,
            stack,
            options,
            only_compatible,
        )
    }

    fn with_random_enchantment(
        &self,
        stack: ItemStackSummary,
        enchantment: &crate::loot_runtime::LootRandomEnchantment,
        level: i32,
    ) -> std::result::Result<ItemStackSummary, LootRuntimeReason> {
        crate::world::enchant_with_levels::with_random_enchantment(self, stack, enchantment, level)
    }

    fn enchant_with_levels(
        &self,
        stack: ItemStackSummary,
        levels: i32,
        options: Option<&bbb_server_datapack::loot::HolderSet>,
        random: &GameRandom,
    ) -> std::result::Result<ItemStackSummary, LootRuntimeReason> {
        enchant_item(self, stack, levels, options, random)
    }

    ///
    /// A stack whose `minecraft:enchantments` names a holder id this
    /// generation's registry has no row for refuses rather than answering a
    /// predicate over a tool whose levels are unknown, the way
    /// `crate::loot_runtime::LootEntityFacts` records one for a held stack; an environment
    /// with no registry at all refuses for the same reason.
    fn matches_item(
        &self,
        predicate: &bbb_server_datapack::loot::ItemPredicate,
        stack: &ItemStackSummary,
    ) -> std::result::Result<bool, LootRuntimeReason> {
        let Some(registry) = self.enchantment_registry else {
            return Err(LootRuntimeReason::UnsupportedField {
                owner: "filtered",
                detail: "the item predicate needs the level's minecraft:enchantment registry"
                    .to_string(),
            });
        };
        let unresolved = unresolved_tool_enchantments(stack, registry);
        if let Some(holder_id) = unresolved.first() {
            return Err(LootRuntimeReason::UnsupportedField {
                owner: "filtered",
                detail: format!(
                    "the stack's enchantment holder {holder_id} is not a \
                     minecraft:enchantment this generation carries"
                ),
            });
        }
        let tool = loot_tool_for_stack(stack, registry);
        let recipes = self.recipes.ok_or_else(|| not_here("item tags"))?;
        matches_item_predicate(predicate, &tool, recipes.item_tags, self.item_resolver())
    }

    fn smelt_result(
        &self,
        stack: &ItemStackSummary,
    ) -> std::result::Result<
        Option<bbb_server_types::item_stack::RuntimeItemStack>,
        LootRuntimeReason,
    > {
        let recipes = self.recipes.ok_or_else(|| not_here("recipes"))?;
        Ok(recipes
            .recipe_index
            .get_recipe_for_cooking(
                recipes.recipes,
                CookingKind::Smelting,
                stack,
                None,
                recipes.item_tags,
            )
            .and_then(|element| element.recipe.cooking_of_kind(CookingKind::Smelting))
            .map(|recipe| {
                bbb_server_types::item_stack::RuntimeItemStack::from_summary(recipe.assemble())
            }))
    }
}

fn refusal(detail: impl Into<String>) -> LootRuntimeReason {
    LootRuntimeReason::UnsupportedField {
        owner: "item modifier",
        detail: detail.into(),
    }
}

fn not_here(function: &'static str) -> LootRuntimeReason {
    LootRuntimeReason::UnsupportedField {
        owner: "mob death loot",
        detail: format!("{function} needs the item component projection this level does not build"),
    }
}
