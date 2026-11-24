use bbb_protocol::chunk_light::light_engine::{self, light_facts, light_facts_with_registry};
use bbb_protocol::content_registry::ContentRegistry;

use super::ChunkStore;
use bbb_server_persistence::Java26ChunkStatus;
use bbb_server_types::chunk_data::{
    ChunkColumn, ChunkFeaturesState, ChunkGenerationState, ChunkInitializeLightState, SectionBlocks,
};
use bbb_server_types::save::{SaveError, SaveResult};

const SECTION_SIDE: i32 = 16;
const COLUMN_SIDE: usize = 16;
const NO_SKY_SOURCE: i32 = i32::MIN;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnLightSources {
    lowest_sky_source_y: [i32; COLUMN_SIDE * COLUMN_SIDE],
    non_empty_sections: Vec<i32>,
}

impl ColumnLightSources {
    pub const fn lowest_sky_source_y(&self) -> &[i32; COLUMN_SIDE * COLUMN_SIDE] {
        &self.lowest_sky_source_y
    }

    pub fn non_empty_sections(&self) -> &[i32] {
        &self.non_empty_sections
    }

    pub(super) fn from_noise(initialized: &ChunkInitializeLightState) -> Self {
        Self {
            lowest_sky_source_y: *initialized.lowest_sky_source_y(),
            non_empty_sections: initialized.non_empty_sections().to_vec(),
        }
    }
}

impl ChunkStore {
    pub fn initialize_light(&mut self, chunk: (i32, i32)) -> SaveResult<()> {
        self.initialize_light_batch(&[chunk])
    }

    /// Stage the requested columns on the caller; a world tick must not wait
    /// for queued terrain generation on the shared pool to start lighting.
    pub(super) fn initialize_light_batch(&mut self, chunks: &[(i32, i32)]) -> SaveResult<()> {
        let staged = chunks
            .iter()
            .map(|&chunk| self.stage_initialize_light_state(chunk))
            .collect::<Vec<_>>();
        for (&chunk, staged) in chunks.iter().zip(staged) {
            self.set_generation(chunk, ChunkGenerationState::InitializeLight(staged?));
            self.advance_pending_retrogen_status(chunk, Java26ChunkStatus::InitializeLight);
            // A saved FEATURES column now differs from its disk copy.
            if self.is_storage_backed_generation(chunk) {
                self.mark_whole_rewrite_pending(chunk)?;
            }
        }
        Ok(())
    }

    /// Advances the direct dependency window required by the LIGHT center task.
    /// `ChunkPyramid.java:36-37` requires INITIALIZE_LIGHT at radius one.
    pub fn initialize_light_window(&mut self, center: (i32, i32)) -> SaveResult<()> {
        self.initialize_light_windows(&[center])
    }

