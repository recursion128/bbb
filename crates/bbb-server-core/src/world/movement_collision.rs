//! The collision geometry the walking and vehicle halves of
//! `ServerGamePacketListenerImpl`'s movement validation both resolve against.

use bbb_protocol::packets::{BlockPos, Direction, Vec3d};
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::geometry::{visit_block_collisions_with_scratch, Aabb};

use super::{stored_block_state, ServerLevel};

thread_local! {
    // PistonMovingBlockEntity.java:44: inline callbacks share the thread's collision context.
    static PISTON_NO_CLIP: std::cell::Cell<Option<Direction>> = const { std::cell::Cell::new(None) };
}

pub(super) fn current_piston_no_clip() -> Option<Direction> {
    PISTON_NO_CLIP.with(std::cell::Cell::get)
}

pub(super) fn set_piston_no_clip(direction: Option<Direction>) {
    PISTON_NO_CLIP.with(|current| current.set(direction));
}

/// Every box a probe has to sweep: the level's entities, the other sessions
/// and the world border, which are the three owners blocks do not carry.
pub(super) struct CollisionSnapshot {
    pub(super) entity_boxes: Vec<Aabb>,
    pub(super) player_boxes: Vec<Aabb>,
    pub(super) border: bbb_server_types::interaction::WorldBorder,
    pub(super) game_time: i64,
}

impl CollisionSnapshot {
    pub(super) fn collision_boxes(&self) -> Vec<Aabb> {
        let mut boxes = self.entity_boxes.clone();
        boxes.extend_from_slice(&self.player_boxes);
        boxes.extend(border_collision_boxes(self.border, self.game_time));
        boxes
    }
}

/// Every block collision box a swept bound can touch, with an unanswerable
/// state failing closed onto a full block.
pub(super) fn block_collision_boxes(chunks: &crate::chunk::ChunkStore, bounds: Aabb) -> Vec<Aabb> {
    let mut boxes = Vec::new();
    let _ = visit_block_collisions_with_scratch(
        bounds,
        |pos| moving_piston_at(chunks, pos).unwrap_or(true),
        |pos, out| push_block_collision_shape(chunks, pos, None, out),
        |_, shape| {
            boxes.push(shape);
            true
        },
    );
    boxes
}

/// Fast zero-collect block collision check using a reusable scratch buffer,
/// short-circuiting on the first intersecting shape.
#[allow(dead_code)]
pub(super) fn has_block_collision(chunks: &crate::chunk::ChunkStore, bounds: Aabb) -> bool {
    let mut hit = false;
    let _ = visit_block_collisions_with_scratch(
        bounds,
        |pos| moving_piston_at(chunks, pos).unwrap_or(true),
        |pos, out| push_block_collision_shape(chunks, pos, None, out),
        |_, _| {
            hit = true;
            false
        },
    );
    hit
}

pub(super) fn moving_piston_at(chunks: &crate::chunk::ChunkStore, pos: BlockPos) -> Option<bool> {
    let state = if chunks.is_inside_build_height(pos.y) {
        chunks.get_block_state(pos.x, pos.y, pos.z)?
    } else {
        chunks.air_state_id()
    };
    if let Some(block_id) =
        bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
    {
        return Some(block_id == bbb_protocol::block_types::VANILLA_BLOCK_MOVING_PISTON_ID);
    }
    chunks.content_registry().block_by_state_id(state)?;
    Some(false)
}

pub(super) fn static_collision_shape_with_live_piston(
    chunks: &crate::chunk::ChunkStore,
    pos: BlockPos,
    state: i32,
) -> Vec<Aabb> {
    let mut boxes = Vec::new();
    push_static_collision_shape_with_live_piston(chunks, pos, state, &mut boxes);
    boxes
}

pub(super) fn push_static_collision_shape_with_live_piston(
    chunks: &crate::chunk::ChunkStore,
    pos: BlockPos,
    state: i32,
    out: &mut Vec<Aabb>,
) {
    if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
        == Some(bbb_protocol::block_types::VANILLA_BLOCK_MOVING_PISTON_ID)
    {
        push_block_collision_shape(chunks, pos, None, out);
        return;
    }
    if bbb_server_types::geometry::push_static_block_collision_boxes(state, out).is_none() {
        if let Some((spec, _)) = chunks.content_registry().block_by_state_id(state) {
            if spec.full_cube_collision {
                out.push(Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 }));
            }
        } else {
            out.push(Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 }));
        }
    }
}

