//! `Enchantment.damageContext` (`Enchantment.java:463-473`): the
//! `ENCHANTED_DAMAGE` loot context every damage-filtered enchantment effect
//! tests its requirements in.

use crate::connection::ConnectionId;
use crate::enchantment_combat_effects::{CombatValue, EnchantmentCombatTable};
use crate::loot_runtime::{
    matches_conditions, param_sets, LootBlockStateAt, LootDamageSource, LootDamageSourceFacts,
    LootEntities, LootEntity, LootOrigin, LootParams, LootWeather, StoredLootEntity,
};
use crate::random::GameRandom;
use crate::world::LevelId;
use bbb_server_datapack::loot::{LootCondition, LootRegistries};
use bbb_server_datapack::tag_loader::LoadedRegistryTags;

use super::player_melee::MeleeTarget;
use crate::world::{LoadedBlocks, ServerWorld};

/// An entity a damage source names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::world) enum SourceEntity {
    Session(ConnectionId),
    Stored(i32),
}

/// The `DamageSource` half of the context.
pub(super) struct EnchantedDamageSource<'a> {
    pub(super) damage_type: &'a str,
    pub(super) is_direct: bool,
    pub(super) causing: Option<SourceEntity>,
    pub(super) direct: Option<SourceEntity>,
}

impl ServerWorld {
    /// `value`'s effects of each `(enchantment, level)` in order
    /// (`Enchantment.applyEffects`, `Enchantment.java:514-526`).
    pub(super) fn enchanted_damage_value(
        &mut self,
        dimension: &LevelId,
        enchantments: &[(String, i32)],
        victim: MeleeTarget,
        source: &EnchantedDamageSource<'_>,
        value: CombatValue,
        amount: f32,
    ) -> Result<f32, String> {
        let table = std::sync::Arc::clone(&self.datapack_tables.enchantment_combat_effects);
        if !enchantments.iter().any(|(id, _)| table.contains_key(id)) {
            return Ok(amount);
        }
        self.with_damage_context(dimension, victim, source, |random, matches| {
            enchantments_value(&table, enchantments, value, amount, random, matches)
        })
    }

    /// `EnchantmentHelper.isImmuneToDamage`'s test of each
    /// `(enchantment, level)` (`EnchantmentHelper.java:158-164`).
    pub(super) fn enchanted_damage_immunity(
        &mut self,
        dimension: &LevelId,
        enchantments: &[(String, i32)],
        victim: MeleeTarget,
        source: &EnchantedDamageSource<'_>,
    ) -> Result<bool, String> {
        let table = std::sync::Arc::clone(&self.datapack_tables.enchantment_combat_effects);
        if !declares_immunity(&table, enchantments) {
            return Ok(false);
        }
        self.with_damage_context(dimension, victim, source, |_random, matches| {
            enchantments_immune(&table, enchantments, matches)
        })
    }

    /// `TargetedConditionalEffect.matches` (`TargetedConditionalEffect.java:41-43`)
    /// for one entry of `(enchantment, level)`.
    pub(super) fn enchanted_damage_condition(
        &mut self,
        dimension: &LevelId,
        enchantment_id: &str,
        level: i32,
        victim: MeleeTarget,
        source: &EnchantedDamageSource<'_>,
        condition: &LootCondition,
    ) -> Result<bool, String> {
        self.with_damage_context(dimension, victim, source, |_random, matches| {
            matches(enchantment_id, level, condition)
        })
    }

