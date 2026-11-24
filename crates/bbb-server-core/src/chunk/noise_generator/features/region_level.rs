use std::sync::Arc;

use bbb_server_datapack::worldgen::{ConfiguredFeature, PlacedFeature, ProcessorList};

use super::*;
use crate::chunk::Chunk;
use crate::registry_tags::RegistryTagSnapshot;

pub(crate) struct FeatureWorldGenRegion<'a> {
    center_x: i32,
    center_z: i32,
    world_seed: i64,
    biome_zoom_seed: i64,
    sea_level: i32,
    pub(super) generation_height: Option<i32>,
    pub(super) generation_min_y: Option<i32>,
    resources: &'a DatapackStartup,
    tags: Option<&'a RegistryTagSnapshot>,
    plan: Option<&'a NoiseFeaturesPlan>,
    level_random: WorldgenRandom,
    chunks: &'a mut rustc_hash::FxHashMap<(i32, i32), Chunk>,
    heightmaps: &'a mut rustc_hash::FxHashMap<(i32, i32), super::borrowed::FeatureHeightmapQueries>,
    pending_light_changes: &'a mut crate::chunk::PendingLightChangeMap,
    biome_climates: &'a [BiomeClimate],
    poi_changes: &'a mut Vec<crate::chunk::poi::PoiBlockChange>,
    level_block_entities: &'a mut LevelBlockEntities,
    /// `Level.registryAccess()` as the two block-entity seams of generation
    /// read it (`StructureTemplate.java:305-306`, `ProtoChunk.java:277-279`).
    publication: Option<&'a GenerationItemPublication>,
    /// Chunks whose generation payload changed.
    dirty: &'a mut BTreeSet<(i32, i32)>,
    // `Entity.java:246-278` gives every created entity its own random UUID.
    // The deterministic offline allocator stays separate from decoration RNG.
    proto_entity_uuids: ProtoEntityUuidSequence,
    /// Distinct biome ids across the 3x3 neighborhood, invariant across all 11 feature steps.
    cached_neighborhood_biomes: Option<Vec<String>>,
}

impl<'a> FeatureWorldGenRegion<'a> {
    fn write_block(&mut self, pos: BlockPos, state: i32, update_flags: i32, notify: bool) -> bool {
        if !self.ensure_can_write(pos) {
            return false;
        }
        let Some((index, local_x, local_z)) = self.block_index(pos) else {
            return false;
        };
        // ImposterProtoChunk.java:63-83; WorldGenRegion.java:257-296.
        if self.is_read_only(index) {
            return true;
        }
        let chunk = self.chunk_key(index);
        let loaded = self.chunks.get_mut(&chunk).unwrap();
        let Some(previous) = loaded.set_block_state(local_x, pos.y, local_z, state) else {
            return false;
        };
        let maps = match loaded.generation.as_mut().unwrap().features_mut() {
            Some(features) => &mut features.final_heightmaps,
            None => self
                .heightmaps
                .get_mut(&chunk)
                .unwrap()
                .carvers
                .as_mut()
                .unwrap(),
        };
        maps.update_after_set(&loaded.column, local_x, pos.y, local_z, state);
        if previous != state {
            crate::chunk::live_updates::record_light_change(
                self.pending_light_changes,
                (pos.x, pos.y, pos.z),
                previous,
                state,
            );
            self.geometry_changed(chunk);
        }
        if notify {
            if let Some(change) =
                crate::chunk::poi::PoiBlockChange::new([pos.x, pos.y, pos.z], previous, state)
            {
                self.poi_changes.push(change);
            }
            let had_entity = self.block_entity_present(pos);
            // Generation has no level to drop into: the removal plan is
            // discarded, as a chunk-status-time write never spawns entities.
            let tags = match self.tags {
                Some(tags) => tags,
                None => crate::registry_tags::RegistryTagSnapshot::builtin(),
            };
            let _ = remove_block_entity_on_block_change(
                self.block_entities(),
                pos,
                previous,
                state,
                update_flags,
                tags,
            );
            create_block_entity_on_block_change(self.block_entities(), pos, state);
            if had_entity != self.block_entity_present(pos) {
                self.metadata_changed(pos);
            }
        }
        if notify && update_flags & 16 == 0 {
            use bbb_protocol::block_types::{
                VANILLA_BLOCK_BROWN_MUSHROOM_ID, VANILLA_BLOCK_MAGMA_BLOCK_ID,
                VANILLA_BLOCK_RED_MUSHROOM_ID, VANILLA_BLOCK_SOUL_SAND_ID,
            };
            let post = match vanilla_block_state_block_type_id(state) {
                Some(VANILLA_BLOCK_BROWN_MUSHROOM_ID | VANILLA_BLOCK_RED_MUSHROOM_ID) => Some(pos),
                Some(VANILLA_BLOCK_SOUL_SAND_ID | VANILLA_BLOCK_MAGMA_BLOCK_ID) => Some(BlockPos {
                    y: pos.y + 1,
                    ..pos
                }),
                _ => None,
            };
            if let Some(post) = post {
                self.mark_pos_for_postprocessing(post);
            }
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        target: [i32; 2],
        world_seed: i64,
        sea_level: i32,
        resources: &'a DatapackStartup,
        tags: Option<&'a RegistryTagSnapshot>,
        chunks: &'a mut rustc_hash::FxHashMap<(i32, i32), Chunk>,
        block_entities: &'a mut LevelBlockEntities,
        poi_changes: &'a mut Vec<crate::chunk::poi::PoiBlockChange>,
        pending_light_changes: &'a mut crate::chunk::PendingLightChangeMap,
        heightmaps: &'a mut rustc_hash::FxHashMap<
            (i32, i32),
            super::borrowed::FeatureHeightmapQueries,
        >,
        dirty: &'a mut BTreeSet<(i32, i32)>,
        biome_climates: &'a [BiomeClimate],
        publication: Option<&'a GenerationItemPublication>,
    ) -> Self {
        Self {
            center_x: target[0],
            center_z: target[1],
            world_seed,
            biome_zoom_seed: biome_zoom_seed(world_seed),
            sea_level,
            generation_height: None,
            generation_min_y: None,
            resources,
            tags,
            plan: None,
            level_random: worldgen_region_random(world_seed, target[0], target[1]),
            chunks,
            heightmaps,
            pending_light_changes,
            biome_climates,
            poi_changes,
            level_block_entities: block_entities,
            publication,
            dirty,
            proto_entity_uuids: ProtoEntityUuidSequence::new(world_seed, target[0], target[1]),
            cached_neighborhood_biomes: None,
        }
    }

    fn block_entities(&mut self) -> BlockEntitiesMut<'_> {
        BlockEntitiesMut::new(self.chunks, self.level_block_entities)
    }

