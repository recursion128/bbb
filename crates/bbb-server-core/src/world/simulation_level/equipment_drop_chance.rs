//! `EnchantmentHelper.processEquipmentDropChance` on a world-owned generation.

use crate::entity::block_drops::BlockDropTables;
use crate::entity::mob_death_loot::{EquipmentDropChanceRequest, MobLootAttacker};
use crate::equipment_drop_effects::EquipmentDropTarget;
use crate::loot_runtime::{
    matches_conditions, param_sets, LootDamageSource, LootDamageSourceFacts, LootEntities,
    LootEntity, LootOrigin, LootParams, LootWeather,
};
use crate::random::GameRandom;
use bbb_protocol::equipment_slots::VANILLA_EQUIPMENT_SLOTS;

use super::LoadedBlocks;
use crate::world::loot_context::LootLevelFacts;

/// `EnchantmentHelper.processEquipmentDropChance`
/// (`EnchantmentHelper.java:344-384`): the explicit subject's enchantments, then a
/// living attacker's, under `Enchantment.damageContext` on the subject.
pub(super) fn equipment_drop_chance(
    tables: &mut BlockDropTables<'_>,
    facts: &LootLevelFacts<'_>,
    level_random: &GameRandom,
    blocks: &LoadedBlocks<'_>,
    weather: LootWeather,
    request: EquipmentDropChanceRequest<'_>,
) -> Result<f32, String> {
    let registry = facts
        .enchantment_registry
        .as_ref()
        .ok_or("equipment drop enchantment registry is unavailable")?;
    let effects = facts
        .equipment_drop_effects
        .as_ref()
        .ok_or("equipment drop effects are unavailable")?;
    let tags = blocks.chunks.registry_tags_shared().map_or(
        crate::entity::EntityRegistryTagView::Builtin26_1,
        crate::entity::EntityRegistryTagView::Frozen,
    );
    let session_view = |uuid| {
        facts
            .session_loot_view(Some(request.bodies), &tags, uuid)?
            .ok_or_else(|| format!("attacker player {uuid} has no session"))
    };
    let attacker_view = match request.attacker {
        MobLootAttacker::SessionPlayer { uuid } => Some(session_view(uuid)?),
        _ => None,
    };
    let direct_view = match request.direct_attacking_entity {
        MobLootAttacker::SessionPlayer { uuid } if attacker_view.is_none() => {
            Some(session_view(uuid)?)
        }
        _ => None,
    };
    let attacker = attacker_view
        .as_ref()
        .map_or(request.attacker.to_loot_entity(), LootEntity::Facts);
    let direct = match request.direct_attacking_entity {
        MobLootAttacker::SessionPlayer { .. } => {
            direct_view.as_ref().map_or(attacker, LootEntity::Facts)
        }
        other => other.to_loot_entity(),
    };
    let damage_source = LootDamageSourceFacts {
        damage_type: request.damage_source.damage_type,
        is_direct: Some(request.damage_source.is_direct),
        tags: tables.damage_type_tags,
        causing: attacker,
        direct: Some(direct),
    };
    let subject_view = match request.subject {
        MobLootAttacker::SessionPlayer { uuid } if !matches!(request.attacker, MobLootAttacker::SessionPlayer { uuid: attacker } if attacker == uuid) => {
            Some(session_view(uuid)?)
        }
        _ => None,
    };
    let subject = match request.subject {
        MobLootAttacker::SessionPlayer { .. } => LootEntity::Facts(
            subject_view
                .as_ref()
                .or(attacker_view.as_ref())
                .ok_or("equipment drop subject player has no session")?,
        ),
        other => other.to_loot_entity(),
    };
    super::block_entity_loot::with_predicate_loot_context(
        blocks.chunks,
        facts,
        tables.tables,
        weather,
        level_random,
        Some(request.bodies),
        true,
        None,
        "loot registries are unavailable",
        |loot_refs| {
            let mut chance = request.original_chance;
            let passes = [
                (EquipmentDropTarget::Victim, Some(subject)),
                (
                    EquipmentDropTarget::Attacker,
                    request.attacker_living.then_some(attacker),
                ),
            ];
            for (target, owner) in passes {
                let Some(owner) = owner else {
                    continue;
                };
                for entry in VANILLA_EQUIPMENT_SLOTS {
                    let slot = entry.slot;
                    let Some(tool) = owner.equipment_item(slot) else {
                        continue;
                    };
                    for (id, level) in &tool.enchantments {
                        let index =
                            registry
                                .iter()
                                .position(|entry| entry == id)
                                .ok_or_else(|| {
                                    format!("enchantment {id} is absent from this generation")
                                })?;
                        let Some(effect) = effects
                            .get(index)
                            .ok_or("equipment effects and registry are out of sync")?
                        else {
                            continue;
                        };
                        if !effect.visits(slot, target) {
                            continue;
                        }
                        chance = effect.apply(
                            slot,
                            target,
                            *level,
                            chance,
                            level_random,
                            |condition| {
                                let position = subject
                                    .position()
                                    .ok_or("subject position is unavailable")?;
                                let registries =
                                    tables.tables.ok_or("loot registries are unavailable")?;
                                let params = LootParams {
                                    weather: Some(weather),
                                    dimension: loot_refs.dimension,
                                    item_environment: Some(loot_refs.item_environment),
                                    origin: Some(LootOrigin {
                                        x: position.x,
                                        y: position.y,
                                        z: position.z,
                                    }),
                                    entities: LootEntities {
                                        this_entity: subject,
                                        attacking_entity: attacker,
                                        direct_attacking_entity: direct,
                                        ..LootEntities::default()
                                    },
                                    damage_source: LootDamageSource::Present(damage_source),
                                    enchantment_level: Some(*level),
                                    level: Some(loot_refs.block_at),
                                    vehicle_slots: Some(loot_refs.vehicle_slots),
                                    block_entity_data: Some(loot_refs.block_entity_data),
                                    ..LootParams::new(param_sets::ENCHANTED_DAMAGE)
                                };
                                matches_conditions(
                                    registries,
                                    id,
                                    std::slice::from_ref(condition),
                                    &params,
                                    level_random,
                                )
                                .map_err(|error| error.to_string())
                            },
                        )?;
                    }
                }
            }
            Ok(chance)
        },
    )
}

