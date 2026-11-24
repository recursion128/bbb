//! `Orientation` (`net/minecraft/world/level/redstone/Orientation.java`),
//! `ExperimentalRedstoneUtils` (`ExperimentalRedstoneUtils.java`), and
//! `ExperimentalRedstoneWireEvaluator` (`ExperimentalRedstoneWireEvaluator.java`).

use std::collections::VecDeque;
use std::sync::OnceLock;

use bbb_protocol::block_types::VANILLA_BLOCK_REDSTONE_WIRE_ID;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::directions::{opposite, relative};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS;
use crate::fluid::{above, below, HORIZONTAL};
use crate::neighbor_updater::UPDATE_LIMIT;
use crate::random::GameRandom;
use crate::redstone::WireSignal;
use crate::world::simulation_level::LevelCtx;
use crate::world::SET_BLOCK_UPDATE_SKIP_SHAPE_UPDATE_ON_WIRE;

use super::{is_redstone_wire, power_of, side_of, with_power};

/// `Orientation.SideBias` (`Orientation.java:185-203`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SideBias {
    Left,
    Right,
}

impl SideBias {
    pub const fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    const fn ordinal(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    X,
    Y,
    Z,
}

const ALL_DIRECTIONS: [Direction; 6] = [
    Direction::Down,
    Direction::Up,
    Direction::North,
    Direction::South,
    Direction::West,
    Direction::East,
];

const fn direction_ordinal(direction: Direction) -> usize {
    match direction {
        Direction::Down => 0,
        Direction::Up => 1,
        Direction::North => 2,
        Direction::South => 3,
        Direction::West => 4,
        Direction::East => 5,
    }
}

const fn direction_axis(direction: Direction) -> Axis {
    match direction {
        Direction::West | Direction::East => Axis::X,
        Direction::Down | Direction::Up => Axis::Y,
        Direction::North | Direction::South => Axis::Z,
    }
}

/// `Direction.AxisDirection.ordinal()` (`Direction.java:546-548`):
/// `POSITIVE = 0`, `NEGATIVE = 1`.
const fn axis_direction_ordinal(direction: Direction) -> usize {
    match direction {
        Direction::Up | Direction::South | Direction::East => 0,
        Direction::Down | Direction::North | Direction::West => 1,
    }
}

const fn unit_vec(direction: Direction) -> (i32, i32, i32) {
    match direction {
        Direction::Down => (0, -1, 0),
        Direction::Up => (0, 1, 0),
        Direction::North => (0, 0, -1),
        Direction::South => (0, 0, 1),
        Direction::West => (-1, 0, 0),
        Direction::East => (1, 0, 0),
    }
}

fn cross_direction(front: Direction, up: Direction) -> Direction {
    let a = unit_vec(front);
    let b = unit_vec(up);
    let cross = (
        a.1 * b.2 - a.2 * b.1,
        a.2 * b.0 - a.0 * b.2,
        a.0 * b.1 - a.1 * b.0,
    );
    match cross {
        (0, -1, 0) => Direction::Down,
        (0, 1, 0) => Direction::Up,
        (0, 0, -1) => Direction::North,
        (0, 0, 1) => Direction::South,
        (-1, 0, 0) => Direction::West,
        (1, 0, 0) => Direction::East,
        _ => unreachable!("front and up must be orthogonal unit vectors"),
    }
}

/// `Orientation.generateIndex` (`Orientation.java:169-183`).
fn generate_index(up: Direction, front: Direction, side_bias: SideBias) -> u8 {
    let up_axis = direction_axis(up);
    let front_axis = direction_axis(front);
    assert!(
        up_axis != front_axis,
        "Up-vector and front-vector cannot be on the same axis"
    );
    let front_axis_key = if up_axis == Axis::Y {
        usize::from(front_axis == Axis::X)
    } else {
        usize::from(front_axis == Axis::Y)
    };
    let front_key = (front_axis_key << 1) | axis_direction_ordinal(front);
    ((((direction_ordinal(up) << 2) + front_key) << 1) + side_bias.ordinal()) as u8
}

#[derive(Debug, Clone, Copy)]
struct OrientationEntry {
    up: Direction,
    front: Direction,
    side: Direction,
    side_bias: SideBias,
    neighbors: [Direction; 6],
    horizontal_neighbors: [Direction; 4],
    vertical_neighbors: [Direction; 2],
    with_front: [u8; 6],
    with_up: [u8; 6],
    with_side_bias: [u8; 2],
}

impl OrientationEntry {
    fn new(up: Direction, front: Direction, side_bias: SideBias) -> Self {
        let right = cross_direction(front, up);
        let side = match side_bias {
            SideBias::Right => right,
            SideBias::Left => opposite(right),
        };
        let neighbors = [
            opposite(front),
            front,
            side,
            opposite(side),
            opposite(up),
            up,
        ];
        let up_axis = direction_axis(up);
        let mut horizontal_neighbors = [Direction::North; 4];
        let mut vertical_neighbors = [Direction::Down; 2];
        let mut h = 0;
        let mut v = 0;
        for direction in neighbors {
            if direction_axis(direction) == up_axis {
                vertical_neighbors[v] = direction;
                v += 1;
            } else {
                horizontal_neighbors[h] = direction;
                h += 1;
            }
        }
        Self {
            up,
            front,
            side,
            side_bias,
            neighbors,
            horizontal_neighbors,
            vertical_neighbors,
            with_front: [0; 6],
            with_up: [0; 6],
            with_side_bias: [0; 2],
        }
    }
}

fn orientation_table() -> &'static [OrientationEntry; 48] {
    static TABLE: OnceLock<[OrientationEntry; 48]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut slots: [Option<OrientationEntry>; 48] = [None; 48];

