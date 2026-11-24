use bbb_protocol::{
    block_entity_types::{
        VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID, VANILLA_BLOCK_ENTITY_TYPE_SKULL_ID,
        VANILLA_BLOCK_ENTITY_TYPE_TRIAL_SPAWNER_ID, VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID,
    },
    block_state_types::{vanilla_block_state_name, vanilla_block_state_render_facts},
    block_types::{
        vanilla_block_type_for_resource_id, VANILLA_BLOCK_COPPER_CHEST_ID,
        VANILLA_BLOCK_COPPER_GOLEM_STATUE_ID, VANILLA_BLOCK_WAXED_OXIDIZED_COPPER_CHEST_ID,
        VANILLA_BLOCK_WAXED_OXIDIZED_COPPER_GOLEM_STATUE_ID,
    },
    codec::ProtocolError,
    packets::{
        BlockEntityData as ProtocolBlockEntityData, BlockUpdate as ProtocolBlockUpdate,
        ChunksBiomes as ProtocolChunksBiomes, LevelChunkWithLight,
        LightUpdate as ProtocolLightUpdate, SectionBlocksUpdate as ProtocolSectionBlocksUpdate,
        SetChunkCacheCenter as ProtocolSetChunkCacheCenter,
        SetChunkCacheRadius as ProtocolSetChunkCacheRadius,
    },
};
use bbb_render_types::VanillaLightCoords;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::{
    registries::block_states::vanilla_block_state_terrain_facts, section_biome_index,
    section_block_index, BannerPatternsState, BlockEntityRecord, BlockPos, BlockProbe, BlockView,
    ChunkColumn, ChunkPos, ChunkProbeSummaryState, ChunkViewState, DecoratedPotSherdsState, Result,
    SignBlockEntityTextState, TerrainBlockCell, TerrainBlockRenderContext, TerrainChunkSnapshot,
    TerrainChunkSource, TerrainLight, TerrainMaterialClass, VaultConnectionParticleState,
    VaultConnectionParticleTargetState, VaultSharedDataState, WorldDecodeError, WorldDimension,
    WorldStore,
};