#[cfg(test)]
impl crate::world::ServerWorld {}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use crate::equipment_drop_effects::EquipmentDropEffects;
    use crate::loot_runtime::{LootEntityPosition, LootTool};
    use bbb_protocol::equipment_slots::EquipmentSlot;

    use super::*;
    use crate::world::{Java26BuiltinDimension, LevelId, ServerWorld};

    fn effect(document: serde_json::Value) -> EquipmentDropEffects {
        EquipmentDropEffects::parse(
            &bbb_registry_codec::parse_registry_json(&serde_json::to_vec(&document).unwrap())
                .unwrap(),
            None,
        )
        .unwrap()
        .unwrap()
    }

    /// Victim and attacker hold the same enchanted sword here.
    #[test]
    fn the_victim_pass_removes_then_the_attacker_pass_adds() {
        let mut world = ServerWorld::new(crate::tests::datapack_config()).unwrap();
        let dimension = LevelId::from(Java26BuiltinDimension::Overworld);
        let (level, mut tables, mut facts, _structures) = world.level_and_drop_tables(&dimension);
        let registry = facts.enchantment_registry.as_ref().unwrap();
        let id = registry
            .iter()
            .position(|id| id == "minecraft:looting")
            .unwrap();
        let mut effects = vec![None; registry.len()];
        effects[id] = Some(effect(serde_json::json!({
            "slots": ["mainhand"],
            "effects": {"minecraft:equipment_drops": [
                {"enchanted": "victim", "effect": {"type": "minecraft:remove_binomial",
                    "chance": {"type": "minecraft:fraction", "numerator": 1.0, "denominator": 2.0}},
                    "requirements": {"condition": "minecraft:random_chance", "chance": 1.0}},
                {"enchanted": "attacker", "effect": {"type": "minecraft:add", "value": 1.0}}
            ]}
        })));
        facts.equipment_drop_effects = Some(Arc::new(effects));
        let attacker = crate::loot_runtime::LootEntityFacts {
            living: Some(true),
            position: Some(LootEntityPosition {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            }),
            equipment: Some(BTreeMap::from([(
                EquipmentSlot::MainHand,
                LootTool {
                    stack: Default::default(),
                    item_id: Some(1),
                    count: 1,
                    enchantments: BTreeMap::from([("minecraft:looting".into(), 1)]),
                },
            )])),
            ..Default::default()
        };
        let blocks = LoadedBlocks {
            chunks: &level.chunks,
        };
        let chance = equipment_drop_chance(
            &mut tables,
            &facts,
            &level.random,
            &blocks,
            LootWeather {
                raining: level.weather.is_raining(),
                thundering: level.weather.is_thundering(),
            },
            EquipmentDropChanceRequest {
                bodies: crate::entity::store::EntityBodyReadView::new(&level.entities, None, None),
                subject: MobLootAttacker::Entity(LootEntity::Facts(&attacker)),
                original_chance: 3.0,
                attacker_living: true,
                attacker: MobLootAttacker::Entity(LootEntity::Facts(&attacker)),
                direct_attacking_entity: MobLootAttacker::Entity(LootEntity::Facts(&attacker)),
                damage_source: crate::entity::mob_death_loot::MobLootDamageSource {
                    damage_type: "minecraft:player_attack",
                    is_direct: true,
                },
            },
        )
        .unwrap();
        // 3 minus a binomial(3, 1/2) removal, then plus one.
        assert!(
            (1.0..=4.0).contains(&chance) && chance.fract() == 0.0,
            "{chance}"
        );
    }

    /// `all_of` runs its effects in order and `exponential` scales by
    /// `base^exponent`: 3 * 2^2 + 1.
    #[test]
    fn all_of_and_exponential_shape_the_victim_pass() {
        let mut world = ServerWorld::new(crate::tests::datapack_config()).unwrap();
        let dimension = LevelId::from(Java26BuiltinDimension::Overworld);
        let (level, mut tables, mut facts, _structures) = world.level_and_drop_tables(&dimension);
        let registry = facts.enchantment_registry.as_ref().unwrap();
        let id = registry
            .iter()
            .position(|id| id == "minecraft:looting")
            .unwrap();
        let mut effects = vec![None; registry.len()];
        effects[id] = Some(effect(serde_json::json!({
            "slots": ["mainhand"],
            "effects": {"minecraft:equipment_drops": [
                {"enchanted": "victim", "effect": {"type": "minecraft:all_of", "effects": [
                    {"type": "minecraft:exponential", "base": 2.0, "exponent": 2.0},
                    {"type": "minecraft:add", "value": 1.0}
                ]}}
            ]}
        })));
        facts.equipment_drop_effects = Some(Arc::new(effects));
        let victim = crate::loot_runtime::LootEntityFacts {
            living: Some(true),
            position: Some(LootEntityPosition {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            }),
            equipment: Some(BTreeMap::from([(
                EquipmentSlot::MainHand,
                LootTool {
                    stack: Default::default(),
                    item_id: Some(1),
                    count: 1,
                    enchantments: BTreeMap::from([("minecraft:looting".into(), 1)]),
                },
            )])),
            ..Default::default()
        };
        let blocks = LoadedBlocks {
            chunks: &level.chunks,
        };
        let chance = equipment_drop_chance(
            &mut tables,
            &facts,
            &level.random,
            &blocks,
            LootWeather {
                raining: level.weather.is_raining(),
                thundering: level.weather.is_thundering(),
            },
            EquipmentDropChanceRequest {
                bodies: crate::entity::store::EntityBodyReadView::new(&level.entities, None, None),
                subject: MobLootAttacker::Entity(LootEntity::Facts(&victim)),
                original_chance: 3.0,
                attacker: MobLootAttacker::Absent,
                attacker_living: false,
                direct_attacking_entity: MobLootAttacker::Absent,
                damage_source: crate::entity::mob_death_loot::MobLootDamageSource {
                    damage_type: "minecraft:generic",
                    is_direct: true,
                },
            },
        )
        .unwrap();
        assert_eq!(chance, 13.0);
    }
}
