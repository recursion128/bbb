//! The block-owned part of Java 26.1 chunk upgrade data.

use crate::chunk::upgrade_data::ChunkUpgradeLevel;

use crate::fluid::with_property;
use bbb_protocol::block_types::{
    vanilla_block_type, VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_ATTACHED_MELON_STEM_ID,
    VANILLA_BLOCK_ATTACHED_PUMPKIN_STEM_ID, VANILLA_BLOCK_CHEST_ID, VANILLA_BLOCK_MELON_ID,
    VANILLA_BLOCK_MELON_STEM_ID, VANILLA_BLOCK_PUMPKIN_ID, VANILLA_BLOCK_PUMPKIN_STEM_ID,
    VANILLA_BLOCK_TRAPPED_CHEST_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_protocol::registry_data_types::VanillaFluidIdentity;
use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType, SavedTick};
use bbb_server_types::directions::relative;
use bbb_server_types::fluid::block_state_fluid_state;

use crate::block_behavior::shape_context::BlockShapeCtx;
use crate::block_behavior::support::{default_block_state_id, direction_property, string_property};
use crate::block_behavior::update_block_shape;
use bbb_server_types::block_queries::{
    block_type_of, direction_serialized_name, int_property, DIRECTION_VALUES,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpgradeSide {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl UpgradeSide {
    const ALL: [Self; 8] = [
        Self::North,
        Self::NorthEast,
        Self::East,
        Self::SouthEast,
        Self::South,
        Self::SouthWest,
        Self::West,
        Self::NorthWest,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UpgradeSection {
    /// Absolute section Y, not the index in a particular chunk implementation.
    pub section_y: i32,
    /// Vanilla's packed `x | z << 4 | y << 8` indices.
    pub indices: Vec<i32>,
}

/// Persistence-neutral input translated from Java 26.1 `UpgradeData`.
#[derive(Debug, Clone)]
pub(crate) struct BlockUpgradeData {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub min_y: i32,
    pub max_y: i32,
    pub inside: Vec<UpgradeSection>,
    pub sides: [bool; 8],
    pub neighbor_block_ticks: Vec<SavedTick<BlockTickType>>,
    pub neighbor_fluid_ticks: Vec<SavedTick<FluidTickType>>,
}

pub(crate) fn execute_upgrade_data(level: &mut ChunkUpgradeLevel<'_>, data: &BlockUpgradeData) {
    upgrade_inside(level, data);
    for (index, side) in UpgradeSide::ALL.into_iter().enumerate() {
        if data.sides[index] {
            upgrade_side(level, data, side);
        }
    }
    replay_ticks(level, data);
}

fn upgrade_inside(level: &mut ChunkUpgradeLevel<'_>, data: &BlockUpgradeData) {
    let min_x = data.chunk_x.wrapping_mul(16);
    let min_z = data.chunk_z.wrapping_mul(16);
    for section in &data.inside {
        let bottom_y = section.section_y.wrapping_mul(16);
        for &coordinate in &section.indices {
            let pos = BlockPos {
                x: min_x.wrapping_add(coordinate & 15),
                y: bottom_y.wrapping_add(coordinate >> 8 & 15),
                z: min_z.wrapping_add(coordinate >> 4 & 15),
            };
            update_position(level, pos);
        }
    }
}

fn upgrade_side(level: &mut ChunkUpgradeLevel<'_>, data: &BlockUpgradeData, side: UpgradeSide) {
    let (min_lx, max_lx, min_lz, max_lz) = match side {
        UpgradeSide::North => (1, 14, 0, 0),
        UpgradeSide::NorthEast => (15, 15, 0, 0),
        UpgradeSide::East => (15, 15, 1, 14),
        UpgradeSide::SouthEast => (15, 15, 15, 15),
        UpgradeSide::South => (1, 14, 15, 15),
        UpgradeSide::SouthWest => (0, 0, 15, 15),
        UpgradeSide::West => (0, 0, 1, 14),
        UpgradeSide::NorthWest => (0, 0, 0, 0),
    };
    let min_x = data.chunk_x.wrapping_mul(16);
    let min_z = data.chunk_z.wrapping_mul(16);
    for z in min_lz..=max_lz {
        for y in data.min_y..=data.max_y {
            for x in min_lx..=max_lx {
                update_position(
                    level,
                    BlockPos {
                        x: min_x.wrapping_add(x),
                        y,
                        z: min_z.wrapping_add(z),
                    },
                );
            }
        }
    }
}

fn update_position(level: &mut ChunkUpgradeLevel<'_>, pos: BlockPos) {
    let Some(old) = read_state(level, pos) else {
        return;
    };
    let mut current = old;
    for direction in DIRECTION_VALUES {
        let neighbor_pos = relative(pos, direction);
        if let Some(neighbor) = read_state(level, neighbor_pos) {
            current = update_state(level, current, direction, neighbor, pos, neighbor_pos);
        }
    }
    level.replace_state(pos, current);
}

fn update_state(
    level: &mut ChunkUpgradeLevel<'_>,
    state: i32,
    direction: Direction,
    neighbor: i32,
    pos: BlockPos,
    neighbor_pos: BlockPos,
) -> i32 {
    let updated = match fixer(state) {
        None | Some(Fixer::Blacklist) => None,
        Some(Fixer::Chest) => update_chest(level, state, direction, neighbor, pos, neighbor_pos),
        Some(Fixer::Leaves) => {
            update_block_shape(
                &mut BlockShapeCtx::Upgrade(level),
                state,
                pos,
                direction,
                neighbor_pos,
                neighbor,
            );
            None
        }
        Some(Fixer::Stem) => update_stem(state, direction, neighbor),
        Some(Fixer::Default) => Some(update_block_shape(
            &mut BlockShapeCtx::Upgrade(level),
            state,
            pos,
            direction,
            neighbor_pos,
            neighbor,
        )),
    };
    updated.unwrap_or(state)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fixer {
    Blacklist,
    Default,
    Chest,
    Leaves,
    Stem,
}

fn fixer(state: i32) -> Option<Fixer> {
    let block = vanilla_block_type(block_type_of(state)?)?;
    if is_blacklisted(block.resource_id) {
        Some(Fixer::Blacklist)
    } else if matches!(
        block.id,
        VANILLA_BLOCK_CHEST_ID | VANILLA_BLOCK_TRAPPED_CHEST_ID
    ) {
        Some(Fixer::Chest)
    } else if matches!(
        block.resource_id,
        "minecraft:acacia_leaves"
            | "minecraft:cherry_leaves"
            | "minecraft:birch_leaves"
            | "minecraft:pale_oak_leaves"
            | "minecraft:dark_oak_leaves"
            | "minecraft:jungle_leaves"
            | "minecraft:oak_leaves"
            | "minecraft:spruce_leaves"
    ) {
        Some(Fixer::Leaves)
    } else if matches!(
        block.id,
        VANILLA_BLOCK_MELON_STEM_ID | VANILLA_BLOCK_PUMPKIN_STEM_ID
    ) {
        Some(Fixer::Stem)
    } else {
        Some(Fixer::Default)
    }
}

fn is_blacklisted(resource_id: &str) -> bool {
    matches!(
        resource_id,
        "minecraft:observer"
            | "minecraft:nether_portal"
            | "minecraft:white_concrete_powder"
            | "minecraft:orange_concrete_powder"
            | "minecraft:magenta_concrete_powder"
            | "minecraft:light_blue_concrete_powder"
            | "minecraft:yellow_concrete_powder"
            | "minecraft:lime_concrete_powder"
            | "minecraft:pink_concrete_powder"
            | "minecraft:gray_concrete_powder"
            | "minecraft:light_gray_concrete_powder"
            | "minecraft:cyan_concrete_powder"
            | "minecraft:purple_concrete_powder"
            | "minecraft:blue_concrete_powder"
            | "minecraft:brown_concrete_powder"
            | "minecraft:green_concrete_powder"
            | "minecraft:red_concrete_powder"
            | "minecraft:black_concrete_powder"
            | "minecraft:anvil"
            | "minecraft:chipped_anvil"
            | "minecraft:damaged_anvil"
            | "minecraft:dragon_egg"
            | "minecraft:gravel"
            | "minecraft:sand"
            | "minecraft:red_sand"
            | "minecraft:oak_sign"
            | "minecraft:spruce_sign"
            | "minecraft:birch_sign"
            | "minecraft:acacia_sign"
            | "minecraft:cherry_sign"
            | "minecraft:jungle_sign"
            | "minecraft:dark_oak_sign"
            | "minecraft:pale_oak_sign"
            | "minecraft:oak_wall_sign"
            | "minecraft:spruce_wall_sign"
            | "minecraft:birch_wall_sign"
            | "minecraft:acacia_wall_sign"
            | "minecraft:jungle_wall_sign"
            | "minecraft:dark_oak_wall_sign"
            | "minecraft:pale_oak_wall_sign"
            | "minecraft:oak_hanging_sign"
            | "minecraft:spruce_hanging_sign"
            | "minecraft:birch_hanging_sign"
            | "minecraft:acacia_hanging_sign"
            | "minecraft:jungle_hanging_sign"
            | "minecraft:dark_oak_hanging_sign"
            | "minecraft:pale_oak_hanging_sign"
            | "minecraft:oak_wall_hanging_sign"
            | "minecraft:spruce_wall_hanging_sign"
            | "minecraft:birch_wall_hanging_sign"
            | "minecraft:acacia_wall_hanging_sign"
            | "minecraft:jungle_wall_hanging_sign"
            | "minecraft:dark_oak_wall_hanging_sign"
            | "minecraft:pale_oak_wall_hanging_sign"
    )
}

fn update_chest(
    level: &mut ChunkUpgradeLevel<'_>,
    state: i32,
    direction: Direction,
    neighbor: i32,
    pos: BlockPos,
    neighbor_pos: BlockPos,
) -> Option<i32> {
    if block_type_of(state) != block_type_of(neighbor)
        || !is_horizontal(direction)
        || string_property(state, "type")?.as_str() != "single"
        || string_property(neighbor, "type")?.as_str() != "single"
    {
        return Some(state);
    }
    let facing = direction_property(state, "facing")?;
    if axis(direction) == axis(facing) || direction_property(neighbor, "facing")? != facing {
        return Some(state);
    }
    let new_type = if direction == clockwise(facing)? {
        "left"
    } else {
        "right"
    };
    let opposite_type = if new_type == "left" { "right" } else { "left" };
    let neighbor = with_property(neighbor, "type", opposite_type)?;
    level.replace_state(neighbor_pos, neighbor);
    if matches!(facing, Direction::North | Direction::East) {
        level.swap_chest_contents(pos, neighbor_pos);
    }
    with_property(state, "type", new_type)
}

fn update_stem(state: i32, direction: Direction, neighbor: i32) -> Option<i32> {
    if int_property(state, "age")? != 7 {
        return Some(state);
    }
    let state_type = block_type_of(state)?;
    let (fruit, attached) = if state_type == VANILLA_BLOCK_PUMPKIN_STEM_ID {
        (
            VANILLA_BLOCK_PUMPKIN_ID,
            VANILLA_BLOCK_ATTACHED_PUMPKIN_STEM_ID,
        )
    } else {
        (VANILLA_BLOCK_MELON_ID, VANILLA_BLOCK_ATTACHED_MELON_STEM_ID)
    };
    if block_type_of(neighbor)? != fruit {
        return Some(state);
    }
    with_property(
        default_block_state_id(attached),
        "facing",
        direction_serialized_name(direction),
    )
}

fn replay_ticks(level: &mut ChunkUpgradeLevel<'_>, data: &BlockUpgradeData) {
    for &tick in &data.neighbor_block_ticks {
        let tick_type = if tick.tick_type.block_type_id() == VANILLA_BLOCK_AIR_ID {
            read_state(level, tick.pos).and_then(BlockTickType::of_block_state)
        } else {
            Some(tick.tick_type)
        };
        if let Some(tick_type) = tick_type {
            level.append_block_tick(SavedTick { tick_type, ..tick });
        }
    }
    for &tick in &data.neighbor_fluid_ticks {
        let tick_type = if tick.tick_type.identity() == VanillaFluidIdentity::Empty {
            read_state(level, tick.pos).map(|state| block_state_fluid_state(state).tick_type())
        } else {
            Some(tick.tick_type)
        };
        if let Some(tick_type) = tick_type {
            level.append_fluid_tick(SavedTick { tick_type, ..tick });
        }
    }
}

fn read_state(level: &ChunkUpgradeLevel<'_>, pos: BlockPos) -> Option<i32> {
    let state = level.state_at(pos)?;
    block_type_of(state)?;
    Some(state)
}

fn is_horizontal(direction: Direction) -> bool {
    matches!(
        direction,
        Direction::North | Direction::South | Direction::West | Direction::East
    )
}

fn axis(direction: Direction) -> u8 {
    match direction {
        Direction::West | Direction::East => 0,
        Direction::Down | Direction::Up => 1,
        Direction::North | Direction::South => 2,
    }
}

fn clockwise(direction: Direction) -> Option<Direction> {
    match direction {
        Direction::North => Some(Direction::East),
        Direction::East => Some(Direction::South),
        Direction::South => Some(Direction::West),
        Direction::West => Some(Direction::North),
        Direction::Down | Direction::Up => None,
    }
}