    /// Applies `edit` to the region's block entity at `pos`, publishing the
    /// cell's metadata when it applied.
    fn edit_block_entity(
        &mut self,
        pos: BlockPos,
        edit: impl FnOnce(&mut BlockEntity) -> Option<()>,
    ) -> bool {
        let Some((index, _, _)) = self.block_index(pos) else {
            return false;
        };
        if self.is_read_only(index) {
            return false;
        }
        let applied = self.block_entities().get_mut(pos).and_then(edit).is_some();
        if applied {
            self.metadata_changed(pos);
        }
        applied
    }

    pub(crate) fn target_chunk(&self) -> [i32; 2] {
        [self.center_x, self.center_z]
    }

    fn chunk_key(&self, index: usize) -> (i32, i32) {
        (
            self.center_x + index as i32 % 3 - 1,
            self.center_z + index as i32 / 3 - 1,
        )
    }

    fn column(&self, index: usize) -> &ChunkColumn {
        &self.chunks[&self.chunk_key(index)].column
    }

    fn state(&self, chunk: (i32, i32)) -> &ChunkGenerationState {
        self.chunks[&chunk].generation.as_ref().unwrap()
    }

    fn state_mut(&mut self, chunk: (i32, i32)) -> &mut ChunkGenerationState {
        self.chunks
            .get_mut(&chunk)
            .unwrap()
            .generation
            .as_mut()
            .unwrap()
    }

    fn add_proto_entity(&mut self, position: [f64; 3], record: ProtoChunkEntityRecord) {
        self.add_proto_entity_finalized(position, record, false);
    }

    fn add_proto_entity_finalized(
        &mut self,
        position: [f64; 3],
        record: ProtoChunkEntityRecord,
        finalize: bool,
    ) {
        let chunk = (
            (position[0].floor() as i32).div_euclid(16),
            (position[2].floor() as i32).div_euclid(16),
        );
        let mut entries = record.entries();
        if finalize {
            bbb_server_worldgen::noise::proto_chunk_entity::mark_structure_finalize(&mut entries);
        }
        self.chunks
            .get_mut(&chunk)
            .unwrap()
            .proto_entities
            .push(entries);
        self.dirty.insert(chunk);
    }

    fn is_read_only(&self, index: usize) -> bool {
        let chunk = self.chunk_key(index);
        self.chunks[&chunk].relighting_full
            || matches!(
                self.state(chunk),
                ChunkGenerationState::Full { .. } | ChunkGenerationState::NoiseFull(_)
            )
    }

