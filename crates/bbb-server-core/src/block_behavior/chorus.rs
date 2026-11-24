//! `ChorusFlowerBlock` and `ChorusPlantBlock`
//! (`net/minecraft/world/level/block/ChorusFlowerBlock.java`,
//! `ChorusPlantBlock.java`).
//!

use super::BlockTags;
use super::{block_tag_matches, BlockGetter, BlockShapeCtx, LevelCtx};

use bbb_protocol::block_types::{
    VanillaCanSurviveOverride, VanillaGetStateForPlacementOverride, VanillaRandomTickOverride,
    VANILLA_BLOCK_CHORUS_FLOWER_ID, VANILLA_BLOCK_CHORUS_PLANT_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::HORIZONTAL;
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::block_state_is_air;

use super::{
    block_type_of, can_survive_override, default_block_state_id, get_state_for_placement_override,
    int_property, random_tick_override, state_is_block, with_property, BlockBehaviorOwner,
    BlockStateAt, DIRECTION_VALUES, SET_BLOCK_UPDATE_CLIENTS,
};

const SUPPORTS_CHORUS_FLOWER_TAG: &str = "minecraft:supports_chorus_flower";
const SUPPORTS_CHORUS_PLANT_TAG: &str = "minecraft:supports_chorus_plant";

/// `ChorusFlowerBlock.DEAD_AGE` (`ChorusFlowerBlock.java:30`).
pub const DEAD_AGE: i32 = 5;

/// `ChorusFlowerBlock.AGE` (`ChorusFlowerBlock.java:31`).
const AGE: &str = "age";

/// The delay `updateShape` arms (`ChorusFlowerBlock.java:160`,
/// `ChorusPlantBlock.java:73`).
pub const TICK_DELAY: i32 = 1;

/// `ChorusFlowerBlock.placeGrownFlower` (`ChorusFlowerBlock.java:130`).
pub const LEVEL_EVENT_CHORUS_GROW: i32 = 1033;

/// `ChorusFlowerBlock.placeDeadFlower` (`ChorusFlowerBlock.java:135`).
pub const LEVEL_EVENT_CHORUS_DEATH: i32 = 1034;

fn face_property(direction: Direction) -> &'static str {
    match direction {
        Direction::Down => "down",
        Direction::Up => "up",
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
    }
}

/// Whether a stored state's behaviour belongs to this family.
#[must_use]
pub fn owner(block_state_id: i32) -> Option<BlockBehaviorOwner> {
    let block_type_id = block_type_of(block_state_id)?;
    if random_tick_override(block_type_id) == Some(VanillaRandomTickOverride::ChorusFlowerBlock)
        || can_survive_override(block_type_id) == Some(VanillaCanSurviveOverride::ChorusFlowerBlock)
    {
        return Some(BlockBehaviorOwner::ChorusFlower);
    }
    if can_survive_override(block_type_id) == Some(VanillaCanSurviveOverride::ChorusPlantBlock) {
        return Some(BlockBehaviorOwner::ChorusPlant);
    }
    None
}

pub(crate) fn plant_state_with_connections(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<i32> {
    let plant = default_block_state_id(VANILLA_BLOCK_CHORUS_PLANT_ID);
    let mut state = plant;
    for direction in DIRECTION_VALUES {
        let neighbor = level.block_state(relative(pos, direction));
        let connect = match direction {
            Direction::Down => {
                state_is_block(neighbor, VANILLA_BLOCK_CHORUS_PLANT_ID)
                    || state_is_block(neighbor, VANILLA_BLOCK_CHORUS_FLOWER_ID)
                    || level.block_state_in_tag(neighbor, SUPPORTS_CHORUS_PLANT_TAG)
            }
            Direction::Up => {
                state_is_block(neighbor, VANILLA_BLOCK_CHORUS_PLANT_ID)
                    || state_is_block(neighbor, VANILLA_BLOCK_CHORUS_FLOWER_ID)
            }
            Direction::North | Direction::East | Direction::South | Direction::West => {
                state_is_block(neighbor, VANILLA_BLOCK_CHORUS_PLANT_ID)
                    || state_is_block(neighbor, VANILLA_BLOCK_CHORUS_FLOWER_ID)
            }
        };
        if let Some(updated) = with_property(state, face_property(direction), bool_str(connect)) {
            state = updated;
        }
    }
    Some(state)
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::ChorusPlantBlock)
    {
        return None;
    }

    let directions = [
        Direction::Down,
        Direction::Up,
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ];
    let neighbors = directions.map(|direction| {
        let neighbor = block_state_at(relative(pos, direction));
        (direction, neighbor)
    });
    for (_, neighbor) in neighbors {
        block_type_of(neighbor)?;
    }

    let mut state = default_block_state_id(block_type_id);
    if block_type_of(state)? != block_type_id {
        return None;
    }
    for (direction, neighbor) in neighbors {
        let connect = state_is_block(neighbor, VANILLA_BLOCK_CHORUS_PLANT_ID)
            || state_is_block(neighbor, VANILLA_BLOCK_CHORUS_FLOWER_ID)
            || direction == Direction::Down
                && tags.block_in_tag(neighbor, "minecraft:supports_chorus_plant");
        state = with_property(state, face_property(direction), bool_str(connect))?;
    }

    placement_can_survive(pos, tags, block_state_at)?.then_some(state)
}

