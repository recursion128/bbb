//! `FireworkRocketItem.useOn` (`FireworkRocketItem.java:28-55`): a rocket
//! launched from the clicked face.

use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING;
use bbb_protocol::packets::{BlockPos, ItemStackSummary, Vec3d};
use bbb_server_types::directions::relative;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::ItemUseOnContext;
use crate::entity::firework_rocket::spawn_launched_firework;
use crate::world::{ServerWorld, WorldEvent};

pub(super) fn use_on(
    world: &mut ServerWorld,
    context: &ItemUseOnContext<'_>,
    _events: &mut Vec<WorldEvent>,
) -> bool {
    let player = &world.levels[context.connection_id];
    // `:30-33`: a gliding player's click falls through to `use`.
    if player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING) {
        return false;
    }
    let owner = player.profile.uuid;
    let hit = context.update.hit;
    let step = relative(BlockPos { x: 0, y: 0, z: 0 }, hit.direction);
    let position = Vec3d {
        x: f64::from(hit.pos.x) + f64::from(hit.cursor_x) + f64::from(step.x) * 0.15,
        y: f64::from(hit.pos.y) + f64::from(hit.cursor_y) + f64::from(step.y) * 0.15,
        z: f64::from(hit.pos.z) + f64::from(hit.cursor_z) + f64::from(step.z) * 0.15,
    };
    let item = RuntimeItemStack::from_summary(ItemStackSummary {
        count: 1,
        ..context.stack.clone()
    });
    let store = &mut world.level_mut_by_id(&context.dimension).entities;
    if let Some(id) = spawn_launched_firework(store, position, item.clone(), owner, None) {
        crate::entity::projectile_weapon::queue_spawn_effects(store, id, item);
    }
    world.shrink_used_stack(context);
    true
}
