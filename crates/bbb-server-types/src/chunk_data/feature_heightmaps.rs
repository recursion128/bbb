use std::collections::BTreeMap;

use bbb_protocol::heightmap_types::{
    VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
    VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID, VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
};

use crate::block_queries::is_leaves;
use crate::chunk_data::ChunkColumn;
use crate::fluid::{block_state_blocks_motion, block_state_fluid_state, block_state_is_air};
use crate::save::{SaveError, SaveResult};

const COLUMN_SIDE: i32 = 16;
pub const HEIGHTMAP_ENTRIES: usize = 256;
const COLUMN_AREA: usize = HEIGHTMAP_ENTRIES;
const MAX_STACK_PALETTE_MASKS: usize = 256;

pub const WORLD_SURFACE_MASK: u8 = 1 << 0;
pub const OCEAN_FLOOR_MASK: u8 = 1 << 1;
pub const MOTION_BLOCKING_MASK: u8 = 1 << 2;
pub const MOTION_BLOCKING_NO_LEAVES_MASK: u8 = 1 << 3;
pub const ALL_HEIGHTMAPS_MASK: u8 =
    WORLD_SURFACE_MASK | OCEAN_FLOOR_MASK | MOTION_BLOCKING_MASK | MOTION_BLOCKING_NO_LEAVES_MASK;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureHeightmaps {
    world_surface: [i32; COLUMN_AREA],
    ocean_floor: [i32; COLUMN_AREA],
    motion_blocking: [i32; COLUMN_AREA],
    motion_blocking_no_leaves: [i32; COLUMN_AREA],
}

impl FeatureHeightmaps {
    // Heightmap.java:43-78: one position uses the same predicate and downward scan.
    pub fn scan_first_available(
        column: &ChunkColumn,
        heightmap_kind_id: i32,
        x: i32,
        z: i32,
    ) -> Option<i32> {
        column_index(x, z)?;
        let predicate = match heightmap_kind_id {
            VANILLA_HEIGHTMAP_WORLD_SURFACE_ID => HeightmapPredicate::WorldSurface,
            VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID => HeightmapPredicate::OceanFloor,
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID => HeightmapPredicate::MotionBlocking,
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID => {
                HeightmapPredicate::MotionBlockingNoLeaves
            }
            _ => return None,
        };
        Some(scan_one(column, predicate, x, z, column_max_y(column)))
    }

    /// `Heightmap.primeHeightmaps` (`Heightmap.java:43-78`).
    pub fn prime(column: &ChunkColumn) -> Self {
        Self::prime_with_mask(column, ALL_HEIGHTMAPS_MASK)
    }

    /// Primes only the heightmap kinds selected in `requested_mask`, leaving
    /// unselected kinds at `column.min_y()`.
    pub fn prime_with_mask(column: &ChunkColumn, requested_mask: u8) -> Self {
        let [world_surface, ocean_floor, motion_blocking, motion_blocking_no_leaves] =
            prime_masked(column, requested_mask);
        Self {
            world_surface,
            ocean_floor,
            motion_blocking,
            motion_blocking_no_leaves,
        }
    }

    /// `SerializableChunkData.read` restores every persisted heightmap and
    /// primes only a missing kind (`SerializableChunkData.java:287-298`).
    pub fn from_saved(
        column: &ChunkColumn,
        heightmaps: &BTreeMap<String, Vec<i64>>,
    ) -> SaveResult<Self> {
        let world_height = column
            .section_count()
            .checked_mul(COLUMN_SIDE as usize)
            .ok_or_else(|| invalid_heightmap("chunk.Heightmaps", "column height overflows"))?;
        let world_surface =
            unpack_optional_saved(heightmaps, "WORLD_SURFACE", column.min_y(), world_height)?;
        let ocean_floor =
            unpack_optional_saved(heightmaps, "OCEAN_FLOOR", column.min_y(), world_height)?;
        let motion_blocking =
            unpack_optional_saved(heightmaps, "MOTION_BLOCKING", column.min_y(), world_height)?;
        let motion_blocking_no_leaves = unpack_optional_saved(
            heightmaps,
            "MOTION_BLOCKING_NO_LEAVES",
            column.min_y(),
            world_height,
        )?;
        let missing_mask = u8::from(world_surface.is_none())
            | (u8::from(ocean_floor.is_none()) << 1)
            | (u8::from(motion_blocking.is_none()) << 2)
            | (u8::from(motion_blocking_no_leaves.is_none()) << 3);
        if missing_mask == 0 {
            return Ok(Self {
                world_surface: world_surface.expect("present"),
                ocean_floor: ocean_floor.expect("present"),
                motion_blocking: motion_blocking.expect("present"),
                motion_blocking_no_leaves: motion_blocking_no_leaves.expect("present"),
            });
        }
        let primed = prime_masked(column, missing_mask);
        Ok(Self {
            world_surface: world_surface.unwrap_or(primed[0]),
            ocean_floor: ocean_floor.unwrap_or(primed[1]),
            motion_blocking: motion_blocking.unwrap_or(primed[2]),
            motion_blocking_no_leaves: motion_blocking_no_leaves.unwrap_or(primed[3]),
        })
    }

