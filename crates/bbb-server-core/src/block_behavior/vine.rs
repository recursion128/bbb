//! `VineBlock` (`net/minecraft/world/level/block/VineBlock.java`).

use super::{BlockShapeCtx, LevelCtx};
use crate::block_getter::BlockGetter;

use crate::fluid::HORIZONTAL;
use bbb_protocol::block_types::VANILLA_BLOCK_VINE_ID;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::block_state_is_air;

use super::{
    air_block_state_id, bool_property, default_block_state_id, multiface_can_attach_to,
    multiface_can_attach_to_at, state_is_block, with_property, BlockStateAt, DIRECTION_VALUES,
    SET_BLOCK_UPDATE_CLIENTS,
};

const UP: &str = "up";

fn is_horizontal(direction: Direction) -> bool {
    matches!(
        direction,
        Direction::North | Direction::South | Direction::West | Direction::East
    )
}

fn face_property(direction: Direction) -> Option<&'static str> {
    match direction {
        Direction::Up => Some(UP),
        Direction::North => Some("north"),
        Direction::South => Some("south"),
        Direction::West => Some("west"),
        Direction::East => Some("east"),
        Direction::Down => None,
    }
}

fn face(state: i32, direction: Direction) -> bool {
    face_property(direction)
        .and_then(|property| bool_property(state, property))
        .unwrap_or(false)
}

fn with_face(state: i32, direction: Direction, value: bool) -> Option<i32> {
    with_property(
        state,
        face_property(direction)?,
        if value { "true" } else { "false" },
    )
}

fn clockwise(direction: Direction) -> Option<Direction> {
    match direction {
        Direction::North => Some(Direction::East),
        Direction::South => Some(Direction::West),
        Direction::West => Some(Direction::North),
        Direction::East => Some(Direction::South),
        Direction::Down | Direction::Up => None,
    }
}

fn counter_clockwise(direction: Direction) -> Option<Direction> {
    match direction {
        Direction::North => Some(Direction::West),
        Direction::South => Some(Direction::East),
        Direction::West => Some(Direction::South),
        Direction::East => Some(Direction::North),
        Direction::Down | Direction::Up => None,
    }
}

fn default_vine() -> i32 {
    default_block_state_id(VANILLA_BLOCK_VINE_ID)
}

fn acceptable_neighbour(
    neighbour_state: i32,
    direction_towards_neighbour: Direction,
) -> Option<bool> {
    multiface_can_attach_to(neighbour_state, direction_towards_neighbour)
}

fn acceptable_at(
    at: &mut dyn FnMut(BlockPos) -> i32,
    neighbour_pos: BlockPos,
    direction_towards_neighbour: Direction,
) -> Option<bool> {
    acceptable_neighbour(at(neighbour_pos), direction_towards_neighbour)
}

