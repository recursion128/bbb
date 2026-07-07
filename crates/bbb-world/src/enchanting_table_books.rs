//! Client-side enchanting-table floating-book animation state and its block
//! render source.
//!
//! Vanilla drives the enchanting table's hovering book with per-block-entity
//! fields (`EnchantingTableBlockEntity`: `time`, `flip`/`oFlip`/`flipT`/
//! `flipA`, `open`/`oOpen`, `rot`/`oRot`/`tRot`) advanced every client tick by
//! `EnchantingTableBlockEntity.bookAnimationTick` (`java:50-106`): the book
//! turns to face the nearest player within 3 blocks
//! (`level.getNearestPlayer(x+0.5, y+0.5, z+0.5, 3.0, false)`), opens/closes,
//! and flips its pages toward a random `flipT` target. bbb has no per-position
//! block-entity objects, so the same state machine lives here as a flat
//! `Vec<EnchantingTableBookState>` on the [`WorldStore`], keyed by block
//! position, ticked by [`WorldStore::advance_enchanting_table_book_ticks`].
//!
//! The render projection ([`WorldStore::enchanting_table_book_source_states`])
//! enumerates enchanting-table blocks straight from the chunk block states and
//! carries the raw animation fields; `EnchantTableRenderer.extractRenderState`
//! (the partial-tick lerp) runs on the native projection side.
//!
//! Vanilla's `RANDOM` is a static, wall-clock-seeded `RandomSource` shared
//! across every enchanting table; to keep `WorldStore` deterministic and
//! serializable, this uses a single fixed-seed [`EnchantingBookRandom`] drawn
//! in a deterministic per-position tick order.

use serde::{Deserialize, Serialize};

use crate::{BlockPos, ChunkColumn, PaletteKind, RegistrySet, WorldStore};

/// Vanilla `EnchantingTableBlockEntity.bookAnimationTick`'s nearest-player
/// search radius (`level.getNearestPlayer(..., 3.0, false)`).
const ENCHANTING_BOOK_NEAREST_PLAYER_RANGE: f64 = 3.0;

/// `LegacyRandomSource` constants (`java.util.Random`): the LCG multiplier /
/// increment and the 48-bit mask.
const RANDOM_MULTIPLIER: u64 = 0x5DEECE66D;
const RANDOM_INCREMENT: u64 = 0xB;
const RANDOM_MASK: u64 = (1_u64 << 48) - 1;

/// Fixed seed for the shared enchanting-book random. Vanilla's static
/// `EnchantingTableBlockEntity.RANDOM` is seeded from a wall-clock
/// `RandomSupport.generateUniqueSeed()`; a fixed local seed keeps the derived
/// page-flip stream deterministic and the `WorldStore` serializable while
/// preserving the `LegacyRandomSource` advancement.
const ENCHANTING_BOOK_RANDOM_SEED: i64 = 0x_B0_0C_A11_ED;

/// A deterministic `LegacyRandomSource` (`java.util.Random`) shared by every
/// enchanting-table book, replacing vanilla's static wall-clock-seeded
/// `RandomSource`. Only the `nextInt(bound)` draws `bookAnimationTick` uses are
/// implemented.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnchantingBookRandom {
    seed: u64,
}

impl EnchantingBookRandom {
    fn with_seed(seed: i64) -> Self {
        Self {
            seed: ((seed as u64) ^ RANDOM_MULTIPLIER) & RANDOM_MASK,
        }
    }

    /// Vanilla `java.util.Random.nextInt(bound)`: the power-of-two fast path and
    /// the rejection loop for other bounds.
    fn next_int(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");
        if (bound & (bound - 1)) == 0 {
            return ((i64::from(bound) * i64::from(self.next_bits(31))) >> 31) as i32;
        }
        loop {
            let sample = self.next_bits(31) as i32;
            let modulo = sample % bound;
            if sample.wrapping_sub(modulo).wrapping_add(bound - 1) >= 0 {
                return modulo;
            }
        }
    }