    fn geometry_changed(&mut self, chunk: (i32, i32)) {
        self.dirty.insert(chunk);
        self.state_mut(chunk).invalidate_feature_lighting();
    }

    fn metadata_changed(&mut self, pos: BlockPos) {
        self.dirty
            .insert((pos.x.div_euclid(16), pos.z.div_euclid(16)));
    }

    pub(crate) fn place_feature_step(
        &mut self,
        source: &super::FeaturePlacementSource<'a>,
        step: usize,
    ) -> SaveResult<()> {
        self.plan = Some(source.plan);
        self.generation_min_y = Some(source.generation_min_y);
        self.generation_height = Some(source.generation_depth);
        if self.cached_neighborhood_biomes.is_none() {
            let biomes = neighborhood_biomes(
                &source.resources.registries,
                (0..9).map(|index| self.column(index)),
                "FEATURES biomes",
            )?;
            self.cached_neighborhood_biomes = Some(biomes);
        }
        let biomes = self
            .cached_neighborhood_biomes
            .take()
            .expect("neighborhood biomes populated above");
        let refs = biomes.iter().map(String::as_str).collect::<Vec<_>>();
        let result = source
            .plan
            .place_step_for_chunk(
                source.world_seed,
                self.center_x,
                self.center_z,
                source.dimension_min_y,
                step,
                &refs,
                self,
            )
            .map_err(|error| invalid_schema("FEATURES placement", error.to_string()));
        self.cached_neighborhood_biomes = Some(biomes);
        result?;
        Ok(())
    }

    pub(crate) fn prepare_structure_placement(&mut self) {
        self.plan = None;
        self.generation_min_y = None;
        self.generation_height = None;
    }

    pub(crate) fn generate_blending_border_ticks(
        &mut self,
        witness: Option<&crate::chunk::blending::BlendingBorderTickWitness>,
        tags: Option<&crate::registry_tags::RegistryTagSnapshot>,
    ) -> SaveResult<()> {
        let Some(witness) = witness else {
            return Ok(());
        };
        let chunk = (self.center_x, self.center_z);
        let loaded = self.chunks.get_mut(&chunk).unwrap();
        let state = loaded.generation.as_mut().unwrap();
        let (positions, maps) = match state {
            ChunkGenerationState::Carvers {
                postprocessing_positions,
                ..
            }
            | ChunkGenerationState::Full {
                postprocessing_positions,
                ..
            } => (
                postprocessing_positions,
                self.heightmaps[&chunk].carvers.as_ref().unwrap(),
            ),
            state => {
                let features = state.features_mut().unwrap();
                (
                    &mut features.postprocessing_positions,
                    &features.final_heightmaps,
                )
            }
        };
        let before = positions.len();
        let result = super::blending::generate_border_ticks(
            chunk,
            &loaded.column,
            positions,
            maps,
            witness,
            tags,
        );
        if positions.len() != before {
            self.geometry_changed(chunk);
        }
        result
    }

    /// Proto-entities land only in writable region chunks; FULL neighbors drop them
    /// (`ImposterProtoChunk.java:63-83`).
    fn writable_entity_block(&self, pos: BlockPos) -> bool {
        self.block_index(pos)
            .is_some_and(|(index, _, _)| !self.is_read_only(index))
    }

    fn writable_entity_position(&self, position: [f64; 3]) -> bool {
        self.writable_entity_block(BlockPos {
            x: position[0].floor() as i32,
            y: position[1].floor() as i32,
            z: position[2].floor() as i32,
        })
    }

    fn next_proto_entity_uuid(&mut self) -> [i32; 4] {
        self.proto_entity_uuids.next_uuid()
    }

    fn block_index(&self, pos: BlockPos) -> Option<(usize, i32, i32)> {
        let chunk_x = pos.x.div_euclid(16);
        let chunk_z = pos.z.div_euclid(16);
        region_index(self.center_x, self.center_z, chunk_x, chunk_z)
            .map(|index| (index, pos.x.rem_euclid(16), pos.z.rem_euclid(16)))
    }

