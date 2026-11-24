//! The chunk column data model: paletted sections, biomes,
//! feature heightmaps and the generation-state payloads.
//!
//! Pure data that worldgen, persistence and the chunk store all read; none of
//! them is reachable from here.

pub mod biomes;
pub mod feature_heightmaps;
pub mod full;
pub mod spawn;

pub use biomes::SectionBiomes;
pub use feature_heightmaps::{pack_heightmap_values, FeatureHeightmaps};

mod server_column {
    use super::{full, spawn, FeatureHeightmaps, SectionBiomes};
    use crate::chunk_ticks::{BlockTickType, ChunkError, ChunkTicks, FluidTickType};
    use bbb_protocol::block_state_types::{
        vanilla_block_state_fluid_randomly_ticking, vanilla_block_state_randomly_ticking,
    };
    use bbb_protocol::codec::{Decoder, Encoder};

    /// Built-in Java 26.1 biome registry length. Registry-aware constructors use
    /// the frozen published length instead; this default preserves standalone
    /// fixture and wire-test callers.
    pub const VANILLA_26_1_BIOME_REGISTRY_LEN: usize = 65;

    /// Decode one primitive, mapping protocol errors into [`ChunkError`] with
    /// context about which wire field failed.
    macro_rules! read_wire {
        ($decoder:expr, $method:ident, $what:literal) => {
            $decoder
                .$method()
                .map_err(|err| ChunkError(format!("failed to decode {}: {err}", $what)))
        };
    }

    /// Cells per section axis (vanilla `bitsPerAxis = 4` for block states).
    pub const SECTION_SIDE: usize = 16;
    /// Cells per 16x16x16 block-state section (`Strategy.entryCount`).
    pub const SECTION_VOLUME: usize = SECTION_SIDE * SECTION_SIDE * SECTION_SIDE;
    /// Minimum bit width for a non-single block-state palette
    /// (`Strategy.FOUR_BITS_LINEAR`, Strategy.java:14).
    const MIN_INDIRECT_BITS: u8 = 4;
    /// Maximum bit width of the indirect (palette-listed) wire form for block
    /// states; past 8 bits vanilla switches to the global palette
    /// (`Strategy.createForBlockStates`, Strategy.java:33-46).
    const MAX_WIRE_INDIRECT_BITS: u8 = 8;
    /// Largest palette the indirect wire form carries (2^8).
    const MAX_INDIRECT_PALETTE_LEN: usize = 1 << MAX_WIRE_INDIRECT_BITS;
    /// Bit width of the global-palette wire form: `ceillog2` of the Vanilla 26.1
    /// global block-state registry size (`Strategy.globalPaletteBitsInMemory`).
    /// The data words then hold global block-state ids directly, with no palette
    /// on the wire (`GlobalPalette.write`).
    const GLOBAL_PALETTE_BITS: u8 = 15;
    /// Maximum bit width of in-memory indirect storage; sections never hold more
    /// than 4096 distinct states, so 12 bits always suffice.
    const MAX_INDIRECT_BITS: u8 = 15;
    /// Distinct block states a section can physically hold; caps wire palette sizes.
    const MAX_PALETTE_LEN: usize = SECTION_VOLUME;

    fn global_palette_bits_for_count(block_state_count: i32) -> u8 {
        let count = usize::try_from(block_state_count).unwrap_or(1).max(2);
        (usize::BITS - (count - 1).leading_zeros()) as u8
    }

    // The global wire form is exactly ceillog2(VANILLA_BLOCK_STATE_COUNT) bits;
    // keep the constant honest if the catalog ever changes.
    const _: () = assert!(
        GLOBAL_PALETTE_BITS as usize
            == usize::BITS as usize
                - (bbb_protocol::block_state_types::VANILLA_BLOCK_STATE_COUNT as usize - 1)
                    .leading_zeros() as usize
    );

    pub fn section_index(x: usize, y: usize, z: usize) -> usize {
        // Strategy.getIndex, Strategy.java:69-71.
        (y << 4 | z) << 4 | x
    }

    /// Smallest `b` with `2^b >= n` for `n >= 1` (vanilla `Mth.ceillog2`).
    fn ceil_log2(n: usize) -> u8 {
        debug_assert!(n >= 1);
        if n <= 1 {
            0
        } else {
            (usize::BITS - (n - 1).leading_zeros()) as u8
        }
    }

    /// Bit width for an indirect palette of `len` entries:
    /// `max(4, ceillog2(len))` capped at [`MAX_INDIRECT_BITS`].
    fn bits_for_palette_len(len: usize) -> u8 {
        ceil_log2(len).clamp(MIN_INDIRECT_BITS, MAX_INDIRECT_BITS)
    }

    /// Entries packed per u64 word (SimpleBitStorage.java:249).
    fn values_per_word(bits: u8) -> usize {
        64 / bits as usize
    }

    /// u64 words needed for a full section at `bits` (padded; entries never
    /// straddle words, SimpleBitStorage.java:244-256).
    fn data_words(bits: u8) -> usize {
        SECTION_VOLUME.div_ceil(values_per_word(bits))
    }

    #[inline]
    pub(super) fn read_cell(data: &[u64], bits: u8, index: usize) -> usize {
        match bits {
            4 => ((data[index >> 4] >> ((index & 15) << 2)) & 0xF) as usize,
            5 => ((data[index / 12] >> ((index % 12) * 5)) & 0x1F) as usize,
            6 => ((data[index / 10] >> ((index % 10) * 6)) & 0x3F) as usize,
            7 => ((data[index / 9] >> ((index % 9) * 7)) & 0x7F) as usize,
            8 => ((data[index >> 3] >> ((index & 7) << 3)) & 0xFF) as usize,
            9 => ((data[index / 7] >> ((index % 7) * 9)) & 0x1FF) as usize,
            10 => ((data[index / 6] >> ((index % 6) * 10)) & 0x3FF) as usize,
            11 => ((data[index / 5] >> ((index % 5) * 11)) & 0x7FF) as usize,
            12 => ((data[index / 5] >> ((index % 5) * 12)) & 0xFFF) as usize,
            15 => ((data[index >> 2] >> ((index & 3) * 15)) & 0x7FFF) as usize,
            _ => {
                let per_word = values_per_word(bits);
                let word = data[index / per_word];
                let shift = (index % per_word) * bits as usize;
                ((word >> shift) & ((1_u64 << bits) - 1)) as usize
            }
        }
    }

