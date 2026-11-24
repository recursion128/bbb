//! `BaseRailBlock` with `RailState`, and the three classes that extend it —
//! `RailBlock`, `PoweredRailBlock` and `DetectorRailBlock`
//! (`net/minecraft/world/level/block/BaseRailBlock.java`, `RailState.java`,
//! `RailBlock.java`, `PoweredRailBlock.java`, `DetectorRailBlock.java`).
//!
//! Four blocks over four classes. [`RailState`] is the whole mechanism: it
//! works a rail's two connection cells out of its `SHAPE`, drops the ones that
//! do not connect back, and rewrites both ends so a rail laid next to another
//! turns to meet it. Everything else is the three `updateState` bodies that
//! decide when to run it.
//!
//! # What is deliberately not here
//!
//! * `rotate`/`mirror`, which belong to the structure seam.

use super::{BlockShapeCtx, LevelCtx};
use bbb_protocol::block_state_types::VanillaBlockSupportType as SupportType;
use bbb_protocol::block_types::{VanillaCanSurviveOverride, VanillaTickOverride};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{above, below, FlowingFluid, HORIZONTAL, SET_BLOCK_UPDATE_ALL};
use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType, TickPriority};
use bbb_server_types::directions::relative;
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};
use bbb_server_types::geometry::Aabb;

use crate::redstone::{self, WireSignal};

use super::level::{BlockEntityQuery, InsideEntity};
use super::support::str_property;
use super::{
    block_type_of, bool_property, can_survive_override, default_block_state_id, face_sturdy,
    offset, tick_override, with_property, BlockStateAt,
};

/// `BaseRailBlock.WATERLOGGED` (`BaseRailBlock.java:28`).
const WATERLOGGED: &str = "waterlogged";
const RAILS_TAG: &str = "minecraft:rails";

/// `BlockStateProperties.RAIL_SHAPE` and `RAIL_SHAPE_STRAIGHT`, both spelled
/// `shape` (`RailBlock.java:16`, `PoweredRailBlock.java:17`).
const SHAPE: &str = "shape";

/// `PoweredRailBlock.POWERED` (`PoweredRailBlock.java:18`), which
/// `DetectorRailBlock` carries under the same name (`DetectorRailBlock.java:32`).
const POWERED: &str = "powered";

/// The depth `findPoweredRailSignal` stops at (`PoweredRailBlock.java:31`).
const MAX_POWERED_RAIL_SEARCH_DEPTH: i32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RailShape {
    NorthSouth,
    EastWest,
    AscendingEast,
    AscendingWest,
    AscendingNorth,
    AscendingSouth,
    SouthEast,
    SouthWest,
    NorthWest,
    NorthEast,
}

impl RailShape {
    /// `RailShape.getSerializedName` (`RailShape.java:37-39`) through the
    /// constructor's names (`:6-15`).
    const fn serialized_name(self) -> &'static str {
        match self {
            Self::NorthSouth => "north_south",
            Self::EastWest => "east_west",
            Self::AscendingEast => "ascending_east",
            Self::AscendingWest => "ascending_west",
            Self::AscendingNorth => "ascending_north",
            Self::AscendingSouth => "ascending_south",
            Self::SouthEast => "south_east",
            Self::SouthWest => "south_west",
            Self::NorthWest => "north_west",
            Self::NorthEast => "north_east",
        }
    }

    fn parse(name: &str) -> Option<Self> {
        [
            Self::NorthSouth,
            Self::EastWest,
            Self::AscendingEast,
            Self::AscendingWest,
            Self::AscendingNorth,
            Self::AscendingSouth,
            Self::SouthEast,
            Self::SouthWest,
            Self::NorthWest,
            Self::NorthEast,
        ]
        .into_iter()
        .find(|shape| shape.serialized_name() == name)
    }

    /// `RailShape.isSlope` (`RailShape.java:30-32`).
    #[must_use]
    pub const fn is_slope(self) -> bool {
        matches!(
            self,
            Self::AscendingNorth | Self::AscendingEast | Self::AscendingSouth | Self::AscendingWest
        )
    }
}

