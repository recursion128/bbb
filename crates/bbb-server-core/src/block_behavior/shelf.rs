//! `ShelfBlock` (`ShelfBlock.java`) identity, POWERED, the
//! `SideChainPartBlock` chain (`SideChainPartBlock.java`) and
//! `SelectableSlotContainer.getHitSlot` for one row of three.
//!
//! A powered shelf reaches sideways for up to two more of them, and the four
//! `SIDE_CHAIN_PART` values are how each one records which way it is holding
//! on. Every path that changes `POWERED` runs the chain: powering up joins,
//! powering down and removal both let go.
//!
use super::LevelCtx;
use bbb_protocol::block_types::{vanilla_block_type, VanillaBlockUseItemOnOverride};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;
use crate::redstone;
use bbb_server_types::directions::{clock_wise, counter_clock_wise, opposite, relative};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};

use super::{
    block_type_of, bool_property, default_block_state_id, direction_serialized_name,
    string_property, BlockStateAt,
};

const FACING: &str = "facing";
const POWERED: &str = "powered";
const WATERLOGGED: &str = "waterlogged";

/// `BlockStateProperties.SIDE_CHAIN_PART` (`BlockStateProperties.java:73`).
const SIDE_CHAIN_PART: &str = "side_chain";

/// `ShelfBlock.getMaxChainLength` (`ShelfBlock.java:273-275`).
const MAX_CHAIN_LENGTH: i32 = 3;

pub const ACTIVATE_SOUND: &str = "minecraft:block.shelf.activate";
pub const DEACTIVATE_SOUND: &str = "minecraft:block.shelf.deactivate";
pub const TAKE_ITEM_SOUND: &str = "minecraft:block.shelf.take_item";
pub const PLACE_ITEM_SOUND: &str = "minecraft:block.shelf.place_item";
pub const SINGLE_SWAP_SOUND: &str = "minecraft:block.shelf.single_swap";
pub const MULTI_SWAP_SOUND: &str = "minecraft:block.shelf.multi_swap";

/// `ShelfBlock.useItemOn` is unique to that class.
#[must_use]
pub fn is_shelf(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.use_item_on_override)
        == Some(VanillaBlockUseItemOnOverride::ShelfBlock)
}

/// `ShelfBlock.getStateForPlacement` (`ShelfBlock.java`).
pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    horizontal_direction: Direction,
    block_state_at: BlockStateAt<'_>,
    sources: redstone::SignalSources<'_>,
) -> Option<i32> {
    let replaced = block_state_at(pos);
    block_type_of(replaced)?;
    let replaced_fluid = block_state_fluid_state(replaced);
    if matches!(horizontal_direction, Direction::Down | Direction::Up) {
        return None;
    }

    let state = default_block_state_id(block_type_id);
    if !is_shelf(state) {
        return None;
    }
    let state = with_property(
        state,
        FACING,
        direction_serialized_name(opposite(horizontal_direction)),
    )?;
    let powered = redstone::has_neighbor_signal(pos, &mut sources.emitting(block_state_at));
    let state = with_property(state, POWERED, if powered { "true" } else { "false" })?;
    with_property(
        state,
        WATERLOGGED,
        if replaced_fluid.is_source() && replaced_fluid.is_same_family(FluidFamily::Water) {
            "true"
        } else {
            "false"
        },
    )
}

