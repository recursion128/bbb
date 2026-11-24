//! Hydration of retained saved columns into their actual chunk storage.

use super::generation_publication::GenerationItemPublication;
use super::*;
use crate::{block_entity::BlockEntity, registry_sync::SynchronizedRegistries};
use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::{packets::BlockPos, NbtSummaryValue};
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_persistence::world_storage::Java26RuntimeColumn;
use bbb_server_persistence::Java26LoadedDimension;
use bbb_server_types::{chunk_data::SectionBlocks, chunk_ticks::ChunkTicks, save::Java26LevelKey};

#[derive(Debug)]
pub struct SavedColumnEffects {
    pub chunk: (i32, i32),
    pub had_saved_ticks: bool,
}

#[derive(Debug)]
pub(crate) struct SavedColumnResources {
    pub(crate) dimension: Java26LevelKey,
    pub(crate) registries: SynchronizedRegistries,
    pub(crate) items: GenerationItemPublication,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SavedColumnRestoration {
    inputs: BTreeMap<(i32, i32), Java26RuntimeColumn>,
    pub(super) resources: Option<Arc<SavedColumnResources>>,
    activated: Vec<Arc<SavedColumnEffects>>,
    pub(super) stored: super::stored_columns::StoredColumns,
}

impl ChunkStore {
    pub(super) fn prepare_generation_admission(&mut self, chunk: (i32, i32)) -> SaveResult<()> {
        self.read_stored_column(chunk)?;
        if self.cold_columns.contains_key(&chunk) {
            self.restore_saved_column_input(chunk)?;
        }
        self.require_generation_admission(chunk)
    }

    pub(super) fn prepare_structure_window_admission(
        &mut self,
        chunk: (i32, i32),
    ) -> SaveResult<()> {
        for source in structure_status::structure_starts_window(chunk.0, chunk.1)? {
            self.prepare_generation_admission(source)?;
        }
        self.require_structure_window_admission(chunk)
    }

    pub(crate) fn install_saved_column_resources(&mut self, resources: SavedColumnResources) {
        self.saved_column_restoration.resources = Some(Arc::new(resources));
    }

    pub fn retain_saved_column_input(&mut self, chunk: (i32, i32), record: Java26RuntimeColumn) {
        if let Some(error) = &record.poi_error {
            tracing::error!(
                event = "world.stored_poi_unreadable",
                x = chunk.0,
                z = chunk.1,
                %error,
                "the column loads without its POI"
            );
        }
        self.poi.forget_disk(chunk);
        self.saved_column_restoration.inputs.insert(chunk, record);
    }

    pub(super) fn discard_saved_column_input(&mut self, chunk: (i32, i32)) {
        self.saved_column_restoration.inputs.remove(&chunk);
    }

    pub fn has_saved_column_input(&self, chunk: (i32, i32)) -> bool {
        self.saved_column_restoration.inputs.contains_key(&chunk)
    }

    pub fn take_saved_column_activations(&mut self) -> Vec<Arc<SavedColumnEffects>> {
        std::mem::take(&mut self.saved_column_restoration.activated)
    }

    pub(crate) fn restore_saved_column_input(&mut self, chunk: (i32, i32)) -> SaveResult<bool> {
        let Some(record) = self.saved_column_restoration.inputs.remove(&chunk) else {
            return Ok(false);
        };
        self.restore_runtime_column_record(chunk, record)?;
        Ok(true)
    }

    pub(super) fn restore_runtime_column_record(
        &mut self,
        chunk: (i32, i32),
        record: Java26RuntimeColumn,
    ) -> SaveResult<()> {
        if let Some(error) = &record.poi_error {
            tracing::error!(
                event = "world.stored_poi_unreadable",
                x = chunk.0,
                z = chunk.1,
                %error,
                "the column loads without its POI"
            );
        }
        self.poi.forget_disk(chunk);
        let resources = self
            .saved_column_restoration
            .resources
            .clone()
            .ok_or_else(|| {
                invalid_carvers_region(
                    [chunk.0, chunk.1],
                    "saved column has no restoration resources",
                )
            })?;
        let data = record.chunk.ok_or_else(|| SaveError::InvalidNbt {
            path: None,
            detail: "saved cold column has no terrain record".into(),
        })?;
        let chunk = (data.data.x(), data.data.z());
        let had_saved_ticks =
            !data.data.block_ticks().is_empty() || !data.data.fluid_ticks().is_empty();
        let mut saved = Java26LoadedDimension {
            chunks: BTreeMap::from([(chunk, data)]),
            ..Java26LoadedDimension::new(resources.dimension.clone())
        };
        if let Some(poi) = record.poi {
            saved.poi_chunks.insert(chunk, poi);
        }
        self.restore_saved_columns(
            Some(&saved),
            &resources.registries,
            resources.items.resolver(),
            false,
        )?;
        let effects = SavedColumnEffects {
            chunk,
            had_saved_ticks,
        };
        self.saved_column_restoration
            .activated
            .push(Arc::new(effects));
        Ok(())
    }

