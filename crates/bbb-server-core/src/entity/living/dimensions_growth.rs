use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
    VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_SLIME_ID,
};
use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::{collect_block_collision_shapes_checked, Aabb, BoxShape};
use bbb_server_types::interaction::WorldBorder;

use crate::entity::Entity;

pub struct SizeGrowthCollisionView<'a> {
    pub collisions: &'a dyn Fn(&Entity, Aabb) -> Option<Vec<BoxShape>>,
    pub border: &'a dyn Fn() -> Option<WorldBorder>,
    pub game_time: &'a dyn Fn() -> i64,
}

pub(crate) fn resolve(
    entity: &mut Entity,
    previous: [f32; 2],
    level: &LevelCtx<'_, '_>,
) -> Option<()> {
    resolve_in_store(entity, previous, level, None)
}

pub(crate) fn resolve_in_store(
    entity: &mut Entity,
    previous: [f32; 2],
    level: &LevelCtx<'_, '_>,
    store: Option<&mut crate::entity::EntityStore>,
) -> Option<()> {
    resolve_with_collision_view_in_store(
        entity,
        previous,
        &SizeGrowthCollisionView {
            collisions: &|entity, query| {
                Some(
                    collect_block_collision_shapes_checked(
                        query,
                        |pos| level.is_moving_piston_at(pos),
                        |pos| level.block_collision_boxes_for_size_growth(entity, pos),
                    )?
                    .into_iter()
                    .map(|(_, shape)| shape)
                    .collect(),
                )
            },
            border: &|| level.world_border(),
            game_time: &|| level.game_time(),
        },
        store,
    )
}

pub fn resolve_with_collision_view(
    entity: &mut Entity,
    previous: [f32; 2],
    view: &SizeGrowthCollisionView<'_>,
) -> Option<()> {
    resolve_with_collision_view_in_store(entity, previous, view, None)
}

fn resolve_with_collision_view_in_store(
    entity: &mut Entity,
    previous: [f32; 2],
    view: &SizeGrowthCollisionView<'_>,
    mut store: Option<&mut crate::entity::EntityStore>,
) -> Option<()> {
    if !requires_collision(entity, previous) {
        return Some(());
    }
    let [width, height] = [entity.bb_width(), entity.bb_height()];
    let [old_width, old_height] = previous;
    let original = entity.position();
    let center = Vec3d {
        y: original.y + f64::from(old_height) / 2.0,
        ..original
    };
    let width_delta = f64::from((width - old_width).max(0.0)) + 1.0e-6;
    let height_delta = f64::from((height - old_height).max(0.0)) + 1.0e-6;
    let allowed = BoxShape::from_box(Aabb::of_size(
        center,
        width_delta,
        height_delta,
        width_delta,
    ))?;
    let mut position =
        find_free(entity, view, &allowed, center, [width, height])?.map(|found| Vec3d {
            y: found.y - f64::from(height) / 2.0,
            ..found
        });
    if position.is_none() && width > old_width && height > old_height {
        let allowed = BoxShape::from_box(Aabb::of_size(center, width_delta, 1.0e-6, width_delta))?;
        position =
            find_free(entity, view, &allowed, center, [width, old_height])?.map(|found| Vec3d {
                y: found.y + (-f64::from(old_height) / 2.0 + 1.0e-6),
                ..found
            });
    }
    if let Some(position) = position {
        entity.set_pos(position);
        if let Some(store) = store.as_deref_mut() {
            store.refile_detached(entity);
        }
        if matches!(
            entity.entity_type_id(),
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
                | VANILLA_ENTITY_TYPE_SLIME_ID
                | VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID
        ) {
            entity.set_pos(original);
            if let Some(store) = store {
                store.refile_detached(entity);
            }
        }
    }
    Some(())
}

pub(crate) fn requires_collision(entity: &Entity, previous: [f32; 2]) -> bool {
    let [width, height] = [entity.bb_width(), entity.bb_height()];
    !entity.is_first_tick()
        && !entity.no_physics()
        && width <= 4.0
        && height <= 4.0
        && (width > previous[0] || height > previous[1])
        && entity.entity_type_id() != VANILLA_ENTITY_TYPE_PLAYER_ID
}

fn find_free(
    entity: &Entity,
    view: &SizeGrowthCollisionView<'_>,
    allowed: &BoxShape,
    preferred: Vec3d,
    size: [f32; 2],
) -> Option<Option<Vec3d>> {
    if allowed.is_empty() {
        return Some(None);
    }
    let [width, height] = size.map(f64::from);
    let search = allowed.bounds()?.inflate(width, height, width);
    let shapes = (view.collisions)(entity, search)?;
    let border = (view.border)();
    let game_time = (view.game_time)();
    let mut expanded = BoxShape::empty();
    for shape in shapes {
        if border.is_some_and(|border| {
            !within_border(
                &border,
                shape.bounds().expect("a collision hit is nonempty"),
                game_time,
            )
        }) {
            continue;
        }
        for bounds in shape.boxes() {
            expanded = expanded.union(&BoxShape::from_box(bounds.inflate(
                width / 2.0,
                height / 2.0,
                width / 2.0,
            ))?)?;
        }
    }
    Some(allowed.only_first(&expanded)?.closest_point(preferred))
}

fn within_border(border: &WorldBorder, bounds: Aabb, game_time: i64) -> bool {
    let epsilon = f64::from(1.0e-5_f32);
    let [min_x, max_x, min_z, max_z] = [
        border.min_x_at(game_time),
        border.max_x_at(game_time),
        border.min_z_at(game_time),
        border.max_z_at(game_time),
    ];
    [bounds.min_x, bounds.max_x - epsilon]
        .into_iter()
        .all(|x| x >= min_x && x < max_x)
        && [bounds.min_z, bounds.max_z - epsilon]
            .into_iter()
            .all(|z| z >= min_z && z < max_z)
}

#[cfg(test)]
mod tests;
