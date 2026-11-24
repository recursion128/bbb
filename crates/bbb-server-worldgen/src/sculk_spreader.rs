//! Vanilla `SculkSpreader` (`SculkSpreader.java:36-360`) in both of its
//! configurations — the world-generation one (`createWorldGenSpreader`,
//! `:71-73`) and the level-time one a sculk catalyst drives
//! (`createLevelSpreader`, `:67-69`) — together with the three
//! `SculkBehaviour` owners its charge cursors can stand on: the interface
//! default (`SculkBehaviour.java:13-43`), `SculkBlock` (`SculkBlock.java:28-106`)
//! and `SculkVeinBlock` (`SculkVeinBlock.java:46-151`).
//!
//! `SculkPatchFeature` drives the world-generation configuration and the
//! catalyst's `serverTick` (`SculkCatalystBlockEntity.java:37-39`) drives the
//! level-time one; the branch is `isWorldGeneration()`, exactly as vanilla
//! branches it. The level-time-only `LevelAccessor` calls a `WorldGenRegion`
//! drops — `levelEvent` (`:150`, `:178`) and `playSound` (`:269`) — are
//! collected into [`SculkSpreadEffects`] so the world tick can flush them into
//! its own sink; the world-generation entry point discards them, which is what
//! `WorldGenRegion.levelEvent`/`playSound` do (`WorldGenRegion.java:406-407`).
//!
//! `ChargeCursor.shouldUpdate`'s level-time gate (`:244-252`) is
//! `ServerLevel.shouldTickBlocksAt(originPos)`, and the caller that reached
//! this spreader is the block-entity ticker, which only runs for a
//! block-ticking chunk of that same position; the gate is therefore the
//! ticker's own and not repeated here.

use bbb_protocol::packets::{BlockPos, Direction};
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::random_source::RandomSource;

use crate::fast_hash::FastHashMap;
use bbb_server_types::fluid::block_state_fluid_state;
use bbb_server_types::sculk_cursor::{cursor_nbt, parse_cursors_nbt, SculkCursorFacts};

use super::multiface::{
    all_shuffled, available_faces, can_attach_to_at, face_property, has_any_face, has_face,
    is_face_sturdy_at, is_source_water, spread_all, SpreadConfig, SpreadType, DIRECTIONS,
};
use super::{
    between_closed, block_is, block_set_bool, catalog_default_state, default_block_state,
    dist_manhattan, dist_to_low_corner_sqr, is_empty_block, mth_floor_f32, offset_block_pos,
    util_shuffle, WorldGenLevel, FEATURE_SET_BLOCK,
};

/// `SculkSpreader.MAX_CHARGE` (`SculkSpreader.java:38`).
const MAX_CHARGE: i32 = 1000;
/// `SculkSpreader.MAX_CURSORS` (`SculkSpreader.java:40`).
const MAX_CURSORS: usize = 32;
/// `SculkSpreader.SHRIEKER_PLACEMENT_RATE` (`SculkSpreader.java:41`).
const SHRIEKER_PLACEMENT_RATE: i32 = 11;
/// `SculkSpreader.MAX_GROWTH_RATE_RADIUS` (`SculkSpreader.java:37`).
const MAX_GROWTH_RATE_RADIUS: i32 = 24;
/// `SculkSpreader.MAX_CURSOR_DISTANCE` (`SculkSpreader.java:42`).
const MAX_CURSOR_DISTANCE: i32 = 1024;
/// `LevelEvent.PARTICLES_SCULK_CHARGE` (`LevelEvent.java:71`), the event the
/// discharge and the per-position charge bursts raise (`SculkSpreader.java:150`,
/// `:178`).
const PARTICLES_SCULK_CHARGE: i32 = 3006;
/// `SoundEvents.SCULK_BLOCK_SPREAD` (`SoundEvents.java:1381`), the sound a
/// successful vein spread plays (`SculkSpreader.java:269`) and
/// `SculkVeinBlock.attemptPlaceSculk` plays again over the new sculk
/// (`SculkVeinBlock.java:117`).
pub const SCULK_BLOCK_SPREAD_SOUND: &str = "minecraft:block.sculk.spread";

/// The level-time `LevelAccessor` calls a world-generation region drops:
/// `WorldGenRegion.levelEvent` and `WorldGenRegion.playSound` are no-ops
/// (`WorldGenRegion.java:406-407`), so the world-generation entry point hands
/// in a scratch value and the world tick hands in one it flushes into the
/// level's own sink.
#[derive(Debug, Default)]
pub struct SculkSpreadEffects {
    /// `LevelAccessor.levelEvent(type, pos, data)` calls in order.
    pub level_events: Vec<(i32, BlockPos, i32)>,
    /// `LevelAccessor.playSound(null, pos, sound, SoundSource.BLOCKS, 1.0F,
    /// 1.0F)` calls in order.
    pub sounds: Vec<(BlockPos, &'static str)>,
}

impl SculkSpreadEffects {
    fn level_event(&mut self, event_type: i32, pos: BlockPos, data: i32) {
        self.level_events.push((event_type, pos, data));
    }