/// Which `BaseRailBlock` subclass a stored state belongs to.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RailKind {
    Plain,
    Powered,
    Detector,
}

#[must_use]
pub fn is_rail(block_state_id: i32) -> bool {
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return false;
    };
    can_survive_override(block_type_id) == Some(VanillaCanSurviveOverride::BaseRailBlock)
}

fn is_live_rail(level: &LevelCtx<'_, '_>, block_state_id: i32) -> bool {
    is_rail(block_state_id) && level.block_state_in_tag(block_state_id, RAILS_TAG)
}

fn is_straight(block_state_id: i32) -> bool {
    with_property(
        block_state_id,
        SHAPE,
        RailShape::NorthEast.serialized_name(),
    )
    .is_none()
}

/// The subclass a stored state belongs to, or `None` when it is not a rail.
#[must_use]
pub fn kind(block_state_id: i32) -> Option<RailKind> {
    if !is_rail(block_state_id) {
        return None;
    }
    if !is_straight(block_state_id) {
        return Some(RailKind::Plain);
    }
    let block_type_id = block_type_of(block_state_id)?;
    if tick_override(block_type_id) == Some(VanillaTickOverride::DetectorRailBlock) {
        Some(RailKind::Detector)
    } else {
        Some(RailKind::Powered)
    }
}

pub(super) fn shape_of(block_state_id: i32) -> Option<RailShape> {
    RailShape::parse(str_property(block_state_id, SHAPE)?)
}

fn with_shape(block_state_id: i32, shape: RailShape) -> Option<i32> {
    with_property(block_state_id, SHAPE, shape.serialized_name())
}

// ---------------------------------------------------------------------------
// `RailState`
// ---------------------------------------------------------------------------

///
/// Vanilla's holds the `Level` as a field; here it is an argument to every
/// method that needs one, because the same solver reads the world while it is
/// being written through.
struct RailState {
    pos: BlockPos,
    state: i32,
    is_straight: bool,
    connections: Vec<BlockPos>,
}

impl RailState {
    fn new(pos: BlockPos, block_state_id: i32) -> Option<Self> {
        let shape = shape_of(block_state_id)?;
        let mut rail = Self {
            pos,
            state: block_state_id,
            is_straight: is_straight(block_state_id),
            connections: Vec::new(),
        };
        rail.update_connections(shape);
        Some(rail)
    }

    fn update_connections(&mut self, shape: RailShape) {
        let pos = self.pos;
        let north = offset(pos, 0, 0, -1);
        let south = offset(pos, 0, 0, 1);
        let west = offset(pos, -1, 0, 0);
        let east = offset(pos, 1, 0, 0);
        self.connections = match shape {
            RailShape::NorthSouth => vec![north, south],
            RailShape::EastWest => vec![west, east],
            RailShape::AscendingEast => vec![west, above(east)],
            RailShape::AscendingWest => vec![above(west), east],
            RailShape::AscendingNorth => vec![above(north), south],
            RailShape::AscendingSouth => vec![north, above(south)],
            RailShape::SouthEast => vec![east, south],
            RailShape::SouthWest => vec![west, south],
            RailShape::NorthWest => vec![west, north],
            RailShape::NorthEast => vec![east, north],
        };
    }

    fn remove_soft_connections(&mut self, level: &LevelCtx<'_, '_>) {
        let mut i = 0;
        while i < self.connections.len() {
            match Self::get_rail(level, self.connections[i]) {
                Some(rail) if rail.connects_to(self) => {
                    self.connections[i] = rail.pos;
                    i += 1;
                }
                _ => {
                    self.connections.remove(i);
                }
            }
        }
    }