        fn populate(
            up: Direction,
            front: Direction,
            side_bias: SideBias,
            slots: &mut [Option<OrientationEntry>; 48],
        ) -> u8 {
            let index = generate_index(up, front, side_bias);
            if slots[usize::from(index)].is_some() {
                return index;
            }
            slots[usize::from(index)] = Some(OrientationEntry::new(up, front, side_bias));

            let mut with_side_bias = [0u8; 2];
            for bias in [SideBias::Left, SideBias::Right] {
                with_side_bias[bias.ordinal()] = populate(up, front, bias, slots);
            }

            let mut with_front = [0u8; 6];
            for facing in ALL_DIRECTIONS {
                let mut next_up = up;
                if facing == up {
                    next_up = opposite(front);
                }
                if facing == opposite(up) {
                    next_up = front;
                }
                with_front[direction_ordinal(facing)] = populate(next_up, facing, side_bias, slots);
            }

            let mut with_up = [0u8; 6];
            for facing in ALL_DIRECTIONS {
                let mut next_front = front;
                if facing == front {
                    next_front = opposite(up);
                }
                if facing == opposite(front) {
                    next_front = up;
                }
                with_up[direction_ordinal(facing)] = populate(facing, next_front, side_bias, slots);
            }

            let entry = slots[usize::from(index)]
                .as_mut()
                .expect("orientation slot initialized");
            entry.with_side_bias = with_side_bias;
            entry.with_front = with_front;
            entry.with_up = with_up;
            index
        }

        populate(Direction::Up, Direction::North, SideBias::Left, &mut slots);
        std::array::from_fn(|index| slots[index].expect("all 48 orientations are reachable"))
    })
}

/// One of the 48 `Orientation` states (`Orientation.java:16-183`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RedstoneOrientation(u8);

impl RedstoneOrientation {
    pub fn of(up: Direction, front: Direction, side_bias: SideBias) -> Self {
        let index = generate_index(up, front, side_bias);
        Self::from_index(index)
    }

    pub fn from_index(index: u8) -> Self {
        assert!(
            index < 48,
            "redstone orientation index {index} out of range"
        );
        let _ = orientation_table();
        Self(index)
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    /// `Orientation.random` (`Orientation.java:124-126`).
    pub fn random(random: &GameRandom) -> Self {
        Self::from_index(random.next_int(48) as u8)
    }

    fn entry(self) -> &'static OrientationEntry {
        &orientation_table()[usize::from(self.0)]
    }

    pub fn up(self) -> Direction {
        self.entry().up
    }

    pub fn front(self) -> Direction {
        self.entry().front
    }

    pub fn side(self) -> Direction {
        self.entry().side
    }

    pub fn side_bias(self) -> SideBias {
        self.entry().side_bias
    }

    pub fn with_up(self, up: Direction) -> Self {
        Self(self.entry().with_up[direction_ordinal(up)])
    }