/// `ShelfBlock.getAnalogOutputSignal` (`ShelfBlock.java:319-331`): one bit per
/// filled slot, read only from behind the shelf.
pub(crate) fn analog_output(
    block_state_id: i32,
    direction: Direction,
    items: &[bbb_server_types::item_stack::RuntimeItemStack],
) -> i32 {
    if super::direction_property(block_state_id, FACING).map(opposite) != Some(direction) {
        return 0;
    }
    items
        .iter()
        .take(3)
        .enumerate()
        .filter(|(_, item)| !item.is_empty())
        .fold(0, |mask, (slot, _)| mask | 1 << slot)
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

/// `SelectableSlotContainer.getHitSlot` with `getRows() == 1` and
/// `getColumns() == 3` (`ShelfBlock.java`).
#[must_use]
pub fn hit_slot(facing: Direction, clicked: Direction, cursor: [f32; 3]) -> Option<usize> {
    if facing != clicked {
        return None;
    }
    let relative_x = match clicked {
        Direction::North => 1.0 - cursor[0],
        Direction::South => cursor[0],
        Direction::West => cursor[2],
        Direction::East => 1.0 - cursor[2],
        Direction::Down | Direction::Up => return None,
    };
    let column = section(relative_x, 3);
    Some(column)
}

fn section(relative: f32, sections: usize) -> usize {
    let targeted_pixel = relative * 16.0;
    let section_size = 16.0 / sections as f32;
    (targeted_pixel / section_size)
        .floor()
        .clamp(0.0, (sections - 1) as f32) as usize
}

/// `ShelfBlock.neighborChanged` (`ShelfBlock.java`).
pub(super) fn neighbor_changed(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    check_powered_state(level, pos, block_state_id);
}

fn check_powered_state(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let signal = level.has_neighbor_signal(pos);
    if powered(block_state_id) == signal {
        return;
    }
    let Some(powered_state) = with_property(
        block_state_id,
        POWERED,
        if signal { "true" } else { "false" },
    ) else {
        return;
    };
    let Some(next) = (if signal {
        Some(powered_state)
    } else {
        with_property(
            powered_state,
            SIDE_CHAIN_PART,
            SideChainPart::Unconnected.name(),
        )
    }) else {
        return;
    };
    level.set_block(
        pos,
        next,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.play_block_sound(
        pos,
        if signal {
            ACTIVATE_SOUND
        } else {
            DEACTIVATE_SOUND
        },
    );
    level.block_game_event(
        if signal {
            VanillaGameEvent::BLOCK_ACTIVATE
        } else {
            VanillaGameEvent::BLOCK_DEACTIVATE
        },
        pos,
        Some(next),
    );
}

/// `ShelfBlock.affectNeighborsAfterRemoval` (`ShelfBlock.java`).
pub(super) fn affect_neighbors_after_removal(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    if let Some(block_type_id) = block_type_of(block_state_id) {
        level.update_neighbour_for_output_signal(pos, block_type_id);
    }
    update_neighbors_after_powering_down(level, pos, block_state_id);
}

/// `ShelfBlock.onPlace` (`ShelfBlock.java`), which is how a `setBlock`
/// that flipped `POWERED` gets the chain rewritten.
pub(super) fn on_place(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    old_state: i32,
) {
    if powered(block_state_id) {
        update_self_and_neighbors_on_powering_up(level, pos, block_state_id, old_state);
    } else {
        update_neighbors_after_powering_down(level, pos, block_state_id);
    }
}

/// `SideChainPart` (`SideChainPart.java`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SideChainPart {
    Unconnected,
    Right,
    Center,
    Left,
}

impl SideChainPart {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "unconnected" => Some(Self::Unconnected),
            "right" => Some(Self::Right),
            "center" => Some(Self::Center),
            "left" => Some(Self::Left),
            _ => None,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Unconnected => "unconnected",
            Self::Right => "right",
            Self::Center => "center",
            Self::Left => "left",
        }
    }

    /// `SideChainPart.isConnected` (`SideChainPart.java`).
    const fn is_connected(self) -> bool {
        !matches!(self, Self::Unconnected)
    }

    /// `SideChainPart.isConnectionTowards` (`SideChainPart.java`).
    fn is_connection_towards(self, end_part: Self) -> bool {
        self == Self::Center || self == end_part
    }

    /// `SideChainPart.isChainEnd` (`SideChainPart.java`).
    fn is_chain_end(self) -> bool {
        self != Self::Center
    }

    /// `SideChainPart.whenConnectedToTheRight` (`SideChainPart.java`).
    const fn when_connected_to_the_right(self) -> Self {
        match self {
            Self::Unconnected | Self::Left => Self::Left,
            Self::Right | Self::Center => Self::Center,
        }
    }

    /// `SideChainPart.whenConnectedToTheLeft` (`SideChainPart.java`).
    const fn when_connected_to_the_left(self) -> Self {
        match self {
            Self::Unconnected | Self::Right => Self::Right,
            Self::Center | Self::Left => Self::Center,
        }
    }

    /// `SideChainPart.whenDisconnectedFromTheRight` (`SideChainPart.java`).
    const fn when_disconnected_from_the_right(self) -> Self {
        match self {
            Self::Unconnected | Self::Left => Self::Unconnected,
            Self::Right | Self::Center => Self::Right,
        }
    }

    /// `SideChainPart.whenDisconnectedFromTheLeft` (`SideChainPart.java`).
    const fn when_disconnected_from_the_left(self) -> Self {
        match self {
            Self::Unconnected | Self::Right => Self::Unconnected,
            Self::Center | Self::Left => Self::Left,
        }
    }
}