    pub fn first_available(&self, heightmap_kind_id: i32, x: i32, z: i32) -> Option<i32> {
        let index = column_index(x, z)?;
        let values = self.values(heightmap_kind_id)?;
        Some(values[index])
    }

    pub fn values(&self, heightmap_kind_id: i32) -> Option<&[i32; COLUMN_AREA]> {
        match heightmap_kind_id {
            VANILLA_HEIGHTMAP_WORLD_SURFACE_ID => Some(&self.world_surface),
            VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID => Some(&self.ocean_floor),
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID => Some(&self.motion_blocking),
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID => Some(&self.motion_blocking_no_leaves),
            _ => None,
        }
    }

    pub fn packed_values(
        &self,
        heightmap_kind_id: i32,
        min_y: i32,
        world_height: usize,
        path: &str,
    ) -> SaveResult<Vec<i64>> {
        let values = self
            .values(heightmap_kind_id)
            .ok_or_else(|| invalid_heightmap(path, "heightmap kind is not a final heightmap"))?;
        pack_heightmap_values(values, min_y, world_height, path)
    }

    /// `Heightmap.update` (`Heightmap.java:81-107`).
    pub fn update_after_set(
        &mut self,
        column: &ChunkColumn,
        x: i32,
        y: i32,
        z: i32,
        new_state: i32,
    ) {
        let Some(index) = column_index(x, z) else {
            return;
        };
        if column.get_block_state(x, y, z).is_none() {
            return;
        }

        update_one(
            &mut self.world_surface,
            HeightmapPredicate::WorldSurface,
            column,
            index,
            x,
            y,
            z,
            new_state,
        );
        update_one(
            &mut self.ocean_floor,
            HeightmapPredicate::OceanFloor,
            column,
            index,
            x,
            y,
            z,
            new_state,
        );
        update_one(
            &mut self.motion_blocking,
            HeightmapPredicate::MotionBlocking,
            column,
            index,
            x,
            y,
            z,
            new_state,
        );
        update_one(
            &mut self.motion_blocking_no_leaves,
            HeightmapPredicate::MotionBlockingNoLeaves,
            column,
            index,
            x,
            y,
            z,
            new_state,
        );
    }
}

/// Packs the 256 first-available Y values stored by `Heightmap.getRawData`
/// (`Heightmap.java:36-40,118-123,136-137`; `SimpleBitStorage.java:244-264,284-291`).
pub fn pack_heightmap_values(
    values: &[i32; HEIGHTMAP_ENTRIES],
    min_y: i32,
    world_height: usize,
    path: &str,
) -> SaveResult<Vec<i64>> {
    if world_height == 0 {
        return Err(invalid_heightmap(path, "column height must be positive"));
    }
    let bits = usize::BITS as usize - world_height.leading_zeros() as usize;
    if !(1..=32).contains(&bits) {
        return Err(invalid_heightmap(
            path,
            "heightmap bit width is outside SimpleBitStorage's 1..=32 range",
        ));
    }
    let max_y = i64::from(min_y)
        .checked_add(
            i64::try_from(world_height)
                .map_err(|_| invalid_heightmap(path, "column height does not fit i64"))?,
        )
        .ok_or_else(|| invalid_heightmap(path, "column height range overflows"))?;
    let values_per_long = 64 / bits;
    let mut packed = vec![0_i64; HEIGHTMAP_ENTRIES.div_ceil(values_per_long)];
    for (index, value) in values.iter().enumerate() {
        let value = i64::from(*value);
        if value < i64::from(min_y) || value > max_y {
            return Err(invalid_heightmap(
                &format!("{path}[{index}]"),
                format!("first-available Y {value} is outside {min_y}..={max_y}"),
            ));
        }
        let relative = u64::try_from(value - i64::from(min_y))
            .map_err(|_| invalid_heightmap(&format!("{path}[{index}]"), "value overflows"))?;
        packed[index / values_per_long] |= (relative << ((index % values_per_long) * bits)) as i64;
    }
    Ok(packed)
}