    fn with_damage_context<R>(
        &mut self,
        dimension: &LevelId,
        victim: MeleeTarget,
        source: &EnchantedDamageSource<'_>,
        body: impl FnOnce(
            &GameRandom,
            &mut dyn FnMut(&str, i32, &LootCondition) -> Result<bool, String>,
        ) -> Result<R, String>,
    ) -> Result<R, String> {
        let view = |entity: Option<SourceEntity>| match entity {
            Some(SourceEntity::Session(id)) => self.session_loot_view(id),
            _ => None,
        };
        let causing_view = view(source.causing);
        let direct_view = view(source.direct);
        let session_victim = match victim {
            MeleeTarget::Session(id) => view(Some(SourceEntity::Session(id))),
            MeleeTarget::Stored(_) => None,
        };
        let datapacks = self.cold.config.datapacks.clone();
        let damage_type_tags = datapacks.as_deref().map(|packs| {
            packs
                .tags
                .registry(bbb_server_datapack::damage::DAMAGE_TYPE_REGISTRY)
        });
        let loot_tables = datapacks.as_deref().map(|packs| &packs.loot);
        self.with_entity_reads(|reads| {
            let level = reads
                .levels
                .iter()
                .find(|level| reads.names.id(level.key) == Some(*dimension))
                .ok_or_else(|| "the level is not loaded".to_string())?;
            let entities = level.entities;
            let raids = level.raids;
            let world_random = level.random;
            let player_slots = |uuid, slot| reads.player_item_slot(uuid, slot);
            let player_nbt = |uuid| reads.player_loot_nbt(uuid);
            let item_environment = reads
                .predicate_environment(level.key, &player_slots, &player_nbt)
                .ok_or_else(|| "the level is not loaded".to_string())?;
            let vehicle_slots = |requested: &crate::entity::Entity, selected: &[i32]| {
                reads.loot_vehicle_slots(requested, selected)
            };
            let context = DamageContextLevel {
                vehicle_slots: Some(&vehicle_slots),
                dimension: Some(level.key.as_str()),
                item_environment: Some(&item_environment),
                loot_tables,
                damage_type_tags,
                blocks: &|pos| {
                    let chunks = reads
                        .blocks(level.key)
                        .expect("the context level has chunks");
                    LoadedBlocks { chunks: &chunks }.loaded_block_state(pos)
                },
                block_entity_data: &|pos| {
                    reads
                        .blocks(level.key)
                        .expect("the context level has chunks")
                        .block_entity_loot_data(pos, Some(entities.item_resolver()))
                        .map_err(|error| {
                            crate::loot_runtime::LootRuntimeReason::Catalog(error.to_string())
                        })
                },
                sky: &|pos| {
                    let chunks = reads
                        .blocks(level.key)
                        .expect("the context level has chunks");
                    LoadedBlocks { chunks: &chunks }.loaded_can_see_sky(pos)
                },
                weather: level.weather,
            };
            let stored = |entity: Option<SourceEntity>| match entity {
                Some(SourceEntity::Stored(id)) => reads.levels.iter().find_map(|level| {
                    level.entities.get(id).map(|entity| {
                        LootEntity::Stored(StoredLootEntity {
                            entity,
                            bodies: Some(crate::entity::store::EntityBodyReadView::new(
                                level.entities,
                                None,
                                None,
                            )),
                            raid_at_position: Some(level.raids.raid_at(entity.block_position())),
                        })
                    })
                }),
                _ => None,
            };
            let causing = stored(source.causing)
                .unwrap_or_else(|| loot_entity(source.causing, causing_view.as_ref()));
            let direct = stored(source.direct)
                .unwrap_or_else(|| loot_entity(source.direct, direct_view.as_ref()));
            let mut body = Some(body);
            let mut evaluate = |victim: LootEntity<'_>| {
                let entities = DamageContextEntities {
                    victim,
                    causing,
                    direct,
                    damage_type: source.damage_type,
                    is_direct: source.is_direct,
                };
                let mut matches = |id: &str, level: i32, condition: &LootCondition| {
                    damage_condition_matches(
                        &context,
                        world_random,
                        &entities,
                        id,
                        level,
                        condition,
                    )
                };
                (body.take().expect("the context is evaluated once"))(world_random, &mut matches)
            };
            match (victim, &session_victim) {
                (_, Some(view)) => evaluate(LootEntity::Facts(view)),
                (MeleeTarget::Stored(victim_id), None) => match entities
                    .get(victim_id)
                    .or_else(|| crate::entity::ender_dragon::part_entity(entities, victim_id))
                {
                    Some(entity) => evaluate(LootEntity::Stored(StoredLootEntity {
                        entity,
                        bodies: Some(crate::entity::store::EntityBodyReadView::new(
                            entities, None, None,
                        )),
                        raid_at_position: Some(raids.raid_at(entity.block_position())),
                    })),
                    None => Err("the victim is no longer stored".to_string()),
                },
                (MeleeTarget::Session(_), None) => unreachable!("a session victim has a view"),
            }
        })
    }
}