    pub fn with_front(self, front: Direction) -> Self {
        Self(self.entry().with_front[direction_ordinal(front)])
    }

    /// `Orientation.withFrontPreserveUp` (`Orientation.java:66-68`).
    pub fn with_front_preserve_up(self, front: Direction) -> Self {
        if direction_axis(front) == direction_axis(self.up()) {
            self
        } else {
            self.with_front(front)
        }
    }

    /// `Orientation.withFrontAdjustSideBias` (`Orientation.java:70-73`).
    pub fn with_front_adjust_side_bias(self, front: Direction) -> Self {
        let next = self.with_front(front);
        if self.front() == next.side() {
            next.with_mirror()
        } else {
            next
        }
    }

    pub fn with_side_bias(self, side_bias: SideBias) -> Self {
        Self(self.entry().with_side_bias[side_bias.ordinal()])
    }

    pub fn with_mirror(self) -> Self {
        self.with_side_bias(self.side_bias().opposite())
    }

    /// `Orientation.getDirections` (`Orientation.java:99-101`).
    pub fn directions(self) -> [Direction; 6] {
        self.entry().neighbors
    }

    /// `Orientation.getHorizontalDirections` (`Orientation.java:103-105`).
    pub fn horizontal_directions(self) -> [Direction; 4] {
        self.entry().horizontal_neighbors
    }

    /// `Orientation.getVerticalDirections` (`Orientation.java:107-109`).
    pub fn vertical_directions(self) -> [Direction; 2] {
        self.entry().vertical_neighbors
    }
}

/// `ExperimentalRedstoneUtils.initialOrientation`
/// (`ExperimentalRedstoneUtils.java:9-24`).
pub(crate) fn initial_orientation(
    level: &LevelCtx<'_, '_>,
    front: Option<Direction>,
    up: Option<Direction>,
) -> Option<RedstoneOrientation> {
    if !level.redstone_experiments_enabled() {
        return None;
    }
    let mut orientation =
        RedstoneOrientation::random(level.level_random()).with_side_bias(SideBias::Left);
    if let Some(up) = up {
        orientation = orientation.with_up(up);
    }
    if let Some(front) = front {
        orientation = orientation.with_front(front);
    }
    Some(orientation)
}

/// `ExperimentalRedstoneUtils.withFront` (`ExperimentalRedstoneUtils.java:26-28`).
pub(crate) fn with_front(
    orientation: Option<RedstoneOrientation>,
    front: Direction,
) -> Option<RedstoneOrientation> {
    orientation.map(|orientation| orientation.with_front(front))
}

/// `ExperimentalRedstoneWireEvaluator`
/// (`net/minecraft/world/level/redstone/ExperimentalRedstoneWireEvaluator.java`).
#[derive(Default)]
struct ExperimentalRedstoneWireEvaluator {
    wires_to_turn_off: VecDeque<BlockPos>,
    in_turn_off: FxHashSet<BlockPos>,
    wires_to_turn_on: VecDeque<BlockPos>,
    in_turn_on: FxHashSet<BlockPos>,
    updated_index: FxHashMap<BlockPos, usize>,
    updated_wires: Vec<(BlockPos, u16)>,
}

thread_local! {
    static WIRE_EVALUATOR: std::cell::RefCell<ExperimentalRedstoneWireEvaluator> =
        std::cell::RefCell::new(ExperimentalRedstoneWireEvaluator::default());
}

pub(super) fn update_power_strength(
    level: &mut LevelCtx<'_, '_>,
    initial_pos: BlockPos,
    orientation: Option<RedstoneOrientation>,
    shape_update_wires_around_initial_position: bool,
) {
    WIRE_EVALUATOR.with(|cell| {
        if let Ok(mut evaluator) = cell.try_borrow_mut() {
            evaluator.clear();
            evaluator.run(
                level,
                initial_pos,
                orientation,
                shape_update_wires_around_initial_position,
            );
            evaluator.clear_and_cap();
        } else {
            let mut evaluator = ExperimentalRedstoneWireEvaluator::default();
            evaluator.run(
                level,
                initial_pos,
                orientation,
                shape_update_wires_around_initial_position,
            );
        }
    });
}