/// MovingPistonBlock.java:106-113: collision comes from the moving block entity.
pub(super) fn block_collision_shape(
    chunks: &crate::chunk::ChunkStore,
    pos: bbb_protocol::packets::BlockPos,
    piston_no_clip: Option<bbb_protocol::packets::Direction>,
) -> Vec<Aabb> {
    let mut boxes = Vec::new();
    push_block_collision_shape(chunks, pos, piston_no_clip, &mut boxes);
    boxes
}

pub(super) fn push_block_collision_shape(
    chunks: &crate::chunk::ChunkStore,
    pos: bbb_protocol::packets::BlockPos,
    piston_no_clip: Option<bbb_protocol::packets::Direction>,
    out: &mut Vec<Aabb>,
) {
    let state = stored_block_state(chunks, pos);
    if matches!(
        bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state),
        Some(
            bbb_protocol::block_types::VANILLA_BLOCK_POWDER_SNOW_ID
                | bbb_protocol::block_types::VANILLA_BLOCK_SCAFFOLDING_ID
        )
    ) {
        out.push(Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 }));
        return;
    }
    let start = out.len();
    if push_block_state_collision_shape_checked(chunks, state, pos, piston_no_clip, out).is_none() {
        out.truncate(start);
        if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
            != Some(bbb_protocol::block_types::VANILLA_BLOCK_MOVING_PISTON_ID)
        {
            out.push(Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 }));
        }
    }
}

pub(super) fn block_collision_shape_checked(
    chunks: &crate::chunk::ChunkStore,
    pos: BlockPos,
    piston_no_clip: Option<Direction>,
) -> Option<Vec<Aabb>> {
    let mut boxes = Vec::new();
    push_block_collision_shape_checked(chunks, pos, piston_no_clip, &mut boxes)?;
    Some(boxes)
}

pub(super) fn push_block_collision_shape_checked(
    chunks: &crate::chunk::ChunkStore,
    pos: BlockPos,
    piston_no_clip: Option<Direction>,
    out: &mut Vec<Aabb>,
) -> Option<()> {
    push_block_state_collision_shape_checked(
        chunks,
        stored_block_state(chunks, pos),
        pos,
        piston_no_clip,
        out,
    )
}

pub(super) fn block_state_collision_shape_checked(
    chunks: &crate::chunk::ChunkStore,
    state: i32,
    pos: BlockPos,
    piston_no_clip: Option<Direction>,
) -> Option<Vec<Aabb>> {
    let mut boxes = Vec::new();
    push_block_state_collision_shape_checked(chunks, state, pos, piston_no_clip, &mut boxes)?;
    Some(boxes)
}

pub(super) fn push_block_state_collision_shape_checked(
    chunks: &crate::chunk::ChunkStore,
    state: i32,
    pos: BlockPos,
    piston_no_clip: Option<Direction>,
    out: &mut Vec<Aabb>,
) -> Option<()> {
    let piston_no_clip = piston_no_clip.or_else(current_piston_no_clip);
    if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
        == Some(bbb_protocol::block_types::VANILLA_BLOCK_MOVING_PISTON_ID)
    {
        return chunks
            .block_entities()
            .get(pos)
            .and_then(crate::block_entity::BlockEntity::piston_snapshot)
            .map_or(Some(()), |snapshot| {
                let boxes = snapshot
                    .collision_boxes_with_resolver(piston_no_clip, |moved_state| {
                        empty_context_collision_shape(chunks, moved_state, pos)
                    })?;
                // PistonMovingBlockEntity.java:386-390: OR optimizes the translated shape.
                out.extend(
                    bbb_server_types::geometry::BoxShape::from_boxes(&boxes)?
                        .optimized()?
                        .boxes(),
                );
                Some(())
            });
    }
    push_empty_context_collision_shape(chunks, state, pos, out)
}

fn empty_context_collision_shape(
    chunks: &crate::chunk::ChunkStore,
    state: i32,
    pos: BlockPos,
) -> Option<Vec<Aabb>> {
    let mut boxes = Vec::new();
    push_empty_context_collision_shape(chunks, state, pos, &mut boxes)?;
    Some(boxes)
}