    fn play_sound(&mut self, pos: BlockPos, sound: &'static str) {
        self.sounds.push((pos, sound));
    }
}

/// `SculkVeinBlock.veinSpreader` (`SculkVeinBlock.java:23`).
const VEIN_SPREADER: SpreadConfig = SpreadConfig::SculkVein {
    spread_types: &super::multiface::DEFAULT_SPREAD_ORDER,
};
/// `SculkVeinBlock.sameSpaceSpreader` (`SculkVeinBlock.java:24-26`).
const SAME_SPACE_SPREADER: SpreadConfig = SpreadConfig::SculkVein {
    spread_types: &[SpreadType::SamePosition],
};

/// Vanilla `SculkSpreader` (`SculkSpreader.java:36-49`).
pub struct SculkSpreader {
    is_world_generation: bool,
    replaceable_blocks: &'static str,
    growth_spawn_cost: i32,
    no_growth_radius: i32,
    charge_decay_rate: i32,
    additional_decay_rate: i32,
    cursors: Vec<ChargeCursor>,
}

impl SculkSpreader {
    /// `SculkSpreader.createLevelSpreader` (`SculkSpreader.java:67-69`), the
    /// configuration `SculkCatalystBlockEntity.CatalystListener` builds
    /// (`SculkCatalystBlockEntity.java:66`).
    pub fn create_level_spreader() -> Self {
        Self::new(false, "minecraft:sculk_replaceable", 10, 4, 10, 5)
    }

    /// `SculkSpreader.createWorldGenSpreader` (`SculkSpreader.java:71-73`).
    pub fn create_world_gen_spreader() -> Self {
        Self::new(true, "minecraft:sculk_replaceable_world_gen", 50, 1, 5, 10)
    }

    /// `SculkSpreader`'s constructor (`SculkSpreader.java:51-65`).
    fn new(
        is_world_generation: bool,
        replaceable_blocks: &'static str,
        growth_spawn_cost: i32,
        no_growth_radius: i32,
        charge_decay_rate: i32,
        additional_decay_rate: i32,
    ) -> Self {
        Self {
            is_world_generation,
            replaceable_blocks,
            growth_spawn_cost,
            no_growth_radius,
            charge_decay_rate,
            additional_decay_rate,
            cursors: Vec::new(),
        }
    }

    /// `SculkSpreader.clear` (`SculkSpreader.java:104-106`).
    pub fn clear(&mut self) {
        self.cursors.clear();
    }

    /// `SculkSpreader.load` (`SculkSpreader.java:108-111`): the list is
    /// replaced whole, and a document the codec rejects — a list longer than
    /// [`MAX_CURSORS`] or one unreadable entry — reads as no cursors.
    pub fn load_cursors(&mut self, values: &[NbtSummaryValue]) {
        self.cursors.clear();
        let Some(facts) = parse_cursors_nbt(values) else {
            return;
        };
        for facts in facts {
            self.add_cursor(ChargeCursor::from_facts(facts));
        }
    }

    /// `SculkSpreader.save` (`SculkSpreader.java:113-114`); the
    /// `DEBUG_SCULK_CATALYST` statistics are not part of the document.
    #[must_use]
    pub fn save_cursors(&self) -> Vec<NbtSummaryValue> {
        self.cursors
            .iter()
            .map(|cursor| cursor_nbt(&cursor.to_facts()))
            .collect()
    }

    /// `SculkSpreader.addCursors` (`SculkSpreader.java:126-132`) over
    /// `addCursor` (`:134-138`).
    pub fn add_cursors(&mut self, start_pos: BlockPos, mut charge: i32) {
        while charge > 0 {
            let current_charge = charge.min(MAX_CHARGE);
            self.add_cursor(ChargeCursor::new(start_pos, current_charge));
            charge -= current_charge;
        }
    }

    /// `SculkSpreader.addCursor` (`SculkSpreader.java:134-138`).
    fn add_cursor(&mut self, cursor: ChargeCursor) {
        if self.cursors.len() < MAX_CURSORS {
            self.cursors.push(cursor);
        }
    }

    /// `SculkSpreader.updateCursors` (`SculkSpreader.java:140-184`) with the
    /// level-time effects dropped, which is the world-generation reading of
    /// `WorldGenRegion`'s no-op `levelEvent`/`playSound`.
    pub fn update_cursors<L: WorldGenLevel>(
        &mut self,
        level: &mut L,
        origin_pos: BlockPos,
        random: &mut dyn RandomSource,
        spread_veins: bool,
    ) -> Option<()> {
        let mut effects = SculkSpreadEffects::default();
        self.update_cursors_with_effects(level, &mut effects, origin_pos, random, spread_veins)
    }

