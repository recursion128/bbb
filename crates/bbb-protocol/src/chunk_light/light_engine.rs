//! Block and sky light propagation over section layers shared by both world hosts.

use std::collections::{BTreeSet, VecDeque};
use std::sync::OnceLock;

use bbb_facts::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_light_face_occlusion_index,
    vanilla_block_state_render_facts, vanilla_block_state_support_facts,
    vanilla_face_shape_union_occludes, VanillaBlockSupportDirection, VANILLA_BLOCK_STATE_COUNT,
};
use bbb_facts::block_types::{VANILLA_BLOCK_BEDROCK_ID, VANILLA_BLOCK_VOID_AIR_ID};

use crate::content_registry::ContentRegistry;

use super::light_storage::ChunkLightStorage;
pub use super::{LightError, LightResult};

const MAX_QUEUE_VISITS: usize = 64 * 1024 * 1024;
const MAX_POOLED_QUEUE_CAPACITY: usize = 16 * 1024;

type LightQueuePair = (VecDeque<(Pos, Entry)>, VecDeque<(Pos, Entry)>);

thread_local! {
    static LIGHT_QUEUE_POOL: std::cell::RefCell<Option<LightQueuePair>> =
        const { std::cell::RefCell::new(None) };
}

struct PooledLightQueues {
    increase: VecDeque<(Pos, Entry)>,
    decrease: VecDeque<(Pos, Entry)>,
}

impl PooledLightQueues {
    fn take() -> Self {
        let (increase, decrease) = LIGHT_QUEUE_POOL
            .try_with(|pool| pool.borrow_mut().take())
            .ok()
            .flatten()
            .unwrap_or_else(|| (VecDeque::new(), VecDeque::new()));
        Self { increase, decrease }
    }
}