fn push_empty_context_collision_shape(
    chunks: &crate::chunk::ChunkStore,
    state: i32,
    pos: BlockPos,
    out: &mut Vec<Aabb>,
) -> Option<()> {
    use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
    use bbb_protocol::block_types::{
        VANILLA_BLOCK_MOVING_PISTON_ID, VANILLA_BLOCK_POWDER_SNOW_ID, VANILLA_BLOCK_SCAFFOLDING_ID,
    };
    let Some(block_type_id) = vanilla_block_state_block_type_id(state) else {
        let (spec, _) = chunks.content_registry().block_by_state_id(state)?;
        if spec.full_cube_collision {
            out.push(Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 }));
        }
        return Some(());
    };
    match block_type_id {
        VANILLA_BLOCK_MOVING_PISTON_ID => None,
        VANILLA_BLOCK_POWDER_SNOW_ID => Some(()),
        VANILLA_BLOCK_SCAFFOLDING_ID => {
            out.extend(bbb_server_types::geometry::scaffolding::collision_boxes(
                state,
                pos,
                bbb_server_types::geometry::scaffolding::Context::Empty,
            )?);
            Some(())
        }
        _ if bbb_server_types::block_queries::is_shulker_box(state) => {
            let bounds = chunks
                .block_entities()
                .get(pos)
                .and_then(crate::block_entity::BlockEntity::shulker_box)
                .map_or_else(
                    || Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 }),
                    |shulker| {
                        shulker.collision_box(crate::block_behavior::shulker_box::facing(state))
                    },
                );
            // ShulkerBoxBlock.java:152-155: Shapes.create rounds near-unit animation bounds.
            out.extend(bbb_server_types::geometry::BoxShape::from_box(bounds)?.boxes());
            Some(())
        }
        _ => bbb_server_types::geometry::positioned_shapes::push_collision_boxes(state, pos, out),
    }
}

pub(super) fn falling_block_collision_shape(
    chunks: &crate::chunk::ChunkStore,
    entity: &crate::entity::Entity,
    pos: BlockPos,
    piston_no_clip: Option<Direction>,
) -> Option<Vec<Aabb>> {
    if !matches!(entity.data(), crate::entity::EntityData::FallingBlock(_)) {
        return None;
    }
    stored_body_collision_shape(chunks, entity, pos, piston_no_clip)
}

pub(super) fn stored_growth_collision_shape(
    chunks: &crate::chunk::ChunkStore,
    entity: &crate::entity::Entity,
    pos: BlockPos,
    piston_no_clip: Option<Direction>,
) -> Option<Vec<Aabb>> {
    // A generated template cannot prove the current collision state of an unloaded column.
    if chunks.is_inside_build_height(pos.y) {
        chunks.get_block_state(pos.x, pos.y, pos.z)?;
    }
    stored_body_collision_shape(chunks, entity, pos, piston_no_clip)
}

pub(super) fn stored_body_collision_shape(
    chunks: &crate::chunk::ChunkStore,
    entity: &crate::entity::Entity,
    pos: BlockPos,
    piston_no_clip: Option<Direction>,
) -> Option<Vec<Aabb>> {
    let mut boxes = Vec::new();
    push_stored_body_collision_shape(chunks, entity, pos, piston_no_clip, &mut boxes)?;
    Some(boxes)
}

pub(super) fn push_stored_body_collision_shape(
    chunks: &crate::chunk::ChunkStore,
    entity: &crate::entity::Entity,
    pos: BlockPos,
    piston_no_clip: Option<Direction>,
    out: &mut Vec<Aabb>,
) -> Option<()> {
    let falling = matches!(entity.data(), crate::entity::EntityData::FallingBlock(_));
    let state = stored_block_state(chunks, pos);
    if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
        == Some(bbb_protocol::block_types::VANILLA_BLOCK_SCAFFOLDING_ID)
    {
        out.extend(bbb_server_types::geometry::scaffolding::collision_boxes(
            state,
            pos,
            bbb_server_types::geometry::scaffolding::Context::Entity {
                bottom: entity.position().y,
                descending: entity.get_shared_flag(
                    bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
                ),
                placement: false,
            },
        )?);
        return Some(());
    }
    if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
        == Some(bbb_protocol::block_types::VANILLA_BLOCK_POWDER_SNOW_ID)
    {
        // PowderSnowBlock.java:116-143: distance and Falling short-circuit the tag lookup.
        let can_walk = if falling || entity.fall_distance() > 2.5 {
            Some(false)
        } else {
            let name = entity.data().entity_resource_id();
            Some(
                crate::entity::inside_blocks::body_contact::can_walk_on_powder_snow(
                    name,
                    entity
                        .living()
                        .and_then(|living| {
                            living.item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::Feet)
                        })
                        .and_then(|item| item.item_id()),
                    |registry, tag, resource| {
                        chunks
                            .registry_tags()
                            .is_some_and(|tags| tags.contains(registry, tag, resource))
                    },
                ),
            )
        };
        out.extend(crate::entity::powder_snow_body_collision(
            entity.fall_distance(),
            falling,
            can_walk,
            entity.position().y,
            entity.get_shared_flag(
                bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
            ),
            pos,
        )?);
        return Some(());
    }
    if crate::fluid::liquid_block_family(state).is_some() {
        return push_stored_liquid_collision_shape(chunks, entity, state, pos, out);
    }
    push_block_state_collision_shape_checked(chunks, state, pos, piston_no_clip, out)
}

