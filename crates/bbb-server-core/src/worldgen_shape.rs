//! Block shape updates run by world generation hosts.

use crate::neighbor_updater::UPDATE_SHAPE_ORDER;
use crate::random::GameRandom;
use bbb_protocol::block_types::{VANILLA_BLOCK_LAVA_ID, VANILLA_BLOCK_WATER_ID};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};
use bbb_server_worldgen::tree::voxel_shape::BitSetDiscreteVoxelShape;
use bbb_server_worldgen::tree::{offset, opposite, step};
use bbb_server_worldgen::WorldGenLevel;

use crate::block_behavior::shape_context::BlockShapeCtx;
use crate::block_behavior::{state_is_block, update_block_shape};

pub fn update_shape_at_edge<L: WorldGenLevel>(
    level: &mut L,
    update_mode: i32,
    shape: &BitSetDiscreteVoxelShape,
    start_x: i32,
    start_y: i32,
    start_z: i32,
) {
    let mut faces = Vec::new();
    shape.for_all_faces(|direction, x, y, z| faces.push((direction, x, y, z)));
    let mut random = GameRandom::with_seed(0);
    for (direction, x, y, z) in faces {
        let pos = BlockPos {
            x: start_x + x,
            y: start_y + y,
            z: start_z + z,
        };
        let (dx, dy, dz) = step(direction);
        let neighbor_pos = offset(pos, dx, dy, dz);
        let state = level.get_block(pos);
        let neighbor_state = level.get_block(neighbor_pos);
        let new_state = update_block_shape(
            &mut BlockShapeCtx::Generation {
                level,
                random: &mut random,
            },
            state,
            pos,
            direction,
            neighbor_pos,
            neighbor_state,
        );
        if state != new_state {
            level.set_block(pos, new_state, update_mode & -2);
        }
        let new_neighbor_state = update_block_shape(
            &mut BlockShapeCtx::Generation {
                level,
                random: &mut random,
            },
            neighbor_state,
            neighbor_pos,
            opposite(direction),
            pos,
            new_state,
        );
        if neighbor_state != new_neighbor_state {
            level.set_block(neighbor_pos, new_neighbor_state, update_mode & -2);
        }
    }
}

pub fn update_from_neighbour_shapes<L: WorldGenLevel>(
    level: &mut L,
    state: i32,
    pos: BlockPos,
) -> i32 {
    let random = GameRandom::with_seed(0);
    let mut new_state = state;
    for direction in UPDATE_SHAPE_ORDER {
        let (dx, dy, dz) = step(direction);
        let neighbour_pos = offset(pos, dx, dy, dz);
        let neighbour_state = level.get_block(neighbour_pos);
        new_state = update_template_block_shape(
            level,
            &random,
            new_state,
            pos,
            direction,
            neighbour_pos,
            neighbour_state,
        );
    }
    new_state
}

pub fn update_template_shape_at_edge<L: WorldGenLevel>(
    level: &mut L,
    update_mode: i32,
    shape: &BitSetDiscreteVoxelShape,
    start_x: i32,
    start_y: i32,
    start_z: i32,
) {
    let mut faces = Vec::new();
    shape.for_all_faces(|direction, x, y, z| faces.push((direction, x, y, z)));
    let random = GameRandom::with_seed(0);
    for (direction, x, y, z) in faces {
        let pos = BlockPos {
            x: start_x + x,
            y: start_y + y,
            z: start_z + z,
        };
        let (dx, dy, dz) = step(direction);
        let neighbor_pos = offset(pos, dx, dy, dz);
        let state = level.get_block(pos);
        let neighbor_state = level.get_block(neighbor_pos);
        let new_state = update_template_block_shape(
            level,
            &random,
            state,
            pos,
            direction,
            neighbor_pos,
            neighbor_state,
        );
        if state != new_state {
            level.set_block(pos, new_state, update_mode & -2);
        }
        let new_neighbor_state = update_template_block_shape(
            level,
            &random,
            neighbor_state,
            neighbor_pos,
            opposite(direction),
            pos,
            new_state,
        );
        if neighbor_state != new_neighbor_state {
            level.set_block(neighbor_pos, new_neighbor_state, update_mode & -2);
        }
    }
}

fn update_template_block_shape<L: WorldGenLevel>(
    level: &mut L,
    random: &GameRandom,
    state: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_pos: BlockPos,
    neighbour_state: i32,
) -> i32 {
    if state_is_block(state, VANILLA_BLOCK_WATER_ID) || state_is_block(state, VANILLA_BLOCK_LAVA_ID)
    {
        let fluid = block_state_fluid_state(state);
        if fluid.is_source() || block_state_fluid_state(neighbour_state).is_source() {
            let delay = match fluid.family() {
                Some(FluidFamily::Water) => 5,
                Some(FluidFamily::Lava) if level.fast_lava_dimension_value() => 10,
                Some(FluidFamily::Lava) => 30,
                None => unreachable!("liquid block without a fluid"),
            };
            level.schedule_fluid_tick(pos, delay);
        }
        if direction_to_neighbour == Direction::Down
            && fluid.family() == Some(FluidFamily::Water)
            && fluid.is_source()
            && fluid.is_full()
            && (crate::block_behavior::shape_context::block_tag_matches(
                neighbour_state,
                "minecraft:enables_bubble_column_drag_down",
                &|registry, tag, id| level.registry_tag_contains(registry, tag, id),
            ) || crate::block_behavior::shape_context::block_tag_matches(
                neighbour_state,
                "minecraft:enables_bubble_column_push_up",
                &|registry, tag, id| level.registry_tag_contains(registry, tag, id),
            ))
        {
            level.schedule_block_tick(pos, 20);
        }
        return state;
    }
    update_block_shape(
        &mut BlockShapeCtx::Generation { level, random },
        state,
        pos,
        direction_to_neighbour,
        neighbour_pos,
        neighbour_state,
    )
}

struct WorldGenBlockShapeView<'a, L>(&'a L);

impl<L: WorldGenLevel> crate::block_getter::BlockTags for WorldGenBlockShapeView<'_, L> {
    fn contains(&self, registry_id: &str, tag_id: &str, element_id: &str) -> bool {
        self.0
            .registry_tag_contains(registry_id, tag_id, element_id)
    }
}

impl<L: WorldGenLevel> crate::block_getter::BlockGetter for WorldGenBlockShapeView<'_, L> {
    fn block_state(&self, pos: BlockPos) -> i32 {
        self.0.get_block(pos)
    }
    fn raw_brightness(&self, pos: BlockPos, sky_darkening: i32) -> Option<i32> {
        Some(self.0.get_raw_brightness(pos, sky_darkening))
    }
    fn tags(&self) -> &dyn crate::block_getter::BlockTags {
        self
    }
}

pub fn can_survive_at<L: WorldGenLevel>(level: &L, state: i32, pos: BlockPos) -> bool {
    crate::block_behavior::can_survive_at(state, pos, &WorldGenBlockShapeView(level))
}