    fn next_bits(&mut self, bits: u32) -> u32 {
        self.seed = self
            .seed
            .wrapping_mul(RANDOM_MULTIPLIER)
            .wrapping_add(RANDOM_INCREMENT)
            & RANDOM_MASK;
        (self.seed >> (48 - bits)) as u32
    }
}

impl Default for EnchantingBookRandom {
    fn default() -> Self {
        Self::with_seed(ENCHANTING_BOOK_RANDOM_SEED)
    }
}

/// One enchanting table's `EnchantingTableBlockEntity` book-animation fields,
/// keyed by the table block position.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnchantingTableBookState {
    pub pos: BlockPos,
    /// Vanilla `time` (the monotonic tick counter feeding the float bob and the
    /// `BookModel.State.forAnimation` openness).
    pub time: i32,
    /// Vanilla `flip` / `oFlip`: the page-flip progress and its previous-tick
    /// value (the renderer lerps between them).
    pub flip: f32,
    pub o_flip: f32,
    /// Vanilla `flipT` / `flipA`: the random page-flip target and the current
    /// flip acceleration.
    pub flip_t: f32,
    pub flip_a: f32,
    /// Vanilla `open` / `oOpen`: the book openness in `[0, 1]` and its
    /// previous-tick value.
    pub open: f32,
    pub o_open: f32,
    /// Vanilla `rot` / `oRot`: the book yaw (radians) and its previous-tick
    /// value; `tRot` is the target yaw toward the nearest player.
    pub rot: f32,
    pub o_rot: f32,
    pub t_rot: f32,
}

impl EnchantingTableBookState {
    fn new(pos: BlockPos) -> Self {
        Self {
            pos,
            time: 0,
            flip: 0.0,
            o_flip: 0.0,
            flip_t: 0.0,
            flip_a: 0.0,
            open: 0.0,
            o_open: 0.0,
            rot: 0.0,
            o_rot: 0.0,
            t_rot: 0.0,
        }
    }
}

/// One enchanting-table block's per-frame render source: the raw
/// `EnchantingTableBlockEntity` fields `EnchantTableRenderer.extractRenderState`
/// interpolates (the partial-tick lerp runs on the projection side).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnchantingTableBookSourceState {
    pub pos: BlockPos,
    pub time: i32,
    pub flip: f32,
    pub o_flip: f32,
    pub open: f32,
    pub o_open: f32,
    pub rot: f32,
    pub o_rot: f32,
}

/// Whether a block name is the enchanting table (`minecraft:enchanting_table` —
/// the one `EnchantingTableBlock` registration; it has no facing/state the book
/// renderer reads).
pub fn is_enchanting_table_block_name(block_name: &str) -> bool {
    block_name == "minecraft:enchanting_table"
}

/// A candidate player position for the nearest-player search
/// (`level.players()` minus spectators, `EntitySelector.NO_SPECTATORS`).
#[derive(Debug, Clone, Copy)]
struct PlayerCandidate {
    x: f64,
    y: f64,
    z: f64,
}

