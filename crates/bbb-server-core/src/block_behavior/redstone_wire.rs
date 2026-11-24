//! `RedStoneWireBlock` (`net/minecraft/world/level/block/RedStoneWireBlock.java`).
//!
//!
//! This is the slice's first block whose *shape* is a function of its
//! surroundings rather than of the hand that placed it, and its first
//! **carrier**: a torch emits a constant and a lamp consumes one, but a wire's
//! output is its input minus one, recomputed every time anything near it moves.
//!
//! # Four connection properties and one power
//!
//! [`RedstoneSide`] is `NORTH`/`EAST`/`SOUTH`/`WEST` (`:46-49`) and `POWER`
//! (`:50`) is the strength. Three shapes matter to the bodies below and each
//! has its own predicate in vanilla: a *cross* is all four sides connected
//! (`:196-201`), a *dot* is none of them (`:203-208`), and everything else is a
//! line or a corner. `getConnectionState` (`:124-154`) is what turns a raw
//! neighbourhood into one of them, and its second half is the rule that a wire
//! with nothing on the north-south axis grows east and west arms instead of
//! staying a dot.
//!
//! # `shouldSignal`
//!
//! `RedStoneWireBlock.shouldSignal` (`:67`) is a mutable field on the singleton
//! block that all three redstone members read, and `getBlockSignal` (`:285-290`)
//! is the only thing that clears it. Here it is
//! [`crate::redstone::WireSignal`], carried on
//! [`crate::redstone::SignalContext`] rather than kept in a global; see that
//! module for why.
//!
//! # Evaluator selection
//!
//! When `minecraft:redstone_experiments` is enabled on the world,
//! [`update_power_strength`] dispatches to [`experimental`]
//! (`ExperimentalRedstoneWireEvaluator`); otherwise it uses [`evaluator`]
//! (`DefaultRedstoneWireEvaluator`).

use super::{BlockShapeCtx, LevelCtx};
use crate::block_getter::BlockGetter;

use bbb_protocol::block_types::{
    vanilla_block_type, VanillaNeighborChangedOverride, VANILLA_BLOCK_HOPPER_ID,
    VANILLA_BLOCK_OBSERVER_ID, VANILLA_BLOCK_REDSTONE_WIRE_ID, VANILLA_BLOCK_REPEATER_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{above, below, with_property, HORIZONTAL, SET_BLOCK_UPDATE_ALL};
use crate::redstone::{SignalContext, WireSignal};
use bbb_server_types::directions::{opposite, relative};

use std::sync::OnceLock;

use super::{
    air_block_state_id, block_type_of, default_block_state_id, direction_property, face_sturdy,
    face_sturdy_at, state_is_block, BlockStateAt, SupportType, DIRECTION_VALUES,
};

pub mod evaluator;
pub mod experimental;

/// `RedStoneWireBlock.NORTH`/`EAST`/`SOUTH`/`WEST`
/// (`RedStoneWireBlock.java:46-49`), which are the four
/// `BlockStateProperties.*_REDSTONE` enum properties.
const NORTH: &str = "north";
const EAST: &str = "east";
const SOUTH: &str = "south";
const WEST: &str = "west";

/// `RedStoneWireBlock.POWER` (`RedStoneWireBlock.java:50`).
pub(crate) const POWER: &str = "power";

/// `RepeaterBlock.FACING` and `ObserverBlock.FACING`, the two properties
/// `shouldConnectTo` (`RedStoneWireBlock.java:392`, `:395`) reads.
const FACING: &str = "facing";

/// `Direction.Plane.VERTICAL` in its declared order (`Direction.java:578`):
/// up first, then down.
///
/// `onPlace` (`RedStoneWireBlock.java:307-309`) is the one walk over it here.
/// The order is not observable in the result -- both cells get the same call --
/// but it is the order the two neighbour updates are enqueued in, and
/// [`crate::neighbor_updater`] is a stack.
const VERTICAL: [Direction; 2] = [Direction::Up, Direction::Down];

/// `RedstoneSide` (`net/minecraft/world/level/block/state/properties/RedstoneSide.java`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedstoneSide {
    Up,
    Side,
    None,
}

impl RedstoneSide {
    const ALL: [Self; 3] = [Self::Up, Self::Side, Self::None];