fn push_stored_liquid_collision_shape(
    chunks: &crate::chunk::ChunkStore,
    entity: &crate::entity::Entity,
    state: i32,
    pos: BlockPos,
    out: &mut Vec<Aabb>,
) -> Option<()> {
    use bbb_server_types::fluid::block_state_fluid_state;

    // LiquidBlock.java:82-101; Strider.java:331-333: only a source has a stable surface.
    let fluid = block_state_fluid_state(state);
    if !fluid.is_source()
        || entity.living().is_none()
        || entity.entity_type_id() != bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_STRIDER_ID
        || entity
            .position()
            .y
            .partial_cmp(&(f64::from(pos.y) + 0.5 - f64::from(1.0e-5_f32)))
            != Some(std::cmp::Ordering::Greater)
    {
        return Some(());
    }
    // EntityCollisionContext.java:61-65; Strider.java:174-176: membership precedes isSame.
    if !chunks.registry_tags().is_some_and(|tags| {
        tags.contains(
            "minecraft:fluid",
            "minecraft:lava",
            fluid.identity().resource_id(),
        )
    }) {
        return Some(());
    }
    let above = BlockPos {
        y: pos.y.checked_add(1)?,
        ..pos
    };
    let above_state = stored_block_state(chunks, above);
    if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(above_state).is_none() {
        chunks.content_registry().block_by_state_id(above_state)?;
    }
    if block_state_fluid_state(above_state).family() == fluid.family() {
        return Some(());
    }
    out.push(Aabb {
        max_y: 0.5,
        ..Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 })
    });
    Some(())
}

pub(super) fn no_blocks_around(chunks: &crate::chunk::ChunkStore, bounds: Aabb) -> bool {
    no_blocks_around_with(bounds, |pos| Some(stored_block_state(chunks, pos))).unwrap()
}

pub(super) fn no_blocks_around_checked(
    chunks: &crate::chunk::ChunkStore,
    bounds: Aabb,
) -> Option<bool> {
    no_blocks_around_with(bounds, |pos| {
        let state = if chunks.is_inside_build_height(pos.y) {
            chunks.get_block_state(pos.x, pos.y, pos.z)?
        } else {
            chunks.air_state_id()
        };
        if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state).is_none() {
            chunks.content_registry().block_by_state_id(state)?;
        }
        Some(state)
    })
}

fn no_blocks_around_with(
    bounds: Aabb,
    mut state_at: impl FnMut(BlockPos) -> Option<i32>,
) -> Option<bool> {
    // BlockGetter.java:40-42; BlockPos.java:397-399.
    for x in bounds.min_x.floor() as i32..=bounds.max_x.floor() as i32 {
        for y in bounds.min_y.floor() as i32..=bounds.max_y.floor() as i32 {
            for z in bounds.min_z.floor() as i32..=bounds.max_z.floor() as i32 {
                if !block_state_is_air(state_at(BlockPos { x, y, z })?) {
                    return Some(false);
                }
            }
        }
    }
    Some(true)
}

/// `ServerGamePacketListenerImpl.getMaximumFlyingTicks` (`:356-364`).
pub(super) fn maximum_flying_ticks(gravity: f64) -> i32 {
    if gravity < f64::from(1.0e-5_f32) {
        return i32::MAX;
    }
    (80.0 * (0.08 / gravity).max(1.0)).ceil() as i32
}

pub(super) fn border_collision_boxes(
    border: bbb_server_types::interaction::WorldBorder,
    game_time: i64,
) -> [Aabb; 4] {
    let min_x = border.min_x_at(game_time);
    let max_x = border.max_x_at(game_time);
    let min_z = border.min_z_at(game_time);
    let max_z = border.max_z_at(game_time);
    let low = -20_000_001.0;
    let high = 20_000_001.0;
    [
        Aabb {
            min_x: -30_000_001.0,
            max_x: min_x,
            min_y: low,
            max_y: high,
            min_z: -30_000_001.0,
            max_z: 30_000_001.0,
        },
        Aabb {
            min_x: max_x,
            max_x: 30_000_001.0,
            min_y: low,
            max_y: high,
            min_z: -30_000_001.0,
            max_z: 30_000_001.0,
        },
        Aabb {
            min_x,
            max_x,
            min_y: low,
            max_y: high,
            min_z: -30_000_001.0,
            max_z: min_z,
        },
        Aabb {
            min_x,
            max_x,
            min_y: low,
            max_y: high,
            min_z: max_z,
            max_z: 30_000_001.0,
        },
    ]
}