impl WorldStore {
    /// Advances every loaded enchanting-table book by `ticks` client ticks,
    /// transcribing `EnchantingTableBlockEntity.bookAnimationTick`
    /// (`java:50-106`) run once per tick per table. State entries for blocks
    /// that are no longer enchanting tables (destroyed or unloaded — vanilla
    /// drops the block entity with the block/chunk) are pruned; newly loaded
    /// tables gain a fresh (all-zero) state. Tables tick in a deterministic
    /// position-sorted order so the shared [`EnchantingBookRandom`] draw stream
    /// stays reproducible (vanilla ticks in block-entity insertion order off a
    /// wall-clock random).
    pub fn advance_enchanting_table_book_ticks(&mut self, ticks: u32) {
        // Reconcile the tracked set with the loaded enchanting tables even when
        // no ticks elapse, so the render projection sees fresh tables.
        let mut positions = self.enchanting_table_positions();
        positions.sort_by_key(|pos| (pos.y, pos.z, pos.x));

        let mut books = std::mem::take(&mut self.enchanting_table_books);
        books.retain(|book| {
            positions
                .binary_search_by_key(&(book.pos.y, book.pos.z, book.pos.x), |pos| {
                    (pos.y, pos.z, pos.x)
                })
                .is_ok()
        });
        for pos in &positions {
            if !books.iter().any(|book| book.pos == *pos) {
                books.push(EnchantingTableBookState::new(*pos));
            }
        }
        books.sort_by_key(|book| (book.pos.y, book.pos.z, book.pos.x));

        if ticks > 0 && !books.is_empty() {
            let players = self.player_candidates();
            let mut random = std::mem::take(&mut self.enchanting_book_random);
            for _ in 0..ticks {
                for book in &mut books {
                    book_animation_tick(book, &players, &mut random);
                }
            }
            self.enchanting_book_random = random;
        }

        self.enchanting_table_books = books;
    }

    pub fn enchanting_table_book_states(&self) -> &[EnchantingTableBookState] {
        &self.enchanting_table_books
    }

    /// Enumerates every enchanting-table block in the loaded chunks as a render
    /// source, folding in the tracked animation state (an untracked table rests
    /// at the all-zero fields of a fresh block entity — a closed book).
    /// Sections whose block palette holds no enchanting table are skipped
    /// wholesale, mirroring the chest/sign/bell palette pre-check. Sorted by
    /// position for a deterministic frame order.
    pub fn enchanting_table_book_source_states(&self) -> Vec<EnchantingTableBookSourceState> {
        let mut states = Vec::new();
        for chunk in &self.chunks {
            self.collect_chunk_enchanting_table_book_source_states(chunk, &mut states);
        }
        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
        states
    }

    fn collect_chunk_enchanting_table_book_source_states(
        &self,
        chunk: &ChunkColumn,
        states: &mut Vec<EnchantingTableBookSourceState>,
    ) {
        for (section_index, section) in chunk.sections.iter().enumerate() {
            if !section_palette_may_contain_enchanting_table(
                &section.block_states.palette_global_ids,
                section.block_states.palette_kind,
                &self.registries,
            ) {
                continue;
            }
            let Ok(section_offset) = i32::try_from(section_index) else {
                continue;
            };
            let section_min_y = (self.dimension.min_section_y() + section_offset) * 16;
            for index in 0..section.block_states.entry_count {
                let Some(value) = section.block_states.value_at(index) else {
                    continue;
                };
                let Some(block_state) = self.registries.block_state(value.global_id) else {
                    continue;
                };
                if !is_enchanting_table_block_name(&block_state.name) {
                    continue;
                }
                let pos = BlockPos {
                    x: chunk.pos.x * 16 + (index & 0xF) as i32,
                    y: section_min_y + (index >> 8) as i32,
                    z: chunk.pos.z * 16 + ((index >> 4) & 0xF) as i32,
                };
                let book = self
                    .enchanting_table_books
                    .iter()
                    .find(|book| book.pos == pos)
                    .copied()
                    .unwrap_or_else(|| EnchantingTableBookState::new(pos));
                states.push(EnchantingTableBookSourceState {
                    pos,
                    time: book.time,
                    flip: book.flip,
                    o_flip: book.o_flip,
                    open: book.open,
                    o_open: book.o_open,
                    rot: book.rot,
                    o_rot: book.o_rot,
                });
            }
        }
    }