fn unpack_optional_saved(
    heightmaps: &BTreeMap<String, Vec<i64>>,
    key: &str,
    min_y: i32,
    world_height: usize,
) -> SaveResult<Option<[i32; COLUMN_AREA]>> {
    let Some(packed) = heightmaps.get(key) else {
        return Ok(None);
    };
    let path = format!("chunk.Heightmaps.{key}");
    if world_height == 0 {
        return Err(invalid_heightmap(&path, "column height must be positive"));
    }
    // `Heightmap.java:36-40,118-123`; `SimpleBitStorage.java:244-260,294-300`.
    let bits = usize::BITS as usize - world_height.leading_zeros() as usize;
    if !(1..=32).contains(&bits) {
        return Err(invalid_heightmap(
            &path,
            "heightmap bit width is outside SimpleBitStorage's 1..=32 range",
        ));
    }
    let values_per_long = 64 / bits;
    let expected_longs = COLUMN_AREA.div_ceil(values_per_long);
    if packed.len() != expected_longs {
        return Err(invalid_heightmap(
            &path,
            format!("must contain exactly {expected_longs} longs"),
        ));
    }
    let mask = (1_u64 << bits) - 1;
    let mut values = [min_y; COLUMN_AREA];
    for (index, value) in values.iter_mut().enumerate() {
        let word = packed[index / values_per_long] as u64;
        let shift = (index % values_per_long) * bits;
        let relative = (word >> shift) & mask;
        if relative > world_height as u64 {
            return Err(invalid_heightmap(
                &format!("{path}[{index}]"),
                format!("relative first-available Y {relative} exceeds height {world_height}"),
            ));
        }
        *value = min_y
            .checked_add(i32::try_from(relative).map_err(|_| {
                invalid_heightmap(&format!("{path}[{index}]"), "heightmap value overflows i32")
            })?)
            .ok_or_else(|| {
                invalid_heightmap(&format!("{path}[{index}]"), "heightmap Y range overflows")
            })?;
    }
    Ok(Some(values))
}

fn invalid_heightmap(path: &str, detail: impl Into<String>) -> SaveError {
    SaveError::InvalidNbt {
        path: None,
        detail: format!("{path}: {}", detail.into()),
    }
}

#[derive(Clone, Copy)]
enum HeightmapPredicate {
    WorldSurface,
    OceanFloor,
    MotionBlocking,
    MotionBlockingNoLeaves,
}

impl HeightmapPredicate {
    /// `Heightmap.Types` predicates (`Heightmap.java:144-155`).
    fn test(self, state: i32) -> bool {
        match self {
            Self::WorldSurface => !block_state_is_air(state),
            Self::OceanFloor => block_state_blocks_motion(state),
            Self::MotionBlocking => {
                block_state_blocks_motion(state) || !block_state_fluid_state(state).is_empty()
            }
            Self::MotionBlockingNoLeaves => {
                (block_state_blocks_motion(state) || !block_state_fluid_state(state).is_empty())
                    && !is_leaves(state)
            }
        }
    }
}

const PREDICATES: [HeightmapPredicate; 4] = [
    HeightmapPredicate::WorldSurface,
    HeightmapPredicate::OceanFloor,
    HeightmapPredicate::MotionBlocking,
    HeightmapPredicate::MotionBlockingNoLeaves,
];

fn predicate_mask(state: i32) -> u8 {
    PREDICATES
        .iter()
        .enumerate()
        .filter(|(_, predicate)| predicate.test(state))
        .fold(0, |mask, (bit, _)| mask | 1 << bit)
}