/// `ShelfBlock.isConnectable` (`ShelfBlock.java`).
fn is_connectable(level: &LevelCtx<'_, '_>, block_state_id: i32) -> bool {
    level.block_state_in_tag(block_state_id, "minecraft:wooden_shelves") && powered(block_state_id)
}

fn part_of(block_state_id: i32) -> Option<SideChainPart> {
    SideChainPart::parse(&string_property(block_state_id, SIDE_CHAIN_PART)?)
}

/// One cell beside the shelf, with the part it carried when it was read.
///
/// `SideChainPartBlock.Neighbors` (`SideChainPartBlock.java`) caches
/// exactly this, so a write to one side does not change what the other side
/// was already found to be.
struct Neighbor {
    pos: BlockPos,
    part: Option<SideChainPart>,
}

impl Neighbor {
    /// `SideChainPartBlock.EmptyNeighbor` versus `SideChainNeighbor`
    /// (`SideChainPartBlock.java`, `:177-212`) as one shape: a
    /// neighbour is only connectable when it is a powered shelf facing the
    /// same way (`:146-148`).
    fn read(
        level: &LevelCtx<'_, '_>,
        neighbour_state: i32,
        pos: BlockPos,
        chain_facing: Direction,
    ) -> Self {
        let part = (is_connectable(level, neighbour_state)
            && facing(neighbour_state) == chain_facing)
            .then(|| part_of(neighbour_state))
            .flatten();
        Self { pos, part }
    }

    fn is_connectable(&self) -> bool {
        self.part.is_some()
    }
}

/// `Neighbors.left(steps)` and `right(steps)` (`SideChainPartBlock.java`),
/// which are the clockwise and counter-clockwise cells from the shelf's own
/// `FACING`.
fn neighbor_at(
    level: &LevelCtx<'_, '_>,
    center: BlockPos,
    chain_facing: Direction,
    towards: Direction,
    steps: i32,
) -> Neighbor {
    let mut pos = center;
    for _ in 0..steps {
        pos = relative(pos, towards);
    }
    Neighbor::read(level, level.block_state(pos), pos, chain_facing)
}

fn left_of(
    level: &LevelCtx<'_, '_>,
    center: BlockPos,
    chain_facing: Direction,
    steps: i32,
) -> Neighbor {
    neighbor_at(level, center, chain_facing, clock_wise(chain_facing), steps)
}

fn right_of(
    level: &LevelCtx<'_, '_>,
    center: BlockPos,
    chain_facing: Direction,
    steps: i32,
) -> Neighbor {
    neighbor_at(
        level,
        center,
        chain_facing,
        counter_clock_wise(chain_facing),
        steps,
    )
}

fn chain_length(level: &LevelCtx<'_, '_>, pos: BlockPos) -> i32 {
    let chain = connected_chain(
        pos,
        |at| level.block_state(at),
        |state| is_connectable(level, state),
    );
    i32::try_from(chain.len()).expect("a shelf chain is short")
}