fn placement_can_survive(
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> Option<bool> {
    let below = block_state_at(below_pos(pos));
    block_type_of(below)?;
    let above = block_state_at(above_pos(pos));
    block_type_of(above)?;
    let blocked_above_or_below = !block_state_is_air(above) && !block_state_is_air(below);

    for &direction in &HORIZONTAL {
        let neighbor_pos = relative(pos, direction);
        let neighbor = block_state_at(neighbor_pos);
        block_type_of(neighbor)?;
        if state_is_block(neighbor, VANILLA_BLOCK_CHORUS_PLANT_ID) {
            if blocked_above_or_below {
                return Some(false);
            }
            let neighbor_below = block_state_at(below_pos(neighbor_pos));
            block_type_of(neighbor_below)?;
            if state_is_block(neighbor_below, VANILLA_BLOCK_CHORUS_PLANT_ID)
                || tags.block_in_tag(neighbor_below, "minecraft:supports_chorus_plant")
            {
                return Some(true);
            }
        }
    }

    Some(
        state_is_block(below, VANILLA_BLOCK_CHORUS_PLANT_ID)
            || tags.block_in_tag(below, "minecraft:supports_chorus_plant"),
    )
}

fn bool_str(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn flower_with_age(age: i32) -> Option<i32> {
    let base = default_block_state_id(VANILLA_BLOCK_CHORUS_FLOWER_ID);
    with_property(base, AGE, &age.to_string())
}

fn all_neighbors_empty(level: &LevelCtx<'_, '_>, pos: BlockPos, ignore: Option<Direction>) -> bool {
    HORIZONTAL.iter().all(|&direction| {
        Some(direction) == ignore || block_state_is_air(level.block_state(relative(pos, direction)))
    })
}

pub fn flower_can_survive(
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> bool {
    flower_survives(pos, block_state_at, |state| {
        tags.block_in_tag(state, "minecraft:supports_chorus_flower")
    })
}

pub fn plant_can_survive(
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> bool {
    plant_survives(pos, block_state_at, |state| {
        tags.block_in_tag(state, "minecraft:supports_chorus_plant")
    })
}

fn flower_can_survive_at<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> bool {
    flower_survives(
        pos,
        |at| level.block_state(at),
        |state| block_tag_matches(state, SUPPORTS_CHORUS_FLOWER_TAG, membership),
    )
}

fn flower_survives(
    pos: BlockPos,
    mut block_state_at: impl FnMut(BlockPos) -> i32,
    mut supports: impl FnMut(i32) -> bool,
) -> bool {
    let below = block_state_at(below_pos(pos));
    if state_is_block(below, VANILLA_BLOCK_CHORUS_PLANT_ID) {
        return true;
    }
    let ordinary_support = if !block_state_is_air(below) {
        false
    } else {
        let mut one_neighbor = false;
        let mut valid = true;
        for &direction in &HORIZONTAL {
            let neighbor = block_state_at(relative(pos, direction));
            if state_is_block(neighbor, VANILLA_BLOCK_CHORUS_PLANT_ID) {
                if one_neighbor {
                    valid = false;
                    break;
                }
                one_neighbor = true;
            } else if !block_state_is_air(neighbor) {
                valid = false;
                break;
            }
        }
        valid && one_neighbor
    };
    ordinary_support || supports(below)
}

fn plant_can_survive_at<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> bool {
    plant_survives(
        pos,
        |at| level.block_state(at),
        |state| block_tag_matches(state, SUPPORTS_CHORUS_PLANT_TAG, membership),
    )
}

fn plant_survives(
    pos: BlockPos,
    mut block_state_at: impl FnMut(BlockPos) -> i32,
    mut supports: impl FnMut(i32) -> bool,
) -> bool {
    let below = block_state_at(below_pos(pos));
    let above = block_state_at(above_pos(pos));
    let blocked_above_or_below = !block_state_is_air(above) && !block_state_is_air(below);
    for &direction in &HORIZONTAL {
        let neighbor_pos = relative(pos, direction);
        let neighbor = block_state_at(neighbor_pos);
        if state_is_block(neighbor, VANILLA_BLOCK_CHORUS_PLANT_ID) {
            if blocked_above_or_below {
                return false;
            }
            let neighbor_below = block_state_at(below_pos(neighbor_pos));
            if state_is_block(neighbor_below, VANILLA_BLOCK_CHORUS_PLANT_ID)
                || supports(neighbor_below)
            {
                return true;
            }
        }
    }
    state_is_block(below, VANILLA_BLOCK_CHORUS_PLANT_ID) || supports(below)
}

fn below_pos(pos: BlockPos) -> BlockPos {
    relative(pos, Direction::Down)
}

fn above_pos(pos: BlockPos) -> BlockPos {
    relative(pos, Direction::Up)
}

pub(super) fn flower_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let _ = block_state_id;
    if !flower_can_survive_at(level, pos, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    }) {
        level.destroy_block(pos, true);
    }
}

pub(super) fn plant_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let _ = block_state_id;
    if !plant_can_survive_at(level, pos, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    }) {
        level.destroy_block(pos, true);
    }
}

