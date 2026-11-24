use crate::block_entity::{BlockEntity, BlockEntityItemProjection, ContainerLoot};
use crate::chunk::ChunkStore;
use crate::entity::block_drops::{pop_resources, BlockDropTables, BlockDropView};
use crate::loot_runtime::{
    fill_container, param_sets, LootBlockEntity, LootEntities, LootEntity, LootItemEnvironment,
    LootOrigin, LootParams, LootTool,
};
use crate::random::GameRandom;
use crate::world::level_blocks::{LoadedBlocks, LootChunks};
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::packets::BlockPos;
use bbb_server_types::block_queries::block_state_has_block_entity;
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::{Result as CoreResult, ServerWorldError};
use std::cell::RefCell;

mod scoped;
use scoped::ScopedLootLevel;

use super::mob_death_loot::LevelLootEnvironment;
use super::{LevelCtx, LootLevelFacts};

/// Which object `Block.dropResources` hands `getDrops` as `BLOCK_ENTITY`.
#[derive(Clone, Copy)]
pub(in crate::world) enum DropBlockEntity<'a> {
    Live,
    Held(Option<&'a BlockEntity>),
}

/// The table a block drop draws.
#[derive(Clone, Copy)]
pub(in crate::world) enum DropTable<'a> {
    Block,
    Named(&'a str),
}

pub(in crate::world) struct ContainerLootOpener<'a> {
    pub(in crate::world) luck: f32,
    pub(in crate::world) entity: LootEntity<'a>,
}

pub(in crate::world) type VehicleSlotsReader<'a> = &'a dyn Fn(
    &crate::entity::Entity,
    &[i32],
) -> Result<
    Vec<bbb_server_types::item_stack::RuntimeItemStack>,
    crate::loot_runtime::LootRuntimeReason,
>;

/// Shared predicate loot environment and callback handles for live damage,
/// equipment-drop, location-effect, and cross-dimension queries.
pub(in crate::world) struct PredicateLootRefs<'a> {
    pub(in crate::world) dimension: Option<&'a str>,
    pub(in crate::world) item_environment: &'a dyn LootItemEnvironment,
    pub(in crate::world) block_at: &'a dyn Fn(BlockPos) -> Option<i32>,
    pub(in crate::world) block_entity_data: &'a dyn Fn(
        crate::loot_runtime::LootBlockEntityRead,
    ) -> Result<
        Option<crate::loot_runtime::LootBlockEntityData>,
        crate::loot_runtime::LootRuntimeReason,
    >,
    pub(in crate::world) sky: &'a dyn Fn(BlockPos) -> Option<bool>,
    pub(in crate::world) vehicle_slots: VehicleSlotsReader<'a>,
}