    fn has_rail(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
        is_live_rail(level, level.block_state(pos))
            || is_live_rail(level, level.block_state(above(pos)))
            || is_live_rail(level, level.block_state(below(pos)))
    }

    fn get_rail(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<Self> {
        for test_pos in [pos, above(pos), below(pos)] {
            let test_state = level.block_state(test_pos);
            if is_live_rail(level, test_state) {
                return Self::new(test_pos, test_state);
            }
        }
        None
    }

    fn connects_to(&self, rail: &Self) -> bool {
        self.has_connection(rail.pos)
    }

    fn has_connection(&self, rail_pos: BlockPos) -> bool {
        self.connections
            .iter()
            .any(|pos| pos.x == rail_pos.x && pos.z == rail_pos.z)
    }

    fn count_potential_connections(&self, level: &LevelCtx<'_, '_>) -> i32 {
        HORIZONTAL
            .iter()
            .filter(|direction| Self::has_rail(level, relative(self.pos, **direction)))
            .count()
            .try_into()
            .unwrap_or(i32::MAX)
    }

    fn can_connect_to(&self, rail: &Self) -> bool {
        self.connects_to(rail) || self.connections.len() != 2
    }

    fn connect_to(&mut self, level: &mut LevelCtx<'_, '_>, rail: &Self) {
        self.connections.push(rail.pos);
        let north = offset(self.pos, 0, 0, -1);
        let south = offset(self.pos, 0, 0, 1);
        let west = offset(self.pos, -1, 0, 0);
        let east = offset(self.pos, 1, 0, 0);
        let n = self.has_connection(north);
        let s = self.has_connection(south);
        let w = self.has_connection(west);
        let e = self.has_connection(east);

        let mut shape = None;
        if n || s {
            shape = Some(RailShape::NorthSouth);
        }
        if w || e {
            shape = Some(RailShape::EastWest);
        }
        if !self.is_straight {
            if s && e && !n && !w {
                shape = Some(RailShape::SouthEast);
            }
            if s && w && !n && !e {
                shape = Some(RailShape::SouthWest);
            }
            if n && w && !s && !e {
                shape = Some(RailShape::NorthWest);
            }
            if n && e && !s && !w {
                shape = Some(RailShape::NorthEast);
            }
        }
        shape = ascend(level, shape, north, south, west, east);
        let shape = shape.unwrap_or(RailShape::NorthSouth);

        let Some(state) = with_shape(self.state, shape) else {
            return;
        };
        self.state = state;
        level.set_block(
            self.pos,
            self.state,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }

    fn has_neighbor_rail(&self, level: &LevelCtx<'_, '_>, rail_pos: BlockPos) -> bool {
        let Some(mut neighbor) = Self::get_rail(level, rail_pos) else {
            return false;
        };
        neighbor.remove_soft_connections(level);
        neighbor.can_connect_to(self)
    }

    fn place(
        &mut self,
        level: &mut LevelCtx<'_, '_>,
        has_signal: bool,
        first: bool,
        default_shape: RailShape,
    ) {
        let north = offset(self.pos, 0, 0, -1);
        let south = offset(self.pos, 0, 0, 1);
        let west = offset(self.pos, -1, 0, 0);
        let east = offset(self.pos, 1, 0, 0);
        let n = self.has_neighbor_rail(level, north);
        let s = self.has_neighbor_rail(level, south);
        let w = self.has_neighbor_rail(level, west);
        let e = self.has_neighbor_rail(level, east);

        let mut shape = None;
        let north_or_south = n || s;
        let west_or_east = w || e;
        if north_or_south && !west_or_east {
            shape = Some(RailShape::NorthSouth);
        }
        if west_or_east && !north_or_south {
            shape = Some(RailShape::EastWest);
        }

        let south_and_east = s && e;
        let south_and_west = s && w;
        let north_and_east = n && e;
        let north_and_west = n && w;
        if !self.is_straight {
            if south_and_east && !n && !w {
                shape = Some(RailShape::SouthEast);
            }
            if south_and_west && !n && !e {
                shape = Some(RailShape::SouthWest);
            }
            if north_and_west && !s && !e {
                shape = Some(RailShape::NorthWest);
            }
            if north_and_east && !s && !w {
                shape = Some(RailShape::NorthEast);
            }
        }

        if shape.is_none() {
            if north_or_south && west_or_east {
                shape = Some(default_shape);
            } else if north_or_south {
                shape = Some(RailShape::NorthSouth);
            } else if west_or_east {
                shape = Some(RailShape::EastWest);
            }

            if !self.is_straight {
                let corners = if has_signal {
                    [
                        (south_and_east, RailShape::SouthEast),
                        (south_and_west, RailShape::SouthWest),
                        (north_and_east, RailShape::NorthEast),
                        (north_and_west, RailShape::NorthWest),
                    ]
                } else {
                    [
                        (north_and_west, RailShape::NorthWest),
                        (north_and_east, RailShape::NorthEast),
                        (south_and_west, RailShape::SouthWest),
                        (south_and_east, RailShape::SouthEast),
                    ]
                };
                for (fires, corner) in corners {
                    if fires {
                        shape = Some(corner);
                    }
                }
            }
        }

        shape = ascend(level, shape, north, south, west, east);
        let shape = shape.unwrap_or(default_shape);

        self.update_connections(shape);
        let Some(state) = with_shape(self.state, shape) else {
            return;
        };
        self.state = state;
        if !first && level.block_state(self.pos) == self.state {
            return;
        }
        level.set_block(
            self.pos,
            self.state,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );

        let connections = self.connections.clone();
        for connection in connections {
            let Some(mut neighbor) = Self::get_rail(level, connection) else {
                continue;
            };
            neighbor.remove_soft_connections(level);
            if neighbor.can_connect_to(self) {
                neighbor.connect_to(level, self);
            }
        }
    }
}

fn ascend(
    level: &LevelCtx<'_, '_>,
    shape: Option<RailShape>,
    north: BlockPos,
    south: BlockPos,
    west: BlockPos,
    east: BlockPos,
) -> Option<RailShape> {
    let mut shape = shape;
    if shape == Some(RailShape::NorthSouth) {
        if is_live_rail(level, level.block_state(above(north))) {
            shape = Some(RailShape::AscendingNorth);
        }
        if is_live_rail(level, level.block_state(above(south))) {
            shape = Some(RailShape::AscendingSouth);
        }
    }
    if shape == Some(RailShape::EastWest) {
        if is_live_rail(level, level.block_state(above(east))) {
            shape = Some(RailShape::AscendingEast);
        }
        if is_live_rail(level, level.block_state(above(west))) {
            shape = Some(RailShape::AscendingWest);
        }
    }
    shape
}

// ---------------------------------------------------------------------------
// `BaseRailBlock`
// ---------------------------------------------------------------------------

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    horizontal_direction: Direction,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    let replaced = block_state_at(pos);
    block_type_of(replaced)?;
    let replaced_fluid = block_state_fluid_state(replaced);
    let shape = match horizontal_direction {
        Direction::West | Direction::East => RailShape::EastWest,
        Direction::North | Direction::South => RailShape::NorthSouth,
        Direction::Down | Direction::Up => return None,
    };

    let state = default_block_state_id(block_type_id);
    if !is_rail(state) {
        return None;
    }
    let state = with_property(state, SHAPE, shape.serialized_name())?;
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

#[must_use]
pub fn can_survive(pos: BlockPos, block_state_at: BlockStateAt<'_>) -> Option<bool> {
    face_sturdy(
        block_state_at(below(pos)),
        Direction::Up,
        SupportType::Rigid,
    )
}

pub(super) fn can_survive_live<L: crate::block_getter::BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
) -> Option<bool> {
    let support_pos = below(pos);
    Some(super::face_sturdy_at(
        level.block_state(support_pos),
        support_pos,
        Direction::Up,
        SupportType::Rigid,
    ))
}

fn should_be_removed(level: &LevelCtx<'_, '_>, pos: BlockPos, shape: RailShape) -> Option<bool> {
    let rigid = |at: BlockPos| {
        Some(super::face_sturdy_at(
            level.block_state(at),
            at,
            Direction::Up,
            SupportType::Rigid,
        ))
    };
    if !rigid(below(pos))? {
        return Some(true);
    }
    Some(match shape {
        RailShape::AscendingEast => !rigid(offset(pos, 1, 0, 0))?,
        RailShape::AscendingWest => !rigid(offset(pos, -1, 0, 0))?,
        RailShape::AscendingNorth => !rigid(offset(pos, 0, 0, -1))?,
        RailShape::AscendingSouth => !rigid(offset(pos, 0, 0, 1))?,
        _ => false,
    })
}

fn update_dir(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    first: bool,
) -> i32 {
    let Some(current) = shape_of(block_state_id) else {
        return block_state_id;
    };
    let has_signal = level.has_neighbor_signal(pos);
    let Some(mut rail) = RailState::new(pos, block_state_id) else {
        return block_state_id;
    };
    rail.place(level, has_signal, first, current);
    rail.state
}

fn update_state_on_place(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) -> i32 {
    let state = update_dir(level, pos, block_state_id, true);
    if is_straight(state) {
        if let Some(block_type_id) = block_type_of(state) {
            level.neighbor_changed_with_state(state, pos, block_type_id);
        }
    }
    state
}

///
/// `movedByPiston` is the call's own argument and only reaches
/// `Level.neighborChanged`, whose seam here does not carry it.
pub(super) fn on_place(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    old_state: i32,
) {
    if block_type_of(old_state) == block_type_of(block_state_id) {
        return;
    }
    let updated = update_state_on_place(level, pos, block_state_id);
    if kind(block_state_id) == Some(RailKind::Detector) {
        check_pressed(level, pos, updated, None);
    }
}

///
/// The `level.getBlockState(pos).is(this)` re-read at `:83` is not a
/// formality: the drop-and-remove leg below runs on the same position the
/// caller is fanning out from, so a rail that a previous update already took
/// away must not be handed to `updateState`.
pub(super) fn neighbor_changed(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    changed_block_type_id: i32,
) {
    if block_type_of(level.block_state(pos)) != block_type_of(block_state_id) {
        return;
    }
    let Some(shape) = shape_of(block_state_id) else {
        return;
    };
    let Some(should_remove) = should_be_removed(level, pos, shape) else {
        return;
    };
    if should_remove {
        level.drop_resources(pos, block_state_id);
        level.remove_block(pos);
        return;
    }
    match kind(block_state_id) {
        Some(RailKind::Plain) => {
            let source = default_block_state_id(changed_block_type_id);
            if !redstone::is_signal_source(source, WireSignal::Emitting) {
                return;
            }
            let Some(rail) = RailState::new(pos, block_state_id) else {
                return;
            };
            if rail.count_potential_connections(level) == 3 {
                update_dir(level, pos, block_state_id, false);
            }
        }
        Some(RailKind::Powered) => powered_update_state(level, pos, block_state_id),
        Some(RailKind::Detector) | None => {}
    }
}

pub(super) fn affect_neighbors_after_removal(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    moved_by_piston: bool,
) {
    if moved_by_piston {
        return;
    }
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    if shape_of(block_state_id).is_some_and(RailShape::is_slope) {
        level.update_neighbors_at(above(pos), block_type_id);
    }
    if is_straight(block_state_id) {
        level.update_neighbors_at(pos, block_type_id);
        level.update_neighbors_at(below(pos), block_type_id);
    }
}

pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    if bool_property(block_state_id, WATERLOGGED) == Some(true) {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
    block_state_id
}

// ---------------------------------------------------------------------------
// `PoweredRailBlock`
// ---------------------------------------------------------------------------

fn find_powered_rail_signal(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    forward: bool,
    search_depth: i32,
) -> bool {
    if search_depth >= MAX_POWERED_RAIL_SEARCH_DEPTH {
        return false;
    }
    let Some(shape) = shape_of(block_state_id) else {
        return false;
    };
    let (mut x, mut y, mut z) = (pos.x, pos.y, pos.z);
    let mut check_below = true;
    let shape = match shape {
        RailShape::NorthSouth => {
            z += if forward { 1 } else { -1 };
            RailShape::NorthSouth
        }
        RailShape::EastWest => {
            x += if forward { -1 } else { 1 };
            RailShape::EastWest
        }
        RailShape::AscendingEast => {
            if forward {
                x -= 1;
            } else {
                x += 1;
                y += 1;
                check_below = false;
            }
            RailShape::EastWest
        }
        RailShape::AscendingWest => {
            if forward {
                x -= 1;
                y += 1;
                check_below = false;
            } else {
                x += 1;
            }
            RailShape::EastWest
        }
        RailShape::AscendingNorth => {
            if forward {
                z += 1;
            } else {
                z -= 1;
                y += 1;
                check_below = false;
            }
            RailShape::NorthSouth
        }
        RailShape::AscendingSouth => {
            if forward {
                z += 1;
                y += 1;
                check_below = false;
            } else {
                z -= 1;
            }
            RailShape::NorthSouth
        }
        // The four corner shapes fall out of the `switch` untouched, so the
        // step is the rail's own cell and `checkBelow` stays true.
        other => other,
    };

    let ahead = BlockPos { x, y, z };
    if is_same_rail_with_power(level, ahead, block_state_id, forward, search_depth, shape) {
        return true;
    }
    check_below
        && is_same_rail_with_power(
            level,
            BlockPos { x, y: y - 1, z },
            block_state_id,
            forward,
            search_depth,
            shape,
        )
}

fn is_same_rail_with_power(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    from_state: i32,
    forward: bool,
    search_depth: i32,
    dir: RailShape,
) -> bool {
    let state = level.block_state(pos);
    if block_type_of(state) != block_type_of(from_state) {
        return false;
    }
    let Some(my_shape) = shape_of(state) else {
        return false;
    };
    let crosses = match dir {
        RailShape::EastWest => matches!(
            my_shape,
            RailShape::NorthSouth | RailShape::AscendingNorth | RailShape::AscendingSouth
        ),
        RailShape::NorthSouth => matches!(
            my_shape,
            RailShape::EastWest | RailShape::AscendingEast | RailShape::AscendingWest
        ),
        _ => false,
    };
    if crosses || bool_property(state, POWERED) != Some(true) {
        return false;
    }
    level.has_neighbor_signal(pos)
        || find_powered_rail_signal(level, pos, state, forward, search_depth + 1)
}

fn powered_update_state(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(is_powered) = bool_property(block_state_id, POWERED) else {
        return;
    };
    let neighbor_signal = level.has_neighbor_signal(pos);
    let should_power = neighbor_signal
        || find_powered_rail_signal(level, pos, block_state_id, true, 0)
        || find_powered_rail_signal(level, pos, block_state_id, false, 0);
    if should_power == is_powered {
        return;
    }
    let Some(powered_state) = with_property(
        block_state_id,
        POWERED,
        if should_power { "true" } else { "false" },
    ) else {
        return;
    };
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    level.set_block(
        pos,
        powered_state,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.update_neighbors_at(below(pos), block_type_id);
    if shape_of(block_state_id).is_some_and(RailShape::is_slope) {
        level.update_neighbors_at(above(pos), block_type_id);
    }
}

// ---------------------------------------------------------------------------
// `DetectorRailBlock`
// ---------------------------------------------------------------------------

pub(super) fn detector_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if kind(block_state_id) != Some(RailKind::Detector)
        || bool_property(block_state_id, POWERED) != Some(true)
    {
        return;
    }
    check_pressed(level, pos, block_state_id, None);
}

pub(super) fn detector_entity_inside(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    inside: InsideEntity,
) -> bool {
    if kind(block_state_id) != Some(RailKind::Detector) {
        return true;
    }
    match bool_property(block_state_id, POWERED) {
        Some(false) => check_pressed(level, pos, block_state_id, Some(inside)),
        Some(true) => true,
        None => false,
    }
}

///
/// `false` is a fact the body needed and this level could not answer, which
/// the entity-inside caller reports rather than reading as an empty box.
/// `canSurvive` answering false is vanilla's own no-op, not that.
fn check_pressed(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    inside: Option<InsideEntity>,
) -> bool {
    match can_survive_live(level, pos) {
        Some(true) => {}
        Some(false) => return true,
        None => return false,
    }
    let Some(was_pressed) = bool_property(block_state_id, POWERED) else {
        return false;
    };
    let search_box = detector_search_box(pos);
    // The entity that reached `entityInside` is inside this cell now; the
    // level's snapshot is built outside the tick that moved it.
    let should_be_pressed = match inside {
        Some(inside) if inside.minecart && inside.bounds.intersects(search_box) => true,
        _ => level.entity_count_in_box(search_box, BlockEntityQuery::Minecart) > 0,
    };
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return false;
    };
    if should_be_pressed != was_pressed {
        let Some(next) = with_property(
            block_state_id,
            POWERED,
            if should_be_pressed { "true" } else { "false" },
        ) else {
            return false;
        };
        level.set_block(
            pos,
            next,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        update_power_to_connected(level, pos, next);
        level.update_neighbors_at(pos, block_type_id);
        level.update_neighbors_at(below(pos), block_type_id);
    }
    if should_be_pressed {
        let Some(block) = BlockTickType::of_block_state(block_state_id) else {
            return false;
        };
        level.schedule_block_tick(pos, block, 20, TickPriority::Normal);
    }
    level.update_neighbour_for_output_signal(pos, block_type_id);
    true
}

fn update_power_to_connected(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(rail) = RailState::new(pos, block_state_id) else {
        return;
    };
    for connection_pos in rail.connections {
        let connection_state = level.block_state(connection_pos);
        if let Some(connection_type_id) = block_type_of(connection_state) {
            level.neighbor_changed_with_state(connection_state, connection_pos, connection_type_id);
        }
    }
}

fn detector_search_box(pos: BlockPos) -> Aabb {
    Aabb {
        min_x: f64::from(pos.x) + 0.2,
        min_y: f64::from(pos.y),
        min_z: f64::from(pos.z) + 0.2,
        max_x: f64::from(pos.x) + 0.8,
        max_y: f64::from(pos.y) + 0.8,
        max_z: f64::from(pos.z) + 0.8,
    }
}

///
/// The unpowered zero is already
/// [`crate::redstone::state_only_analog_output`]'s; this is the powered half,
/// which needs the minecarts standing on the rail.
pub(crate) fn detector_analog_output(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) -> i32 {
    if bool_property(block_state_id, POWERED) != Some(true) {
        return 0;
    }
    let found = level.detector_rail_minecarts(detector_search_box(pos));
    found
        .command_block_success_count
        .or(found.container_signal.flatten())
        .unwrap_or(0)
}

#[must_use]
pub fn detector_signal(block_state_id: i32) -> i32 {
    if bool_property(block_state_id, POWERED) == Some(true) {
        redstone::SIGNAL_MAX
    } else {
        0
    }
}

#[must_use]
pub fn detector_direct_signal(block_state_id: i32, direction: Direction) -> i32 {
    if direction == Direction::Up {
        detector_signal(block_state_id)
    } else {
        0
    }
}
