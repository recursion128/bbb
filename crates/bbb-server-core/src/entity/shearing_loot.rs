use crate::entity::block_drops::{
    loot_tool_for_stack, unresolved_tool_enchantments, BlockDropTables,
};
use crate::entity::store::EntityStore;
use crate::entity::Entity;
use crate::loot_runtime::{
    loot_table_by_id, loot_table_drops, param_sets, LootEntities, LootEntity, LootOrigin,
    LootParams, LootWeather, StoredLootEntity,
};
use bbb_protocol::packets::ItemStackSummary;
use bbb_server_types::item_stack::RuntimeItemStack;

pub(crate) struct ShearingLoot<'a> {
    pub(crate) tables: BlockDropTables<'a>,
    pub(crate) interacting_entity: LootEntity<'a>,
    pub(crate) dimension: Option<&'a str>,
    pub(crate) weather: LootWeather,
    pub(crate) level: Option<&'a crate::loot_runtime::LootBlockStateAt<'a>>,
    pub(crate) block_entity_data: Option<&'a crate::loot_runtime::LootBlockEntityDataAt<'a>>,
    pub(crate) item_environment: Option<&'a dyn crate::loot_runtime::LootItemEnvironment>,
    pub(crate) enchantment_registry: &'a [String],
}

pub(crate) fn draw_loot(
    table_id: &str,
    store: &EntityStore,
    target: &Entity,
    hand: &ItemStackSummary,
    loot: &mut ShearingLoot<'_>,
) -> Option<Vec<RuntimeItemStack>> {
    let position = target.position();
    if !unresolved_tool_enchantments(hand, loot.enchantment_registry).is_empty() {
        return None;
    }
    let tables = loot.tables.tables?;
    if loot_table_by_id(tables, table_id).is_none() {
        return Some(Vec::new());
    }
    let tool = loot_tool_for_stack(hand, loot.enchantment_registry);
    let params = LootParams {
        origin: Some(LootOrigin {
            x: position.x,
            y: position.y,
            z: position.z,
        }),
        tool: Some(&tool),
        entities: LootEntities {
            this_entity: LootEntity::Stored(StoredLootEntity {
                entity: target,
                bodies: Some(crate::entity::store::EntityBodyReadView::new(
                    store,
                    Some(target),
                    None,
                )),
                raid_at_position: None,
            }),
            ..LootEntities::default()
        },
        dimension: loot.dimension,
        weather: Some(loot.weather),
        level: loot.level,
        block_entity_data: loot.block_entity_data,
        item_environment: loot.item_environment,
        ..LootParams::new(param_sets::SHEARING)
    };
    let drawn = loot_table_drops(tables, table_id, &params, store.random()).ok()?;
    Some(
        drawn
            .into_iter()
            .map(RuntimeItemStack::from_summary)
            .collect(),
    )
}

/// `LivingEntity.dropFromEntityInteractLootTable` (`LivingEntity.java:1552-1567`).
pub(crate) fn draw_entity_interact_loot(
    table_id: &str,
    store: &EntityStore,
    target: &Entity,
    hand: &ItemStackSummary,
    loot: &mut ShearingLoot<'_>,
) -> Option<Vec<RuntimeItemStack>> {
    if !unresolved_tool_enchantments(hand, loot.enchantment_registry).is_empty() {
        return None;
    }
    let tables = loot.tables.tables?;
    if loot_table_by_id(tables, table_id).is_none() {
        return Some(Vec::new());
    }
    let tool = loot_tool_for_stack(hand, loot.enchantment_registry);
    let params = LootParams {
        tool: Some(&tool),
        entities: LootEntities {
            target_entity: LootEntity::Stored(StoredLootEntity {
                entity: target,
                bodies: Some(crate::entity::store::EntityBodyReadView::new(
                    store,
                    Some(target),
                    None,
                )),
                raid_at_position: None,
            }),
            interacting_entity: loot.interacting_entity,
            ..LootEntities::default()
        },
        dimension: loot.dimension,
        weather: Some(loot.weather),
        level: loot.level,
        block_entity_data: loot.block_entity_data,
        item_environment: loot.item_environment,
        ..LootParams::new(param_sets::ENTITY_INTERACT)
    };
    Some(
        loot_table_drops(tables, table_id, &params, store.random())
            .ok()?
            .into_iter()
            .map(RuntimeItemStack::from_summary)
            .collect(),
    )
}