fn prime_masked(column: &ChunkColumn, requested_mask: u8) -> [[i32; COLUMN_AREA]; 4] {
    let mut values = [[column.min_y(); COLUMN_AREA]; 4];
    let requested_mask = requested_mask & ALL_HEIGHTMAPS_MASK;
    if requested_mask == 0 {
        return values;
    }
    // Bit k of `pending[i]` is set while kind k has not found its surface at cell i.
    let mut pending = [requested_mask; COLUMN_AREA];
    let mut remaining = COLUMN_AREA * requested_mask.count_ones() as usize;
    let mut stack_masks = [0_u8; MAX_STACK_PALETTE_MASKS];
    let mut heap_masks: Vec<u8> = Vec::new();
    for index in (0..column.section_count()).rev() {
        let section_min_y = column.min_y() + index as i32 * COLUMN_SIDE;
        match column.section(index) {
            crate::chunk_data::SectionBlocks::Single(state) => {
                let single_mask = predicate_mask(*state) & requested_mask;
                if single_mask == 0 {
                    continue;
                }
                // A uniform section hits its highest matching block at `local_y = 15`.
                let first_available = section_min_y + COLUMN_SIDE;
                for cell in 0..COLUMN_AREA {
                    let mask = pending[cell] & single_mask;
                    if mask == 0 {
                        continue;
                    }
                    for (kind, kind_values) in values.iter_mut().enumerate() {
                        if mask & (1 << kind) != 0 {
                            kind_values[cell] = first_available;
                            remaining -= 1;
                        }
                    }
                    pending[cell] &= !mask;
                }
                if remaining == 0 {
                    return values;
                }
            }
            crate::chunk_data::SectionBlocks::Indirect {
                bits,
                palette,
                data,
            } => {
                let masks: &[u8] = if palette.len() <= MAX_STACK_PALETTE_MASKS {
                    let mut any = 0_u8;
                    for (dst, &state) in stack_masks.iter_mut().zip(palette.iter()) {
                        let m = predicate_mask(state) & requested_mask;
                        *dst = m;
                        any |= m;
                    }
                    if any == 0 {
                        continue;
                    }
                    &stack_masks[..palette.len()]
                } else {
                    heap_masks.clear();
                    heap_masks.reserve(palette.len());
                    let mut any = 0_u8;
                    for &state in palette {
                        let m = predicate_mask(state) & requested_mask;
                        heap_masks.push(m);
                        any |= m;
                    }
                    if any == 0 {
                        continue;
                    }
                    &heap_masks
                };
                for local_y in (0..crate::chunk_data::SECTION_SIDE).rev() {
                    let y_offset = local_y << 8;
                    let first_available = section_min_y + local_y as i32 + 1;
                    for cell in 0..COLUMN_AREA {
                        let open = pending[cell];
                        if open == 0 {
                            continue;
                        }
                        let mask = masks
                            [crate::chunk_data::read_cell(data, *bits, y_offset | cell)]
                            & open;
                        if mask == 0 {
                            continue;
                        }
                        for (kind, kind_values) in values.iter_mut().enumerate() {
                            if mask & (1 << kind) != 0 {
                                kind_values[cell] = first_available;
                                remaining -= 1;
                            }
                        }
                        pending[cell] &= !mask;
                    }
                    if remaining == 0 {
                        return values;
                    }
                }
            }
        }
    }
    values
}

fn column_max_y(column: &ChunkColumn) -> i32 {
    column
        .min_y()
        .checked_add(
            i32::try_from(column.section_count())
                .expect("a chunk column section count fits i32")
                .checked_mul(COLUMN_SIDE)
                .expect("a chunk column height fits i32"),
        )
        .expect("a chunk column y range fits i32")
}

fn scan_one(
    column: &ChunkColumn,
    predicate: HeightmapPredicate,
    x: i32,
    z: i32,
    max_y_exclusive: i32,
) -> i32 {
    for y in (column.min_y()..max_y_exclusive).rev() {
        let state = column
            .get_block_state(x, y, z)
            .expect("loop coordinates belong to the column");
        if predicate.test(state) {
            return y + 1;
        }
    }
    column.min_y()
}

#[allow(clippy::too_many_arguments)]
fn update_one(
    values: &mut [i32; COLUMN_AREA],
    predicate: HeightmapPredicate,
    column: &ChunkColumn,
    index: usize,
    x: i32,
    y: i32,
    z: i32,
    new_state: i32,
) {
    let first_available = values[index];
    if i64::from(y) <= i64::from(first_available) - 2 {
        return;
    }

    if predicate.test(new_state) {
        if y >= first_available {
            values[index] = y + 1;
        }
        return;
    }
    if i64::from(first_available) - 1 != i64::from(y) {
        return;
    }

    for scan_y in (column.min_y()..y).rev() {
        let state = column
            .get_block_state(x, scan_y, z)
            .expect("the rescan stays inside the column");
        if predicate.test(state) {
            values[index] = scan_y + 1;
            return;
        }
    }
    values[index] = column.min_y();
}

fn column_index(x: i32, z: i32) -> Option<usize> {
    ((0..COLUMN_SIDE).contains(&x) && (0..COLUMN_SIDE).contains(&z))
        .then_some((x + z * COLUMN_SIDE) as usize)
}

#[cfg(test)]
mod tests;