impl ExperimentalRedstoneWireEvaluator {
    fn clear(&mut self) {
        self.wires_to_turn_off.clear();
        self.in_turn_off.clear();
        self.wires_to_turn_on.clear();
        self.in_turn_on.clear();
        self.updated_index.clear();
        self.updated_wires.clear();
    }

    fn clear_and_cap(&mut self) {
        self.clear();
        const MAX_RETAINED_CAPACITY: usize = 1024;
        if self.wires_to_turn_off.capacity() > MAX_RETAINED_CAPACITY {
            self.wires_to_turn_off.shrink_to(MAX_RETAINED_CAPACITY);
        }
        if self.in_turn_off.capacity() > MAX_RETAINED_CAPACITY {
            self.in_turn_off.shrink_to(MAX_RETAINED_CAPACITY);
        }
        if self.wires_to_turn_on.capacity() > MAX_RETAINED_CAPACITY {
            self.wires_to_turn_on.shrink_to(MAX_RETAINED_CAPACITY);
        }
        if self.in_turn_on.capacity() > MAX_RETAINED_CAPACITY {
            self.in_turn_on.shrink_to(MAX_RETAINED_CAPACITY);
        }
        if self.updated_index.capacity() > MAX_RETAINED_CAPACITY {
            self.updated_index.shrink_to(MAX_RETAINED_CAPACITY);
        }
        if self.updated_wires.capacity() > MAX_RETAINED_CAPACITY {
            self.updated_wires.shrink_to(MAX_RETAINED_CAPACITY);
        }
    }

    /// `ExperimentalRedstoneWireEvaluator.updatePowerStrength` (`:30-60`).
    fn run(
        &mut self,
        level: &mut LevelCtx<'_, '_>,
        initial_pos: BlockPos,
        orientation: Option<RedstoneOrientation>,
        shape_update_wires_around_initial_position: bool,
    ) {
        let initial_orientation = get_initial_orientation(level, orientation);
        self.calculate_current_changes(level, initial_pos, initial_orientation);

        let mut committed_len = 0;
        for index in 0..self.updated_wires.len() {
            let (pos, packed) = self.updated_wires[index];
            let initial_wire = index == 0;
            let new_level = unpack_power(packed);
            let state = level.block_state(pos);
            if is_redstone_wire(state) && power_of(state) != new_level {
                let mut update_flags = SET_BLOCK_UPDATE_CLIENTS;
                if !shape_update_wires_around_initial_position || !initial_wire {
                    update_flags |= SET_BLOCK_UPDATE_SKIP_SHAPE_UPDATE_ON_WIRE;
                }
                level.set_block(
                    pos,
                    with_power(state, new_level),
                    update_flags,
                    UPDATE_LIMIT,
                );
                self.updated_wires[committed_len] = (pos, packed);
                committed_len += 1;
            }
        }
        self.updated_wires.truncate(committed_len);
        self.cause_neighbor_updates(level);
    }

    /// `ExperimentalRedstoneWireEvaluator.causeNeighborUpdates` (`:62-90`).
    fn cause_neighbor_updates(&self, level: &mut LevelCtx<'_, '_>) {
        for &(wire_pos, packed) in &self.updated_wires {
            let orientation = unpack_orientation(packed);
            let state = level.block_state(wire_pos);
            if !is_redstone_wire(state) {
                continue;
            }
            for neighbor_direction in orientation.directions() {
                if !is_connected(state, neighbor_direction) {
                    continue;
                }
                let neighbor_pos = relative(wire_pos, neighbor_direction);
                let neighbor_state = level.block_state(neighbor_pos);
                let neighbor_conducts = level.is_redstone_conductor_at(neighbor_pos);
                let neighbor_orientation = orientation.with_front_preserve_up(neighbor_direction);
                level.neighbor_changed_with_state_and_orientation(
                    neighbor_state,
                    neighbor_pos,
                    VANILLA_BLOCK_REDSTONE_WIRE_ID,
                    Some(neighbor_orientation),
                );
                if neighbor_conducts {
                    for direction in neighbor_orientation.directions() {
                        if direction != opposite(neighbor_direction) {
                            level.neighbor_changed_with_orientation(
                                relative(neighbor_pos, direction),
                                VANILLA_BLOCK_REDSTONE_WIRE_ID,
                                Some(neighbor_orientation.with_front_preserve_up(direction)),
                            );
                        }
                    }
                }
            }
        }
    }

