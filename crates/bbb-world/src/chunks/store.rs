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
use std::collections::BTreeMap;

use crate::{
    protocol_block_pos, section_biome_index, section_block_index,
    terrain::{classify_terrain_material, terrain_fluid_state},
    BlockEntityRecord, BlockPos, BlockProbe, ChunkColumn, ChunkPos, ChunkProbeSummaryState,
    ChunkViewState, HeightmapData, RegistrySet, Result, TerrainBlockCell, TerrainChunkSnapshot,
    TerrainLight, TerrainMaterialClass, WorldDecodeError, WorldDimension, WorldStore,
};

use super::{
    decode_biome_sections, decode_level_chunk_with_light, decode_nbt_payload_summary,
    merge_light_data, palette::packed_long_len, sample_terrain_light,
    sign_text::decode_sign_block_entity_text,
};

const VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID: i32 = 4;
const VANILLA_HEIGHTMAP_ENTRY_COUNT: usize = 16 * 16;

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
        let pos = protocol_block_pos(update.pos);
        let applied = if self.update_local_block_prediction_server_state(pos, update.block_state_id)
        {
            self.counters.local_block_predictions_reconciled_by_update += 1;
            true
        } else {
            self.set_block_state_id(pos, update.block_state_id)
        };
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
            let pos = protocol_block_pos(block_update.pos);
            if self.update_local_block_prediction_server_state(pos, block_update.block_state_id) {
                self.counters.local_block_predictions_reconciled_by_update += 1;
                applied += 1;
            } else if self.set_block_state_id(pos, block_update.block_state_id) {
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
        let sign_text = match decode_sign_block_entity_text(&packet.raw_nbt) {
            Ok(text) => text,
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
            sign_text,
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

    pub fn probe_chunk_summary(&self, pos: ChunkPos) -> Option<ChunkProbeSummaryState> {
        self.probe_chunk(pos)
            .map(ChunkProbeSummaryState::from_chunk)
    }

    pub fn sign_text_lines(&self, pos: BlockPos, is_front_text: bool) -> Option<&[String; 4]> {
        let chunk_pos = ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        };
        let local_x = pos.x.rem_euclid(16) as u8;
        let y = i16::try_from(pos.y).ok()?;
        let local_z = pos.z.rem_euclid(16) as u8;
        let sign_text = self
            .probe_chunk(chunk_pos)?
            .block_entities
            .iter()
            .find(|entity| entity.local_x == local_x && entity.y == y && entity.local_z == local_z)?
            .sign_text
            .as_ref()?;
        if is_front_text {
            Some(&sign_text.front)
        } else {
            Some(&sign_text.back)
        }
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

    /// Samples the stored block+sky light at a world block position, mirroring
    /// vanilla `LevelReader.getBrightness(LightLayer, BlockPos)` (the raw stored
    /// nibble per layer). Returns `None` for out-of-world heights and unloaded
    /// chunks so callers can apply a context-appropriate fallback.
    pub fn sample_block_light(&self, pos: BlockPos) -> Option<TerrainLight> {
        if !self.dimension.contains_y(pos.y) {
            return None;
        }
        let chunk_pos = ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        };
        let column = self.probe_chunk(chunk_pos)?;
        let local_x = pos.x.rem_euclid(16) as usize;
        let local_z = pos.z.rem_euclid(16) as usize;
        Some(sample_terrain_light(
            &column.light,
            self.dimension,
            local_x,
            pos.y,
            local_z,
        ))
    }

    /// Samples vanilla `Heightmap.Types.MOTION_BLOCKING` first-available height
    /// from the chunk heightmap sent by `ClientboundLevelChunkWithLightPacket`.
    /// Returns `None` for unloaded chunks, missing heightmaps, or malformed raw
    /// data so callers can fall back to their own slower scan.
    pub fn sample_motion_blocking_height(&self, x: i32, z: i32) -> Option<i32> {
        let chunk_pos = ChunkPos {
            x: x.div_euclid(16),
            z: z.div_euclid(16),
        };
        let local_x = x.rem_euclid(16) as u8;
        let local_z = z.rem_euclid(16) as u8;
        let index = heightmap_index(local_x, local_z);
        let chunk = self.probe_chunk(chunk_pos)?;
        chunk_heightmap_first_available(
            chunk,
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
            self.dimension,
            index,
        )
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

    pub fn predict_local_destroy_block(&mut self, pos: BlockPos, sequence: i32) -> bool {
        let Some(block) = self.probe_block(pos) else {
            self.counters.local_block_predictions_failed += 1;
            return false;
        };
        let server_block_state_id = block.block_state_id;
        let predicted_block_state_id = self.local_destroy_legacy_block_state_id(&block);
        if server_block_state_id == predicted_block_state_id {
            return true;
        }
        if !self.set_block_state_id(pos, predicted_block_state_id) {
            self.counters.local_block_predictions_failed += 1;
            return false;
        }
        self.record_local_block_prediction(
            sequence,
            pos,
            server_block_state_id,
            predicted_block_state_id,
            self.local_player.pose.map(|pose| pose.position),
        );
        true
    }

    pub(crate) fn sync_ended_local_block_predictions(&mut self, sequence: i32) -> usize {
        let predictions = self.take_local_block_predictions_through_sequence(sequence);
        let ended = predictions.len();
        for prediction in &predictions {
            let current = self
                .probe_block(prediction.pos)
                .map(|block| block.block_state_id);
            if current == Some(prediction.server_block_state_id) {
                continue;
            }
            if self.set_block_state_id(prediction.pos, prediction.server_block_state_id) {
                self.snap_local_player_to_prediction_position_if_colliding(*prediction);
                continue;
            } else {
                self.counters.local_block_predictions_failed += 1;
            }
        }
        ended
    }

    fn snap_local_player_to_prediction_position_if_colliding(
        &mut self,
        prediction: crate::LocalBlockPredictionState,
    ) {
        let Some(predicted_position) = prediction.player_position else {
            return;
        };
        let Some(mut pose) = self.local_player.pose else {
            return;
        };
        if !self.local_player_pose_collides_with_block(prediction.pos, pose) {
            return;
        }
        pose.position = predicted_position;
        self.local_player.pose = Some(pose);
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

        let registries = &self.registries;
        let dimension = self.dimension;
        let Some(chunk) = self.chunks.iter_mut().find(|chunk| chunk.pos == chunk_pos) else {
            return false;
        };
        let Some(section) = chunk.sections.get_mut(section_index) else {
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
        update_motion_blocking_heightmap_for_block(
            chunk,
            dimension,
            registries,
            local_x,
            pos.y,
            local_z,
            block_state_id,
        );
        true
    }

    fn local_destroy_legacy_block_state_id(&self, block: &BlockProbe) -> i32 {
        let Some(fluid) = block.fluid else {
            return 0;
        };
        let (name, level) = match fluid.kind {
            crate::TerrainFluidKind::Water => ("minecraft:water", fluid_legacy_level(fluid)),
            crate::TerrainFluidKind::Lava => ("minecraft:lava", fluid_legacy_level(fluid)),
        };
        let mut properties = BTreeMap::new();
        properties.insert("level".to_string(), level.to_string());
        self.registries
            .block_state_id_by_name_and_properties(name, &properties)
            .unwrap_or(0)
    }
}

fn fluid_legacy_level(fluid: crate::TerrainFluidState) -> u8 {
    if fluid.amount >= 8 && !fluid.falling {
        0
    } else {
        8_u8.saturating_sub(fluid.amount.min(8)) + u8::from(fluid.falling) * 8
    }
}

fn chunk_heightmap_first_available(
    chunk: &ChunkColumn,
    kind_id: i32,
    dimension: WorldDimension,
    index: usize,
) -> Option<i32> {
    let bits = heightmap_bits(dimension)?;
    let heightmap = chunk.heightmaps.iter().find(|map| map.kind_id == kind_id)?;
    if !valid_heightmap_data_len(heightmap, bits) {
        return None;
    }
    let raw = read_heightmap_value(&heightmap.data, bits, index)?;
    let raw = i32::try_from(raw).ok()?;
    Some(dimension.min_y + raw)
}

fn update_motion_blocking_heightmap_for_block(
    chunk: &mut ChunkColumn,
    dimension: WorldDimension,
    registries: &RegistrySet,
    local_x: u8,
    y: i32,
    local_z: u8,
    block_state_id: i32,
) {
    let index = heightmap_index(local_x, local_z);
    let Some(bits) = heightmap_bits(dimension) else {
        return;
    };
    let Some(heightmap_index) = chunk
        .heightmaps
        .iter()
        .position(|map| map.kind_id == VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID)
    else {
        return;
    };
    if !valid_heightmap_data_len(&chunk.heightmaps[heightmap_index], bits) {
        return;
    }

    let Some(first_available) = chunk_heightmap_first_available(
        chunk,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
        dimension,
        index,
    ) else {
        return;
    };
    if y <= first_available - 2 {
        return;
    }

    if block_state_id_blocks_motion_or_fluid(registries, block_state_id) {
        if y >= first_available {
            set_heightmap_first_available(
                &mut chunk.heightmaps[heightmap_index],
                dimension,
                bits,
                index,
                y + 1,
            );
        }
    } else if first_available - 1 == y {
        let next_height = ((dimension.min_y)..y)
            .rev()
            .find(|candidate_y| {
                chunk_block_at_blocks_motion_or_fluid(
                    chunk,
                    dimension,
                    registries,
                    local_x,
                    *candidate_y,
                    local_z,
                )
            })
            .map(|candidate_y| candidate_y + 1)
            .unwrap_or(dimension.min_y);
        set_heightmap_first_available(
            &mut chunk.heightmaps[heightmap_index],
            dimension,
            bits,
            index,
            next_height,
        );
    }
}

fn chunk_block_at_blocks_motion_or_fluid(
    chunk: &ChunkColumn,
    dimension: WorldDimension,
    registries: &RegistrySet,
    local_x: u8,
    y: i32,
    local_z: u8,
) -> bool {
    if !dimension.contains_y(y) {
        return false;
    }
    let section_y = y.div_euclid(16);
    let Ok(section_index) = usize::try_from(section_y - dimension.min_section_y()) else {
        return false;
    };
    let Some(section) = chunk.sections.get(section_index) else {
        return false;
    };
    let local_y = y.rem_euclid(16) as u8;
    let block_index = section_block_index(local_x, local_y, local_z);
    let Some(block_value) = section.block_states.value_at(block_index) else {
        return false;
    };
    block_state_id_blocks_motion_or_fluid(registries, block_value.global_id)
}

fn block_state_id_blocks_motion_or_fluid(registries: &RegistrySet, block_state_id: i32) -> bool {
    let Some(block_state) = registries.block_state(block_state_id) else {
        return true;
    };
    let material = classify_terrain_material(Some(block_state.name.as_str()));
    let fluid = terrain_fluid_state(Some(block_state.name.as_str()), &block_state.properties);
    !matches!(
        material,
        TerrainMaterialClass::Empty | TerrainMaterialClass::Invisible
    ) || fluid.is_some()
}

fn set_heightmap_first_available(
    heightmap: &mut HeightmapData,
    dimension: WorldDimension,
    bits: u8,
    index: usize,
    first_available: i32,
) -> bool {
    let raw = first_available - dimension.min_y;
    let Ok(raw) = u64::try_from(raw) else {
        return false;
    };
    set_heightmap_value(&mut heightmap.data, bits, index, raw)
}

fn valid_heightmap_data_len(heightmap: &HeightmapData, bits: u8) -> bool {
    heightmap.data.len() == packed_long_len(VANILLA_HEIGHTMAP_ENTRY_COUNT, usize::from(bits))
}

fn heightmap_index(local_x: u8, local_z: u8) -> usize {
    usize::from(local_x) + usize::from(local_z) * 16
}

fn heightmap_bits(dimension: WorldDimension) -> Option<u8> {
    if dimension.height <= 0 {
        return None;
    }
    let value = u64::try_from(dimension.height).ok()?.checked_add(1)?;
    Some(ceil_log2(value).max(1))
}

fn ceil_log2(value: u64) -> u8 {
    if value <= 1 {
        0
    } else {
        u8::try_from(u64::BITS - (value - 1).leading_zeros()).unwrap_or(u8::MAX)
    }
}

fn read_heightmap_value(packed_data: &[i64], bits_per_entry: u8, index: usize) -> Option<u64> {
    if bits_per_entry == 0 || bits_per_entry > 64 {
        return None;
    }
    let bits = usize::from(bits_per_entry);
    let values_per_long = 64 / bits;
    if values_per_long == 0 {
        return None;
    }
    let cell_index = index / values_per_long;
    let bit_index = (index - cell_index * values_per_long) * bits;
    let cell = *packed_data.get(cell_index)? as u64;
    let mask = if bits == 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    Some((cell >> bit_index) & mask)
}

fn set_heightmap_value(
    packed_data: &mut [i64],
    bits_per_entry: u8,
    index: usize,
    value: u64,
) -> bool {
    if bits_per_entry == 0 || bits_per_entry > 64 {
        return false;
    }
    let bits = usize::from(bits_per_entry);
    let values_per_long = 64 / bits;
    if values_per_long == 0 {
        return false;
    }
    let cell_index = index / values_per_long;
    let bit_index = (index - cell_index * values_per_long) * bits;
    let Some(cell) = packed_data.get_mut(cell_index) else {
        return false;
    };
    let mask = if bits == 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    if value & !mask != 0 {
        return false;
    }
    let raw = *cell as u64;
    *cell = ((raw & !(mask << bit_index)) | (value << bit_index)) as i64;
    true
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