/// `SideChainPartBlock.getAllBlocksConnectedTo` (`SideChainPartBlock.java:26-53`),
/// left end first; `connectable` is `ShelfBlock.isConnectable`.
pub fn connected_chain(
    pos: BlockPos,
    state_at: impl Fn(BlockPos) -> i32,
    connectable: impl Fn(i32) -> bool,
) -> Vec<BlockPos> {
    let state = state_at(pos);
    if !connectable(state) {
        return Vec::new();
    }
    let chain_facing = facing(state);
    let part_at = |at: BlockPos| {
        let neighbour = state_at(at);
        (connectable(neighbour) && facing(neighbour) == chain_facing)
            .then(|| part_of(neighbour))
            .flatten()
    };
    let walk = |towards: Direction, end_part: SideChainPart| {
        let mut found = Vec::new();
        let mut at = pos;
        for _ in 1..MAX_CHAIN_LENGTH {
            at = relative(at, towards);
            let part = part_at(at);
            if part.is_some_and(|part| part.is_connection_towards(end_part)) {
                found.push(at);
            }
            if part.is_none_or(SideChainPart::is_chain_end) {
                break;
            }
        }
        found
    };
    let mut chain = walk(clock_wise(chain_facing), SideChainPart::Left);
    chain.reverse();
    chain.push(pos);
    chain.extend(walk(counter_clock_wise(chain_facing), SideChainPart::Right));
    chain
}

/// `SideChainPartBlock.setPart` (`SideChainPartBlock.java`), which
/// re-reads the cell rather than trusting what the neighbour was found to be.
fn set_part(level: &mut LevelCtx<'_, '_>, pos: BlockPos, new_part: SideChainPart) {
    let state = level.block_state(pos);
    if part_of(state) == Some(new_part) {
        return;
    }
    let Some(next) = with_property(state, SIDE_CHAIN_PART, new_part.name()) else {
        return;
    };
    level.set_block(
        pos,
        next,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
}

/// `SideChainPartBlock.updateNeighborsAfterPoweringDown`
/// (`SideChainPartBlock.java`).
fn update_neighbors_after_powering_down(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    let chain_facing = facing(block_state_id);
    let left = left_of(level, pos, chain_facing, 1);
    let right = right_of(level, pos, chain_facing, 1);
    if let Some(part) = left.part {
        set_part(level, left.pos, part.when_disconnected_from_the_right());
    }
    if let Some(part) = right.part {
        set_part(level, right.pos, part.when_disconnected_from_the_left());
    }
}

/// `SideChainPartBlock.canConnect` (`SideChainPartBlock.java`).
fn can_connect(new_blocks_to_connect_to: i32, current_chain_length: i32) -> bool {
    new_blocks_to_connect_to > 0
        && current_chain_length + new_blocks_to_connect_to <= MAX_CHAIN_LENGTH
}

/// `SideChainPartBlock.isBeingUpdatedByNeighbor` (`SideChainPartBlock.java`):
/// the guard that keeps a shelf joined by its neighbour from rebuilding the
/// chain a second time.
fn is_being_updated_by_neighbor(
    level: &LevelCtx<'_, '_>,
    block_state_id: i32,
    old_state: i32,
) -> bool {
    let getting_connected = part_of(block_state_id).is_some_and(SideChainPart::is_connected);
    let connected_before = is_connectable(level, old_state)
        && part_of(old_state).is_some_and(SideChainPart::is_connected);
    getting_connected || connected_before
}

/// `SideChainPartBlock.updateSelfAndNeighborsOnPoweringUp`
/// (`SideChainPartBlock.java`).
fn update_self_and_neighbors_on_powering_up(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    old_state: i32,
) {
    if !is_connectable(level, block_state_id)
        || is_being_updated_by_neighbor(level, block_state_id, old_state)
    {
        return;
    }
    let chain_facing = facing(block_state_id);
    let left = left_of(level, pos, chain_facing, 1);
    let right = right_of(level, pos, chain_facing, 1);
    let existing_on_the_left = if left.is_connectable() {
        chain_length(level, left.pos)
    } else {
        0
    };
    let existing_on_the_right = if right.is_connectable() {
        chain_length(level, right.pos)
    } else {
        0
    };
    let mut new_part_for_self = SideChainPart::Unconnected;
    let mut current_chain_length = 1;
    if can_connect(existing_on_the_left, current_chain_length) {
        new_part_for_self = new_part_for_self.when_connected_to_the_left();
        if let Some(part) = left.part {
            set_part(level, left.pos, part.when_connected_to_the_right());
        }
        current_chain_length += existing_on_the_left;
    }
    if can_connect(existing_on_the_right, current_chain_length) {
        new_part_for_self = new_part_for_self.when_connected_to_the_right();
        if let Some(part) = right.part {
            set_part(level, right.pos, part.when_connected_to_the_left());
        }
    }
    set_part(level, pos, new_part_for_self);
}
