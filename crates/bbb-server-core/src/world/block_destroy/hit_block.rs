//! `EnchantmentHelper.onHitBlock` (`EnchantmentHelper.java:324-336`) for the
//! stack a player starts to dig with (`ServerPlayerGameMode.java:187-196`) and
//! for the weapon an arrow or trident was loosed from.

use crate::connection::ConnectionId;
use crate::entity::projectile_action::PendingProjectileHitBlock;
use crate::loot_runtime::{
    matches_conditions, param_sets, LootEntities, LootEntity, LootOrigin, LootParams,
    StoredLootEntity,
};
use crate::world::combat::EnchantmentEffectEntity;
use crate::world::LevelId;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_datapack::loot::LootCondition;

use crate::world::{ServerWorld, WorldEvent};

fn centre(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    }
}

impl ServerWorld {
    /// The digging player's main-hand stack, with the player as the entity.
    pub(super) fn on_hit_block(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        let stack = match self.levels[connection_id].inventory.selected_item() {
            Ok(stack) => stack,
            Err(error) => {
                tracing::warn!("the digging stack cannot project for hit_block effects: {error}");
                return;
            }
        };
        if stack.item_id.is_none()
            || stack
                .component_patch
                .removed_type_ids
                .contains(&VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID)
        {
            return;
        }
        let mut enchantments = Vec::new();
        for entry in &stack.component_patch.enchantments {
            let Some(id) = usize::try_from(entry.holder_id)
                .ok()
                .and_then(|index| self.datapack_tables.enchantment_registry.get(index))
            else {
                tracing::warn!("the digging stack carries enchantment holder id {}, which the loaded enchantment registry has no row for, so its hit_block effects are unknown", entry.holder_id);
                return;
            };
            enchantments.push((id.clone(), entry.level));
        }
        let player = EnchantmentEffectEntity::Session(connection_id);
        self.run_hit_block_effects(
            &dimension,
            enchantments,
            crate::world::combat::EnchantedItem::equipment(player, EquipmentSlot::MainHand, None),
            player,
            centre(pos),
            block_state_id,
            events,
        );
    }

    /// `AbstractArrow.hitBlockEnchantmentEffects` (`AbstractArrow.java:550-562`)
    /// and the trident's (`ThrownTrident.java:152-164`): the projectile's weapon
    /// over `EnchantmentHelper.onHitBlock` with the projectile as the entity.
    pub(in crate::world) fn projectile_hit_block_effects(
        &mut self,
        dimension: &LevelId,
        hit: PendingProjectileHitBlock,
        events: &mut Vec<WorldEvent>,
    ) {
        let enchantments = crate::entity::enchanted_damage::stack_enchantments(
            &hit.weapon,
            &self.datapack_tables.enchantment_slots,
        );
        if enchantments.is_empty() {
            return;
        }
        let projectile = EnchantmentEffectEntity::Stored(hit.projectile_entity_id);
        let owner = hit.owner.and_then(|uuid| {
            self.connection_of(uuid)
                .map(EnchantmentEffectEntity::Session)
                .or_else(|| {
                    let entities = &self.level_by_id(dimension).entities;
                    entities
                        .entity_id_for_uuid(uuid)
                        .filter(|id| entities.get(*id).is_some_and(|e| e.living().is_some()))
                        .map(EnchantmentEffectEntity::Stored)
                })
        });
        let item = if self
            .level_by_id(dimension)
            .entities
            .get(hit.projectile_entity_id)
            .and_then(|entity| crate::entity::abstract_arrow::state(entity.data()))
            .is_some_and(|state| state.weapon().is_some())
        {
            crate::world::combat::EnchantedItem::projectile_weapon(owner, hit.projectile_entity_id)
        } else {
            crate::world::combat::EnchantedItem::detached(owner, hit.weapon)
        };
        self.run_hit_block_effects(
            dimension,
            enchantments,
            item,
            projectile,
            hit.position,
            hit.block_state_id,
            events,
        );
    }