    pub fn defer_ordinary_light_restore(&mut self) {
        self.ordinary_light_restoring = false;
    }

    pub(crate) fn restore_saved_columns(
        &mut self,
        saved: Option<&bbb_server_persistence::Java26LoadedDimension>,
        registries: &SynchronizedRegistries,
        resolver: &dyn PersistentItemStackHolderResolver,
        finalize: bool,
    ) -> SaveResult<()> {
        let Some(saved) = saved else {
            return Ok(());
        };
        self.restore_poi(saved.poi_chunks.values().map(|chunk| chunk.data.clone()));
        self.begin_ordinary_light_restore();
        let restored_chunks = saved.chunks.keys().copied().collect::<BTreeSet<_>>();
        let min_quart_y = self.min_y().div_euclid(4);
        let max_quart_y = min_quart_y + self.section_count() as i32 * 4;
        for (&(chunk_x, chunk_z), chunk) in &saved.chunks {
            if !self.hydrate_saved_generation_column(chunk_x, chunk_z, chunk.data.status()) {
                self.try_load_column(chunk_x, chunk_z)?;
            }
            let column = self
                .column_mut(chunk_x, chunk_z)
                .expect("the restored column has a transaction revision");
            column.set_inhabited_time(chunk.data.inhabited_time());
            *column.block_ticks_mut() =
                ChunkTicks::from_saved(chunk.data.block_ticks()).map_err(|error| {
                    SaveError::InvalidNbt {
                        path: None,
                        detail: format!("chunk ({chunk_x},{chunk_z}) block_ticks: {error}"),
                    }
                })?;
            *column.fluid_ticks_mut() =
                ChunkTicks::from_saved(chunk.data.fluid_ticks()).map_err(|error| {
                    SaveError::InvalidNbt {
                        path: None,
                        detail: format!("chunk ({chunk_x},{chunk_z}) fluid_ticks: {error}"),
                    }
                })?;
            let min_section_y = self.min_y().div_euclid(16);
            let mut restored_sections = Vec::new();
            for section in chunk.data.sections() {
                let Some(block_states) = &section.block_states else {
                    continue;
                };
                let Some(index) = usize::try_from(i32::from(section.y) - min_section_y)
                    .ok()
                    .filter(|&index| index < self.section_count())
                else {
                    continue;
                };
                let content_registry = self.content_registry();
                let palette = block_states
                    .palette
                    .iter()
                    .map(|saved| {
                        vanilla_block_state_id_by_name_and_properties(
                            &saved.name,
                            &saved.properties,
                        )
                        .or_else(|| {
                            let spec = content_registry.block_by_resource_id(&saved.name)?;
                            let offset = saved
                                .properties
                                .get(bbb_server_types::block_state_parse::MOD_STATE_OFFSET_PROPERTY)
                                .map_or(Some(0_u16), |raw| raw.parse::<u16>().ok())?;
                            (offset < spec.state_count)
                                .then(|| spec.first_state_id.saturating_add(i32::from(offset)))
                        })
                        .ok_or_else(|| SaveError::InvalidNbt {
                            path: None,
                            detail: format!(
                                "chunk ({chunk_x},{chunk_z}) block state {} {:?} has no registered \
                                 global id",
                                saved.name, saved.properties
                            ),
                        })
                    })
                    .collect::<SaveResult<Vec<_>>>()?;
                let blocks = if palette.len() == 1 {
                    SectionBlocks::Single(palette[0])
                } else {
                    let mut indices = [0_u16; bbb_server_types::chunk_data::SECTION_VOLUME];
                    for (cell, slot) in indices.iter_mut().enumerate() {
                        *slot = block_states
                            .palette_index(cell)
                            .filter(|&entry| entry < palette.len())
                            .and_then(|entry| u16::try_from(entry).ok())
                            .ok_or_else(|| SaveError::InvalidNbt {
                                path: None,
                                detail: format!(
                                    "chunk ({chunk_x},{chunk_z}) section {} palette misses entry {cell}",
                                    section.y
                                ),
                            })?;
                    }
                    SectionBlocks::from_palette_indices(palette, &indices)
                };
                restored_sections.push((index, blocks));
            }
            let column = self
                .column_mut(chunk_x, chunk_z)
                .expect("the restored column has a transaction revision");
            for section in chunk.data.sections() {
                if section.block_states.is_none() {
                    continue;
                }
                let biomes = section.biomes.as_ref().expect(
                    "the validated chunk schema keeps block-state and biome containers paired",
                );
                let section_quart_min_y = i32::from(section.y) * 4;
                let section_in_bounds =
                    section_quart_min_y >= min_quart_y && section_quart_min_y + 4 <= max_quart_y;
                if !section_in_bounds
                    && (section_quart_min_y + 4 <= min_quart_y
                        || section_quart_min_y >= max_quart_y)
                {
                    continue;
                }
                let mut resolved_palette: Vec<Option<i32>> = vec![None; biomes.palette.len()];
                if biomes.palette.len() == 1 && section_in_bounds {
                    let resource_id = &biomes.palette[0];
                    let biome_id = registries
                        .registry_element_index("minecraft:worldgen/biome", resource_id)
                        .ok_or_else(|| SaveError::InvalidNbt {
                            path: None,
                            detail: format!(
                                "chunk ({chunk_x},{chunk_z}) biome {resource_id} is absent from the frozen published registry"
                            ),
                        })?;
                    let section_index = usize::try_from(i32::from(section.y) - min_section_y)
                        .expect("section_in_bounds guarantees valid section index");
                    column
                        .replace_section_single_biome(section_index, biome_id)
                        .map_err(|error| SaveError::InvalidNbt {
                            path: None,
                            detail: format!(
                                "chunk ({chunk_x},{chunk_z}) section {} biome: {error}",
                                section.y
                            ),
                        })?;
                    continue;
                }
                for local_y in 0..4 {
                    let quart_y = section_quart_min_y + local_y;
                    if quart_y < min_quart_y || quart_y >= max_quart_y {
                        continue;
                    }
                    for local_z in 0..4 {
                        for local_x in 0..4 {
                            let cell = ((local_y as usize) << 4)
                                | ((local_z as usize) << 2)
                                | (local_x as usize);
                            let palette_idx = biomes
                                .palette_index(cell)
                                .filter(|&idx| idx < biomes.palette.len())
                                .ok_or_else(|| SaveError::InvalidNbt {
                                    path: None,
                                    detail: format!(
                                        "chunk ({chunk_x},{chunk_z}) section {} biome palette misses entry ({local_x}, {local_y}, {local_z})",
                                        section.y
                                    ),
                                })?;
                            let biome_id = match resolved_palette[palette_idx] {
                                Some(id) => id,
                                None => {
                                    let resource_id = &biomes.palette[palette_idx];
                                    let id = registries
                                        .registry_element_index(
                                            "minecraft:worldgen/biome",
                                            resource_id,
                                        )
                                        .ok_or_else(|| SaveError::InvalidNbt {
                                            path: None,
                                            detail: format!(
                                                "chunk ({chunk_x},{chunk_z}) biome {resource_id} is absent from the frozen published registry"
                                            ),
                                        })?;
                                    resolved_palette[palette_idx] = Some(id);
                                    id
                                }
                            };
                            column
                                .set_noise_biome(local_x, quart_y, local_z, biome_id)
                                .map_err(|error| SaveError::InvalidNbt {
                                    path: None,
                                    detail: format!(
                                        "chunk ({chunk_x},{chunk_z}) section {} biome: {error}",
                                        section.y
                                    ),
                                })?;
                        }
                    }
                }
            }
            self.restore_section_blocks(chunk_x, chunk_z, restored_sections)
                .expect("the restored column is loaded");
            self.restore_chunk_block_entities(chunk, resolver)?;
            self.restore_saved_generation_metadata(&chunk.data)?;
        }
        if finalize {
            self.finish_restored_generation_batch(&restored_chunks)?;
            self.finish_ordinary_light_restore()?;
        } else {
            self.defer_ordinary_light_restore();
        }
        for (&column, saved_chunk) in &saved.chunks {
            let sections = saved_chunk
                .data
                .sections()
                .iter()
                .map(|section| (i32::from(section.y), section.block_states.is_some()))
                .collect::<Vec<_>>();
            self.check_poi_consistency(column, &sections);
        }
        Ok(())
    }

