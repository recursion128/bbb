use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use crate::registry_tags::RegistryTagSnapshot;
use bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1;
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_static_collision_shape,
};
use bbb_protocol::block_types::{
    vanilla_block_type, VANILLA_BLOCK_BROWN_MUSHROOM_BLOCK_ID, VANILLA_BLOCK_COARSE_DIRT_ID,
    VANILLA_BLOCK_DIRT_ID, VANILLA_BLOCK_GRASS_BLOCK_ID, VANILLA_BLOCK_GRAVEL_ID,
    VANILLA_BLOCK_MYCELIUM_ID, VANILLA_BLOCK_PODZOL_ID, VANILLA_BLOCK_RED_MUSHROOM_BLOCK_ID,
    VANILLA_BLOCK_RED_SAND_ID, VANILLA_BLOCK_SAND_ID, VANILLA_BLOCK_SNOW_BLOCK_ID,
    VANILLA_BLOCK_STONE_ID, VANILLA_BLOCK_TERRACOTTA_ID,
};
use bbb_protocol::PersistentRegistryTagMembers;
use bbb_server_persistence::Java26BlendingData;
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::save::SaveResult;
use bbb_server_worldgen::noise::density::NoiseBlender;
use bbb_server_worldgen::noise::structure_starts::nbt::{pack_chunk, unpack_chunk};
use bbb_server_worldgen::noise::NormalNoise;

use super::{invalid_carvers_region, ChunkStore};

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BlendingBorderTickWitness {
    pub(crate) old_min_y: i32,
    pub(crate) old_max_y: i32,
    pub(crate) neighbor_old: [bool; 4],
}

#[derive(Clone, Debug)]
pub(crate) struct BlendingResolver {
    by_chunk: BTreeMap<i64, Arc<BlendingColumn>>,
    height_columns: Vec<Arc<BlendingColumn>>,
    density_columns: Vec<Arc<BlendingColumn>>,
}

#[derive(Clone, Debug)]
struct BlendingColumn {
    min_cell_x: i32,
    min_cell_z: i32,
    min_density_cell_y: i32,
    density_cell_count: i32,
    min_quart_y: i32,
    max_quart_y_exclusive: i32,
    heights: [f64; 16],
    densities: Vec<Option<Vec<f64>>>,
    biomes: Vec<Option<Vec<i32>>>,
}

