//! `BellBlock` (`BellBlock.java`) identity, POWERED, hit geometry and
//! survival.

use super::support::{can_support_center_at_tagged, can_support_center_tagged};
use super::{BlockShapeCtx, LevelCtx};
use crate::block_getter::{BlockGetter, BlockTags};

use bbb_protocol::block_types::{
    vanilla_block_type, VanillaBlockUseWithoutItemOverride, VanillaGetStateForPlacementOverride,
};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{above, with_property, SET_BLOCK_UPDATE_ALL};
use bbb_server_types::directions::{opposite, relative};

use super::{
    air_block_state_id, bool_property, face_sturdy, face_sturdy_at, BlockStateAt, SupportType,
};
use super::{
    block_type_of, default_block_state_id, direction_serialized_name,
    get_state_for_placement_override,
};

const ATTACHMENT: &str = "attachment";
const FACING: &str = "facing";
const POWERED: &str = "powered";
pub const RING_SOUND: &str = "minecraft:block.bell.use";
/// `BellBlock` click-Y gate (`BellBlock.java:116`).
pub const PROPER_HIT_Y: f32 = 0.8124;

/// `BellBlock.useWithoutItem` is unique to that class.
#[must_use]
pub fn is_bell(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.use_without_item_override)
        == Some(VanillaBlockUseWithoutItemOverride::BellBlock)
}

#[must_use]
pub fn powered(block_state_id: i32) -> bool {
    bool_property(block_state_id, POWERED).unwrap_or(false)
}

#[must_use]
pub fn facing(block_state_id: i32) -> Direction {
    match bbb_protocol::block_state_types::vanilla_block_state_property(block_state_id, "facing") {
        Some("south") => Direction::South,
        Some("west") => Direction::West,
        Some("east") => Direction::East,
        _ => Direction::North,
    }
}

#[must_use]
pub fn attachment(block_state_id: i32) -> &'static str {
    bbb_protocol::block_state_types::vanilla_block_state_property(block_state_id, "attachment")
        .unwrap_or("floor")
}

/// `BellBlock.getConnectedDirection` (`:264-272`).
#[must_use]
pub fn connected_direction(block_state_id: i32) -> Direction {
    match attachment(block_state_id) {
        "ceiling" => Direction::Down,
        "single_wall" | "double_wall" => opposite(facing(block_state_id)),
        _ => Direction::Up,
    }
}

/// `BellBlock.isProperHit` (`:115-132`). `click_y` is the local 0–1 cursor.
#[must_use]
pub fn is_proper_hit(block_state_id: i32, clicked: Direction, click_y: f32) -> bool {
    if matches!(clicked, Direction::Up | Direction::Down) || click_y > PROPER_HIT_Y {
        return false;
    }
    let facing = facing(block_state_id);
    match attachment(block_state_id) {
        "floor" => axis(facing) == axis(clicked),
        "single_wall" | "double_wall" => axis(facing) != axis(clicked),
        "ceiling" => true,
        _ => false,
    }
}

fn axis(direction: Direction) -> char {
    match direction {
        Direction::East | Direction::West => 'x',
        Direction::Up | Direction::Down => 'y',
        Direction::North | Direction::South => 'z',
    }
}

/// `BellBlock.canSurvive` (`:256-261`).
#[must_use]
pub fn can_survive(
    block_state_id: i32,
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> Option<bool> {
    let connection = opposite(connected_direction(block_state_id));
    if connection == Direction::Up {
        return can_support_center_tagged(tags, block_state_at(above(pos)), Direction::Down);
    }
    let support = relative(pos, connection);
    face_sturdy(
        block_state_at(support),
        opposite(connection),
        SupportType::Full,
    )
}

pub(super) fn can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
) -> Option<bool> {
    let connection = opposite(connected_direction(block_state_id));
    if connection == Direction::Up {
        let support_pos = above(pos);
        return Some(can_support_center_at_tagged(
            level.tags(),
            level.block_state(support_pos),
            support_pos,
            Direction::Down,
        ));
    }
    let support_pos = relative(pos, connection);
    Some(face_sturdy_at(
        level.block_state(support_pos),
        support_pos,
        opposite(connection),
        SupportType::Full,
    ))
}

fn sturdy_face(
    pos: BlockPos,
    direction: Direction,
    block_state_at: BlockStateAt<'_>,
) -> Option<bool> {
    face_sturdy(
        block_state_at(relative(pos, direction)),
        opposite(direction),
        SupportType::Full,
    )
}

