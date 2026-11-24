use crate::entity::block_drops::RecipeDropFacts;
use crate::loot_runtime::{
    loot_table_drops_from_table, param_sets, LootEntities, LootEntity, LootOrigin, LootParams,
    LootStack,
};
use crate::random::GameRandom;
use crate::world::level_blocks::{LoadedBlocks, LootChunks};
use crate::world::LevelId;
use bbb_protocol::{component_literal_nbt, NbtSummaryValue};
use bbb_server_datapack::loot::{LootRegistries, LootTable};
use bbb_server_datapack::recipe::ItemTagIndex;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::commands::{
    CommandLootTable, CommandPlayerTarget, CommandSource, CommandSourceIdentity,
};
use crate::world::simulation_level::block_entity_loot::{
    fill_unpacked_container_with_environment, unpack_live_container_loot,
    vehicle_slots_with_environment,
};
use crate::world::simulation_level::mob_death_loot::LevelLootEnvironment;
use crate::world::{ServerWorld, WorldEvent};
use bbb_server_types::save::Java26LevelKey;

struct CommandLootContextArgs<'a> {
    level_key: &'a LevelId,
    origin: [f64; 3],
    source: &'a CommandSource,
    tool: Option<&'a RuntimeItemStack>,
    target: Option<&'a CommandSource>,
}

impl ServerWorld {
    /// `ClearInventoryCommands.clearInventory`'s per-target clear; a target
    /// that left answers zero.
    pub(crate) fn clear_command_items(
        &mut self,
        target: &CommandPlayerTarget,
        max_count: i32,
        matches: &mut impl FnMut(&RuntimeItemStack) -> bbb_server_types::Result<bool>,
        events: &mut Vec<WorldEvent>,
    ) -> Result<i32, NbtSummaryValue> {
        let Some(connection) = self.command_player_connection(target) else {
            return Ok(0);
        };
        self.clear_items_from_player(connection, max_count, matches, events)
            .map_err(|error| component_literal_nbt(error.to_string()))
    }

    /// `GiveCommand.giveItem`'s per-target loop (`GiveCommand.java:57-89`).
    pub(crate) fn give_command_items(
        &mut self,
        target: &CommandPlayerTarget,
        prototype: &RuntimeItemStack,
        count: i32,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), NbtSummaryValue> {
        let connection = self
            .command_player_connection(target)
            .ok_or_else(|| component_literal_nbt("give target is no longer connected"))?;
        self.give_native_items_to_player(connection, prototype, count, events)
            .map_err(|error| component_literal_nbt(error.to_string()))
    }

    pub(crate) fn roll_command_loot_table(
        &mut self,
        table: &CommandLootTable,
        level: &Java26LevelKey,
        origin: [f64; 3],
        source: &CommandSource,
        tool: Option<&RuntimeItemStack>,
    ) -> Result<Vec<RuntimeItemStack>, NbtSummaryValue> {
        let startup = self
            .cold
            .config
            .datapacks
            .clone()
            .unwrap_or_else(crate::registry_sync::builtin_datapack_startup_shared);
        let table_id = if table.id.is_empty() {
            "bbb:command_inline"
        } else {
            &table.id
        };
        let level_id = self.require_level_id(level);
        let random = self.random.clone();
        self.roll_command_loot(
            &startup.loot,
            table_id,
            &table.table,
            CommandLootContextArgs {
                level_key: &level_id,
                origin,
                source,
                tool,
                target: None,
            },
            &random,
        )
    }

    pub(crate) fn roll_command_kill_loot(
        &mut self,
        table_id: &str,
        target: &CommandSource,
        level: &Java26LevelKey,
        origin: [f64; 3],
        source: &CommandSource,
    ) -> Result<Vec<RuntimeItemStack>, NbtSummaryValue> {
        let startup = self
            .cold
            .config
            .datapacks
            .clone()
            .unwrap_or_else(crate::registry_sync::builtin_datapack_startup_shared);
        let level = self.require_level_id(level);
        let random = self.random.clone();
        self.with_command_loot_context(
            CommandLootContextArgs {
                level_key: &level,
                origin,
                source,
                tool: None,
                target: Some(target),
            },
            param_sets::ENTITY,
            |params| {
                crate::loot_runtime::loot_table_drops(&startup.loot, table_id, params, &random)
            },
        )
        .map(runtime_loot_stacks)
    }