    fn heightmap_value(&self, kind: i32, world_x: i32, world_z: i32) -> i32 {
        let chunk_x = world_x.div_euclid(16);
        let chunk_z = world_z.div_euclid(16);
        let Some(index) = region_index(self.center_x, self.center_z, chunk_x, chunk_z) else {
            return self.get_max_y();
        };
        let local_x = world_x.rem_euclid(16);
        let local_z = world_z.rem_euclid(16);
        let value = match kind {
            VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID | VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID => {
                let queries = &self.heightmaps[&self.chunk_key(index)];
                let values = if kind == VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID {
                    &queries.world_surface_wg
                } else {
                    &queries.ocean_floor_wg
                };
                Some(values[(local_z * 16 + local_x) as usize])
            }
            _ => self
                .state(self.chunk_key(index))
                .features()
                .map(|features| &features.final_heightmaps)
                .or_else(|| self.heightmaps[&self.chunk_key(index)].carvers.as_ref())
                .unwrap()
                .first_available(kind, local_x, local_z),
        };
        value.unwrap_or_else(|| self.get_max_y())
    }

    fn biome_id_at(&self, pos: BlockPos) -> Option<&str> {
        let index = self.biome_registry_index_at(pos)?;
        self.resources
            .registries
            .registry(BIOME_REGISTRY)?
            .elements
            .get(index)
            .map(|element| element.id.as_str())
    }

    fn biome_registry_index_at(&self, pos: BlockPos) -> Option<usize> {
        let (quart_x, quart_y, quart_z) =
            fuzzy_biome_quart_position(self.biome_zoom_seed, pos.x, pos.y, pos.z);
        let chunk_x = quart_x.div_euclid(4);
        let chunk_z = quart_z.div_euclid(4);
        let index = region_index(self.center_x, self.center_z, chunk_x, chunk_z)?;
        let quart_min_y = self.column(index).min_y().div_euclid(4);
        let quart_height = i32::try_from(self.column(index).section_count())
            .expect("a validated dimension section count fits Java int")
            .checked_mul(4)
            .expect("a validated biome height fits Java int");
        let quart_max_y = quart_min_y
            .checked_add(quart_height - 1)
            .expect("a validated biome range fits Java int");
        // `ChunkAccess.getNoiseBiome` clamps vertical quart coordinates before
        // selecting a section (`ChunkAccess.java:427-433`).
        let quart_y = quart_y.clamp(quart_min_y, quart_max_y);
        let raw_id = self.column(index).get_noise_biome(
            quart_x.rem_euclid(4),
            quart_y,
            quart_z.rem_euclid(4),
        )?;
        let index = usize::try_from(raw_id).ok()?;
        if index >= self.biome_climates.len() {
            return None;
        }
        Some(index)
    }

    fn biome_climate_at(&self, pos: BlockPos) -> Option<BiomeClimate> {
        let index = self.biome_registry_index_at(pos)?;
        self.biome_climates.get(index).copied()
    }

    fn cold_enough_to_snow(&self, climate: BiomeClimate, pos: BlockPos) -> bool {
        height_adjusted_temperature(
            climate.temperature,
            climate.modifier,
            pos.x,
            pos.y,
            pos.z,
            self.sea_level,
        ) < 0.15
    }

    /// `SnowLayerBlock.canSurvive` (`SnowLayerBlock.java:77-85`).
    fn snow_can_survive(&self, pos: BlockPos) -> bool {
        let below = self.get_block(BlockPos {
            x: pos.x,
            y: pos.y - 1,
            z: pos.z,
        });
        let Some(block) = vanilla_block_state_block_type_id(below)
            .and_then(bbb_protocol::block_types::vanilla_block_type)
        else {
            return false;
        };
        let in_tag = |tag| self.block_state_in_tag(below, tag);
        if in_tag("minecraft:cannot_support_snow_layer") {
            return false;
        }
        if in_tag("minecraft:support_override_snow_layer") {
            return true;
        }
        if block.id == bbb_protocol::block_types::VANILLA_BLOCK_SNOW_ID
            && bbb_server_types::block_queries::int_property(below, "layers") == Some(8)
        {
            return true;
        }
        let Some(facts) = vanilla_block_state_support_facts(below) else {
            return false;
        };
        if facts.dynamic_shape() {
            return false;
        }
        facts.collision_face_full(VanillaBlockSupportDirection::Up)
    }

    fn generation_block_brightness(&self, _pos: BlockPos) -> i32 {
        // FEATURES precedes INITIALIZE_LIGHT, and a missing block-light layer
        // reads zero (`ChunkPyramid.java:28-37`; `BlockLightSectionStorage.java:16-25`).
        0
    }
}

