use crate::entity::{Entity, EntityStore};
use crate::random::GameRandom;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::packets::Vec3d;

pub fn body_water_splash(movement: Vec3d, controlled: bool, random: &GameRandom) -> (f32, f32) {
    let modifier = if controlled { 0.9_f32 } else { 0.2_f32 };
    let speed = ((movement.x * movement.x * f64::from(0.2_f32)
        + movement.y * movement.y
        + movement.z * movement.z * f64::from(0.2_f32))
    .sqrt() as f32
        * modifier)
        .min(1.0);
    let pitch = 1.0_f32 + (random.next_float() - random.next_float()) * 0.4_f32;
    (speed, pitch)
}

pub(crate) fn controller_movement(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<(Vec3d, bool)> {
    use crate::entity::passenger::DirectLivingController;
    let tags = level.capture_registry_tags();
    match crate::entity::passenger::direct_controlling_living_with_registry_tags(
        entity,
        Some(store),
        &tags,
        |id| level.player_by_entity_id(id),
    )? {
        None => Some((entity.delta_movement(), false)),
        Some(DirectLivingController::Stored(id)) => Some((store.get(id)?.delta_movement(), true)),
        Some(DirectLivingController::Player(player)) => {
            let movement = if let Some(body) = store.get(player.entity_id) {
                body.delta_movement()
            } else {
                level.player_by_entity_id(player.entity_id)?.delta_movement
            };
            Some((movement, true))
        }
    }
}

pub(super) fn sound(entity: &Entity, speed: f32) -> &'static str {
    use bbb_protocol::entity_types::*;
    if entity.entity_type_id() == VANILLA_ENTITY_TYPE_PLAYER_ID {
        return if speed < 0.25 {
            "minecraft:entity.player.splash"
        } else {
            "minecraft:entity.player.splash.high_speed"
        };
    }
    if speed >= 0.25 {
        return "minecraft:entity.generic.splash";
    }
    match entity.entity_type_id() {
        VANILLA_ENTITY_TYPE_DOLPHIN_ID => "minecraft:entity.dolphin.splash",
        VANILLA_ENTITY_TYPE_AXOLOTL_ID => "minecraft:entity.axolotl.splash",
        VANILLA_ENTITY_TYPE_HOGLIN_ID => "minecraft:entity.hostile.splash",
        id if crate::entity::ai::mob::spec::is_instance_of(id, "Monster") => {
            "minecraft:entity.hostile.splash"
        }
        _ => "minecraft:entity.generic.splash",
    }
}