    /// Every loaded enchanting-table block position (unsorted).
    fn enchanting_table_positions(&self) -> Vec<BlockPos> {
        let mut positions = Vec::new();
        for chunk in &self.chunks {
            for (section_index, section) in chunk.sections.iter().enumerate() {
                if !section_palette_may_contain_enchanting_table(
                    &section.block_states.palette_global_ids,
                    section.block_states.palette_kind,
                    &self.registries,
                ) {
                    continue;
                }
                let Ok(section_offset) = i32::try_from(section_index) else {
                    continue;
                };
                let section_min_y = (self.dimension.min_section_y() + section_offset) * 16;
                for index in 0..section.block_states.entry_count {
                    let Some(value) = section.block_states.value_at(index) else {
                        continue;
                    };
                    let Some(block_state) = self.registries.block_state(value.global_id) else {
                        continue;
                    };
                    if !is_enchanting_table_block_name(&block_state.name) {
                        continue;
                    }
                    positions.push(BlockPos {
                        x: chunk.pos.x * 16 + (index & 0xF) as i32,
                        y: section_min_y + (index >> 8) as i32,
                        z: chunk.pos.z * 16 + ((index >> 4) & 0xF) as i32,
                    });
                }
            }
        }
        positions
    }

    /// The candidate players for the nearest-player search: the local player
    /// plus every remote player entity, minus spectators
    /// (`EntitySelector.NO_SPECTATORS`; creative players stay in), mirroring
    /// the particle nearest-player context.
    fn player_candidates(&self) -> Vec<PlayerCandidate> {
        let mut players = Vec::new();
        if !self.local_player_is_spectator() {
            if let Some(pose) = self.local_player_pose() {
                players.push(PlayerCandidate {
                    x: pose.position.x,
                    y: pose.position.y,
                    z: pose.position.z,
                });
            }
        }
        for transform in self.entity_transforms() {
            if transform.entity_type_id != crate::entities::VANILLA_ENTITY_TYPE_PLAYER_ID {
                continue;
            }
            if self
                .player_info_entry(transform.uuid)
                .is_some_and(|info| info.is_spectator())
            {
                continue;
            }
            players.push(PlayerCandidate {
                x: transform.position.x,
                y: transform.position.y,
                z: transform.position.z,
            });
        }
        players
    }
}

/// Vanilla `EnchantingTableBlockEntity.bookAnimationTick` for one table:
/// `oOpen`/`oRot` capture, the nearest-player-driven `tRot`/`open` update, the
/// random page-flip target, the `rot`/`tRot` `(-π, π]` wraps and the
/// `rot += rotDir·0.4` chase, the `open` clamp, and the `flip`/`flipA` step.
fn book_animation_tick(
    book: &mut EnchantingTableBookState,
    players: &[PlayerCandidate],
    random: &mut EnchantingBookRandom,
) {
    book.o_open = book.open;
    book.o_rot = book.rot;

    let center_x = f64::from(book.pos.x) + 0.5;
    let center_y = f64::from(book.pos.y) + 0.5;
    let center_z = f64::from(book.pos.z) + 0.5;
    let nearest = nearest_player(players, center_x, center_y, center_z);

    if let Some(player) = nearest {
        let xd = player.x - center_x;
        let zd = player.z - center_z;
        book.t_rot = (zd.atan2(xd)) as f32;
        book.open += 0.1;
        if book.open < 0.5 || random.next_int(40) == 0 {
            let old = book.flip_t;
            loop {
                book.flip_t += (random.next_int(4) - random.next_int(4)) as f32;
                if old != book.flip_t {
                    break;
                }
            }
        }
    } else {
        book.t_rot += 0.02;
        book.open -= 0.1;
    }

    book.rot = wrap_radians(book.rot);
    book.t_rot = wrap_radians(book.t_rot);
    let rot_dir = wrap_radians(book.t_rot - book.rot);
    book.rot += rot_dir * 0.4;
    book.open = book.open.clamp(0.0, 1.0);
    book.time += 1;
    book.o_flip = book.flip;
    let diff = ((book.flip_t - book.flip) * 0.4).clamp(-0.2, 0.2);
    book.flip_a += (diff - book.flip_a) * 0.9;
    book.flip += book.flip_a;
}