impl Drop for PooledLightQueues {
    fn drop(&mut self) {
        let mut increase = std::mem::take(&mut self.increase);
        let mut decrease = std::mem::take(&mut self.decrease);
        increase.clear();
        decrease.clear();
        if increase.capacity() > MAX_POOLED_QUEUE_CAPACITY {
            increase = VecDeque::new();
        }
        if decrease.capacity() > MAX_POOLED_QUEUE_CAPACITY {
            decrease = VecDeque::new();
        }
        let _ = LIGHT_QUEUE_POOL.try_with(|pool| {
            if let Ok(mut slot) = pool.try_borrow_mut() {
                if slot.is_none() {
                    *slot = Some((increase, decrease));
                }
            }
        });
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LightFacts {
    pub emission: u8,
    dampening: u8,
    empty_shape: bool,
    use_shape: bool,
    faces: [u8; 6],
}

impl LightFacts {
    pub fn sky_light_properties_differ(self, other: Self) -> bool {
        self.dampening != other.dampening || self.use_shape || other.use_shape
    }
}

/// Per-state light facts, built once for every Vanilla block state.
pub fn light_facts(state: i32) -> Option<LightFacts> {
    static TABLE: OnceLock<Vec<Option<LightFacts>>> = OnceLock::new();
    let table = TABLE.get_or_init(|| (0..VANILLA_BLOCK_STATE_COUNT).map(compute_facts).collect());
    *table.get(usize::try_from(state).ok()?)?
}

/// Per-state light facts for both Vanilla and session-registered modded blocks.
pub fn light_facts_with_registry(state: i32, registry: &ContentRegistry) -> Option<LightFacts> {
    if state < VANILLA_BLOCK_STATE_COUNT {
        return light_facts(state);
    }
    let (spec, _offset) = registry.block_by_state_id(state)?;
    Some(LightFacts {
        emission: spec.light_emission.min(15),
        dampening: if spec.full_cube_collision { 15 } else { 0 },
        empty_shape: true,
        use_shape: false,
        faces: [0; 6],
    })
}

fn compute_facts(state: i32) -> Option<LightFacts> {
    let render = vanilla_block_state_render_facts(state)?;
    let support = vanilla_block_state_support_facts(state)?;
    let mut faces = [0; 6];
    for direction in Direction::ALL {
        faces[direction.index()] =
            vanilla_block_state_light_face_occlusion_index(state, direction.support())?;
    }
    Some(LightFacts {
        emission: render.light_emission(),
        dampening: support.light_dampening(),
        empty_shape: !support.can_occlude() || !render.use_shape_for_light_occlusion(),
        use_shape: render.use_shape_for_light_occlusion(),
        faces,
    })
}

/// `LightEngine.hasDifferentLightProperties` (`LightEngine.java:41-48`).
pub fn has_different_light_properties(old: i32, new: i32) -> bool {
    has_different_light_properties_by(old, new, light_facts)
}

/// Registry-aware [`has_different_light_properties`].
pub fn has_different_light_properties_with_registry(
    old: i32,
    new: i32,
    registry: &ContentRegistry,
) -> bool {
    has_different_light_properties_by(old, new, |state| light_facts_with_registry(state, registry))
}

#[inline]
fn has_different_light_properties_by(
    old: i32,
    new: i32,
    facts_for: impl Fn(i32) -> Option<LightFacts>,
) -> bool {
    if old == new {
        return false;
    }
    match (facts_for(old), facts_for(new)) {
        (Some(old), Some(new)) => {
            old.emission != new.emission || old.sky_light_properties_differ(new)
        }
        _ => true,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Direction {
    const ALL: [Self; 6] = [
        Self::Down,
        Self::Up,
        Self::North,
        Self::South,
        Self::West,
        Self::East,
    ];

    const fn index(self) -> usize {
        self as usize
    }

    const fn support(self) -> VanillaBlockSupportDirection {
        match self {
            Self::Down => VanillaBlockSupportDirection::Down,
            Self::Up => VanillaBlockSupportDirection::Up,
            Self::North => VanillaBlockSupportDirection::North,
            Self::South => VanillaBlockSupportDirection::South,
            Self::West => VanillaBlockSupportDirection::West,
            Self::East => VanillaBlockSupportDirection::East,
        }
    }

    const fn opposite(self) -> Self {
        match self {
            Self::Down => Self::Up,
            Self::Up => Self::Down,
            Self::North => Self::South,
            Self::South => Self::North,
            Self::West => Self::East,
            Self::East => Self::West,
        }
    }

    const fn delta(self) -> (i32, i32, i32) {
        match self {
            Self::Down => (0, -1, 0),
            Self::Up => (0, 1, 0),
            Self::North => (0, 0, -1),
            Self::South => (0, 0, 1),
            Self::West => (-1, 0, 0),
            Self::East => (1, 0, 0),
        }
    }
}

/// `LightEngine.QueueEntry` (`LightEngine.java:213-323`).
#[derive(Clone, Copy)]
struct Entry {
    level: u8,
    directions: u8,
    from_empty_shape: bool,
    from_emission: bool,
}

impl Entry {
    const ALL: u8 = 0b11_1111;

    const fn all(level: u8) -> Self {
        Self {
            level,
            directions: Self::ALL,
            from_empty_shape: false,
            from_emission: false,
        }
    }

    const fn emission(level: u8, from_empty_shape: bool) -> Self {
        Self {
            level,
            directions: Self::ALL,
            from_empty_shape,
            from_emission: true,
        }
    }

    const fn skip(level: u8, from_empty_shape: bool, direction: Direction) -> Self {
        Self {
            level,
            directions: Self::ALL & !(1 << direction.index()),
            from_empty_shape,
            from_emission: false,
        }
    }

    const fn only(level: u8, direction: Direction) -> Self {
        Self {
            level,
            directions: 1 << direction.index(),
            from_empty_shape: false,
            from_emission: false,
        }
    }

    const fn sky_source(down: bool, north: bool, south: bool, west: bool, east: bool) -> Self {
        let directions = (down as u8) << Direction::Down.index()
            | (north as u8) << Direction::North.index()
            | (south as u8) << Direction::South.index()
            | (west as u8) << Direction::West.index()
            | (east as u8) << Direction::East.index();
        Self {
            level: 15,
            directions,
            from_empty_shape: false,
            from_emission: false,
        }
    }

    const fn propagates(self, direction: Direction) -> bool {
        self.directions & (1 << direction.index()) != 0
    }
}

type Pos = (i32, i32, i32);

/// One light pass over `storage`, reading blocks through the host's column accessors.
pub struct LightEngine<'a, B, H> {
    block_at: B,
    has_column: H,
    storage: &'a mut ChunkLightStorage,
    registry: Option<&'a ContentRegistry>,
    sky: bool,
    queues: PooledLightQueues,
    slot: Option<((i32, i32), Option<usize>)>,
    block: Option<((i32, i32), bool)>,
    air: LightFacts,
    void_air: i32,
    bedrock: LightFacts,
    visits: usize,
}

impl<'a, B, H> LightEngine<'a, B, H>
where
    B: FnMut(Pos) -> Option<i32>,
    H: Fn((i32, i32)) -> bool,
{
    pub fn new(
        storage: &'a mut ChunkLightStorage,
        block_at: B,
        has_column: H,
        air_state: i32,
    ) -> LightResult<Self> {
        Self::new_inner(storage, block_at, has_column, air_state, None)
    }

    pub fn new_with_registry(
        storage: &'a mut ChunkLightStorage,
        block_at: B,
        has_column: H,
        air_state: i32,
        registry: &'a ContentRegistry,
    ) -> LightResult<Self> {
        Self::new_inner(storage, block_at, has_column, air_state, Some(registry))
    }

    fn new_inner(
        storage: &'a mut ChunkLightStorage,
        block_at: B,
        has_column: H,
        air_state: i32,
        registry: Option<&'a ContentRegistry>,
    ) -> LightResult<Self> {
        let void_air =
            vanilla_block_default_state_id(VANILLA_BLOCK_VOID_AIR_ID).ok_or_else(|| {
                invalid_engine("Vanilla minecraft:void_air state is absent".to_string())
            })?;
        let bedrock =
            vanilla_block_default_state_id(VANILLA_BLOCK_BEDROCK_ID).ok_or_else(|| {
                invalid_engine("Vanilla minecraft:bedrock state is absent".to_string())
            })?;
        Ok(Self {
            block_at,
            has_column,
            storage,
            registry,
            sky: false,
            queues: PooledLightQueues::take(),
            slot: None,
            block: None,
            air: facts_for(registry, air_state)?,
            void_air,
            bedrock: facts_for(registry, bedrock)?,
            visits: 0,
        })
    }

    pub fn set_sky(&mut self, sky: bool) {
        self.sky = sky;
    }

    #[inline]
    fn slot_of(&mut self, chunk: (i32, i32)) -> Option<usize> {
        match self.slot {
            Some((cached, slot)) if cached == chunk => slot,
            _ => {
                let slot = self.storage.slot(chunk);
                self.slot = Some((chunk, slot));
                slot
            }
        }
    }

    /// The stored level of a storing section.
    #[inline]
    fn stored(&mut self, pos: Pos) -> Option<u8> {
        let slot = self.slot_of((pos.0 >> 4, pos.2 >> 4))?;
        self.storage.level_at(slot, pos, self.sky)
    }

    #[inline]
    fn set(&mut self, pos: Pos, value: u8) {
        if let Some(slot) = self.slot_of((pos.0 >> 4, pos.2 >> 4)) {
            self.storage.set_level_at(slot, pos, self.sky, value);
        }
    }

    fn section_storing(&mut self, chunk: (i32, i32), section_y: i32) -> bool {
        self.slot_of(chunk)
            .is_some_and(|slot| self.storage.section_storing_at(slot, section_y))
    }

    fn light_on(&mut self, chunk: (i32, i32)) -> bool {
        self.slot_of(chunk)
            .is_some_and(|slot| self.storage.enabled_at(slot))
    }

    #[inline]
    fn facts_at(&mut self, pos: Pos) -> LightResult<LightFacts> {
        let chunk = (pos.0 >> 4, pos.2 >> 4);
        let present = match self.block {
            Some((cached, present)) if cached == chunk => present,
            _ => {
                let present = (self.has_column)(chunk);
                self.block = Some((chunk, present));
                present
            }
        };
        if present {
            let state = (self.block_at)(pos).unwrap_or(self.void_air);
            Ok(match self.registry {
                Some(registry) => light_facts_with_registry(state, registry),
                None => light_facts(state),
            }
            .unwrap_or(self.air))
        } else {
            Ok(self.bedrock)
        }
    }

    fn visit(&mut self) -> LightResult<()> {
        self.visits += 1;
        if self.visits > MAX_QUEUE_VISITS {
            return Err(invalid_engine(
                "light queue exceeds the fixed operation budget",
            ));
        }
        Ok(())
    }

    /// `LightEngine.runLightUpdates` (`LightEngine.java:137-189`).
    pub fn run(&mut self) -> LightResult<()> {
        while let Some((from, entry)) = self.queues.decrease.pop_front() {
            self.visit()?;
            self.propagate_decrease(from, entry)?;
        }
        while let Some((from, entry)) = self.queues.increase.pop_front() {
            self.visit()?;
            let Some(mut from_level) = self.stored(from) else {
                continue;
            };
            if entry.from_emission && from_level < entry.level {
                self.set(from, entry.level);
                from_level = entry.level;
            }
            if from_level == entry.level {
                self.propagate_increase(from, entry, from_level)?;
            }
        }
        Ok(())
    }

    fn propagate_increase(&mut self, from: Pos, entry: Entry, from_level: u8) -> LightResult<()> {
        let empty_below = if self.sky {
            self.count_empty_sections_below(from)
        } else {
            0
        };
        let mut from_facts = None;
        for direction in Direction::ALL {
            if !entry.propagates(direction) {
                continue;
            }
            let to = offset(from, direction)?;
            if self.sky && empty_below > 0 && crossed_section_edge(direction, to.0, to.2) {
                self.propagate_from_empty_sections(to, direction, from_level, true, empty_below)?;
            }
            let Some(to_level) = self.stored(to) else {
                continue;
            };
            if from_level.saturating_sub(1) <= to_level {
                continue;
            }
            let to_facts = self.facts_at(to)?;
            let new_level = from_level.saturating_sub(to_facts.dampening.max(1));
            if new_level <= to_level {
                continue;
            }
            let from_facts = match from_facts {
                Some(facts) => facts,
                None => {
                    let facts = if entry.from_empty_shape {
                        self.air
                    } else {
                        self.facts_at(from)?
                    };
                    from_facts = Some(facts);
                    facts
                }
            };
            if shape_occludes(from_facts, to_facts, direction)? {
                continue;
            }
            self.set(to, new_level);
            if new_level > 1 {
                self.queues.increase.push_back((
                    to,
                    Entry::skip(new_level, to_facts.empty_shape, direction.opposite()),
                ));
            }
        }
        Ok(())
    }

    /// `BlockLightEngine.propagateDecrease` and `SkyLightEngine.propagateDecrease`.
    fn propagate_decrease(&mut self, from: Pos, entry: Entry) -> LightResult<()> {
        let empty_below = if self.sky {
            self.count_empty_sections_below(from)
        } else {
            0
        };
        for direction in Direction::ALL {
            if !entry.propagates(direction) {
                continue;
            }
            let to = offset(from, direction)?;
            if self.sky && empty_below > 0 && crossed_section_edge(direction, to.0, to.2) {
                self.propagate_from_empty_sections(to, direction, entry.level, false, empty_below)?;
            }
            let Some(to_level) = self.stored(to) else {
                continue;
            };
            if to_level == 0 {
                continue;
            }
            if to_level >= entry.level {
                self.queues
                    .increase
                    .push_back((to, Entry::only(to_level, direction.opposite())));
                continue;
            }
            self.set(to, 0);
            if self.sky {
                self.queues
                    .decrease
                    .push_back((to, Entry::skip(to_level, false, direction.opposite())));
                continue;
            }
            let to_facts = self.facts_at(to)?;
            let emission = self.emission(to, to_facts);
            if emission < to_level {
                self.queues
                    .decrease
                    .push_back((to, Entry::skip(to_level, false, direction.opposite())));
            }
            if emission > 0 {
                self.queues
                    .increase
                    .push_back((to, Entry::emission(emission, to_facts.empty_shape)));
            }
        }
        Ok(())
    }

    fn emission(&mut self, pos: Pos, facts: LightFacts) -> u8 {
        if facts.emission > 0 && self.light_on((pos.0 >> 4, pos.2 >> 4)) {
            facts.emission
        } else {
            0
        }
    }

    fn count_empty_sections_below(&mut self, pos: Pos) -> i32 {
        if pos.1 & 15 != 0 {
            return 0;
        }
        let (local_x, local_z) = (pos.0 & 15, pos.2 & 15);
        if local_x != 0 && local_x != 15 && local_z != 0 && local_z != 15 {
            return 0;
        }
        let chunk = (pos.0 >> 4, pos.2 >> 4);
        let section_y = pos.1 >> 4;
        let mut count = 0;
        while !self.section_storing(chunk, section_y - count - 1)
            && self
                .storage
                .has_light_data_at_or_below(section_y - count - 1)
        {
            count += 1;
        }
        count
    }

    /// `SkyLightEngine.propagateFromEmptySections` (`SkyLightEngine.java:221-265`).
    fn propagate_from_empty_sections(
        &mut self,
        to: Pos,
        direction: Direction,
        from_level: u8,
        increase: bool,
        empty_below: i32,
    ) -> LightResult<()> {
        if empty_below == 0 || !crossed_section_edge(direction, to.0, to.2) {
            return Ok(());
        }
        let chunk = (to.0 >> 4, to.2 >> 4);
        let mut section_y = (to.1 >> 4) - 1;
        let bottom = section_y - empty_below + 1;
        while section_y >= bottom {
            if self.section_storing(chunk, section_y) {
                for local_y in (0..16).rev() {
                    let pos = (to.0, section_y * 16 + local_y, to.2);
                    let Some(existing) = self.stored(pos) else {
                        continue;
                    };
                    if increase {
                        if from_level.saturating_sub(1) <= existing {
                            continue;
                        }
                        let to_facts = self.facts_at(pos)?;
                        let new_level = from_level.saturating_sub(to_facts.dampening.max(1));
                        if new_level <= existing {
                            continue;
                        }
                        if shape_occludes(self.air, to_facts, direction)? {
                            continue;
                        }
                        self.set(pos, new_level);
                        if new_level > 1 {
                            self.queues.increase.push_back((
                                pos,
                                Entry::skip(new_level, to_facts.empty_shape, direction.opposite()),
                            ));
                        }
                    } else {
                        if existing == 0 {
                            continue;
                        }
                        if existing >= from_level {
                            self.queues
                                .increase
                                .push_back((pos, Entry::only(existing, direction.opposite())));
                            continue;
                        }
                        self.set(pos, 0);
                        self.queues
                            .decrease
                            .push_back((pos, Entry::skip(existing, false, direction.opposite())));
                    }
                }
            }
            section_y -= 1;
        }
        Ok(())
    }

    pub fn queue_block_source(&mut self, pos: Pos, facts: LightFacts) {
        self.queues
            .increase
            .push_back((pos, Entry::emission(facts.emission, facts.empty_shape)));
    }

    fn lowest_source(&self, x: i32, z: i32) -> i32 {
        self.storage.lowest_sky_source(x, z).unwrap_or(i32::MIN)
    }

    /// `SkyLightEngine.propagateLightSources` (`SkyLightEngine.java:301-357`).
    pub fn propagate_sky_sources(&mut self, chunk: (i32, i32)) -> LightResult<()> {
        let (Some(top_section), Some(bottom_section)) = (
            self.storage.top_section_y(chunk),
            self.storage.bottom_section_y(),
        ) else {
            return Ok(());
        };
        let (base_x, base_z) = (chunk.0 * 16, chunk.1 * 16);
        for section_y in (bottom_section..top_section).rev() {
            if !self.section_storing(chunk, section_y) {
                continue;
            }
            let section_min_y = section_y * 16;
            let section_max_y = section_min_y + 15;
            let mut sources_below = false;
            for z in 0..16 {
                for x in 0..16 {
                    let (x, z) = (base_x + x, base_z + z);
                    let lowest = self.lowest_source(x, z);
                    if lowest > section_max_y {
                        continue;
                    }
                    let north = self.lowest_source(x, z - 1);
                    let south = self.lowest_source(x, z + 1);
                    let west = self.lowest_source(x - 1, z);
                    let east = self.lowest_source(x + 1, z);
                    for y in (section_min_y.max(lowest)..=section_max_y).rev() {
                        self.set((x, y, z), 15);
                        if y == lowest || y < north || y < south || y < west || y < east {
                            self.queues.increase.push_back((
                                (x, y, z),
                                Entry::sky_source(
                                    y == lowest,
                                    y < north,
                                    y < south,
                                    y < west,
                                    y < east,
                                ),
                            ));
                        }
                    }
                    if lowest < section_min_y {
                        sources_below = true;
                    }
                }
            }
            if !sources_below {
                break;
            }
        }
        self.run()
    }

    /// Seeds light across a newly lit chunk's four faces in both directions, so
    /// it and its lit neighbours outside `skip` see each other's light.
    pub fn exchange_faces(&mut self, chunk: (i32, i32), skip: &BTreeSet<(i32, i32)>) {
        let Some(bottom) = self.storage.bottom_section_y() else {
            return;
        };
        for direction in [
            Direction::North,
            Direction::South,
            Direction::West,
            Direction::East,
        ] {
            let delta = direction.delta();
            let neighbor = (chunk.0 + delta.0, chunk.1 + delta.2);
            if skip.contains(&neighbor) || !(self.has_column)(neighbor) {
                continue;
            }
            let top = self
                .storage
                .top_section_y(chunk)
                .max(self.storage.top_section_y(neighbor))
                .unwrap_or(bottom);
            for section_y in bottom..top {
                if !self.section_storing(chunk, section_y)
                    || !self.section_storing(neighbor, section_y)
                {
                    continue;
                }
                for local_y in 0..16 {
                    for along in 0..16 {
                        let y = section_y * 16 + local_y;
                        let (inside, outside) = match direction {
                            Direction::North => ((along, y, 0), (along, y, -1)),
                            Direction::South => ((along, y, 15), (along, y, 16)),
                            Direction::West => ((0, y, along), (-1, y, along)),
                            _ => ((15, y, along), (16, y, along)),
                        };
                        let shift = |(x, y, z): Pos| (chunk.0 * 16 + x, y, chunk.1 * 16 + z);
                        for (from, toward) in [
                            (shift(outside), direction.opposite()),
                            (shift(inside), direction),
                        ] {
                            if let Some(level) = self.stored(from).filter(|&level| level > 1) {
                                self.queues
                                    .increase
                                    .push_back((from, Entry::only(level, toward)));
                            }
                        }
                    }
                }
            }
        }
    }

    /// `BlockLightEngine.checkNode` (`BlockLightEngine.java:26-43`).
    pub fn check_block(&mut self, pos: Pos) -> LightResult<()> {
        let Some(old) = self.stored(pos) else {
            return Ok(());
        };
        let facts = self.facts_at(pos)?;
        let emission = self.emission(pos, facts);
        if emission < old {
            self.set(pos, 0);
            self.queues.decrease.push_back((pos, Entry::all(old)));
        } else {
            self.queues.decrease.push_back((pos, Entry::all(1)));
        }
        if emission > 0 {
            self.queues
                .increase
                .push_back((pos, Entry::emission(emission, facts.empty_shape)));
        }
        Ok(())
    }

    /// `SkyLightEngine.checkNode` (`SkyLightEngine.java:52-79`).
    pub fn check_sky(&mut self, pos: Pos) {
        let chunk = (pos.0 >> 4, pos.2 >> 4);
        let lowest = if self.light_on(chunk) {
            self.storage
                .lowest_sky_source(pos.0, pos.2)
                .unwrap_or(i32::MAX)
        } else {
            i32::MAX
        };
        if lowest != i32::MAX {
            self.update_sources_in_column(pos.0, pos.2, lowest);
        }
        let Some(old) = self.stored(pos) else {
            return;
        };
        if pos.1 >= lowest {
            self.queues
                .decrease
                .push_back((pos, Entry::skip(15, false, Direction::Up)));
            self.queues
                .increase
                .push_back((pos, Entry::skip(15, false, Direction::Up)));
        } else if old > 0 {
            self.set(pos, 0);
            self.queues.decrease.push_back((pos, Entry::all(old)));
        } else {
            self.queues.decrease.push_back((pos, Entry::all(1)));
        }
    }

    fn update_sources_in_column(&mut self, x: i32, z: i32, lowest: i32) {
        let Some(bottom) = self.storage.bottom_section_y() else {
            return;
        };
        let world_bottom = bottom * 16;
        self.remove_sources_below(x, z, lowest, world_bottom);
        self.add_sources_above(x, z, lowest, world_bottom);
    }

    fn remove_sources_below(&mut self, x: i32, z: i32, lowest: i32, world_bottom: i32) {
        if lowest <= world_bottom {
            return;
        }
        let chunk = (x >> 4, z >> 4);
        let start = lowest - 1;
        let mut section_y = start >> 4;
        while self.storage.has_light_data_at_or_below(section_y) {
            if self.section_storing(chunk, section_y) {
                let bottom_y = section_y * 16;
                for y in (bottom_y..=(bottom_y + 15).min(start)).rev() {
                    if self.stored((x, y, z)) != Some(15) {
                        return;
                    }
                    self.set((x, y, z), 0);
                    let entry = if y == start {
                        Entry::all(15)
                    } else {
                        Entry::skip(15, false, Direction::Up)
                    };
                    self.queues.decrease.push_back(((x, y, z), entry));
                }
            }
            section_y -= 1;
        }
    }

    fn add_sources_above(&mut self, x: i32, z: i32, lowest: i32, world_bottom: i32) {
        let chunk = (x >> 4, z >> 4);
        let neighbor_lowest = self
            .lowest_source(x - 1, z)
            .max(self.lowest_source(x + 1, z))
            .max(self.lowest_source(x, z - 1))
            .max(self.lowest_source(x, z + 1));
        let start = lowest.max(world_bottom);
        let Some(top) = self.storage.top_section_y(chunk) else {
            return;
        };
        let mut section_y = start >> 4;
        while section_y < top {
            if self.section_storing(chunk, section_y) {
                let bottom_y = section_y * 16;
                for y in bottom_y.max(start)..=bottom_y + 15 {
                    if self.stored((x, y, z)) == Some(15) {
                        return;
                    }
                    self.set((x, y, z), 15);
                    if y < neighbor_lowest || y == lowest {
                        self.queues
                            .increase
                            .push_back(((x, y, z), Entry::skip(15, false, Direction::Up)));
                    }
                }
            }
            section_y += 1;
        }
    }
}

#[inline]
fn facts_for(registry: Option<&ContentRegistry>, state: i32) -> LightResult<LightFacts> {
    match registry {
        Some(registry) => light_facts_with_registry(state, registry),
        None => light_facts(state),
    }
    .ok_or_else(|| invalid_engine(format!("block state {state} has no light facts")))
}

fn shape_occludes(from: LightFacts, to: LightFacts, direction: Direction) -> LightResult<bool> {
    vanilla_face_shape_union_occludes(
        from.faces[direction.index()],
        to.faces[direction.opposite().index()],
    )
    .ok_or_else(|| invalid_engine("light face union palette lookup failed"))
}

fn crossed_section_edge(direction: Direction, x: i32, z: i32) -> bool {
    match direction {
        Direction::North => z & 15 == 15,
        Direction::South => z & 15 == 0,
        Direction::West => x & 15 == 15,
        Direction::East => x & 15 == 0,
        Direction::Down | Direction::Up => false,
    }
}

fn offset(pos: Pos, direction: Direction) -> LightResult<Pos> {
    let delta = direction.delta();
    match (
        pos.0.checked_add(delta.0),
        pos.1.checked_add(delta.1),
        pos.2.checked_add(delta.2),
    ) {
        (Some(x), Some(y), Some(z)) => Ok((x, y, z)),
        _ => Err(invalid_engine("light propagation overflows")),
    }
}

fn invalid_engine(detail: impl Into<String>) -> LightError {
    LightError::Engine(detail.into())
}

pub fn lowest_sky_source_y(
    min_section_y: i32,
    empty_sections: &[bool],
    air_state: i32,
    state_at: impl FnMut(usize, usize) -> i32,
) -> LightResult<i32> {
    lowest_sky_source_y_by(
        min_section_y,
        empty_sections,
        air_state,
        state_at,
        light_facts,
    )
}

pub fn lowest_sky_source_y_with_registry(
    min_section_y: i32,
    empty_sections: &[bool],
    air_state: i32,
    registry: &ContentRegistry,
    state_at: impl FnMut(usize, usize) -> i32,
) -> LightResult<i32> {
    lowest_sky_source_y_by(
        min_section_y,
        empty_sections,
        air_state,
        state_at,
        |state| light_facts_with_registry(state, registry),
    )
}

fn lowest_sky_source_y_by(
    min_section_y: i32,
    empty_sections: &[bool],
    air_state: i32,
    mut state_at: impl FnMut(usize, usize) -> i32,
    facts_lookup: impl Fn(i32) -> Option<LightFacts>,
) -> LightResult<i32> {
    let Some(highest_non_empty) = empty_sections.iter().rposition(|empty| !empty) else {
        return Ok(i32::MIN);
    };
    let min_y = min_section_y
        .checked_mul(16)
        .ok_or_else(|| invalid_engine("minimum light y overflows"))?;
    // ChunkSkyLightSources.java:47-75 resets the top edge across empty sections.
    let top_section_count = i32::try_from(highest_non_empty + 1)
        .map_err(|_| invalid_engine("top section index exceeds Java int"))?;
    let mut top_y = min_y
        .checked_add(
            top_section_count
                .checked_mul(16)
                .ok_or_else(|| invalid_engine("top section coordinate overflows Java int"))?,
        )
        .ok_or_else(|| invalid_engine("top light edge overflows Java int"))?;
    let mut top_state = air_state;

    for section_index in (0..=highest_non_empty).rev() {
        if empty_sections[section_index] {
            top_state = air_state;
            let section_index = i32::try_from(section_index)
                .map_err(|_| invalid_engine("section index exceeds Java int"))?;
            top_y = min_y
                .checked_add(
                    section_index
                        .checked_mul(16)
                        .ok_or_else(|| invalid_engine("section coordinate overflows Java int"))?,
                )
                .ok_or_else(|| invalid_engine("empty-section light edge overflows Java int"))?;
            continue;
        }
        for y in (0..16).rev() {
            let bottom_state = state_at(section_index, y);
            if sky_edge_occludes(top_state, bottom_state, &facts_lookup)? {
                return Ok(top_y);
            }
            top_state = bottom_state;
            top_y = top_y
                .checked_sub(1)
                .ok_or_else(|| invalid_engine("descending light edge underflows Java int"))?;
        }
    }
    Ok(i32::MIN)
}

fn sky_edge_occludes(
    top_state: i32,
    bottom_state: i32,
    facts_lookup: &impl Fn(i32) -> Option<LightFacts>,
) -> LightResult<bool> {
    let fact = |state: i32| {
        facts_lookup(state)
            .ok_or_else(|| invalid_engine(format!("block state {state} has no light facts")))
    };
    let bottom = fact(bottom_state)?;
    if bottom.dampening != 0 {
        return Ok(true);
    }
    let top = fact(top_state)?;
    vanilla_face_shape_union_occludes(top.faces[0], bottom.faces[1]).ok_or_else(|| {
        invalid_engine(format!(
            "light-face union indices {} and {} are absent",
            top.faces[0], bottom.faces[1]
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content_registry::{ContentRegistryBuilder, ModBlockRegistration};
    use bbb_facts::block_types::VANILLA_BLOCK_AIR_ID;

    fn test_registry() -> ContentRegistry {
        let mut builder = ContentRegistryBuilder::new();
        builder.register_block(ModBlockRegistration {
            resource_id: "demo:glow_cube".to_string(),
            state_count: 1,
            light_emission: 15,
            destroy_speed: 1.0,
            full_cube_collision: true,
            dynamic_light_rgb: None,
        });
        builder.register_block(ModBlockRegistration {
            resource_id: "demo:opaque_cube".to_string(),
            state_count: 2,
            light_emission: 0,
            destroy_speed: 2.0,
            full_cube_collision: true,
            dynamic_light_rgb: None,
        });
        builder.register_block(ModBlockRegistration {
            resource_id: "demo:crystal_torch".to_string(),
            state_count: 1,
            light_emission: 12,
            destroy_speed: 0.2,
            full_cube_collision: false,
            dynamic_light_rgb: None,
        });
        builder.register_block(ModBlockRegistration {
            resource_id: "demo:glass_panel".to_string(),
            state_count: 1,
            light_emission: 0,
            destroy_speed: 0.3,
            full_cube_collision: false,
            dynamic_light_rgb: None,
        });
        builder.freeze().unwrap()
    }

    #[test]
    fn modded_blocks_resolve_light_facts_and_propagate_without_error() {
        let registry = test_registry();
        let air = vanilla_block_default_state_id(VANILLA_BLOCK_AIR_ID).unwrap();
        let glow_cube = registry
            .block_by_resource_id("demo:glow_cube")
            .unwrap()
            .first_state_id;
        let opaque_cube = registry
            .block_by_resource_id("demo:opaque_cube")
            .unwrap()
            .first_state_id
            + 1;
        let crystal_torch = registry
            .block_by_resource_id("demo:crystal_torch")
            .unwrap()
            .first_state_id;
        let glass_panel = registry
            .block_by_resource_id("demo:glass_panel")
            .unwrap()
            .first_state_id;

        assert!(light_facts(glow_cube).is_none());
        let glow_facts = light_facts_with_registry(glow_cube, &registry).unwrap();
        assert_eq!(glow_facts.emission, 15);
        assert_eq!(glow_facts.dampening, 15);
        let opaque_facts = light_facts_with_registry(opaque_cube, &registry).unwrap();
        assert_eq!(opaque_facts.emission, 0);
        assert_eq!(opaque_facts.dampening, 15);
        let crystal_facts = light_facts_with_registry(crystal_torch, &registry).unwrap();
        assert_eq!(crystal_facts.emission, 12);
        assert_eq!(crystal_facts.dampening, 0);
        assert!(light_facts_with_registry(registry.total_block_state_count(), &registry).is_none());

        assert!(has_different_light_properties_with_registry(
            air, glow_cube, &registry
        ));
        assert!(has_different_light_properties_with_registry(
            air,
            opaque_cube,
            &registry
        ));
        assert!(has_different_light_properties_with_registry(
            air,
            crystal_torch,
            &registry
        ));
        assert!(!has_different_light_properties_with_registry(
            air,
            glass_panel,
            &registry
        ));
        assert!(!has_different_light_properties_with_registry(
            glow_cube, glow_cube, &registry
        ));

        let mut storage = ChunkLightStorage::default();
        storage.register_non_empty_section((0, 0, 0));
        storage.enable_column((0, 0));

        let source_pos = (4, 8, 4);
        let blocked_pos = (5, 8, 4);
        let behind_opaque = (6, 8, 4);
        let panel_pos = (4, 8, 5);
        let past_panel = (4, 8, 6);
        let crystal_pos = (12, 8, 12);
        let crystal_neighbor = (13, 8, 12);

        let block_at = |pos: Pos| {
            if pos == source_pos {
                Some(glow_cube)
            } else if pos == blocked_pos {
                Some(opaque_cube)
            } else if pos == panel_pos {
                Some(glass_panel)
            } else if pos == crystal_pos {
                Some(crystal_torch)
            } else {
                Some(air)
            }
        };

        let mut engine =
            LightEngine::new_with_registry(&mut storage, block_at, |c| c == (0, 0), air, &registry)
                .unwrap();
        engine.check_block(source_pos).unwrap();
        engine.check_block(crystal_pos).unwrap();
        engine.run().unwrap();

        assert_eq!(storage.block_level(source_pos), 15);
        assert_eq!(storage.block_level((3, 8, 4)), 14);
        assert_eq!(storage.block_level(blocked_pos), 0);
        // Light cannot pass straight through `opaque_cube` (which would yield 13);
        // it must detour 4 steps around `(5, 8, 4)` to reach 11.
        assert_eq!(storage.block_level(behind_opaque), 11);
        assert_eq!(storage.block_level(panel_pos), 14);
        assert_eq!(storage.block_level(past_panel), 13);
        assert_eq!(storage.block_level(crystal_pos), 12);
        assert_eq!(storage.block_level(crystal_neighbor), 11);

        // Skylight stops on full-cube modded block and passes through non-cube modded block.
        let lowest_opaque =
            lowest_sky_source_y_with_registry(0, &[false], air, &registry, |_, y| {
                if y == 5 {
                    opaque_cube
                } else {
                    air
                }
            })
            .unwrap();
        assert_eq!(lowest_opaque, 6);

        let lowest_panel =
            lowest_sky_source_y_with_registry(0, &[false], air, &registry, |_, y| {
                if y == 5 {
                    glass_panel
                } else {
                    air
                }
            })
            .unwrap();
        assert_eq!(lowest_panel, i32::MIN);
    }
}