    pub(super) fn initialize_light_windows(&mut self, centers: &[(i32, i32)]) -> SaveResult<()> {
        let mut pending = Vec::new();
        let mut seen = rustc_hash::FxHashSet::default();
        let mut failure = None;
        'centers: for &center in centers {
            let coordinates = match initialize_light_window_coordinates(center) {
                Ok(coordinates) => coordinates,
                Err(error) => {
                    failure = Some(error);
                    break;
                }
            };
            for chunk in coordinates {
                match self.generation(chunk) {
                    Some(ChunkGenerationState::InitializeLight(_))
                    | Some(ChunkGenerationState::Light(_))
                    | Some(ChunkGenerationState::Spawn(_))
                    | Some(ChunkGenerationState::NoiseFull(_)) => {}
                    Some(ChunkGenerationState::Features(_)) => {
                        if seen.insert(chunk) {
                            pending.push(chunk);
                        }
                    }
                    Some(_) => {
                        failure = Some(invalid_initialize_light(
                            chunk,
                            "LIGHT dependency window requires a noise FEATURES-or-later state",
                        ));
                        break 'centers;
                    }
                    None => {
                        failure = Some(invalid_initialize_light(
                            chunk,
                            "LIGHT dependency window requires existing generation state",
                        ));
                        break 'centers;
                    }
                }
            }
        }
        self.initialize_light_batch(&pending)?;
        failure.map_or(Ok(()), Err)
    }

    fn stage_initialize_light_state(
        &self,
        chunk: (i32, i32),
    ) -> SaveResult<ChunkInitializeLightState> {
        if !self.chunk_flag(chunk, |chunk| chunk.pending_initial_persistence)
            && !self.is_durable(chunk)
        {
            return Err(invalid_initialize_light(
                chunk,
                "INITIALIZE_LIGHT requires a generated or saved FEATURES column",
            ));
        }
        let features = match self.generation(chunk) {
            Some(ChunkGenerationState::Features(features)) => features.clone(),
            Some(_) => {
                return Err(invalid_initialize_light(
                    chunk,
                    "INITIALIZE_LIGHT requires FEATURES state",
                ))
            }
            None => {
                return Err(invalid_initialize_light(
                    chunk,
                    "INITIALIZE_LIGHT requires existing generation state",
                ))
            }
        };
        self.stage_initialize_light_from_features(chunk, features)
    }

    pub fn stage_initialize_light_from_features(
        &self,
        chunk: (i32, i32),
        features: ChunkFeaturesState,
    ) -> SaveResult<ChunkInitializeLightState> {
        let sources = self.stage_column_light_sources(chunk)?;
        Ok(ChunkInitializeLightState {
            features,
            lowest_sky_source_y: sources.lowest_sky_source_y,
            non_empty_sections: sources.non_empty_sections,
        })
    }

    // ChunkStatusTasks.java:149-156: initializing light sources is generator-independent.
    pub fn stage_column_light_sources(&self, chunk: (i32, i32)) -> SaveResult<ColumnLightSources> {
        let column = self.column_at(chunk).ok_or_else(|| {
            invalid_initialize_light(chunk, "INITIALIZE_LIGHT requires a loaded column")
        })?;
        let (lowest_sky_source_y, non_empty_sections) =
            stage_initialize_light(chunk, column, self.custom_block_registry())?;
        Ok(ColumnLightSources {
            lowest_sky_source_y,
            non_empty_sections,
        })
    }
}

pub(super) fn noise_initialized(
    state: &ChunkGenerationState,
) -> Option<&ChunkInitializeLightState> {
    match state {
        ChunkGenerationState::InitializeLight(initialized) => Some(initialized),
        ChunkGenerationState::Light(light) => Some(light.initialized()),
        ChunkGenerationState::Spawn(spawn) => Some(spawn.light().initialized()),
        ChunkGenerationState::NoiseFull(full) => Some(full.light().initialized()),
        _ => None,
    }
}

pub fn initialize_light_window_coordinates(center: (i32, i32)) -> SaveResult<[(i32, i32); 9]> {
    let mut coordinates = [(0, 0); 9];
    for (index, coordinate) in coordinates.iter_mut().enumerate() {
        let offset_x = i32::try_from(index / 3).expect("window x index fits i32") - 1;
        let offset_z = i32::try_from(index % 3).expect("window z index fits i32") - 1;
        coordinate.0 = center
            .0
            .checked_add(offset_x)
            .ok_or_else(|| invalid_initialize_light(center, "dependency window x overflows"))?;
        coordinate.1 = center
            .1
            .checked_add(offset_z)
            .ok_or_else(|| invalid_initialize_light(center, "dependency window z overflows"))?;
    }
    Ok(coordinates)
}