fn can_support_at_face(
    pos: BlockPos,
    direction: Direction,
    at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<bool> {
    if direction == Direction::Down {
        return Some(false);
    }
    let acceptable = acceptable_at(at, relative(pos, direction), direction);
    match acceptable {
        Some(true) => return Some(true),
        Some(false) => {}
        None if !is_horizontal(direction) => return None,
        None => {}
    }
    if !is_horizontal(direction) {
        return Some(false);
    }
    let above = at(relative(pos, Direction::Up));
    if state_is_block(above, VANILLA_BLOCK_VINE_ID) && face(above, direction) {
        Some(true)
    } else {
        acceptable
    }
}

pub(super) fn has_vacant_face(block_state_id: i32) -> Option<bool> {
    if !state_is_block(block_state_id, VANILLA_BLOCK_VINE_ID) {
        return None;
    }
    let mut faces = 0;
    for direction in [
        Direction::Up,
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ] {
        faces += i32::from(bool_property(block_state_id, face_property(direction)?)?);
    }
    Some(faces < 5)
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    old_state: i32,
    directions: [Direction; 6],
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    if block_type_id != VANILLA_BLOCK_VINE_ID {
        return None;
    }
    let old_vine = state_is_block(old_state, VANILLA_BLOCK_VINE_ID);
    let state = if old_vine { old_state } else { default_vine() };
    for direction in directions {
        if direction == Direction::Down {
            continue;
        }
        if old_vine && bool_property(old_state, face_property(direction)?)? {
            continue;
        }
        match can_support_at_face(pos, direction, block_state_at) {
            Some(true) => return with_face(state, direction, true),
            Some(false) => {}
            None => return None,
        }
    }
    old_vine.then_some(state)
}

fn updated_state(
    mut state: i32,
    pos: BlockPos,
    at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<i32> {
    if face(state, Direction::Up) {
        state = with_face(
            state,
            Direction::Up,
            acceptable_at(at, relative(pos, Direction::Up), Direction::Down)?,
        )?;
    }
    for direction in HORIZONTAL {
        if face(state, direction) {
            state = with_face(state, direction, can_support_at_face(pos, direction, at)?)?;
        }
    }
    Some(state)
}

fn can_support_at_face_at<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
    direction: Direction,
) -> Option<bool> {
    if direction == Direction::Down {
        return Some(false);
    }
    let neighbour_pos = relative(pos, direction);
    let acceptable = Some(multiface_can_attach_to_at(
        level.block_state(neighbour_pos),
        neighbour_pos,
        direction,
    ));
    match acceptable {
        Some(true) => return Some(true),
        Some(false) => {}
        None if !is_horizontal(direction) => return None,
        None => {}
    }
    if !is_horizontal(direction) {
        return Some(false);
    }
    let above = level.block_state(relative(pos, Direction::Up));
    if state_is_block(above, VANILLA_BLOCK_VINE_ID) && face(above, direction) {
        Some(true)
    } else {
        acceptable
    }
}

fn updated_state_at<L: BlockGetter + ?Sized>(
    level: &L,
    mut state: i32,
    pos: BlockPos,
) -> Option<i32> {
    if face(state, Direction::Up) {
        let support_pos = relative(pos, Direction::Up);
        let supported = Some(multiface_can_attach_to_at(
            level.block_state(support_pos),
            support_pos,
            Direction::Down,
        ))?;
        state = with_face(state, Direction::Up, supported)?;
    }
    for direction in HORIZONTAL {
        if face(state, direction) {
            state = with_face(
                state,
                direction,
                can_support_at_face_at(level, pos, direction)?,
            )?;
        }
    }
    Some(state)
}

fn has_horizontal_connection(state: i32) -> bool {
    HORIZONTAL
        .into_iter()
        .any(|direction| face(state, direction))
}

fn has_faces(state: i32) -> bool {
    face(state, Direction::Up) || has_horizontal_connection(state)
}

fn can_spread(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    let mut remaining = 5;
    for dz in -4..=4 {
        for dy in -1..=1 {
            for dx in -4..=4 {
                let test = BlockPos {
                    x: pos.x.wrapping_add(dx),
                    y: pos.y.wrapping_add(dy),
                    z: pos.z.wrapping_add(dz),
                };
                if state_is_block(level.block_state(test), VANILLA_BLOCK_VINE_ID) {
                    remaining -= 1;
                    if remaining <= 0 {
                        return false;
                    }
                }
            }
        }
    }
    true
}

fn random_direction(level: &mut LevelCtx<'_, '_>) -> Direction {
    let index = level.level_random().next_int(6);
    DIRECTION_VALUES[usize::try_from(index).expect("nextInt answers a non-negative index")]
}

pub(super) fn can_survive(
    block_state_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt<'_>,
) -> Option<bool> {
    if !state_is_block(block_state_id, VANILLA_BLOCK_VINE_ID) {
        return None;
    }
    updated_state(block_state_id, pos, block_state_at).map(has_faces)
}

pub(super) fn can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    mut state: i32,
    pos: BlockPos,
) -> Option<bool> {
    if !state_is_block(state, VANILLA_BLOCK_VINE_ID) {
        return None;
    }
    if face(state, Direction::Up) {
        let support_pos = relative(pos, Direction::Up);
        let supported = multiface_can_attach_to_at(
            level.block_state(support_pos),
            support_pos,
            Direction::Down,
        );
        let next = with_face(state, Direction::Up, supported)?;
        state = next;
    }
    for direction in HORIZONTAL {
        if !face(state, direction) {
            continue;
        }
        let neighbour_pos = relative(pos, direction);
        let direct =
            multiface_can_attach_to_at(level.block_state(neighbour_pos), neighbour_pos, direction);
        let supported = direct || {
            let above = level.block_state(relative(pos, Direction::Up));
            state_is_block(above, VANILLA_BLOCK_VINE_ID) && face(above, direction)
        };
        let next = with_face(state, direction, supported)?;
        state = next;
    }
    Some(has_faces(state))
}

pub(super) fn update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    if direction_to_neighbour == Direction::Down {
        return block_state_id;
    }
    let Some(state) = updated_state_at(level, block_state_id, pos) else {
        return block_state_id;
    };
    if has_faces(state) {
        state
    } else {
        air_block_state_id()
    }
}

pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if !state_is_block(block_state_id, VANILLA_BLOCK_VINE_ID)
        || !level.spread_vines_enabled()
        || level.level_random().next_int(4) != 0
    {
        return;
    }
    let test_direction = random_direction(level);
    let above_pos = relative(pos, Direction::Up);
    if is_horizontal(test_direction) && !face(block_state_id, test_direction) {
        if !can_spread(level, pos) {
            return;
        }
        let test_pos = relative(pos, test_direction);
        let edge_state = level.block_state(test_pos);
        if block_state_is_air(edge_state) {
            let cw = clockwise(test_direction).expect("horizontal direction has a clockwise face");
            let ccw = counter_clockwise(test_direction)
                .expect("horizontal direction has a counter-clockwise face");
            let cw_has_face = face(block_state_id, cw);
            let ccw_has_face = face(block_state_id, ccw);
            let cw_test_pos = relative(test_pos, cw);
            let ccw_test_pos = relative(test_pos, ccw);
            if cw_has_face
                && acceptable_at(&mut |at| level.block_state(at), cw_test_pos, cw) == Some(true)
            {
                if let Some(state) = with_face(default_vine(), cw, true) {
                    level.set_block(
                        test_pos,
                        state,
                        SET_BLOCK_UPDATE_CLIENTS,
                        crate::neighbor_updater::UPDATE_LIMIT,
                    );
                }
            } else if ccw_has_face
                && acceptable_at(&mut |at| level.block_state(at), ccw_test_pos, ccw) == Some(true)
            {
                if let Some(state) = with_face(default_vine(), ccw, true) {
                    level.set_block(
                        test_pos,
                        state,
                        SET_BLOCK_UPDATE_CLIENTS,
                        crate::neighbor_updater::UPDATE_LIMIT,
                    );
                }
            } else {
                let opposite_direction = opposite(test_direction);
                if cw_has_face
                    && block_state_is_air(level.block_state(cw_test_pos))
                    && acceptable_at(
                        &mut |at| level.block_state(at),
                        relative(pos, cw),
                        opposite_direction,
                    ) == Some(true)
                {
                    if let Some(state) = with_face(default_vine(), opposite_direction, true) {
                        level.set_block(
                            cw_test_pos,
                            state,
                            SET_BLOCK_UPDATE_CLIENTS,
                            crate::neighbor_updater::UPDATE_LIMIT,
                        );
                    }
                } else if ccw_has_face
                    && block_state_is_air(level.block_state(ccw_test_pos))
                    && acceptable_at(
                        &mut |at| level.block_state(at),
                        relative(pos, ccw),
                        opposite_direction,
                    ) == Some(true)
                {
                    if let Some(state) = with_face(default_vine(), opposite_direction, true) {
                        level.set_block(
                            ccw_test_pos,
                            state,
                            SET_BLOCK_UPDATE_CLIENTS,
                            crate::neighbor_updater::UPDATE_LIMIT,
                        );
                    }
                } else if level.level_random().next_float() < 0.05
                    && acceptable_at(
                        &mut |at| level.block_state(at),
                        relative(test_pos, Direction::Up),
                        Direction::Up,
                    ) == Some(true)
                {
                    if let Some(state) = with_face(default_vine(), Direction::Up, true) {
                        level.set_block(
                            test_pos,
                            state,
                            SET_BLOCK_UPDATE_CLIENTS,
                            crate::neighbor_updater::UPDATE_LIMIT,
                        );
                    }
                }
            }
        } else if acceptable_neighbour(edge_state, test_direction) == Some(true) {
            if let Some(state) = with_face(block_state_id, test_direction, true) {
                level.set_block(
                    pos,
                    state,
                    SET_BLOCK_UPDATE_CLIENTS,
                    crate::neighbor_updater::UPDATE_LIMIT,
                );
            }
        }
        return;
    }

    if test_direction == Direction::Up && pos.y < level.get_max_y() {
        if can_support_at_face(pos, test_direction, &mut |at| level.block_state(at)) == Some(true) {
            if let Some(state) = with_face(block_state_id, Direction::Up, true) {
                level.set_block(
                    pos,
                    state,
                    SET_BLOCK_UPDATE_CLIENTS,
                    crate::neighbor_updater::UPDATE_LIMIT,
                );
            }
            return;
        }
        if block_state_is_air(level.block_state(above_pos)) {
            if !can_spread(level, pos) {
                return;
            }
            let mut above_state = block_state_id;
            for direction in HORIZONTAL {
                if level.level_random().next_boolean()
                    || acceptable_at(
                        &mut |at| level.block_state(at),
                        relative(above_pos, direction),
                        direction,
                    ) != Some(true)
                {
                    let Some(updated) = with_face(above_state, direction, false) else {
                        return;
                    };
                    above_state = updated;
                }
            }
            if has_horizontal_connection(above_state) {
                level.set_block(
                    above_pos,
                    above_state,
                    SET_BLOCK_UPDATE_CLIENTS,
                    crate::neighbor_updater::UPDATE_LIMIT,
                );
            }
            return;
        }
    }

    if pos.y <= level.get_min_y() {
        return;
    }
    let below_pos = relative(pos, Direction::Down);
    let below_state = level.block_state(below_pos);
    if !block_state_is_air(below_state) && !state_is_block(below_state, VANILLA_BLOCK_VINE_ID) {
        return;
    }
    let before = if block_state_is_air(below_state) {
        default_vine()
    } else {
        below_state
    };
    let mut after = before;
    for direction in HORIZONTAL {
        if level.level_random().next_boolean() && face(block_state_id, direction) {
            let Some(updated) = with_face(after, direction, true) else {
                return;
            };
            after = updated;
        }
    }
    if before != after && has_horizontal_connection(after) {
        level.set_block(
            below_pos,
            after,
            SET_BLOCK_UPDATE_CLIENTS,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}