impl BlendingResolver {
    pub(crate) fn empty() -> Self {
        Self {
            by_chunk: BTreeMap::new(),
            height_columns: Vec::new(),
            density_columns: Vec::new(),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.by_chunk.is_empty()
            && self.height_columns.is_empty()
            && self.density_columns.is_empty()
    }

    pub(crate) fn blend(&self, quart_x: i32, quart_y: i32, quart_z: i32) -> Option<i32> {
        let mut closest_distance = f64::INFINITY;
        let mut closest_biome = None;
        for data in &self.height_columns {
            if quart_y < data.min_quart_y || quart_y >= data.max_quart_y_exclusive {
                continue;
            }
            let quart_index = usize::try_from(quart_y - data.min_quart_y)
                .expect("validated old-generation quart index is non-negative");
            for (index, column) in data.biomes.iter().enumerate() {
                let Some(column) = column else {
                    continue;
                };
                let test_x = data.min_cell_x + blending_column_x(index);
                let test_z = data.min_cell_z + blending_column_z(index);
                let dx = quart_x.wrapping_sub(test_x);
                let dz = quart_z.wrapping_sub(test_z);
                let dx = f64::from(dx);
                let dz = f64::from(dz);
                let distance = (dx * dx + dz * dz).sqrt();
                if distance <= 27.0 && distance < closest_distance {
                    closest_distance = distance;
                    closest_biome = Some(column[quart_index]);
                }
            }
        }
        let closest_biome = closest_biome?;
        // `Blender.blendBiome` uses the fixed seed-42 DEFAULT_SHIFT noise
        // before deciding whether the old biome wins (`Blender.java:232-263`).
        let shift =
            blending_shift_noise().get_value(f64::from(quart_x), 0.0, f64::from(quart_z)) * 12.0;
        let alpha = ((closest_distance + shift) / 28.0).clamp(0.0, 1.0);
        (alpha <= 0.5).then_some(closest_biome)
    }

    fn direct_value(
        &self,
        cell_x: i32,
        cell_y: i32,
        cell_z: i32,
        read: impl Fn(&BlendingColumn, i32, i32, i32) -> f64,
    ) -> f64 {
        let chunk_x = cell_x >> 2;
        let chunk_z = cell_z >> 2;
        let min_x = cell_x & 3 == 0;
        let min_z = cell_z & 3 == 0;
        let mut value = self.read_chunk_value(chunk_x, chunk_z, cell_x, cell_y, cell_z, &read);
        if value == f64::MAX && min_x && min_z {
            value = self.read_chunk_value(
                chunk_x.wrapping_sub(1),
                chunk_z.wrapping_sub(1),
                cell_x,
                cell_y,
                cell_z,
                &read,
            );
        }
        if value == f64::MAX && min_x {
            value = self.read_chunk_value(
                chunk_x.wrapping_sub(1),
                chunk_z,
                cell_x,
                cell_y,
                cell_z,
                &read,
            );
        }
        if value == f64::MAX && min_z {
            value = self.read_chunk_value(
                chunk_x,
                chunk_z.wrapping_sub(1),
                cell_x,
                cell_y,
                cell_z,
                &read,
            );
        }
        value
    }

    fn read_chunk_value(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        cell_x: i32,
        cell_y: i32,
        cell_z: i32,
        read: &impl Fn(&BlendingColumn, i32, i32, i32) -> f64,
    ) -> f64 {
        self.by_chunk
            .get(&pack_chunk(chunk_x, chunk_z))
            .map_or(f64::MAX, |column| {
                read(
                    column,
                    cell_x.wrapping_sub(chunk_x.wrapping_mul(4)),
                    cell_y,
                    cell_z.wrapping_sub(chunk_z.wrapping_mul(4)),
                )
            })
    }

    fn blend_offset_and_factor_value(&self, block_x: i32, block_z: i32) -> (f64, f64) {
        // Direct lookup precedes fourth-power weighting
        // (`Blender.java:107-158`).
        let cell_x = block_x >> 2;
        let cell_z = block_z >> 2;
        let fixed = self.direct_value(cell_x, 0, cell_z, |column, x, _, z| column.height(x, z));
        if fixed != f64::MAX {
            return (0.0, height_to_offset(fixed));
        }

        let mut total_weight = 0.0;
        let mut weighted_heights = 0.0;
        let mut closest = f64::INFINITY;
        for column in &self.height_columns {
            for (index, height) in column.heights.iter().copied().enumerate() {
                if height == f64::MAX {
                    continue;
                }
                let dx = cell_x.wrapping_sub(column.min_cell_x + blending_column_x(index));
                let dz = cell_z.wrapping_sub(column.min_cell_z + blending_column_z(index));
                let dx = f64::from(dx);
                let dz = f64::from(dz);
                let distance = (dx * dx + dz * dz).sqrt();
                if distance > 27.0 {
                    continue;
                }
                if distance < closest {
                    closest = distance;
                }
                let weight = 1.0 / (distance * distance * distance * distance);
                weighted_heights += height * weight;
                total_weight += weight;
            }
        }
        if closest == f64::INFINITY {
            return (1.0, 0.0);
        }
        let average = weighted_heights / total_weight;
        let alpha = (closest / 28.0).clamp(0.0, 1.0);
        let alpha = 3.0 * alpha * alpha - 2.0 * alpha * alpha * alpha;
        (alpha, height_to_offset(average))
    }

    fn blend_density_value(&self, block_x: i32, block_y: i32, block_z: i32, value: f64) -> f64 {
        // Density uses the separate near-chunk map and doubled Y distance
        // (`Blender.java:168-210`).
        let cell_x = block_x >> 2;
        let cell_y = block_y / 8;
        let cell_z = block_z >> 2;
        let fixed = self.direct_value(cell_x, cell_y, cell_z, |column, x, y, z| {
            column.density(x, y, z)
        });
        if fixed != f64::MAX {
            return fixed;
        }

        let mut total_weight = 0.0;
        let mut weighted_densities = 0.0;
        let mut closest = f64::INFINITY;
        for column in &self.density_columns {
            let from = (cell_y - 1 - column.min_density_cell_y).max(0);
            let to = (cell_y + 1 - column.min_density_cell_y).min(column.density_cell_count());
            for (index, density_column) in column.densities.iter().enumerate() {
                let Some(density_column) = density_column else {
                    continue;
                };
                let test_x = column.min_cell_x + blending_column_x(index);
                let test_z = column.min_cell_z + blending_column_z(index);
                for density_index in from..to {
                    let test_y = density_index + column.min_density_cell_y;
                    let dx = cell_x.wrapping_sub(test_x);
                    let dy = cell_y.wrapping_sub(test_y).wrapping_mul(2);
                    let dz = cell_z.wrapping_sub(test_z);
                    let dx = f64::from(dx);
                    let dy = f64::from(dy);
                    let dz = f64::from(dz);
                    let distance = (dx * dx + dy * dy + dz * dz).sqrt();
                    if distance > 2.0 {
                        continue;
                    }
                    if distance < closest {
                        closest = distance;
                    }
                    let weight = 1.0 / (distance * distance * distance * distance);
                    weighted_densities += density_column[density_index as usize] * 0.1 * weight;
                    total_weight += weight;
                }
            }
        }
        if closest == f64::INFINITY {
            return value;
        }
        let average = weighted_densities / total_weight;
        let alpha = (closest / 3.0).clamp(0.0, 1.0);
        average + alpha * (value - average)
    }
}

impl NoiseBlender for BlendingResolver {
    fn blend_offset_and_factor(&self, block_x: i32, block_z: i32) -> (f64, f64) {
        self.blend_offset_and_factor_value(block_x, block_z)
    }