impl WorldGenLevel for FeatureWorldGenRegion<'_> {
    fn update_shape_at_edge(
        &mut self,
        update_mode: i32,
        shape: &bbb_server_worldgen::tree::voxel_shape::BitSetDiscreteVoxelShape,
        start_x: i32,
        start_y: i32,
        start_z: i32,
    ) {
        crate::worldgen_shape::update_shape_at_edge(
            self,
            update_mode,
            shape,
            start_x,
            start_y,
            start_z,
        )
    }

    fn update_template_shape_at_edge(
        &mut self,
        update_mode: i32,
        shape: &bbb_server_worldgen::tree::voxel_shape::BitSetDiscreteVoxelShape,
        start_x: i32,
        start_y: i32,
        start_z: i32,
    ) {
        crate::worldgen_shape::update_template_shape_at_edge(
            self,
            update_mode,
            shape,
            start_x,
            start_y,
            start_z,
        )
    }

    fn update_from_neighbour_shapes(&mut self, state: i32, pos: BlockPos) -> i32 {
        crate::worldgen_shape::update_from_neighbour_shapes(self, state, pos)
    }

    fn can_survive_at(&self, state: i32, pos: BlockPos) -> bool {
        crate::worldgen_shape::can_survive_at(self, state, pos)
    }

    fn collision_shape_full_block_at(&self, state: i32, pos: BlockPos) -> bool {
        crate::block_behavior::collision_shape_full_block_at(state, pos)
    }

    fn fire_state_for_placement(&self, pos: BlockPos) -> Option<i32> {
        let tags = crate::block_getter::TagLookup(|registry: &str, tag: &str, id: &str| {
            self.registry_tag_contains(registry, tag, id)
        });
        crate::block_behavior::fire::state_for_placement(pos, &tags, &mut |probe| {
            self.get_block(probe)
        })
    }

    /// FEATURES has block-state write radius one (`ChunkPyramid.java:30-34`),
    /// matching `WorldGenRegion.ensureCanWrite` (`WorldGenRegion.java:225-253`).
    fn ensure_can_write(&self, pos: BlockPos) -> bool {
        let chunk_x = pos.x.div_euclid(16);
        let chunk_z = pos.z.div_euclid(16);
        (i64::from(chunk_x) - i64::from(self.center_x)).abs() <= 1
            && (i64::from(chunk_z) - i64::from(self.center_z)).abs() <= 1
    }

    /// World generation exposes no live or ProtoChunk entities through
    /// `EntityGetter` (`WorldGenRegion.java:429-440`).
    fn is_unobstructed(&self, _origin: BlockPos) -> bool {
        true
    }

    /// FEATURES `getChunk(EMPTY)` at chessboard distance 2-8 is a
    /// STRUCTURE_STARTS protochunk (`ChunkPyramid.java:30-34`,
    /// `WorldGenRegion.java:105-151`), so those reads are air.
    fn get_block(&self, pos: BlockPos) -> i32 {
        let Some((index, local_x, local_z)) = self.block_index(pos) else {
            return self.source_air_state();
        };
        self.column(index)
            .get_block_state(local_x, pos.y, local_z)
            .unwrap_or_else(|| self.source_air_state())
    }

    fn block_entity_present(&self, pos: BlockPos) -> bool {
        self.chunks
            .get(&(pos.x >> 4, pos.z >> 4))
            .is_some_and(|chunk| chunk.block_entities.contains_key(&(pos.x, pos.y, pos.z)))
    }

    /// `WorldGenLevel.registryAccess()` tag view: the STATIC and worldgen tags
    /// this generation's configured features were loaded against
    /// (`WorldLoader.java:37-39`).
    fn registry_tag_contains(&self, registry_id: &str, tag_id: &str, element_id: &str) -> bool {
        if let Some(tags) = self.tags {
            return tags.contains(registry_id, tag_id, element_id);
        }
        self.resources
            .tags
            .registry(registry_id)
            .and_then(|registry| registry.tag(tag_id))
            .is_some_and(|members| members.iter().any(|member| member == element_id))
    }

    fn block_state_in_tag(&self, state: i32, tag_id: &str) -> bool {
        if let Some(tags) = self.tags {
            return tags.block_state_in(tag_id, state);
        }
        bbb_protocol::block_state_types::vanilla_block_state_name(state)
            .is_some_and(|name| self.registry_tag_contains("minecraft:block", tag_id, name))
    }

    fn registry_tag_members(&self, registry_id: &str, tag_id: &str) -> Vec<&str> {
        if let Some(tags) = self.tags {
            return tags
                .members(registry_id, tag_id)
                .iter()
                .map(Arc::as_ref)
                .collect();
        }
        self.resources
            .tags
            .registry(registry_id)
            .and_then(|registry| registry.tag(tag_id))
            .map(|members| members.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

    fn set_block(&mut self, pos: BlockPos, state: i32, update_flags: i32) -> bool {
        self.write_block(pos, state, update_flags, true)
    }

    fn set_section_block(&mut self, pos: BlockPos, state: i32) -> bool {
        self.write_block(pos, state, 260, false)
    }

    /// `blockEntity.loadWithComponents(TagValueInput.create(..., level.registryAccess(), blockInfo.nbt))`
    /// (`StructureTemplate.java:305-306`).
    fn load_structure_block_entity(
        &mut self,
        pos: BlockPos,
        state: i32,
        record: &[NbtSummaryEntry],
    ) {
        let Some((index, _, _)) = self.block_index(pos) else {
            return;
        };
        if self.is_read_only(index) {
            return;
        }
        // `getBlockEntity` answers the entity `setBlock` created, so data
        // without an `id` (e.g. an `append_loot` table) still loads into it.
        let publication = self.publication;
        if self.edit_block_entity(pos, |entity| {
            match publication {
                Some(publication) => entity.load_command_data_in_place_with_resolver(
                    state,
                    record,
                    publication.resolver(),
                ),
                None => entity.load_command_data_in_place(state, record),
            }
            .ok()
            .map(|_| ())
        }) {
            return;
        }
        let loaded = match self.publication {
            Some(publication) => {
                BlockEntity::load_static_with_resolver(pos, state, record, publication.resolver())
            }
            None => BlockEntity::load_static(pos, state, record),
        };
        if let Ok(Some(entity)) = loaded {
            self.block_entities().add_and_register(entity);
            self.metadata_changed(pos);
        }
    }

    fn get_min_y(&self) -> i32 {
        self.column(4).min_y()
    }

    fn get_max_y(&self) -> i32 {
        let sections = i32::try_from(self.column(4).section_count())
            .expect("a validated dimension section count fits Java int");
        // `LevelHeightAccessor.java:11-13` defines the maximum as inclusive.
        self.get_min_y()
            .checked_add(
                sections
                    .checked_mul(16)
                    .expect("a validated dimension height fits Java int"),
            )
            .and_then(|exclusive_max| exclusive_max.checked_sub(1))
            .expect("a validated dimension y range fits Java int")
    }

    fn get_min_gen_y(&self) -> i32 {
        self.get_min_y()
            .max(self.generation_min_y.unwrap_or(self.get_min_y()))
    }

    fn get_gen_depth(&self) -> i32 {
        let level_height = self.get_max_y() - self.get_min_y() + 1;
        level_height.min(self.generation_height.unwrap_or(level_height))
    }

    fn get_sea_level(&self) -> i32 {
        self.sea_level
    }

    fn get_raw_brightness(&self, pos: BlockPos, _sky_darkening: i32) -> i32 {
        self.generation_block_brightness(pos)
    }

    /// `Biome.shouldFreeze(level, pos, false)` (`Biome.java:146-170`).
    fn biome_at_should_freeze(&self, biome_pos: BlockPos, pos: BlockPos) -> bool {
        let Some(climate) = self.biome_climate_at(biome_pos) else {
            return false;
        };
        if !self.cold_enough_to_snow(climate, pos)
            || !(self.get_min_y()..=self.get_max_y()).contains(&pos.y)
            || self.generation_block_brightness(pos) >= 10
        {
            return false;
        }
        let state = self.get_block(pos);
        let fluid = block_state_fluid_state(state);
        fluid.is_source()
            && fluid.family() == Some(FluidFamily::Water)
            && vanilla_block_state_block_type_id(state)
                == Some(FluidFamily::Water.liquid_block_type_id())
    }

    /// `Biome.shouldSnow` (`Biome.java:184-197`).
    fn biome_should_snow(&self, pos: BlockPos) -> bool {
        let Some(climate) = self.biome_climate_at(pos) else {
            return false;
        };
        if !climate.has_precipitation
            || !self.cold_enough_to_snow(climate, pos)
            || !(self.get_min_y()..=self.get_max_y()).contains(&pos.y)
            || self.generation_block_brightness(pos) >= 10
        {
            return false;
        }
        let state = self.get_block(pos);
        let is_snow = vanilla_block_state_block_type_id(state)
            == Some(bbb_protocol::block_types::VANILLA_BLOCK_SNOW_ID);
        (block_state_is_air(state) || is_snow) && self.snow_can_survive(pos)
    }

    fn schedule_fluid_tick(&mut self, pos: BlockPos, _delay: i32) {
        let state = self.get_block(pos);
        let fluid = block_state_fluid_state(state);
        if fluid.is_empty() {
            return;
        }
        let Some((index, _, _)) = self.block_index(pos) else {
            return;
        };
        if self.is_read_only(index) {
            return;
        }
        let chunk = self.chunk_key(index);
        if self
            .chunks
            .get_mut(&chunk)
            .unwrap()
            .column
            .fluid_ticks_mut()
            .schedule_for_generation(fluid.tick_type(), pos, TickPriority::Normal)
        {
            self.geometry_changed(chunk);
        }
    }

    fn schedule_block_tick(&mut self, pos: BlockPos, _delay: i32) {
        let state = self.get_block(pos);
        let Some(tick_type) = BlockTickType::of_block_state(state) else {
            return;
        };
        let Some((index, _, _)) = self.block_index(pos) else {
            return;
        };
        if self.is_read_only(index) {
            return;
        }
        let chunk = self.chunk_key(index);
        if self
            .chunks
            .get_mut(&chunk)
            .unwrap()
            .column
            .block_ticks_mut()
            .schedule_for_generation(tick_type, pos, TickPriority::Normal)
        {
            self.geometry_changed(chunk);
        }
    }

    fn mark_pos_for_postprocessing(&mut self, pos: BlockPos) {
        let Some((index, _, _)) = self.block_index(pos) else {
            return;
        };
        if !self.is_read_only(index) && (self.get_min_y()..=self.get_max_y()).contains(&pos.y) {
            let chunk = self.chunk_key(index);
            self.state_mut(chunk)
                .postprocessing_positions_mut()
                .push((pos.x, pos.y, pos.z));
            self.geometry_changed(chunk);
        }
    }

    fn get_world_surface_wg_height(&self, x: i32, z: i32) -> i32 {
        self.heightmap_value(VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID, x, z)
    }

    fn get_ocean_floor_wg_height(&self, x: i32, z: i32) -> i32 {
        self.heightmap_value(VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID, x, z)
    }

    fn get_motion_blocking_no_leaves_height(&self, x: i32, z: i32) -> i32 {
        self.heightmap_value(VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID, x, z)
    }

    fn get_motion_blocking_height(&self, x: i32, z: i32) -> i32 {
        self.heightmap_value(VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, x, z)
    }

    fn get_world_surface_height(&self, x: i32, z: i32) -> i32 {
        self.heightmap_value(VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, x, z)
    }

    fn get_ocean_floor_height(&self, x: i32, z: i32) -> i32 {
        self.heightmap_value(VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID, x, z)
    }

    fn set_brushable_block_loot_table(
        &mut self,
        pos: BlockPos,
        loot_table: &'static str,
        loot_table_seed: i64,
    ) {
        self.edit_block_entity(pos, |entity| {
            let brushable = entity.brushable_mut()?;
            brushable.set_loot_table(NbtSummaryValue::string(loot_table), loot_table_seed);
            Some(())
        });
    }

    fn store_bee_nest_occupant(&mut self, pos: BlockPos, ticks_in_hive: i32) {
        self.edit_block_entity(pos, |entity| {
            entity.beehive_mut()?.store_bee(ticks_in_hive);
            Some(())
        });
    }

    fn configured_feature(&self, id: &str) -> Option<Arc<ConfiguredFeature>> {
        self.resources
            .registries
            .worldgen
            .configured_features
            .get(id)
            .cloned()
    }

    fn placed_feature(&self, id: &str) -> Option<Arc<PlacedFeature>> {
        self.resources
            .registries
            .worldgen
            .placed_features
            .get(id)
            .cloned()
    }

    fn processor_list(&self, id: &str) -> Option<Arc<ProcessorList>> {
        self.resources
            .registries
            .worldgen
            .processor_lists
            .get(id)
            .cloned()
    }

    fn biome_filter_should_place(&self, origin: BlockPos, top_feature: &str) -> bool {
        let Some(plan) = self.plan else {
            return false;
        };
        let Some(biome_id) = self.biome_id_at(origin) else {
            return false;
        };
        plan.biome_contains_top_feature(biome_id, top_feature)
    }

    fn get_seed(&self) -> i64 {
        self.world_seed
    }

    fn add_end_crystal(
        &mut self,
        position: [f64; 3],
        y_rot: f32,
        beam_target: Option<BlockPos>,
        invulnerable: bool,
    ) {
        if !self.writable_entity_position(position) {
            return;
        }
        let uuid = self.next_proto_entity_uuid();
        let Ok(record) = EndCrystalRecord::new(
            position,
            y_rot,
            beam_target.map(|target| [target.x, target.y, target.z]),
            invulnerable,
            uuid,
        ) else {
            return;
        };
        self.add_proto_entity(position, ProtoChunkEntityRecord::EndCrystal(record));
    }

    fn structure_template(&self, resource_id: &str) -> Option<StructureTemplate> {
        loaded_structure_template(self.resources, resource_id)
    }

    fn set_block_entity_loot_table(&mut self, pos: BlockPos, loot_table: &'static str, seed: i64) {
        let table = || NbtSummaryValue::string(loot_table);
        self.edit_block_entity(pos, |entity| {
            if let Some(chest) = entity.chest_mut() {
                chest.set_loot_table(table(), seed);
            } else {
                entity.dispenser_mut()?.set_loot_table(table(), seed);
            }
            Some(())
        });
    }

    fn set_spawner_entity_id(&mut self, pos: BlockPos, entity_type: &'static str) {
        self.edit_block_entity(pos, |entity| {
            entity.spawner_mut()?.set_entity_id(entity_type);
            Some(())
        });
    }

    fn set_end_gateway_exit_position(&mut self, pos: BlockPos, exit: BlockPos, exact: bool) {
        // TheEndGatewayBlockEntity.java:272-276: the exit write also marks its record.
        if self.edit_block_entity(pos, |entity| {
            entity.end_gateway_mut()?.set_exit_position(exit, exact);
            Some(())
        }) {
            self.block_entities().set_changed(pos);
        }
    }

    fn next_level_random_boolean(&mut self) -> bool {
        self.level_random.next_boolean()
    }

    fn next_level_random_int(&mut self, bound: i32) -> i32 {
        self.level_random.next_int(bound)
    }

    fn add_structure_mob(&mut self, resource_id: &str, position: [f64; 3]) {
        let Some(kind) = StructureMobKind::from_resource_id(resource_id) else {
            return;
        };
        if !self.writable_entity_position(position) {
            return;
        }
        let uuid = self.next_proto_entity_uuid();
        let Ok(record) = StructureMobRecord::new(kind, position, uuid) else {
            return;
        };
        self.add_proto_entity_finalized(
            position,
            ProtoChunkEntityRecord::StructureMob(record),
            kind.finalizes_spawn(),
        );
    }

    fn add_structure_item_frame(&mut self, position: BlockPos, facing: Direction, item: &str) {
        if !self.writable_entity_block(position) {
            return;
        }
        let uuid = self.next_proto_entity_uuid();
        let Ok(record) = EndCityItemFrameRecord::new(position, facing, item, uuid) else {
            return;
        };
        self.add_proto_entity(
            record.position(),
            ProtoChunkEntityRecord::EndCityItemFrame(record),
        );
    }

    // StructureTemplate.java:500-502: only a Mob of a finalizing template.
    fn add_template_entity_with_finalize(
        &mut self,
        nbt: &NbtSummaryValue,
        position: [f64; 3],
        rotation: [f32; 2],
        finalize: bool,
        _transform: (u8, bbb_protocol::block_state_types::VanillaBlockStateMirror),
    ) {
        if !self.writable_entity_position(position) {
            return;
        }
        let uuid = self.next_proto_entity_uuid();
        let Some(Ok(record)) = TemplateMobRecord::from_template_nbt(nbt, position, rotation, uuid)
        else {
            return;
        };
        let finalize = finalize && record.kind().is_mob();
        self.add_proto_entity_finalized(
            position,
            ProtoChunkEntityRecord::TemplateMob(record),
            finalize,
        );
    }
}

impl MineshaftPlacementLevel for FeatureWorldGenRegion<'_> {
    fn mineshaft_blocking_biome(&self, pos: BlockPos) -> bool {
        self.biome_id_at(pos).is_some_and(|biome| {
            self.registry_tag_contains(BIOME_REGISTRY, "minecraft:mineshaft_blocking", biome)
        })
    }

    fn add_mineshaft_chest_minecart(&mut self, position: [f64; 3], loot_table_seed: i64) {
        if !self.writable_entity_position(position) {
            return;
        }
        let uuid = self.next_proto_entity_uuid();
        if let Ok(record) = MineshaftChestMinecartRecord::new(position, uuid, loot_table_seed) {
            self.add_proto_entity(position, ProtoChunkEntityRecord::ChestMinecart(record));
        }
    }
}

pub(super) fn loaded_structure_template(
    resources: &DatapackStartup,
    resource_id: &str,
) -> Option<StructureTemplate> {
    resources
        .structures
        .template(resource_id)
        .map(|entry| entry.template.clone())
}

impl FeatureWorldGenRegion<'_> {
    fn source_air_state(&self) -> i32 {
        self.column(4).air_state_id()
    }
}

#[cfg(test)]
mod feature_region_tests {}
