use super::*;
use crate::entity::skeleton_horse::{activate_trap, has_trigger_player, is_trap, tame_and_grow};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;
use uuid::Uuid;

impl ServerWorld {
    // SkeletonTrapGoal.java:32-59.
    pub(in crate::world) fn activate_skeleton_horse_trap(
        &mut self,
        dimension: &LevelId,
        entity_id: i32,
        uuid: Uuid,
    ) {
        let position = self.with_level_and_entities(*dimension, |level, store| {
            let horse = store.get(entity_id)?;
            if horse.uuid() != uuid
                || horse.entity_type_id() != VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
                || !horse.is_alive_with_health()
                || !is_trap(horse)
                || !has_trigger_player(horse.position(), level.players())
            {
                return None;
            }
            let position = horse.position();
            activate_trap(store.get_mut(entity_id)?).then_some(position)
        });
        let Some(position) = position else {
            return;
        };
        let pairs = self.with_spawn_runtime(dimension, &[], |runtime, random, _| {
            crate::entity::lightning_bolt::spawn_lightning_bolt(
                runtime.entities,
                position,
                true,
                None,
            );
            let mut pairs = Vec::with_capacity(4);
            if let Some(rider) = create_rider(runtime, position, random) {
                pairs.push((entity_id, runtime.entities.add_fresh(rider)));
            }
            for _ in 0..3 {
                let Some(mut horse) =
                    create_trap_mob(runtime, "minecraft:skeleton_horse", position, random)
                else {
                    continue;
                };
                if !tame_and_grow(&mut horse) {
                    runtime.entities.discard_fresh(horse);
                    continue;
                }
                let Some(rider) = create_rider(runtime, position, random) else {
                    runtime.entities.discard_fresh(horse);
                    continue;
                };
                horse.set_delta_movement(Vec3d {
                    x: random.triangle(0.0, 1.1485),
                    y: 0.0,
                    z: random.triangle(0.0, 1.1485),
                });
                let horse_id = runtime.entities.add_fresh(horse);
                let rider_id = runtime.entities.add_fresh(rider);
                pairs.push((horse_id, rider_id));
            }
            pairs
        });
        if let Some(pairs) = pairs {
            self.with_level_and_entities(*dimension, |level, store| {
                for (horse_id, rider_id) in pairs {
                    store.with_entity_mut(rider_id, |body, store| {
                        crate::entity::passenger::start_riding_detached(
                            body, store, level, horse_id,
                        );
                    });
                }
            });
        }
    }
}

// SkeletonTrapGoal.java:63-83.
fn create_trap_mob(
    runtime: &mut MinecartSpawnerRuntime<'_>,
    resource_id: &str,
    position: Vec3d,
    random: &GameRandom,
) -> Option<Box<Entity>> {
    let mut entity = crate::entity::create_fresh_entity(runtime.entities, resource_id, position)?;
    if finalize_spawn(runtime, &mut entity, SpawnReason::Triggered, None, random).is_none() {
        runtime.entities.discard_fresh(entity);
        return None;
    }
    crate::entity::conversion::set_persistence_required(&mut entity);
    if let Some(living) = entity.living_mut() {
        living.invulnerable_time = 60;
    }
    Some(entity)
}

// SkeletonTrapGoal.java:77-101.
fn create_rider(
    runtime: &mut MinecartSpawnerRuntime<'_>,
    position: Vec3d,
    random: &GameRandom,
) -> Option<Box<Entity>> {
    let mut rider = create_trap_mob(runtime, "minecraft:skeleton", position, random)?;
    let equipped = (|| {
        let special_multiplier = runtime
            .spawn_context("minecraft:skeleton", rider.block_position())?
            .special_multiplier;
        if rider.living()?.item_by_slot(EquipmentSlot::Head).is_none() {
            let helmet = crate::entity::item::plain_stack("minecraft:iron_helmet", 1)?;
            rider
                .living_mut()?
                .set_item_slot(EquipmentSlot::Head, Some(helmet));
        }
        for slot in [EquipmentSlot::MainHand, EquipmentSlot::Head] {
            let stack = rider.living()?.item_by_slot(slot)?.clone();
            let cleared = stack
                .with_enchantments(Vec::new(), runtime.item_holders())
                .ok()?;
            let stack = runtime
                .enchant(
                    "minecraft:mob_spawn_equipment",
                    &cleared,
                    special_multiplier,
                    random,
                )
                .unwrap_or(cleared);
            rider.living_mut()?.set_item_slot(slot, Some(stack));
        }
        Some(())
    })();
    if equipped.is_none() {
        runtime.entities.discard_fresh(rider);
        return None;
    }
    Some(rider)
}
