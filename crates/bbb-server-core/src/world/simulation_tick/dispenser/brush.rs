use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result as CoreResult;

use super::{BlockSource, Dispensed};
use crate::entity::shearing_loot::ShearingLoot;
use crate::loot_runtime::LootEntity;
use crate::world::level_blocks::{LoadedBlocks, LootChunks};
use crate::world::simulation_level::mob_death_loot::LevelLootEnvironment;
use crate::world::{ServerLevel, ServerWorld};

/// `DispenseItemBehavior` brush behavior (`DispenseItemBehavior.java:370-392`).
pub(super) fn dispense(
    world: &mut ServerWorld,
    source: &BlockSource,
    mut stack: RuntimeItemStack,
) -> CoreResult<Dispensed> {
    let area = super::equipment::block_area(source);
    let enchantment_registry = std::sync::Arc::clone(&world.datapack_tables.enchantment_registry);
    let success = {
        let (level, tables, facts, _) = world.level_and_drop_tables(&source.level);
        let ServerLevel {
            chunks,
            entities: store,
            weather,
            ..
        } = level;
        let chunks = std::cell::RefCell::new(chunks);
        let environment = LevelLootEnvironment::for_level(
            tables.recipes,
            LootChunks::Chunks(&chunks),
            &facts,
            None,
        );
        let blocks = |pos| {
            LoadedBlocks {
                chunks: &chunks.borrow(),
            }
            .loaded_block_state(pos)
        };
        let mut loot = ShearingLoot {
            dimension: facts.dimension_key.as_deref().map(|key| key.as_str()),
            weather: crate::loot_runtime::LootWeather {
                raining: weather.is_raining(),
                thundering: weather.is_thundering(),
            },
            tables,
            interacting_entity: LootEntity::Absent,
            level: Some(&blocks),
            block_entity_data: Some(&|pos| {
                let resolver = facts.item_stack_resolver.as_ref().ok_or_else(|| {
                    crate::loot_runtime::LootRuntimeReason::Catalog(
                        "block entity NBT publication is unavailable".into(),
                    )
                })?;
                chunks
                    .borrow()
                    .block_entity_loot_data(pos, Some(resolver))
                    .map_err(|error| {
                        crate::loot_runtime::LootRuntimeReason::Catalog(error.to_string())
                    })
            }),
            item_environment: environment
                .as_ref()
                .map(|environment| environment as &dyn crate::loot_runtime::LootItemEnvironment),
            enchantment_registry: &enchantment_registry,
        };
        store.ids_in_box_sections(area).into_iter().any(|id| {
            store
                .get(id)
                .is_some_and(|entity| entity.bounding_box().intersects(area))
                && crate::entity::armadillo::brush(store, id, stack.summary(), Some(&mut loot))
        })
    };
    if success {
        world.hurt_dispensed(
            source.level,
            &mut stack,
            crate::entity::armadillo::BRUSH_DAMAGE,
        );
    }
    Ok(Dispensed::optional(stack, success))
}
