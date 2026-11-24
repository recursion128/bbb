//! Per-chunk scheduled-tick containers, vanilla `LevelChunkTicks` semantics.
//!
//! This is the container layer only: it owns *which* ticks are pending for one
//! chunk and in what order they drain; the level scheduler collects from it.
//!
//! Two vanilla facts drive everything here:
//!
//! * Order is `ScheduledTick.DRAIN_ORDER`
//!   (`net/minecraft/world/ticks/ScheduledTick.java`): `triggerTick`
//!   ascending, then `priority` in enum declaration order
//!   (`TickPriority.java`), then `subTickOrder` ascending.
//! * Identity is `ScheduledTick.UNIQUE_TICK_HASH`
//!   (`ScheduledTick.java`): only `pos` and `type` participate, so a
//!   chunk holds at most one pending tick per (position, type) and
//!   `schedule` silently drops a later duplicate
//!   (`LevelChunkTicks.java`). `poll` frees the slot again
//!   (`LevelChunkTicks.java`), which is what lets a self-scheduling
//!   block re-arm itself from inside its own tick.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{vanilla_block_type, vanilla_block_type_for_resource_id};
use bbb_protocol::packets::BlockPos;
use bbb_protocol::registry_data_types::VanillaFluidIdentity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkError(pub String);

impl fmt::Display for ChunkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ChunkError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26SavedTick {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub delay: i32,
    pub priority: i32,
}

/// The `type` half of a scheduled tick's identity.
///
/// Vanilla parameterizes `LevelChunkTicks<T>` over the registry singleton --
/// `Block` for block ticks, `Fluid` for fluid ticks -- and `UNIQUE_TICK_HASH`
/// compares those singletons by reference (`ScheduledTick.java`). The
/// saved form carries the singleton's registry name in the `i` field
/// (`SavedTick.codec`, `SavedTick.java`), so the name round-trip belongs to
/// the type as well.
pub trait TickType: Copy + Eq + Hash + fmt::Debug {
    /// The registry name a saved tick stores in `i`.
    fn resource_id(self) -> &'static str;

    /// The inverse of [`TickType::resource_id`]; `None` for a name outside the
    /// built-in 26.1 registry.
    fn from_resource_id(resource_id: &str) -> Option<Self>;
}

/// A block scheduled tick's type: the `Block`, never the `BlockState`.
///
/// Keying on the block and not on the state is load-bearing, not a
/// simplification. `redstone_wire[power=0]` and `redstone_wire[power=15]` are
/// two states of one block, so vanilla lets them share a single pending tick
/// slot; a state-keyed container would hold two, and the block would tick
/// twice where vanilla ticks once.
#[derive(Clone, Copy)]
pub struct BlockTickType {
    block_type_id: i32,
    resource_id: &'static str,
}

impl BlockTickType {
    /// The block a built-in global state id belongs to (`BlockState.getBlock`).
    pub fn of_block_state(block_state_id: i32) -> Option<Self> {
        Self::of_block_type(vanilla_block_state_block_type_id(block_state_id)?)
    }

    /// Registry-aware form of [`Self::of_block_state`].
    pub fn of_block_state_with_registry(
        block_state_id: i32,
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> Option<Self> {
        if let Some(vanilla) = Self::of_block_state(block_state_id) {
            return Some(vanilla);
        }
        let (spec, _) = content_registry.block_by_state_id(block_state_id)?;
        Some(Self {
            block_type_id: spec.block_id,
            resource_id: spec.try_resource_id_static().ok()?,
        })
    }

    /// `None` for an id outside the built-in 26.1 block registry.
    pub fn of_block_type(block_type_id: i32) -> Option<Self> {
        vanilla_block_type(block_type_id).map(|entry| Self {
            block_type_id: entry.id,
            resource_id: entry.resource_id,
        })
    }

    /// Registry-aware form of [`Self::of_block_type`].
    pub fn of_block_type_with_registry(
        block_type_id: i32,
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> Option<Self> {
        if let Some(vanilla) = Self::of_block_type(block_type_id) {
            return Some(vanilla);
        }
        let spec = content_registry.block_by_id(block_type_id)?;
        Some(Self {
            block_type_id: spec.block_id,
            resource_id: spec.try_resource_id_static().ok()?,
        })
    }

    /// Registry-aware form of [`TickType::from_resource_id`].
    pub fn from_resource_id_with_registry(
        resource_id: &str,
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> Option<Self> {
        if let Some(vanilla) = Self::from_resource_id(resource_id) {
            return Some(vanilla);
        }
        let spec = content_registry.block_by_resource_id(resource_id)?;
        Some(Self {
            block_type_id: spec.block_id,
            resource_id: spec.try_resource_id_static().ok()?,
        })
    }

    pub const fn block_type_id(self) -> i32 {
        self.block_type_id
    }
}

impl TickType for BlockTickType {
    fn resource_id(self) -> &'static str {
        self.resource_id
    }

    fn from_resource_id(resource_id: &str) -> Option<Self> {
        vanilla_block_type_for_resource_id(resource_id).map(|entry| Self {
            block_type_id: entry.id,
            resource_id: entry.resource_id,
        })
    }
}

// Identity is the registry index; the catalog entry it points at is derived
// from that index, so hashing or comparing the rest would be redundant work.
impl PartialEq for BlockTickType {
    fn eq(&self, other: &Self) -> bool {
        self.block_type_id == other.block_type_id
    }
}

impl Eq for BlockTickType {}

impl Hash for BlockTickType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.block_type_id.hash(state);
    }
}