    /// `runIterationOnItem` (`EnchantmentHelper.java:126-132`), each enchantment
    /// running `Enchantment.onHitBlock` (`Enchantment.java:400-413`): every
    /// `hit_block` entry whose requirements match `blockHitContext` (`:502-512`)
    /// applies its effect (`Enchantment.applyEffects`, `:514-520`).
    #[allow(clippy::too_many_arguments)]
    fn run_hit_block_effects(
        &mut self,
        dimension: &LevelId,
        enchantments: Vec<(String, i32)>,
        mut item: crate::world::combat::EnchantedItem,
        entity: EnchantmentEffectEntity,
        position: Vec3d,
        block_state_id: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let table = std::sync::Arc::clone(&self.datapack_tables.enchantment_entity_effects);
        for (enchantment_id, level) in enchantments {
            let Some(row) = table.get(&enchantment_id) else {
                continue;
            };
            let effects = match &row.hit_block {
                Ok(effects) => effects,
                Err(detail) => {
                    tracing::warn!("{detail}");
                    continue;
                }
            };
            for effect in effects {
                let matched = match &effect.requirements {
                    None => Ok(true),
                    Some(requirements) => self.enchantment_entity_requirements_match(
                        dimension,
                        entity,
                        position,
                        Some(block_state_id),
                        level,
                        requirements,
                    ),
                };
                match matched {
                    Ok(true) => {
                        if let Err(detail) = self.apply_entity_effect(
                            dimension,
                            &effect.effect,
                            level,
                            &mut item,
                            entity,
                            position,
                            None,
                            events,
                        ) {
                            tracing::warn!("{enchantment_id} hit_block: {detail}");
                        }
                    }
                    Ok(false) => {}
                    Err(detail) => {
                        tracing::warn!("{enchantment_id} hit_block requirements: {detail}")
                    }
                }
            }
        }
    }

    pub(in crate::world) fn enchantment_entity_requirements_match(
        &mut self,
        dimension: &LevelId,
        entity: EnchantmentEffectEntity,
        origin: Vec3d,
        block_state_id: Option<i32>,
        enchantment_level: i32,
        requirements: &LootCondition,
    ) -> Result<bool, String> {
        if self.cold.config.datapacks.is_none() {
            return Err("no loot registries are loaded".to_string());
        }
        let view = match entity {
            EnchantmentEffectEntity::Session(id) => {
                Some(self.loot_view_for_player(self.levels.player_at(id)))
            }
            EnchantmentEffectEntity::Stored(_) => None,
        };
        let param_set = if block_state_id.is_some() {
            param_sets::HIT_BLOCK
        } else {
            param_sets::ENCHANTED_ENTITY
        };
        self.with_entity_predicate_level(
            *dimension,
            param_set,
            move |bodies, tables, random, base| {
                let this_entity = match entity {
                    EnchantmentEffectEntity::Session(_) => {
                        LootEntity::Facts(view.as_ref().ok_or("the hitting session left")?)
                    }
                    EnchantmentEffectEntity::Stored(id) => LootEntity::Stored(StoredLootEntity {
                        entity: bodies.get(id).ok_or("the hitting entity left the level")?,
                        bodies: Some(bodies),
                        raid_at_position: None,
                    }),
                };
                let params = LootParams {
                    origin: Some(LootOrigin {
                        x: origin.x,
                        y: origin.y,
                        z: origin.z,
                    }),
                    block_state_id,
                    enchantment_level: Some(enchantment_level),
                    entities: LootEntities {
                        this_entity,
                        ..LootEntities::default()
                    },
                    ..base
                };
                matches_conditions(
                    tables.ok_or("no loot registries are loaded")?,
                    if block_state_id.is_some() {
                        "minecraft:hit_block"
                    } else {
                        "minecraft:projectile_spawned"
                    },
                    std::slice::from_ref(requirements),
                    &params,
                    random,
                )
                .map_err(|error| error.to_string())
            },
        )
    }
}