    const fn index(self) -> usize {
        match self {
            Self::Up => 0,
            Self::Side => 1,
            Self::None => 2,
        }
    }

    const fn from_index(index: u8) -> Self {
        match index {
            0 => Self::Up,
            1 => Self::Side,
            _ => Self::None,
        }
    }

    /// `RedstoneSide.getSerializedName` (`RedstoneSide.java:22-24`).
    const fn serialized_name(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Side => "side",
            Self::None => "none",
        }
    }

    /// `RedstoneSide.isConnected` (`RedstoneSide.java:26-28`), which is
    /// `this != NONE` -- so a wire that climbs counts as connected.
    #[must_use]
    pub const fn is_connected(self) -> bool {
        !matches!(self, Self::None)
    }
}

#[derive(Clone, Copy)]
struct WireProps {
    north: u8,
    east: u8,
    south: u8,
    west: u8,
    power: u8,
}

struct RedstoneWireTable {
    state_ids: [[[[[i32; 16]; 3]; 3]; 3]; 3],
    min_state_id: i32,
    by_offset: Vec<Option<WireProps>>,
}

fn wire_table() -> &'static RedstoneWireTable {
    static TABLE: OnceLock<RedstoneWireTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        const POWER_NAMES: [&str; 16] = [
            "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
        ];
        let base = default_block_state_id(VANILLA_BLOCK_REDSTONE_WIRE_ID);
        let mut state_ids = [[[[[0i32; 16]; 3]; 3]; 3]; 3];
        let mut min_id = i32::MAX;
        let mut max_id = i32::MIN;
        for north in RedstoneSide::ALL {
            let s_n = with_property(base, NORTH, north.serialized_name())
                .expect("redstone wire north property");
            for east in RedstoneSide::ALL {
                let s_e = with_property(s_n, EAST, east.serialized_name())
                    .expect("redstone wire east property");
                for south in RedstoneSide::ALL {
                    let s_s = with_property(s_e, SOUTH, south.serialized_name())
                        .expect("redstone wire south property");
                    for west in RedstoneSide::ALL {
                        let s_w = with_property(s_s, WEST, west.serialized_name())
                            .expect("redstone wire west property");
                        for (power, power_name) in POWER_NAMES.iter().enumerate() {
                            let id = with_property(s_w, POWER, power_name)
                                .expect("redstone wire power property");
                            state_ids[north.index()][east.index()][south.index()][west.index()]
                                [power] = id;
                            min_id = min_id.min(id);
                            max_id = max_id.max(id);
                        }
                    }
                }
            }
        }
        let span = usize::try_from(max_id - min_id + 1).expect("valid redstone wire state span");
        let mut by_offset = vec![None; span];
        for n in 0..3u8 {
            for e in 0..3u8 {
                for s in 0..3u8 {
                    for w in 0..3u8 {
                        for p in 0..16u8 {
                            let id = state_ids[n as usize][e as usize][s as usize][w as usize]
                                [p as usize];
                            by_offset[(id - min_id) as usize] = Some(WireProps {
                                north: n,
                                east: e,
                                south: s,
                                west: w,
                                power: p,
                            });
                        }
                    }
                }
            }
        }
        RedstoneWireTable {
            state_ids,
            min_state_id: min_id,
            by_offset,
        }
    })
}

fn wire_props(block_state_id: i32) -> WireProps {
    let table = wire_table();
    let offset = usize::try_from(block_state_id - table.min_state_id)
        .expect("redstone wire state id in table bounds");
    table
        .by_offset
        .get(offset)
        .and_then(|entry| *entry)
        .expect("every redstone wire state is in the lookup table")
}

/// Whether a stored state is `minecraft:redstone_wire`.
///
#[must_use]
pub fn is_redstone_wire(block_state_id: i32) -> bool {
    state_is_block(block_state_id, VANILLA_BLOCK_REDSTONE_WIRE_ID)
}

/// `state.getValue(PROPERTY_BY_DIRECTION.get(direction))`.
///
/// # Panics
///
/// Panics for a state that is not redstone wire or a vertical direction; every
/// caller here has already been routed by [`is_redstone_wire`] and walks
/// [`HORIZONTAL`].
fn side_of(block_state_id: i32, direction: Direction) -> RedstoneSide {
    let props = wire_props(block_state_id);
    let idx = match direction {
        Direction::North => props.north,
        Direction::East => props.east,
        Direction::South => props.south,
        Direction::West => props.west,
        Direction::Down | Direction::Up => panic!("a horizontal side"),
    };
    RedstoneSide::from_index(idx)
}