    fn blend_density(&self, block_x: i32, block_y: i32, block_z: i32, value: f64) -> f64 {
        self.blend_density_value(block_x, block_y, block_z, value)
    }
}

impl BlendingColumn {
    fn height(&self, cell_x: i32, cell_z: i32) -> f64 {
        blending_index(cell_x, cell_z).map_or(f64::MAX, |index| self.heights[index])
    }

    fn density(&self, cell_x: i32, cell_y: i32, cell_z: i32) -> f64 {
        if cell_y == self.min_density_cell_y - 1 {
            return 0.1;
        }
        let Some(index) = blending_index(cell_x, cell_z) else {
            return f64::MAX;
        };
        let Some(column) = &self.densities[index] else {
            return f64::MAX;
        };
        let y_index = cell_y - self.min_density_cell_y;
        usize::try_from(y_index)
            .ok()
            .and_then(|index| column.get(index))
            .map_or(f64::MAX, |density| density * 0.1)
    }

    fn density_cell_count(&self) -> i32 {
        self.density_cell_count
    }
}

fn blending_index(cell_x: i32, cell_z: i32) -> Option<usize> {
    if cell_x == 4 || cell_z == 4 {
        (cell_x >= 0 && cell_z >= 0 && cell_x <= 4 && cell_z <= 4)
            .then(|| outside_index(cell_x as usize, cell_z as usize))
    } else if cell_x == 0 || cell_z == 0 {
        (cell_x >= 0 && cell_z >= 0 && cell_x < 4 && cell_z < 4)
            .then(|| inside_index(cell_x as usize, cell_z as usize))
    } else {
        None
    }
}

fn height_to_offset(height: f64) -> f64 {
    let target_y = height + 0.5;
    let target_y_mod = ((target_y % 8.0) + 8.0) % 8.0;
    (32.0 * (target_y - 128.0) - 3.0 * (target_y - 120.0) * target_y_mod
        + 3.0 * target_y_mod * target_y_mod)
        / (128.0 * (32.0 - 3.0 * target_y_mod))
}

fn old_surface_height(
    column: &bbb_server_types::chunk_data::ChunkColumn,
    block_x: i32,
    block_z: i32,
    old_min_y: i32,
    old_max_y: i32,
    target: (i32, i32),
) -> SaveResult<f64> {
    // Lazy old columns scan the fixed surface-block list before read1/read7
    // density construction (`BlendingData.java:179-245`).
    let mut y = old_max_y;
    while y > old_min_y {
        let state = old_block_state(column, block_x, y, block_z, target)?;
        if is_surface_block(state, target)? {
            return Ok(f64::from(y));
        }
        y -= 1;
    }
    Ok(f64::from(old_min_y))
}

fn old_density_column(
    column: &bbb_server_types::chunk_data::ChunkColumn,
    [block_x, block_z]: [i32; 2],
    [old_min_y, old_max_y]: [i32; 2],
    density_cell_count: i32,
    surface_height: i32,
    registry_tags: Option<&RegistryTagSnapshot>,
    target: (i32, i32),
) -> SaveResult<Vec<f64>> {
    let count = usize::try_from(density_cell_count).map_err(|_| {
        invalid_carvers_region(
            [target.0, target.1],
            "blending density column has a negative cell count",
        )
    })?;
    let mut densities = vec![-1.0; count];
    let mut y = old_max_y.checked_add(1).ok_or_else(|| {
        invalid_carvers_region(
            [target.0, target.1],
            "blending density scan maximum overflows",
        )
    })?;
    let mut last_seven =
        read_old_ground_seven(column, block_x, block_z, &mut y, registry_tags, target)?;
    if count >= 2 {
        for cell_index in (0..=(count - 2)).rev() {
            let one = read_old_ground_one(column, block_x, block_z, &mut y, registry_tags, target)?;
            let current_seven =
                read_old_ground_seven(column, block_x, block_z, &mut y, registry_tags, target)?;
            densities[cell_index] = (last_seven + one + current_seven) / 15.0;
            last_seven = current_seven;
        }
    }
    if y < old_min_y {
        return Err(invalid_carvers_region(
            [target.0, target.1],
            "blending density scan escaped the old-generation area",
        ));
    }

    let min_density_cell_y = old_min_y.div_euclid(8).checked_add(1).ok_or_else(|| {
        invalid_carvers_region(
            [target.0, target.1],
            "blending density minimum overflows cell coordinates",
        )
    })?;
    let surface_index = surface_height
        .div_euclid(8)
        .checked_sub(min_density_cell_y)
        .ok_or_else(|| {
            invalid_carvers_region(
                [target.0, target.1],
                "blending density surface index overflows",
            )
        })?;
    if surface_index >= 0 && surface_index < density_cell_count - 1 {
        let in_cell = (f64::from(surface_height) + 0.5) % 8.0 / 8.0;
        let amplitude = (1.0 - in_cell) / in_cell;
        let java_max = if amplitude.is_nan() || amplitude > 1.0 {
            amplitude
        } else {
            1.0
        };
        let max = java_max * 0.25;
        let index = surface_index as usize;
        densities[index + 1] = -amplitude / max;
        densities[index] = 1.0 / max;
    }
    Ok(densities)
}

fn old_block_state(
    column: &bbb_server_types::chunk_data::ChunkColumn,
    block_x: i32,
    y: i32,
    block_z: i32,
    target: (i32, i32),
) -> SaveResult<i32> {
    column.get_block_state(block_x, y, block_z).ok_or_else(|| {
        invalid_carvers_region(
            [target.0, target.1],
            format!("old-generation blending block ({block_x},{y},{block_z}) is absent"),
        )
    })
}

fn read_old_ground_one(
    column: &bbb_server_types::chunk_data::ChunkColumn,
    block_x: i32,
    block_z: i32,
    y: &mut i32,
    registry_tags: Option<&RegistryTagSnapshot>,
    target: (i32, i32),
) -> SaveResult<f64> {
    *y = (*y).checked_sub(1).ok_or_else(|| {
        invalid_carvers_region(
            [target.0, target.1],
            "blending density scan minimum overflows",
        )
    })?;
    let state = old_block_state(column, block_x, *y, block_z, target)?;
    Ok(if is_old_ground(state, registry_tags, target)? {
        1.0
    } else {
        -1.0
    })
}

fn read_old_ground_seven(
    column: &bbb_server_types::chunk_data::ChunkColumn,
    block_x: i32,
    block_z: i32,
    y: &mut i32,
    registry_tags: Option<&RegistryTagSnapshot>,
    target: (i32, i32),
) -> SaveResult<f64> {
    let mut sum = 0.0;
    for _ in 0..7 {
        sum += read_old_ground_one(column, block_x, block_z, y, registry_tags, target)?;
    }
    Ok(sum)
}

fn is_surface_block(state: i32, target: (i32, i32)) -> SaveResult<bool> {
    let owner = vanilla_block_state_block_type_id(state).ok_or_else(|| {
        invalid_carvers_region(
            [target.0, target.1],
            format!("unknown blending block state {state}"),
        )
    })?;
    Ok(matches!(
        owner,
        VANILLA_BLOCK_PODZOL_ID
            | VANILLA_BLOCK_GRAVEL_ID
            | VANILLA_BLOCK_GRASS_BLOCK_ID
            | VANILLA_BLOCK_STONE_ID
            | VANILLA_BLOCK_COARSE_DIRT_ID
            | VANILLA_BLOCK_SAND_ID
            | VANILLA_BLOCK_RED_SAND_ID
            | VANILLA_BLOCK_MYCELIUM_ID
            | VANILLA_BLOCK_SNOW_BLOCK_ID
            | VANILLA_BLOCK_TERRACOTTA_ID
            | VANILLA_BLOCK_DIRT_ID
    ))
}

fn is_old_ground(
    state: i32,
    registry_tags: Option<&RegistryTagSnapshot>,
    target: (i32, i32),
) -> SaveResult<bool> {
    // `BlendingData.isGround` preserves this order (`BlendingData.java:250-260`).
    if block_state_is_air(state) {
        return Ok(false);
    }
    let owner = vanilla_block_state_block_type_id(state).ok_or_else(|| {
        invalid_carvers_region(
            [target.0, target.1],
            format!("unknown old-generation blending block state {state}"),
        )
    })?;
    if generation_block_type_is_tagged(registry_tags, "minecraft:leaves", owner)
        || generation_block_type_is_tagged(registry_tags, "minecraft:logs", owner)
        || matches!(
            owner,
            VANILLA_BLOCK_BROWN_MUSHROOM_BLOCK_ID | VANILLA_BLOCK_RED_MUSHROOM_BLOCK_ID
        )
    {
        return Ok(false);
    }
    let shape = vanilla_block_state_static_collision_shape(state).ok_or_else(|| {
        invalid_carvers_region(
            [target.0, target.1],
            format!("old-generation blending state {state} has a dynamic collision shape"),
        )
    })?;
    Ok(shape.boxes().len() != 0)
}

pub(crate) fn generation_block_state_is_tagged(
    registry_tags: Option<&RegistryTagSnapshot>,
    tag_id: &str,
    state: i32,
) -> bool {
    vanilla_block_state_block_type_id(state)
        .is_some_and(|owner| generation_block_type_is_tagged(registry_tags, tag_id, owner))
}

fn generation_block_type_is_tagged(
    registry_tags: Option<&RegistryTagSnapshot>,
    tag_id: &str,
    block_type_id: i32,
) -> bool {
    let Some(resource_id) = vanilla_block_type(block_type_id).map(|entry| entry.resource_id) else {
        return false;
    };
    let Some(registry_tags) = registry_tags else {
        // Standalone generation keeps base-Vanilla compatibility; level generation installs a snapshot.
        return matches!(
            vanilla_persistent_registry_tag_members_26_1("minecraft:block", tag_id),
            PersistentRegistryTagMembers::Present(members)
                if members.iter().any(|member| member == resource_id)
        );
    };
    registry_tags.contains_block(tag_id, block_type_id)
}

fn blending_shift_noise() -> &'static NormalNoise {
    static SHIFT: OnceLock<NormalNoise> = OnceLock::new();
    SHIFT.get_or_init(|| {
        let mut random = bbb_server_worldgen::random::WorldgenRandom::named(0, "blending_shift");
        NormalNoise::create(&mut random, -3, &[1.0, 1.0, 1.0, 0.0])
            .expect("NoiseData.DEFAULT_SHIFT is a valid NormalNoise configuration")
    })
}