pub(super) fn add(left: Vec3d, right: Vec3d) -> Vec3d {
    Vec3d {
        x: left.x + right.x,
        y: left.y + right.y,
        z: left.z + right.z,
    }
}

pub(super) fn subtract(left: Vec3d, right: Vec3d) -> Vec3d {
    Vec3d {
        x: left.x - right.x,
        y: left.y - right.y,
        z: left.z - right.z,
    }
}

/// Accessible collidable entity bounding boxes in `level` intersecting `bounds`
/// (plus Vanilla's `1.0e-7` inflation).
pub(super) fn level_entity_boxes_in(level: &ServerLevel, bounds: Aabb) -> Vec<(i32, Aabb)> {
    let query = bounds.inflate(1.0e-7, 1.0e-7, 1.0e-7);
    let mut boxes: Vec<_> = level
        .entities
        .ids_in_box_sections(query)
        .into_iter()
        .filter_map(|id| level.entities.get(id))
        .filter(|entity| {
            entity.can_be_collided_with()
                && entity.bounding_box().intersects(query)
                && level
                    .entities
                    .section_visibility(crate::entity::store::section_of_block(
                        entity.block_position(),
                    ))
                    .is_accessible()
        })
        .map(|entity| (entity.id(), entity.bounding_box()))
        .collect();
    boxes.sort_unstable_by_key(|&(id, _)| id);
    boxes
}

pub(super) fn player_block_collision_shape_checked(
    chunks: &crate::chunk::ChunkStore,
    player: &super::PlayerState,
    pos: BlockPos,
) -> Option<Vec<Aabb>> {
    player_block_collision_shape_with_context(chunks, player, pos, player.position().y, false)
}

pub(super) fn push_player_block_collision_shape_checked(
    chunks: &crate::chunk::ChunkStore,
    player: &super::PlayerState,
    pos: BlockPos,
    out: &mut Vec<Aabb>,
) -> Option<()> {
    push_player_block_collision_shape_with_context(
        chunks,
        player,
        pos,
        player.position().y,
        false,
        out,
    )
}

pub(super) fn player_block_collision_shape_with_context(
    chunks: &crate::chunk::ChunkStore,
    player: &super::PlayerState,
    pos: BlockPos,
    bottom: f64,
    placement: bool,
) -> Option<Vec<Aabb>> {
    let mut boxes = Vec::new();
    push_player_block_collision_shape_with_context(
        chunks, player, pos, bottom, placement, &mut boxes,
    )?;
    Some(boxes)
}

pub(super) fn push_player_block_collision_shape_with_context(
    chunks: &crate::chunk::ChunkStore,
    player: &super::PlayerState,
    pos: BlockPos,
    bottom: f64,
    placement: bool,
    out: &mut Vec<Aabb>,
) -> Option<()> {
    if chunks.is_inside_build_height(pos.y) {
        chunks.get_block_state(pos.x, pos.y, pos.z)?;
    }
    let state = stored_block_state(chunks, pos);
    crate::entity::player_collision::push_collision_boxes_with_context(
        state,
        pos,
        bottom,
        player.shared_flag(
            bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
        ),
        placement,
        Some(crate::entity::player_collision::PlayerCollisionFacts {
            fall_distance: player.block_contact.fall_distance,
            feet_item_id: player
                .inventory
                .equipment_slots()
                .into_iter()
                .find(|(slot, _)| *slot == bbb_protocol::equipment_slots::EquipmentSlot::Feet)
                .and_then(|(_, index)| player.inventory.slot(index))
                .and_then(|stack| stack.item_id()),
        }),
        |registry, tag, entry| {
            chunks
                .registry_tags()
                .is_some_and(|tags| tags.contains(registry, tag, entry))
        },
        |out| push_block_state_collision_shape_checked(chunks, state, pos, None, out),
        out,
    )
}

#[cfg(test)]
mod shulker_tests;

#[cfg(test)]
mod falling_shapes_tests;

#[cfg(test)]
mod stored_context_tests;

#[cfg(test)]
mod strider_fluid_tests;

#[cfg(test)]
mod piston_cursor_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod growth_tests;

#[cfg(test)]
mod death_growth_tests;
