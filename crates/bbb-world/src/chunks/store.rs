use bbb_protocol::{
    codec::ProtocolError,
    packets::{
        BlockEntityData as ProtocolBlockEntityData, BlockUpdate as ProtocolBlockUpdate,
        ChunksBiomes as ProtocolChunksBiomes, LevelChunkWithLight,
        LightUpdate as ProtocolLightUpdate, SectionBlocksUpdate as ProtocolSectionBlocksUpdate,
        SetChunkCacheCenter as ProtocolSetChunkCacheCenter,
        SetChunkCacheRadius as ProtocolSetChunkCacheRadius,
    },
};

use crate::{
    protocol_block_pos, section_biome_index, section_block_index,
    terrain::{classify_terrain_material, terrain_fluid_state},
    BlockEntityRecord, BlockPos, BlockProbe, ChunkColumn, ChunkPos, ChunkViewState, RegistrySet,
    Result, TerrainBlockCell, TerrainChunkSnapshot, WorldDecodeError, WorldStore,
};

use super::{
    decode_biome_sections, decode_level_chunk_with_light, decode_nbt_payload_summary,
    merge_light_data, sample_terrain_light,
};

impl WorldStore {
    pub fn insert_level_chunk_with_light(
        &mut self,
        packet: LevelChunkWithLight,
    ) -> Result<ChunkPos> {
        let pos = ChunkPos {
            x: packet.x,
            z: packet.z,
        };
        let decoded = decode_level_chunk_with_light(pos, packet.chunk_data, packet.light_data);
        let column = match decoded {
            Ok(column) => column,
            Err(err) => {
                self.record_apply_error("level_chunk_with_light", &err);
                return Err(err);
            }
        };
        self.insert_decoded_chunk(column);
        Ok(pos)
    }

    pub fn insert_decoded_chunk(&mut self, column: ChunkColumn) {
        let pos = column.pos;
        self.first_chunk.get_or_insert(pos);
        self.counters.chunks_received += 1;
        self.counters.chunks_decoded += 1;
        self.counters.sections_decoded += column.sections.len();
        self.counters.block_entities_seen += column.block_entities.len();
        self.counters.light_arrays_seen +=
            column.light.sky_updates.len() + column.light.block_updates.len();
        if let Some(existing) = self.chunks.iter_mut().find(|chunk| chunk.pos == pos) {
            *existing = column;
        } else {
            self.chunks.push(column);
        }
    }

    pub fn apply_block_update(&mut self, update: ProtocolBlockUpdate) -> bool {
        self.counters.block_updates_received += 1;
        let applied =
            self.set_block_state_id(protocol_block_pos(update.pos), update.block_state_id);
        if applied {
            self.counters.block_updates_applied += 1;
        } else {
            self.counters.block_updates_ignored += 1;
        }
        applied
    }

    pub fn apply_section_blocks_update(&mut self, update: ProtocolSectionBlocksUpdate) -> usize {
        let received = update.updates.len();
        self.counters.block_updates_received += received;
        let mut applied = 0;
        for block_update in update.updates {
            if self.set_block_state_id(
                protocol_block_pos(block_update.pos),
                block_update.block_state_id,
            ) {
                applied += 1;
            }
        }
        self.counters.block_updates_applied += applied;
        self.counters.block_updates_ignored += received.saturating_sub(applied);
        applied
    }