    /// `SculkSpreader.updateCursors` (`SculkSpreader.java:140-184`) keeping the
    /// level-time `levelEvent` and `playSound` calls in `effects`.
    ///
    /// The same-position merge (`:158-165`) is the level-time branch vanilla
    /// gates on `!isWorldGeneration()`; the world-generation branch keeps every
    /// surviving cursor in order, which is what this loop does when the gate
    /// is false.
    pub fn update_cursors_with_effects<L: WorldGenLevel>(
        &mut self,
        level: &mut L,
        effects: &mut SculkSpreadEffects,
        origin_pos: BlockPos,
        random: &mut dyn RandomSource,
        spread_veins: bool,
    ) -> Option<()> {
        if self.cursors.is_empty() {
            return Some(());
        }
        let mut processed_cursors = Vec::new();
        let mut mergeable_cursors: FastHashMap<(i32, i32, i32), usize> = FastHashMap::default();
        // `chargeMap` is an `Object2IntOpenHashMap`, whose iteration order is
        // its hash order; this keeps insertion order, which only orders the
        // particle events the loop below raises.
        let mut charge_map: Vec<(BlockPos, i32)> = Vec::new();
        let cursors = std::mem::take(&mut self.cursors);
        for mut cursor in cursors {
            if cursor.is_pos_unreasonable(origin_pos) {
                continue;
            }
            cursor.update(level, effects, origin_pos, random, self, spread_veins)?;
            if cursor.charge <= 0 {
                effects.level_event(PARTICLES_SCULK_CHARGE, cursor.pos, 0);
                continue;
            }
            let pos = cursor.pos;
            match charge_map
                .iter_mut()
                .find(|(charged_pos, _)| *charged_pos == pos)
            {
                Some((_, charge)) => *charge += cursor.charge,
                None => charge_map.push((pos, cursor.charge)),
            }
            match mergeable_cursors.get(&(pos.x, pos.y, pos.z)).copied() {
                None => {
                    mergeable_cursors.insert((pos.x, pos.y, pos.z), processed_cursors.len());
                    processed_cursors.push(cursor);
                }
                Some(existing)
                    if !self.is_world_generation && {
                        let merged = cursor.charge + processed_cursors[existing].charge;
                        merged <= MAX_CHARGE
                    } =>
                {
                    processed_cursors[existing].merge_with(cursor);
                }
                Some(existing) => {
                    let cursor_charge = cursor.charge;
                    processed_cursors.push(cursor);
                    if cursor_charge < processed_cursors[existing].charge {
                        mergeable_cursors
                            .insert((pos.x, pos.y, pos.z), processed_cursors.len() - 1);
                    }
                }
            }
        }
        for (pos, charge) in charge_map {
            let Some(faces) = mergeable_cursors
                .get(&(pos.x, pos.y, pos.z))
                .and_then(|index| processed_cursors[*index].facings.as_deref())
            else {
                continue;
            };
            if charge > 0 {
                // `(int)(Math.log1p(charge) / 2.3F) + 1` (`:176`) over
                // `MultifaceBlock.pack` (`:177`).
                let num_particles = (f64::ln_1p(f64::from(charge)) / f64::from(2.3_f32)) as i32 + 1;
                effects.level_event(
                    PARTICLES_SCULK_CHARGE,
                    pos,
                    (num_particles << 6) + pack_faces(faces),
                );
            }
        }
        self.cursors = processed_cursors;
        Some(())
    }
}

/// `MultifaceBlock.pack` (`MultifaceBlock.java:98-106`), the six-bit face code
/// `SculkSpreader` folds into the charge particle's `data` (`:177`). The
/// `byte` vanilla returns never sign-extends here, because only the six
/// direction bits are ever set.
fn pack_faces(faces: &[Direction]) -> i32 {
    faces
        .iter()
        .fold(0, |code, direction| code | (1 << direction.id()))
}

/// `SculkSpreader.ChargeCursor.NON_CORNER_NEIGHBOURS` (`SculkSpreader.java:187-193`):
/// the `betweenClosedStream` order filtered to offsets with a zero coordinate.
fn non_corner_neighbours() -> Vec<(i32, i32, i32)> {
    between_closed(
        BlockPos {
            x: -1,
            y: -1,
            z: -1,
        },
        BlockPos { x: 1, y: 1, z: 1 },
    )
    .filter(|position| {
        (position.x == 0 || position.y == 0 || position.z == 0)
            && !(position.x == 0 && position.y == 0 && position.z == 0)
    })
    .map(|position| (position.x, position.y, position.z))
    .collect()
}

/// Vanilla `SculkSpreader.ChargeCursor` (`SculkSpreader.java:186-260`).
struct ChargeCursor {
    pos: BlockPos,
    charge: i32,
    update_delay: i32,
    decay_delay: i32,
    /// `EnumSet<Direction>` iterates in ordinal order, which is the order
    /// `MultifaceBlock.availableFaces` fills it in.
    facings: Option<Vec<Direction>>,
}

impl ChargeCursor {
    /// `ChargeCursor(pos, charge)` (`SculkSpreader.java:220-222`).
    fn new(pos: BlockPos, charge: i32) -> Self {
        Self {
            pos,
            charge,
            update_delay: 0,
            decay_delay: 1,
            facings: None,
        }
    }

    /// The codec's own constructor (`SculkSpreader.java:212-218`).
    fn from_facts(facts: SculkCursorFacts) -> Self {
        Self {
            pos: facts.pos,
            charge: facts.charge,
            update_delay: facts.update_delay,
            decay_delay: facts.decay_delay,
            facings: facts.facings,
        }
    }

    fn to_facts(&self) -> SculkCursorFacts {
        SculkCursorFacts {
            pos: self.pos,
            charge: self.charge,
            decay_delay: self.decay_delay,
            update_delay: self.update_delay,
            facings: self.facings.clone(),
        }
    }