pub(super) fn flower_update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    if direction_to_neighbour != Direction::Up
        && !flower_can_survive_at(level, pos, &|registry, tag, id| {
            level.registry_tag_membership(registry, tag, id)
        })
    {
        if let Some(block) = BlockTickType::of_block_state(block_state_id) {
            level.schedule_block_tick(pos, block, TICK_DELAY, TickPriority::Normal);
        }
    }
    block_state_id
}

pub(super) fn plant_update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    let survives = plant_can_survive_at(level, pos, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    });
    if !survives {
        if let Some(block) = BlockTickType::of_block_state(block_state_id) {
            level.schedule_block_tick(pos, block, TICK_DELAY, TickPriority::Normal);
        }
        return block_state_id;
    }
    let mut connect = state_is_block(neighbour_state, VANILLA_BLOCK_CHORUS_PLANT_ID)
        || state_is_block(neighbour_state, VANILLA_BLOCK_CHORUS_FLOWER_ID);
    if !connect && direction_to_neighbour == Direction::Down {
        let tagged = level.block_state_in_tag(neighbour_state, SUPPORTS_CHORUS_PLANT_TAG);
        connect = tagged;
    }
    with_property(
        block_state_id,
        face_property(direction_to_neighbour),
        bool_str(connect),
    )
    .unwrap_or(block_state_id)
}

fn place_grown_flower(level: &mut LevelCtx<'_, '_>, pos: BlockPos, age: i32) {
    if let Some(state) = flower_with_age(age) {
        let _ = level.set_block(
            pos,
            state,
            SET_BLOCK_UPDATE_CLIENTS,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        level.level_event(LEVEL_EVENT_CHORUS_GROW, pos, 0);
    }
}

fn place_dead_flower(level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    if let Some(state) = flower_with_age(DEAD_AGE) {
        let _ = level.set_block(
            pos,
            state,
            SET_BLOCK_UPDATE_CLIENTS,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        level.level_event(LEVEL_EVENT_CHORUS_DEATH, pos, 0);
    }
}

pub(super) fn flower_random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let above = above_pos(pos);
    if !block_state_is_air(level.block_state(above)) || above.y > level.get_max_y() {
        return;
    }
    let Some(current_age) = int_property(block_state_id, AGE) else {
        return;
    };
    if current_age >= DEAD_AGE {
        return;
    }
    let below = level.block_state(below_pos(pos));
    let mut grow_upwards = false;
    let mut pillar_on_support = false;
    let on_support = level.block_state_in_tag(below, SUPPORTS_CHORUS_FLOWER_TAG);
    if on_support {
        grow_upwards = true;
    } else if state_is_block(below, VANILLA_BLOCK_CHORUS_PLANT_ID) {
        let mut height = 1;
        for _ in 0..4 {
            let test_pos = BlockPos {
                x: pos.x,
                y: pos.y - (height + 1),
                z: pos.z,
            };
            let test = level.block_state(test_pos);
            if !state_is_block(test, VANILLA_BLOCK_CHORUS_PLANT_ID) {
                let on_support = level.block_state_in_tag(test, SUPPORTS_CHORUS_FLOWER_TAG);
                if on_support {
                    pillar_on_support = true;
                }
                break;
            }
            height += 1;
        }
        let growth_limit = if pillar_on_support { 5 } else { 4 };
        if height < 2 || height <= level.level_random().next_int(growth_limit) {
            grow_upwards = true;
        }
    } else if block_state_is_air(below) {
        grow_upwards = true;
    }

    if grow_upwards
        && all_neighbors_empty(level, above, None)
        && block_state_is_air(level.block_state(BlockPos {
            x: pos.x,
            y: pos.y + 2,
            z: pos.z,
        }))
    {
        let Some(plant) = plant_state_with_connections(level, pos) else {
            return;
        };
        let _ = level.set_block(
            pos,
            plant,
            SET_BLOCK_UPDATE_CLIENTS,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        place_grown_flower(level, above, current_age);
    } else if current_age < 4 {
        let mut attempts = level.level_random().next_int(4);
        if pillar_on_support {
            attempts += 1;
        }
        let mut created_branch = false;
        for _ in 0..attempts {
            let direction = HORIZONTAL[level.level_random().next_int(4) as usize];
            let target = relative(pos, direction);
            if block_state_is_air(level.block_state(target))
                && block_state_is_air(level.block_state(below_pos(target)))
                && all_neighbors_empty(level, target, Some(opposite(direction)))
            {
                place_grown_flower(level, target, current_age + 1);
                created_branch = true;
            }
        }
        if created_branch {
            let Some(plant) = plant_state_with_connections(level, pos) else {
                return;
            };
            let _ = level.set_block(
                pos,
                plant,
                SET_BLOCK_UPDATE_CLIENTS,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        } else {
            place_dead_flower(level, pos);
        }
    } else {
        place_dead_flower(level, pos);
    }
}