impl fmt::Debug for BlockTickType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("BlockTickType")
            .field(&self.resource_id)
            .finish()
    }
}

/// A fluid scheduled tick's type: one of the five `Fluids` singletons.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FluidTickType(VanillaFluidIdentity);

impl FluidTickType {
    pub const fn of_fluid(identity: VanillaFluidIdentity) -> Self {
        Self(identity)
    }

    pub const fn identity(self) -> VanillaFluidIdentity {
        self.0
    }
}

impl TickType for FluidTickType {
    fn resource_id(self) -> &'static str {
        self.0.resource_id()
    }

    fn from_resource_id(resource_id: &str) -> Option<Self> {
        VanillaFluidIdentity::for_resource_id(resource_id).map(Self)
    }
}

impl Hash for FluidTickType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.vanilla_registry_id().hash(state);
    }
}

/// `TickPriority` (`net/minecraft/world/ticks/TickPriority.java`).
///
/// `DRAIN_ORDER` compares priorities with `Enum.compareTo`
/// (`ScheduledTick.java`), i.e. by declaration order, which the derived
/// `Ord` reproduces exactly. The `-3..=3` int is only the saved/codec form
/// (`TickPriority.java`), so it is kept separate from the ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum TickPriority {
    ExtremelyHigh,
    VeryHigh,
    High,
    #[default]
    Normal,
    Low,
    VeryLow,
    ExtremelyLow,
}

impl TickPriority {
    /// Declaration order, in vanilla `TickPriority.java:6-12`.
    pub const ALL: [Self; 7] = [
        Self::ExtremelyHigh,
        Self::VeryHigh,
        Self::High,
        Self::Normal,
        Self::Low,
        Self::VeryLow,
        Self::ExtremelyLow,
    ];

    /// `TickPriority.getValue()`.
    pub const fn value(self) -> i32 {
        match self {
            Self::ExtremelyHigh => -3,
            Self::VeryHigh => -2,
            Self::High => -1,
            Self::Normal => 0,
            Self::Low => 1,
            Self::VeryLow => 2,
            Self::ExtremelyLow => 3,
        }
    }

    /// `TickPriority.byValue()` (`TickPriority.java`): an out-of-range
    /// saved value clamps to the nearest end instead of failing the load.
    pub const fn by_value(value: i32) -> Self {
        match value {
            -3 => Self::ExtremelyHigh,
            -2 => Self::VeryHigh,
            -1 => Self::High,
            0 => Self::Normal,
            1 => Self::Low,
            2 => Self::VeryLow,
            3 => Self::ExtremelyLow,
            other if other < -3 => Self::ExtremelyHigh,
            _ => Self::ExtremelyLow,
        }
    }
}

/// A pending tick in memory (`ScheduledTick`, `ScheduledTick.java`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduledTick<T> {
    pub tick_type: T,
    pub pos: BlockPos,
    pub trigger_tick: i64,
    pub priority: TickPriority,
    pub sub_tick_order: i64,
}

impl<T: TickType> ScheduledTick<T> {
    /// The `NORMAL`-priority constructor (`ScheduledTick.java`).
    pub fn new(tick_type: T, pos: BlockPos, trigger_tick: i64, sub_tick_order: i64) -> Self {
        Self {
            tick_type,
            pos,
            trigger_tick,
            priority: TickPriority::Normal,
            sub_tick_order,
        }
    }

    pub fn with_priority(
        tick_type: T,
        pos: BlockPos,
        trigger_tick: i64,
        priority: TickPriority,
        sub_tick_order: i64,
    ) -> Self {
        Self {
            tick_type,
            pos,
            trigger_tick,
            priority,
            sub_tick_order,
        }
    }