/// `state.setValue(PROPERTY_BY_DIRECTION.get(direction), side)`.
fn with_side(block_state_id: i32, direction: Direction, side: RedstoneSide) -> i32 {
    let mut props = wire_props(block_state_id);
    let idx = side.index() as u8;
    match direction {
        Direction::North => props.north = idx,
        Direction::East => props.east = idx,
        Direction::South => props.south = idx,
        Direction::West => props.west = idx,
        Direction::Down | Direction::Up => panic!("a horizontal side"),
    }
    wire_table().state_ids[props.north as usize][props.east as usize][props.south as usize]
        [props.west as usize][props.power as usize]
}

/// `state.getValue(POWER)`.
pub(crate) fn power_of(block_state_id: i32) -> i32 {
    i32::from(wire_props(block_state_id).power)
}

/// `state.setValue(POWER, power)`.
pub(crate) fn with_power(block_state_id: i32, power: i32) -> i32 {
    let props = wire_props(block_state_id);
    let power = usize::try_from(power)
        .ok()
        .filter(|&p| p < 16)
        .expect("POWER accepts every strength a wire can carry");
    wire_table().state_ids[props.north as usize][props.east as usize][props.south as usize]
        [props.west as usize][power]
}

fn default_wire_state() -> i32 {
    default_block_state_id(VANILLA_BLOCK_REDSTONE_WIRE_ID)
}

fn cross_state() -> i32 {
    let s = RedstoneSide::Side.index();
    wire_table().state_ids[s][s][s][s][0]
}

fn is_cross(block_state_id: i32) -> bool {
    let props = wire_props(block_state_id);
    let none = RedstoneSide::None.index() as u8;
    props.north != none && props.east != none && props.south != none && props.west != none
}

fn is_dot(block_state_id: i32) -> bool {
    let props = wire_props(block_state_id);
    let none = RedstoneSide::None.index() as u8;
    props.north == none && props.east == none && props.south == none && props.west == none
}

pub(in crate::block_behavior) fn is_trapdoor(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.neighbor_changed_override)
        == Some(VanillaNeighborChangedOverride::TrapDoorBlock)
}

// ---------------------------------------------------------------------------
// The three redstone members
// ---------------------------------------------------------------------------

/// `RedStoneWireBlock.isSignalSource` (`RedStoneWireBlock.java:400-402`), which
/// is `this.shouldSignal` and nothing else.
#[must_use]
pub(crate) const fn is_signal_source(wire_signal: WireSignal) -> bool {
    matches!(wire_signal, WireSignal::Emitting)
}

///
/// Three gates in a row, and the order is the whole of the behaviour: a
/// suppressed wire and the cell below a wire get nothing (`:369`); a wire at
/// zero gets nothing whatever its shape (`:370-372`); and a horizontal side
/// only carries power when the wire's *recomputed* connection state points at
/// it (`:374-377`). `UP` is exempt from that last test, which is why a wire
/// feeds the block above it whatever it is connected to.
///
/// `getConnectionState` is recomputed rather than read off `state` because the
/// stored shape can be stale -- the level is asked again, which is also why
/// this member needs a `BlockGetter` at all.
pub(crate) fn signal(
    block_state_id: i32,
    pos: BlockPos,
    direction: Direction,
    context: &mut SignalContext<'_>,
) -> i32 {
    // `:369`.
    if !is_signal_source(context.wire_signal()) || direction == Direction::Down {
        return 0;
    }
    // `:370-373`.
    let power = power_of(block_state_id);
    if power == 0 {
        return 0;
    }
    // `:374-377`. `direction != Direction.UP` is first, so a wire never
    // recomputes its shape to answer the block above it.
    if direction == Direction::Up {
        return power;
    }
    let connection_state = get_connection_state(context, block_state_id, pos);
    if side_of(connection_state, opposite(direction)).is_connected() {
        power
    } else {
        0
    }
}

pub(crate) fn direct_signal(
    block_state_id: i32,
    pos: BlockPos,
    direction: Direction,
    context: &mut SignalContext<'_>,
) -> i32 {
    if !is_signal_source(context.wire_signal()) {
        return 0;
    }
    signal(block_state_id, pos, direction, context)
}