    fn count_palette_entries(bits: u8, data: &[u64], counts: &mut [u16]) {
        let per_word = values_per_word(bits);
        let mask = (1_u64 << bits) - 1;
        let mut remaining = SECTION_VOLUME;
        for &word in data {
            let mut w = word;
            let n = remaining.min(per_word);
            for _ in 0..n {
                counts[(w & mask) as usize] += 1;
                w >>= bits;
            }
            remaining -= n;
        }
    }

    const PALETTE_BITSET_WORDS: usize = MAX_INDIRECT_PALETTE_LEN / 64;

    #[inline]
    fn bitset_contains(words: &[u64], index: usize) -> bool {
        (words[index >> 6] >> (index & 63)) & 1 != 0
    }

    fn write_cell(data: &mut [u64], bits: u8, index: usize, value: usize) {
        let per_word = values_per_word(bits);
        let word = index / per_word;
        let shift = (index % per_word) * bits as usize;
        let mask = (1_u64 << bits) - 1;
        data[word] = data[word] & !(mask << shift) | ((value as u64) & mask) << shift;
    }

    /// Re-pack every cell of `data` from `bits` into a wider `new_bits` array.
    fn repack(data: &[u64], bits: u8, new_bits: u8) -> Vec<u64> {
        debug_assert!(new_bits > bits);
        let mut out = vec![0_u64; data_words(new_bits)];
        for index in 0..SECTION_VOLUME {
            let value = read_cell(data, bits, index);
            write_cell(&mut out, new_bits, index, value);
        }
        out
    }