    /// `ChargeCursor.isPosUnreasonable` (`SculkSpreader.java:228-230`) with
    /// `Vec3i.distChessboard` (`Vec3i.java:230-235`).
    fn is_pos_unreasonable(&self, origin_pos: BlockPos) -> bool {
        let xd = (self.pos.x - origin_pos.x).abs();
        let yd = (self.pos.y - origin_pos.y).abs();
        let zd = (self.pos.z - origin_pos.z).abs();
        xd.max(yd).max(zd) > MAX_CURSOR_DISTANCE
    }

    /// `ChargeCursor.update` (`SculkSpreader.java:254-297`); `shouldUpdate`
    /// (`:244-252`) is `charge > 0` for the world-generation branch and the
    /// ticker's own block-ticking gate for the level-time one.
    fn update<L: WorldGenLevel>(
        &mut self,
        level: &mut L,
        effects: &mut SculkSpreadEffects,
        origin_pos: BlockPos,
        random: &mut dyn RandomSource,
        spreader: &SculkSpreader,
        spread_veins: bool,
    ) -> Option<()> {
        if self.charge <= 0 {
            return Some(());
        }
        if self.update_delay > 0 {
            self.update_delay -= 1;
            return Some(());
        }
        let mut current_state = level.get_block(self.pos);
        let mut sculk_behaviour = SculkBehaviour::of(current_state);
        if spread_veins
            && sculk_behaviour.attempt_spread_vein(
                level,
                self.pos,
                current_state,
                self.facings.as_deref(),
                spreader.is_world_generation,
            )
        {
            if sculk_behaviour.can_change_block_state_on_spread() {
                current_state = level.get_block(self.pos);
                sculk_behaviour = SculkBehaviour::of(current_state);
            }
            effects.play_sound(self.pos, SCULK_BLOCK_SPREAD_SOUND);
        }
        self.charge = sculk_behaviour.attempt_use_charge(
            self.pos,
            self.charge,
            self.decay_delay,
            level,
            effects,
            origin_pos,
            random,
            spreader,
            spread_veins,
        )?;
        if self.charge <= 0 {
            sculk_behaviour.on_discharged(level, current_state, self.pos);
            return Some(());
        }
        if let Some(transfer_pos) = get_valid_movement_pos(level, self.pos, random) {
            sculk_behaviour.on_discharged(level, current_state, self.pos);
            self.pos = transfer_pos;
            if spreader.is_world_generation && !closer_than_xz(self.pos, origin_pos, 15.0) {
                self.charge = 0;
                return Some(());
            }
            current_state = level.get_block(transfer_pos);
        }
        if SculkBehaviour::is_sculk_behaviour(current_state) {
            self.facings = Some(available_faces(current_state));
        }
        self.decay_delay = sculk_behaviour.update_decay_delay(self.decay_delay);
        self.update_delay = sculk_behaviour.get_sculk_spread_delay();
        Some(())
    }

    /// `ChargeCursor.mergeWith` (`SculkSpreader.java:299-303`). The zeroed
    /// charge vanilla writes into `other` is unobservable: the merged cursor is
    /// dropped by the caller.
    fn merge_with(&mut self, other: Self) {
        self.charge += other.charge;
        self.update_delay = self.update_delay.min(other.update_delay);
    }
}

/// `this.pos.closerThan(new Vec3i(originPos.getX(), this.pos.getY(), originPos.getZ()), 15.0)`
/// (`SculkSpreader.java:280`) through `Vec3i.closerThan` (`Vec3i.java:193-195`).
fn closer_than_xz(pos: BlockPos, origin_pos: BlockPos, distance: f64) -> bool {
    let dx = f64::from(pos.x) - f64::from(origin_pos.x);
    let dz = f64::from(pos.z) - f64::from(origin_pos.z);
    dx * dx + dz * dz < distance * distance
}

/// `ChargeCursor.getValidMovementPos` (`SculkSpreader.java:313-329`) over
/// `getRandomizedNonCornerNeighbourOffsets` (`:309-311`).
fn get_valid_movement_pos<L: WorldGenLevel>(
    level: &L,
    pos: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<BlockPos> {
    let mut offsets = non_corner_neighbours();
    util_shuffle(&mut offsets, random);
    let mut sculk_position = pos;
    for (dx, dy, dz) in offsets {
        let neighbour = BlockPos {
            x: pos.x + dx,
            y: pos.y + dy,
            z: pos.z + dz,
        };
        let transferee = level.get_block(neighbour);
        if SculkBehaviour::is_sculk_behaviour(transferee)
            && is_movement_unobstructed(level, pos, neighbour)
        {
            sculk_position = neighbour;
            if has_substrate_access(level, transferee, neighbour) {
                break;
            }
        }
    }
    (sculk_position != pos).then_some(sculk_position)
}

/// `ChargeCursor.isMovementUnobstructed` (`SculkSpreader.java:331-353`) with
/// `Direction.fromAxisAndDirection` (`Direction.java:295-301`).
fn is_movement_unobstructed<L: WorldGenLevel>(level: &L, from: BlockPos, to: BlockPos) -> bool {
    if dist_manhattan(from, to) == 1 {
        return true;
    }
    let (dx, dy, dz) = (to.x - from.x, to.y - from.y, to.z - from.z);
    let direction_x = if dx < 0 {
        Direction::West
    } else {
        Direction::East
    };
    let direction_y = if dy < 0 {
        Direction::Down
    } else {
        Direction::Up
    };
    let direction_z = if dz < 0 {
        Direction::North
    } else {
        Direction::South
    };
    if dx == 0 {
        is_unobstructed(level, from, direction_y) || is_unobstructed(level, from, direction_z)
    } else if dy == 0 {
        is_unobstructed(level, from, direction_x) || is_unobstructed(level, from, direction_z)
    } else {
        is_unobstructed(level, from, direction_x) || is_unobstructed(level, from, direction_y)
    }
}

/// `ChargeCursor.isUnobstructed` (`SculkSpreader.java:355-358`).
fn is_unobstructed<L: WorldGenLevel>(level: &L, from: BlockPos, direction: Direction) -> bool {
    let test_pos = offset_block_pos(from, direction);
    !is_face_sturdy_at(
        level,
        test_pos,
        level.get_block(test_pos),
        super::direction_opposite(direction),
    )
}

/// `SculkVeinBlock.hasSubstrateAccess` (`SculkVeinBlock.java:139-151`).
fn has_substrate_access<L: WorldGenLevel>(level: &L, state: i32, pos: BlockPos) -> bool {
    if !block_is(state, "minecraft:sculk_vein") {
        return false;
    }
    for direction in DIRECTIONS {
        if !has_face(state, direction) {
            continue;
        }
        let neighbour = level.get_block(offset_block_pos(pos, direction));
        if level.block_state_in_tag(neighbour, "minecraft:sculk_replaceable") {
            return true;
        }
    }
    false
}

/// The `SculkBehaviour` a block state resolves to
/// (`SculkSpreader.java:305-307`): the two implementing blocks
/// (`SculkBlock.java:15`, `SculkVeinBlock.java:21`) or `SculkBehaviour.DEFAULT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SculkBehaviour {
    Default,
    Sculk,
    SculkVein,
}

impl SculkBehaviour {
    fn of(state: i32) -> Self {
        if block_is(state, "minecraft:sculk") {
            Self::Sculk
        } else if block_is(state, "minecraft:sculk_vein") {
            Self::SculkVein
        } else {
            Self::Default
        }
    }

