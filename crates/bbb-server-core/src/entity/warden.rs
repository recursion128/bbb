//! Shared Warden targeting rules and the owned anger runtime.

pub(crate) mod anger;
pub(crate) mod awareness;
pub(crate) mod lifecycle;
pub(crate) mod sonic_boom;

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_WARDEN_ID,
};
use bbb_server_types::geometry::Aabb;
use bbb_server_types::interaction::WorldBorder;

// Warden.java:164-166.
pub(crate) fn is_digging_or_emerging(entity_type_id: i32, pose_id: i32) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_WARDEN_ID
        && matches!(
            pose_id,
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_DIGGING_ID
                | bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_EMERGING_ID
        )
}

// Warden.java:388-399; WorldBorder.java:60-74.
#[allow(clippy::too_many_arguments)]
pub(crate) fn can_target(
    entity_type_id: i32,
    same_level: bool,
    creative_or_spectator: bool,
    allied: bool,
    invulnerable: bool,
    dead_or_dying: bool,
    border: WorldBorder,
    game_time: i64,
    bounds: Aabb,
) -> bool {
    let inset = f64::from(1.0E-5_f32);
    let within = |x: f64, z: f64| {
        x >= border.min_x_at(game_time)
            && x < border.max_x_at(game_time)
            && z >= border.min_z_at(game_time)
            && z < border.max_z_at(game_time)
    };
    same_level
        && !creative_or_spectator
        && !allied
        && !matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID | VANILLA_ENTITY_TYPE_WARDEN_ID
        )
        && !invulnerable
        && !dead_or_dying
        && within(bounds.min_x, bounds.min_z)
        && within(bounds.max_x - inset, bounds.max_z - inset)
}

// Warden.java:291-293.
pub(crate) fn custom_server_ai_step(
    entity: &crate::entity::Entity,
    store: &mut crate::entity::EntityStore,
) {
    if entity.entity_type_id() == VANILLA_ENTITY_TYPE_WARDEN_ID && entity.tick_count() % 120 == 0 {
        store.queue_command(crate::entity::EntityCommand::WardenDarkness {
            source: entity.uuid(),
            position: entity.position(),
        });
    }
}