// ---------------------------------------------------------------------------
// The connection walk
// ---------------------------------------------------------------------------

///
/// Wire connects to wire from every side, to a repeater only along its own
/// axis, to an observer only on the face it outputs from, and to any other
/// signal source only when a side was named at all.
///
/// The last arm is written the other way round from vanilla, which spells
/// `blockState.isSignalSource() && direction != null` (`:395`) and therefore
/// evaluates the call first. `isSignalSource` is pure in vanilla, so the two
/// orders answer the same thing.
fn should_connect_to(
    block_state_id: i32,
    direction: Option<Direction>,
    wire_signal: WireSignal,
) -> bool {
    // `:389-390`.
    if is_redstone_wire(block_state_id) {
        return true;
    }
    // `:391-393`.
    if state_is_block(block_state_id, VANILLA_BLOCK_REPEATER_ID) {
        let Some(facing) = direction_property(block_state_id, FACING) else {
            return false;
        };
        return direction == Some(facing) || direction == Some(opposite(facing));
    }
    // `:395`, first arm.
    if state_is_block(block_state_id, VANILLA_BLOCK_OBSERVER_ID) {
        return direction == direction_property(block_state_id, FACING);
    }
    // `:395`, second arm. `BlockState.isSignalSource` is the free question in
    // [`crate::redstone`], not this module's own member of the same name.
    direction.is_some() && crate::redstone::is_signal_source(block_state_id, wire_signal)
}

fn can_survive_on(block_state_id: i32) -> bool {
    face_sturdy(block_state_id, Direction::Up, SupportType::Full) == Some(true)
        || state_is_block(block_state_id, VANILLA_BLOCK_HOPPER_ID)
}

fn can_survive_on_at(context: &SignalContext<'_>, pos: BlockPos, block_state_id: i32) -> bool {
    context.support_face(block_state_id, pos, Direction::Up, SupportType::Full)
        || state_is_block(block_state_id, VANILLA_BLOCK_HOPPER_ID)
}

///
/// The first half is the climb: when nothing solid is directly above this wire,
/// a block on the given side that could itself hold wire, and that has
/// something to connect to *on top of it*, is either climbed (`UP`, when the
/// block's facing side is sturdy enough to draw the wire against) or merely
/// reached (`SIDE`).
///
/// The second half is the flat case, and it is a double negative: the side is
/// `NONE` only when the neighbour does not connect **and** either conducts --
/// so nothing under it can be seen -- or has nothing under it to see.
fn get_connecting_side(
    context: &mut SignalContext<'_>,
    pos: BlockPos,
    direction: Direction,
    can_connect_up: bool,
) -> RedstoneSide {
    // `:241-242`.
    let relative_pos = relative(pos, direction);
    let relative_state = context.block_state(relative_pos);
    let wire_signal = context.wire_signal();
    // `:243`.
    if can_connect_up {
        // `:244`.
        let is_placeable_above =
            is_trapdoor(relative_state) || can_survive_on_at(context, relative_pos, relative_state);
        // `:245`.
        if is_placeable_above {
            let above_state = context.block_state(above(relative_pos));
            if should_connect_to(above_state, None, wire_signal) {
                // `:246-248`.
                if context.support_face(
                    relative_state,
                    relative_pos,
                    opposite(direction),
                    SupportType::Full,
                ) {
                    return RedstoneSide::Up;
                }
                // `:250`.
                return RedstoneSide::Side;
            }
        }
    }

    // `:254-257`. Both `&&` and `||` short-circuit in vanilla too, so a
    // neighbour that connects is never asked whether it conducts and a
    // conductor is never asked what is under it.
    let unconnected = !should_connect_to(relative_state, Some(direction), wire_signal)
        && (context.is_conductor_at(relative_pos, relative_state) || {
            let below_state = context.block_state(below(relative_pos));
            !should_connect_to(below_state, None, wire_signal)
        });
    if unconnected {
        RedstoneSide::None
    } else {
        RedstoneSide::Side
    }
}

fn get_connecting_side_here(
    context: &mut SignalContext<'_>,
    pos: BlockPos,
    direction: Direction,
) -> RedstoneSide {
    let above_state = context.block_state(above(pos));
    let can_connect_up = !context.is_conductor_at(above(pos), above_state);
    get_connecting_side(context, pos, direction, can_connect_up)
}

