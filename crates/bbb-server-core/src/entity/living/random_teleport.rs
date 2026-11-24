use crate::world::simulation_level::LevelCtx;
use bbb_protocol::packets::{BlockPos, EntityEvent, Vec3d};
use bbb_server_types::fluid::block_state_fluid_state;
use bbb_server_types::geometry::{no_block_collision_with_moving_pistons, Aabb};

use crate::entity::passenger::teleport::TeleportTree;
use crate::entity::{store::EntityStore, Entity};

pub(crate) fn attempt(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    destination: Vec3d,
    show_particles: bool,
) -> Option<bool> {
    let origin = body.position();
    let snap_back_to_origin = |body: &mut Entity, store: &mut EntityStore| -> Option<()> {
        if store.passengers(body.id()).is_empty() {
            body.set_old_pos_and_rot();
            return Some(());
        }
        let mut tree = TeleportTree::capture(body, store)?;
        tree.snap_to(origin, level.game_time())?;
        tree.apply_to(body, store)
    };
    let Some(landed_dest) = find_landed_destination(level, destination)? else {
        snap_back_to_origin(body, store)?;
        return Some(false);
    };
    let bounds = Aabb::of_entity(landed_dest, body.bb_width(), body.bb_height());
    // Pre-validate blocks, liquids, world border, and nearby entities spatially
    // before cloning the entity/passenger tree via `TeleportTree::capture`.
    if !prevalidate_destination(body, store, level, landed_dest, bounds)? {
        snap_back_to_origin(body, store)?;
        return Some(false);
    }
    let mut tree = TeleportTree::capture(body, store)?;
    tree.snap_to(landed_dest, level.game_time())?;
    tree.apply_to(body, store)?;
    if show_particles {
        store.broadcast_to_tracking(
            body.id(),
            crate::connection::ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id: body.id(),
                event_id: 46,
            }),
        );
    }
    if let Some(runtime) = store.mob_ai_mut(body.id()) {
        runtime.stop_navigation();
    }
    Some(true)
}

fn find_landed_destination(level: &LevelCtx<'_, '_>, destination: Vec3d) -> Option<Option<Vec3d>> {
    let mut y = destination.y;
    let mut pos = block_position(destination);
    if !level.has_ticking_column_at(pos) {
        return Some(None);
    }
    while pos.y > level.min_y() {
        let below = BlockPos {
            y: pos.y - 1,
            ..pos
        };
        if blocks_motion(level, below)? {
            return Some(Some(Vec3d { y, ..destination }));
        }
        y -= 1.0;
        pos = below;
    }
    Some(None)
}

fn prevalidate_destination(
    body: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    destination: Vec3d,
    bounds: Aabb,
) -> Option<bool> {
    if !level
        .world_border()?
        .no_collision_at(destination, bounds, level.game_time())
    {
        return Some(false);
    }
    if contains_any_liquid(level, bounds)? {
        return Some(false);
    }
    if !no_block_collision_with_moving_pistons(
        bounds,
        |pos| {
            level.block_state_checked(pos)?;
            level.is_moving_piston_at(pos)
        },
        |pos| {
            level.block_state_checked(pos)?;
            level.block_collision_boxes_for_entity(body, pos)
        },
    )? {
        return Some(false);
    }
    let size = (bounds.max_x - bounds.min_x + bounds.max_y - bounds.min_y + bounds.max_z
        - bounds.min_z)
        / 3.0;
    if size >= 1.0e-7 {
        let search = bounds.inflate(1.0e-7, 1.0e-7, 1.0e-7);
        for id in store.ids_in_box_sections(search) {
            if id == body.id() {
                continue;
            }
            let Some(other) = store.get(id) else {
                continue;
            };
            if other.removal_reason().is_none()
                && other.bounding_box().intersects(search)
                && other.can_be_collided_with()
                && !store.checked_same_vehicle(body, id)?
            {
                return Some(false);
            }
        }
    }
    Some(true)
}

pub(super) fn contains_any_liquid(level: &LevelCtx<'_, '_>, bounds: Aabb) -> Option<bool> {
    for x in bounds.min_x.floor() as i32..bounds.max_x.ceil() as i32 {
        for y in bounds.min_y.floor() as i32..bounds.max_y.ceil() as i32 {
            for z in bounds.min_z.floor() as i32..bounds.max_z.ceil() as i32 {
                if !block_state_fluid_state(level.block_state_checked(BlockPos { x, y, z })?)
                    .is_empty()
                {
                    return Some(true);
                }
            }
        }
    }
    Some(false)
}

pub(crate) fn block_position(position: Vec3d) -> BlockPos {
    BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}

pub(crate) fn blocks_motion(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<bool> {
    Some(bbb_server_types::fluid::block_state_blocks_motion(
        level.block_state_checked(pos)?,
    ))
}

#[cfg(test)]
pub(crate) mod tests;