fn blending_column_x(index: usize) -> i32 {
    if index < 7 {
        (3_i32 - index as i32).max(0)
    } else {
        let offset = index as i32 - 7;
        4 - (4 - offset).max(0)
    }
}

fn blending_column_z(index: usize) -> i32 {
    if index < 7 {
        (index as i32 - 3).max(0)
    } else {
        let offset = index as i32 - 7;
        4 - (offset - 4).max(0)
    }
}

fn inside_index(x: usize, z: usize) -> usize {
    3 - x + z
}

fn outside_index(x: usize, z: usize) -> usize {
    7 + x + 4 - z
}

impl ChunkStore {
    pub(super) fn blending_witness(
        &self,
        chunk: (i32, i32),
    ) -> Option<&Option<Java26BlendingData>> {
        self.chunk(chunk)?.blending.as_ref()
    }

    /// A resident chunk saved with `blending_data`. The resolver below only
    /// reads resident witnesses, so these never unload.
    pub(crate) fn is_old_generation(&self, chunk: (i32, i32)) -> bool {
        self.blending_witness(chunk).is_some_and(Option::is_some)
    }

    pub(crate) fn blending_resolver(&self, target: (i32, i32)) -> SaveResult<BlendingResolver> {
        let registry_tags = self.registry_tags();
        let mut insertion_order = Vec::new();
        let mut density_insertion_order = Vec::new();
        // `Blender.of` scans dx outer/dz inner and keeps the radius-eight
        // circle in its height/biome map (`Blender.java:66-100`).
        for dx in -7_i32..=7 {
            for dz in -7_i32..=7 {
                if dx * dx + dz * dz > 64 {
                    continue;
                }
                let chunk = (
                    target.0.checked_add(dx).ok_or_else(|| {
                        invalid_carvers_region(
                            [target.0, target.1],
                            "blending biome chunk x overflows",
                        )
                    })?,
                    target.1.checked_add(dz).ok_or_else(|| {
                        invalid_carvers_region(
                            [target.0, target.1],
                            "blending biome chunk z overflows",
                        )
                    })?,
                );
                if self.is_old_generation(chunk) {
                    let packed = pack_chunk(chunk.0, chunk.1);
                    insertion_order.push(packed);
                    if (-1..=1).contains(&dx) && (-1..=1).contains(&dz) {
                        density_insertion_order.push(packed);
                    }
                }
            }
        }
        if insertion_order.is_empty() {
            return Ok(BlendingResolver::empty());
        }
        if !self.generation_blending_catalog_complete {
            if self.generation_blending_restore_in_progress {
                // Restore generates saved-column preimages before publishing the
                // complete saved-coordinate catalog; those preimages are not served.
                return Ok(BlendingResolver::empty());
            }
            return Err(invalid_carvers_region(
                [target.0, target.1],
                "old-generation biome blending catalog is incomplete",
            ));
        }

        let mut compiled = BTreeMap::new();
        for packed in insertion_order.iter().copied() {
            let [chunk_x, chunk_z] = unpack_chunk(packed);
            let chunk = (chunk_x, chunk_z);
            let data = self
                .blending_witness(chunk)
                .and_then(Option::as_ref)
                .expect("the old-generation candidate came from the typed witness map");
            let column = self.column_at(chunk).ok_or_else(|| {
                invalid_carvers_region(
                    [target.0, target.1],
                    format!("old-generation blending chunk [{chunk_x},{chunk_z}] is not loaded"),
                )
            })?;
            let state = self.generation(chunk);
            if state.is_none() && self.pre_carvers(chunk).is_none() {
                return Err(invalid_carvers_region(
                    [target.0, target.1],
                    format!(
                        "old-generation blending chunk [{chunk_x},{chunk_z}] has no generation state"
                    ),
                ));
            }
            // Old blending data is visible only at BIOMES or later
            // (`BlendingData.java:101-110`); every noise-store state starts at CARVERS.
            if matches!(
                state,
                Some(bbb_server_types::chunk_data::ChunkGenerationState::Full { .. })
            ) {
                return Err(invalid_carvers_region(
                    [target.0, target.1],
                    format!(
                        "old-generation blending chunk [{chunk_x},{chunk_z}] has a non-noise generation state"
                    ),
                ));
            }
            let min_quart_y = data.min_section().checked_mul(4).ok_or_else(|| {
                invalid_carvers_region(
                    [target.0, target.1],
                    "blending-data minimum section overflows quart coordinates",
                )
            })?;
            let max_quart_y_exclusive = data.max_section().checked_mul(4).ok_or_else(|| {
                invalid_carvers_region(
                    [target.0, target.1],
                    "blending-data maximum section overflows quart coordinates",
                )
            })?;
            if min_quart_y >= max_quart_y_exclusive {
                return Err(invalid_carvers_region(
                    [target.0, target.1],
                    "blending-data biome area has non-positive height",
                ));
            }
            let old_min_y = data.min_section().checked_mul(16).ok_or_else(|| {
                invalid_carvers_region(
                    [target.0, target.1],
                    "blending-data minimum section overflows block coordinates",
                )
            })?;
            let old_max_y = data
                .max_section()
                .checked_mul(16)
                .and_then(|exclusive| exclusive.checked_sub(1))
                .ok_or_else(|| {
                    invalid_carvers_region(
                        [target.0, target.1],
                        "blending-data maximum section overflows block coordinates",
                    )
                })?;
            let density_cell_count = data
                .max_section()
                .checked_sub(data.min_section())
                .and_then(|sections| sections.checked_mul(2))
                .ok_or_else(|| {
                    invalid_carvers_region(
                        [target.0, target.1],
                        "blending-data density cell count overflows",
                    )
                })?;
            let min_density_cell_y = data
                .min_section()
                .checked_mul(2)
                .and_then(|cell| cell.checked_add(1))
                .ok_or_else(|| {
                    invalid_carvers_region(
                        [target.0, target.1],
                        "blending-data density minimum overflows cell coordinates",
                    )
                })?;
            let column_min_quart = column.min_y().div_euclid(4);
            let column_max_quart = i32::try_from(column.section_count())
                .ok()
                .and_then(|sections| sections.checked_mul(4))
                .and_then(|height| column_min_quart.checked_add(height))
                .ok_or_else(|| {
                    invalid_carvers_region(
                        [target.0, target.1],
                        "old-generation column quart height overflows",
                    )
                })?;
            if min_quart_y < column_min_quart || max_quart_y_exclusive > column_max_quart {
                return Err(invalid_carvers_region(
                    [target.0, target.1],
                    format!(
                        "old-generation blending area for [{chunk_x},{chunk_z}] escapes its loaded column"
                    ),
                ));
            }
            let column_max_y = i32::try_from(column.section_count())
                .ok()
                .and_then(|sections| sections.checked_mul(16))
                .and_then(|height| column.min_y().checked_add(height))
                .and_then(|exclusive| exclusive.checked_sub(1))
                .ok_or_else(|| {
                    invalid_carvers_region(
                        [target.0, target.1],
                        "old-generation column block height overflows",
                    )
                })?;
            if old_min_y < column.min_y() || old_max_y > column_max_y {
                return Err(invalid_carvers_region(
                    [target.0, target.1],
                    format!(
                        "old-generation blending area for [{chunk_x},{chunk_z}] escapes its loaded column"
                    ),
                ));
            }
            let mut new_sides = [false; 8];
            for (index, (dx, dz)) in [
                (0, -1),
                (1, -1),
                (1, 0),
                (1, 1),
                (0, 1),
                (-1, 1),
                (-1, 0),
                (-1, -1),
            ]
            .iter()
            .copied()
            .enumerate()
            {
                let neighbor = (
                    chunk_x.checked_add(dx).ok_or_else(|| {
                        invalid_carvers_region(
                            [target.0, target.1],
                            "blending age neighbor x overflows",
                        )
                    })?,
                    chunk_z.checked_add(dz).ok_or_else(|| {
                        invalid_carvers_region(
                            [target.0, target.1],
                            "blending age neighbor z overflows",
                        )
                    })?,
                );
                new_sides[index] = !self.blending_witness(neighbor).is_some_and(Option::is_some);
            }
            let mut heights = data.heights().unwrap_or([f64::MAX; 16]);
            let mut densities = vec![None; 16];
            let mut biomes = vec![None; 16];
            let mut add_column = |index: usize, block_x: i32, block_z: i32| -> SaveResult<()> {
                if heights[index] == f64::MAX {
                    heights[index] =
                        old_surface_height(column, block_x, block_z, old_min_y, old_max_y, target)?;
                }
                densities[index] = Some(old_density_column(
                    column,
                    [block_x, block_z],
                    [old_min_y, old_max_y],
                    density_cell_count,
                    heights[index].floor() as i32,
                    registry_tags,
                    target,
                )?);
                let values = (min_quart_y..max_quart_y_exclusive)
                    .map(|quart_y| {
                        column
                            .get_noise_biome(block_x.div_euclid(4), quart_y, block_z.div_euclid(4))
                            .ok_or_else(|| {
                                invalid_carvers_region(
                                    [target.0, target.1],
                                    format!(
                                        "old-generation biome column [{chunk_x},{chunk_z}] ({block_x},{quart_y},{block_z}) is absent"
                                    ),
                                )
                            })
                    })
                    .collect::<SaveResult<Vec<_>>>()?;
                biomes[index] = Some(values);
                Ok(())
            };
            // `BlendingData.calculateData` selects and orders the sixteen
            // boundary columns from new-age sides (`BlendingData.java:126-175`).
            if new_sides[0] || new_sides[6] || new_sides[7] {
                add_column(inside_index(0, 0), 0, 0)?;
            }
            if new_sides[0] {
                for index in 1..4 {
                    add_column(inside_index(index, 0), (index * 4) as i32, 0)?;
                }
            }
            if new_sides[6] {
                for index in 1..4 {
                    add_column(inside_index(0, index), 0, (index * 4) as i32)?;
                }
            }
            if new_sides[2] {
                for index in 1..4 {
                    add_column(outside_index(4, index), 15, (index * 4) as i32)?;
                }
            }
            if new_sides[4] {
                for index in 0..4 {
                    add_column(outside_index(index, 4), (index * 4) as i32, 15)?;
                }
            }
            if new_sides[2] && new_sides[1] {
                add_column(outside_index(4, 0), 15, 0)?;
            }
            if new_sides[2] && new_sides[4] && new_sides[3] {
                add_column(outside_index(4, 4), 15, 15)?;
            }
            let min_cell_x = chunk_x.checked_mul(4).ok_or_else(|| {
                invalid_carvers_region([target.0, target.1], "blending cell x overflows")
            })?;
            let min_cell_z = chunk_z.checked_mul(4).ok_or_else(|| {
                invalid_carvers_region([target.0, target.1], "blending cell z overflows")
            })?;
            min_cell_x.checked_add(4).ok_or_else(|| {
                invalid_carvers_region([target.0, target.1], "blending cell x extent overflows")
            })?;
            min_cell_z.checked_add(4).ok_or_else(|| {
                invalid_carvers_region([target.0, target.1], "blending cell z extent overflows")
            })?;
            compiled.insert(
                packed,
                Arc::new(BlendingColumn {
                    min_cell_x,
                    min_cell_z,
                    min_density_cell_y,
                    density_cell_count,
                    min_quart_y,
                    max_quart_y_exclusive,
                    heights,
                    densities,
                    biomes,
                }),
            );
        }
        let column = |packed: &i64| Arc::clone(&compiled[packed]);
        let height_columns = insertion_order.iter().map(column).collect();
        let density_columns = density_insertion_order.iter().map(column).collect();
        Ok(BlendingResolver {
            by_chunk: compiled,
            height_columns,
            density_columns,
        })
    }

