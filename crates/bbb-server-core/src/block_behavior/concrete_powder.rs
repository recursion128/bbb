use super::{fluid_tag_matches, BlockGetter, BlockShapeCtx, LevelCtx};
use bbb_protocol::block_state_types::VanillaBlockSupportType as SupportType;
use bbb_protocol::block_types::vanilla_concrete_block_type_id_for_powder;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};

use super::{block_type_of, default_block_state_id, face_sturdy, face_sturdy_at, falling};

const WATER_TAG: &str = "minecraft:water";
const LIQUID_DIRECTIONS: [Direction; 6] = [
    Direction::Down,
    Direction::Up,
    Direction::North,
    Direction::South,
    Direction::West,
    Direction::East,
];
#[must_use]
pub(super) fn is_concrete_powder(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_concrete_block_type_id_for_powder)
        .is_some()
}

fn concrete_state(block_state_id: i32) -> Option<i32> {
    block_type_of(block_state_id)
        .and_then(vanilla_concrete_block_type_id_for_powder)
        .map(default_block_state_id)
}

fn can_solidify_for_placement(block_state_id: i32) -> Option<bool> {
    block_type_of(block_state_id)?;
    Some(block_state_fluid_state(block_state_id).family() == Some(FluidFamily::Water))
}

fn touches_liquid_with(
    pos: BlockPos,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
    can_solidify: &mut dyn FnMut(i32) -> Option<bool>,
    is_sturdy: &mut dyn FnMut(i32, BlockPos, Direction) -> Option<bool>,
) -> Option<bool> {
    let mut test_pos = pos;
    for direction in LIQUID_DIRECTIONS {
        let current = block_state_at(test_pos);
        if direction != Direction::Down || can_solidify(current)? {
            test_pos = relative(pos, direction);
            let neighbour = block_state_at(test_pos);
            if can_solidify(neighbour)? && !is_sturdy(neighbour, pos, opposite(direction))? {
                return Some(true);
            }
        }
    }
    Some(false)
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<i32> {
    let powder = default_block_state_id(block_type_id);
    let concrete = concrete_state(powder)?;
    let replaced = block_state_at(pos);
    if can_solidify_for_placement(replaced)? {
        return Some(concrete);
    }
    if touches_liquid_with(
        pos,
        block_state_at,
        &mut can_solidify_for_placement,
        &mut |state, _, face| face_sturdy(state, face, SupportType::Full),
    )? {
        Some(concrete)
    } else {
        Some(powder)
    }
}

fn can_solidify(state: i32, membership: &dyn Fn(&str, &str, &str) -> bool) -> Option<bool> {
    block_type_of(state)?;
    Some(fluid_tag_matches(
        block_state_fluid_state(state).identity(),
        WATER_TAG,
        membership,
    ))
}

fn touches_liquid<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> Option<bool> {
    touches_liquid_with(
        pos,
        &mut |at| level.block_state(at),
        &mut |state| can_solidify(state, membership),
        &mut |state, at, face| Some(face_sturdy_at(state, at, face, SupportType::Full)),
    )
}

pub(crate) fn on_land(
    level: &mut LevelCtx<'_, '_>,
    original_state: i32,
    pos: BlockPos,
    replaced_state: i32,
) -> Option<()> {
    let concrete = concrete_state(original_state)?;
    if can_solidify(replaced_state, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    })? || touches_liquid(level, pos, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    })? {
        // The second setBlock's false result does not undo a successful powder placement.
        let _ = level.set_block(
            pos,
            concrete,
            crate::fluid::SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
    Some(())
}

#[must_use]
pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    let Some(concrete) = concrete_state(block_state_id) else {
        return block_state_id;
    };
    let Some(touches_liquid) = touches_liquid(level, pos, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    }) else {
        return block_state_id;
    };
    if touches_liquid {
        return concrete;
    }
    falling::update_shape(level, block_state_id, pos)
}