fn get_missing_connections(
    context: &mut SignalContext<'_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    // `:157`. One read for all four sides: a solid block overhead stops the
    // wire climbing anything.
    let above_state = context.block_state(above(pos));
    let can_connect_up = !context.is_conductor_at(above(pos), above_state);
    let mut state = block_state_id;
    // `:159-164`.
    for direction in HORIZONTAL {
        if !side_of(state, direction).is_connected() {
            let side = get_connecting_side(context, pos, direction, can_connect_up);
            state = with_side(state, direction, side);
        }
    }
    state
}

///
/// The connections are computed from `defaultBlockState()` and not from the
/// state handed in (`:126`), so only `POWER` survives; a dot that is still a
/// dot afterwards stays one (`:127-129`), and everything else grows the arms
/// that make a bare wire a cross instead of an invisible point.
fn get_connection_state(
    context: &mut SignalContext<'_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    // `:125`.
    let was_dot = is_dot(block_state_id);
    // `:126`.
    let mut state = get_missing_connections(
        context,
        with_power(default_wire_state(), power_of(block_state_id)),
        pos,
    );
    // `:127-129`.
    if was_dot && is_dot(state) {
        return state;
    }
    // `:131-136`.
    let north = side_of(state, Direction::North).is_connected();
    let south = side_of(state, Direction::South).is_connected();
    let east = side_of(state, Direction::East).is_connected();
    let west = side_of(state, Direction::West).is_connected();
    let north_south_empty = !north && !south;
    let east_west_empty = !east && !west;
    // `:137-151`.
    if !west && north_south_empty {
        state = with_side(state, Direction::West, RedstoneSide::Side);
    }
    if !east && north_south_empty {
        state = with_side(state, Direction::East, RedstoneSide::Side);
    }
    if !north && east_west_empty {
        state = with_side(state, Direction::North, RedstoneSide::Side);
    }
    if !south && east_west_empty {
        state = with_side(state, Direction::South, RedstoneSide::Side);
    }
    state
}

// ---------------------------------------------------------------------------
// Survival and placement
// ---------------------------------------------------------------------------

///
/// `None` is "no answer", never "cannot survive", exactly as
/// [`super::can_survive`] documents: it is what [`can_survive_on`] refuses with
/// for a block whose shape is dynamic.
pub(super) fn can_survive(pos: BlockPos, block_state_at: BlockStateAt<'_>) -> Option<bool> {
    // `:262-264`.
    Some(can_survive_on(block_state_at(below(pos))))
}

pub(super) fn can_survive_live<L: BlockGetter + ?Sized>(level: &L, pos: BlockPos) -> Option<bool> {
    let support_pos = below(pos);
    let support = level.block_state(support_pos);
    if face_sturdy_at(support, support_pos, Direction::Up, SupportType::Full) {
        Some(true)
    } else {
        Some(state_is_block(support, VANILLA_BLOCK_HOPPER_ID))
    }
}

///
/// The state handed to `getConnectionState` is the **cross**, not the default,
/// so a wire placed with nothing around it lands as a cross rather than as a
/// dot -- `getConnectionState` only keeps a dot that was already one.
pub(super) fn state_for_placement(pos: BlockPos, block_state_at: BlockStateAt<'_>) -> Option<i32> {
    let mut context = SignalContext::emitting(block_state_at);
    Some(get_connection_state(&mut context, cross_state(), pos))
}

// ---------------------------------------------------------------------------
// The level-touching bodies
// ---------------------------------------------------------------------------