#[allow(clippy::too_many_arguments)]
pub(in crate::world) fn with_predicate_loot_context<R>(
    chunks: &ChunkStore,
    facts: &LootLevelFacts<'_>,
    tables: Option<&bbb_server_datapack::loot::LootRegistries>,
    weather: crate::loot_runtime::LootWeather,
    random: &GameRandom,
    bodies: Option<crate::entity::store::EntityBodyReadView<'_>>,
    with_player_slots: bool,
    resolver_override: Option<&dyn bbb_registry_codec::PersistentItemStackHolderResolver>,
    missing_tables_error: &'static str,
    body: impl FnOnce(PredicateLootRefs<'_>) -> R,
) -> R {
    let blocks = LoadedBlocks { chunks };
    let sources = crate::world::loot_context::PlayerLootNbtSources {
        snapshot: None,
        resolver: None,
        dimension: facts.dimension.as_ref(),
        bodies,
        other_levels: facts.other_levels.map_or(
            crate::world::loot_context::entity_scoreboard::LootEntityLevels::Stores(&[]),
            crate::world::loot_context::entity_scoreboard::LootEntityLevels::Other,
        ),
    };
    let player_slots = |uuid, slot| sources.slot(uuid, slot);
    let mut item_environment = LevelLootEnvironment::for_predicates(chunks, facts, bodies);
    if with_player_slots {
        item_environment = item_environment.with_player_slots(&player_slots);
    }
    let dimension = facts.dimension_key.as_deref().map(|key| key.as_str());
    let block_at = |pos| blocks.loaded_block_state(pos);
    let sky = |pos| blocks.loaded_can_see_sky(pos);
    let block_entity_data = |pos| {
        let resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver =
            if let Some(resolver) = resolver_override {
                resolver
            } else {
                facts.item_stack_resolver.as_ref().ok_or_else(|| {
                    crate::loot_runtime::LootRuntimeReason::Catalog(
                        "block entity NBT publication is unavailable".into(),
                    )
                })?
            };
        chunks
            .block_entity_loot_data(pos, Some(resolver))
            .map_err(|error| crate::loot_runtime::LootRuntimeReason::Catalog(error.to_string()))
    };
    let vehicle_slots = |requested: &crate::entity::Entity, selected: &[i32]| {
        vehicle_slots_with_environment(
            tables.ok_or_else(|| {
                crate::loot_runtime::LootRuntimeReason::Catalog(missing_tables_error.into())
            })?,
            Some(&item_environment),
            dimension,
            &block_at,
            &block_entity_data,
            weather,
            random,
            bodies,
            requested,
            selected,
        )
    };
    body(PredicateLootRefs {
        dimension,
        item_environment: &item_environment,
        block_at: &block_at,
        block_entity_data: &block_entity_data,
        sky: &sky,
        vehicle_slots: &vehicle_slots,
    })
}

impl LevelCtx<'_, '_> {
    pub(in crate::world) fn drop_block_loot_at(
        &mut self,
        pos: BlockPos,
        block_state_id: i32,
        table: DropTable<'_>,
        block_entity: DropBlockEntity<'_>,
        tool: &LootTool,
        breaker: LootEntity<'_>,
    ) {
        let mut drawn =
            self.draw_block_loot_at(pos, block_state_id, table, block_entity, tool, breaker);
        let LevelCtx {
            random,
            game_rules,
            item_spawns,
            ..
        } = self;
        pop_resources(&mut drawn, random, game_rules.block_drops, item_spawns);
    }

    pub(in crate::world) fn draw_block_loot_at(
        &mut self,
        pos: BlockPos,
        block_state_id: i32,
        table: DropTable<'_>,
        block_entity: DropBlockEntity<'_>,
        tool: &LootTool,
        breaker: LootEntity<'_>,
    ) -> Vec<crate::entity::block_drops::PendingPopResource> {
        let held = match block_entity {
            DropBlockEntity::Live => block_state_has_block_entity(block_state_id)
                .then(|| self.chunks.block_entities().get(pos))
                .flatten()
                .cloned(),
            DropBlockEntity::Held(held) => held.map(|body| self.current_container_instance(body)),
        };
        let scope = ScopedLootLevel {
            level: RefCell::new(self.reborrow()),
        };
        let mut drawn = Vec::new();
        scope.with_view(held, |source, random, view, _facts| match table {
            DropTable::Block => crate::entity::block_drops::drop_resources_with_source(
                source,
                random,
                view,
                pos,
                block_state_id,
                tool,
                breaker,
                &mut drawn,
            ),
            DropTable::Named(table) => crate::entity::block_drops::drop_named_table_with_source(
                source,
                random,
                table,
                view,
                pos,
                block_state_id,
                tool,
                breaker,
                &mut drawn,
            ),
        });
        drawn
    }

    pub(in crate::world) fn collect_explosion_block_loot_at(
        &mut self,
        pos: BlockPos,
        block_state_id: i32,
        explosion_radius: Option<f32>,
        breaker: LootEntity<'_>,
        collectors: &mut Vec<crate::entity::block_drops::PendingPopResource>,
    ) {
        let held = block_state_has_block_entity(block_state_id)
            .then(|| self.chunks.block_entities().get(pos))
            .flatten()
            .cloned();
        let scope = ScopedLootLevel {
            level: RefCell::new(self.reborrow()),
        };
        scope.with_view(held, |source, random, view, _| {
            crate::entity::block_drops::collect_explosion_resources_with_source(
                source,
                random,
                view,
                pos,
                block_state_id,
                explosion_radius,
                breaker,
                collectors,
            );
        });
    }

    pub(in crate::world) fn unpack_container_loot(
        &mut self,
        pos: BlockPos,
        opener: Option<ContainerLootOpener<'_>>,
    ) -> CoreResult<()> {
        let changed = {
            let chunks = RefCell::new(&mut *self.chunks);
            unpack_live_container_loot(&chunks, pos, |request, available| {
                fill_unpacked_container(
                    &mut chunks.borrow_mut(),
                    &mut self.drop_tables,
                    &self.loot_level_facts,
                    self.structure_entities.as_ref().map(|context| {
                        crate::entity::store::EntityBodyReadView::new(
                            context.store,
                            context.current,
                            context.ancestors,
                        )
                    }),
                    self.weather,
                    self.random,
                    LootOrigin::at_center_of(pos),
                    request,
                    opener,
                    available,
                )
                .map_err(|detail| {
                    ServerWorldError::InvalidConfiguration(format!(
                        "{} could not unpack into the container: {detail}",
                        request.table_id
                    ))
                })
            })?
        };
        if changed {
            if let Some(body) = self.chunks.block_entities().get(pos) {
                self.block_entity_set_changed(pos, body.block_state());
            }
        }
        Ok(())
    }

    pub(in crate::world) fn unpack_original_container_loot(
        &mut self,
        original: &BlockEntity,
        opener: Option<ContainerLootOpener<'_>>,
    ) -> CoreResult<()> {
        let pos = original.block_pos();
        if self
            .chunks
            .block_entities()
            .get(pos)
            .is_some_and(|body| body.same_position_and_type(original))
        {
            return self.unpack_container_loot(pos, opener);
        }
        let mut body = self.current_container_instance(original);
        let Some((request, available)) = take_container_fill(&mut body) else {
            return Ok(());
        };
        self.publish_container_instance(&body);
        let written = self.fill_container_slots(pos, &request, opener, available)?;
        for (slot, stack) in written {
            self.set_container_stack(&body, slot, stack)?;
        }
        Ok(())
    }

    fn fill_container_slots(
        &mut self,
        pos: BlockPos,
        request: &ContainerLoot,
        opener: Option<ContainerLootOpener<'_>>,
        available: Vec<usize>,
    ) -> CoreResult<Vec<(usize, bbb_server_types::item_stack::RuntimeItemStack)>> {
        let filled = {
            let LevelCtx {
                chunks,
                drop_tables,
                loot_level_facts,
                weather,
                random,
                ..
            } = self;
            fill_unpacked_container(
                chunks,
                drop_tables,
                loot_level_facts,
                self.structure_entities.as_ref().map(|context| {
                    crate::entity::store::EntityBodyReadView::new(
                        context.store,
                        context.current,
                        context.ancestors,
                    )
                }),
                weather,
                random,
                LootOrigin::at_center_of(pos),
                request,
                opener,
                available,
            )
        };
        filled.map_err(|detail| {
            ServerWorldError::InvalidConfiguration(format!(
                "{} could not unpack into the container: {detail}",
                request.table_id
            ))
        })
    }

    pub(in crate::world) fn current_container_instance(
        &self,
        original: &BlockEntity,
    ) -> BlockEntity {
        original.resolve_instance(self.chunks.block_entities())
    }

    pub(in crate::world) fn publish_container_instance(&mut self, body: &BlockEntity) {
        if let Some(current) = self.chunks.block_entities_mut().get_mut(body.block_pos()) {
            if current.same_position_and_type(body) {
                *current = body.clone();
            }
        }
    }

    pub(crate) fn set_randomizable_container_item(
        &mut self,
        pos: BlockPos,
        slot: usize,
        stack: bbb_server_types::item_stack::RuntimeItemStack,
    ) -> CoreResult<()> {
        let Some(body) = self.chunks.block_entities_mut().get_mut(pos) else {
            return Ok(());
        };
        if body
            .randomizable_container_mut()
            .is_none_or(|(_, items)| slot >= items.len())
        {
            return Ok(());
        }
        let type_id = body.type_id();
        let block_state = body.block_state();
        let enabled = body.enable_filled_crafter_slot(slot);
        if enabled {
            self.block_entity_set_changed(pos, block_state);
        }
        self.unpack_container_loot(pos, None)?;
        let Some(body) = self
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .filter(|body| body.type_id() == type_id)
        else {
            return Ok(());
        };
        let block_state = body.block_state();
        let marks = body.set_filled_original(slot, stack);
        if marks {
            self.block_entity_set_changed(pos, block_state);
        }
        Ok(())
    }

    pub(in crate::world) fn set_container_stack(
        &mut self,
        original: &BlockEntity,
        slot: usize,
        stack: bbb_server_types::item_stack::RuntimeItemStack,
    ) -> CoreResult<()> {
        if self
            .chunks
            .block_entities()
            .get(original.block_pos())
            .is_some_and(|body| body.same_position_and_type(original))
        {
            return self.set_randomizable_container_item(original.block_pos(), slot, stack);
        }
        let mut body = self.current_container_instance(original);
        if body
            .randomizable_container_mut()
            .is_none_or(|(_, items)| slot >= items.len())
        {
            return Ok(());
        }
        let pos = body.block_pos();
        let enabled = body.enable_filled_crafter_slot(slot);
        self.publish_container_instance(&body);
        if enabled {
            self.block_entity_set_changed(pos, body.block_state());
        }
        self.unpack_original_container_loot(&body, None)?;
        body = self.current_container_instance(&body);
        let marks = body.set_filled_original(slot, stack);
        self.publish_container_instance(&body);
        if marks {
            self.block_entity_set_changed(pos, body.block_state());
        }
        Ok(())
    }

    pub(in crate::world) fn set_crafter_slot_state(
        &mut self,
        original: &BlockEntity,
        slot: i32,
        enabled: bool,
    ) {
        let Some(body) = self
            .chunks
            .block_entities_mut()
            .get_mut(original.block_pos())
            .filter(|body| body.same_position_and_type(original))
        else {
            return;
        };
        let Some(crafter) = body.crafter_mut() else {
            return;
        };
        if !crafter.set_slot_disabled(slot, !enabled) {
            return;
        }
        let pos = body.block_pos();
        let block_state = body.block_state();
        self.block_entity_set_changed(pos, block_state);
    }

    pub(in crate::world) fn block_entity_set_changed(&mut self, pos: BlockPos, block_state: i32) {
        self.chunks.block_entities_mut().set_changed(pos);
        if !block_state_is_air(block_state) {
            if let Some(block) = vanilla_block_state_block_type_id(block_state) {
                self.update_neighbour_for_output_signal(pos, block);
            }
        }
    }
}

fn take_container_fill(body: &mut BlockEntity) -> Option<(ContainerLoot, Vec<usize>)> {
    let (loot, slots) = body.randomizable_container_mut()?;
    let request = loot.take()?;
    let available = slots
        .iter()
        .enumerate()
        .filter(|(_, stack)| stack.count() == 0)
        .map(|(slot, _)| slot)
        .collect();
    Some((request, available))
}

pub(in crate::world) fn unpack_live_container_loot(
    chunks: &RefCell<&mut ChunkStore>,
    pos: BlockPos,
    fill: impl FnOnce(
        &ContainerLoot,
        Vec<usize>,
    ) -> CoreResult<Vec<(usize, bbb_server_types::item_stack::RuntimeItemStack)>>,
) -> CoreResult<bool> {
    let pending = {
        let mut chunks = chunks.borrow_mut();
        chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(take_container_fill)
    };
    let Some((request, available)) = pending else {
        return Ok(false);
    };
    chunks.borrow_mut().block_entities_mut().set_changed(pos);
    let written = fill(&request, available)?;
    let mut chunks = chunks.borrow_mut();
    let Some(body) = chunks.block_entities_mut().get_mut(pos) else {
        return Ok(false);
    };
    let mut changed = false;
    for (slot, stack) in written {
        changed |= body.enable_filled_crafter_slot(slot);
        changed |= body.set_filled_original(slot, stack);
    }
    Ok(changed)
}

#[allow(clippy::too_many_arguments)]
pub(in crate::world) fn fill_unpacked_container(
    chunks: &mut ChunkStore,
    drop_tables: &mut BlockDropTables<'_>,
    facts: &LootLevelFacts<'_>,
    bodies: Option<crate::entity::store::EntityBodyReadView<'_>>,
    weather: &crate::world::weather::LevelWeather,
    level_random: &GameRandom,
    origin: LootOrigin,
    request: &ContainerLoot,
    opener: Option<ContainerLootOpener<'_>>,
    available: Vec<usize>,
) -> Result<Vec<(usize, bbb_server_types::item_stack::RuntimeItemStack)>, String> {
    let registries = drop_tables
        .tables
        .ok_or_else(|| "no loot registries are loaded".to_string())?;
    let chunks = RefCell::new(chunks);

    let environment = LevelLootEnvironment::for_level(
        drop_tables.recipes,
        LootChunks::Chunks(&chunks),
        facts,
        bodies,
    );
    fill_unpacked_container_with_environment(
        registries,
        environment
            .as_ref()
            .map(|environment| environment as &dyn LootItemEnvironment),
        facts.dimension_key.as_deref().map(|key| key.as_str()),
        &|pos| {
            LoadedBlocks {
                chunks: &chunks.borrow(),
            }
            .loaded_block_state(pos)
        },
        &|pos| {
            let resolver = facts.item_stack_resolver.as_ref().ok_or_else(|| {
                crate::loot_runtime::LootRuntimeReason::Catalog(
                    "block-entity data needs the level item publication".into(),
                )
            })?;
            chunks
                .borrow()
                .block_entity_loot_data(pos, Some(resolver.as_dyn()))
                .map_err(|error| crate::loot_runtime::LootRuntimeReason::Catalog(error.to_string()))
        },
        crate::loot_runtime::LootWeather {
            raining: weather.is_raining(),
            thundering: weather.is_thundering(),
        },
        level_random,
        origin,
        request,
        opener,
        available,
    )
}

#[allow(clippy::too_many_arguments)]
pub(in crate::world) fn vehicle_slots_with_environment(
    registries: &bbb_server_datapack::loot::LootRegistries,
    environment: Option<&dyn LootItemEnvironment>,
    dimension: Option<&str>,
    level: &crate::loot_runtime::LootBlockStateAt<'_>,
    block_entity_data: &crate::loot_runtime::LootBlockEntityDataAt<'_>,
    weather: crate::loot_runtime::LootWeather,
    level_random: &GameRandom,
    bodies: Option<crate::entity::store::EntityBodyReadView<'_>>,
    requested: &crate::entity::Entity,
    selected: &[i32],
) -> Result<
    Vec<bbb_server_types::item_stack::RuntimeItemStack>,
    crate::loot_runtime::LootRuntimeReason,
> {
    use crate::loot_runtime::LootRuntimeReason;

    let actual = bodies
        .and_then(|bodies| bodies.get(requested.id()))
        .filter(|actual| actual.uuid() == requested.uuid())
        .ok_or_else(|| {
            LootRuntimeReason::Catalog("vehicle slots need the actual source level/body".into())
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
    crate::entity::item_slots::unpack_vehicle_cargo(actual, |position, request, available| {
        fill_unpacked_container_with_environment(
            registries,
            environment,
            dimension,
            level,
            block_entity_data,
            weather,
            level_random,
            LootOrigin {
                x: position.x,
                y: position.y,
                z: position.z,
            },
            request,
            None,
            available,
        )
    })
    .map_err(LootRuntimeReason::Catalog)?;
    selected.iter().try_fold(Vec::new(), |mut output, slot| {
        if let Some(stack) =
            crate::entity::item_slots::read(actual, *slot).map_err(LootRuntimeReason::Catalog)?
        {
            output.push(stack);
        }
        Ok(output)
    })
}

#[allow(clippy::too_many_arguments)]
pub(in crate::world) fn fill_unpacked_container_with_environment(
    registries: &bbb_server_datapack::loot::LootRegistries,
    environment: Option<&dyn LootItemEnvironment>,
    dimension: Option<&str>,
    level: &crate::loot_runtime::LootBlockStateAt<'_>,
    block_entity_data: &crate::loot_runtime::LootBlockEntityDataAt<'_>,
    weather: crate::loot_runtime::LootWeather,
    level_random: &GameRandom,
    origin: LootOrigin,
    request: &ContainerLoot,
    opener: Option<ContainerLootOpener<'_>>,
    available: Vec<usize>,
) -> Result<Vec<(usize, bbb_server_types::item_stack::RuntimeItemStack)>, String> {
    let (luck, this_entity) = opener.map_or((0.0, LootEntity::Absent), |opener| {
        (opener.luck, opener.entity)
    });
    let params = LootParams {
        dimension,
        origin: Some(origin),
        entities: LootEntities {
            this_entity,
            ..LootEntities::default()
        },
        luck,
        item_environment: environment,
        level: Some(level),
        block_entity_data: Some(block_entity_data),
        weather: Some(weather),
        ..LootParams::new(param_sets::CHEST)
    };
    // A saved `LootTableSeed` makes the fill repeatable.
    let seeded;
    let random = if request.seed != 0 {
        seeded = GameRandom::with_seed(request.seed);
        &seeded
    } else {
        level_random
    };
    fill_container(registries, &request.table_id, &params, random, available)
        .map_err(|error| error.to_string())
}

/// The publication a block entity's container stacks are written through when
/// a loot function collects its components.
pub(in crate::world) fn item_projection<'a>(
    facts: &'a LootLevelFacts<'_>,
) -> BlockEntityItemProjection<'a> {
    facts
        .item_stack_resolver
        .as_ref()
        .map_or_else(BlockEntityItemProjection::without_resolver, |resolver| {
            BlockEntityItemProjection::with_resolver(resolver.as_dyn())
        })
}