    /// `ScheduledTick.toSavedTick` (`ScheduledTick.java`): the saved
    /// delay is the remaining ticks, narrowed to `int` exactly as vanilla's
    /// `(int)` cast does.
    pub fn to_saved_tick(self, current_tick: i64) -> SavedTick<T> {
        SavedTick {
            tick_type: self.tick_type,
            pos: self.pos,
            delay: self.trigger_tick.wrapping_sub(current_tick) as i32,
            priority: self.priority,
        }
    }

    /// The `UNIQUE_TICK_HASH` key (`ScheduledTick.java`): position and
    /// type only.
    pub fn unique_key(&self) -> UniqueTickKey<T> {
        unique_tick_key(self.pos, self.tick_type)
    }
}

/// A pending tick as saved (`SavedTick`, `SavedTick.java`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedTick<T> {
    pub tick_type: T,
    pub pos: BlockPos,
    pub delay: i32,
    pub priority: TickPriority,
}

impl<T: TickType> SavedTick<T> {
    /// `SavedTick.unpack` (`SavedTick.java`).
    pub fn unpack(self, current_tick: i64, current_sub_tick: i64) -> ScheduledTick<T> {
        ScheduledTick {
            tick_type: self.tick_type,
            pos: self.pos,
            trigger_tick: current_tick + i64::from(self.delay),
            priority: self.priority,
            sub_tick_order: current_sub_tick,
        }
    }

    /// Reads the save schema's row (`persistence::Java26SavedTick`, the
    /// `i`/`x`/`y`/`z`/`t`/`p` fields of `SavedTick.codec`).
    pub fn from_saved(saved: &Java26SavedTick) -> Result<Self, ChunkError> {
        let tick_type = T::from_resource_id(&saved.id).ok_or_else(|| {
            ChunkError(format!(
                "saved tick names {}, which is not a built-in Vanilla 26.1 tick type",
                saved.id
            ))
        })?;
        Ok(Self {
            tick_type,
            pos: BlockPos {
                x: saved.x,
                y: saved.y,
                z: saved.z,
            },
            delay: saved.delay,
            priority: TickPriority::by_value(saved.priority),
        })
    }

    /// The inverse of [`SavedTick::from_saved`].
    pub fn to_saved(self) -> Java26SavedTick {
        Java26SavedTick {
            id: self.tick_type.resource_id().to_string(),
            x: self.pos.x,
            y: self.pos.y,
            z: self.pos.z,
            delay: self.delay,
            priority: self.priority.value(),
        }
    }
}

pub type UniqueTickKey<T> = (T, i32, i32, i32);

/// The same key for a position and type that has no tick yet, which is what
/// `ScheduledTick.probe` (`ScheduledTick.java`) builds a throwaway tick
/// for. Only the position and the type are in the key, so the probe is the key.
pub fn unique_tick_key<T: TickType>(pos: BlockPos, tick_type: T) -> UniqueTickKey<T> {
    (tick_type, pos.x, pos.y, pos.z)
}

/// `ScheduledTick.DRAIN_ORDER` (`ScheduledTick.java`), reversed.
///
/// Vanilla drains from a min-heap `PriorityQueue`; `BinaryHeap` is a max-heap,
/// so the comparison is inverted here and `peek`/`pop` yield the tick vanilla
/// would `peek`/`poll`.
#[derive(Debug, Clone)]
struct DrainOrder<T>(ScheduledTick<T>);

impl<T> DrainOrder<T> {
    fn key(&self) -> (i64, TickPriority, i64) {
        (self.0.trigger_tick, self.0.priority, self.0.sub_tick_order)
    }
}

// Equality here is the ordering key, not the whole tick: `Ord` must agree with
// `Eq`, and DRAIN_ORDER deliberately ignores pos and type.
impl<T> PartialEq for DrainOrder<T> {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl<T> Eq for DrainOrder<T> {}

impl<T> Ord for DrainOrder<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        other.key().cmp(&self.key())
    }
}

impl<T> PartialOrd for DrainOrder<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Default)]
struct FastTickHasher {
    hash: u64,
}

impl FastTickHasher {
    #[inline]
    fn mix(&mut self, value: u64) {
        self.hash = (self.hash.rotate_left(5) ^ value).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
}

impl std::hash::Hasher for FastTickHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }

    #[inline]
    fn write(&mut self, mut bytes: &[u8]) {
        while bytes.len() >= 8 {
            let (chunk, rest) = bytes.split_at(8);
            self.mix(u64::from_ne_bytes(chunk.try_into().unwrap()));
            bytes = rest;
        }
        if bytes.len() >= 4 {
            let (chunk, rest) = bytes.split_at(4);
            self.mix(u64::from(u32::from_ne_bytes(chunk.try_into().unwrap())));
            bytes = rest;
        }
        for &b in bytes {
            self.mix(u64::from(b));
        }
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.mix(u64::from(i));
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.mix(i);
    }

    #[inline]
    fn write_i32(&mut self, i: i32) {
        self.mix(i as u64);
    }
}