    /// `state.getBlock() instanceof SculkBehaviour`.
    fn is_sculk_behaviour(state: i32) -> bool {
        Self::of(state) != Self::Default
    }

    /// `SculkBehaviour.getSculkSpreadDelay` (`SculkBehaviour.java:45-47`).
    fn get_sculk_spread_delay(self) -> i32 {
        1
    }

    /// `SculkBehaviour.canChangeBlockStateOnSpread` (`SculkBehaviour.java:62-64`);
    /// `SculkBlock` overrides it to false (`SculkBlock.java:104-106`).
    fn can_change_block_state_on_spread(self) -> bool {
        !matches!(self, Self::Sculk)
    }

    /// `SculkBehaviour.updateDecayDelay` (`SculkBehaviour.java:66-68`); the
    /// `DEFAULT` instance ages instead (`SculkBehaviour.java:40-42`).
    fn update_decay_delay(self, age: i32) -> i32 {
        match self {
            Self::Default => (age - 1).max(0),
            Self::Sculk | Self::SculkVein => 1,
        }
    }

    /// `SculkBehaviour.attemptSpreadVein` (`SculkBehaviour.java:56-60`) with
    /// the `DEFAULT` override (`SculkBehaviour.java:15-25`).
    fn attempt_spread_vein<L: WorldGenLevel>(
        self,
        level: &mut L,
        pos: BlockPos,
        state: i32,
        facings: Option<&[Direction]>,
        post_process: bool,
    ) -> bool {
        match self {
            Self::Default => match facings {
                None => {
                    let current = level.get_block(pos);
                    spread_all(SAME_SPACE_SPREADER, current, level, pos, post_process) > 0
                }
                Some(faces) if !faces.is_empty() => {
                    if !is_empty_block(state) && !is_source_water(state) {
                        false
                    } else {
                        regrow(level, pos, state, faces)
                    }
                }
                Some(_) => spread_all(VEIN_SPREADER, state, level, pos, post_process) > 0,
            },
            Self::Sculk | Self::SculkVein => {
                spread_all(VEIN_SPREADER, state, level, pos, post_process) > 0
            }
        }
    }

    /// `SculkBehaviour.attemptUseCharge` as implemented by the `DEFAULT`
    /// instance (`SculkBehaviour.java:28-37`), `SculkBlock`
    /// (`SculkBlock.java:28-58`) and `SculkVeinBlock` (`SculkVeinBlock.java:90-103`).
    #[allow(clippy::too_many_arguments)]
    fn attempt_use_charge<L: WorldGenLevel>(
        self,
        cursor_pos: BlockPos,
        cursor_charge: i32,
        cursor_decay_delay: i32,
        level: &mut L,
        effects: &mut SculkSpreadEffects,
        origin_pos: BlockPos,
        random: &mut dyn RandomSource,
        spreader: &SculkSpreader,
        spread_veins: bool,
    ) -> Option<i32> {
        match self {
            Self::Default => Some(if cursor_decay_delay > 0 {
                cursor_charge
            } else {
                0
            }),
            Self::Sculk => Some(sculk_attempt_use_charge(
                cursor_pos,
                cursor_charge,
                level,
                effects,
                origin_pos,
                random,
                spreader,
            )),
            Self::SculkVein => {
                if spread_veins && attempt_place_sculk(spreader, level, effects, cursor_pos, random)
                {
                    Some(cursor_charge - 1)
                } else if random.next_int(spreader.charge_decay_rate) == 0 {
                    Some(mth_floor_f32(cursor_charge as f32 * 0.5))
                } else {
                    Some(cursor_charge)
                }
            }
        }
    }