/// The facts of `Enchantment.damageContext` that name no entity.
pub(in crate::world) struct DamageContextLevel<'a> {
    pub(in crate::world) vehicle_slots:
        Option<&'a crate::loot_runtime::params::LootVehicleSlots<'a>>,
    pub(in crate::world) item_environment: Option<&'a dyn crate::loot_runtime::LootItemEnvironment>,
    pub(in crate::world) loot_tables: Option<&'a LootRegistries>,
    pub(in crate::world) damage_type_tags: Option<Option<&'a LoadedRegistryTags>>,
    pub(in crate::world) dimension: Option<&'a str>,
    pub(in crate::world) blocks: &'a LootBlockStateAt<'a>,
    pub(in crate::world) block_entity_data: &'a crate::loot_runtime::LootBlockEntityDataAt<'a>,
    pub(in crate::world) sky: &'a crate::loot_runtime::LootSkyAt<'a>,
    pub(in crate::world) weather: LootWeather,
}

/// `THIS_ENTITY`, both attackers and the source of one `damageContext`.
pub(in crate::world) struct DamageContextEntities<'a> {
    pub(in crate::world) victim: LootEntity<'a>,
    pub(in crate::world) causing: LootEntity<'a>,
    pub(in crate::world) direct: LootEntity<'a>,
    pub(in crate::world) damage_type: &'a str,
    pub(in crate::world) is_direct: bool,
}

/// One requirement under `Enchantment.damageContext` (`Enchantment.java:463-473`).
pub(in crate::world) fn damage_condition_matches(
    context: &DamageContextLevel<'_>,
    random: &GameRandom,
    entities: &DamageContextEntities<'_>,
    enchantment_id: &str,
    enchantment_level: i32,
    condition: &LootCondition,
) -> Result<bool, String> {
    let damage_source = LootDamageSourceFacts {
        damage_type: entities.damage_type,
        is_direct: Some(entities.is_direct),
        tags: context.damage_type_tags,
        causing: entities.causing,
        direct: Some(entities.direct),
    };
    let origin = entities
        .victim
        .position()
        .ok_or("victim position is unavailable")?;
    let params = LootParams {
        origin: Some(LootOrigin {
            x: origin.x,
            y: origin.y,
            z: origin.z,
        }),
        entities: LootEntities {
            this_entity: entities.victim,
            attacking_entity: entities.causing,
            direct_attacking_entity: entities.direct,
            ..LootEntities::default()
        },
        damage_source: LootDamageSource::Present(damage_source),
        enchantment_level: Some(enchantment_level),
        item_environment: context.item_environment,
        vehicle_slots: context.vehicle_slots,
        dimension: context.dimension,
        level: Some(context.blocks),
        block_entity_data: Some(context.block_entity_data),
        sky: Some(context.sky),
        weather: Some(context.weather),
        ..LootParams::new(param_sets::ENCHANTED_DAMAGE)
    };
    matches_conditions(
        context.loot_tables.ok_or("no loot registries are loaded")?,
        enchantment_id,
        std::slice::from_ref(condition),
        &params,
        random,
    )
    .map_err(|error| error.to_string())
}

/// `value`'s effects of each `(enchantment, level)` in order
/// (`Enchantment.applyEffects`, `Enchantment.java:514-526`).
pub(in crate::world) fn enchantments_value(
    table: &EnchantmentCombatTable,
    enchantments: &[(String, i32)],
    value: CombatValue,
    mut amount: f32,
    random: &GameRandom,
    matches: &mut dyn FnMut(&str, i32, &LootCondition) -> Result<bool, String>,
) -> Result<f32, String> {
    for (id, level) in enchantments {
        if let Some(effects) = table.get(id) {
            amount = effects.apply(value, *level, amount, random, |condition| {
                matches(id, *level, condition)
            })?;
        }
    }
    Ok(amount)
}

pub(in crate::world) fn declares_immunity(
    table: &EnchantmentCombatTable,
    enchantments: &[(String, i32)],
) -> bool {
    enchantments.iter().any(|(id, _)| {
        table
            .get(id)
            .is_some_and(|effects| effects.declares_immunity())
    })
}