type FastTickSet<K> = HashSet<K, std::hash::BuildHasherDefault<FastTickHasher>>;

/// One chunk's pending ticks of one kind (`LevelChunkTicks`,
/// `net/minecraft/world/ticks/LevelChunkTicks.java`).
#[derive(Debug, Clone)]
pub struct ChunkTicks<T: TickType> {
    queue: BinaryHeap<DrainOrder<T>>,
    ticks_per_position: FastTickSet<UniqueTickKey<T>>,
    /// Ticks read from a save and not yet given a trigger tick
    /// (`LevelChunkTicks.java`, `:91-114`). They count towards
    /// [`ChunkTicks::count`] but never drain until [`ChunkTicks::unpack`].
    pending: Option<Vec<SavedTick<T>>>,
    generation_scheduling_open: bool,
}

impl<T: TickType> Default for ChunkTicks<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: TickType> ChunkTicks<T> {
    pub fn new() -> Self {
        Self {
            queue: BinaryHeap::new(),
            ticks_per_position: FastTickSet::default(),
            pending: None,
            generation_scheduling_open: true,
        }
    }

    /// The saved-ticks constructor (`LevelChunkTicks.java`): the
    /// positions are claimed immediately, so a block that schedules before the
    /// chunk is unpacked cannot displace a saved tick, but the ticks
    /// themselves only enter the queue at [`ChunkTicks::unpack`].
    pub fn with_pending(pending: Vec<SavedTick<T>>) -> Self {
        let ticks_per_position = pending
            .iter()
            .map(|tick| (tick.tick_type, tick.pos.x, tick.pos.y, tick.pos.z))
            .collect();
        Self {
            queue: BinaryHeap::new(),
            ticks_per_position,
            pending: Some(pending),
            generation_scheduling_open: true,
        }
    }

    /// Builds the container from the save schema's rows.
    pub fn from_saved(saved: &[Java26SavedTick]) -> Result<Self, ChunkError> {
        saved
            .iter()
            .map(SavedTick::from_saved)
            .collect::<Result<Vec<_>, _>>()
            .map(Self::with_pending)
    }

    /// `LevelChunkTicks.peek` (`:37-39`).
    pub fn peek(&self) -> Option<&ScheduledTick<T>> {
        self.queue.peek().map(|entry| &entry.0)
    }

    /// `LevelChunkTicks.poll` (`:41-48`): the position is released here, so
    /// the block being ticked may schedule itself again right away.
    pub fn poll(&mut self) -> Option<ScheduledTick<T>> {
        let entry = self.queue.pop()?;
        self.ticks_per_position.remove(&entry.0.unique_key());
        Some(entry.0)
    }

    /// `LevelChunkTicks.schedule` (`:51-55`).
    ///
    /// Returns whether the tick was accepted. Vanilla returns nothing, but the
    /// boolean is the observable behavior: with a tick already pending for the
    /// same (position, type) the new one is silently dropped, whatever its
    /// trigger tick or priority. First scheduler wins.
    pub fn schedule(&mut self, tick: ScheduledTick<T>) -> bool {
        if !self.ticks_per_position.insert(tick.unique_key()) {
            return false;
        }
        self.schedule_unchecked(tick);
        true
    }

    /// Saves a generation-stage tick with zero delay.
    ///
    /// `net/minecraft/world/ticks/ProtoChunkTicks.java`.
    pub fn schedule_for_generation(
        &mut self,
        tick_type: T,
        pos: BlockPos,
        priority: TickPriority,
    ) -> bool {
        assert!(
            self.generation_scheduling_open,
            "generation ticks cannot be scheduled after pending ticks were unpacked"
        );
        if !self
            .ticks_per_position
            .insert(unique_tick_key(pos, tick_type))
        {
            return false;
        }
        self.pending.get_or_insert_default().push(SavedTick {
            tick_type,
            pos,
            delay: 0,
            priority,
        });
        true
    }

