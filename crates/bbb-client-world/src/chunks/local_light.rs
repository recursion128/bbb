//! Batch-local client relighting over canonical packet light arrays.

use bbb_protocol::chunk_light::{
    light_engine::{
        has_different_light_properties_with_registry, light_facts_with_registry,
        lowest_sky_source_y_with_registry, LightEngine,
    },
    light_storage::ChunkLightStorage,
    LightResult,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{section_block_index, vanilla_air_block_state_id, BlockPos, ChunkPos, WorldStore};

use super::light::{light_layer, replace_light_layer_bytes, sample_terrain_light};

type Pos = (i32, i32, i32);

fn neighborhood((x, z): (i32, i32)) -> impl Iterator<Item = (i32, i32)> {
    (-1..=1).flat_map(move |dz| (-1..=1).map(move |dx| (x + dx, z + dz)))
}

fn affected_chunks(x: i32, z: i32) -> impl Iterator<Item = (i32, i32)> {
    let min_cx = (x - 15).div_euclid(16);
    let max_cx = (x + 15).div_euclid(16);
    let min_cz = (z - 15).div_euclid(16);
    let max_cz = (z + 15).div_euclid(16);
    (min_cz..=max_cz).flat_map(move |cz| (min_cx..=max_cx).map(move |cx| (cx, cz)))
}

#[derive(Debug, Default)]
pub(crate) struct RuntimeLocalLightUpdates(FxHashMap<Pos, bool>);

impl Clone for RuntimeLocalLightUpdates {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl RuntimeLocalLightUpdates {
    pub(super) fn clear(&mut self) {
        self.0.clear();
    }
}

impl WorldStore {
    pub(super) fn queue_local_light_change(&mut self, pos: BlockPos, old: i32, new: i32) {
        let registry = &self.content_registry.0;
        if has_different_light_properties_with_registry(old, new, registry) {
            let sky = match (
                light_facts_with_registry(old, registry),
                light_facts_with_registry(new, registry),
            ) {
                (Some(old_facts), Some(new_facts)) => {
                    old_facts.sky_light_properties_differ(new_facts)
                }
                _ => true,
            };
            self.local_light_updates
                .0
                .entry((pos.x, pos.y, pos.z))
                .and_modify(|pending_sky| *pending_sky |= sky)
                .or_insert(sky);
        }
    }

    pub(super) fn discard_local_light_updates_for_chunk(&mut self, pos: ChunkPos) {
        self.local_light_updates
            .0
            .retain(|&(x, _, z), _| (x.div_euclid(16), z.div_euclid(16)) != (pos.x, pos.z));
    }

    /// Propagates this simulation step's block edits before its world is rendered.
    pub fn run_light_updates(&mut self) -> LightResult<()> {
        if self.local_light_updates.0.is_empty() {
            return Ok(());
        }
        let pending = std::mem::take(&mut self.local_light_updates.0);
        let mut changes = FxHashMap::with_capacity_and_hasher(pending.len(), Default::default());
        for (pos, sky) in pending {
            let awaiting_initial_light = neighborhood((pos.0.div_euclid(16), pos.2.div_euclid(16)))
                .any(|(x, z)| self.terrain_chunk_initial_light_pending(ChunkPos { x, z }));
            if awaiting_initial_light {
                self.local_light_updates.0.insert(pos, sky);
            } else {
                changes.insert(pos, sky);
            }
        }
        if changes.is_empty() {
            return Ok(());
        }

        let has_sky = self.level_info().is_some_and(|level| level.has_sky_light());
        let min_section = self.dimension.min_section_y();
        let light_section_count = self.dimension.section_count() + 2;
        let min_light_sy = min_section - 1;
        let max_light_sy = min_light_sy + light_section_count as i32 - 1;
        let min_light_y = min_light_sy * 16;
        let max_light_y = max_light_sy * 16 + 15;
        let air = vanilla_air_block_state_id();

        let mut block_sections = FxHashMap::<(i32, i32), FxHashSet<i32>>::default();
        let mut sky_window = FxHashSet::<(i32, i32)>::default();
        for (&(x, y, z), &sky) in &changes {
            let section_y = y.div_euclid(16);
            for (cx, cz) in affected_chunks(x, z) {
                if self.probe_chunk(ChunkPos { x: cx, z: cz }).is_some() {
                    // Light attenuates within 15 blocks, including its boundary samples.
                    block_sections.entry((cx, cz)).or_default().extend(
                        (section_y - 1).max(min_light_sy)..=(section_y + 1).min(max_light_sy),
                    );
                    if has_sky && sky {
                        sky_window.insert((cx, cz));
                    }
                }
            }
        }

        let mut storage = ChunkLightStorage::default();
        let mut sky_section_range = None;
        if !sky_window.is_empty() {
            // `LightEngine::check_sky` (`add_sources_above`) only queries `lowest_sky_source`
            // at each sky-edited `(x, z)` column and its four cardinal neighbor columns.
            let mut sky_columns_by_chunk = FxHashMap::<(i32, i32), Vec<(u8, u8)>>::default();
            for (&(px, _, pz), &sky) in &changes {
                if !sky {
                    continue;
                }
                for (qx, qz) in [
                    (px, pz),
                    (px - 1, pz),
                    (px + 1, pz),
                    (px, pz - 1),
                    (px, pz + 1),
                ] {
                    let chunk = (qx.div_euclid(16), qz.div_euclid(16));
                    if sky_window.contains(&chunk) {
                        sky_columns_by_chunk
                            .entry(chunk)
                            .or_default()
                            .push((qx.rem_euclid(16) as u8, qz.rem_euclid(16) as u8));
                    }
                }
            }

            let mut stack_empty = [false; 64];
            let mut heap_empty = Vec::new();
            for ((cx, cz), mut cols) in sky_columns_by_chunk {
                cols.sort_unstable();
                cols.dedup();
                let column = self.probe_chunk(ChunkPos { x: cx, z: cz }).unwrap();
                let empty: &[bool] = if column.sections.len() <= stack_empty.len() {
                    for (slot, section) in stack_empty.iter_mut().zip(&column.sections) {
                        *slot = section.non_empty_block_count == 0 && section.fluid_count == 0;
                    }
                    &stack_empty[..column.sections.len()]
                } else {
                    heap_empty.clear();
                    heap_empty.extend(
                        column
                            .sections
                            .iter()
                            .map(|s| s.non_empty_block_count == 0 && s.fluid_count == 0),
                    );
                    &heap_empty
                };
                let mut sources = [i32::MIN; 256];
                for (local_x, local_z) in cols {
                    sources[usize::from(local_x) + usize::from(local_z) * 16] =
                        lowest_sky_source_y_with_registry(
                            min_section,
                            empty,
                            air,
                            &self.content_registry.0,
                            |section_index, local_y| {
                                column.sections[section_index]
                                    .block_states
                                    .value_at(section_block_index(local_x, local_y as u8, local_z))
                                    .map_or(air, |value| value.global_id)
                            },
                        )?;
                }
                storage.set_column_sources((cx, cz), &sources);
            }

            let mut min_sy = max_light_sy;
            let mut max_sy = min_light_sy;
            for (&(px, py, pz), &sky) in &changes {
                if !sky {
                    continue;
                }
                let cx = px.div_euclid(16);
                let cz = pz.div_euclid(16);
                let Some(column) = self.probe_chunk(ChunkPos { x: cx, z: cz }) else {
                    continue;
                };
                let local_x = px.rem_euclid(16) as usize;
                let local_z = pz.rem_euclid(16) as usize;
                let lowest = storage.lowest_sky_source(px, pz).unwrap_or(i32::MAX);

                let bottom_y = if lowest == i32::MIN {
                    min_light_y
                } else {
                    let mut bottom = py.min(lowest);
                    let mut scan_y = lowest.saturating_sub(1).min(max_light_y);
                    while scan_y >= min_light_y
                        && sample_terrain_light(
                            &column.light,
                            self.dimension,
                            local_x,
                            scan_y,
                            local_z,
                        )
                        .sky == 15
                    {
                        bottom = bottom.min(scan_y);
                        scan_y -= 1;
                    }
                    bottom
                };

                let mut top_y = py;
                if lowest < i32::MAX {
                    top_y = top_y.max(lowest.min(max_light_y));
                    let mut scan_y = py.max(lowest.max(min_light_y));
                    while scan_y <= max_light_y
                        && sample_terrain_light(
                            &column.light,
                            self.dimension,
                            local_x,
                            scan_y,
                            local_z,
                        )
                        .sky != 15
                    {
                        top_y = top_y.max(scan_y);
                        scan_y += 1;
                    }
                    // Include `scan_y`'s section so `add_sources_above` observes the stored `15` stop.
                    top_y = top_y.max(scan_y.min(max_light_y));
                }

                // Include `(lowest - 1)`'s section so `remove_sources_below` observes its stop,
                // plus 16 blocks vertical attenuation margin in both directions.
                let stop_below = if lowest == i32::MIN || lowest == i32::MAX {
                    bottom_y
                } else {
                    bottom_y.min(lowest.saturating_sub(1).max(min_light_y))
                };
                min_sy = min_sy.min((stop_below - 16).div_euclid(16).max(min_light_sy));
                max_sy = max_sy.max((top_y + 16).div_euclid(16).min(max_light_sy));
            }
            if min_sy <= max_sy {
                sky_section_range = Some(min_sy..=max_sy);
            }
        }

        for (&(x, z), sections) in &block_sections {
            if let Some(range) = sky_section_range
                .as_ref()
                .filter(|_| sky_window.contains(&(x, z)))
            {
                for section_y in range.clone() {
                    storage.register_non_empty_section((x, section_y, z));
                }
            }
            for &section_y in sections {
                storage.register_non_empty_section((x, section_y, z));
            }
        }
        for (&(x, z), sections) in &block_sections {
            let column = self.probe_chunk(ChunkPos { x, z }).unwrap();
            for &section_y in sections {
                let index = (section_y - min_light_sy) as usize;
                storage.queue_layer_data(
                    (x, section_y, z),
                    false,
                    light_layer(&column.light, index, false),
                );
            }
            if let Some(range) = sky_section_range
                .as_ref()
                .filter(|_| sky_window.contains(&(x, z)))
            {
                for section_y in range.clone() {
                    let index = (section_y - min_light_sy) as usize;
                    storage.queue_layer_data(
                        (x, section_y, z),
                        true,
                        light_layer(&column.light, index, true),
                    );
                }
            }
            storage.enable_column((x, z));
        }
        storage.take_dirty();

        {
            let mut engine = LightEngine::new_with_registry(
                &mut storage,
                |(x, y, z)| self.block_state_id_at(BlockPos { x, y, z }),
                |chunk| block_sections.contains_key(&chunk),
                air,
                &self.content_registry.0,
            )?;
            for &position in changes.keys() {
                engine.check_block(position)?;
            }
            engine.run()?;
        }
        if !sky_window.is_empty() {
            let mut engine = LightEngine::new_with_registry(
                &mut storage,
                |(x, y, z)| self.block_state_id_at(BlockPos { x, y, z }),
                |chunk| sky_window.contains(&chunk),
                air,
                &self.content_registry.0,
            )?;
            engine.set_sky(true);
            for (&position, &sky) in &changes {
                if sky {
                    engine.check_sky(position);
                }
            }
            engine.run()?;
        }

        let mut changed_sections = Vec::new();
        for ((x, z), (sky, block)) in storage.take_dirty() {
            if !block_sections.contains_key(&(x, z)) {
                continue;
            }
            let pos = ChunkPos { x, z };
            let light = self.chunks.light_mut(pos).unwrap();
            changed_sections.clear();
            for (is_sky, sections) in [(false, block), (true, sky)] {
                if is_sky && !sky_window.contains(&(x, z)) {
                    continue;
                }
                for section_y in sections {
                    let index = section_y - min_light_sy;
                    if !(0..light_section_count as i32).contains(&index) {
                        continue;
                    }
                    if replace_light_layer_bytes(
                        light,
                        index as usize,
                        is_sky,
                        storage.resolved_layer_bytes((x, section_y, z), is_sky),
                    ) {
                        changed_sections.push(index as usize);
                    }
                }
            }
            if !changed_sections.is_empty() {
                changed_sections.sort_unstable();
                changed_sections.dedup();
                self.mark_terrain_light_sections_changed(pos, &changed_sections);
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChunkColumn, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension,
    };
    use bbb_protocol::packets::{BlockChangedAck, BlockUpdate, SectionBlocksUpdate};
    use std::collections::BTreeMap;
    use std::sync::Arc;

    fn state(name: &str) -> i32 {
        crate::vanilla_block_state_id(name, &BTreeMap::new()).unwrap()
    }

    fn chunk(pos: ChunkPos, sections: usize) -> ChunkColumn {
        let palette = |domain, entry_count| PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![0],
            packed_data: Vec::new(),
            entry_count,
        };
        ChunkColumn {
            pos,
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: (0..sections)
                .map(|_| {
                    Arc::new(ChunkSection {
                        non_empty_block_count: 0,
                        fluid_count: 0,
                        block_states: palette(PaletteDomain::BlockStates, 4096),
                        biomes: palette(PaletteDomain::Biomes, 64),
                    })
                })
                .collect(),
            block_entities: Arc::default(),
            light: {
                let mut light = LightData::default();
                for section_index in 0..(sections + 2) {
                    light.set_sky_empty(section_index);
                    light.set_block_empty(section_index);
                }
                light
            },
        }
    }

    #[test]
    fn block_updates_relight_multiple_sources_across_chunks_and_prediction_rollback() {
        let mut world = WorldStore::with_dimension(crate::WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(chunk(ChunkPos { x: -1, z: 0 }, 1));
        world.insert_decoded_chunk(chunk(ChunkPos { x: 0, z: 0 }, 1));
        world.insert_decoded_chunk(chunk(ChunkPos { x: 1, z: 0 }, 1));
        let torch = state("minecraft:torch");
        let exterior_source = BlockPos { x: -1, y: 8, z: 8 };
        let exterior_neighbor = BlockPos { x: 0, y: 8, z: 8 };
        assert!(world.apply_block_update(BlockUpdate {
            pos: exterior_source,
            block_state_id: torch
        }));
        let air = vanilla_air_block_state_id();
        let first = BlockPos { x: 15, y: 8, z: 8 };
        let second = BlockPos { x: 18, y: 8, z: 8 };
        let between = BlockPos { x: 16, y: 8, z: 8 };
        assert!(world.apply_block_update(BlockUpdate {
            pos: first,
            block_state_id: torch
        }));
        assert_eq!(
            world.apply_section_blocks_update(SectionBlocksUpdate {
                section_x: 1,
                section_y: 0,
                section_z: 0,
                updates: vec![BlockUpdate {
                    pos: second,
                    block_state_id: torch
                }],
            }),
            1
        );
        let before = world.publication_snapshot();
        world.run_light_updates().unwrap();
        assert_eq!(world.sample_block_light(first).unwrap().block, 14);
        assert_eq!(world.sample_block_light(between).unwrap().block, 13);
        assert_eq!(before.sample_block_light(between).unwrap().block, 0);

        assert!(world.apply_block_update(BlockUpdate {
            pos: first,
            block_state_id: air
        }));
        world.run_light_updates().unwrap();
        assert_eq!(world.sample_block_light(between).unwrap().block, 12);
        assert!(world.apply_block_update(BlockUpdate {
            pos: second,
            block_state_id: air
        }));
        world.run_light_updates().unwrap();
        assert_eq!(world.sample_block_light(between).unwrap().block, 0);
        assert_eq!(
            world.sample_block_light(exterior_neighbor).unwrap().block,
            13
        );
        assert!(world.apply_block_update(BlockUpdate {
            pos: exterior_source,
            block_state_id: air
        }));
        world.run_light_updates().unwrap();
        assert_eq!(
            world.sample_block_light(exterior_neighbor).unwrap().block,
            0
        );

        assert!(world.predict_local_player_block_state(first, torch, 7));
        world.run_light_updates().unwrap();
        assert_eq!(world.sample_block_light(between).unwrap().block, 13);
        assert!(world.apply_block_update(BlockUpdate {
            pos: first,
            block_state_id: air
        }));
        world.run_light_updates().unwrap();
        assert_eq!(world.sample_block_light(between).unwrap().block, 13);
        world.apply_block_changed_ack(BlockChangedAck { sequence: 7 });
        world.run_light_updates().unwrap();
        assert_eq!(world.sample_block_light(between).unwrap().block, 0);
    }

    #[test]
    fn removing_a_light_keeps_an_exterior_source_above_the_updated_sections() {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 80,
        });
        world.insert_decoded_chunk(chunk(ChunkPos { x: 0, z: 0 }, 5));
        let torch = state("minecraft:torch");
        let exterior = BlockPos { x: 8, y: 64, z: 8 };
        let edited = BlockPos { x: 8, y: 40, z: 8 };
        let between = BlockPos { x: 8, y: 51, z: 8 };
        assert!(world.apply_block_update(BlockUpdate {
            pos: exterior,
            block_state_id: torch
        }));
        world.run_light_updates().unwrap();
        assert_eq!(world.sample_block_light(between).unwrap().block, 1);
        assert!(world.apply_block_update(BlockUpdate {
            pos: edited,
            block_state_id: torch
        }));
        world.run_light_updates().unwrap();
        assert_eq!(world.sample_block_light(between).unwrap().block, 3);
        assert!(world.apply_block_update(BlockUpdate {
            pos: edited,
            block_state_id: vanilla_air_block_state_id(),
        }));
        world.run_light_updates().unwrap();
        assert_eq!(world.sample_block_light(between).unwrap().block, 1);
    }

    #[test]
    fn opening_and_closing_a_roof_propagates_sky_light() {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 64,
        });
        world.level = Some(crate::WorldLevelInfo {
            dimension: "minecraft:overworld".into(),
            dimension_type_id: 0,
            dimension_type_name: None,
            dimension_type_properties: Default::default(),
            biome_zoom_seed: 0,
            default_clock_resource_id: None,
            timeline_resource_ids: Vec::new(),
            has_sky_light: true,
            has_ceiling: false,
            cardinal_lighting: Default::default(),
            skybox: Default::default(),
            ambient_light: 0.0,
            environment_attributes: Default::default(),
            last_death_location: None,
            sea_level: 0,
            is_debug: false,
            is_flat: true,
        });
        let stone = state("minecraft:stone");
        let mut column = chunk(ChunkPos { x: 0, z: 0 }, 4);
        let roof_section = Arc::make_mut(&mut column.sections[3]);
        for z in 0..16 {
            for x in 0..16 {
                assert!(roof_section
                    .block_states
                    .set_value_at(section_block_index(x, 15, z), stone));
            }
        }
        roof_section.non_empty_block_count = 256;
        column.light = LightData::default();
        for section_index in 0..5 {
            column.light.set_sky_empty(section_index);
        }
        column.light.set_sky_packed(5, [0xFF; 2048]);
        world.insert_decoded_chunk(column);
        let roof = BlockPos { x: 8, y: 63, z: 8 };
        let below = BlockPos { x: 8, y: 8, z: 8 };
        assert_eq!(world.sample_block_light(below).unwrap().sky, 0);
        assert!(world.apply_block_update(BlockUpdate {
            pos: roof,
            block_state_id: vanilla_air_block_state_id()
        }));
        world.run_light_updates().unwrap();
        assert_eq!(world.sample_block_light(below).unwrap().sky, 15);
        assert!(world.apply_block_update(BlockUpdate {
            pos: roof,
            block_state_id: stone
        }));
        world.run_light_updates().unwrap();
        assert_eq!(world.sample_block_light(below).unwrap().sky, 0);
    }

    fn run_full_window_reference(world: &mut WorldStore) {
        let pending = std::mem::take(&mut world.local_light_updates.0);
        assert!(!pending.is_empty());
        let has_sky = world
            .level_info()
            .is_some_and(|level| level.has_sky_light());
        let min_section = world.dimension.min_section_y();
        let light_section_count = world.dimension.section_count() + 2;
        let air = vanilla_air_block_state_id();
        let mut block_sections =
            std::collections::BTreeMap::<(i32, i32), std::collections::BTreeSet<i32>>::new();
        let mut sky_window = std::collections::BTreeSet::new();
        for (&(x, y, z), &sky) in &pending {
            for (cx, cz) in neighborhood((x.div_euclid(16), z.div_euclid(16))) {
                if world.probe_chunk(ChunkPos { x: cx, z: cz }).is_some() {
                    let section_y = y.div_euclid(16);
                    block_sections
                        .entry((cx, cz))
                        .or_default()
                        .extend(section_y - 1..=section_y + 1);
                    if has_sky && sky {
                        sky_window.insert((cx, cz));
                    }
                }
            }
        }
        let window = block_sections
            .keys()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let mut storage = ChunkLightStorage::default();
        for (&(x, z), sections) in &block_sections {
            if sky_window.contains(&(x, z)) {
                for index in 0..light_section_count {
                    storage.register_non_empty_section((x, min_section - 1 + index as i32, z));
                }
            } else {
                for &section_y in sections {
                    storage.register_non_empty_section((x, section_y, z));
                }
            }
        }
        for (&(x, z), sections) in &block_sections {
            let column = world.probe_chunk(ChunkPos { x, z }).unwrap();
            for &section_y in sections {
                let index = (section_y - (min_section - 1)) as usize;
                storage.queue_layer_data(
                    (x, section_y, z),
                    false,
                    light_layer(&column.light, index, false),
                );
            }
            if sky_window.contains(&(x, z)) {
                for index in 0..light_section_count {
                    storage.queue_layer_data(
                        (x, min_section - 1 + index as i32, z),
                        true,
                        light_layer(&column.light, index, true),
                    );
                }
                let empty = column
                    .sections
                    .iter()
                    .map(|section| section.non_empty_block_count == 0 && section.fluid_count == 0)
                    .collect::<Vec<_>>();
                let mut sources = [i32::MIN; 256];
                for local_z in 0..16 {
                    for local_x in 0..16 {
                        sources[local_x + local_z * 16] = lowest_sky_source_y_with_registry(
                            min_section,
                            &empty,
                            air,
                            bbb_protocol::content_registry::ContentRegistry::vanilla(),
                            |section_index, local_y| {
                                column.sections[section_index]
                                    .block_states
                                    .value_at(section_block_index(
                                        local_x as u8,
                                        local_y as u8,
                                        local_z as u8,
                                    ))
                                    .unwrap()
                                    .global_id
                            },
                        )
                        .unwrap();
                    }
                }
                storage.set_column_sources((x, z), &sources);
            }
            storage.enable_column((x, z));
        }
        storage.take_dirty();
        {
            let mut engine = LightEngine::new(
                &mut storage,
                |(x, y, z)| world.block_state_id_at(BlockPos { x, y, z }),
                |chunk| window.contains(&chunk),
                air,
            )
            .unwrap();
            for &position in pending.keys() {
                engine.check_block(position).unwrap();
            }
            engine.run().unwrap();
        }
        if !sky_window.is_empty() {
            let mut engine = LightEngine::new(
                &mut storage,
                |(x, y, z)| world.block_state_id_at(BlockPos { x, y, z }),
                |chunk| sky_window.contains(&chunk),
                air,
            )
            .unwrap();
            engine.set_sky(true);
            for (&position, &sky) in &pending {
                if sky {
                    engine.check_sky(position);
                }
            }
            engine.run().unwrap();
        }
        for ((x, z), (sky, block)) in storage.take_dirty() {
            if !window.contains(&(x, z)) {
                continue;
            }
            let light = world.chunks.light_mut(ChunkPos { x, z }).unwrap();
            for (is_sky, sections) in [(false, block), (true, sky)] {
                if is_sky && !sky_window.contains(&(x, z)) {
                    continue;
                }
                for section_y in sections {
                    let index = section_y - (min_section - 1);
                    if (0..light_section_count as i32).contains(&index) {
                        replace_light_layer_bytes(
                            light,
                            index as usize,
                            is_sky,
                            storage.resolved_layer_bytes((x, section_y, z), is_sky),
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn restricted_column_and_section_relighting_is_bit_identical_to_full_window() {
        let mut fast = WorldStore::with_dimension(WorldDimension {
            min_y: -64,
            height: 384,
        });
        fast.level = Some(crate::WorldLevelInfo {
            dimension: "minecraft:overworld".into(),
            dimension_type_id: 0,
            dimension_type_name: None,
            dimension_type_properties: Default::default(),
            biome_zoom_seed: 0,
            default_clock_resource_id: None,
            timeline_resource_ids: Vec::new(),
            has_sky_light: true,
            has_ceiling: false,
            cardinal_lighting: Default::default(),
            skybox: Default::default(),
            ambient_light: 0.0,
            environment_attributes: Default::default(),
            last_death_location: None,
            sea_level: 63,
            is_debug: false,
            is_flat: false,
        });
        let stone = state("minecraft:stone");
        let torch = state("minecraft:torch");
        let glowstone = state("minecraft:glowstone");
        let air = vanilla_air_block_state_id();

        let chunks: Vec<_> = (-1..=1)
            .flat_map(|cz| (-1..=1).map(move |cx| ChunkPos { x: cx, z: cz }))
            .collect();
        for &pos in &chunks {
            let mut col = chunk(pos, 24);
            // Solid floor at section 8 (`y = 64..79`, local_y = 0 -> `y = 64`) and roof at section 10 (`y = 111`).
            {
                let floor = Arc::make_mut(&mut col.sections[8]);
                for z in 0..16 {
                    for x in 0..16 {
                        assert!(floor
                            .block_states
                            .set_value_at(section_block_index(x, 0, z), stone));
                    }
                }
                floor.non_empty_block_count = 256;
            }
            {
                let roof = Arc::make_mut(&mut col.sections[10]);
                for z in 0..16 {
                    for x in 0..16 {
                        assert!(roof
                            .block_states
                            .set_value_at(section_block_index(x, 15, z), stone));
                    }
                }
                roof.non_empty_block_count = 256;
            }
            // Sky light 15 above roof (`index >= 12`), 0 at and below roof (`index < 12`).
            col.light = LightData::default();
            for section_index in 0..12 {
                col.light.set_sky_empty(section_index);
            }
            for section_index in 12..26 {
                col.light.set_sky_packed(section_index, [0xFF; 2048]);
            }
            fast.insert_decoded_chunk(col);
        }
        let mut reference = fast.clone();

        let batches: &[&[(BlockPos, i32)]] = &[
            // Multi-block skylight roof openings across chunk borders + block light emitters.
            &[
                (BlockPos { x: 0, y: 111, z: 0 }, air),
                (
                    BlockPos {
                        x: 15,
                        y: 111,
                        z: 15,
                    },
                    air,
                ),
                (
                    BlockPos {
                        x: 16,
                        y: 111,
                        z: 15,
                    },
                    air,
                ),
                (BlockPos { x: 8, y: 70, z: 8 }, glowstone),
                (BlockPos { x: -1, y: 65, z: 0 }, torch),
            ],
            // Partial overhang under the open shaft + closing one shaft while opening another.
            &[
                (BlockPos { x: 0, y: 90, z: 0 }, stone),
                (
                    BlockPos {
                        x: 15,
                        y: 111,
                        z: 15,
                    },
                    stone,
                ),
                (
                    BlockPos {
                        x: -8,
                        y: 111,
                        z: 4,
                    },
                    air,
                ),
                (BlockPos { x: 8, y: 70, z: 8 }, air),
            ],
            // Open all the way through the floor into empty sections down to min_y.
            &[
                (BlockPos { x: -8, y: 64, z: 4 }, air),
                (
                    BlockPos {
                        x: 16,
                        y: 111,
                        z: 15,
                    },
                    stone,
                ),
                (BlockPos { x: -1, y: 65, z: 0 }, air),
            ],
        ];

        for (step, batch) in batches.iter().enumerate() {
            for &(pos, block_state_id) in *batch {
                assert!(fast.apply_block_update(BlockUpdate {
                    pos,
                    block_state_id,
                }));
                assert!(reference.apply_block_update(BlockUpdate {
                    pos,
                    block_state_id,
                }));
            }
            fast.run_light_updates().unwrap();
            run_full_window_reference(&mut reference);

            for &pos in &chunks {
                assert_eq!(
                    fast.probe_chunk(pos).unwrap().light,
                    reference.probe_chunk(pos).unwrap().light,
                    "bit-identical LightData mismatch at step {step}, chunk {pos:?}"
                );
            }
        }
    }

    #[test]
    fn modded_block_updates_and_local_light_do_not_drop_or_panic() {
        use bbb_protocol::content_registry::{ContentRegistryBuilder, ModBlockRegistration};
        use std::sync::Arc;

        let mut builder = ContentRegistryBuilder::new();
        builder.register_block(ModBlockRegistration {
            resource_id: "mod_test:glow_cube".into(),
            state_count: 1,
            light_emission: 12,
            destroy_speed: 1.0,
            full_cube_collision: true,
            dynamic_light_rgb: None,
        });
        let registry = Arc::new(builder.freeze_for_session().unwrap());
        let mod_block = registry.blocks().first().unwrap();
        let mod_state_id = mod_block.first_state_id;
        let mod_block_id = mod_block.block_id;

        let mut store = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        store.insert_decoded_chunk(chunk(ChunkPos { x: 0, z: 0 }, 1));
        store.set_content_registry(Arc::clone(&registry));
        assert_eq!(
            store.registries().block_name_by_registry_id(mod_block_id),
            Some("mod_test:glow_cube")
        );

        let pos = BlockPos { x: 4, y: 8, z: 4 };
        assert!(store.apply_block_update(BlockUpdate {
            pos,
            block_state_id: mod_state_id,
        }));
        assert_eq!(store.probe_block(pos).unwrap().block_state_id, mod_state_id);
        store.run_light_updates().unwrap();
        assert_eq!(store.sample_block_light(pos).unwrap().block, 12);

        let pos2 = BlockPos { x: 8, y: 8, z: 8 };
        let applied = store.apply_section_blocks_update(SectionBlocksUpdate {
            section_x: 0,
            section_y: 0,
            section_z: 0,
            updates: vec![BlockUpdate {
                pos: pos2,
                block_state_id: mod_state_id,
            }],
        });
        assert_eq!(applied, 1);
        assert_eq!(
            store.probe_block(pos2).unwrap().block_state_id,
            mod_state_id
        );
        store.run_light_updates().unwrap();
        assert_eq!(store.sample_block_light(pos2).unwrap().block, 12);
    }
}