/// `EnchantmentHelper.isImmuneToDamage`'s test of each `(enchantment, level)`
/// (`EnchantmentHelper.java:158-164`).
pub(in crate::world) fn enchantments_immune(
    table: &EnchantmentCombatTable,
    enchantments: &[(String, i32)],
    matches: &mut dyn FnMut(&str, i32, &LootCondition) -> Result<bool, String>,
) -> Result<bool, String> {
    for (id, level) in enchantments {
        let Some(effects) = table.get(id).filter(|effects| effects.declares_immunity()) else {
            continue;
        };
        if effects.immune(|condition| matches(id, *level, condition))? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn loot_entity<'a>(
    entity: Option<SourceEntity>,
    view: Option<&'a crate::loot_runtime::LootEntityFacts>,
) -> LootEntity<'a> {
    match (entity, view) {
        (_, Some(view)) => LootEntity::Facts(view),
        (Some(_), None) => LootEntity::Opaque,
        (None, None) => LootEntity::Absent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityData, GeneralLivingData};
    use crate::tests::{join_loaded, mailbox, operator_config};
    use crate::world::ServerWorldConfig;
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;
    use bbb_protocol::game_types::GameType;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
    use bbb_server_datapack::vanilla_snapshot::VanillaDatapackSnapshot;
    use bbb_server_types::save::Java26BuiltinDimension;
    use serde_json::json;
    use std::sync::Arc;

    #[test]
    fn combat_enchantment_remove_binomial_evaluates_with_world_random() {
        let temp = std::env::temp_dir().join(format!(
            "bbb-combat-binomial-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let pack = temp.join("pack");
        let ench_dir = pack.join("data/test/enchantment");
        std::fs::create_dir_all(&ench_dir).unwrap();
        std::fs::write(
            pack.join("pack.mcmeta"),
            br#"{"pack":{"description":"test","min_format":101,"max_format":101}}"#,
        )
        .unwrap();
        let snapshot = VanillaDatapackSnapshot::embedded_shared().unwrap();
        let mut definition: serde_json::Value = serde_json::from_slice(
            snapshot
                .read_path("enchantment/sharpness.json")
                .expect("built-in sharpness"),
        )
        .unwrap();
        definition["effects"] = json!({
            "minecraft:damage": [
                {
                    "effect": {
                        "type": "minecraft:remove_binomial",
                        "chance": 0.5
                    }
                }
            ],
            "minecraft:damage_protection": [
                {
                    "effect": {
                        "type": "minecraft:remove_binomial",
                        "chance": 1.0
                    }
                }
            ]
        });
        std::fs::write(
            ench_dir.join("binomial_combat.json"),
            serde_json::to_vec(&definition).unwrap(),
        )
        .unwrap();

        let startup = Arc::new(
            DatapackStartup::load(&DatapackStartupOptions {
                datapacks_dir: Some(temp.clone()),
                saved_data_packs: None,
                safe_mode: false,
                limits: Default::default(),
            })
            .expect("custom binomial combat enchantment loads"),
        );
        std::fs::remove_dir_all(&temp).unwrap();

        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(ServerWorldConfig {
            game_type: GameType::Survival,
            datapacks: Some(startup),
            ..operator_config(&[("alice", 1)])
        })
        .unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        world.tick(receiver.drain());

        let dimension = LevelId::from(Java26BuiltinDimension::Overworld);
        let source = EnchantedDamageSource {
            damage_type: "minecraft:player_attack",
            is_direct: true,
            causing: Some(SourceEntity::Session(ConnectionId(1))),
            direct: Some(SourceEntity::Session(ConnectionId(1))),
        };
        let enchantments = vec![("test:binomial_combat".to_string(), 1)];

        let reduced_damage = world
            .enchanted_damage_value(
                &dimension,
                &enchantments,
                MeleeTarget::Session(ConnectionId(1)),
                &source,
                CombatValue::Damage,
                20.0,
            )
            .expect("damage remove_binomial evaluates cleanly");
        assert!(
            (0.0..=20.0).contains(&reduced_damage) && reduced_damage.fract() == 0.0,
            "unexpected binomial damage result: {reduced_damage}"
        );

        let pos = world.publication().players[0].position;
        let (zombie_id, _) = world.spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            pos,
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap(),
            ),
        );
        let reduced_protection = world
            .enchanted_damage_value(
                &dimension,
                &enchantments,
                MeleeTarget::Stored(zombie_id),
                &source,
                CombatValue::DamageProtection,
                8.0,
            )
            .expect("damage_protection remove_binomial evaluates cleanly");
        assert_eq!(reduced_protection, 0.0);
    }
}