    pub(crate) fn roll_command_mine_loot(
        &mut self,
        level: &Java26LevelKey,
        pos: bbb_protocol::packets::BlockPos,
        source: &CommandSource,
        tool: &RuntimeItemStack,
    ) -> Result<(String, Vec<RuntimeItemStack>), NbtSummaryValue> {
        let startup = self
            .cold
            .config
            .datapacks
            .clone()
            .unwrap_or_else(crate::registry_sync::builtin_datapack_startup_shared);
        let level = self.require_level_id(level);
        let chunks = &self.level_by_id(&level).chunks;
        let state = chunks
            .get_block_state(pos.x, pos.y, pos.z)
            .expect("loaded command block");
        let content_registry = chunks.content_registry();
        let table_id =
            crate::loot_runtime::block_loot_table_id_with_registry(state, content_registry)
                .ok_or_else(|| {
                    // `LootCommand.dropBlockLoot` (`LootCommand.java:429-431`) passes
                    // `blockState.getBlock().getName()`, a translatable description key.
                    let block_name =
                        bbb_protocol::block_state_types::vanilla_block_state_name(state)
                            .and_then(crate::commands::block_description_id_opt)
                            .or_else(|| {
                                let (block, _) = content_registry.block_by_state_id(state)?;
                                let (namespace, path) = block.resource_id().split_once(':')?;
                                Some(format!("block.{namespace}.{path}"))
                            })
                            .map(|key| crate::commands::translatable(key, Vec::new()))
                            .unwrap_or_else(|| component_literal_nbt("unknown"));
                    crate::commands::translatable(
                        "commands.drop.no_loot_table.block",
                        vec![block_name],
                    )
                })?;
        let block_entity = chunks.block_entities().get(pos).cloned();
        let resolver = self.item_stack_resolver();
        let tool = crate::entity::mob_death_loot::loot_tool_for_persistent_stack(
            tool,
            &self.datapack_tables.enchantment_slots,
        );
        let random = self.random.clone();
        let origin = [
            f64::from(pos.x) + 0.5,
            f64::from(pos.y) + 0.5,
            f64::from(pos.z) + 0.5,
        ];
        let drops = self.with_command_loot_context(
            CommandLootContextArgs {
                level_key: &level,
                origin,
                source,
                tool: None,
                target: None,
            },
            param_sets::BLOCK,
            |params| {
                crate::loot_runtime::block_drops_with_text(
                    &startup.loot,
                    &crate::loot_runtime::BlockDropParams {
                        block_state_id: state,
                        pos,
                        dimension: params.dimension,
                        level: params.level,
                        block_entity_data: params.block_entity_data,
                        tool: &tool,
                        breaker: params.entities.this_entity,
                        explosion_radius: None,
                        block_entity: block_entity.as_ref().map(|block_entity| {
                            crate::loot_runtime::LootBlockEntity {
                                block_entity,
                                projection:
                                    crate::block_entity::BlockEntityItemProjection::with_resolver(
                                        &resolver,
                                    ),
                            }
                        }),
                        item_environment: params.item_environment,
                    },
                    &random,
                    params.text_resolver,
                    params.vehicle_slots,
                )
            },
        );
        if block_entity
            .as_ref()
            .is_some_and(|body| body.container_loot_table_id().is_some())
            && self
                .level_by_id(&level)
                .chunks
                .block_entities()
                .get(pos)
                .is_some_and(|body| body.container_loot_table_id().is_none())
        {
            self.with_level(level, |level| level.block_entity_set_changed(pos, state));
        }
        let drops = drops?;
        Ok((table_id, runtime_loot_stacks(drops)))
    }

