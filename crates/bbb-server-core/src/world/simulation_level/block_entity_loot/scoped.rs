use super::*;
use crate::entity::block_drops::BlockDropSource;
use crate::loot_runtime::LootRuntimeReason;
use crate::random::GameRandom;
use crate::world::level_blocks::{LoadedBlocks, LootChunks};

pub(super) struct ScopedLootLevel<'a, 'w> {
    pub(super) level: RefCell<LevelCtx<'a, 'w>>,
}

impl ScopedLootLevel<'_, '_> {
    pub(super) fn with_view<R>(
        &self,
        held: Option<BlockEntity>,
        run: impl FnOnce(BlockDropSource<'_>, &GameRandom, BlockDropView<'_>, &LootLevelFacts<'_>) -> R,
    ) -> R {
        let (facts, recipes) = {
            let level = self.level.borrow();
            (
                std::rc::Rc::clone(&level.loot_level_facts),
                level.drop_tables.recipes,
            )
        };
        let player_slots = |uuid, slot| self.player_slot(uuid, slot);
        let block_entity_slots =
            |_: &dyn LootItemEnvironment, original: &BlockEntity, slots: &[i32]| {
                let mut level = self.level.borrow_mut();
                level
                    .unpack_original_container_loot(original, None)
                    .map_err(|error| LootRuntimeReason::Catalog(error.to_string()))?;
                let body = level.current_container_instance(original);
                Ok(body.loot_slots().map_or_else(Vec::new, |items| {
                    slots
                        .iter()
                        .filter_map(|slot| usize::try_from(*slot).ok().and_then(|i| items.get(i)))
                        .cloned()
                        .collect()
                }))
            };
        let player_nbt = |uuid| self.player_nbt(uuid);
        let entity_team = |entity: crate::loot_runtime::LootEntity<'_>,
                           expected: &bbb_protocol::JavaString| {
            let level = self.level.borrow();
            let bodies = level.structure_entities.as_ref().map(|context| {
                crate::entity::store::EntityBodyReadView::new(
                    context.store,
                    context.current,
                    context.ancestors,
                )
            });
            facts
                .entity_scoreboard(bodies)
                .ok_or_else(|| {
                    LootRuntimeReason::Catalog("entity team requires the actual scoreboard".into())
                })?
                .matches(entity, expected, None)
        };
        let entity_score =
            |entity: crate::loot_runtime::LootEntity<'_>,
             objective: &bbb_protocol::JavaString,
             bodies: Option<crate::entity::store::EntityBodyReadView<'_>>| {
                let level = self.level.borrow();
                let current = level.structure_entities.as_ref().map(|context| {
                    crate::entity::store::EntityBodyReadView::new(
                        context.store,
                        context.current,
                        context.ancestors,
                    )
                });
                facts
                    .entity_scoreboard(current)
                    .ok_or_else(|| {
                        LootRuntimeReason::Catalog("score requires the actual scoreboard".into())
                    })?
                    .entity_score(entity, objective, bodies)
            };
        let environment =
            LevelLootEnvironment::for_level(recipes, LootChunks::Level(&self.level), &facts, None)
                .map(|environment| {
                    environment
                        .with_player_nbt(&player_nbt)
                        .with_player_slots(&player_slots)
                        .with_block_entity_slots(&block_entity_slots)
                        .with_entity_team(&entity_team)
                        .with_entity_score(&entity_score)
                });
        let loaded_block_state = |pos| {
            LoadedBlocks {
                chunks: self.level.borrow().chunks,
            }
            .loaded_block_state(pos)
        };
        let block_entity_data = |pos| {
            let level = self.level.borrow();
            let resolver = facts.item_stack_resolver.as_ref().ok_or_else(|| {
                LootRuntimeReason::Catalog("block entity NBT publication is unavailable".into())
            })?;
            level
                .chunks
                .block_entity_loot_data(pos, Some(resolver))
                .map_err(|error| LootRuntimeReason::Catalog(error.to_string()))
        };
        let view = BlockDropView {
            dimension: facts.dimension_key.as_deref().map(|key| key.as_str()),
            level: Some(&loaded_block_state),
            block_entity_data: Some(&block_entity_data),
            block_entity: held.as_ref().map(|body| LootBlockEntity {
                block_entity: body,
                projection: item_projection(&facts),
            }),
            item_environment: environment
                .as_ref()
                .map(|view| view as &dyn LootItemEnvironment),
        };
        let random = self.level.borrow().random;
        run(self.source(), random, view, &facts)
    }

    pub(super) fn source(&self) -> BlockDropSource<'_> {
        let level = self.level.borrow();
        BlockDropSource {
            tables: level.drop_tables.tables,
        }
    }

    fn player_slot(
        &self,
        uuid: uuid::Uuid,
        slot: i32,
    ) -> Result<Option<crate::loot_runtime::LootTool>, LootRuntimeReason> {
        let level = self.level.borrow();
        let facts = &level.loot_level_facts;
        crate::world::loot_context::PlayerLootNbtSources {
            snapshot: None,
            resolver: None,
            dimension: facts.dimension.as_ref(),
            bodies: level.structure_entities.as_ref().map(|context| {
                crate::entity::store::EntityBodyReadView::new(
                    context.store,
                    context.current,
                    context.ancestors,
                )
            }),
            other_levels: facts.other_levels.map_or(
                crate::world::loot_context::entity_scoreboard::LootEntityLevels::Stores(&[]),
                crate::world::loot_context::entity_scoreboard::LootEntityLevels::Other,
            ),
        }
        .slot(uuid, slot)
    }

    pub(super) fn player_nbt(
        &self,
        uuid: uuid::Uuid,
    ) -> Result<bbb_protocol::NbtSummaryValue, LootRuntimeReason> {
        let level = self.level.borrow();
        let facts = &level.loot_level_facts;
        crate::world::loot_context::PlayerLootNbtSources {
            snapshot: facts.entity_nbt.as_ref(),
            resolver: facts.item_stack_resolver.as_ref(),
            dimension: facts.dimension.as_ref(),
            bodies: level.structure_entities.as_ref().map(|context| {
                crate::entity::store::EntityBodyReadView::new(
                    context.store,
                    context.current,
                    context.ancestors,
                )
            }),
            other_levels: facts.other_levels.map_or(
                crate::world::loot_context::entity_scoreboard::LootEntityLevels::Stores(&[]),
                crate::world::loot_context::entity_scoreboard::LootEntityLevels::Other,
            ),
        }
        .read(uuid)
    }
}