    fn restore_chunk_block_entities(
        &mut self,
        chunk: &bbb_server_persistence::Java26LoadedChunk,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> SaveResult<()> {
        for record in chunk.data.block_entities() {
            let coordinate = |name: &str| match record
                .iter()
                .find(|entry| entry.name == name)
                .map(|entry| &entry.value)
            {
                Some(NbtSummaryValue::Int(value)) => Some(*value),
                _ => None,
            };
            let (Some(x), Some(y), Some(z)) = (coordinate("x"), coordinate("y"), coordinate("z"))
            else {
                continue;
            };
            if (x.div_euclid(16), z.div_euclid(16)) != (chunk.data.x(), chunk.data.z()) {
                continue;
            }
            let pos = BlockPos { x, y, z };
            let Some(block_state_id) = self.get_block_state(x, y, z) else {
                continue;
            };
            let Some(block_entity) =
                BlockEntity::load_static_with_resolver(pos, block_state_id, record, resolver)
                    .map_err(|error| SaveError::InvalidNbt {
                        path: None,
                        detail: error.to_string(),
                    })?
            else {
                if let Some(loaded) = self.chunks.get_mut(&(chunk.data.x(), chunk.data.z())) {
                    loaded
                        .packed_block_entities
                        .insert((x, y, z), record.clone());
                }
                continue;
            };
            self.block_entities_mut().add_and_register(block_entity);
        }
        Ok(())
    }
}
