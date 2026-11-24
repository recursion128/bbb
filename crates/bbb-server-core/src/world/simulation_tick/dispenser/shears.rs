//! `ShearsDispenseItemBehavior`: a full hive in front, else the first entity
//! there whose leashes can be cut or that is ready for shearing.

use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result as CoreResult;

use super::equipment::block_area;
use super::{BlockSource, Dispensed};
use crate::block_behavior::beehive::{
    honey_level, is_beehive, reset_honey_state, HARVEST_LOOT_TABLE, MAX_HONEY_LEVELS, SHEAR_SOUND,
};
use crate::entity::block_drops::loot_tool_for_runtime_stack;
use crate::entity::leash::shear_off_all_leash_connections;
use crate::entity::shearing_loot::ShearingLoot;
use crate::entity::sheep_shear::{mob_interact, SheepInteraction};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use crate::loot_runtime::LootEntity;
use crate::world::level_blocks::{LoadedBlocks, LootChunks};
use crate::world::simulation_level::block_entity_loot::{DropBlockEntity, DropTable};
use crate::world::simulation_level::mob_death_loot::LevelLootEnvironment;
use crate::world::{ServerLevel, ServerWorld};
use bbb_protocol::packets::SoundSource;

pub(super) fn dispense(
    world: &mut ServerWorld,
    source: &BlockSource,
    mut stack: RuntimeItemStack,
) -> CoreResult<Dispensed> {
    let success = shear_beehive(world, source, &stack) || shear_entity(world, source, &stack);
    if success {
        world.hurt_dispensed(source.level, &mut stack, 1);
    }
    Ok(Dispensed::optional(stack, success))
}

/// `tryShearBeehive`.
fn shear_beehive(world: &mut ServerWorld, source: &BlockSource, tool: &RuntimeItemStack) -> bool {
    let target = source.target();
    let state = world.with_level(source.level, |level| level.block_state(target));
    if !is_beehive(state) || honey_level(state) < MAX_HONEY_LEVELS {
        return false;
    }
    let tool = loot_tool_for_runtime_stack(tool, &world.datapack_tables.enchantment_registry);
    world.with_level(source.level, |level| {
        level.play_block_sound(target, SHEAR_SOUND);
        level.drop_block_loot_at(
            target,
            state,
            DropTable::Named(HARVEST_LOOT_TABLE),
            DropBlockEntity::Live,
            &tool,
            LootEntity::Absent,
        );
    });
    if let Some(reset) = reset_honey_state(state) {
        world.set_block(source.level, target, reset, SET_BLOCK_UPDATE_ALL);
        world.release_beehive_occupants(
            source.level,
            target,
            reset,
            crate::world::beehive_release::BeehiveRelease::Harvest,
        );
    }
    world.level_mut_by_id(&source.level).event_sink.game_event(
        VanillaGameEvent::SHEAR,
        target,
        None,
    );
    true
}

/// `tryShearEntity`: leashes first, then ready shearable entities.
fn shear_entity(world: &mut ServerWorld, source: &BlockSource, tool: &RuntimeItemStack) -> bool {
    let area = block_area(source);
    let tags = super::equipment::tag_view(world, &source.level);
    let enchantment_registry = std::sync::Arc::clone(&world.datapack_tables.enchantment_registry);
    let (level, tables, facts, _) = world.level_and_drop_tables(&source.level);
    let ServerLevel {
        chunks,
        entities: store,
        event_sink,
        weather,
        ..
    } = level;
    let chunks = std::cell::RefCell::new(chunks);
    let environment =
        LevelLootEnvironment::for_level(tables.recipes, LootChunks::Chunks(&chunks), &facts, None);
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
        interacting_entity: LootEntity::Absent,
        tables,
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
                .map_err(|error| crate::loot_runtime::LootRuntimeReason::Catalog(error.to_string()))
        }),
        item_environment: environment
            .as_ref()
            .map(|environment| environment as &dyn crate::loot_runtime::LootItemEnvironment),
        enchantment_registry: &enchantment_registry,
    };
    let mut sheared = false;
    for id in store.ids_in_box_sections(area) {
        let Some(entity) = store.get(id) else {
            continue;
        };
        let bounds = entity.bounding_box();
        if !bounds.intersects(area) {
            continue;
        }
        let position = entity.position();
        if shear_off_all_leash_connections(store, id, bounds.center(), position) {
            return true;
        }
        if crate::entity::bogged::shear(
            store,
            id,
            tool.summary(),
            SoundSource::Blocks,
            Some(&mut loot),
        ) || crate::entity::snow_golem::shear(
            store,
            id,
            tool.summary(),
            SoundSource::Blocks,
            Some(&mut loot),
        ) || crate::entity::mooshroom::shear(
            store,
            id,
            tool.summary(),
            SoundSource::Blocks,
            Some(&mut loot),
        ) || crate::entity::copper_golem::shear(
            store,
            id,
            tool.summary(),
            SoundSource::Blocks,
            &tags,
        ) || matches!(
            mob_interact(
                store,
                id,
                tool.summary(),
                SoundSource::Blocks,
                Some(&mut loot)
            ),
            SheepInteraction::Sheared
        ) {
            sheared = true;
            break;
        }
    }
    if sheared {
        event_sink.game_event(VanillaGameEvent::SHEAR, source.target(), None);
    }
    sheared
}