/// `BellBlock.getStateForPlacement` (`BellBlock.java`).
pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    clicked_face: Direction,
    horizontal_direction: Direction,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::BellBlock)
    {
        return None;
    }
    let default_state = default_block_state_id(block_type_id);
    if matches!(clicked_face, Direction::Down | Direction::Up) {
        if matches!(horizontal_direction, Direction::Down | Direction::Up) {
            return None;
        }
        let attachment = if clicked_face == Direction::Down {
            "ceiling"
        } else {
            "floor"
        };
        let state = with_property(default_state, ATTACHMENT, attachment)?;
        let state = with_property(
            state,
            FACING,
            direction_serialized_name(horizontal_direction),
        )?;
        return can_survive(state, pos, tags, &mut *block_state_at)?.then_some(state);
    }

    let double_attached = match clicked_face {
        Direction::West | Direction::East => {
            sturdy_face(pos, Direction::West, &mut *block_state_at)?
                && sturdy_face(pos, Direction::East, &mut *block_state_at)?
        }
        Direction::North | Direction::South => {
            sturdy_face(pos, Direction::North, &mut *block_state_at)?
                && sturdy_face(pos, Direction::South, &mut *block_state_at)?
        }
        Direction::Down | Direction::Up => unreachable!(),
    };
    let state = with_property(
        default_state,
        FACING,
        direction_serialized_name(opposite(clicked_face)),
    )?;
    let state = with_property(
        state,
        ATTACHMENT,
        if double_attached {
            "double_wall"
        } else {
            "single_wall"
        },
    )?;
    match can_survive(state, pos, tags, &mut *block_state_at) {
        Some(true) => return Some(state),
        Some(false) => {}
        None => return None,
    }

    let can_attach_below = sturdy_face(pos, Direction::Down, &mut *block_state_at)?;
    let state = with_property(
        state,
        ATTACHMENT,
        if can_attach_below { "floor" } else { "ceiling" },
    )?;
    can_survive(state, pos, tags, block_state_at)?.then_some(state)
}

/// `BellBlock.updateShape` (`BellBlock.java:225-253`).
pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    let current_attachment = attachment(block_state_id);
    let connected_dir = opposite(connected_direction(block_state_id));
    if connected_dir == direction_to_neighbour
        && can_survive_live(level, block_state_id, pos) != Some(true)
        && current_attachment != "double_wall"
    {
        return air_block_state_id();
    }
    let bell_facing = facing(block_state_id);
    if axis(direction_to_neighbour) == axis(bell_facing) {
        let neighbour_pos = relative(pos, direction_to_neighbour);
        if current_attachment == "double_wall"
            && !face_sturdy_at(
                neighbour_state,
                neighbour_pos,
                direction_to_neighbour,
                SupportType::Full,
            )
        {
            if let Some(next) =
                with_property(block_state_id, ATTACHMENT, "single_wall").and_then(|state| {
                    with_property(
                        state,
                        FACING,
                        direction_serialized_name(opposite(direction_to_neighbour)),
                    )
                })
            {
                return next;
            }
        }
        if current_attachment == "single_wall"
            && opposite(connected_dir) == direction_to_neighbour
            && face_sturdy_at(
                neighbour_state,
                neighbour_pos,
                bell_facing,
                SupportType::Full,
            )
        {
            if let Some(next) = with_property(block_state_id, ATTACHMENT, "double_wall") {
                return next;
            }
        }
    }
    block_state_id
}

/// `BellBlock.neighborChanged` (`:71-82`).
pub(super) fn neighbor_changed(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    check_powered_state(level, pos, block_state_id);
}

fn check_powered_state(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let signal = level.has_neighbor_signal(pos);
    if powered(block_state_id) == signal {
        return;
    }
    if signal {
        ring(level, pos, block_state_id, facing(block_state_id), None);
    }
    if let Some(next) = with_property(
        block_state_id,
        POWERED,
        if signal { "true" } else { "false" },
    ) {
        level.set_block(
            pos,
            next,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}

/// `BellBlock.onProjectileHit` (`BellBlock.java:85-88`) over `onHit`
/// (`:97-112`): whether a proper hit rang the bell.
pub(super) fn on_projectile_hit(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    direction: Direction,
    click_y: f64,
    ringing_entity_id: Option<i32>,
) -> bool {
    is_proper_hit(block_state_id, direction, click_y as f32)
        && ring(level, pos, block_state_id, direction, ringing_entity_id)
}

/// `BellBlock.attemptToRing` (`BellBlock.java:139-153`); the block event's
/// `BellBlockEntity.triggerEvent` starts the shake.
pub(crate) fn ring(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    direction: Direction,
    source_entity_id: Option<i32>,
) -> bool {
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return false;
    };
    if level
        .chunks()
        .block_entities()
        .get(pos)
        .and_then(crate::block_entity::BlockEntity::bell)
        .is_none()
    {
        return false;
    }
    level.block_event(
        pos,
        block_type_id,
        crate::block_entity::bell::EVENT_BELL_RING,
        crate::block_entity::bell::direction_3d_data_value(direction),
    );
    level.play_block_sound_with_volume_pitch(pos, RING_SOUND, 2.0, 1.0);
    level.game_event_from_entity(
        crate::level_event::VanillaGameEvent::BLOCK_CHANGE,
        pos,
        source_entity_id,
        None,
    );
    true
}
