//! Shared typed structure starts.

use super::*;

impl StructureGenerationContext<'_> {
    pub(crate) fn generate_jigsaw_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[JigsawStructureStart],
    ) -> JigsawStructureChunkData {
        let startup = self.startup;

        let height_access = StructureHeightAccess::new(self);
        let generated = plan.generate_missing_random_spread_jigsaw_starts(
            startup,
            self.world_seed,
            chunk_x,
            chunk_z,
            self.dimension_shape.min_y,
            self.dimension_shape.height,
            existing_starts,
            &height_access,
            |position| self.biome_name_at(position),
        );
        JigsawStructureChunkData::from_supplied_starts(
            chunk_x,
            chunk_z,
            existing_starts.iter().cloned().chain(generated),
            std::iter::empty::<&JigsawStructureStart>(),
        )
    }

    pub(crate) fn generate_mineshaft_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[MineshaftStructureStart],
    ) -> Vec<MineshaftStructureStart> {
        let height_access = StructureHeightAccess::new(self);
        plan.generate_missing_random_spread_mineshaft_starts(
            self.world_seed,
            chunk_x,
            chunk_z,
            self.dimension_shape.min_y,
            self.sea_level,
            existing_starts,
            &height_access,
            |position| self.biome_name_at(position),
        )
    }

    pub(crate) fn generate_shipwreck_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[ShipwreckStructureStart],
    ) -> Vec<ShipwreckStructureStart> {
        let startup = self.startup;

        plan.generate_missing_random_spread_shipwreck_starts(
            self.world_seed,
            chunk_x,
            chunk_z,
            existing_starts,
            |x, z, heightmap| match heightmap {
                StructureHeightmap::WorldSurfaceWg => {
                    self.fresh_base_height(x, z, VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID)
                }
                StructureHeightmap::OceanFloorWg => {
                    self.fresh_base_height(x, z, VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID)
                }
                _ => None,
            },
            |position| self.biome_name_at(position),
            |template_id| {
                startup
                    .structures
                    .template(template_id)
                    .map(|element| element.template.size)
            },
        )
    }

    pub(crate) fn generate_ocean_ruin_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[OceanRuinStructureStart],
    ) -> Vec<OceanRuinStructureStart> {
        let startup = self.startup;

        plan.generate_missing_random_spread_ocean_ruin_starts(
            self.world_seed,
            chunk_x,
            chunk_z,
            existing_starts,
            |x, z, heightmap| match heightmap {
                StructureHeightmap::OceanFloorWg => {
                    self.fresh_base_height(x, z, VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID)
                }
                _ => None,
            },
            |position| self.biome_name_at(position),
            |template_id| {
                startup
                    .structures
                    .template(template_id)
                    .map(|element| element.template.size)
            },
        )
    }

    pub(crate) fn generate_igloo_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[IglooStructureStart],
    ) -> Vec<IglooStructureStart> {
        let startup = self.startup;

        plan.generate_missing_random_spread_igloo_starts(
            self.world_seed,
            chunk_x,
            chunk_z,
            existing_starts,
            |x, z, heightmap| match heightmap {
                StructureHeightmap::WorldSurfaceWg => {
                    self.fresh_base_height(x, z, VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID)
                }
                _ => None,
            },
            |position| self.biome_name_at(position),
            |template_id| {
                startup
                    .structures
                    .template(template_id)
                    .map(|element| element.template.size)
            },
        )
    }

    pub(crate) fn generate_stronghold_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[StrongholdStructureStart],
        ring_positions: &NoiseStructureRingPositions,
    ) -> Vec<StrongholdStructureStart> {
        let mut current = existing_starts.to_vec();
        for set in plan
            .sets()
            .iter()
            .filter(|set| plan.structure_start_owner(set) == StructureStartSetOwner::Stronghold)
        {
            let Some(positions) = ring_positions.get(&set.id) else {
                continue;
            };
            let rings = positions.iter().copied().collect::<Vec<_>>();
            let generated = plan.generate_missing_concentric_ring_stronghold_starts(
                &set.id,
                self.world_seed,
                chunk_x,
                chunk_z,
                &current,
                &rings,
                self.sea_level,
                self.dimension_shape.min_y,
                |position| self.biome_name_at(position),
            );
            current.extend(generated);
        }
        current.split_off(existing_starts.len())
    }

    pub(crate) fn generate_scattered_feature_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[ScatteredFeatureStructureStart],
    ) -> Vec<ScatteredFeatureStructureStart> {
        plan.generate_missing_random_spread_scattered_feature_starts(
            self.world_seed,
            chunk_x,
            chunk_z,
            existing_starts,
            self.sea_level,
            |x, z, heightmap| match heightmap {
                StructureHeightmap::WorldSurfaceWg => {
                    self.fresh_base_height(x, z, VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID)
                }
                _ => None,
            },
            |position| self.biome_name_at(position),
        )
    }

    pub(crate) fn generate_ruined_portal_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[RuinedPortalStructureStart],
    ) -> Vec<RuinedPortalStructureStart> {
        let startup = self.startup;
        let Some(biome_registry) = startup.registries.registry("minecraft:worldgen/biome") else {
            return Vec::new();
        };
        let mut column_cache = rustc_hash::FxHashMap::default();
        plan.generate_missing_random_spread_ruined_portal_starts(
            self.world_seed,
            chunk_x,
            chunk_z,
            existing_starts,
            self.sea_level,
            self.dimension_shape.min_y,
            |x, z, heightmap| match heightmap {
                StructureHeightmap::WorldSurfaceWg => {
                    self.fresh_base_height(x, z, VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID)
                }
                StructureHeightmap::OceanFloorWg => {
                    self.fresh_base_height(x, z, VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID)
                }
                _ => None,
            },
            |x, y, z, heightmap| {
                self.fresh_column_opaque_cached(&mut column_cache, x, y, z, heightmap)
            },
            |position| self.biome_name_at(position),
            |biome_id, position| {
                biome_cold_enough_to_snow(
                    biome_registry,
                    &startup.registries.worldgen.biome_climates,
                    biome_id,
                    position,
                    self.sea_level,
                )
            },
            |template_id| {
                startup
                    .structures
                    .template(template_id)
                    .map(|element| element.template.size)
            },
        )
    }

    pub(crate) fn generate_ocean_monument_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[OceanMonumentStructureStart],
    ) -> Vec<OceanMonumentStructureStart> {
        let Some(surrounding) = required_surrounding_biomes(self.startup) else {
            return Vec::new();
        };
        plan.generate_missing_random_spread_ocean_monument_starts(
            self.world_seed,
            chunk_x,
            chunk_z,
            existing_starts,
            self.sea_level,
            &surrounding,
            |x, z, heightmap| match heightmap {
                StructureHeightmap::OceanFloorWg => {
                    self.fresh_base_height(x, z, VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID)
                }
                _ => None,
            },
            |position| self.biome_name_at(position),
        )
    }

    pub(crate) fn generate_woodland_mansion_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[WoodlandMansionStructureStart],
    ) -> Vec<WoodlandMansionStructureStart> {
        let startup = self.startup;

        plan.generate_missing_random_spread_woodland_mansion_starts(
            self.world_seed,
            chunk_x,
            chunk_z,
            existing_starts,
            |x, z, heightmap| match heightmap {
                StructureHeightmap::WorldSurfaceWg => {
                    self.fresh_base_height(x, z, VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID)
                }
                _ => None,
            },
            |position| self.biome_name_at(position),
            |template_id| {
                startup
                    .structures
                    .template(template_id)
                    .map(|element| element.template.size)
            },
        )
    }

    pub(crate) fn generate_buried_treasure_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[BuriedTreasureStructureStart],
    ) -> Vec<BuriedTreasureStructureStart> {
        plan.generate_missing_random_spread_buried_treasure_stubs(
            self.world_seed,
            chunk_x,
            chunk_z,
            existing_starts,
            |x, z, heightmap| match heightmap {
                StructureHeightmap::OceanFloorWg => {
                    self.fresh_base_height(x, z, VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID)
                }
                _ => None,
            },
            |position| self.biome_name_at(position),
        )
        .into_iter()
        .map(|stub| BuriedTreasureStructureStart::from_generation_stub(stub, 0))
        .collect()
    }

    pub(crate) fn generate_nether_complex_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_jigsaw: &[JigsawStructureStart],
        existing_fortress: &[FortressStructureStart],
    ) -> (Vec<JigsawStructureStart>, Vec<FortressStructureStart>) {
        let startup = self.startup;
        let owned_sets = plan
            .sets()
            .iter()
            .filter(|set| plan.structure_start_owner(set) == StructureStartSetOwner::NetherComplex);
        let mut jigsaw = existing_jigsaw.to_vec();
        let mut fortress = existing_fortress.to_vec();
        let height_access = StructureHeightAccess::new(self);
        for set in owned_sets {
            if set.structures.iter().any(|selection| {
                jigsaw
                    .iter()
                    .any(|start| start.structure_id() == selection.structure)
                    || fortress
                        .iter()
                        .any(|start| start.structure_id() == selection.structure)
            }) {
                continue;
            }
            let Some(attempts) =
                plan.random_spread_attempt_order(&set.id, self.world_seed, chunk_x, chunk_z)
            else {
                continue;
            };
            for structure_id in attempts {
                let Some(definition) = plan.structures().get(&structure_id) else {
                    continue;
                };
                if definition.type_id == FORTRESS_TYPE_ID {
                    let Some(start) =
                        plan.fortress_start_for(&structure_id, self.world_seed, chunk_x, chunk_z)
                    else {
                        continue;
                    };
                    if !self
                        .biome_name_at(start.generation_anchor())
                        .is_some_and(|biome| definition.biomes.contains(&biome))
                    {
                        continue;
                    }
                    fortress.push(start);
                    break;
                }
                let Some(planned) = plan.plan_jigsaw_anchor(
                    startup,
                    &structure_id,
                    self.world_seed,
                    chunk_x,
                    chunk_z,
                    self.dimension_shape.min_y,
                    self.dimension_shape.height,
                    &height_access,
                ) else {
                    continue;
                };
                if !self
                    .biome_name_at(planned.generation_anchor())
                    .is_some_and(|biome| definition.biomes.contains(&biome))
                {
                    continue;
                }
                let Some(graph) = planned.expand(startup, &height_access) else {
                    continue;
                };
                let Some(start) = JigsawStructureStart::from_graph(plan, graph, 0) else {
                    continue;
                };
                jigsaw.push(start);
                break;
            }
        }
        (jigsaw, fortress)
    }

    pub(crate) fn generate_nether_fossil_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing: &[NetherFossilStructureStart],
    ) -> Vec<NetherFossilStructureStart> {
        let startup = self.startup;

        let (gen_min_y, gen_height) = self.generation_bounds();
        let mut column_cache = rustc_hash::FxHashMap::default();
        plan.generate_missing_nether_fossil_starts(
            self.world_seed,
            chunk_x,
            chunk_z,
            gen_min_y,
            gen_height,
            self.sea_level,
            existing,
            |x, y, z| self.fresh_column_state_cached(&mut column_cache, x, y, z),
            |state| {
                Some(bbb_server_worldgen::multiface::is_face_sturdy(
                    state,
                    bbb_protocol::packets::Direction::Up,
                ))
            },
            |pos| self.biome_name_at(pos),
            |id| {
                startup
                    .structures
                    .template(id)
                    .map(|element| element.template.size)
            },
        )
    }

    pub(crate) fn generate_end_city_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing: &[EndCityStructureStart],
    ) -> Vec<EndCityStructureStart> {
        let startup = self.startup;
        plan.generate_missing_random_spread_end_city_starts(
            self.world_seed,
            chunk_x,
            chunk_z,
            existing,
            |x, z, heightmap| match heightmap {
                StructureHeightmap::WorldSurfaceWg => {
                    self.fresh_base_height(x, z, VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID)
                }
                _ => None,
            },
            |position| self.biome_name_at(position),
            |id| {
                startup
                    .structures
                    .template(id)
                    .map(|element| element.template.size)
            },
        )
    }
}