    pub(crate) fn spawn_command_loot(
        &mut self,
        level: &Java26LevelKey,
        position: [f64; 3],
        stack: RuntimeItemStack,
    ) {
        let level = self.require_level_id(level);
        crate::entity::spawn_dropped_item_entity(
            &mut self.level_mut_by_id(&level).entities,
            bbb_protocol::packets::Vec3d {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            stack,
        );
    }

    /// `LootCommand.playerGive`'s `inventory.add` (`LootCommand.java:333-345`)
    /// of one copy of `stack`.
    pub(crate) fn add_command_loot(
        &mut self,
        target: &CommandPlayerTarget,
        stack: &RuntimeItemStack,
    ) -> Result<bool, NbtSummaryValue> {
        let resolver = self.item_stack_resolver();
        let connection = self
            .command_player_connection(target)
            .ok_or_else(|| component_literal_nbt("loot target is no longer connected"))?;
        let player = self
            .levels
            .player_mut(connection)
            .expect("resolved loot target");
        let creative = player.abilities.instabuild;
        player
            .inventory
            .add_anywhere(&resolver, &mut stack.clone(), creative)
            .map_err(|error| component_literal_nbt(error.to_string()))
    }

    fn roll_command_loot(
        &mut self,
        tables: &LootRegistries,
        table_id: &str,
        table: &LootTable,
        context: CommandLootContextArgs<'_>,
        random: &GameRandom,
    ) -> Result<Vec<RuntimeItemStack>, NbtSummaryValue> {
        let param_set = if context.tool.is_some() {
            param_sets::FISHING
        } else {
            param_sets::CHEST
        };
        let drops = self.with_command_loot_context(context, param_set, |params| {
            loot_table_drops_from_table(tables, table_id, table, params, random)
        })?;
        Ok(runtime_loot_stacks(drops))
    }

    pub(super) fn apply_command_item_modifier(
        &mut self,
        command: &crate::commands::ItemCommand,
        modifier: &crate::commands::CommandItemModifier,
        stack: RuntimeItemStack,
        random: &GameRandom,
    ) -> Result<RuntimeItemStack, NbtSummaryValue> {
        let startup = self
            .cold
            .config
            .datapacks
            .clone()
            .unwrap_or_else(crate::registry_sync::builtin_datapack_startup_shared);
        self.with_command_loot_context(
            CommandLootContextArgs {
                level_key: &self.require_level_id(&command.level),
                origin: command.origin,
                source: &command.source,
                tool: None,
                target: None,
            },
            param_sets::COMMAND,
            |params| {
                crate::loot_runtime::LootEvaluator::new(&startup.loot, &modifier.id, params, random)
                    .and_then(|mut evaluator| {
                        evaluator.modifier_runtime(&modifier.function, stack.clone())
                    })
            },
        )
    }

    fn with_command_loot_context<T>(
        &mut self,
        context: CommandLootContextArgs<'_>,
        param_set: &'static crate::loot_runtime::LootContextParamSet,
        mut evaluate: impl FnMut(&LootParams<'_>) -> Result<T, crate::loot_runtime::LootRuntimeError>,
    ) -> Result<T, NbtSummaryValue> {
        let CommandLootContextArgs {
            level_key,
            origin,
            source,
            tool,
            target,
        } = context;
        let startup = self
            .cold
            .config
            .datapacks
            .clone()
            .unwrap_or_else(crate::registry_sync::builtin_datapack_startup_shared);
        let resolver = self.item_stack_resolver();
        let nbt_snapshot = self.entity_nbt_read_snapshot();
        if matches!(
            source.identity,
            crate::commands::CommandSourceIdentity::Player
        ) && self.command_source_connection(source).is_none()
        {
            return Err(component_literal_nbt("command source player left"));
        }
        let player_view = self
            .command_source_connection(source)
            .and_then(|id| self.levels.player(id))
            .map(|player| self.loot_view_for_player(player));
        let target_player_view = target
            .and_then(|target| self.command_source_connection(target))
            .and_then(|id| self.levels.player(id))
            .map(|player| self.loot_view_for_player(player));
        let entity_tags = self
            .levels
            .iter()
            .map(|(key, level)| (key, level.chunks.registry_tags_shared()))
            .collect::<Vec<_>>();
        let level_names = std::sync::Arc::clone(&self.levels.names);
        let environment_weather = crate::world::environment_attributes::ScalarWeather::from_level(
            &self.level_by_id(level_key).weather,
        );
        let environment_attributes = self.cold.dimension_sky_lights.loot_environment_context(
            self.cold
                .level_dimension_types
                .get(level_key)
                .map(String::as_str),
            &self.registries,
            &self.world_clocks,
            environment_weather,
        );
        let mut entities = Vec::new();
        let mut level_blocks = Vec::new();
        let mut selector_levels = Vec::new();
        let mut current = None;
        for (key, level) in self.levels.iter_mut() {
            let weather = crate::loot_runtime::LootWeather {
                raining: level.weather.is_raining(),
                thundering: level.weather.is_thundering(),
            };
            entities.push((key, &level.entities));
            selector_levels.push((
                key,
                &level.random,
                weather,
                &level.raids,
                self.cold.dimension_sky_lights.loot_environment_context(
                    self.cold
                        .level_dimension_types
                        .get(&key)
                        .map(String::as_str),
                    &self.registries,
                    &self.world_clocks,
                    crate::world::environment_attributes::ScalarWeather::from_level(&level.weather),
                ),
                level.sky_darkening.map(i32::from),
            ));
            level_blocks.push((key, std::cell::RefCell::new(&mut level.chunks)));
            if key == *level_key {
                current = Some((&level.entities, &level.random, weather, level.sky_darkening));
            }
        }
        let (current_entities, current_random, weather, sky_darkening) =
            current.expect("a command source names a served level");
        let chunks = &level_blocks
            .iter()
            .find(|(key, _)| key == level_key)
            .expect("a command source names a served level")
            .1;
        let maps = std::cell::RefCell::new(&mut self.maps);
        static BUILTIN_ITEM_TAGS: std::sync::OnceLock<ItemTagIndex> = std::sync::OnceLock::new();
        let tags: &ItemTagIndex = if self.cold.config.datapacks.is_some() {
            &self.cold.loot_item_tags
        } else {
            BUILTIN_ITEM_TAGS.get_or_init(|| {
                crate::registry_sync::builtin_datapack_startup_shared()
                    .tags
                    .registry("minecraft:item")
                    .map(ItemTagIndex::from_loaded)
                    .unwrap_or_default()
            })
        };
        let sources = crate::world::loot_context::PlayerLootNbtSources {
            snapshot: Some(&nbt_snapshot),
            resolver: Some(&resolver),
            dimension: Some(level_key),
            bodies: Some(crate::entity::store::EntityBodyReadView::new(
                current_entities,
                None,
                None,
            )),
            other_levels: crate::world::loot_context::entity_scoreboard::LootEntityLevels::Stores(
                &entities,
            ),
        };
        let player_slots = |uuid, slot| sources.slot(uuid, slot);
        let source_body_nbt: std::cell::OnceCell<NbtSummaryValue> = std::cell::OnceCell::new();
        let player_nbt = |uuid| {
            if uuid == source.uuid {
                if let Some(cached) = source_body_nbt.get() {
                    return Ok(cached.clone());
                }
                let value = sources.read(uuid)?;
                let _ = source_body_nbt.set(value.clone());
                return Ok(value);
            }
            sources.read(uuid)
        };
        let scoreboard = self.scoreboard.board();
        let entity_scoreboard =
            crate::world::loot_context::entity_scoreboard::LootEntityScoreboard {
                scoreboard,
                names: &level_names,
                dimension: *level_key,
                bodies: Some(crate::entity::store::EntityBodyReadView::new(
                    current_entities,
                    None,
                    None,
                )),
                other_levels:
                    crate::world::loot_context::entity_scoreboard::LootEntityLevels::Stores(
                        &entities,
                    ),
            };
        let loaded_block_state = |pos| {
            LoadedBlocks {
                chunks: &chunks.borrow(),
            }
            .loaded_block_state(pos)
        };
        let block_entity_data = |pos| {
            chunks
                .borrow()
                .block_entity_loot_data(pos, Some(&resolver))
                .map_err(|error| crate::loot_runtime::LootRuntimeReason::Catalog(error.to_string()))
        };
        let block_entity_slots = |environment: &dyn crate::loot_runtime::LootItemEnvironment,
                                  original: &crate::block_entity::BlockEntity,
                                  selected: &[i32]| {
            unpack_live_container_loot(chunks, original.block_pos(), |request, available| {
                fill_unpacked_container_with_environment(
                    &startup.loot,
                    Some(environment),
                    Some(level_names.key(*level_key).as_str()),
                    &loaded_block_state,
                    &block_entity_data,
                    weather,
                    current_random,
                    LootOrigin::at_center_of(original.block_pos()),
                    request,
                    None,
                    available,
                )
                .map_err(bbb_server_types::ServerWorldError::InvalidConfiguration)
            })
            .map_err(|error| crate::loot_runtime::LootRuntimeReason::Catalog(error.to_string()))?;
            let body = original.resolve_instance(chunks.borrow().block_entities());
            Ok(body.loot_slots().map_or_else(Vec::new, |items| {
                selected
                    .iter()
                    .filter_map(|slot| usize::try_from(*slot).ok().and_then(|i| items.get(i)))
                    .cloned()
                    .collect()
            }))
        };
        let environment = LevelLootEnvironment::new(
            RecipeDropFacts {
                recipes: &startup.recipes,
                recipe_index: &startup.recipe_index,
                item_tags: tags,
            },
            LootChunks::Chunks(chunks),
            Some(&startup),
            Some(&self.datapack_tables.enchantment_registry),
            Some(&resolver),
        )
        .with_environment_attributes(environment_attributes)
        .with_clocks(&self.world_clocks)
        .with_maps(
            &maps,
            self.cold.seed,
            level_names.key(*level_key).as_str(),
            self.cold.generate_structures,
        )
        .with_nbt_sources(nbt_snapshot.clone(), &self.cold.command_storage)
        .with_player_nbt(&player_nbt)
        .with_player_slots(&player_slots)
        .with_block_entity_slots(&block_entity_slots)
        .with_entity_scoreboard(entity_scoreboard)
        .with_advancement_tree(&self.cold.advancement_tree);

        let at = crate::commands::CommandOrigin {
            position: origin,
            rotation: [0.0; 2],
            level: level_names.key(*level_key).clone(),
            anchor: bbb_protocol::entity_anchors::EntityAnchor::Feet,
            eye_height: None,
        };
        let overworld_tags = entity_tags
            .iter()
            .find(|(key, _)| {
                key.builtin() == Some(bbb_server_types::save::Java26BuiltinDimension::Overworld)
            })
            .and_then(|(_, tags)| tags.as_deref());
        let (storage, registries) = (&self.cold.command_storage, &self.registries);
        let advancements = &self.cold.advancement_tree;
        let datapacks = self.cold.config.datapacks.as_ref();
        let enchantment_registry = &self.datapack_tables.enchantment_registry;
        let enchantment_slots = &self.datapack_tables.enchantment_slots;
        let clocks = &self.world_clocks;
        let world_seed = self.cold.seed;
        let generate_structures = self.cold.generate_structures;
        let vehicle_slots = |requested: &crate::entity::Entity, selected: &[i32]| {
            let (source_level, source_store, actual) = entities
                .iter()
                .find_map(|(key, store)| {
                    let actual = store
                        .get(requested.id())
                        .filter(|actual| actual.uuid() == requested.uuid())?;
                    Some((key, *store, actual))
                })
                .ok_or_else(|| {
                    crate::loot_runtime::LootRuntimeReason::Catalog(
                        "vehicle slots need the actual source level/body".into(),
                    )
                })?;
            let Some(size) = crate::entity::item_slots::container_size(actual) else {
                return Ok(Vec::new());
            };
            if !selected
                .iter()
                .any(|slot| usize::try_from(*slot).is_ok_and(|slot| slot < size))
            {
                return Ok(Vec::new());
            }
            let source_chunks = &level_blocks
                .iter()
                .find(|(key, _)| key == source_level)
                .expect("every served level has blocks")
                .1;
            let (_, source_random, source_weather, _, source_attributes, source_sky) =
                selector_levels
                    .iter()
                    .find(|(key, ..)| key == source_level)
                    .expect("every served level has loot metadata");
            let source_blocks = |pos| {
                LoadedBlocks {
                    chunks: &source_chunks.borrow(),
                }
                .loaded_block_state(pos)
            };
            let source_block_data = |pos| {
                source_chunks
                    .borrow()
                    .block_entity_loot_data(pos, Some(&resolver))
                    .map_err(|error| {
                        crate::loot_runtime::LootRuntimeReason::Catalog(error.to_string())
                    })
            };
            let source_environment = LevelLootEnvironment::new(
                RecipeDropFacts {
                    recipes: &startup.recipes,
                    recipe_index: &startup.recipe_index,
                    item_tags: tags,
                },
                LootChunks::Chunks(source_chunks),
                Some(&startup),
                Some(enchantment_registry),
                Some(&resolver),
            )
            .with_position_queries(registries, world_seed, *source_sky)
            .with_environment_attributes(*source_attributes)
            .with_clocks(clocks)
            .with_maps(
                &maps,
                world_seed,
                level_names.key(*source_level).as_str(),
                generate_structures,
            )
            .with_nbt_sources(nbt_snapshot.clone(), storage)
            .with_player_nbt(&player_nbt)
            .with_player_slots(&player_slots)
            .with_advancement_tree(advancements)
            .with_entity_scoreboard(
                crate::world::loot_context::entity_scoreboard::LootEntityScoreboard {
                    scoreboard,
                    names: &level_names,
                    dimension: *source_level,
                    bodies: Some(crate::entity::store::EntityBodyReadView::new(
                        source_store,
                        None,
                        None,
                    )),
                    other_levels:
                        crate::world::loot_context::entity_scoreboard::LootEntityLevels::Stores(
                            &entities,
                        ),
                },
            );
            vehicle_slots_with_environment(
                &startup.loot,
                Some(&source_environment),
                Some(level_names.key(*source_level).as_str()),
                &source_blocks,
                &source_block_data,
                *source_weather,
                source_random,
                Some(crate::entity::store::EntityBodyReadView::new(
                    source_store,
                    None,
                    None,
                )),
                actual,
                selected,
            )
        };
        let target_source = target;
        let text_resolver = |target, value: &NbtSummaryValue| {
            let text_source = match target {
                crate::loot_runtime::LootContextParam::ThisEntity => {
                    target_source.unwrap_or(source)
                }
                crate::loot_runtime::LootContextParam::AttackingEntity
                | crate::loot_runtime::LootContextParam::DirectAttackingEntity => source,
                crate::loot_runtime::LootContextParam::LastDamagePlayer
                    if matches!(source.identity, CommandSourceIdentity::Player) =>
                {
                    source
                }
                _ => {
                    return Err(crate::loot_runtime::LootRuntimeReason::Catalog(
                        "command text source is not bound".into(),
                    ))
                }
            };
            let reads = crate::world::EntityReads {
                names: &level_names,
                levels: entities
                    .iter()
                    .zip(&entity_tags)
                    .zip(&selector_levels)
                    .map(
                        |(
                            ((key, store), (_, tags)),
                            (_, random, weather, raids, environment_attributes, sky_darkening),
                        )| {
                            crate::world::command_reads::LevelRead {
                                key: level_names.key(*key),
                                entities: store,
                                tags: tags.as_deref(),
                                random,
                                weather: *weather,
                                sky_darkening: *sky_darkening,
                                raids,
                                environment_attributes: *environment_attributes,
                            }
                        },
                    )
                    .collect(),
                chunk_inputs: level_blocks
                    .iter()
                    .map(|(key, chunks)| (level_names.key(*key), LootChunks::Chunks(chunks)))
                    .collect(),
                maps: Some(&maps),
                generate_structures,
                resolver: std::cell::OnceCell::new(),
                datapacks,
                item_tags: tags,
                registries,
                clocks: &self.world_clocks,
                world_seed: self.cold.seed,
                storage,
                scoreboard,
                advancements,
                loot: Some(&startup.loot),
                entity_tags: overworld_tags,
                enchantment_registry,
                enchantment_slots,
            };
            Ok(
                crate::commands::resolve_loot_text(value, text_source, &at, &reads)
                    .unwrap_or_else(|_| value.clone()),
            )
        };
        let tool = tool.map(|stack| {
            crate::entity::mob_death_loot::loot_tool_for_persistent_stack(
                stack,
                &self.datapack_tables.enchantment_slots,
            )
        });
        let zoom_seed = bbb_server_worldgen::noise::biome::biome_zoom_seed(self.cold.seed);
        let context = CommandLootContext {
            dimension: at.level.as_str(),
            origin,
            environment: &environment,
            blocks: &loaded_block_state,
            block_entity_data: &block_entity_data,
            weather,
            biome: &|pos| {
                crate::world::natural_spawn::fuzzy_biome_resource_id(
                    registries,
                    &chunks.borrow(),
                    zoom_seed,
                    pos,
                )
                .map(str::to_owned)
            },
            sky: &|pos| {
                LoadedBlocks {
                    chunks: &chunks.borrow(),
                }
                .loaded_can_see_sky(pos)
            },
            light: &|pos| {
                let chunks = chunks.borrow();
                LoadedBlocks { chunks: &chunks }.loaded_block_state(pos)?;
                Some(crate::world::simulation_level::max_local_raw_brightness(
                    &chunks,
                    pos,
                    i32::from(sky_darkening?),
                ))
            },
            structures: &|pos, expected| {
                LoadedBlocks {
                    chunks: &chunks.borrow(),
                }
                .loaded_structure_matches(pos, expected, &resolver)
            },
            param_set,
            tool: tool.as_ref(),
            text_resolver: &text_resolver,
            vehicle_slots: &vehicle_slots,
        };
        let project_stored = |identity: &CommandSource| match &identity.identity {
            CommandSourceIdentity::StoredEntity {
                entity_id,
                dimension,
                ..
            } => {
                let level = level_names.id(dimension);
                entities
                    .iter()
                    .find(|(key, _)| Some(*key) == level)
                    .and_then(|(key, store)| {
                        let entity = store
                            .get(*entity_id)
                            .filter(|entity| entity.uuid() == identity.uuid)?;
                        let raids = selector_levels.iter().find(|(id, ..)| id == key)?.3;
                        Some(LootEntity::Stored(crate::loot_runtime::StoredLootEntity {
                            entity,
                            bodies: Some(crate::entity::store::EntityBodyReadView::new(
                                store, None, None,
                            )),
                            raid_at_position: Some(raids.raid_at(entity.block_position())),
                        }))
                    })
                    .unwrap_or(LootEntity::Absent)
            }
            _ => LootEntity::Absent,
        };
        let source_entity = if matches!(source.identity, CommandSourceIdentity::Player) {
            player_view
                .as_ref()
                .map_or(LootEntity::Absent, LootEntity::Facts)
        } else {
            project_stored(source)
        };
        let target_entity = target.map(|target| {
            if matches!(target.identity, CommandSourceIdentity::Player) {
                target_player_view
                    .as_ref()
                    .map_or(LootEntity::Absent, LootEntity::Facts)
            } else {
                project_stored(target)
            }
        });
        context
            .evaluate(
                source_entity,
                target_entity,
                Some(startup.tags.registry("minecraft:damage_type")),
                matches!(source.identity, CommandSourceIdentity::Player),
                &mut evaluate,
            )
            .map_err(|error| component_literal_nbt(error.to_string()))
    }
}

fn runtime_loot_stacks(drops: Vec<LootStack>) -> Vec<RuntimeItemStack> {
    drops
        .into_iter()
        .map(RuntimeItemStack::from_summary)
        .collect()
}

struct CommandLootContext<'a> {
    dimension: &'a str,
    origin: [f64; 3],
    environment: &'a dyn crate::loot_runtime::LootItemEnvironment,
    blocks: &'a crate::loot_runtime::LootBlockStateAt<'a>,
    block_entity_data: &'a crate::loot_runtime::LootBlockEntityDataAt<'a>,
    weather: crate::loot_runtime::LootWeather,
    biome: &'a crate::loot_runtime::LootBiomeAt<'a>,
    sky: &'a crate::loot_runtime::LootSkyAt<'a>,
    light: &'a crate::loot_runtime::LootLightAt<'a>,
    structures: &'a crate::loot_runtime::LootStructuresAt<'a>,
    param_set: &'static crate::loot_runtime::LootContextParamSet,
    tool: Option<&'a crate::loot_runtime::LootTool>,
    text_resolver: &'a dyn crate::loot_runtime::LootTextResolver,
    vehicle_slots: &'a crate::loot_runtime::params::LootVehicleSlots<'a>,
}
impl CommandLootContext<'_> {
    fn evaluate<T>(
        &self,
        source_entity: LootEntity<'_>,
        target_entity: Option<LootEntity<'_>>,
        damage_tags: Option<Option<&bbb_server_datapack::tag_loader::LoadedRegistryTags>>,
        source_is_player: bool,
        evaluate: &mut impl FnMut(&LootParams<'_>) -> Result<T, crate::loot_runtime::LootRuntimeError>,
    ) -> Result<T, crate::loot_runtime::LootRuntimeError> {
        let params = LootParams {
            dimension: Some(self.dimension),
            origin: Some(LootOrigin {
                x: self.origin[0],
                y: self.origin[1],
                z: self.origin[2],
            }),
            entities: LootEntities {
                this_entity: target_entity.unwrap_or(source_entity),
                attacking_entity: target_entity.map_or(LootEntity::Absent, |_| source_entity),
                direct_attacking_entity: target_entity
                    .map_or(LootEntity::Absent, |_| source_entity),
                last_damage_player: if target_entity.is_some() && source_is_player {
                    source_entity
                } else {
                    LootEntity::Absent
                },
                ..Default::default()
            },
            damage_source: target_entity.map_or(
                crate::loot_runtime::LootDamageSource::Absent,
                |_| {
                    crate::loot_runtime::LootDamageSource::Present(
                        crate::loot_runtime::LootDamageSourceFacts {
                            damage_type: "minecraft:magic",
                            is_direct: Some(true),
                            tags: damage_tags,
                            causing: LootEntity::Absent,
                            direct: Some(LootEntity::Absent),
                        },
                    )
                },
            ),
            tool: self.tool,
            item_environment: Some(self.environment),
            text_resolver: Some(self.text_resolver),
            vehicle_slots: Some(self.vehicle_slots),
            level: Some(self.blocks),
            block_entity_data: Some(self.block_entity_data),
            weather: Some(self.weather),
            biome: Some(self.biome),
            sky: Some(self.sky),
            light: Some(self.light),
            structures: Some(self.structures),
            ..LootParams::new(self.param_set)
        };
        evaluate(&params)
    }
}

#[cfg(test)]
mod tests;
