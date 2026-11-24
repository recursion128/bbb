use bbb_protocol::block_state_types::VanillaBlockSupportType;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::block_queries::{
    collision_face_nonempty_at, collision_shape_empty_at, face_sturdy_at,
    multiface_can_attach_to_at,
};

use crate::WorldGenLevel;

pub(crate) fn feature_owned_block_can_survive<L: WorldGenLevel>(
    level: &L,
    pos: BlockPos,
    state: i32,
) -> bool {
    level.can_survive_at(state, pos)
}

pub(crate) fn feature_face_sturdy<L: WorldGenLevel>(
    _level: &L,
    pos: BlockPos,
    state: i32,
    direction: Direction,
    support_type: VanillaBlockSupportType,
) -> bool {
    face_sturdy_at(state, pos, direction, support_type)
}

pub(crate) fn feature_collision_face_nonempty<L: WorldGenLevel>(
    _level: &L,
    pos: BlockPos,
    state: i32,
    direction: Direction,
) -> bool {
    collision_face_nonempty_at(state, pos, direction)
}

pub(crate) fn feature_collision_shape_empty<L: WorldGenLevel>(
    _level: &L,
    pos: BlockPos,
    state: i32,
) -> bool {
    collision_shape_empty_at(state, pos)
}

pub(crate) fn feature_collision_shape_full_block<L: WorldGenLevel>(
    level: &L,
    pos: BlockPos,
    state: i32,
) -> bool {
    level.collision_shape_full_block_at(state, pos)
}

pub(crate) fn feature_multiface_can_attach_to<L: WorldGenLevel>(
    _level: &L,
    neighbour_pos: BlockPos,
    neighbour_state: i32,
    direction_towards_neighbour: Direction,
) -> bool {
    multiface_can_attach_to_at(neighbour_state, neighbour_pos, direction_towards_neighbour)
}