pub(super) fn use_without_item(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    may_build: bool,
) -> bool {
    // `:493-498`.
    if !may_build || !(is_cross(block_state_id) || is_dot(block_state_id)) {
        return false;
    }
    let seed = if is_cross(block_state_id) {
        default_wire_state()
    } else {
        cross_state()
    };
    let seed = with_power(seed, power_of(block_state_id));
    let new_state = {
        let this = &*level;
        let mut read = |at: BlockPos| this.block_state(at);
        let mut context =
            SignalContext::emitting(&mut read).with_reader(&crate::redstone::SturdySignalSources);
        // `:499-500`.
        get_connection_state(&mut context, seed, pos)
    };
    // `:501-505`.
    if new_state == block_state_id {
        return false;
    }
    level.set_block(
        pos,
        new_state,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    updates_on_shape_change(level, pos, block_state_id, new_state);
    true
}

fn update_power_strength(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    orientation: Option<experimental::RedstoneOrientation>,
    shape_update_wires_around_initial_position: bool,
) {
    // `RedStoneWireBlock.updatePowerStrength` (`RedStoneWireBlock.java:271-283`).
    if level.redstone_experiments_enabled() {
        experimental::update_power_strength(
            level,
            pos,
            orientation,
            shape_update_wires_around_initial_position,
        );
    } else {
        evaluator::update_power_strength(level, pos, block_state_id);
    }
}

fn updates_on_shape_change(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    old_state: i32,
    new_state: i32,
) {
    // `RedStoneWireBlock.updatesOnShapeChange` (`RedStoneWireBlock.java:511-523`).
    let orientation = experimental::initial_orientation(level, None, Some(Direction::Up));
    for direction in HORIZONTAL {
        if side_of(old_state, direction).is_connected()
            == side_of(new_state, direction).is_connected()
        {
            continue;
        }
        let neighbour_pos = relative(pos, direction);
        if level.is_redstone_conductor_at(neighbour_pos) {
            level.update_neighbors_at_except_from_facing_with_orientation(
                neighbour_pos,
                VANILLA_BLOCK_REDSTONE_WIRE_ID,
                opposite(direction),
                experimental::with_front(orientation, direction),
            );
        }
    }
}

///
/// Seven `updateNeighborsAt` calls, but only when the cell really does hold
/// wire: the position's own neighbourhood plus one centred on each of its six
/// faces. It is how a corner two cells away learns that the shape it was
/// connected to has changed.
fn check_corner_change_at(level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    // `:293`.
    if !is_redstone_wire(level.block_state(pos)) {
        return;
    }
    // `:294`.
    level.update_neighbors_at(pos, VANILLA_BLOCK_REDSTONE_WIRE_ID);
    // `:296-298`.
    for direction in DIRECTION_VALUES {
        level.update_neighbors_at(relative(pos, direction), VANILLA_BLOCK_REDSTONE_WIRE_ID);
    }
}

///
/// Two passes over the four horizontal sides, in that order: the sides
/// themselves first, and only then the cell each side's *diagonal* would be --
/// one above a conductor, one below anything else, which is exactly the pair of
/// cells a wire can climb between.
fn update_neighbors_of_neighboring_wires(level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    // `:328-330`.
    for direction in HORIZONTAL {
        check_corner_change_at(level, relative(pos, direction));
    }
    // `:332-339`.
    for direction in HORIZONTAL {
        let target = relative(pos, direction);
        if level.is_redstone_conductor_at(target) {
            check_corner_change_at(level, above(target));
        } else {
            check_corner_change_at(level, below(target));
        }
    }
}

pub(super) fn on_place(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    old_state: i32,
) {
    // `:304`. The `!level.isClientSide()` half is always true here.
    if is_redstone_wire(old_state) {
        return;
    }
    // `:305`.
    update_power_strength(level, pos, block_state_id, None, true);
    // `:307-309`.
    for direction in VERTICAL {
        level.update_neighbors_at(relative(pos, direction), VANILLA_BLOCK_REDSTONE_WIRE_ID);
    }
    // `:311`.
    update_neighbors_of_neighboring_wires(level, pos);
}

///
/// The mirror of `onPlace` with one difference that matters: the six-way
/// notification happens **before** the power update here and after it there, so
/// a wire that is going away drops its neighbours' power on the way out rather
/// than on the way in.
///
pub(super) fn affect_neighbors_after_removal(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    moved_by_piston: bool,
) {
    // `:317`.
    if moved_by_piston {
        return;
    }
    // `:318-320`.
    for direction in DIRECTION_VALUES {
        level.update_neighbors_at(relative(pos, direction), VANILLA_BLOCK_REDSTONE_WIRE_ID);
    }
    // `:322`.
    update_power_strength(level, pos, block_state_id, None, false);
    // `:323`.
    update_neighbors_of_neighboring_wires(level, pos);
}

/// `RedStoneWireBlock.neighborChanged` (`RedStoneWireBlock.java:343-356`).
pub(super) fn neighbor_changed(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    changed_block_type_id: i32,
    orientation: Option<experimental::RedstoneOrientation>,
) {
    // `:347`: when `minecraft:redstone_experiments` is active, wire-to-wire
    // neighbor updates are skipped because the experimental evaluator resolves
    // the full connected wire graph in one pass.
    if changed_block_type_id == VANILLA_BLOCK_REDSTONE_WIRE_ID
        && level.redstone_experiments_enabled()
    {
        return;
    }
    // `:348`.
    let support_pos = below(pos);
    let support = level.block_state(support_pos);
    let survives = super::face_sturdy_at(support, support_pos, Direction::Up, SupportType::Full)
        || state_is_block(support, VANILLA_BLOCK_HOPPER_ID);
    if survives {
        // `:349`.
        update_power_strength(level, pos, block_state_id, orientation, false);
    } else {
        // `:351-352`.
        level.drop_resources(pos, block_state_id);
        level.remove_block(pos);
    }
}

///
/// Three different answers depending on where the update came from: from below
/// it is a survival test, from above the whole shape is recomputed because what
/// is overhead decides whether this wire may climb at all, and from a side the
/// one property changes -- unless doing so would break a cross, in which case
/// the shape is recomputed from the cross with that one side already set.
pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    // `:180-182`.
    if direction_to_neighbour == Direction::Down {
        let survives = super::face_sturdy_at(
            neighbour_state,
            below(pos),
            Direction::Up,
            SupportType::Full,
        ) || state_is_block(neighbour_state, VANILLA_BLOCK_HOPPER_ID);
        return if survives {
            block_state_id
        } else {
            air_block_state_id()
        };
    }
    let this = &*level;
    let mut read = |at: BlockPos| this.block_state(at);
    let mut context = SignalContext::emitting(&mut read).with_reader(this);
    // `:184-186`.
    if direction_to_neighbour == Direction::Up {
        get_connection_state(&mut context, block_state_id, pos)
    } else {
        update_shape_from_side(&mut context, block_state_id, pos, direction_to_neighbour)
    }
}

