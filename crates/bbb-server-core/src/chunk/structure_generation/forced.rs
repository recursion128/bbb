//! Holder-specific structure starts for command placement.

use bbb_server_worldgen::noise::structure_pieces::JigsawTemplateAccess;

use super::*;

impl crate::chunk::ChunkStore {
    pub fn generate_forced_structure_start(
        &self,
        structure_id: &str,
        chunk_x: i32,
        chunk_z: i32,
        templates: &dyn JigsawTemplateAccess,
    ) -> SaveResult<NoiseStructureChunkData> {
        match &self.source {
            crate::chunk::ChunkSource::NoiseCarvers(program) => program
                .structure_generation_context()?
                .generate_forced_structure_start(structure_id, chunk_x, chunk_z, templates),
            crate::chunk::ChunkSource::Flat(_) | crate::chunk::ChunkSource::Debug(_) => self
                .ordinary_structures
                .as_ref()
                .ok_or_else(|| {
                    invalid_schema("structure_generation.forced", "frozen resources are absent")
                })?
                .context(&self.source)
                .generate_forced_structure_start(structure_id, chunk_x, chunk_z, templates),
        }
    }
}

impl StructureGenerationContext<'_> {
    fn generate_forced_structure_start(
        &self,
        structure_id: &str,
        chunk_x: i32,
        chunk_z: i32,
        templates: &dyn JigsawTemplateAccess,
    ) -> SaveResult<NoiseStructureChunkData> {
        let path = format!("structure_generation.forced.{structure_id}[{chunk_x},{chunk_z}]");
        let definition = self.plan.structures().get(structure_id).ok_or_else(|| {
            invalid_schema(
                &path,
                "structure definition is absent from the frozen registry",
            )
        })?;
        let starts = NoiseStructureChunkData::from_supplied_starts(chunk_x, chunk_z, [], []);
        let height_access = StructureHeightAccess::new(self);
        let first_occupied_height =
            |x, z, heightmap| height_access.first_free_height(x, z, heightmap);
        let biome_at = |position| self.biome_name_at(position);
        let template_size = |id: &str| {
            templates
                .structure_template(id)
                .map(|template| template.size)
        };

        if definition.jigsaw.is_some() {
            let start = self
                .plan
                .jigsaw_piece_graph_with_templates(
                    templates,
                    structure_id,
                    self.world_seed,
                    chunk_x,
                    chunk_z,
                    self.dimension_shape.min_y,
                    self.dimension_shape.height,
                    &height_access,
                )
                .and_then(|graph| JigsawStructureStart::from_graph(self.plan, graph, 0));
            return Ok(starts.with_jigsaw_starts(start));
        }

        Ok(match definition.type_id.as_str() {
            "minecraft:mineshaft" => {
                starts.with_mineshaft_starts(self.plan.generate_forced_mineshaft_start(
                    structure_id,
                    self.world_seed,
                    chunk_x,
                    chunk_z,
                    self.dimension_shape.min_y,
                    self.sea_level,
                    &height_access,
                    biome_at,
                ))
            }
            "minecraft:buried_treasure" => {
                starts.with_buried_treasure_starts(self.plan.generate_forced_buried_treasure_start(
                    structure_id,
                    self.world_seed,
                    chunk_x,
                    chunk_z,
                    first_occupied_height,
                    biome_at,
                ))
            }
            "minecraft:shipwreck" => {
                starts.with_shipwreck_starts(self.plan.generate_forced_shipwreck_start(
                    structure_id,
                    self.world_seed,
                    chunk_x,
                    chunk_z,
                    first_occupied_height,
                    biome_at,
                    template_size,
                ))
            }
            "minecraft:ocean_ruin" => {
                starts.with_ocean_ruin_starts(self.plan.generate_forced_ocean_ruin_start(
                    structure_id,
                    self.world_seed,
                    chunk_x,
                    chunk_z,
                    first_occupied_height,
                    biome_at,
                    template_size,
                ))
            }
            "minecraft:igloo" => starts.with_igloo_starts(self.plan.generate_forced_igloo_start(
                structure_id,
                self.world_seed,
                chunk_x,
                chunk_z,
                first_occupied_height,
                biome_at,
                template_size,
            )),
            "minecraft:desert_pyramid" | "minecraft:jungle_temple" | "minecraft:swamp_hut" => {
                starts.with_scattered_starts(self.plan.generate_forced_scattered_feature_start(
                    structure_id,
                    self.world_seed,
                    chunk_x,
                    chunk_z,
                    self.sea_level,
                    first_occupied_height,
                    biome_at,
                ))
            }
            "minecraft:woodland_mansion" => starts.with_woodland_mansion_starts(
                self.plan.generate_forced_woodland_mansion_start(
                    structure_id,
                    self.world_seed,
                    chunk_x,
                    chunk_z,
                    first_occupied_height,
                    biome_at,
                    template_size,
                ),
            ),
            "minecraft:stronghold" => {
                starts.with_stronghold_starts(self.plan.generate_forced_stronghold_start(
                    structure_id,
                    self.world_seed,
                    chunk_x,
                    chunk_z,
                    self.sea_level,
                    self.dimension_shape.min_y,
                    biome_at,
                ))
            }
            "minecraft:fortress" => starts.with_fortress_starts(self.plan.fortress_start_for(
                structure_id,
                self.world_seed,
                chunk_x,
                chunk_z,
            )),
            "minecraft:end_city" => {
                starts.with_end_city_starts(self.plan.generate_forced_end_city_start(
                    structure_id,
                    self.world_seed,
                    chunk_x,
                    chunk_z,
                    first_occupied_height,
                    biome_at,
                    template_size,
                ))
            }
            "minecraft:ruined_portal" => {
                let biome_registry = self
                    .startup
                    .registries
                    .registry("minecraft:worldgen/biome")
                    .ok_or_else(|| invalid_schema(&path, "the frozen biome registry is absent"))?;
                let mut column_cache = rustc_hash::FxHashMap::default();
                starts.with_ruined_portal_starts(self.plan.generate_forced_ruined_portal_start(
                    structure_id,
                    self.world_seed,
                    chunk_x,
                    chunk_z,
                    self.sea_level,
                    self.dimension_shape.min_y,
                    first_occupied_height,
                    |x, y, z, heightmap| {
                        self.fresh_column_opaque_cached(&mut column_cache, x, y, z, heightmap)
                    },
                    biome_at,
                    |biome_id, position| {
                        biome_cold_enough_to_snow(
                            biome_registry,
                            &self.startup.registries.worldgen.biome_climates,
                            biome_id,
                            position,
                            self.sea_level,
                        )
                    },
                    template_size,
                ))
            }
            "minecraft:ocean_monument" => {
                let surrounding = required_surrounding_biomes(self.startup).ok_or_else(|| {
                    invalid_schema(
                        &path,
                        "required ocean-monument surrounding biome tag is absent",
                    )
                })?;
                starts.with_ocean_monument_starts(self.plan.generate_forced_ocean_monument_start(
                    structure_id,
                    self.world_seed,
                    chunk_x,
                    chunk_z,
                    self.sea_level,
                    &surrounding,
                    first_occupied_height,
                    biome_at,
                ))
            }
            "minecraft:nether_fossil" => {
                let (min_y, height) = self.generation_bounds();
                let mut column_cache = rustc_hash::FxHashMap::default();
                starts.with_nether_fossil_starts(self.plan.generate_forced_nether_fossil_start(
                    structure_id,
                    self.world_seed,
                    chunk_x,
                    chunk_z,
                    min_y,
                    height,
                    self.sea_level,
                    |x, y, z| self.fresh_column_state_cached(&mut column_cache, x, y, z),
                    |state| {
                        Some(bbb_server_worldgen::multiface::is_face_sturdy(
                            state,
                            bbb_protocol::packets::Direction::Up,
                        ))
                    },
                    biome_at,
                    template_size,
                ))
            }
            type_id => {
                return Err(invalid_schema(
                    &path,
                    format!("unsupported structure codec {type_id}"),
                ))
            }
        })
    }
}