fn stage_initialize_light(
    chunk: (i32, i32),
    column: &ChunkColumn,
    registry: Option<&ContentRegistry>,
) -> SaveResult<([i32; COLUMN_SIDE * COLUMN_SIDE], Vec<i32>)> {
    chunk
        .0
        .checked_mul(SECTION_SIDE)
        .ok_or_else(|| invalid_initialize_light(chunk, "chunk x overflows block coordinates"))?;
    chunk
        .1
        .checked_mul(SECTION_SIDE)
        .ok_or_else(|| invalid_initialize_light(chunk, "chunk z overflows block coordinates"))?;
    column
        .min_y()
        .checked_sub(1)
        .ok_or_else(|| invalid_initialize_light(chunk, "minimum light y underflows"))?;
    let section_count = i32::try_from(column.section_count())
        .map_err(|_| invalid_initialize_light(chunk, "section count exceeds Java int"))?;
    let height = section_count
        .checked_mul(SECTION_SIDE)
        .ok_or_else(|| invalid_initialize_light(chunk, "column height overflows Java int"))?;
    column
        .min_y()
        .checked_add(height)
        .ok_or_else(|| invalid_initialize_light(chunk, "maximum light y overflows"))?;

    validate_light_facts(chunk, column, registry)?;
    let mut non_empty_sections = Vec::new();
    let mut empty_sections = Vec::with_capacity(column.section_count());
    let min_section_y = column.min_y().div_euclid(SECTION_SIDE);
    for index in 0..column.section_count() {
        let empty = section_has_only_air(column, index);
        empty_sections.push(empty);
        if empty {
            continue;
        }
        let index_i32 = i32::try_from(index)
            .map_err(|_| invalid_initialize_light(chunk, "section index exceeds Java int"))?;
        non_empty_sections.push(min_section_y.checked_add(index_i32).ok_or_else(|| {
            invalid_initialize_light(chunk, "absolute section y overflows Java int")
        })?);
    }

    let mut lowest_sky_source_y = [NO_SKY_SOURCE; COLUMN_SIDE * COLUMN_SIDE];
    let Some(highest_non_empty) = empty_sections.iter().rposition(|empty| !empty) else {
        return Ok((lowest_sky_source_y, non_empty_sections));
    };
    let empty_sections = &empty_sections[..=highest_non_empty];
    for z in 0..COLUMN_SIDE {
        for x in 0..COLUMN_SIDE {
            lowest_sky_source_y[x + z * COLUMN_SIDE] = column_lowest_sky_source_y(
                min_section_y,
                empty_sections,
                column.air_state_id(),
                registry,
                |section, y| column.section(section).get(x, y, z),
            )?;
        }
    }
    Ok((lowest_sky_source_y, non_empty_sections))
}

/// `ChunkSkyLightSources.update` for one changed column of a lit chunk,
/// recomputed from the current blocks (`ChunkSkyLightSources.java:79-150`).
pub(super) fn lowest_sky_source_y_with_empty_sections(
    column: &ChunkColumn,
    empty_sections: &[bool],
    x: i32,
    z: i32,
    registry: Option<&ContentRegistry>,
) -> SaveResult<i32> {
    column_lowest_sky_source_y(
        column.min_y().div_euclid(SECTION_SIDE),
        empty_sections,
        column.air_state_id(),
        registry,
        |section, y| column.section(section).get(x as usize, y, z as usize),
    )
}

fn column_lowest_sky_source_y(
    min_section_y: i32,
    empty_sections: &[bool],
    air_state: i32,
    registry: Option<&ContentRegistry>,
    state_at: impl FnMut(usize, usize) -> i32,
) -> SaveResult<i32> {
    Ok(match registry {
        Some(registry) => light_engine::lowest_sky_source_y_with_registry(
            min_section_y,
            empty_sections,
            air_state,
            registry,
            state_at,
        )?,
        None => {
            light_engine::lowest_sky_source_y(min_section_y, empty_sections, air_state, state_at)?
        }
    })
}