    pub fn apply_block_entity_data(&mut self, packet: ProtocolBlockEntityData) -> Result<bool> {
        self.counters.block_entity_updates_received += 1;
        let pos = protocol_block_pos(packet.pos);
        let y = match i16::try_from(pos.y) {
            Ok(y) => y,
            Err(_) => {
                let err = WorldDecodeError::from(ProtocolError::InvalidData(format!(
                    "block entity y {} is out of i16 range",
                    pos.y
                )));
                self.record_apply_error("block_entity_data", &err);
                return Err(err);
            }
        };
        let nbt = match decode_nbt_payload_summary(&packet.raw_nbt) {
            Ok(nbt) => nbt,
            Err(err) => {
                self.record_apply_error("block_entity_data", &err);
                return Err(err);
            }
        };

        let chunk_pos = ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        };
        let record = BlockEntityRecord {
            local_x: pos.x.rem_euclid(16) as u8,
            y,
            local_z: pos.z.rem_euclid(16) as u8,
            type_id: packet.block_entity_type_id,
            nbt,
        };
        let Some(chunk) = self.chunks.iter_mut().find(|chunk| chunk.pos == chunk_pos) else {
            self.counters.block_entity_updates_ignored += 1;
            return Ok(false);
        };
        if let Some(existing) = chunk.block_entities.iter_mut().find(|entity| {
            entity.local_x == record.local_x
                && entity.y == record.y
                && entity.local_z == record.local_z
        }) {
            *existing = record;
        } else {
            chunk.block_entities.push(record);
        }
        self.counters.block_entity_updates_applied += 1;
        Ok(true)
    }

    pub fn apply_light_update(&mut self, update: ProtocolLightUpdate) -> Result<bool> {
        self.counters.light_updates_received += 1;
        let update_light = update.light_data.into();
        let pos = ChunkPos {
            x: update.chunk_x,
            z: update.chunk_z,
        };
        let Some(chunk) = self.chunks.iter_mut().find(|chunk| chunk.pos == pos) else {
            self.counters.light_updates_ignored += 1;
            return Ok(false);
        };

        merge_light_data(&mut chunk.light, update_light);
        self.counters.light_updates_applied += 1;
        Ok(true)
    }

    pub fn apply_biome_update(&mut self, update: ProtocolChunksBiomes) -> Result<usize> {
        let received = update.chunks.len();
        self.counters.biome_updates_received += received;
        let mut replacements = Vec::new();
        let mut ignored = 0;
        for chunk_update in update.chunks {
            let pos = ChunkPos {
                x: chunk_update.pos.x,
                z: chunk_update.pos.z,
            };
            let Some(chunk_index) = self.chunks.iter().position(|chunk| chunk.pos == pos) else {
                ignored += 1;
                continue;
            };
            let section_count = self.chunks[chunk_index].sections.len();
            let biomes = match decode_biome_sections(&chunk_update.raw_biomes, section_count) {
                Ok(biomes) => biomes,
                Err(err) => {
                    self.record_apply_error("chunks_biomes", &err);
                    return Err(err);
                }
            };
            replacements.push((chunk_index, biomes));
        }

        let applied = replacements.len();
        for (chunk_index, biomes) in replacements {
            if let Some(chunk) = self.chunks.get_mut(chunk_index) {
                for (section, biomes) in chunk.sections.iter_mut().zip(biomes) {
                    section.biomes = biomes;
                }
            }
        }
        self.counters.biome_updates_applied += applied;
        self.counters.biome_updates_ignored += ignored;
        Ok(applied)
    }

    pub fn apply_set_chunk_cache_center(
        &mut self,
        update: ProtocolSetChunkCacheCenter,
    ) -> ChunkViewState {
        self.counters.chunk_cache_center_updates_received += 1;
        self.chunk_view.center = Some(ChunkPos {
            x: update.chunk_x,
            z: update.chunk_z,
        });
        self.chunk_view
    }

    pub fn apply_set_chunk_cache_radius(
        &mut self,
        update: ProtocolSetChunkCacheRadius,
    ) -> ChunkViewState {
        self.counters.chunk_cache_radius_updates_received += 1;
        self.chunk_view.radius = Some(update.radius);
        self.chunk_view
    }

    pub fn chunk_view(&self) -> ChunkViewState {
        self.chunk_view
    }

    pub fn chunk_cache_center(&self) -> Option<ChunkPos> {
        self.chunk_view.center
    }

    pub fn first_chunk(&self) -> Option<ChunkPos> {
        self.first_chunk
    }

    pub fn chunk_cache_radius(&self) -> Option<i32> {
        self.chunk_view.radius
    }

    pub fn forget_chunk(&mut self, pos: ChunkPos) -> bool {
        self.counters.chunk_forgets_received += 1;
        let Some(index) = self.chunks.iter().position(|chunk| chunk.pos == pos) else {
            self.counters.chunk_forgets_ignored += 1;
            return false;
        };
        self.chunks.remove(index);
        self.counters.chunks_forgotten += 1;
        true
    }

    pub fn probe_chunk(&self, pos: ChunkPos) -> Option<&ChunkColumn> {
        self.chunks.iter().find(|chunk| chunk.pos == pos)
    }

    pub fn probe_block(&self, pos: BlockPos) -> Option<BlockProbe> {
        if !self.dimension.contains_y(pos.y) {
            return None;
        }

        let chunk = ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        };
        let local_x = pos.x.rem_euclid(16) as u8;
        let local_y = pos.y.rem_euclid(16) as u8;
        let local_z = pos.z.rem_euclid(16) as u8;
        let section_y = pos.y.div_euclid(16);
        let section_index = usize::try_from(section_y - self.dimension.min_section_y()).ok()?;
        let section = self.probe_chunk(chunk)?.sections.get(section_index)?;

        let block_index = section_block_index(local_x, local_y, local_z);
        let block_value = section.block_states.value_at(block_index)?;
        let block_state = self.registries.block_state(block_value.global_id);
        let block_name = block_state.map(|state| state.name.clone());
        let block_properties = block_state
            .map(|state| state.properties.clone())
            .unwrap_or_default();
        let biome_index = section_biome_index(local_x / 4, local_y / 4, local_z / 4);
        let biome_value = section.biomes.value_at(biome_index);

        Some(BlockProbe {
            pos,
            chunk,
            local_x,
            local_y,
            local_z,
            section_y,
            section_index,
            block_state_id: block_value.global_id,
            material: classify_terrain_material(block_name.as_deref()),
            fluid: terrain_fluid_state(block_name.as_deref(), &block_properties),
            block_name,
            block_properties,
            block_palette_kind: section.block_states.palette_kind,
            block_palette_index: block_value.palette_index,
            biome_id: biome_value.map(|value| value.global_id),
            biome_palette_kind: section.biomes.palette_kind,
        })
    }

    pub fn extract_terrain_chunk(&self, pos: ChunkPos) -> Option<TerrainChunkSnapshot> {
        let chunk = self.probe_chunk(pos)?;

        let height = usize::try_from(self.dimension.height).ok()?;
        let mut cells = Vec::with_capacity(16 * height * 16);
        for y_offset in 0..height {
            let y = self.dimension.min_y + y_offset as i32;
            let section_y = y.div_euclid(16);
            let section_index = usize::try_from(section_y - self.dimension.min_section_y()).ok()?;
            let section = chunk.sections.get(section_index)?;
            let local_y = y.rem_euclid(16) as u8;
            for z in 0..16 {
                for x in 0..16 {
                    let block_index = section_block_index(x as u8, local_y, z as u8);
                    let block_value = section.block_states.value_at(block_index)?;
                    let block_state = self.registries.block_state(block_value.global_id);
                    let block_name = block_state.map(|state| state.name.clone());
                    let block_properties = block_state
                        .map(|state| state.properties.clone())
                        .unwrap_or_default();
                    let biome_index = section_biome_index(x as u8 / 4, local_y / 4, z as u8 / 4);
                    let biome_id = section
                        .biomes
                        .value_at(biome_index)
                        .map(|value| value.global_id);
                    cells.push(TerrainBlockCell {
                        block_state_id: block_value.global_id,
                        biome_id,
                        material: classify_terrain_material(block_name.as_deref()),
                        fluid: terrain_fluid_state(block_name.as_deref(), &block_properties),
                        block_name,
                        block_properties,
                        light: sample_terrain_light(&chunk.light, self.dimension, x, y, z),
                    });
                }
            }
        }

        Some(TerrainChunkSnapshot {
            pos,
            min_y: self.dimension.min_y,
            height,
            cells,
        })
    }

    pub fn extract_terrain_chunks(&self) -> Vec<TerrainChunkSnapshot> {
        self.chunks
            .iter()
            .filter_map(|chunk| self.extract_terrain_chunk(chunk.pos))
            .collect()
    }

    pub fn chunk_positions(&self) -> Vec<ChunkPos> {
        self.chunks.iter().map(|chunk| chunk.pos).collect()
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    fn set_block_state_id(&mut self, pos: BlockPos, block_state_id: i32) -> bool {
        if block_state_id < 0 || !self.dimension.contains_y(pos.y) {
            return false;
        }

        let chunk_pos = ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        };
        let local_x = pos.x.rem_euclid(16) as u8;
        let local_y = pos.y.rem_euclid(16) as u8;
        let local_z = pos.z.rem_euclid(16) as u8;
        let section_y = pos.y.div_euclid(16);
        let Some(section_index) = usize::try_from(section_y - self.dimension.min_section_y()).ok()
        else {
            return false;
        };
        let block_index = section_block_index(local_x, local_y, local_z);
        let old_block_state_id = self
            .probe_chunk(chunk_pos)
            .and_then(|chunk| chunk.sections.get(section_index))
            .and_then(|section| section.block_states.value_at(block_index))
            .map(|value| value.global_id);
        let Some(old_block_state_id) = old_block_state_id else {
            return false;
        };

        let old_non_empty = !is_empty_block_state_id(&self.registries, old_block_state_id);
        let new_non_empty = !is_empty_block_state_id(&self.registries, block_state_id);
        let old_fluid = is_fluid_block_state_id(&self.registries, old_block_state_id);
        let new_fluid = is_fluid_block_state_id(&self.registries, block_state_id);

        let Some(section) = self
            .chunks
            .iter_mut()
            .find(|chunk| chunk.pos == chunk_pos)
            .and_then(|chunk| chunk.sections.get_mut(section_index))
        else {
            return false;
        };
        if !section
            .block_states
            .set_value_at(block_index, block_state_id)
        {
            return false;
        }

        apply_counted_delta(
            &mut section.non_empty_block_count,
            old_non_empty,
            new_non_empty,
        );
        apply_counted_delta(&mut section.fluid_count, old_fluid, new_fluid);
        true
    }
}

fn is_empty_block_state_id(registries: &RegistrySet, block_state_id: i32) -> bool {
    matches!(
        registries
            .block_state(block_state_id)
            .map(|state| state.name.as_str()),
        Some("minecraft:air" | "minecraft:cave_air" | "minecraft:void_air")
    )
}

fn is_fluid_block_state_id(registries: &RegistrySet, block_state_id: i32) -> bool {
    registries
        .block_state(block_state_id)
        .is_some_and(|state| terrain_fluid_state(Some(&state.name), &state.properties).is_some())
}

fn apply_counted_delta(count: &mut i16, old_counted: bool, new_counted: bool) {
    match (old_counted, new_counted) {
        (true, false) => *count = count.saturating_sub(1),
        (false, true) => *count = count.saturating_add(1),
        _ => {}
    }
}