    /// `ExperimentalRedstoneWireEvaluator.calculateCurrentChanges` (`:108-159`).
    fn calculate_current_changes(
        &mut self,
        level: &LevelCtx<'_, '_>,
        initial_position: BlockPos,
        initial_orientation: RedstoneOrientation,
    ) {
        let initial_state = level.block_state(initial_position);
        if is_redstone_wire(initial_state) {
            self.set_power(
                initial_position,
                power_of(initial_state),
                initial_orientation,
            );
            self.push_turn_off(initial_position);
        } else {
            self.propagate_change_to_neighbors(
                level,
                initial_position,
                0,
                initial_orientation,
                true,
            );
        }

        while let Some(pos) = self.pop_turn_off() {
            let packed = self.get_packed(pos);
            let orientation = unpack_orientation(packed);
            let old_power = unpack_power(packed);
            let block_power = level.get_best_neighbor_signal_with(pos, WireSignal::Suppressed);
            let wire_power = self.get_incoming_wire_signal(level, pos);
            let new_power = block_power.max(wire_power);
            let power_to_set = if new_power < old_power {
                if block_power > 0 {
                    self.push_turn_on(pos);
                }
                0
            } else {
                new_power
            };
            if power_to_set != old_power {
                self.set_power(pos, power_to_set, orientation);
            }
            self.propagate_change_to_neighbors(
                level,
                pos,
                power_to_set,
                orientation,
                old_power > new_power,
            );
        }

        while let Some(pos) = self.pop_turn_on() {
            let packed = self.get_packed(pos);
            let orientation = unpack_orientation(packed);
            let old_power = unpack_power(packed);
            let block_power = level.get_best_neighbor_signal_with(pos, WireSignal::Suppressed);
            let wire_power = self.get_incoming_wire_signal(level, pos);
            let new_power = block_power.max(wire_power);
            if new_power > old_power {
                self.set_power(pos, new_power, orientation);
            }
            self.propagate_change_to_neighbors(level, pos, new_power, orientation, false);
        }
    }

    /// `ExperimentalRedstoneWireEvaluator.propagateChangeToNeighbors` (`:181-204`).
    fn propagate_change_to_neighbors(
        &mut self,
        level: &LevelCtx<'_, '_>,
        pos: BlockPos,
        new_power: i32,
        orientation: RedstoneOrientation,
        allow_turning_off: bool,
    ) {
        for direction_horizontal in orientation.horizontal_directions() {
            let offset_pos = relative(pos, direction_horizontal);
            self.enqueue_neighbor_wire(
                level,
                offset_pos,
                new_power,
                orientation.with_front(direction_horizontal),
                allow_turning_off,
            );
        }
        for direction_vertical in orientation.vertical_directions() {
            let offset_pos = relative(pos, direction_vertical);
            let solid_block = level.is_redstone_conductor_at(offset_pos);
            for direction_horizontal in orientation.horizontal_directions() {
                let neighbor = relative(pos, direction_horizontal);
                if (direction_vertical == Direction::Up && !solid_block)
                    || (direction_vertical == Direction::Down
                        && !level.is_redstone_conductor_at(neighbor))
                {
                    let neighbor_wire = relative(offset_pos, direction_horizontal);
                    self.enqueue_neighbor_wire(
                        level,
                        neighbor_wire,
                        new_power,
                        orientation.with_front(direction_horizontal),
                        allow_turning_off,
                    );
                }
            }
        }
    }

    /// `ExperimentalRedstoneWireEvaluator.enqueueNeighborWire` (`:206-220`).
    fn enqueue_neighbor_wire(
        &mut self,
        level: &LevelCtx<'_, '_>,
        pos: BlockPos,
        new_from_power: i32,
        orientation: RedstoneOrientation,
        allow_turning_off: bool,
    ) {
        let state = level.block_state(pos);
        if !is_redstone_wire(state) {
            return;
        }
        let to_power = self.get_wire_signal(pos, state);
        if to_power < new_from_power - 1 && self.push_turn_on(pos) {
            self.set_power(pos, to_power, orientation);
        }
        if allow_turning_off && to_power > new_from_power && self.push_turn_off(pos) {
            self.set_power(pos, to_power, orientation);
        }
    }