fn validate_light_facts(
    chunk: (i32, i32),
    column: &ChunkColumn,
    registry: Option<&ContentRegistry>,
) -> SaveResult<()> {
    for section_index in 0..column.section_count() {
        let section = column.section(section_index);
        let palette = match section {
            SectionBlocks::Single(state) => std::slice::from_ref(state),
            SectionBlocks::Indirect { palette, .. } => palette.as_slice(),
        };
        for &state in palette {
            // Unused palette entries must not introduce a new validation failure.
            let has_facts = match registry {
                Some(registry) => light_facts_with_registry(state, registry).is_some(),
                None => light_facts(state).is_some(),
            };
            if !has_facts && section.any_cell(|actual| actual == state) {
                return Err(invalid_initialize_light(
                    chunk,
                    format!("block state {state} has no light facts"),
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn section_has_only_air(column: &ChunkColumn, index: usize) -> bool {
    column.section_has_only_air(index)
}

fn invalid_initialize_light(chunk: (i32, i32), detail: impl Into<String>) -> SaveError {
    SaveError::InvalidNbt {
        path: None,
        detail: format!(
            "INITIALIZE_LIGHT chunk [{},{}]: {}",
            chunk.0,
            chunk.1,
            detail.into()
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::block_state_types::{
        vanilla_block_state_id_by_name_and_properties,
        vanilla_block_state_light_face_occlusion_index, vanilla_block_state_support_facts,
        vanilla_face_shape_union_occludes, VanillaBlockSupportDirection, VANILLA_BLOCK_STATE_COUNT,
    };
    use bbb_protocol::packets::BlockPos;
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    use super::*;
    use crate::block_entity::new_block_entity;
    use bbb_server_types::chunk_data::feature_heightmaps::FeatureHeightmaps;
    use bbb_server_types::chunk_data::ChunkFeaturesState;
    use bbb_server_types::chunk_ticks::{BlockTickType, ScheduledTick};

    const CHUNK: (i32, i32) = (0, 0);

    fn state(name: &str, properties: &[(&str, &str)]) -> i32 {
        vanilla_block_state_id_by_name_and_properties(
            name,
            &properties
                .iter()
                .map(|&(name, value)| (name.to_string(), value.to_string()))
                .collect::<BTreeMap<_, _>>(),
        )
        .unwrap()
    }

    fn air() -> i32 {
        state("minecraft:air", &[])
    }

    fn stone() -> i32 {
        state("minecraft:stone", &[])
    }

    fn empty_column() -> ChunkColumn {
        let store = ChunkStore::new(
            crate::world_creation::default_overworld_level_source().unwrap(),
            0,
        );
        ChunkColumn::new(store.min_y(), store.section_count(), air(), 0)
    }

    fn features_store(mut column: ChunkColumn) -> ChunkStore {
        let stone = stone();
        let min_y = column.min_y();
        assert!(column.block_ticks_mut().schedule(ScheduledTick::new(
            BlockTickType::of_block_state(stone).unwrap(),
            BlockPos {
                x: 1,
                y: min_y,
                z: 1,
            },
            19,
            3,
        )));
        let mut store = ChunkStore::new(
            crate::world_creation::default_overworld_level_source().unwrap(),
            0,
        );
        store.insert_column(CHUNK.0, CHUNK.1, column);
        let features = ChunkFeaturesState {
            postprocessing_positions: vec![(1, store.min_y(), 2)],
            carving_mask: vec![3, 5, 8],
            final_heightmaps: FeatureHeightmaps::prime(store.column(CHUNK.0, CHUNK.1).unwrap()),
        };
        store.set_generation(CHUNK, ChunkGenerationState::Features(features));
        store.loaded_mut(CHUNK).pending_initial_persistence = true;
        store.loaded_mut(CHUNK).proto_entities = vec![vec![NbtSummaryEntry {
            name: "id".into(),
            value: NbtSummaryValue::string("minecraft:marker"),
        }]];
        store
    }

    type ChunkTestSnapshot = BTreeMap<(i32, i32), (ChunkColumn, Vec<Vec<NbtSummaryEntry>>, bool)>;

    fn chunk_snapshot(store: &ChunkStore) -> ChunkTestSnapshot {
        store
            .chunks
            .iter()
            .map(|(&key, chunk)| {
                (
                    key,
                    (
                        chunk.column.clone(),
                        chunk.proto_entities.clone(),
                        chunk.pending_initial_persistence,
                    ),
                )
            })
            .collect()
    }

    fn insert_features_chunk(store: &mut ChunkStore, chunk: (i32, i32), column: ChunkColumn) {
        let final_heightmaps = FeatureHeightmaps::prime(&column);
        store.insert_column(chunk.0, chunk.1, column);
        store.set_generation(
            chunk,
            ChunkGenerationState::Features(ChunkFeaturesState {
                postprocessing_positions: Vec::new(),
                carving_mask: Vec::new(),
                final_heightmaps,
            }),
        );
        store.loaded_mut(chunk).pending_initial_persistence = true;
    }

    fn features_window_store() -> ChunkStore {
        let mut store = ChunkStore::new(
            crate::world_creation::default_overworld_level_source().unwrap(),
            0,
        );
        for chunk in initialize_light_window_coordinates(CHUNK).unwrap() {
            insert_features_chunk(&mut store, chunk, empty_column());
        }
        insert_features_chunk(&mut store, (2, 0), empty_column());
        store
    }

    #[test]
    fn initialize_light_all_air_uses_public_sentinel_and_no_sections() {
        let mut store = features_store(empty_column());
        store.initialize_light(CHUNK).unwrap();
        let ChunkGenerationState::InitializeLight(initialized) =
            store.generation_state(CHUNK.0, CHUNK.1).unwrap()
        else {
            panic!("FEATURES did not advance")
        };
        assert_eq!(initialized.lowest_sky_source_y, [i32::MIN; 256]);
        assert!(initialized.non_empty_sections.is_empty());
    }

    #[test]
    fn initialize_light_treats_all_air_variants_as_empty() {
        let mut column = empty_column();
        let min_y = column.min_y();
        column
            .set_block_state(0, min_y, 0, state("minecraft:cave_air", &[]))
            .unwrap();
        column
            .set_block_state(1, min_y, 0, state("minecraft:void_air", &[]))
            .unwrap();
        let mut store = features_store(column);

        store.initialize_light(CHUNK).unwrap();

        let ChunkGenerationState::InitializeLight(initialized) =
            store.generation_state(CHUNK.0, CHUNK.1).unwrap()
        else {
            unreachable!()
        };
        assert_eq!(initialized.lowest_sky_source_y, [i32::MIN; 256]);
        assert!(initialized.non_empty_sections.is_empty());
    }

    #[test]
    fn initialize_light_ignores_unused_solid_and_invalid_palette_entries() {
        let mut column = empty_column();
        let baseline = stage_initialize_light(CHUNK, &column, None).unwrap();
        for block in [stone(), i32::MAX] {
            column.set_block_state(1, 200, 1, block).unwrap();
            column.set_block_state(1, 200, 1, air()).unwrap();
        }
        assert_eq!(
            stage_initialize_light(CHUNK, &column, None).unwrap(),
            baseline
        );
        column.set_block_state(1, 200, 1, stone()).unwrap();
        let (sources, nonempty) = stage_initialize_light(CHUNK, &column, None).unwrap();
        assert_eq!(sources[1 + 16], 201);
        assert_eq!(nonempty, [12]);
        column.set_block_state(1, 200, 1, i32::MAX).unwrap();
        assert!(stage_initialize_light(CHUNK, &column, None).is_err());
    }

    #[test]
    fn initialize_light_and_light_chunk_support_registered_custom_mod_blocks() {
        use std::sync::Arc;

        use bbb_protocol::content_registry::{ContentRegistryBuilder, ModBlockRegistration};

        let mut builder = ContentRegistryBuilder::new();
        builder.register_block(ModBlockRegistration {
            resource_id: "modded:glow_block".to_string(),
            state_count: 1,
            light_emission: 13,
            destroy_speed: 1.0,
            full_cube_collision: true,
            dynamic_light_rgb: None,
        });
        builder.register_block(ModBlockRegistration {
            resource_id: "modded:clear_panel".to_string(),
            state_count: 1,
            light_emission: 0,
            destroy_speed: 0.3,
            full_cube_collision: false,
            dynamic_light_rgb: None,
        });
        let registry = Arc::new(builder.freeze().unwrap());
        let glow_block = registry
            .block_by_resource_id("modded:glow_block")
            .unwrap()
            .first_state_id;
        let clear_panel = registry
            .block_by_resource_id("modded:clear_panel")
            .unwrap()
            .first_state_id;

        let mut store = features_window_store();
        store.install_content_registry(Arc::clone(&registry));
        let min_y = store.min_y();
        store
            .column_mut(CHUNK.0, CHUNK.1)
            .unwrap()
            .set_block_state(4, min_y + 10, 4, glow_block)
            .unwrap();
        store
            .column_mut(CHUNK.0, CHUNK.1)
            .unwrap()
            .set_block_state(5, min_y + 10, 4, clear_panel)
            .unwrap();

        store.initialize_light_window(CHUNK).unwrap();
        let ChunkGenerationState::InitializeLight(initialized) =
            store.generation_state(CHUNK.0, CHUNK.1).unwrap()
        else {
            unreachable!()
        };
        assert_eq!(initialized.lowest_sky_source_y[4 + 4 * 16], min_y + 11);
        assert_eq!(initialized.lowest_sky_source_y[5 + 4 * 16], i32::MIN);

        store.light_chunk(CHUNK).unwrap();
        assert_eq!(store.block_light_level(4, min_y + 10, 4), 13);
        assert_eq!(store.block_light_level(5, min_y + 10, 4), 12);
        assert_eq!(store.block_light_level(6, min_y + 10, 4), 11);
        assert_eq!(store.sky_light_level(4, min_y + 10, 4), 0);
        assert_eq!(store.sky_light_level(5, min_y + 10, 4), 15);

        let mut invalid = empty_column();
        invalid
            .set_block_state(0, min_y, 0, registry.total_block_state_count())
            .unwrap();
        assert!(stage_initialize_light(CHUNK, &invalid, Some(&registry)).is_err());
    }

    #[test]
    fn initialize_light_dampening_edge_and_metadata_are_transactional() {
        let mut column = empty_column();
        let min_y = column.min_y();
        column.set_block_state(0, min_y, 0, stone()).unwrap();
        let chest = state(
            "minecraft:chest",
            &[
                ("facing", "north"),
                ("type", "single"),
                ("waterlogged", "false"),
            ],
        );
        column.set_block_state(2, min_y, 2, chest).unwrap();
        let mut store = features_store(column);
        store.block_entities_mut().add_and_register(
            new_block_entity(
                BlockPos {
                    x: 2,
                    y: min_y,
                    z: 2,
                },
                chest,
            )
            .unwrap(),
        );

        let before_column = store.column(CHUNK.0, CHUNK.1).unwrap().clone();
        let before_features = match store.generation_state(CHUNK.0, CHUNK.1).unwrap() {
            ChunkGenerationState::Features(features) => features.clone(),
            _ => unreachable!(),
        };

        let before_block_entities = store
            .block_entities()
            .chunk(CHUNK)
            .cloned()
            .collect::<Vec<_>>();
        let before_chunks = chunk_snapshot(&store);
        let before_structures = format!("{:?}", store.structure_status);

        store.initialize_light(CHUNK).unwrap();

        let ChunkGenerationState::InitializeLight(initialized) =
            store.generation_state(CHUNK.0, CHUNK.1).unwrap()
        else {
            panic!("FEATURES did not advance")
        };
        assert_eq!(initialized.features, before_features);
        assert_eq!(initialized.lowest_sky_source_y[0], min_y + 1);
        assert_eq!(initialized.non_empty_sections, vec![min_y.div_euclid(16)]);
        assert_eq!(store.column(CHUNK.0, CHUNK.1).unwrap(), &before_column);
        assert_eq!(
            store
                .block_entities()
                .chunk(CHUNK)
                .cloned()
                .collect::<Vec<_>>(),
            before_block_entities
        );
        assert_eq!(chunk_snapshot(&store), before_chunks);
        assert_eq!(format!("{:?}", store.structure_status), before_structures);
    }

    #[test]
    fn initialize_light_complementary_faces_occlude_but_one_face_does_not() {
        let (top, bottom) = complementary_face_states();
        let mut column = empty_column();
        let min_y = column.min_y();
        column.set_block_state(0, min_y + 1, 0, top).unwrap();
        column.set_block_state(0, min_y, 0, bottom).unwrap();
        column.set_block_state(1, min_y + 1, 0, top).unwrap();
        let mut store = features_store(column);

        store.initialize_light(CHUNK).unwrap();

        let ChunkGenerationState::InitializeLight(initialized) =
            store.generation_state(CHUNK.0, CHUNK.1).unwrap()
        else {
            unreachable!()
        };
        assert_eq!(initialized.lowest_sky_source_y[0], min_y + 1);
        assert_eq!(initialized.lowest_sky_source_y[1], i32::MIN);
    }

    #[test]
    fn initialize_light_records_absolute_non_empty_sections_and_skips_empty_gaps() {
        let mut column = empty_column();
        let min_y = column.min_y();
        column.set_block_state(3, min_y, 0, stone()).unwrap();
        column
            .set_block_state(4, min_y + 2 * 16, 0, stone())
            .unwrap();
        let mut store = features_store(column);

        store.initialize_light(CHUNK).unwrap();

        let ChunkGenerationState::InitializeLight(initialized) =
            store.generation_state(CHUNK.0, CHUNK.1).unwrap()
        else {
            unreachable!()
        };
        let base_section = min_y.div_euclid(16);
        assert_eq!(
            initialized.non_empty_sections,
            vec![base_section, base_section + 2]
        );
        assert_eq!(initialized.lowest_sky_source_y[3], min_y + 1);
        assert_eq!(initialized.lowest_sky_source_y[4], min_y + 33);
    }

    #[test]
    fn initialize_light_window_advances_exact_radius_one_atomically() {
        let mut store = features_window_store();
        store.initialize_light((-1, -1)).unwrap();
        let existing = store.generation_state(-1, -1).unwrap().clone();
        let before_chunks = chunk_snapshot(&store);
        let before_structures = format!("{:?}", store.structure_status);

        store.initialize_light_window(CHUNK).unwrap();

        for chunk in initialize_light_window_coordinates(CHUNK).unwrap() {
            assert!(matches!(
                store.generation_state(chunk.0, chunk.1),
                Some(ChunkGenerationState::InitializeLight(_))
            ));
        }
        assert_eq!(store.generation_state(-1, -1), Some(&existing));
        assert!(matches!(
            store.generation_state(2, 0),
            Some(ChunkGenerationState::Features(_))
        ));
        assert_eq!(chunk_snapshot(&store), before_chunks);
        assert_eq!(format!("{:?}", store.structure_status), before_structures);
    }

    fn complementary_face_states() -> (i32, i32) {
        for top in 0..VANILLA_BLOCK_STATE_COUNT {
            let Some(top_facts) = vanilla_block_state_support_facts(top) else {
                continue;
            };
            if top_facts.light_dampening() != 0 {
                continue;
            }
            let Some(top_up) = vanilla_block_state_light_face_occlusion_index(
                top,
                VanillaBlockSupportDirection::Up,
            ) else {
                continue;
            };
            let Some(top_down) = vanilla_block_state_light_face_occlusion_index(
                top,
                VanillaBlockSupportDirection::Down,
            ) else {
                continue;
            };
            if vanilla_face_shape_union_occludes(0, top_up) != Some(false)
                || vanilla_face_shape_union_occludes(top_down, 0) != Some(false)
            {
                continue;
            }
            for bottom in 0..VANILLA_BLOCK_STATE_COUNT {
                let Some(bottom_facts) = vanilla_block_state_support_facts(bottom) else {
                    continue;
                };
                if bottom_facts.light_dampening() != 0 {
                    continue;
                }
                let Some(bottom_up) = vanilla_block_state_light_face_occlusion_index(
                    bottom,
                    VanillaBlockSupportDirection::Up,
                ) else {
                    continue;
                };
                if vanilla_face_shape_union_occludes(0, bottom_up) == Some(false)
                    && vanilla_face_shape_union_occludes(top_down, bottom_up) == Some(true)
                {
                    return (top, bottom);
                }
            }
        }
        panic!("Vanilla 26.1 has no complementary vertical light-face pair")
    }
}