    /// Appends one saved tick while keeping pending ticks in saved form (or
    /// falling back to re-packing only if queued ticks already exist).
    pub fn push_saved_tick(&mut self, tick: SavedTick<T>) -> bool {
        if !self
            .ticks_per_position
            .insert(unique_tick_key(tick.pos, tick.tick_type))
        {
            return false;
        }
        if self.queue.is_empty() {
            self.pending.get_or_insert_default().push(tick);
        } else {
            let mut packed = self.pack(0);
            packed.push(tick);
            *self = Self::with_pending(packed);
        }
        true
    }

    /// `LevelChunkTicks.scheduleUnchecked` (`:57-62`): queue only, no
    /// deduplication. Only the saved-tick path uses this, because
    /// [`ChunkTicks::with_pending`] already claimed those positions.
    fn schedule_unchecked(&mut self, tick: ScheduledTick<T>) {
        self.queue.push(DrainOrder(tick));
    }

    /// `LevelChunkTicks.hasScheduledTick` (`:65-67`).
    pub fn has_scheduled_tick(&self, pos: BlockPos, tick_type: T) -> bool {
        self.ticks_per_position
            .contains(&(tick_type, pos.x, pos.y, pos.z))
    }

    pub fn scheduled_tick_keys(&self) -> impl Iterator<Item = (BlockPos, T)> + '_ {
        self.ticks_per_position
            .iter()
            .map(|&(tick_type, x, y, z)| (BlockPos { x, y, z }, tick_type))
    }

    /// `LevelChunkTicks.getAll` (`:80-82`): every queued tick, in the heap's
    /// own order. Use [`ChunkTicks::drain_ordered`] when order matters.
    pub fn iter(&self) -> impl Iterator<Item = &ScheduledTick<T>> {
        self.queue.iter().map(|entry| &entry.0)
    }

    /// Every queued tick in `DRAIN_ORDER`, without draining the queue.
    pub fn drain_ordered(&self) -> Vec<ScheduledTick<T>> {
        let mut entries: Vec<&DrainOrder<T>> = self.queue.iter().collect();
        // `Ord` is the reversed DRAIN_ORDER, so descending by it is ascending
        // in drain order.
        entries.sort_by(|a, b| b.cmp(a));
        entries.into_iter().map(|entry| entry.0).collect()
    }

    /// `LevelChunkTicks.count` (`:84-87`): queued plus not-yet-unpacked.
    pub fn count(&self) -> usize {
        self.queue.len() + self.pending.as_ref().map_or(0, Vec::len)
    }

    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// `LevelChunkTicks.pack` (`:89-100`): saved ticks that were never
    /// unpacked pass through untouched, and queued ticks become delays
    /// relative to `current_tick`.
    ///
    /// Vanilla emits the queued ticks in the `PriorityQueue`'s internal array
    /// order, which is unspecified. This packs them in `DRAIN_ORDER` instead:
    /// [`ChunkTicks::unpack`] numbers restored ticks by list position, so a
    /// drain-ordered list is the one that restores the same drain order.
    pub fn pack(&self, current_tick: i64) -> Vec<SavedTick<T>> {
        let mut packed = Vec::with_capacity(self.count());
        if let Some(pending) = &self.pending {
            packed.extend(pending.iter().copied());
        }
        packed.extend(
            self.drain_ordered()
                .into_iter()
                .map(|tick| tick.to_saved_tick(current_tick)),
        );
        packed
    }

    /// [`ChunkTicks::pack`] in the save schema's shape.
    pub fn pack_saved(&self, current_tick: i64) -> Vec<Java26SavedTick> {
        self.pack(current_tick)
            .into_iter()
            .map(SavedTick::to_saved)
            .collect()
    }

    /// `LevelChunkTicks.unpack` (`:102-114`).
    ///
    /// The restored sub-tick orders run from `-pendingTicks.size()` upwards,
    /// so every restored tick sorts before every tick scheduled during this
    /// run (whose sub-tick orders start at zero and count up).
    pub fn unpack(&mut self, current_tick: i64) {
        self.generation_scheduling_open = false;
        let Some(pending) = self.pending.take() else {
            return;
        };
        let sub_tick_base = -(pending.len() as i64);
        for (offset, saved) in pending.into_iter().enumerate() {
            self.schedule_unchecked(saved.unpack(current_tick, sub_tick_base + offset as i64));
        }
    }
}

// Two containers are equal when they hold the same ticks and generation phase;
// the heap's internal array order is an implementation detail.
impl<T: TickType> PartialEq for ChunkTicks<T> {
    fn eq(&self, other: &Self) -> bool {
        self.pending == other.pending
            && self.generation_scheduling_open == other.generation_scheduling_open
            && self.drain_ordered() == other.drain_ordered()
    }
}

impl<T: TickType> Eq for ChunkTicks<T> {}

#[cfg(test)]
mod tests;