    /// `RedstoneWireEvaluator.getIncomingWireSignal` (`RedstoneWireEvaluator.java:29-47`)
    /// using the overridden `getWireSignal` (`ExperimentalRedstoneWireEvaluator.java:223-226`).
    fn get_incoming_wire_signal(&self, level: &LevelCtx<'_, '_>, pos: BlockPos) -> i32 {
        let mut signal = 0;
        let above_pos = above(pos);
        let above_conducts = level.is_redstone_conductor_at(above_pos);
        for direction in HORIZONTAL {
            let neighbor_pos = relative(pos, direction);
            let neighbor_state = level.block_state(neighbor_pos);
            signal = signal.max(self.get_wire_signal(neighbor_pos, neighbor_state));
            let neighbor_conducts = level.is_redstone_conductor_at(neighbor_pos);
            if neighbor_conducts && !above_conducts {
                let above_neighbor = above(neighbor_pos);
                signal = signal
                    .max(self.get_wire_signal(above_neighbor, level.block_state(above_neighbor)));
            } else if !neighbor_conducts {
                let below_neighbor = below(neighbor_pos);
                signal = signal
                    .max(self.get_wire_signal(below_neighbor, level.block_state(below_neighbor)));
            }
        }
        0.max(signal - 1)
    }

    /// `ExperimentalRedstoneWireEvaluator.getWireSignal` (`:223-226`).
    fn get_wire_signal(&self, pos: BlockPos, state: i32) -> i32 {
        if let Some(&idx) = self.updated_index.get(&pos) {
            unpack_power(self.updated_wires[idx].1)
        } else if is_redstone_wire(state) {
            power_of(state)
        } else {
            0
        }
    }

    fn set_power(&mut self, pos: BlockPos, new_power: i32, orientation: RedstoneOrientation) {
        if let Some(&idx) = self.updated_index.get(&pos) {
            let preserved = unpack_orientation(self.updated_wires[idx].1);
            self.updated_wires[idx].1 = pack_orientation_and_power(preserved, new_power);
        } else {
            let idx = self.updated_wires.len();
            self.updated_wires
                .push((pos, pack_orientation_and_power(orientation, new_power)));
            self.updated_index.insert(pos, idx);
        }
    }

    fn get_packed(&self, pos: BlockPos) -> u16 {
        let idx = *self
            .updated_index
            .get(&pos)
            .expect("queued wire must be recorded in updated_wires");
        self.updated_wires[idx].1
    }

    fn push_turn_off(&mut self, pos: BlockPos) -> bool {
        if self.in_turn_off.insert(pos) {
            self.wires_to_turn_off.push_back(pos);
            true
        } else {
            false
        }
    }

    fn pop_turn_off(&mut self) -> Option<BlockPos> {
        let pos = self.wires_to_turn_off.pop_front()?;
        self.in_turn_off.remove(&pos);
        Some(pos)
    }

    fn push_turn_on(&mut self, pos: BlockPos) -> bool {
        if self.in_turn_on.insert(pos) {
            self.wires_to_turn_on.push_back(pos);
            true
        } else {
            false
        }
    }

    fn pop_turn_on(&mut self) -> Option<BlockPos> {
        let pos = self.wires_to_turn_on.pop_front()?;
        self.in_turn_on.remove(&pos);
        Some(pos)
    }
}

fn is_connected(state: i32, direction: Direction) -> bool {
    match direction {
        Direction::Down => true,
        Direction::Up => false,
        Direction::North | Direction::East | Direction::South | Direction::West => {
            side_of(state, direction).is_connected()
        }
    }
}

fn get_initial_orientation(
    level: &LevelCtx<'_, '_>,
    incoming: Option<RedstoneOrientation>,
) -> RedstoneOrientation {
    let orientation = incoming.unwrap_or_else(|| RedstoneOrientation::random(level.level_random()));
    orientation
        .with_up(Direction::Up)
        .with_side_bias(SideBias::Left)
}

fn pack_orientation_and_power(orientation: RedstoneOrientation, power: i32) -> u16 {
    (u16::from(orientation.index()) << 4) | ((power as u16) & 15)
}

fn unpack_orientation(packed: u16) -> RedstoneOrientation {
    RedstoneOrientation::from_index((packed >> 4) as u8)
}

fn unpack_power(packed: u16) -> i32 {
    i32::from(packed & 15)
}