fn update_shape_from_side(
    context: &mut SignalContext<'_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    // `:188`.
    let side_connection = get_connecting_side_here(context, pos, direction_to_neighbour);
    // `:189-190`: the cheap answer, taken only when the side's *connectedness*
    // did not change and this is not a cross that would silently lose an arm.
    if side_connection.is_connected()
        == side_of(block_state_id, direction_to_neighbour).is_connected()
        && !is_cross(block_state_id)
    {
        return with_side(block_state_id, direction_to_neighbour, side_connection);
    }
    // `:191-193`.
    let seed = with_side(
        with_power(cross_state(), power_of(block_state_id)),
        direction_to_neighbour,
        side_connection,
    );
    get_connection_state(context, seed, pos)
}

///
/// For every side this wire is connected to that is not itself wire, the wire
/// one cell up and the wire one cell down from that side are told that their
/// shape may have changed -- and told from the direction pointing *back* at
/// this wire. Those two cells are the diagonals a wire climbs, and no ordinary
/// neighbour fan-out reaches them, which is why the member exists at all.
///
pub(super) fn update_indirect_neighbour_shapes(
    level: &mut LevelCtx<'_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    update_flags: i32,
    update_limit: i32,
) {
    // `:216`.
    for direction in HORIZONTAL {
        // `:217-218`.
        let side_pos = relative(pos, direction);
        if !side_of(block_state_id, direction).is_connected()
            || is_redstone_wire(level.block_state(side_pos))
        {
            continue;
        }
        // `:219-224`, the cell below that side.
        relay_shape_change(
            level,
            below(side_pos),
            direction,
            update_flags,
            update_limit,
        );
        // `:226-231`, the cell above it.
        relay_shape_change(
            level,
            above(side_pos),
            direction,
            update_flags,
            update_limit,
        );
    }
}

fn relay_shape_change(
    level: &mut LevelCtx<'_, '_>,
    wire_pos: BlockPos,
    direction: Direction,
    update_flags: i32,
    update_limit: i32,
) {
    if !is_redstone_wire(level.block_state(wire_pos)) {
        return;
    }
    let neighbor_pos = relative(wire_pos, opposite(direction));
    let neighbor_state = level.block_state(neighbor_pos);
    level.neighbor_shape_changed(
        opposite(direction),
        wire_pos,
        neighbor_pos,
        neighbor_state,
        update_flags,
        update_limit,
    );
}