/// Vanilla's nearest-player pick: minimum `distanceToSqr` within
/// `ENCHANTING_BOOK_NEAREST_PLAYER_RANGE`, or `None` when no player is in
/// range.
fn nearest_player(players: &[PlayerCandidate], x: f64, y: f64, z: f64) -> Option<PlayerCandidate> {
    let range_sqr = ENCHANTING_BOOK_NEAREST_PLAYER_RANGE * ENCHANTING_BOOK_NEAREST_PLAYER_RANGE;
    let mut best: Option<(f64, PlayerCandidate)> = None;
    for player in players {
        let dx = player.x - x;
        let dy = player.y - y;
        let dz = player.z - z;
        let dist = dx * dx + dy * dy + dz * dz;
        if dist < range_sqr && best.map_or(true, |(best_dist, _)| dist < best_dist) {
            best = Some((dist, *player));
        }
    }
    best.map(|(_, player)| player)
}

/// Vanilla's `while (v >= π) v -= 2π; while (v < -π) v += 2π;` fold into
/// `(-π, π]`.
fn wrap_radians(mut value: f32) -> f32 {
    use std::f32::consts::PI;
    while value >= PI {
        value -= 2.0 * PI;
    }
    while value < -PI {
        value += 2.0 * PI;
    }
    value
}