    pub(super) fn blending_border_tick_witness(
        &self,
        target: (i32, i32),
    ) -> SaveResult<Option<BlendingBorderTickWitness>> {
        let target_data = self.require_generation_blending_witness(target, target)?;
        let Some(target_data) = target_data else {
            return Ok(None);
        };
        let old_min_y = target_data.min_section().checked_mul(16).ok_or_else(|| {
            invalid_carvers_region(
                [target.0, target.1],
                "blending-data minimum section overflows block coordinates",
            )
        })?;
        // The packed maximum section is exclusive; the old area's getMaxY is
        // one below it (`BlendingData.java:69-77`; `LevelHeightAccessor.java:6-13`).
        let old_max_y = target_data
            .max_section()
            .checked_mul(16)
            .and_then(|exclusive_max| exclusive_max.checked_sub(1))
            .ok_or_else(|| {
                invalid_carvers_region(
                    [target.0, target.1],
                    "blending-data maximum section overflows block coordinates",
                )
            })?;
        if old_max_y < old_min_y {
            return Err(invalid_carvers_region(
                [target.0, target.1],
                "blending-data old-generation area has non-positive height",
            ));
        }
        let column = self
            .column_at(target)
            .expect("the target blending witness requires a loaded column");
        let build_max_y = i32::try_from(column.section_count())
            .ok()
            .and_then(|sections| sections.checked_mul(16))
            .and_then(|height| column.min_y().checked_add(height))
            .and_then(|exclusive_max| exclusive_max.checked_sub(1))
            .ok_or_else(|| {
                invalid_carvers_region(
                    [target.0, target.1],
                    "target chunk build height overflows block coordinates",
                )
            })?;
        if old_min_y < column.min_y() || old_max_y > build_max_y {
            return Err(invalid_carvers_region(
                [target.0, target.1],
                "blending-data old-generation area escapes the target build height",
            ));
        }
        old_min_y.checked_sub(1).ok_or_else(|| {
            invalid_carvers_region(
                [target.0, target.1],
                "blending-data lower border overflows block coordinates",
            )
        })?;
        old_max_y.checked_add(1).ok_or_else(|| {
            invalid_carvers_region(
                [target.0, target.1],
                "blending-data upper border overflows block coordinates",
            )
        })?;
        for (axis, coordinate) in [("x", target.0), ("z", target.1)].iter().copied() {
            coordinate
                .checked_mul(16)
                .and_then(|origin| origin.checked_add(15))
                .ok_or_else(|| {
                    invalid_carvers_region(
                        [target.0, target.1],
                        format!("blending target block {axis} range overflows"),
                    )
                })?;
        }
        column.min_y().checked_sub(1).ok_or_else(|| {
            invalid_carvers_region(
                [target.0, target.1],
                "target build minimum cannot represent an empty heightmap",
            )
        })?;

        // Direction.Plane.HORIZONTAL is NORTH, EAST, SOUTH, WEST
        // (`Direction.java:576-578`).
        let offsets = [(0, -1), (1, 0), (0, 1), (-1, 0)];
        let mut neighbor_old = [false; 4];
        for (index, (dx, dz)) in offsets.iter().copied().enumerate() {
            let neighbor = (
                target.0.checked_add(dx).ok_or_else(|| {
                    invalid_carvers_region(
                        [target.0, target.1],
                        "blending neighbor chunk x overflows",
                    )
                })?,
                target.1.checked_add(dz).ok_or_else(|| {
                    invalid_carvers_region(
                        [target.0, target.1],
                        "blending neighbor chunk z overflows",
                    )
                })?,
            );
            neighbor_old[index] = self
                .require_generation_blending_witness(neighbor, target)?
                .is_some();
        }
        Ok(Some(BlendingBorderTickWitness {
            old_min_y,
            old_max_y,
            neighbor_old,
        }))
    }

    fn require_generation_blending_witness(
        &self,
        chunk: (i32, i32),
        target: (i32, i32),
    ) -> SaveResult<Option<&Java26BlendingData>> {
        if self.generation(chunk).is_none() {
            return Err(invalid_carvers_region(
                [target.0, target.1],
                format!(
                    "blending border chunk [{},{}] has incomplete loaded generation state",
                    chunk.0, chunk.1
                ),
            ));
        }
        self.blending_witness(chunk)
            .map(Option::as_ref)
            .ok_or_else(|| {
                invalid_carvers_region(
                    [target.0, target.1],
                    format!(
                        "blending border chunk [{},{}] has no old-generation witness",
                        chunk.0, chunk.1
                    ),
                )
            })
    }
}