    /// `SculkBehaviour.onDischarged` (`SculkBehaviour.java:49-50`), overridden
    /// only by `SculkVeinBlock` (`SculkVeinBlock.java:70-87`).
    fn on_discharged<L: WorldGenLevel>(self, level: &mut L, state: i32, pos: BlockPos) {
        match self {
            Self::Default | Self::Sculk => {}
            Self::SculkVein => sculk_vein_on_discharged(level, state, pos),
        }
    }
}

/// `SculkVeinBlock.regrow` (`SculkVeinBlock.java:46-67`).
fn regrow<L: WorldGenLevel>(
    level: &mut L,
    pos: BlockPos,
    existing: i32,
    faces: &[Direction],
) -> bool {
    let mut has_at_least_one_face = false;
    let mut new_state = catalog_default_state("minecraft:sculk_vein");
    for face in faces {
        let neighbour_pos = offset_block_pos(pos, *face);
        let neighbour = level.get_block(neighbour_pos);
        if can_attach_to_at(level, neighbour_pos, neighbour, *face) {
            new_state = block_set_bool(new_state, face_property(*face), true);
            has_at_least_one_face = true;
        }
    }
    if !has_at_least_one_face {
        return false;
    }
    if !block_state_fluid_state(existing).is_empty() {
        new_state = block_set_bool(new_state, "waterlogged", true);
    }
    let _ = level.set_block(pos, new_state, FEATURE_SET_BLOCK);
    true
}

/// `SculkVeinBlock.onDischarged` (`SculkVeinBlock.java:70-87`): the
/// `SculkBehaviour.super.onDischarged` tail is the empty default.
fn sculk_vein_on_discharged<L: WorldGenLevel>(level: &mut L, mut state: i32, pos: BlockPos) {
    if !block_is(state, "minecraft:sculk_vein") {
        return;
    }
    for dir in DIRECTIONS {
        if has_face(state, dir)
            && block_is(
                level.get_block(offset_block_pos(pos, dir)),
                "minecraft:sculk",
            )
        {
            state = block_set_bool(state, face_property(dir), false);
        }
    }
    if !has_any_face(state) {
        let fluid_state = block_state_fluid_state(level.get_block(pos));
        state = if fluid_state.is_empty() {
            default_block_state("minecraft:air")
        } else {
            catalog_default_state("minecraft:water")
        };
    }
    let _ = level.set_block(pos, state, FEATURE_SET_BLOCK);
}

/// `SculkVeinBlock.attemptPlaceSculk` (`SculkVeinBlock.java:105-137`), whose
/// `Block.pushEntitiesUp` (`:116`) runs between the write and the sound, as
/// vanilla orders them.
fn attempt_place_sculk<L: WorldGenLevel>(
    spreader: &SculkSpreader,
    level: &mut L,
    effects: &mut SculkSpreadEffects,
    pos: BlockPos,
    random: &mut dyn RandomSource,
) -> bool {
    let state = level.get_block(pos);
    let replace_tag = spreader.replaceable_blocks;
    for support in all_shuffled(random) {
        if has_face(state, support) {
            let support_pos = offset_block_pos(pos, support);
            let support_state = level.get_block(support_pos);
            if level.block_state_in_tag(support_state, replace_tag) {
                let default_sculk = default_block_state("minecraft:sculk");
                let _ = level.set_block(support_pos, default_sculk, FEATURE_SET_BLOCK);
                level.push_entities_up(support_state, default_sculk, support_pos);
                effects.play_sound(support_pos, SCULK_BLOCK_SPREAD_SOUND);
                spread_all(
                    VEIN_SPREADER,
                    default_sculk,
                    level,
                    support_pos,
                    spreader.is_world_generation,
                );
                let skip = super::direction_opposite(support);
                for vein_blocks in DIRECTIONS {
                    if vein_blocks != skip {
                        let vein_pos = offset_block_pos(support_pos, vein_blocks);
                        let possible_vein_block = level.get_block(vein_pos);
                        if block_is(possible_vein_block, "minecraft:sculk_vein") {
                            sculk_vein_on_discharged(level, possible_vein_block, vein_pos);
                        }
                    }
                }
                return true;
            }
        }
    }
    false
}

/// `SculkBlock.attemptUseCharge` (`SculkBlock.java:28-58`).
///
/// The growth placement's own sound (`:46`) is the placed state's group place
/// event, at the *cursor's* cell rather than the growth's, on
/// `SoundSource.BLOCKS` at volume and pitch one.
fn sculk_attempt_use_charge<L: WorldGenLevel>(
    charge_pos: BlockPos,
    charge: i32,
    level: &mut L,
    effects: &mut SculkSpreadEffects,
    origin_pos: BlockPos,
    random: &mut dyn RandomSource,
    spreader: &SculkSpreader,
) -> i32 {
    if charge != 0 && random.next_int(spreader.charge_decay_rate) == 0 {
        let no_growth_radius = f64::from(spreader.no_growth_radius);
        let is_close_to_catalyst =
            dist_to_low_corner_sqr(charge_pos, origin_pos) < no_growth_radius * no_growth_radius;
        if !is_close_to_catalyst && can_place_growth(level, charge_pos) {
            let xp_per_growth_spawn = spreader.growth_spawn_cost;
            if random.next_int(xp_per_growth_spawn) < charge {
                let growth_placement = BlockPos {
                    x: charge_pos.x,
                    y: charge_pos.y + 1,
                    z: charge_pos.z,
                };
                let growth_state = get_random_growth_state(
                    level,
                    growth_placement,
                    random,
                    spreader.is_world_generation,
                );
                let _ = level.set_block(growth_placement, growth_state, FEATURE_SET_BLOCK);
                // `SculkBlock.java:46`, after the write and ignoring whether
                // it landed.
                let sound_type =
                    bbb_protocol::block_sound_facts_26_1::vanilla_block_state_sound_type_26_1(
                        growth_state,
                    );
                effects.play_sound(charge_pos, sound_type.place_sound);
            }
            (charge - xp_per_growth_spawn).max(0)
        } else if random.next_int(spreader.additional_decay_rate) != 0 {
            charge
        } else {
            charge
                - if is_close_to_catalyst {
                    1
                } else {
                    get_decay_penalty(spreader, charge_pos, origin_pos, charge)
                }
        }
    } else {
        charge
    }
}

/// `SculkBlock.getDecayPenalty` (`SculkBlock.java:60-66`) with
/// `Mth.square(float)` (`Mth.java:616-618`) and `Mth.square(int)` (`:628-630`).
fn get_decay_penalty(
    spreader: &SculkSpreader,
    pos: BlockPos,
    origin_pos: BlockPos,
    charge: i32,
) -> i32 {
    let no_growth_radius = spreader.no_growth_radius;
    let outer_distance =
        dist_to_low_corner_sqr(pos, origin_pos).sqrt() as f32 - no_growth_radius as f32;
    let outer_distance_squared = outer_distance * outer_distance;
    let max_reach = MAX_GROWTH_RATE_RADIUS - no_growth_radius;
    let max_reach_squared = max_reach * max_reach;
    let distance_factor = f32::min(1.0, outer_distance_squared / max_reach_squared as f32);
    ((charge as f32 * distance_factor * 0.5) as i32).max(1)
}

/// `SculkBlock.getRandomGrowthState` (`SculkBlock.java:68-79`).
fn get_random_growth_state<L: WorldGenLevel>(
    level: &L,
    pos: BlockPos,
    random: &mut dyn RandomSource,
    is_world_gen: bool,
) -> i32 {
    let state = if random.next_int(SHRIEKER_PLACEMENT_RATE) == 0 {
        block_set_bool(
            catalog_default_state("minecraft:sculk_shrieker"),
            "can_summon",
            is_world_gen,
        )
    } else {
        catalog_default_state("minecraft:sculk_sensor")
    };
    if !block_state_fluid_state(level.get_block(pos)).is_empty() {
        block_set_bool(state, "waterlogged", true)
    } else {
        state
    }
}

/// `SculkBlock.canPlaceGrowth` (`SculkBlock.java:81-101`).
fn can_place_growth<L: WorldGenLevel>(level: &L, pos: BlockPos) -> bool {
    let state_above = level.get_block(BlockPos {
        x: pos.x,
        y: pos.y + 1,
        z: pos.z,
    });
    if is_empty_block(state_above)
        || (block_is(state_above, "minecraft:water") && is_source_water(state_above))
    {
        let mut growth_count = 0;
        for block_pos in between_closed(
            BlockPos {
                x: pos.x - 4,
                y: pos.y,
                z: pos.z - 4,
            },
            BlockPos {
                x: pos.x + 4,
                y: pos.y + 2,
                z: pos.z + 4,
            },
        ) {
            let state = level.get_block(block_pos);
            if block_is(state, "minecraft:sculk_sensor")
                || block_is(state, "minecraft:sculk_shrieker")
            {
                growth_count += 1;
            }
            if growth_count > 2 {
                return false;
            }
        }
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::random::WorldgenRandom;

    use super::*;

    /// A world-generation level with an in-memory block map. It leaves
    /// `push_entities_up` at the trait default, which is the reading a
    /// `WorldGenRegion` gives it.
    struct PlainLevel {
        blocks: BTreeMap<(i32, i32, i32), i32>,
    }

    impl PlainLevel {
        fn with_blocks(blocks: &[(BlockPos, i32)]) -> Self {
            Self {
                blocks: blocks
                    .iter()
                    .map(|(pos, state)| ((pos.x, pos.y, pos.z), *state))
                    .collect(),
            }
        }
    }

    impl WorldGenLevel for PlainLevel {
        fn ensure_can_write(&self, _origin: BlockPos) -> bool {
            true
        }

        fn get_block(&self, pos: BlockPos) -> i32 {
            self.blocks
                .get(&(pos.x, pos.y, pos.z))
                .copied()
                .unwrap_or_else(|| default_block_state("minecraft:air"))
        }

        fn registry_tag_contains(&self, registry_id: &str, tag_id: &str, element_id: &str) -> bool {
            registry_id == "minecraft:block"
                && tag_id == "minecraft:sculk_replaceable_world_gen"
                && element_id == "minecraft:stone"
        }

        fn set_block(&mut self, pos: BlockPos, state: i32, _update_flags: i32) -> bool {
            self.blocks.insert((pos.x, pos.y, pos.z), state) != Some(state)
        }

        fn get_min_y(&self) -> i32 {
            -64
        }

        fn get_max_y(&self) -> i32 {
            319
        }

        fn get_sea_level(&self) -> i32 {
            63
        }

        fn schedule_fluid_tick(&mut self, _pos: BlockPos, _delay: i32) {}

        fn schedule_block_tick(&mut self, _pos: BlockPos, _delay: i32) {}

        fn mark_pos_for_postprocessing(&mut self, _pos: BlockPos) {}
    }

    /// The same level recording the `push_entities_up` calls it receives.
    struct RecordingLevel {
        inner: PlainLevel,
        pushes: Vec<(i32, i32, BlockPos)>,
    }

    impl WorldGenLevel for RecordingLevel {
        fn ensure_can_write(&self, origin: BlockPos) -> bool {
            self.inner.ensure_can_write(origin)
        }

        fn get_block(&self, pos: BlockPos) -> i32 {
            self.inner.get_block(pos)
        }

        fn registry_tag_contains(&self, registry_id: &str, tag_id: &str, element_id: &str) -> bool {
            self.inner
                .registry_tag_contains(registry_id, tag_id, element_id)
        }

        fn block_state_in_tag(&self, state_id: i32, tag_id: &str) -> bool {
            self.inner.block_state_in_tag(state_id, tag_id)
        }

        fn set_block(&mut self, pos: BlockPos, state: i32, update_flags: i32) -> bool {
            self.inner.set_block(pos, state, update_flags)
        }

        fn get_min_y(&self) -> i32 {
            self.inner.get_min_y()
        }

        fn get_max_y(&self) -> i32 {
            self.inner.get_max_y()
        }

        fn get_sea_level(&self) -> i32 {
            self.inner.get_sea_level()
        }

        fn schedule_fluid_tick(&mut self, pos: BlockPos, delay: i32) {
            self.inner.schedule_fluid_tick(pos, delay);
        }

        fn schedule_block_tick(&mut self, pos: BlockPos, delay: i32) {
            self.inner.schedule_block_tick(pos, delay);
        }

        fn mark_pos_for_postprocessing(&mut self, pos: BlockPos) {
            self.inner.mark_pos_for_postprocessing(pos);
        }

        fn push_entities_up(&mut self, old_state: i32, new_state: i32, pos: BlockPos) {
            self.pushes.push((old_state, new_state, pos));
        }
    }

    fn vein_with_down_face() -> i32 {
        block_set_bool(catalog_default_state("minecraft:sculk_vein"), "down", true)
    }

    fn at(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos { x, y, z }
    }

    /// `attemptPlaceSculk` (`SculkVeinBlock.java:105-137`) replaces the
    /// replaceable support a vein's face reaches, and asks the level to lift
    /// the entities standing in what the new state added (`:116`).
    #[test]
    fn the_vein_placement_asks_the_level_to_push_entities_after_the_write() {
        let pos = at(0, 0, 0);
        let support_pos = at(0, -1, 0);
        let stone = default_block_state("minecraft:stone");
        let sculk = default_block_state("minecraft:sculk");
        let mut level = RecordingLevel {
            inner: PlainLevel::with_blocks(&[(pos, vein_with_down_face()), (support_pos, stone)]),
            pushes: Vec::new(),
        };
        let spreader = SculkSpreader::create_world_gen_spreader();
        let mut effects = SculkSpreadEffects::default();
        let mut random = WorldgenRandom::with_seed(0);

        assert!(attempt_place_sculk(
            &spreader,
            &mut level,
            &mut effects,
            pos,
            &mut random
        ));
        assert_eq!(level.inner.get_block(support_pos), sculk);
        assert_eq!(level.pushes, vec![(stone, sculk, support_pos)]);
        assert_eq!(
            effects.sounds,
            vec![(support_pos, SCULK_BLOCK_SPREAD_SOUND)],
            "the sound follows the push (`SculkVeinBlock.java:117`)"
        );
    }

    /// `SculkVeinBlock.onDischarged` (`SculkVeinBlock.java:70-87`): a
    /// waterlogged vein left without faces becomes water, not air.
    #[test]
    fn a_waterlogged_vein_that_loses_every_face_becomes_water() {
        let pos = at(0, 0, 0);
        let vein = block_set_bool(vein_with_down_face(), "waterlogged", true);
        let sculk = default_block_state("minecraft:sculk");
        let mut level = PlainLevel::with_blocks(&[(pos, vein), (at(0, -1, 0), sculk)]);
        sculk_vein_on_discharged(&mut level, vein, pos);
        assert_eq!(
            level.get_block(pos),
            catalog_default_state("minecraft:water")
        );
    }
}