/// Whether a section's block palette can hold the enchanting table. Local and
/// single-value palettes are answered from the palette id list; a global
/// palette stores raw state ids, so it must be scanned.
fn section_palette_may_contain_enchanting_table(
    palette_global_ids: &[i32],
    palette_kind: PaletteKind,
    registries: &RegistrySet,
) -> bool {
    match palette_kind {
        PaletteKind::SingleValue | PaletteKind::Local => {
            palette_global_ids.iter().any(|global_id| {
                registries
                    .block_state(*global_id)
                    .is_some_and(|state| is_enchanting_table_block_name(&state.name))
            })
        }
        PaletteKind::Global => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PalettedContainerData,
        WorldDimension,
    };
    use bbb_protocol::packets::{BlockPos as ProtocolBlockPos, BlockUpdate};
    use std::collections::BTreeMap;

    const VANILLA_AIR_BLOCK_STATE_ID: i32 = 0;

    fn world_with_air_chunk() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    VANILLA_AIR_BLOCK_STATE_ID,
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        world
    }

    fn single_value_container(
        domain: PaletteDomain,
        entry_count: usize,
        global_id: i32,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn set_block(world: &mut WorldStore, pos: BlockPos, name: &str) {
        let properties: BTreeMap<String, String> = BTreeMap::new();
        let state_id = world
            .registries
            .block_state_id_by_name_and_properties(name, &properties)
            .unwrap_or_else(|| panic!("no registered state for {name}"));
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id: state_id,
        }));
    }

    /// A reference `bookAnimationTick` used to cross-check the world tick chain
    /// hand-derives the same values as the transcribed vanilla method.
    fn reference_tick(
        book: &mut EnchantingTableBookState,
        player: Option<(f64, f64, f64)>,
        random: &mut EnchantingBookRandom,
    ) {
        let candidates: Vec<PlayerCandidate> = player
            .into_iter()
            .map(|(x, y, z)| PlayerCandidate { x, y, z })
            .collect();
        book_animation_tick(book, &candidates, random);
    }

    #[test]
    fn nearest_player_respects_the_three_block_range() {
        let players = [
            PlayerCandidate {
                x: 2.6,
                y: 0.5,
                z: 0.5,
            },
            PlayerCandidate {
                x: 10.0,
                y: 0.5,
                z: 0.5,
            },
        ];
        // Center at (0.5, 0.5, 0.5): the first player is 2.1 away (in range),
        // the second 9.5 (out of range).
        let near = nearest_player(&players, 0.5, 0.5, 0.5);
        assert_eq!(near.map(|p| p.x), Some(2.6));
        // A player exactly 3.0 away is excluded (distSqr == range², not <).
        let boundary = [PlayerCandidate {
            x: 3.5,
            y: 0.5,
            z: 0.5,
        }];
        assert!(nearest_player(&boundary, 0.5, 0.5, 0.5).is_none());
    }

    #[test]
    fn open_and_rot_chase_the_nearest_player_then_relax_when_absent() {
        let mut book = EnchantingTableBookState::new(BlockPos { x: 0, y: 0, z: 0 });
        let mut random = EnchantingBookRandom::default();
        // Player straight along +x from the block centre: tRot = atan2(0, +) = 0.
        for _ in 0..3 {
            reference_tick(&mut book, Some((2.5, 0.5, 0.5)), &mut random);
        }
        // open += 0.1 per present tick -> 0.3.
        assert!((book.open - 0.3).abs() < 1e-5);
        // tRot = 0 and rot chases it (starts at 0) so rot stays 0.
        assert!(book.rot.abs() < 1e-5);
        assert_eq!(book.time, 3);
        // With no player, open decays by 0.1/tick and tRot drifts by 0.02.
        let open_before = book.open;
        reference_tick(&mut book, None, &mut random);
        assert!((book.open - (open_before - 0.1)).abs() < 1e-5);
        assert!((book.t_rot - 0.02).abs() < 1e-5);
        // oOpen/oRot captured the pre-tick values for the renderer lerp.
        assert!((book.o_open - open_before).abs() < 1e-5);
    }

    #[test]
    fn page_flip_target_moves_and_flip_eases_toward_it() {
        let mut book = EnchantingTableBookState::new(BlockPos { x: 0, y: 0, z: 0 });
        let mut random = EnchantingBookRandom::default();
        // While open < 0.5 the flip target is re-rolled every tick (the
        // short-circuit skips the 1/40 gate); flipT must change each roll.
        let mut last = book.flip_t;
        for _ in 0..3 {
            reference_tick(&mut book, Some((2.5, 0.5, 0.5)), &mut random);
            assert_ne!(book.flip_t, last, "flipT re-rolls until it changes");
            last = book.flip_t;
        }
        // flip eases toward flipT via flipA (both start at 0, so after the first
        // nonzero flipT the flip has begun to move).
        assert!(book.flip.abs() > 0.0 || book.flip_a.abs() > 0.0);
    }

    #[test]
    fn advance_tracks_new_tables_and_prunes_removed_ones() {
        let mut world = world_with_air_chunk();
        let a = BlockPos { x: 3, y: 4, z: 5 };
        let b = BlockPos { x: 6, y: 4, z: 5 };
        set_block(&mut world, a, "minecraft:enchanting_table");
        set_block(&mut world, b, "minecraft:enchanting_table");
        world.advance_enchanting_table_book_ticks(1);
        assert_eq!(world.enchanting_table_book_states().len(), 2);
        assert!(world
            .enchanting_table_book_states()
            .iter()
            .all(|book| book.time == 1));
        // Removing one prunes its state on the next advance.
        set_block(&mut world, a, "minecraft:air");
        world.advance_enchanting_table_book_ticks(1);
        let states = world.enchanting_table_book_states();
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].pos, b);
        assert_eq!(states[0].time, 2);
    }

    #[test]
    fn source_states_enumerate_tables_with_raw_animation_fields() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_block(&mut world, pos, "minecraft:enchanting_table");
        // An untracked table projects the closed-book default.
        let sources = world.enchanting_table_book_source_states();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].pos, pos);
        assert_eq!(sources[0].time, 0);
        assert_eq!(sources[0].open, 0.0);
        // After a tick the raw fields flow through.
        world.advance_enchanting_table_book_ticks(1);
        let sources = world.enchanting_table_book_source_states();
        assert_eq!(sources[0].time, 1);
    }

    #[test]
    fn random_is_deterministic_across_identical_tick_streams() {
        let mut first = EnchantingBookRandom::default();
        let mut second = EnchantingBookRandom::default();
        for _ in 0..50 {
            assert_eq!(first.next_int(40), second.next_int(40));
            assert_eq!(first.next_int(4), second.next_int(4));
        }
    }
}