    /// One 16x16x16 section's block states, vanilla PalettedContainer semantics.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum SectionBlocks {
        /// All 4096 cells hold the same global block-state id (0 bits on the wire).
        Single(i32),
        /// Palette-indexed storage; `palette` holds global block-state ids and
        /// `data` packs palette indices LSB-first at `bits` per entry.
        Indirect {
            bits: u8,
            palette: Vec<i32>,
            data: Vec<u64>,
        },
    }

    impl SectionBlocks {
        /// Local coordinates are 0..16; out-of-range values are saturated so no
        /// input can panic (callers in [`ChunkColumn`] always validate first).
        pub fn get(&self, x: usize, y: usize, z: usize) -> i32 {
            let index = section_index(x.min(15), y.min(15), z.min(15));
            match self {
                Self::Single(state) => *state,
                Self::Indirect {
                    bits,
                    palette,
                    data,
                } => palette[read_cell(data, *bits, index)],
            }
        }

        /// Returns the previous global block-state id.
        pub fn set(&mut self, x: usize, y: usize, z: usize, state: i32) -> i32 {
            let index = section_index(x.min(15), y.min(15), z.min(15));
            match self {
                Self::Single(current) => {
                    let previous = *current;
                    if state != previous {
                        // First diverging write: 0 bits -> 4 bits linear palette
                        // (Strategy.java:37-40).
                        let mut data = vec![0_u64; data_words(MIN_INDIRECT_BITS)];
                        write_cell(&mut data, MIN_INDIRECT_BITS, index, 1);
                        *self = Self::Indirect {
                            bits: MIN_INDIRECT_BITS,
                            palette: vec![previous, state],
                            data,
                        };
                    }
                    previous
                }
                Self::Indirect {
                    bits,
                    palette,
                    data,
                } => {
                    let previous = palette[read_cell(data, *bits, index)];
                    if previous == state {
                        return previous;
                    }
                    let palette_index = match palette.iter().position(|&entry| entry == state) {
                        Some(existing) => existing,
                        None => {
                            palette.push(state);
                            let needed = bits_for_palette_len(palette.len());
                            if needed > *bits {
                                *data = repack(data, *bits, needed);
                                *bits = needed;
                            }
                            palette.len() - 1
                        }
                    };
                    write_cell(data, *bits, index, palette_index);
                    previous
                }
            }
        }

        /// Packs a whole section from `palette` and each cell's palette index,
        /// in [`section_index`] order.
        pub fn from_palette_indices(palette: Vec<i32>, indices: &[u16; SECTION_VOLUME]) -> Self {
            if palette.len() == 1 {
                return Self::Single(palette[0]);
            }
            let bits = bits_for_palette_len(palette.len());
            let mut data = vec![0_u64; data_words(bits)];
            for (index, &value) in indices.iter().enumerate() {
                write_cell(&mut data, bits, index, usize::from(value));
            }
            Self::Indirect {
                bits,
                palette,
                data,
            }
        }

        /// Count of cells not holding `air_state_id`; the wire header's
        /// non-empty block count (`LevelChunkSection.write`,
        /// LevelChunkSection.java:182).
        pub fn non_air_count(&self, air_state_id: i32) -> u16 {
            self.count_cells(|state| state != air_state_id)
        }

        /// Cells whose block state satisfies `predicate`.
        ///
        /// The predicate is evaluated once per distinct palette entry, not once
        /// per cell, which is how vanilla's `recalcBlockCounts` counts as well
        /// (`PalettedContainer.count` walks the palette,
        /// LevelChunkSection.java:118-146).
        pub fn count_cells(&self, predicate: impl Fn(i32) -> bool) -> u16 {
            match self {
                Self::Single(state) => {
                    if predicate(*state) {
                        SECTION_VOLUME as u16
                    } else {
                        0
                    }
                }
                Self::Indirect {
                    bits,
                    palette,
                    data,
                } => {
                    let mut stack_bits = [0_u64; PALETTE_BITSET_WORDS];
                    let mut heap_bits;
                    let bits_slice: &mut [u64] = if palette.len() <= MAX_INDIRECT_PALETTE_LEN {
                        &mut stack_bits
                    } else {
                        heap_bits = vec![0_u64; palette.len().div_ceil(64)];
                        &mut heap_bits
                    };
                    let mut any_true = false;
                    let mut any_false = false;
                    for (idx, &state) in palette.iter().enumerate() {
                        if predicate(state) {
                            bits_slice[idx >> 6] |= 1_u64 << (idx & 63);
                            any_true = true;
                        } else {
                            any_false = true;
                        }
                    }
                    if !any_true {
                        return 0;
                    }
                    if !any_false {
                        return SECTION_VOLUME as u16;
                    }
                    let per_word = values_per_word(*bits);
                    let mask = (1_u64 << *bits) - 1;
                    let mut remaining = SECTION_VOLUME;
                    let mut total = 0_u16;
                    for &word in data {
                        let mut w = word;
                        let n = remaining.min(per_word);
                        for _ in 0..n {
                            if bitset_contains(bits_slice, (w & mask) as usize) {
                                total += 1;
                            }
                            w >>= *bits;
                        }
                        remaining -= n;
                    }
                    total
                }
            }
        }

        /// Whether any cell's block state satisfies `predicate`.
        pub fn any_cell(&self, predicate: impl Fn(i32) -> bool) -> bool {
            match self {
                Self::Single(state) => predicate(*state),
                Self::Indirect {
                    bits,
                    palette,
                    data,
                } => {
                    let mut stack_bits = [0_u64; PALETTE_BITSET_WORDS];
                    let mut heap_bits;
                    let bits_slice: &mut [u64] = if palette.len() <= MAX_INDIRECT_PALETTE_LEN {
                        &mut stack_bits
                    } else {
                        heap_bits = vec![0_u64; palette.len().div_ceil(64)];
                        &mut heap_bits
                    };
                    let mut any_true = false;
                    let mut any_false = false;
                    for (idx, &state) in palette.iter().enumerate() {
                        if predicate(state) {
                            bits_slice[idx >> 6] |= 1_u64 << (idx & 63);
                            any_true = true;
                        } else {
                            any_false = true;
                        }
                    }
                    if !any_true {
                        return false;
                    }
                    if !any_false {
                        return true;
                    }
                    let per_word = values_per_word(*bits);
                    let mask = (1_u64 << *bits) - 1;
                    let mut remaining = SECTION_VOLUME;
                    for &word in data {
                        let mut w = word;
                        let n = remaining.min(per_word);
                        for _ in 0..n {
                            if bitset_contains(bits_slice, (w & mask) as usize) {
                                return true;
                            }
                            w >>= *bits;
                        }
                        remaining -= n;
                    }
                    false
                }
            }
        }

        /// Whether any cell holds a non-air block state (`!block_state_is_air`).
        pub fn any_non_air(&self) -> bool {
            self.any_cell(|state| !crate::fluid::block_state_is_air(state))
        }

        /// Whether every cell holds an air block state (`#minecraft:air`).
        pub fn has_only_air(&self) -> bool {
            !self.any_non_air()
        }

        fn encode_container_with_block_state_count(
            &self,
            out: &mut Encoder,
            block_state_count: i32,
        ) {
            match self {
                Self::Single(state) => {
                    out.write_u8(0);
                    out.write_var_i32(*state);
                }
                Self::Indirect {
                    bits,
                    palette,
                    data,
                } => {
                    if palette.len() <= MAX_INDIRECT_PALETTE_LEN {
                        // Indirect wire form: the palette list precedes the data
                        // words (PalettedContainer.Data.write).
                        out.write_u8(*bits);
                        out.write_var_i32(palette.len() as i32);
                        for &entry in palette {
                            out.write_var_i32(entry);
                        }
                        for &word in data {
                            out.write_i64(word as i64);
                        }
                    } else {
                        // Global wire form (Strategy.java:44): no palette list;
                        // the data words hold global block-state ids at the
                        // registry-wide bit width (`ceillog2(block_state_count)`).
                        let global_bits = global_palette_bits_for_count(block_state_count);
                        out.write_u8(global_bits);
                        let mut global = vec![0_u64; data_words(global_bits)];
                        for index in 0..SECTION_VOLUME {
                            let id = palette[read_cell(data, *bits, index)];
                            debug_assert!(id >= 0 && id < block_state_count);
                            write_cell(&mut global, global_bits, index, id as usize);
                        }
                        for &word in &global {
                            out.write_i64(word as i64);
                        }
                    }
                }
            }
        }

        fn decode_with_block_state_count(
            decoder: &mut Decoder<'_>,
            block_state_count: i32,
        ) -> Result<Self, ChunkError> {
            let bits = read_wire!(decoder, read_u8, "section bits")?;
            if bits == 0 {
                let value = read_wire!(decoder, read_var_i32, "single-value palette")?;
                return Ok(Self::Single(value));
            }
            if bits > MAX_WIRE_INDIRECT_BITS {
                // Global palette form: past 8 bits vanilla ignores the exact byte
                // and reads the registry-wide `ceillog2(block_state_count)`-bit array
                // with no palette list (`PalettedContainer.read` -> `Configuration.Global`).
                let global_bits = global_palette_bits_for_count(block_state_count);
                let words = data_words(global_bits);
                let mut global = Vec::with_capacity(words);
                for _ in 0..words {
                    global.push(read_wire!(decoder, read_i64, "section data word")? as u64);
                }
                // Materialize as an indirect section, preserving the
                // first-appearance order of the global ids.
                let mut palette: Vec<i32> = Vec::new();
                let mut indices = vec![0_usize; SECTION_VOLUME];
                for (index, cell) in indices.iter_mut().enumerate() {
                    let id = read_cell(&global, global_bits, index) as i32;
                    if id >= block_state_count {
                        return Err(ChunkError(format!(
                            "global block-state id {id} outside active registry count {block_state_count}"
                        )));
                    }
                    *cell = match palette.iter().position(|&entry| entry == id) {
                        Some(existing) => existing,
                        None => {
                            palette.push(id);
                            palette.len() - 1
                        }
                    };
                }
                let bits = bits_for_palette_len(palette.len());
                let mut data = vec![0_u64; data_words(bits)];
                for (index, &palette_index) in indices.iter().enumerate() {
                    write_cell(&mut data, bits, index, palette_index);
                }
                return Ok(Self::Indirect {
                    bits,
                    palette,
                    data,
                });
            }
            // Vanilla maps wire bits 1..=4 onto 4-bit storage (Strategy.java:37-40).
            let bits = bits.max(MIN_INDIRECT_BITS);
            let palette_len = read_wire!(decoder, read_var_i32, "palette length")?;
            if !(1..=MAX_PALETTE_LEN as i32).contains(&palette_len) {
                return Err(ChunkError(format!(
                    "palette length {palette_len} outside 1..={MAX_PALETTE_LEN}"
                )));
            }
            let mut palette = Vec::with_capacity(palette_len as usize);
            for _ in 0..palette_len {
                palette.push(read_wire!(decoder, read_var_i32, "palette entry")?);
            }
            let words = data_words(bits);
            let mut data = Vec::with_capacity(words);
            for _ in 0..words {
                data.push(read_wire!(decoder, read_i64, "section data word")? as u64);
            }
            for index in 0..SECTION_VOLUME {
                let palette_index = read_cell(&data, bits, index);
                if palette_index >= palette.len() {
                    return Err(ChunkError(format!(
                        "palette index {palette_index} out of range for palette of {}",
                        palette.len()
                    )));
                }
            }
            Ok(Self::Indirect {
                bits,
                palette,
                data,
            })
        }
    }

    /// `BlockState.isRandomlyTicking()` for a stored global state id.
    ///
    /// A state id outside the built-in 26.1 registry never randomly ticks: the
    /// store accepts arbitrary ids (tests use synthetic ones) and vanilla has no
    /// behavior to attribute to them.
    fn state_randomly_ticks(state: i32) -> bool {
        vanilla_block_state_randomly_ticking(state).unwrap_or(false)
    }

    /// `BlockState.getFluidState().isRandomlyTicking()` for a stored global state
    /// id; see [`state_randomly_ticks`] for out-of-registry ids.
    fn state_fluid_randomly_ticks(state: i32) -> bool {
        vanilla_block_state_fluid_randomly_ticking(state).unwrap_or(false)
    }

    fn state_has_fluid(state: i32) -> bool {
        !crate::fluid::block_state_fluid_state(state).is_empty()
    }

    /// One column section: its blocks plus the non-air, fluid, and ticking
    /// counters vanilla keeps beside them (`LevelChunkSection.nonEmptyBlockCount`
    /// / `tickingBlockCount` / `tickingFluidCount`, LevelChunkSection.java:18-20).
    ///
    /// Maintaining these incrementally inside `set` avoids scanning 4,096 cells
    /// per section on random-tick checks and `LevelChunkSection.write` wire encoding.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ChunkSection {
        blocks: SectionBlocks,
        biomes: SectionBiomes,
        non_air_count: u16,
        fluid_count: u16,
        ticking_block_count: u16,
        ticking_fluid_count: u16,
    }

    /// `LevelChunkSection.setBlockState` / `recalcBlockCounts`
    /// (`LevelChunkSection.java:72-100`, `LevelChunkSection.java:125-147`)
    /// treats any state with `BlockState.isAir()` (`air`, `cave_air`, `void_air`) as air,
    /// while fixture callers may also pass a synthetic `air_state_id`.
    #[inline]
    fn is_section_air(state: i32, air_state_id: i32) -> bool {
        state == air_state_id || crate::fluid::block_state_is_air(state)
    }

    impl ChunkSection {
        /// Wraps existing blocks, deriving the counters from them
        /// (`LevelChunkSection.recalcBlockCounts`, LevelChunkSection.java:118-146,
        /// which vanilla likewise runs when a section arrives from a save).
        pub fn new(blocks: SectionBlocks) -> Self {
            Self::with_biomes(
                blocks,
                SectionBiomes::single(0, VANILLA_26_1_BIOME_REGISTRY_LEN)
                    .expect("the built-in biome registry contains id zero"),
            )
        }

        pub fn with_biomes(blocks: SectionBlocks, biomes: SectionBiomes) -> Self {
            Self::with_biomes_and_air(blocks, biomes, 0)
        }

        pub fn with_biomes_and_air(
            blocks: SectionBlocks,
            biomes: SectionBiomes,
            air_state_id: i32,
        ) -> Self {
            let (non_air_count, fluid_count, ticking_block_count, ticking_fluid_count) =
                match &blocks {
                    SectionBlocks::Single(state) => {
                        let vol = SECTION_VOLUME as u16;
                        (
                            if !is_section_air(*state, air_state_id) {
                                vol
                            } else {
                                0
                            },
                            if state_has_fluid(*state) { vol } else { 0 },
                            if state_randomly_ticks(*state) { vol } else { 0 },
                            if state_fluid_randomly_ticks(*state) {
                                vol
                            } else {
                                0
                            },
                        )
                    }
                    SectionBlocks::Indirect {
                        bits,
                        palette,
                        data,
                    } => {
                        let mut stack_counts = [0_u16; MAX_INDIRECT_PALETTE_LEN];
                        let mut heap_counts;
                        let counts: &mut [u16] = if palette.len() <= MAX_INDIRECT_PALETTE_LEN {
                            &mut stack_counts[..palette.len()]
                        } else {
                            heap_counts = vec![0_u16; palette.len()];
                            &mut heap_counts
                        };
                        count_palette_entries(*bits, data, counts);
                        let mut non_air = 0_u16;
                        let mut fluid = 0_u16;
                        let mut ticking_block = 0_u16;
                        let mut ticking_fluid = 0_u16;
                        for (&state, &count) in palette.iter().zip(counts.iter()) {
                            if count == 0 {
                                continue;
                            }
                            if !is_section_air(state, air_state_id) {
                                non_air += count;
                            }
                            if state_has_fluid(state) {
                                fluid += count;
                            }
                            if state_randomly_ticks(state) {
                                ticking_block += count;
                            }
                            if state_fluid_randomly_ticks(state) {
                                ticking_fluid += count;
                            }
                        }
                        (non_air, fluid, ticking_block, ticking_fluid)
                    }
                };
            Self {
                blocks,
                biomes,
                non_air_count,
                fluid_count,
                ticking_block_count,
                ticking_fluid_count,
            }
        }

        pub fn blocks(&self) -> &SectionBlocks {
            &self.blocks
        }

        pub fn biomes(&self) -> &SectionBiomes {
            &self.biomes
        }

        fn biomes_mut(&mut self) -> &mut SectionBiomes {
            &mut self.biomes
        }

        pub fn get(&self, x: usize, y: usize, z: usize) -> i32 {
            self.blocks.get(x, y, z)
        }

        /// Returns the previous global block-state id, keeping the section
        /// counters exact (LevelChunkSection.java:74-100).
        pub fn set(&mut self, x: usize, y: usize, z: usize, state: i32) -> i32 {
            self.set_with_air(x, y, z, state, 0)
        }

        fn set_with_air(
            &mut self,
            x: usize,
            y: usize,
            z: usize,
            state: i32,
            air_state_id: i32,
        ) -> i32 {
            let previous = self.blocks.set(x, y, z, state);
            if previous == state {
                return previous;
            }
            if !is_section_air(previous, air_state_id) {
                debug_assert!(self.non_air_count > 0);
                self.non_air_count = self.non_air_count.saturating_sub(1);
            }
            if state_has_fluid(previous) {
                debug_assert!(self.fluid_count > 0);
                self.fluid_count = self.fluid_count.saturating_sub(1);
            }
            if state_randomly_ticks(previous) {
                debug_assert!(self.ticking_block_count > 0);
                self.ticking_block_count = self.ticking_block_count.saturating_sub(1);
            }
            if state_fluid_randomly_ticks(previous) {
                debug_assert!(self.ticking_fluid_count > 0);
                self.ticking_fluid_count = self.ticking_fluid_count.saturating_sub(1);
            }
            if !is_section_air(state, air_state_id) {
                self.non_air_count += 1;
            }
            if state_has_fluid(state) {
                self.fluid_count += 1;
            }
            if state_randomly_ticks(state) {
                self.ticking_block_count += 1;
            }
            if state_fluid_randomly_ticks(state) {
                self.ticking_fluid_count += 1;
            }
            previous
        }

        /// `LevelChunkSection.isRandomlyTicking` (:113-115).
        pub fn is_randomly_ticking(&self) -> bool {
            self.is_randomly_ticking_blocks() || self.is_randomly_ticking_fluids()
        }

        /// `LevelChunkSection.isRandomlyTickingBlocks` (:117-119).
        pub fn is_randomly_ticking_blocks(&self) -> bool {
            self.ticking_block_count > 0
        }

        /// `LevelChunkSection.isRandomlyTickingFluids` (:121-123).
        pub fn is_randomly_ticking_fluids(&self) -> bool {
            self.ticking_fluid_count > 0
        }

        pub fn non_air_count(&self) -> u16 {
            self.non_air_count
        }

        /// Whether every cell in this section holds an air block state, using
        /// the maintained `non_air_count` as an O(1) fast path when
        /// `air_state_id` itself is air.
        pub fn has_only_air(&self, air_state_id: i32) -> bool {
            if self.non_air_count == 0 && crate::fluid::block_state_is_air(air_state_id) {
                return true;
            }
            self.blocks.has_only_air()
        }

        pub fn ticking_block_count(&self) -> u16 {
            self.ticking_block_count
        }

        pub fn ticking_fluid_count(&self) -> u16 {
            self.ticking_fluid_count
        }
    }

    /// One chunk column: sections indexed from `min_y`, 16 blocks each.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ChunkColumn {
        min_y: i32,
        sections: Vec<ChunkSection>,
        biome_registry_len: usize,
        inhabited_time: i64,
        /// Global block-state id treated as air for the wire non-air count.
        air_state_id: i32,
        /// Pending block ticks for this column (`LevelChunk.blockTicks`, a
        /// `LevelChunkTicks<Block>`). Ticks are chunk state, not section state,
        /// so they take no part in [`ChunkColumn::encode_section_data`].
        block_ticks: ChunkTicks<BlockTickType>,
        /// Pending fluid ticks for this column (`LevelChunk.fluidTicks`, a
        /// `LevelChunkTicks<Fluid>`).
        fluid_ticks: ChunkTicks<FluidTickType>,
    }

    impl ChunkColumn {
        /// All sections start as Single(air_state_id); biome is a single-value palette.
        pub fn new(min_y: i32, section_count: usize, air_state_id: i32, biome_id: i32) -> Self {
            Self::new_with_biome_registry(
                min_y,
                section_count,
                air_state_id,
                biome_id,
                VANILLA_26_1_BIOME_REGISTRY_LEN,
            )
        }

        pub fn new_with_biome_registry(
            min_y: i32,
            section_count: usize,
            air_state_id: i32,
            biome_id: i32,
            biome_registry_len: usize,
        ) -> Self {
            let biomes = SectionBiomes::single(biome_id, biome_registry_len)
                .expect("a column's initial biome must belong to its frozen registry");
            Self {
                min_y,
                sections: vec![
                    ChunkSection::with_biomes_and_air(
                        SectionBlocks::Single(air_state_id),
                        biomes,
                        air_state_id,
                    );
                    section_count
                ],
                biome_registry_len,
                inhabited_time: 0,
                air_state_id,
                block_ticks: ChunkTicks::new(),
                fluid_ticks: ChunkTicks::new(),
            }
        }

        pub fn min_y(&self) -> i32 {
            self.min_y
        }

        pub fn section_count(&self) -> usize {
            self.sections.len()
        }

        pub const fn inhabited_time(&self) -> i64 {
            self.inhabited_time
        }

        pub fn set_inhabited_time(&mut self, inhabited_time: i64) {
            self.inhabited_time = inhabited_time.max(0);
        }

        pub fn increment_inhabited_time(&mut self, delta: i64) {
            self.inhabited_time = self.inhabited_time.saturating_add(delta.max(0));
        }

        pub fn biome_id(&self) -> Option<i32> {
            let first = self.sections.first()?.biomes().get(0, 0, 0);
            self.sections
                .iter()
                .all(|section| match section.biomes() {
                    SectionBiomes::Single(id) => *id == first,
                    biomes => {
                        (0..4).all(|y| (0..4).all(|z| (0..4).all(|x| biomes.get(x, y, z) == first)))
                    }
                })
                .then_some(first)
        }

        pub fn air_state_id(&self) -> i32 {
            self.air_state_id
        }

        /// The blocks of the section at `index`, counting up from [`Self::min_y`].
        ///
        /// The save writer reads a column section by section, because that is the
        /// unit a saved chunk stores.
        pub fn section(&self, index: usize) -> &SectionBlocks {
            self.sections[index].blocks()
        }

        /// Replaces the blocks of the section at `index`, keeping its biomes.
        pub fn replace_section_blocks(&mut self, index: usize, blocks: SectionBlocks) {
            let biomes =
                std::mem::replace(&mut self.sections[index].biomes, SectionBiomes::Single(0));
            self.sections[index] =
                ChunkSection::with_biomes_and_air(blocks, biomes, self.air_state_id);
        }

        /// Replaces the biomes of the section at `index` with a uniform biome.
        pub fn replace_section_single_biome(
            &mut self,
            index: usize,
            biome: i32,
        ) -> Result<(), ChunkError> {
            self.sections[index].biomes = SectionBiomes::single(biome, self.biome_registry_len)?;
            Ok(())
        }

        /// The section at `index` including its ticking counters.
        pub fn chunk_section(&self, index: usize) -> &ChunkSection {
            &self.sections[index]
        }

        /// Whether every cell in section `index` holds an air block state.
        pub fn section_has_only_air(&self, index: usize) -> bool {
            self.sections[index].has_only_air(self.air_state_id)
        }

        pub fn section_biomes(&self, index: usize) -> &SectionBiomes {
            self.sections[index].biomes()
        }

        pub fn biome_registry_len(&self) -> usize {
            self.biome_registry_len
        }

        /// Pending block ticks for this column.
        pub fn block_ticks(&self) -> &ChunkTicks<BlockTickType> {
            &self.block_ticks
        }

        pub fn block_ticks_mut(&mut self) -> &mut ChunkTicks<BlockTickType> {
            &mut self.block_ticks
        }

        /// Pending fluid ticks for this column.
        pub fn fluid_ticks(&self) -> &ChunkTicks<FluidTickType> {
            &self.fluid_ticks
        }

        pub fn fluid_ticks_mut(&mut self) -> &mut ChunkTicks<FluidTickType> {
            &mut self.fluid_ticks
        }

        /// `x`/`z` are chunk-local (0..16). `None` outside the y range or outside
        /// the local x/z range.
        pub fn get_block_state(&self, x: i32, y: i32, z: i32) -> Option<i32> {
            let (section, local) = self.locate(x, y, z)?;
            Some(self.sections[section].get(
                local[0] as usize,
                local[1] as usize,
                local[2] as usize,
            ))
        }

        /// Returns the previous global block-state id, or `None` outside the
        /// column (no column is created, nothing panics).
        pub fn set_block_state(&mut self, x: i32, y: i32, z: i32, state: i32) -> Option<i32> {
            let (section, local) = self.locate(x, y, z)?;
            Some(self.sections[section].set_with_air(
                local[0] as usize,
                local[1] as usize,
                local[2] as usize,
                state,
                self.air_state_id,
            ))
        }

        /// Quart coordinates use chunk-local x/z and world-relative y.
        pub fn get_noise_biome(&self, x: i32, quart_y: i32, z: i32) -> Option<i32> {
            let (section, local_y) = self.locate_biome(x, quart_y, z)?;
            Some(
                self.sections[section]
                    .biomes()
                    .get(x as usize, local_y, z as usize),
            )
        }

        pub fn set_noise_biome(
            &mut self,
            x: i32,
            quart_y: i32,
            z: i32,
            biome: i32,
        ) -> Result<Option<i32>, ChunkError> {
            let Some((section, local_y)) = self.locate_biome(x, quart_y, z) else {
                return Ok(None);
            };
            self.sections[section]
                .biomes_mut()
                .set(
                    x as usize,
                    local_y,
                    z as usize,
                    biome,
                    self.biome_registry_len,
                )
                .map(Some)
        }

        fn locate_biome(&self, x: i32, quart_y: i32, z: i32) -> Option<(usize, usize)> {
            if !(0..4).contains(&x) || !(0..4).contains(&z) {
                return None;
            }
            let min_quart_y = self.min_y.div_euclid(4);
            let offset = quart_y.checked_sub(min_quart_y)?;
            if offset < 0 || offset as usize >= self.sections.len() * 4 {
                return None;
            }
            Some((offset as usize / 4, offset as usize % 4))
        }

        fn locate(&self, x: i32, y: i32, z: i32) -> Option<(usize, [i32; 3])> {
            if !(0..SECTION_SIDE as i32).contains(&x) || !(0..SECTION_SIDE as i32).contains(&z) {
                return None;
            }
            let offset = y.checked_sub(self.min_y)?;
            if offset < 0 || offset as usize >= self.sections.len() * SECTION_SIDE {
                return None;
            }
            let section = offset as usize / SECTION_SIDE;
            let local_y = offset % SECTION_SIDE as i32;
            Some((section, [x, local_y, z]))
        }

        /// Serialize to the wire bytes `LevelChunkSection.write`
        /// (LevelChunkSection.java:181-186) produces per section: i16 non-air
        /// count, i16 fluid count, block-state container, biome container.
        pub fn encode_section_data(&self) -> Vec<u8> {
            self.encode_section_data_with_block_state_count(
                bbb_protocol::block_state_types::VANILLA_BLOCK_STATE_COUNT,
            )
        }

        /// Registry-aware form of [`Self::encode_section_data`] that sizes
        /// global-palette sections from `block_state_count` (`ceillog2`).
        pub fn encode_section_data_with_block_state_count(
            &self,
            block_state_count: i32,
        ) -> Vec<u8> {
            let mut out = Encoder::new();
            for section in &self.sections {
                out.write_i16(section.non_air_count as i16);
                out.write_i16(section.fluid_count as i16);
                section
                    .blocks
                    .encode_container_with_block_state_count(&mut out, block_state_count);
                section.biomes().encode(&mut out, self.biome_registry_len);
            }
            out.into_inner()
        }

        /// Decode wire bytes produced by [`ChunkColumn::encode_section_data`]
        /// (round-trip for restore tests). The section count is implied by the
        /// byte length. The air id is not present on the wire; it is recovered
        /// from the first all-air section (non-air count 0) and otherwise falls
        /// back to 0 (the vanilla global id of the default `minecraft:air`
        /// state). A decoded column whose air id fell back re-encodes with
        /// different non-air counts than the original; block states and palettes
        /// are unaffected.
        pub fn decode_section_data(min_y: i32, bytes: &[u8]) -> Result<Self, ChunkError> {
            Self::decode_section_data_with_biome_registry(
                min_y,
                bytes,
                VANILLA_26_1_BIOME_REGISTRY_LEN,
            )
        }

        pub fn decode_section_data_with_biome_registry(
            min_y: i32,
            bytes: &[u8],
            biome_registry_len: usize,
        ) -> Result<Self, ChunkError> {
            Self::decode_section_data_with_registries(
                min_y,
                bytes,
                biome_registry_len,
                bbb_protocol::block_state_types::VANILLA_BLOCK_STATE_COUNT,
            )
        }

        pub fn decode_section_data_with_registries(
            min_y: i32,
            bytes: &[u8],
            biome_registry_len: usize,
            block_state_count: i32,
        ) -> Result<Self, ChunkError> {
            let mut decoder = Decoder::new(bytes);
            let mut raw_sections = Vec::new();
            let mut air_state_id: Option<i32> = None;
            while !decoder.is_empty() {
                let non_air = read_wire!(decoder, read_i16, "non-air block count")?;
                if non_air < 0 {
                    return Err(ChunkError(format!(
                        "negative non-air block count {non_air}"
                    )));
                }
                // Fluid count is derived again from block states when encoding.
                read_wire!(decoder, read_i16, "fluid count")?;
                let section =
                    SectionBlocks::decode_with_block_state_count(&mut decoder, block_state_count)?;
                let biomes = SectionBiomes::decode(&mut decoder, biome_registry_len)?;
                if air_state_id.is_none() && non_air == 0 {
                    // Every cell of this section is air, regardless of the
                    // palette form.
                    air_state_id = Some(section.get(0, 0, 0));
                }
                raw_sections.push((section, biomes));
            }
            if raw_sections.is_empty() {
                return Err(ChunkError("section data holds no sections".to_string()));
            }
            let air_state_id = air_state_id.unwrap_or(0);
            let sections = raw_sections
                .into_iter()
                .map(|(blocks, biomes)| {
                    ChunkSection::with_biomes_and_air(blocks, biomes, air_state_id)
                })
                .collect();
            Ok(Self {
                min_y,
                sections,
                biome_registry_len,
                inhabited_time: 0,
                air_state_id,
                block_ticks: ChunkTicks::new(),
                fluid_ticks: ChunkTicks::new(),
            })
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ChunkPreCarversStatus {
        Empty,
        StructureStarts,
        StructureReferences,
        Biomes,
        Noise,
        Surface,
    }

    impl ChunkPreCarversStatus {
        pub const fn is_at_least(&self, other: &Self) -> bool {
            (*self as u8) >= (*other as u8)
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ChunkPreCarversState {
        status: ChunkPreCarversStatus,
        postprocessing_positions: Vec<(i32, i32, i32)>,
    }

    impl ChunkPreCarversState {
        pub fn new(
            status: ChunkPreCarversStatus,
            postprocessing_positions: Vec<(i32, i32, i32)>,
        ) -> Self {
            Self {
                status,
                postprocessing_positions,
            }
        }

        pub const fn status(&self) -> &ChunkPreCarversStatus {
            &self.status
        }

        pub fn advance(&mut self, status: ChunkPreCarversStatus) {
            debug_assert!(status.is_at_least(&self.status));
            self.status = status;
        }

        pub fn postprocessing_positions(&self) -> &[(i32, i32, i32)] {
            &self.postprocessing_positions
        }

        pub fn clear_postprocessing_positions(&mut self) {
            self.postprocessing_positions.clear();
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ChunkGenerationState {
        Full {
            surface_y: i32,
            postprocessing_positions: Vec<(i32, i32, i32)>,
        },
        Carvers {
            postprocessing_positions: Vec<(i32, i32, i32)>,
            carving_mask: Vec<i64>,
        },
        Features(ChunkFeaturesState),
        InitializeLight(ChunkInitializeLightState),
        Light(ChunkLightState),
        Spawn(spawn::ChunkSpawnState),
        NoiseFull(full::ChunkNoiseFullState),
    }

    impl ChunkGenerationState {
        pub fn features(&self) -> Option<&ChunkFeaturesState> {
            match self {
                Self::Features(features) => Some(features),
                Self::InitializeLight(light) => Some(&light.features),
                Self::Light(light) => Some(&light.initialized.features),
                Self::Spawn(spawn) => Some(&spawn.light().initialized.features),
                Self::NoiseFull(full) => Some(&full.light().initialized.features),
                _ => None,
            }
        }

        pub fn features_mut(&mut self) -> Option<&mut ChunkFeaturesState> {
            match self {
                Self::Features(features) => Some(features),
                Self::InitializeLight(light) => Some(&mut light.features),
                Self::Light(light) => Some(&mut light.initialized.features),
                Self::Spawn(spawn) => Some(&mut spawn.light_mut().initialized.features),
                Self::NoiseFull(full) => Some(&mut full.light_mut().initialized.features),
                _ => None,
            }
        }

        pub fn postprocessing_positions_mut(&mut self) -> &mut Vec<(i32, i32, i32)> {
            match self {
                Self::Carvers {
                    postprocessing_positions,
                    ..
                }
                | Self::Full {
                    postprocessing_positions,
                    ..
                } => postprocessing_positions,
                state => {
                    &mut state
                        .features_mut()
                        .expect("typed feature payload")
                        .postprocessing_positions
                }
            }
        }

        pub fn invalidate_feature_lighting(&mut self) {
            if !matches!(
                self,
                Self::InitializeLight(_) | Self::Light(_) | Self::Spawn(_)
            ) {
                return;
            }
            let previous = std::mem::replace(
                self,
                Self::Carvers {
                    postprocessing_positions: Vec::new(),
                    carving_mask: Vec::new(),
                },
            );
            *self = Self::Features(match previous {
                Self::InitializeLight(light) => light.features,
                Self::Light(light) => light.initialized.features,
                Self::Spawn(spawn) => spawn.into_light().initialized.features,
                _ => unreachable!(),
            });
        }

        pub fn complete_features(&mut self, final_heightmaps: FeatureHeightmaps) {
            let Self::Carvers {
                postprocessing_positions,
                carving_mask,
            } = self
            else {
                return;
            };
            *self = Self::Features(ChunkFeaturesState {
                postprocessing_positions: std::mem::take(postprocessing_positions),
                carving_mask: std::mem::take(carving_mask),
                final_heightmaps,
            });
        }

        pub fn clear_postprocessing_positions(&mut self) {
            match self {
                Self::Full {
                    postprocessing_positions,
                    ..
                }
                | Self::Carvers {
                    postprocessing_positions,
                    ..
                } => postprocessing_positions.clear(),
                Self::Features(features) => features.postprocessing_positions.clear(),
                Self::InitializeLight(initialized) => {
                    initialized.features.postprocessing_positions.clear()
                }
                Self::Light(light) => light.initialized.features.postprocessing_positions.clear(),
                Self::Spawn(spawn) => spawn.clear_postprocessing_positions(),
                Self::NoiseFull(full) => full.clear_postprocessing_positions(),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ChunkFeaturesState {
        pub postprocessing_positions: Vec<(i32, i32, i32)>,
        pub carving_mask: Vec<i64>,
        pub final_heightmaps: FeatureHeightmaps,
    }

    impl ChunkFeaturesState {
        pub fn postprocessing_positions(&self) -> &[(i32, i32, i32)] {
            &self.postprocessing_positions
        }

        pub fn carving_mask(&self) -> &[i64] {
            &self.carving_mask
        }

        pub const fn final_heightmaps(&self) -> &FeatureHeightmaps {
            &self.final_heightmaps
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ChunkInitializeLightState {
        pub features: ChunkFeaturesState,
        pub lowest_sky_source_y: [i32; 16 * 16],
        pub non_empty_sections: Vec<i32>,
    }

    impl ChunkInitializeLightState {
        pub const fn features(&self) -> &ChunkFeaturesState {
            &self.features
        }

        pub const fn lowest_sky_source_y(&self) -> &[i32; 16 * 16] {
            &self.lowest_sky_source_y
        }

        pub fn non_empty_sections(&self) -> &[i32] {
            &self.non_empty_sections
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ChunkLightState {
        pub initialized: ChunkInitializeLightState,

        pub is_light_correct: bool,
    }

    impl ChunkLightState {
        pub const fn initialized(&self) -> &ChunkInitializeLightState {
            &self.initialized
        }

        pub const fn is_light_correct(&self) -> bool {
            self.is_light_correct
        }
    }
}

use server_column::read_cell;
pub use server_column::*;

#[cfg(test)]
mod tests;
