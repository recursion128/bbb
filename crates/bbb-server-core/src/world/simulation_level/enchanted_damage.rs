//! The enchantment reads of a stored hurt (`EnchantmentHelper.java:90-94`,
//! `:158-200`) on the world-owned generation.

use crate::enchantment_combat_effects::{CombatValue, EnchantmentCombatTable};
use crate::entity::enchanted_damage::{EnchantedDamageItems, EnchantedDamageRequest};
use crate::entity::mob_death_loot::MobLootAttacker;
use crate::item_durability::process_value_effects;
use crate::loot_runtime::{LootEntity, LootWeather};
use crate::random::GameRandom;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_server_datapack::loot::LootCondition;
use uuid::Uuid;

use super::LevelCtx;
use crate::world::combat::{
    damage_condition_matches, enchantments_immune, enchantments_value, DamageContextEntities,
    DamageContextLevel,
};
use crate::world::ServerWorld;

pub(super) fn immunity(
    slice: &mut LevelCtx<'_, '_>,
    request: EnchantedDamageRequest<'_>,
) -> Result<bool, String> {
    with_context(slice, request, |table, enchantments, _random, matches| {
        enchantments_immune(table, enchantments, matches)
    })
}

pub(super) fn value(
    slice: &mut LevelCtx<'_, '_>,
    request: EnchantedDamageRequest<'_>,
    value: CombatValue,
    amount: f32,
) -> Result<f32, String> {
    with_context(slice, request, |table, enchantments, random, matches| {
        enchantments_value(table, enchantments, value, amount, random, matches)
    })
}

/// `EnchantmentHelper.processDurabilityChange` (`EnchantmentHelper.java:90-94`).
pub(super) fn durability_change(
    slice: &mut LevelCtx<'_, '_>,
    stack: &bbb_server_types::item_stack::RuntimeItemStack,
    amount: i32,
) -> Result<i32, String> {
    let facts = &slice.loot_level_facts;
    let summary = stack.summary();
    let table = facts
        .enchantment_item_damage
        .as_ref()
        .ok_or("item damage effects are unavailable")?;
    let enchantments = ServerWorld::resolve_value_effects(summary, table);
    Ok(process_value_effects(
        summary,
        amount as f32,
        &enchantments,
        slice.random,
        &slice.value_effect_level(),
    ) as i32)
}

fn with_context<R>(
    slice: &mut LevelCtx<'_, '_>,
    request: EnchantedDamageRequest<'_>,
    body: impl FnOnce(
        &EnchantmentCombatTable,
        &[(String, i32)],
        &GameRandom,
        &mut dyn FnMut(&str, i32, &LootCondition) -> Result<bool, String>,
    ) -> Result<R, String>,
) -> Result<R, String> {
    let LevelCtx {
        chunks,
        drop_tables,
        loot_level_facts,
        random,
        weather,
        ..
    } = slice;
    let level_random: &GameRandom = random;
    let table = loot_level_facts
        .enchantment_combat_effects
        .clone()
        .ok_or("enchantment combat effects are unavailable")?;
    let bodies = request.bodies;
    let weather = LootWeather {
        raining: weather.is_raining(),
        thundering: weather.is_thundering(),
    };
    let tags = chunks.registry_tags_shared().map_or(
        crate::entity::EntityRegistryTagView::Builtin26_1,
        crate::entity::EntityRegistryTagView::Frozen,
    );
    super::block_entity_loot::with_predicate_loot_context(
        chunks,
        loot_level_facts,
        drop_tables.tables,
        weather,
        random,
        Some(bodies),
        true,
        None,
        "no loot registries are loaded",
        |loot_refs| {
            let context = DamageContextLevel {
                vehicle_slots: Some(loot_refs.vehicle_slots),
                dimension: loot_refs.dimension,
                item_environment: Some(loot_refs.item_environment),
                loot_tables: drop_tables.tables,
                damage_type_tags: drop_tables.damage_type_tags,
                blocks: loot_refs.block_at,
                block_entity_data: loot_refs.block_entity_data,
                sky: loot_refs.sky,
                weather,
            };
            with_request_context(
                &table,
                &context,
                &|uuid| {
                    loot_level_facts
                        .session_loot_view(Some(bodies), &tags, uuid)?
                        .ok_or_else(|| format!("player {uuid} has no session"))
                },
                random,
                request,
                |table, enchantments, matches| body(table, enchantments, level_random, matches),
            )
        },
    )
}

/// `Enchantment.damageContext` (`Enchantment.java:463-473`) for one request
/// from the entity crate, its session attackers answered by `session`.
pub(in crate::world) fn with_request_context<R>(
    table: &EnchantmentCombatTable,
    context: &DamageContextLevel<'_>,
    session: &dyn Fn(Uuid) -> Result<crate::loot_runtime::LootEntityFacts, String>,
    random: &GameRandom,
    request: EnchantedDamageRequest<'_>,
    body: impl FnOnce(
        &EnchantmentCombatTable,
        &[(String, i32)],
        &mut dyn FnMut(&str, i32, &LootCondition) -> Result<bool, String>,
    ) -> Result<R, String>,
) -> Result<R, String> {
    let causing_view = match request.causing {
        MobLootAttacker::SessionPlayer { uuid } => Some(session(uuid)?),
        _ => None,
    };
    let direct_view = match request.direct {
        MobLootAttacker::SessionPlayer { uuid } => Some(session(uuid)?),
        _ => None,
    };
    let enchantments = match request.items {
        EnchantedDamageItems::Listed(listed) => listed.to_vec(),
        EnchantedDamageItems::SessionWeapon { uuid } => {
            let weapon = match request.direct {
                MobLootAttacker::Entity(entity) => entity.equipment_item(EquipmentSlot::MainHand),
                _ => LootEntity::Facts(&session(uuid)?).equipment_item(EquipmentSlot::MainHand),
            };
            weapon
                .ok_or("the weapon's enchantments do not resolve in this generation")?
                .enchantments
                .into_iter()
                .collect()
        }
    };
    let entities = DamageContextEntities {
        victim: request.victim,
        causing: bound(request.causing, causing_view.as_ref()),
        direct: bound(request.direct, direct_view.as_ref()),
        damage_type: request.damage_source.damage_type,
        is_direct: request.damage_source.is_direct,
    };
    body(table, &enchantments, &mut |id, level, condition| {
        damage_condition_matches(context, random, &entities, id, level, condition)
    })
}

fn bound<'a>(
    attacker: MobLootAttacker<'a>,
    session: Option<&'a crate::loot_runtime::LootEntityFacts>,
) -> LootEntity<'a> {
    match session {
        Some(facts) => LootEntity::Facts(facts),
        None => attacker.to_loot_entity(),
    }
}
