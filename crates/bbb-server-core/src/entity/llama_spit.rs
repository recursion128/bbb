//! `LlamaSpit`: a `Projectile` that hurts what it hits for its living owner
//! and discards on block or water contact.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};

use crate::entity::hurting_projectile;
use crate::entity::level::fluid_state_tag_membership;
use crate::entity::living::SynchedValue;
use crate::entity::projectile_hit::{hit_result_on_move_vector, HitResult};
use crate::entity::store::EntityStore;
use crate::entity::throwable_projectile;
use crate::entity::{apply_gravity, Entity, EntityData};
use crate::level_event::VanillaGameEvent;
use bbb_server_types::fluid::{block_state_fluid_state, block_state_is_air};
use bbb_server_types::geometry::{Axis, Vec3Ops};

/// `LlamaSpit.getDefaultGravity()` (`LlamaSpit.java:37-38`).
pub const DEFAULT_GRAVITY: f64 = 0.06;

/// `LlamaSpit.tick` inertia (`LlamaSpit.java:51`).
const INERTIA: f32 = 0.99;

pub use super::hurting_projectile::ProjectileState as LlamaSpitData;

/// `LlamaSpit(Level, x, y, z)` filed in the store with the motion `shoot`
/// would have installed.
pub fn spawn_llama_spit(
    store: &mut EntityStore,
    position: Vec3d,
    delta_movement: Vec3d,
) -> Option<i32> {
    let uuid = store.next_uuid();
    let data = LlamaSpitData::default();
    let y_rot = (delta_movement.x.atan2(delta_movement.z).to_degrees() as f32).rem_euclid(360.0);
    let horizontal =
        (delta_movement.x * delta_movement.x + delta_movement.z * delta_movement.z).sqrt();
    let x_rot = (-delta_movement.y.atan2(horizontal).to_degrees() as f32).rem_euclid(360.0);
    let id = store.restore(
        uuid,
        VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID,
        position,
        EntityData::LlamaSpit(data),
    )?;
    let entity = store.get_mut(id).expect("restore leaves the entity live");
    entity.set_delta_movement(delta_movement);
    entity.set_rot(y_rot, x_rot);
    entity.set_needs_sync(true);
    Some(id)
}

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let EntityData::LlamaSpit(mut data) = entity.data else {
        return true;
    };
    // Projectile.java:99-103.
    if !data.has_been_shot() {
        level.game_event_at(
            VanillaGameEvent::PROJECTILE_SHOOT,
            entity.position(),
            throwable_projectile::owner_entity_id(data.owner(), store, level),
        );
        data.mark_shot();
    }
    if !data.left_owner() {
        data.set_left_owner(throwable_projectile::outside_owner_collision_range(
            entity,
            data.owner(),
            store,
            level,
        ));
    }
    entity.data = EntityData::LlamaSpit(data);
    if !crate::entity::damage::base_tick_from_environment(entity, store, level) {
        return false;
    }

    let owner = data.owner();
    let owner_vehicle = throwable_projectile::owner_vehicle(owner, data.left_owner(), store, level);
    let Some(hit) = hit_result_on_move_vector(entity, store, &*level, |candidate, store, level| {
        candidate.is_alive_and_pickable_in_level(level)
            && !throwable_projectile::rides_with_owner(candidate, owner_vehicle, store)
    }) else {
        return false;
    };
    match hit {
        HitResult::Entity {
            target,
            location,
            deflection,
        } => {
            if !hurting_projectile::apply_target_deflection(entity, store, target, deflection) {
                let Ok(redirect) =
                    hurting_projectile::projectile_deflection_action(target, owner, store, &*level)
                else {
                    return false;
                };
                if let Some(action) = redirect {
                    store.queue_command(crate::entity::EntityCommand::Projectile(action));
                }
                // `LlamaSpit.onHitEntity` (`LlamaSpit.java:64-73`): only a
                // living owner spits for damage.
                if let Some(causing) = throwable_projectile::pending_owner(owner, store, level) {
                    throwable_projectile::queue_hurt(
                        entity,
                        store,
                        level,
                        throwable_projectile::ThrowableHurt {
                            target: throwable_projectile::pending_target(target),
                            damage_type: "minecraft:spit",
                            amount: 1.0,
                            causing: Some(causing),
                            direct_resource_id: "minecraft:llama_spit",
                        },
                    );
                }
                level.game_event_at(
                    VanillaGameEvent::PROJECTILE_LAND,
                    location,
                    Some(entity.id()),
                );
            }
        }
        HitResult::Block {
            location,
            pos,
            direction,
            ..
        } => {
            crate::block_behavior::projectile_hit::on_projectile_hit(
                level,
                store,
                entity,
                crate::block_behavior::projectile_hit::ProjectileBlockHit {
                    location,
                    pos,
                    direction,
                },
            );
            let state = level.block_state(pos);
            level.game_event_with_state(
                VanillaGameEvent::PROJECTILE_LAND,
                pos,
                Some(entity.id()),
                Some(state),
            );
            entity.discard();
        }
        HitResult::Miss => {}
    }

    let movement = entity.delta_movement();
    let y_rot = (movement.x.atan2(movement.z).to_degrees() as f32).rem_euclid(360.0);
    let horizontal = (movement.x * movement.x + movement.z * movement.z).sqrt();
    let x_rot = (-movement.y.atan2(horizontal).to_degrees() as f32).rem_euclid(360.0);
    entity.set_rot(y_rot, x_rot);
    if overlaps_non_air(entity, level) || is_in_water(entity, level) {
        entity.discard();
        return true;
    }
    entity.delta_movement = movement.scale(f64::from(INERTIA));
    apply_gravity(entity);
    entity.set_pos(entity.position().plus(movement));
    store.refile_detached(entity);
    true
}

pub fn synched_values(entity: &Entity) -> Vec<SynchedValue> {
    crate::entity::living::entity_synched_values(entity)
}

fn overlaps_non_air(entity: &Entity, level: &LevelCtx<'_, '_>) -> bool {
    let bounding_box = entity.bounding_box();
    let min_x = bounding_box.min(Axis::X).floor() as i32;
    let min_y = bounding_box.min(Axis::Y).floor() as i32;
    let min_z = bounding_box.min(Axis::Z).floor() as i32;
    let max_x = bounding_box.max(Axis::X).floor() as i32;
    let max_y = bounding_box.max(Axis::Y).floor() as i32;
    let max_z = bounding_box.max(Axis::Z).floor() as i32;

    for x in min_x..=max_x {
        for y in min_y..=max_y {
            for z in min_z..=max_z {
                if block_state_is_air(level.block_state(BlockPos { x, y, z })) {
                    return false;
                }
            }
        }
    }
    true
}

fn is_in_water(entity: &Entity, level: &LevelCtx<'_, '_>) -> bool {
    fluid_state_tag_membership(
        level,
        block_state_fluid_state(level.block_state(entity.block_position())),
        "minecraft:water",
    )
}