use super::{
    decode_biome_sections, decode_block_entity_data_record,
    decode_level_chunk_with_light_with_registry,
    heightmap::{
        chunk_heightmap_first_available, heightmap_index,
        update_motion_blocking_heightmaps_for_block, VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
    },
    merge_light_data, updated_light_section_indices, QueuedLightUpdate,
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
        let decoded = decode_level_chunk_with_light_with_registry(
            pos,
            packet.chunk_data,
            packet.light_data,
            self.dimension,
            &self.content_registry.0,
        );
        let mut column = match decoded {
            Ok(column) => column,
            Err(err) => {
                self.record_apply_error("level_chunk_with_light", &err);
                return Err(err);
            }
        };
        let light = std::mem::take(&mut column.light);
        let light_array_count = light.sky_array_count() + light.block_array_count();
        let stored = self.store_decoded_chunk(column);
        self.counters.light_arrays_seen += light_array_count;
        if stored {
            // Vanilla defers the loaded chunk's terrain dirty marking and its
            // first renderability to the queued light closure; the chunk stays
            // light-disabled until that closure runs.
            self.chunks_pending_initial_light.insert(pos);
            self.queued_light_updates.push_back(QueuedLightUpdate {
                pos,
                light,
                initial_chunk_load: true,
                counted_packet: false,
            });
        }
        Ok(pos)
    }

    pub fn insert_decoded_chunk(&mut self, column: ChunkColumn) {
        let pos = column.pos;
        if self.store_decoded_chunk(column) {
            // Direct callers deliver light inline with the column, so the
            // chunk is renderable immediately (vanilla's applyLightData →
            // setLightEnabled collapsed into the insert).
            self.chunks_pending_initial_light.remove(&pos);
            self.mark_terrain_chunk_and_neighbors_changed(pos);
        }
    }

    fn store_decoded_chunk(&mut self, mut column: ChunkColumn) -> bool {
        let pos = column.pos;
        let has_sky_light = self.dimension_has_sky_light();
        column.light.set_has_sky_light(has_sky_light);
        self.counters.chunks_received += 1;
        self.counters.chunks_decoded += 1;
        self.counters.sections_decoded += column.sections.len();
        self.counters.block_entities_seen += column.block_entities.len();
        self.counters.light_arrays_seen +=
            column.light.sky_array_count() + column.light.block_array_count();
        if !chunk_is_in_view(self.chunk_view, pos) {
            return false;
        }
        self.prune_moving_pistons_in_chunk(pos);
        self.prune_trial_spawner_blocks_in_chunk(pos);
        self.first_chunk.get_or_insert(pos);
        self.discard_queued_light_updates_for_chunk(pos);
        self.discard_local_light_updates_for_chunk(pos);
        self.chunks.push(column.into());
        self.advance_block_entity_render_revision();
        true
    }

    pub fn apply_block_update(&mut self, update: ProtocolBlockUpdate) -> bool {
        self.counters.block_updates_received += 1;
        let pos = update.pos;
        let applied = self.apply_server_verified_block_state(pos, update.block_state_id);
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
            let pos = block_update.pos;
            if block_pos_is_in_section(pos, update.section_x, update.section_y, update.section_z)
                && self.apply_server_verified_block_state(pos, block_update.block_state_id)
            {
                applied += 1;
            }
        }
        self.counters.block_updates_applied += applied;
        self.counters.block_updates_ignored += received.saturating_sub(applied);
        applied
    }

    pub fn apply_block_entity_data(&mut self, packet: ProtocolBlockEntityData) -> Result<bool> {
        self.counters.block_entity_updates_received += 1;
        let pos = packet.pos;
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
        let chunk_pos = ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        };
        let local_x = pos.x.rem_euclid(16) as u8;
        let local_z = pos.z.rem_euclid(16) as u8;
        let Some(record_index) = self.chunks.block_entity_index(pos) else {
            self.counters.block_entity_updates_ignored += 1;
            return Ok(false);
        };
        if self
            .chunks
            .get(chunk_pos)
            .map(|chunk| chunk.block_entities[record_index].type_id)
            != Some(packet.block_entity_type_id)
        {
            self.counters.block_entity_updates_ignored += 1;
            return Ok(false);
        }

        let jigsaw_default_joint = self.jigsaw_default_joint_at(pos);
        let record = match decode_block_entity_data_record(
            local_x,
            y,
            local_z,
            packet.block_entity_type_id,
            packet.raw_nbt,
            jigsaw_default_joint,
        ) {
            Ok(record) => record,
            Err(err) => {
                self.record_apply_error("block_entity_data", &err);
                return Err(err);
            }
        };
        self.chunks
            .replace_block_entity_in_place(chunk_pos, record_index, record);
        self.advance_block_entity_render_revision();
        self.counters.block_entity_updates_applied += 1;
        if packet.block_entity_type_id == VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID {
            self.refresh_command_block_editor_from_authoritative_data(pos);
        }
        Ok(true)
    }

    pub fn queue_light_update(&mut self, update: ProtocolLightUpdate) -> Result<()> {
        let section_count = self.dimension.section_count().saturating_add(2);
        let light = match crate::LightData::try_from_protocol(update.light_data, section_count) {
            Ok(light) => light,
            Err(err) => {
                self.record_apply_error("light_update", &err);
                return Err(err);
            }
        };
        self.counters.light_updates_received += 1;
        self.queued_light_updates.push_back(QueuedLightUpdate {
            pos: ChunkPos {
                x: update.chunk_x,
                z: update.chunk_z,
            },
            light,
            initial_chunk_load: false,
            counted_packet: true,
        });
        Ok(())
    }

    /// Whether the chunk's packet-load light is still queued, i.e. vanilla's
    /// light engine has not yet run `setLightEnabled(pos, true)` for it. The
    /// terrain mesh consumer withholds such a chunk's own mesh (a fresh chunk
    /// stays invisible, a re-sent chunk keeps its previous mesh) while still
    /// serving it as neighbor context for face culling.
    pub fn terrain_chunk_initial_light_pending(&self, pos: ChunkPos) -> bool {
        self.chunks_pending_initial_light.contains(&pos)
    }

    pub fn queued_light_update_count(&self) -> usize {
        self.queued_light_updates.len()
    }

    pub fn poll_queued_light_updates(&mut self) -> usize {
        let queued = self.queued_light_updates.len();
        let budget = if queued < 1_000 {
            10.max(queued / 10)
        } else {
            queued
        };
        let mut applied = 0;
        for _ in 0..budget.min(queued) {
            let Some(update) = self.queued_light_updates.pop_front() else {
                break;
            };
            self.apply_light_data(
                update.pos,
                update.light,
                update.initial_chunk_load,
                update.counted_packet,
            );
            applied += 1;
        }
        applied
    }

    pub(crate) fn clear_queued_light_updates(&mut self) {
        let ignored = self
            .queued_light_updates
            .iter()
            .filter(|update| update.counted_packet)
            .count();
        self.counters.light_updates_ignored += ignored;
        self.queued_light_updates.clear();
        self.local_light_updates.clear();
        self.chunks_pending_initial_light.clear();
    }

    fn apply_light_data(
        &mut self,
        pos: ChunkPos,
        update_light: crate::LightData,
        initial_chunk_load: bool,
        counted_packet: bool,
    ) -> bool {
        let has_sky_light = self.dimension_has_sky_light();
        let Some(light) = self.chunks.light_mut(pos) else {
            if counted_packet {
                self.counters.light_updates_ignored += 1;
            }
            return false;
        };

        let updated_sections = if initial_chunk_load {
            Vec::new()
        } else {
            updated_light_section_indices(&update_light)
        };
        light.set_has_sky_light(has_sky_light);
        merge_light_data(light, update_light);
        if counted_packet {
            self.counters.light_updates_applied += 1;
        }
        // Vanilla's applyLightData ends with setLightEnabled(pos, true), but a
        // standalone incremental LightUpdate must not unblock a chunk whose
        // initial chunk-load light update is still pending.
        if initial_chunk_load {
            self.chunks_pending_initial_light.remove(&pos);
            // Vanilla's chunk-load closure: enableChunkLight →
            // setSectionRangeDirty(x±1, full height, z±1) → onChunkReadyToRender.
            self.mark_terrain_chunk_and_neighbors_changed(pos);
        } else if !updated_sections.is_empty() {
            self.mark_terrain_light_sections_changed(pos, &updated_sections);
        }
        true
    }

    pub fn apply_biome_update(&mut self, update: ProtocolChunksBiomes) -> Result<usize> {
        let received = update.chunks.len();
        self.counters.biome_updates_received += received;
        let mut replacements = Vec::new();
        let mut ignored = 0;
        for chunk_update in update.chunks {
            let pos = chunk_update.pos;
            let Some(chunk) = self.chunks.get(pos) else {
                ignored += 1;
                continue;
            };
            let section_count = chunk.sections.len();
            let biomes = match decode_biome_sections(&chunk_update.raw_biomes, section_count) {
                Ok(biomes) => biomes,
                Err(err) => {
                    self.record_apply_error("chunks_biomes", &err);
                    return Err(err);
                }
            };
            replacements.push((pos, biomes));
        }

        let applied = replacements.len();
        let changed_positions: Vec<_> = replacements.iter().map(|(pos, _)| *pos).collect();
        for (pos, biomes) in replacements {
            if let Some(chunk) = self.chunks.get_mut(pos) {
                for (section, biomes) in chunk.sections.iter_mut().zip(biomes) {
                    if section.biomes != biomes {
                        Arc::make_mut(section).biomes = biomes;
                    }
                }
            }
        }
        for pos in changed_positions {
            self.mark_terrain_chunk_and_neighbors_changed(pos);
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
        self.prune_chunks_outside_view();
        self.advance_terrain_revision();
        self.chunk_view
    }

    pub fn apply_set_chunk_cache_radius(
        &mut self,
        update: ProtocolSetChunkCacheRadius,
    ) -> ChunkViewState {
        self.counters.chunk_cache_radius_updates_received += 1;
        self.chunk_view.radius = Some(update.radius);
        self.prune_chunks_outside_view();
        self.advance_terrain_revision();
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

    fn prune_chunks_outside_view(&mut self) {
        let view = self.chunk_view;
        let removed: Vec<_> = self
            .chunks
            .iter()
            .filter(|chunk| !chunk_is_in_view(view, chunk.pos))
            .map(|chunk| chunk.pos)
            .collect();
        if removed.is_empty() {
            return;
        }
        self.chunks
            .retain(|chunk| chunk_is_in_view(view, chunk.pos));
        for pos in removed {
            self.close_command_block_editor_in_chunk(pos);
            self.prune_moving_pistons_in_chunk(pos);
            self.prune_playing_jukebox_songs_in_chunk(pos);
            self.discard_queued_light_updates_for_chunk(pos);
            self.mark_terrain_chunk_removed(pos);
        }
    }

    pub fn forget_chunk(&mut self, pos: ChunkPos) -> bool {
        self.counters.chunk_forgets_received += 1;
        self.drop_debug_chunk(pos);
        self.discard_queued_light_updates_for_chunk(pos);
        if !self.chunks.remove(pos) {
            self.counters.chunk_forgets_ignored += 1;
            return false;
        }
        self.close_command_block_editor_in_chunk(pos);
        self.prune_moving_pistons_in_chunk(pos);
        self.prune_playing_jukebox_songs_in_chunk(pos);
        self.counters.chunks_forgotten += 1;
        self.mark_terrain_chunk_removed(pos);
        true
    }

    fn discard_queued_light_updates_for_chunk(&mut self, pos: ChunkPos) {
        self.discard_local_light_updates_for_chunk(pos);
        let mut ignored = 0;
        self.queued_light_updates.retain(|update| {
            let keep = update.pos != pos;
            if !keep && update.counted_packet {
                ignored += 1;
            }
            keep
        });
        self.counters.light_updates_ignored += ignored;
        // Vanilla's unload path disables the chunk's light exactly once.
        self.chunks_pending_initial_light.remove(&pos);
    }

    pub fn probe_chunk(&self, pos: ChunkPos) -> Option<&ChunkColumn> {
        self.chunks.get(pos)
    }

    pub fn probe_chunk_summary(&self, pos: ChunkPos) -> Option<ChunkProbeSummaryState> {
        self.probe_chunk(pos)
            .map(ChunkProbeSummaryState::from_chunk)
    }

    /// The plain (style-dropped) sign lines of one face, for the sign edit
    /// screen preload (vanilla `AbstractSignEditScreen` edits raw strings).
    /// Render paths read the styled state via [`Self::sign_text_state_at`].
    pub fn sign_text_lines(&self, pos: BlockPos, is_front_text: bool) -> Option<[String; 4]> {
        Some(
            self.sign_text_state_at(pos)?
                .side(is_front_text)
                .plain_lines(),
        )
    }

    /// The effective plain sign lines selected by the client text-filter
    /// setting. A side without decoded filtered messages uses its raw lines.
    pub fn sign_text_lines_for_filter(
        &self,
        pos: BlockPos,
        is_front_text: bool,
        should_filter: bool,
    ) -> Option<[String; 4]> {
        Some(
            self.sign_text_state_at(pos)?
                .side(is_front_text)
                .plain_lines_for_filter(should_filter),
        )
    }

    /// The stored sign text state at a block position (both faces + waxed),
    /// if a sign block entity record exists there.
    pub fn sign_text_state_at(&self, pos: BlockPos) -> Option<&SignBlockEntityTextState> {
        self.block_entity_record_at(pos)?.sign_text()
    }

    pub fn block_entity_type_id_at(&self, pos: BlockPos) -> Option<i32> {
        self.block_entity_record_at(pos)
            .map(|entity| entity.type_id)
    }

    pub fn block_entity_raw_nbt_at(&self, pos: BlockPos) -> Option<&[u8]> {
        self.block_entity_record_at(pos)
            .map(|entity| entity.raw_nbt.as_slice())
    }

    pub fn skull_note_block_sound_at(&self, pos: BlockPos) -> Option<&str> {
        let entity = self.block_entity_record_at(pos)?;
        (entity.type_id == VANILLA_BLOCK_ENTITY_TYPE_SKULL_ID)
            .then_some(entity.note_block_sound())
            .flatten()
    }

    pub fn vault_shared_data_at(&self, pos: BlockPos) -> Option<&VaultSharedDataState> {
        self.block_entity_record_at(pos)?.vault_shared_data()
    }

    /// The stored decorated pot sherd faces at a block position, if a pot
    /// block entity record with a `sherds` list exists there.
    pub fn decorated_pot_sherds_at(&self, pos: BlockPos) -> Option<&DecoratedPotSherdsState> {
        self.block_entity_record_at(pos)?.decorated_pot_sherds()
    }

    /// The stored banner pattern layers at a block position, if a banner
    /// block entity record with a `patterns` list exists there.
    pub fn banner_patterns_at(&self, pos: BlockPos) -> Option<&BannerPatternsState> {
        self.block_entity_record_at(pos)?.banner_patterns()
    }

    pub fn vault_connection_particle_state(
        &self,
        pos: BlockPos,
    ) -> Option<VaultConnectionParticleState> {
        let record = self.block_entity_record_at(pos)?;
        if record.type_id != VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID {
            return None;
        }
        let shared_data = record.vault_shared_data()?;
        if shared_data.connected_players.is_empty() {
            return None;
        }
        let probe = self.block_view(pos)?;
        if probe.block_name != Some("minecraft:vault") {
            return None;
        }
        let (step_x, step_z) = vault_facing_steps(probe.block_properties.get("facing")?.as_str())?;
        let origin = [
            f64::from(pos.x) + 0.5 + step_x * 0.5,
            f64::from(pos.y) + 1.75,
            f64::from(pos.z) + 0.5 + step_z * 0.5,
        ];
        let targets = shared_data
            .connected_players
            .iter()
            .filter_map(|uuid| {
                let (entity_id, target_position) = self.entities.vault_connection_player_target(
                    *uuid,
                    pos,
                    shared_data.connected_particles_range,
                )?;
                Some(VaultConnectionParticleTargetState {
                    entity_id,
                    uuid: *uuid,
                    target_position,
                })
            })
            .collect::<Vec<_>>();
        if targets.is_empty() {
            return None;
        }
        Some(VaultConnectionParticleState { origin, targets })
    }

    pub(crate) fn block_entity_record_at(&self, pos: BlockPos) -> Option<&BlockEntityRecord> {
        self.chunks.block_entity(pos)
    }

    /// The global block-state id at a loaded position.
    pub fn block_state_id_at(&self, pos: BlockPos) -> Option<i32> {
        if !self.dimension.contains_y(pos.y) {
            return None;
        }
        let section_index =
            usize::try_from(pos.y.div_euclid(16) - self.dimension.min_section_y()).ok()?;
        let section = self.chunks.get(pos.chunk())?.sections.get(section_index)?;
        let block_index = section_block_index(
            pos.x.rem_euclid(16) as u8,
            pos.y.rem_euclid(16) as u8,
            pos.z.rem_euclid(16) as u8,
        );
        Some(section.block_states.value_at(block_index)?.global_id)
    }

    /// [`Self::probe_block`] without the owned name/property copies or the
    /// palette and biome diagnostics.
    pub fn block_view(&self, pos: BlockPos) -> Option<BlockView> {
        Some(BlockView::of_state(pos, self.block_state_id_at(pos)?))
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
        let view = BlockView::of_state(pos, block_value.global_id);
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
            material: view.material,
            fluid: view.fluid,
            block_name: view.block_name.map(str::to_owned),
            block_properties: view.block_properties.clone(),
            block_palette_kind: section.block_states.palette_kind,
            block_palette_index: block_value.palette_index,
            biome_id: biome_value.map(|value| value.global_id),
            biome_palette_kind: section.biomes.palette_kind,
        })
    }

    /// Scans one loaded column downward from `max_y` (or the dimension top) to
    /// find the highest non-air block, including fluids.
    pub fn probe_surface_column(
        &self,
        x: i32,
        z: i32,
        max_y: Option<i32>,
    ) -> super::SurfaceColumnProbe {
        let chunk_pos = ChunkPos {
            x: x.div_euclid(16),
            z: z.div_euclid(16),
        };
        let Some(chunk) = self.probe_chunk(chunk_pos) else {
            return super::SurfaceColumnProbe {
                x,
                z,
                loaded: false,
                top_y: None,
                block_state_id: None,
                block_name: None,
                is_fluid: false,
            };
        };
        let min_y = self.dimension.min_y;
        let dim_max_y = min_y
            .saturating_add(self.dimension.height)
            .saturating_sub(1);
        let effective_max_y = max_y.map_or(dim_max_y, |limit| limit.min(dim_max_y));
        if self.dimension.height <= 0 || effective_max_y < min_y || chunk.sections.is_empty() {
            return super::SurfaceColumnProbe {
                x,
                z,
                loaded: true,
                top_y: None,
                block_state_id: None,
                block_name: None,
                is_fluid: false,
            };
        }

        let local_x = x.rem_euclid(16) as u8;
        let local_z = z.rem_euclid(16) as u8;
        let min_section_y = self.dimension.min_section_y();
        let top_section_index = usize::try_from(effective_max_y.div_euclid(16) - min_section_y)
            .unwrap_or(0)
            .min(chunk.sections.len() - 1);

        for section_index in (0..=top_section_index).rev() {
            let section = &chunk.sections[section_index];
            if section.non_empty_block_count == 0 && section.fluid_count == 0 {
                continue;
            }
            let section_base_y = min_section_y.saturating_add(section_index as i32) * 16;
            let start_local_y = (effective_max_y - section_base_y).clamp(0, 15) as u8;
            let end_local_y = (min_y - section_base_y).clamp(0, 15) as u8;
            for local_y in (end_local_y..=start_local_y).rev() {
                let block_index = section_block_index(local_x, local_y, local_z);
                let Some(block_value) = section.block_states.value_at(block_index) else {
                    continue;
                };
                let y = section_base_y + i32::from(local_y);
                let view = BlockView::of_state(BlockPos { x, y, z }, block_value.global_id);
                if view.material != TerrainMaterialClass::Empty {
                    return super::SurfaceColumnProbe {
                        x,
                        z,
                        loaded: true,
                        top_y: Some(y),
                        block_state_id: Some(view.block_state_id),
                        block_name: view.block_name.map(str::to_owned),
                        is_fluid: view.fluid.is_some(),
                    };
                }
            }
        }

        super::SurfaceColumnProbe {
            x,
            z,
            loaded: true,
            top_y: None,
            block_state_id: None,
            block_name: None,
            is_fluid: false,
        }
    }

    /// Samples Vanilla `BlockState.isCollisionShapeFullBlock` for a loaded block.
    ///
    /// Returns `None` when the block or its registry state is unavailable so
    /// local visual consumers can suppress only the affected effect.
    pub fn sample_block_collision_shape_is_full_block(&self, pos: BlockPos) -> Option<bool> {
        let probe = self.block_view(pos)?;
        probe
            .block_name
            .as_ref()
            .map(|_| crate::client::block_collision_shape_is_full_block(&probe, pos))
    }

    /// The stored biome holder at a loaded block position, without block diagnostics.
    pub fn biome_id_at(&self, pos: BlockPos) -> Option<i32> {
        if !self.dimension.contains_y(pos.y) {
            return None;
        }
        self.noise_biome_holder_id_at_quart(
            pos.x.div_euclid(4),
            pos.y.div_euclid(4),
            pos.z.div_euclid(4),
        )
    }

    /// Mirrors `ChunkAccess.getNoiseBiome`: horizontal quart coordinates
    /// select a loaded chunk and 4x4x4 biome cell, while the vertical quart
    /// coordinate is clamped to the chunk's build-height range.
    pub(crate) fn noise_biome_holder_id_at_quart(
        &self,
        quart_x: i32,
        quart_y: i32,
        quart_z: i32,
    ) -> Option<i32> {
        let (section_index, local_qy) = self.clamped_biome_section_and_local_y(quart_y)?;
        let chunk = self.probe_chunk(ChunkPos {
            x: quart_x.div_euclid(4),
            z: quart_z.div_euclid(4),
        })?;
        Self::noise_biome_holder_id_in_chunk(
            chunk,
            section_index,
            quart_x.rem_euclid(4) as u8,
            local_qy,
            quart_z.rem_euclid(4) as u8,
        )
    }

    pub(crate) fn clamped_biome_section_and_local_y(&self, quart_y: i32) -> Option<(usize, u8)> {
        let quart_height = self.dimension.height.div_euclid(4);
        if quart_height <= 0 {
            return None;
        }
        let min_quart_y = self.dimension.min_y.div_euclid(4);
        let max_quart_y = min_quart_y.checked_add(quart_height - 1)?;
        let clamped_quart_y = quart_y.clamp(min_quart_y, max_quart_y);
        let section_y = clamped_quart_y.div_euclid(4);
        let section_index = usize::try_from(section_y - self.dimension.min_section_y()).ok()?;
        Some((section_index, clamped_quart_y.rem_euclid(4) as u8))
    }

    pub(crate) fn noise_biome_holder_id_in_chunk(
        chunk: &ChunkColumn,
        section_index: usize,
        local_qx: u8,
        local_qy: u8,
        local_qz: u8,
    ) -> Option<i32> {
        let section = chunk.sections.get(section_index)?;
        let biome_index = section_biome_index(local_qx, local_qy, local_qz);
        section
            .biomes
            .value_at(biome_index)
            .map(|value| value.global_id)
    }

    /// Samples the stored block+sky light at a world block position, mirroring
    /// vanilla `LevelReader.getBrightness(LightLayer, BlockPos)` (the raw stored
    /// nibble per layer). Returns `None` for out-of-world heights and unloaded
    /// chunks so callers can apply a context-appropriate fallback.
    pub fn sample_block_light(&self, pos: BlockPos) -> Option<TerrainLight> {
        if !self.dimension.contains_y(pos.y) {
            return None;
        }
        self.sample_stored_light(pos)
    }

    /// Samples Vanilla `LevelRenderer.getLightCoords(level, pos)` without
    /// collapsing the result into a renderer-specific scalar.
    pub fn sample_render_light(&self, pos: BlockPos) -> Option<VanillaLightCoords> {
        let state_id = self.block_state_id_at(pos)?;
        self.sample_render_light_for_state(state_id, pos)
    }

    /// Samples the explicit-state overload used by `BlockModelLighter` and
    /// brushable block entities. The state supplies emissive/emission facts;
    /// `pos` remains the brightness-getter query coordinate.
    pub fn sample_render_light_for_state(
        &self,
        state_id: i32,
        pos: BlockPos,
    ) -> Option<VanillaLightCoords> {
        let stored = self.sample_block_light(pos)?;
        let facts = vanilla_block_state_render_facts(state_id)?;
        Some(VanillaLightCoords::from_stored(
            stored.block,
            stored.sky,
            facts.light_emission(),
            facts.emissive_rendering(),
        ))
    }

    /// Samples the light engine's one-section vertical padding used by debug
    /// queries. `LightDebugRenderer` can scan below/above the playable build
    /// height while the camera is near an edge; vanilla retains light data for
    /// those padding sections even though block probes reject them.
    pub fn sample_debug_light(&self, pos: BlockPos) -> Option<TerrainLight> {
        let min_y = self.dimension.min_y.saturating_sub(16);
        let max_y = self
            .dimension
            .min_y
            .saturating_add(self.dimension.height)
            .saturating_add(16);
        if pos.y < min_y || pos.y >= max_y {
            return None;
        }
        self.sample_stored_light(pos)
    }

    pub(crate) fn dimension_has_sky_light(&self) -> bool {
        self.level_info().is_none_or(|level| level.has_sky_light())
    }

    fn sample_stored_light(&self, pos: BlockPos) -> Option<TerrainLight> {
        let chunk_pos = ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        };
        let column = self.probe_chunk(chunk_pos)?;
        let local_x = pos.x.rem_euclid(16) as usize;
        let local_z = pos.z.rem_euclid(16) as usize;
        Some(super::light::sample_terrain_light_with_sky(
            &column.light,
            self.dimension,
            self.dimension_has_sky_light(),
            local_x,
            pos.y,
            local_z,
        ))
    }

    /// Mirrors the sky `LightEngine.getDebugData` section type consumed by
    /// `LightDebugRenderer`: `0` has block data, `1` exists only because a
    /// neighbouring section has data, `2` is absent, and dimensions without a
    /// sky light engine report `n/a`.
    pub fn debug_sky_light_section_status(&self, section: [i32; 3]) -> &'static str {
        if self
            .level_info()
            .is_some_and(|level| !level.has_sky_light())
        {
            return "n/a";
        }
        match self.debug_sky_light_section_occupancy(section) {
            [true, true] => "0",
            [false, true] => "1",
            _ => "2",
        }
    }

    /// `LayerLightSectionStorage.SectionType` projected as
    /// `[LIGHT_AND_DATA, LIGHT]` for `LightSectionDebugRenderer`. A dimension
    /// without a sky light engine returns `EMPTY` (`[false, false]`).
    pub fn debug_sky_light_section_occupancy(&self, section: [i32; 3]) -> [bool; 2] {
        if self
            .level_info()
            .is_some_and(|level| !level.has_sky_light())
        {
            return [false, false];
        }
        if self.debug_sky_light_section_has_data(section) {
            return [true, true];
        }
        for offset_z in -1..=1 {
            for offset_y in -1..=1 {
                for offset_x in -1..=1 {
                    if (offset_x, offset_y, offset_z) == (0, 0, 0) {
                        continue;
                    }
                    if self.debug_sky_light_section_has_data([
                        section[0].saturating_add(offset_x),
                        section[1].saturating_add(offset_y),
                        section[2].saturating_add(offset_z),
                    ]) {
                        return [false, true];
                    }
                }
            }
        }
        [false, false]
    }

    /// Whether the sky `LayerLightSectionStorage.SectionState` carries the
    /// `HAS_DATA_BIT` for this section. Exposed separately so volume consumers
    /// can derive all 26-neighbour LIGHT_ONLY states in one bounded pass.
    pub fn debug_sky_light_section_has_data(&self, section: [i32; 3]) -> bool {
        if self
            .level_info()
            .is_some_and(|level| !level.has_sky_light())
        {
            return false;
        }
        let Some(section_index) = section[1]
            .checked_sub(self.dimension.min_section_y())
            .and_then(|index| usize::try_from(index).ok())
        else {
            return false;
        };
        self.probe_chunk(ChunkPos {
            x: section[0],
            z: section[2],
        })
        .and_then(|chunk| chunk.sections.get(section_index))
        .is_some_and(|section| section.non_empty_block_count != 0)
    }

    /// Samples a vanilla heightmap first-available height from the chunk
    /// heightmap sent by `ClientboundLevelChunkWithLightPacket`. Returns
    /// `None` for unloaded chunks, missing heightmaps, or malformed raw data so
    /// callers can fall back to their own slower scan.
    pub fn sample_heightmap_first_available(&self, kind_id: i32, x: i32, z: i32) -> Option<i32> {
        let chunk_pos = ChunkPos {
            x: x.div_euclid(16),
            z: z.div_euclid(16),
        };
        let local_x = x.rem_euclid(16) as u8;
        let local_z = z.rem_euclid(16) as u8;
        let index = heightmap_index(local_x, local_z);
        let chunk = self.probe_chunk(chunk_pos)?;
        chunk_heightmap_first_available(chunk, kind_id, self.dimension, index)
    }

    pub fn sample_motion_blocking_height(&self, x: i32, z: i32) -> Option<i32> {
        self.sample_heightmap_first_available(VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, x, z)
    }

    pub fn extract_terrain_chunk(&self, pos: ChunkPos) -> Option<TerrainChunkSnapshot> {
        let height = usize::try_from(self.dimension.height).ok()?;
        self.extract_terrain_chunk_y_range(pos, self.dimension.min_y, height)
    }

    /// Selects only the vertical span that can affect renderable cells in
    /// `pos` or its immediate chunk neighbourhood.
    ///
    /// Vanilla still treats every loaded section as resident, including empty
    /// sky sections. Native records that residency separately from this CPU
    /// snapshot. Keeping one section above and below the neighbourhood's
    /// non-empty span retains the block/light samples used by face culling and
    /// ambient occlusion without allocating full-height heavy render cells for
    /// hundreds of provably empty sections.
    pub fn terrain_render_section_range(
        &self,
        pos: ChunkPos,
    ) -> Option<std::ops::RangeInclusive<i32>> {
        self.chunks.non_empty_section_span(pos)?;
        let section_count = self.dimension.section_count();
        let mut non_empty_span = None::<(usize, usize)>;
        for (first, last) in chunk_neighborhood(pos)
            .filter_map(|neighbor| self.chunks.non_empty_section_span(neighbor).flatten())
        {
            non_empty_span = Some(
                non_empty_span.map_or((first, last), |(min_first, max_last)| {
                    (min_first.min(first), max_last.max(last))
                }),
            );
        }
        let (first_section, last_section) = non_empty_span.map_or((0, 0), |(first, last)| {
            (first.saturating_sub(1), (last + 1).min(section_count - 1))
        });
        let min_section_y = self.dimension.min_section_y();
        Some(min_section_y + first_section as i32..=min_section_y + last_section as i32)
    }

    pub fn extract_terrain_chunk_source(
        &self,
        pos: ChunkPos,
    ) -> Option<(i32, usize, i32, std::sync::Arc<dyn TerrainChunkSource>)> {
        let column = self.chunks.shared_column(pos)?;
        let range = self.terrain_render_section_range(pos)?;
        let min_y = *range.start() * 16;
        let height = (*range.end() - *range.start() + 1) as usize * 16;
        let source: std::sync::Arc<dyn TerrainChunkSource> = column.clone_arc();
        Some((min_y, height, self.dimension.min_section_y(), source))
    }

    fn extract_terrain_chunk_y_range(
        &self,
        pos: ChunkPos,
        min_y: i32,
        height: usize,
    ) -> Option<TerrainChunkSnapshot> {
        let chunk = self.probe_chunk(pos)?;
        let context = self.terrain_block_render_context();
        let has_sky_light = self.dimension_has_sky_light();
        let mut cells = Vec::with_capacity(16 * height * 16);
        for y_offset in 0..height {
            let y = min_y.checked_add(i32::try_from(y_offset).ok()?)?;
            let section_y = y.div_euclid(16);
            let section_index = usize::try_from(section_y - self.dimension.min_section_y()).ok()?;
            let section = chunk.sections.get(section_index)?;
            let local_y = y.rem_euclid(16) as u8;
            for z in 0..16 {
                for x in 0..16 {
                    let block_index = section_block_index(x as u8, local_y, z as u8);
                    let block_value = section.block_states.value_at(block_index)?;
                    let biome_index = section_biome_index(x as u8 / 4, local_y / 4, z as u8 / 4);
                    let biome_id = section
                        .biomes
                        .value_at(biome_index)
                        .map(|value| value.global_id);
                    let state = context.block_state(block_value.global_id);
                    cells.push(TerrainBlockCell {
                        block_state_id: state.block_state_id,
                        biome_id,
                        material: state.material,
                        solid_render: state.solid_render,
                        can_occlude: state.can_occlude,
                        view_blocking: state.view_blocking,
                        light_dampening: state.light_dampening,
                        shade_brightness: state.shade_brightness,
                        skip_rendering: state.skip_rendering,
                        face_occlusion_shape_indices: state.face_occlusion_shape_indices,
                        fluid_overlay_neighbor: state.fluid_overlay_neighbor,
                        state_light_emission: state.state_light_emission,
                        emissive_rendering: state.emissive_rendering,
                        fluid: state.fluid,
                        block_name: state.block_name.map(str::to_owned),
                        block_properties: state.block_properties.clone(),
                        light: super::light::sample_terrain_light_with_sky(
                            &chunk.light,
                            self.dimension,
                            has_sky_light,
                            x,
                            y,
                            z,
                        ),
                    });
                }
            }
        }

        Some(TerrainChunkSnapshot {
            pos,
            min_y,
            height,
            cells,
        })
    }

    /// The tag-dependent input of every terrain block render state in this
    /// session.
    pub fn terrain_block_render_context(&self) -> TerrainBlockRenderContext {
        TerrainBlockRenderContext::new(
            self.registries.bars_block_types(),
            crate::terrain::terrain_base_state_facts,
        )
    }

    /// Builds a biome sampler for the 3x3 chunk neighbourhood centred on
    /// `base`, pre-resolving the (up to) nine neighbour columns once so the
    /// per-block biome lookups used by biome colour blending
    /// (`ClientLevel.calculateBlockTint`) do not re-scan the chunk list. A
    /// biome-blend window of radius `r < 16` can only reach the immediately
    /// adjacent chunks, so 3x3 always covers it; columns that are not loaded
    /// stay `None`, letting callers honestly truncate the blend window at the
    /// render-distance edge instead of fabricating biome samples.
    pub fn chunk_biome_sampler(&self, base: ChunkPos) -> ChunkBiomeSampler<'_> {
        let mut columns: [Option<&ChunkColumn>; 9] = [None; 9];
        for dz in -1..=1 {
            for dx in -1..=1 {
                let pos = ChunkPos {
                    x: base.x + dx,
                    z: base.z + dz,
                };
                columns[((dz + 1) * 3 + (dx + 1)) as usize] = self.probe_chunk(pos);
            }
        }
        ChunkBiomeSampler {
            columns,
            base,
            dimension: self.dimension,
        }
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

    pub fn chunk_membership_revision(&self) -> u64 {
        self.chunks.membership_revision()
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Monotonic edge for renderer-relevant chunk contents, residency, or
    /// view-selection changes. Native extraction uses this only as a cheap
    /// "anything changed" guard; per-chunk revisions below own invalidation.
    pub fn terrain_revision(&self) -> u64 {
        self.terrain_revision
    }

    /// Revision of the canonical cells/light/biomes extracted for one chunk.
    pub fn terrain_chunk_content_revision(&self, pos: ChunkPos) -> u64 {
        self.terrain_chunk_content_revisions
            .get(&pos)
            .copied()
            .unwrap_or(0)
    }

    /// Revision of the converted render snapshot for one chunk. Its own
    /// canonical content and only neighbor events that affect biome/context
    /// projection participate; a boundary block in another chunk does not
    /// force this chunk's full cell projection to be rebuilt.
    pub fn terrain_chunk_snapshot_revision(&self, pos: ChunkPos) -> u64 {
        let content = self.terrain_chunk_content_revision(pos);
        let context = self
            .terrain_chunk_mesh_revisions
            .get(&pos)
            .map(|revisions| revisions.snapshot_context_revision())
            .unwrap_or(0);
        content.max(context)
    }

    /// Revision of the final mesh for one chunk. Neighbor changes can advance
    /// this without changing the chunk's own extracted cell snapshot because
    /// face culling and biome/light sampling cross chunk edges.
    pub fn terrain_chunk_mesh_revision(&self, pos: ChunkPos) -> u64 {
        self.terrain_chunk_mesh_revisions
            .get(&pos)
            .map(|revisions| revisions.mesh_revision())
            .unwrap_or(0)
    }

    /// Exact per-section mesh revisions in bottom-to-top dimension order.
    /// The iterator borrows one small publication-shared chunk owner and does
    /// not allocate a dense world-wide section table.
    pub fn terrain_chunk_section_mesh_revisions(
        &self,
        pos: ChunkPos,
    ) -> impl ExactSizeIterator<Item = crate::TerrainSectionMeshRevision> + '_ {
        let min_section_y = self.dimension.min_section_y();
        let section_count = self.dimension.section_count();
        let revisions = self.terrain_chunk_mesh_revisions.get(&pos);
        (0..section_count).map(move |section_index| crate::TerrainSectionMeshRevision {
            section_y: min_section_y
                .saturating_add(i32::try_from(section_index).unwrap_or(i32::MAX)),
            revision: revisions
                .map(|revisions| revisions.section_revision(section_index))
                .unwrap_or(0),
            player_changed_revision: revisions
                .map(|revisions| revisions.section_player_changed_revision(section_index))
                .unwrap_or(0),
        })
    }

    pub fn predict_local_destroy_block(&mut self, pos: BlockPos, sequence: i32) -> bool {
        let Some(block) = self.block_view(pos) else {
            self.counters.local_block_predictions_failed += 1;
            return false;
        };
        let old_block_state_id = block.block_state_id;
        let predicted_block_state_id = self.local_destroy_legacy_block_state_id(&block);
        let predicted =
            self.predict_local_player_block_state(pos, predicted_block_state_id, sequence);
        if predicted {
            // Vanilla `MultiPlayerGameMode.destroyBlock` (`MultiPlayerGameMode.java:133`)
            // triggers client-local `LevelEvent(2001)` since `ServerLevel.levelEvent`
            // excludes the breaking player from the broadcast (`ServerLevel.java:1095-1105`).
            self.push_pending_local_level_event(bbb_protocol::packets::LevelEvent {
                event_type: 2001,
                pos,
                data: old_block_state_id,
                global: false,
            });
            self.queue_local_interaction_sound_action(
                crate::LocalInteractionSoundAction::BlockDestroy {
                    pos,
                    block_state_id: old_block_state_id,
                },
            );
        }
        predicted
    }

    pub fn predict_local_block_state(
        &mut self,
        pos: BlockPos,
        predicted_block_state_id: i32,
        sequence: i32,
    ) -> bool {
        self.predict_local_block_states_with_player_flag(
            &[(pos, predicted_block_state_id)],
            sequence,
            false,
        )
    }

    pub fn predict_local_player_block_state(
        &mut self,
        pos: BlockPos,
        predicted_block_state_id: i32,
        sequence: i32,
    ) -> bool {
        self.predict_local_block_states_with_player_flag(
            &[(pos, predicted_block_state_id)],
            sequence,
            true,
        )
    }

    /// Validates the whole interaction before writing, so an unavailable cell
    /// cannot leave half a multi-block placement predicted.
    pub fn predict_local_block_states(
        &mut self,
        updates: &[(BlockPos, i32)],
        sequence: i32,
    ) -> bool {
        self.predict_local_block_states_with_player_flag(updates, sequence, false)
    }

    pub fn predict_local_player_block_states(
        &mut self,
        updates: &[(BlockPos, i32)],
        sequence: i32,
    ) -> bool {
        self.predict_local_block_states_with_player_flag(updates, sequence, true)
    }

    fn predict_local_block_states_with_player_flag(
        &mut self,
        updates: &[(BlockPos, i32)],
        sequence: i32,
        player_changed: bool,
    ) -> bool {
        let mut prepared = Vec::with_capacity(updates.len());
        for (index, &(pos, predicted_block_state_id)) in updates.iter().enumerate() {
            let is_known_state = vanilla_block_state_name(predicted_block_state_id).is_some()
                || (!self.content_registry().is_vanilla_only()
                    && self
                        .content_registry()
                        .block_by_state_id(predicted_block_state_id)
                        .is_some())
                || self
                    .registries
                    .block_state(predicted_block_state_id)
                    .is_some();
            if updates[..index]
                .iter()
                .any(|(previous_pos, _)| *previous_pos == pos)
                || !is_known_state
            {
                self.counters.local_block_predictions_failed += 1;
                return false;
            }
            let Some(block) = self.block_view(pos) else {
                self.counters.local_block_predictions_failed += 1;
                return false;
            };
            prepared.push((pos, block.block_state_id, predicted_block_state_id));
        }

        let player_position = self.local_player.pose.map(|pose| pose.position);
        for (pos, server_block_state_id, predicted_block_state_id) in prepared {
            if server_block_state_id == predicted_block_state_id {
                continue;
            }
            if !self.set_block_state_id_with_player_flag(
                pos,
                predicted_block_state_id,
                player_changed,
            ) {
                self.counters.local_block_predictions_failed += 1;
                return false;
            }
            self.record_local_block_prediction(
                sequence,
                pos,
                server_block_state_id,
                predicted_block_state_id,
                player_position,
            );
        }
        true
    }

    pub(crate) fn sync_ended_local_block_predictions(&mut self, sequence: i32) -> usize {
        let predictions = self.take_local_block_predictions_through_sequence(sequence);
        let ended = predictions.len();
        for prediction in &predictions {
            let current = self
                .block_view(prediction.pos)
                .map(|block| block.block_state_id);
            if current == Some(prediction.server_block_state_id) {
                continue;
            }
            if self.set_block_state_id(prediction.pos, prediction.server_block_state_id) {
                let can_move_player = self.local_prediction_last_teleport_sequence.unwrap_or(-1)
                    < prediction.sequence;
                if can_move_player {
                    self.snap_local_player_to_prediction_position_if_colliding(*prediction);
                }
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

    pub(crate) fn set_block_state_id(&mut self, pos: BlockPos, block_state_id: i32) -> bool {
        self.set_block_state_id_with_player_flag(pos, block_state_id, false)
    }

    fn set_block_state_id_with_player_flag(
        &mut self,
        pos: BlockPos,
        block_state_id: i32,
        player_changed: bool,
    ) -> bool {
        if !self.registries.is_valid_block_state_id(block_state_id)
            || !self.dimension.contains_y(pos.y)
        {
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
        if old_block_state_id == block_state_id {
            // Vanilla LevelChunk.setBlockState returns null for an identical
            // state, so Level.setBlock marks nothing and sends no notification.
            return false;
        }

        let old_non_empty = !is_empty_block_state_id(old_block_state_id);
        let new_non_empty = !is_empty_block_state_id(block_state_id);
        let old_fluid = is_fluid_block_state_id(old_block_state_id);
        let new_fluid = is_fluid_block_state_id(block_state_id);

        let registries = &self.registries;
        let old_block_name = registries.block_state_name(old_block_state_id);
        let new_block_name = registries.block_state_name(block_state_id);
        let block_kind_changed = old_block_name != new_block_name;
        let old_block_entity_type_id =
            registries.block_entity_type_id_for_block_state(old_block_state_id);
        let new_block_entity_type_id =
            registries.block_entity_type_id_for_block_state(block_state_id);
        let keep_changed_block_entity = match (old_block_name, new_block_name) {
            (Some(old), Some(new)) if block_kind_changed => {
                changed_state_keeps_block_entity(old, new)
            }
            _ => false,
        };
        let reset_trial_spawner_cache = block_kind_changed
            && !keep_changed_block_entity
            && old_block_entity_type_id == Some(VANILLA_BLOCK_ENTITY_TYPE_TRIAL_SPAWNER_ID);
        let dimension = self.dimension;
        let Some(chunk) = self.chunks.get_mut_for_block_state_change(chunk_pos) else {
            return false;
        };
        let Some(section) = chunk.sections.get_mut(section_index).map(Arc::make_mut) else {
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
        update_motion_blocking_heightmaps_for_block(
            chunk,
            dimension,
            registries,
            local_x,
            pos.y,
            local_z,
            block_state_id,
        );
        if old_block_entity_type_id.is_some() || new_block_entity_type_id.is_some() {
            self.chunks.reconcile_block_entity_at(
                pos,
                block_kind_changed
                    && old_block_entity_type_id.is_some()
                    && !keep_changed_block_entity,
                new_block_entity_type_id,
            );
        }
        if !self.moving_pistons.is_empty() {
            self.remove_moving_piston_at(pos);
        }
        if reset_trial_spawner_cache && !self.trial_spawner_blocks.is_empty() {
            self.remove_trial_spawner_block_at(pos);
        }
        self.queue_local_light_change(pos, old_block_state_id, block_state_id);
        self.mark_terrain_block_changed(pos, player_changed);
        if old_block_entity_type_id == Some(VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID)
            || new_block_entity_type_id == Some(VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID)
        {
            self.reconcile_command_block_editor_after_block_change(pos, old_block_state_id);
        }
        true
    }

    fn apply_server_verified_block_state(&mut self, pos: BlockPos, block_state_id: i32) -> bool {
        if self.update_local_block_prediction_server_state(pos, block_state_id) {
            self.counters.local_block_predictions_reconciled_by_update += 1;
            true
        } else {
            self.set_block_state_id(pos, block_state_id)
        }
    }

    pub(crate) fn remove_block_entity_record_at(&mut self, pos: BlockPos) {
        self.chunks.remove_block_entity_at(pos);
    }

    fn local_destroy_legacy_block_state_id(&self, block: &BlockView) -> i32 {
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

    pub(crate) fn clear_terrain_revisions(&mut self) {
        self.terrain_chunk_content_revisions.clear();
        self.terrain_chunk_mesh_revisions.clear();
        self.advance_terrain_revision();
    }

    fn advance_terrain_revision(&mut self) -> u64 {
        self.terrain_revision = self.terrain_revision.saturating_add(1);
        self.terrain_revision
    }

    fn mark_terrain_chunk_and_neighbors_changed(&mut self, pos: ChunkPos) {
        let revision = self.advance_terrain_revision();
        self.terrain_chunk_content_revisions.insert(pos, revision);
        let own = self.terrain_chunk_mesh_revisions.entry(pos).or_default();
        own.mark_all_sections(revision);
        for neighbor in chunk_neighborhood(pos).filter(|neighbor| *neighbor != pos) {
            self.mark_terrain_neighbor_changed(neighbor, revision);
        }
    }

    fn mark_terrain_chunk_removed(&mut self, pos: ChunkPos) {
        let revision = self.advance_terrain_revision();
        self.terrain_chunk_content_revisions.remove(&pos);
        self.terrain_chunk_mesh_revisions.remove(&pos);
        for neighbor in chunk_neighborhood(pos).filter(|neighbor| *neighbor != pos) {
            if !chunk_neighborhood(neighbor).any(|column| self.chunks.get(column).is_some()) {
                self.terrain_chunk_mesh_revisions.remove(&neighbor);
                continue;
            }
            self.mark_terrain_neighbor_changed(neighbor, revision);
        }
    }

    fn mark_terrain_neighbor_changed(&mut self, pos: ChunkPos, revision: u64) {
        let revisions = self.terrain_chunk_mesh_revisions.entry(pos).or_default();
        // The compact snapshot's height depends on neighbors. An empty section's
        // mesh and open visibility do not, even when its sampling band changes.
        revisions.mark_snapshot_context(revision);
        if let Some(chunk) = self.chunks.get(pos) {
            for (index, section) in chunk.sections.iter().enumerate() {
                if section.non_empty_block_count != 0 || section.fluid_count != 0 {
                    revisions.mark_section_index(index, chunk.sections.len(), revision);
                }
            }
        } else {
            revisions.mark_all_sections(revision);
        }
    }

    fn mark_terrain_block_changed(&mut self, pos: BlockPos, player_changed: bool) {
        let chunk = ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        };
        let revision = self.advance_terrain_revision();
        self.terrain_chunk_content_revisions.insert(chunk, revision);

        let x_offsets: &[i32] = match pos.x.rem_euclid(16) {
            0 => &[-1, 0],
            15 => &[0, 1],
            _ => &[0],
        };
        let z_offsets: &[i32] = match pos.z.rem_euclid(16) {
            0 => &[-1, 0],
            15 => &[0, 1],
            _ => &[0],
        };
        let min_section_y = self.dimension.min_section_y();
        let section_count = self.dimension.section_count();
        let section_ys = [
            (pos.y - 1).div_euclid(16),
            pos.y.div_euclid(16),
            (pos.y + 1).div_euclid(16),
        ];
        for section_y in section_ys {
            if !self.dimension.contains_section_y(section_y) {
                continue;
            }
            for dz in z_offsets {
                for dx in x_offsets {
                    let revisions = &mut *self
                        .terrain_chunk_mesh_revisions
                        .entry(ChunkPos {
                            x: chunk.x + dx,
                            z: chunk.z + dz,
                        })
                        .or_default();
                    if player_changed {
                        revisions.mark_section_from_player(
                            min_section_y,
                            section_count,
                            section_y,
                            revision,
                        );
                    } else {
                        revisions.mark_section(min_section_y, section_count, section_y, revision);
                    }
                }
            }
        }
    }

    pub(super) fn mark_terrain_light_sections_changed(
        &mut self,
        chunk: ChunkPos,
        light_section_indices: &[usize],
    ) {
        let revision = self.advance_terrain_revision();
        self.terrain_chunk_content_revisions.insert(chunk, revision);
        let min_section_y = self.dimension.min_section_y();
        let section_count = self.dimension.section_count();
        for &light_section_index in light_section_indices {
            let Ok(light_section_offset) = i32::try_from(light_section_index) else {
                continue;
            };
            let light_section_y = min_section_y - 1 + light_section_offset;
            for section_y in light_section_y - 1..=light_section_y + 1 {
                if !self.dimension.contains_section_y(section_y) {
                    continue;
                }
                for dz in -1..=1 {
                    for dx in -1..=1 {
                        let (Some(x), Some(z)) = (chunk.x.checked_add(dx), chunk.z.checked_add(dz))
                        else {
                            continue;
                        };
                        let revisions = &mut *self
                            .terrain_chunk_mesh_revisions
                            .entry(ChunkPos { x, z })
                            .or_default();
                        revisions.mark_section(min_section_y, section_count, section_y, revision);
                    }
                }
            }
        }
    }
}

fn chunk_neighborhood(pos: ChunkPos) -> impl Iterator<Item = ChunkPos> {
    (-1..=1).flat_map(move |dz| {
        (-1..=1).filter_map(move |dx| {
            Some(ChunkPos {
                x: pos.x.checked_add(dx)?,
                z: pos.z.checked_add(dz)?,
            })
        })
    })
}

fn block_pos_is_in_section(pos: BlockPos, section_x: i32, section_y: i32, section_z: i32) -> bool {
    pos.x.div_euclid(16) == section_x
        && pos.y.div_euclid(16) == section_y
        && pos.z.div_euclid(16) == section_z
}

fn changed_state_keeps_block_entity(old_block_name: &str, new_block_name: &str) -> bool {
    let Some(old_block_id) =
        vanilla_block_type_for_resource_id(old_block_name).map(|block| block.id)
    else {
        return false;
    };
    let Some(new_block_id) =
        vanilla_block_type_for_resource_id(new_block_name).map(|block| block.id)
    else {
        return false;
    };
    let is_copper_chest = |block_id| {
        (VANILLA_BLOCK_COPPER_CHEST_ID..=VANILLA_BLOCK_WAXED_OXIDIZED_COPPER_CHEST_ID)
            .contains(&block_id)
    };
    let is_copper_golem_statue = |block_id| {
        (VANILLA_BLOCK_COPPER_GOLEM_STATUE_ID..=VANILLA_BLOCK_WAXED_OXIDIZED_COPPER_GOLEM_STATUE_ID)
            .contains(&block_id)
    };
    let is_command_block = |block_name: &str| {
        matches!(
            block_name,
            "minecraft:command_block"
                | "minecraft:chain_command_block"
                | "minecraft:repeating_command_block"
        )
    };
    (is_copper_chest(old_block_id) && is_copper_chest(new_block_id))
        || (is_copper_golem_statue(old_block_id) && is_copper_golem_statue(new_block_id))
        || (is_command_block(old_block_name) && is_command_block(new_block_name))
}

fn chunk_is_in_view(view: ChunkViewState, pos: ChunkPos) -> bool {
    let (Some(center), Some(view_range)) = (view.center, view.radius) else {
        return true;
    };
    let storage_radius = view_range.max(2).saturating_add(3);
    (i64::from(pos.x) - i64::from(center.x)).abs() <= i64::from(storage_radius)
        && (i64::from(pos.z) - i64::from(center.z)).abs() <= i64::from(storage_radius)
}

fn fluid_legacy_level(fluid: crate::TerrainFluidState) -> u8 {
    if fluid.amount >= 8 && !fluid.falling {
        0
    } else {
        8_u8.saturating_sub(fluid.amount.min(8)) + u8::from(fluid.falling) * 8
    }
}

fn vault_facing_steps(facing: &str) -> Option<(f64, f64)> {
    match facing {
        "north" => Some((0.0, -1.0)),
        "south" => Some((0.0, 1.0)),
        "west" => Some((-1.0, 0.0)),
        "east" => Some((1.0, 0.0)),
        _ => None,
    }
}

fn is_empty_block_state_id(block_state_id: i32) -> bool {
    vanilla_block_state_terrain_facts(block_state_id)
        .is_some_and(|facts| facts.material == TerrainMaterialClass::Empty)
}

fn is_fluid_block_state_id(block_state_id: i32) -> bool {
    vanilla_block_state_terrain_facts(block_state_id).is_some_and(|facts| facts.fluid.is_some())
}

fn apply_counted_delta(count: &mut i16, old_counted: bool, new_counted: bool) {
    match (old_counted, new_counted) {
        (true, false) => *count = count.saturating_sub(1),
        (false, true) => *count = count.saturating_add(1),
        _ => {}
    }
}

/// A cheap biome sampler over the 3x3 chunk neighbourhood around a base chunk,
/// created by [`WorldStore::chunk_biome_sampler`]. Mirrors the per-column biome
/// lookup vanilla `ClientLevel.calculateBlockTint` performs while averaging
/// `ColorResolver` samples across the biome-blend window.
#[derive(Debug, Clone, Copy)]
pub struct ChunkBiomeSampler<'a> {
    columns: [Option<&'a ChunkColumn>; 9],
    base: ChunkPos,
    dimension: WorldDimension,
}

impl ChunkBiomeSampler<'_> {
    /// Returns the biome global id stored at world position `(x, y, z)`, or
    /// `None` when that column is outside the pre-resolved 3x3 neighbourhood,
    /// out of the world's vertical range, or belongs to a chunk that is not
    /// currently loaded. Biomes are stored per 4x4x4 cell, matching vanilla.
    pub fn biome_id_at(&self, x: i32, y: i32, z: i32) -> Option<i32> {
        if !self.dimension.contains_y(y) {
            return None;
        }
        let dx = x.div_euclid(16) - self.base.x;
        let dz = z.div_euclid(16) - self.base.z;
        if !(-1..=1).contains(&dx) || !(-1..=1).contains(&dz) {
            return None;
        }
        let column = self.columns[((dz + 1) * 3 + (dx + 1)) as usize]?;
        let section_y = y.div_euclid(16);
        let section_index = usize::try_from(section_y - self.dimension.min_section_y()).ok()?;
        let section = column.sections.get(section_index)?;
        let local_x = x.rem_euclid(16) as u8;
        let local_y = y.rem_euclid(16) as u8;
        let local_z = z.rem_euclid(16) as u8;
        let biome_index = section_biome_index(local_x / 4, local_y / 4, local_z / 4);
        section
            .biomes
            .value_at(biome_index)
            .map(|value| value.global_id)
    }
}
