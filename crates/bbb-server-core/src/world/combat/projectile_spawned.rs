//! Birth effects evaluated against the actual projectile and its carried weapon.

use bbb_protocol::packets::ItemStackSummary;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::post_attack::{EnchantedItem, EnchantedItemSource};
use super::SourceEntity;
use crate::enchantment_weapon_effects::projectile_spawned_conditions;
use crate::world::{LevelId, ServerWorld, WorldEvent};

impl ServerWorld {
    // Projectile.java:225-242; EnchantmentHelper.java:316-321.
    pub(in crate::world) fn projectile_spawned_effects(
        &mut self,
        dimension: &LevelId,
        entity_id: i32,
        item: RuntimeItemStack,
        events: &mut Vec<WorldEvent>,
    ) -> RuntimeItemStack {
        let owner = self
            .level_by_id(dimension)
            .entities
            .get(entity_id)
            .and_then(|body| body.data().projectile_owner())
            .and_then(|uuid| {
                self.connection_of(uuid)
                    .filter(|id| self.player_level_id(*id) == *dimension)
                    .map(SourceEntity::Session)
                    .or_else(|| {
                        let store = &self.level_by_id(dimension).entities;
                        store
                            .entity_id_for_uuid(uuid)
                            .filter(|id| store.get(*id).is_some_and(|body| body.living().is_some()))
                            .map(SourceEntity::Stored)
                    })
            });
        let input_item = item.item_id();
        let summary = item.summary().clone();
        let mut input = EnchantedItem::detached(owner, item);
        self.run_projectile_spawned_item(dimension, entity_id, &summary, &mut input, events);
        let weapon = self
            .level_by_id(dimension)
            .entities
            .get(entity_id)
            .and_then(|body| crate::entity::abstract_arrow::state(body.data()))
            .and_then(|state| state.weapon())
            .filter(|weapon| !weapon.is_empty() && weapon.item_id() != input_item)
            .map(|weapon| weapon.summary().clone());
        if let Some(weapon) = weapon {
            self.run_projectile_spawned_item(
                dimension,
                entity_id,
                &weapon,
                &mut EnchantedItem::projectile_weapon(owner, entity_id),
                events,
            );
        }
        let EnchantedItemSource::Detached(item) = input.source else {
            unreachable!()
        };
        item
    }

    fn run_projectile_spawned_item(
        &mut self,
        dimension: &LevelId,
        entity_id: i32,
        stack: &ItemStackSummary,
        item: &mut EnchantedItem,
        events: &mut Vec<WorldEvent>,
    ) {
        let table = self.level_by_id(dimension).entities.weapon_effects_shared();
        for (level, conditional) in projectile_spawned_conditions(&table, stack) {
            let Some(position) = self
                .level_by_id(dimension)
                .entities
                .get(entity_id)
                .filter(|body| body.removal_reason().is_none())
                .map(|body| body.position())
            else {
                break;
            };
            let target = SourceEntity::Stored(entity_id);
            let matched = conditional
                .requirements
                .as_ref()
                .map_or(Ok(true), |condition| {
                    self.enchantment_entity_requirements_match(
                        dimension, target, position, None, level, condition,
                    )
                });
            match matched {
                Ok(true) => {
                    if let Err(detail) = self.apply_entity_effect(
                        dimension,
                        &conditional.effect,
                        level,
                        item,
                        target,
                        position,
                        None,
                        events,
                    ) {
                        tracing::warn!("projectile_spawned: {detail}");
                    }
                }
                Ok(false) => {}
                Err(detail) => tracing::warn!("projectile_spawned requirements: {detail}"),
            }
        }
    }
}
