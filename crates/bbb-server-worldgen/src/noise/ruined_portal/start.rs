//! Ruined-portal `StructureStart` materialization and biome-gated selection.

use crate::random::large_feature_random;
use std::collections::BTreeSet;

use crate::random::WorldgenRandom;
use bbb_protocol::normalize_resource_location;
use bbb_protocol::packets::BlockPos;

use super::{
    template_bounding_box, RuinedPortalGenerationStub, RuinedPortalMirror, RuinedPortalPiece,
    RuinedPortalProperties, RuinedPortalSetup, RuinedPortalStructureStart,
    RuinedPortalVerticalPlacement, GIANT_PORTAL_PROBABILITY, GIANT_PORTAL_TEMPLATES, MIN_Y_INDEX,
    PORTAL_TEMPLATES,
};
use crate::noise::structure_pieces::JigsawRotation;
use crate::noise::structures::{NoiseStructurePlan, StructureHeightmap, StructureStartSetOwner};

const ROTATIONS: [JigsawRotation; 4] = [
    JigsawRotation::None,
    JigsawRotation::Clockwise90,
    JigsawRotation::Clockwise180,
    JigsawRotation::CounterClockwise90,
];

impl NoiseStructurePlan {
    /// Runs the ruined-portal random-spread half of `createStructures` and
    /// biome-gates the completed generation stub (`ChunkGenerator.java:476-589`,
    /// `Structure.java:204-206`, `RuinedPortalStructure.java:67-150`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_missing_random_spread_ruined_portal_starts(
        &self,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[RuinedPortalStructureStart],
        sea_level: i32,
        min_y: i32,
        mut first_occupied_height: impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        mut column_opaque: impl FnMut(i32, i32, i32, StructureHeightmap) -> Option<bool>,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
        mut cold_enough_to_snow: impl FnMut(&str, BlockPos) -> Option<bool>,
        mut template_size: impl FnMut(&str) -> Option<[i32; 3]>,
    ) -> Vec<RuinedPortalStructureStart> {
        let mut existing_ids: BTreeSet<String> = existing_starts
            .iter()
            .map(|start| start.structure_id().to_string())
            .collect();
        let mut generated = Vec::new();
        for set in self.sets() {
            if self.structure_start_owner(set) != StructureStartSetOwner::RuinedPortal
                || set
                    .structures
                    .iter()
                    .any(|selection| existing_ids.contains(&selection.structure))
            {
                continue;
            }
            let Some(attempts) =
                self.random_spread_attempt_order(&set.id, level_seed, chunk_x, chunk_z)
            else {
                continue;
            };
            for structure_id in attempts {
                let Some(stub) = self.ruined_portal_generation_stub(
                    &structure_id,
                    level_seed,
                    chunk_x,
                    chunk_z,
                    sea_level,
                    min_y,
                    &mut first_occupied_height,
                    &mut column_opaque,
                    &mut biome_at,
                    &mut cold_enough_to_snow,
                    &mut template_size,
                    true,
                ) else {
                    continue;
                };
                let terrain_adjustment = self.structures().get(&structure_id).map_or(
                    crate::noise::structures::StructureTerrainAdjustment::None,
                    |def| def.terrain_adjustment,
                );
                existing_ids.insert(structure_id);
                generated.push(
                    RuinedPortalStructureStart::from_generation_stub(stub, 0)
                        .with_terrain_adjustment(terrain_adjustment),
                );
                break;
            }
        }
        generated
    }

    /// `PlaceCommand.placeStructure` (`PlaceCommand.java:277-294`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_forced_ruined_portal_start(
        &self,
        structure_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        sea_level: i32,
        min_y: i32,
        mut first_occupied_height: impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        mut column_opaque: impl FnMut(i32, i32, i32, StructureHeightmap) -> Option<bool>,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
        mut cold_enough_to_snow: impl FnMut(&str, BlockPos) -> Option<bool>,
        mut template_size: impl FnMut(&str) -> Option<[i32; 3]>,
    ) -> Option<RuinedPortalStructureStart> {
        let terrain_adjustment = self.structures().get(structure_id).map_or(
            crate::noise::structures::StructureTerrainAdjustment::None,
            |def| def.terrain_adjustment,
        );
        self.ruined_portal_generation_stub(
            structure_id,
            level_seed,
            chunk_x,
            chunk_z,
            sea_level,
            min_y,
            &mut first_occupied_height,
            &mut column_opaque,
            &mut biome_at,
            &mut cold_enough_to_snow,
            &mut template_size,
            false,
        )
        .map(|stub| {
            RuinedPortalStructureStart::from_generation_stub(stub, 0)
                .with_terrain_adjustment(terrain_adjustment)
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn ruined_portal_generation_stub(
        &self,
        structure_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        _sea_level: i32,
        min_y: i32,
        first_occupied_height: &mut impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        column_opaque: &mut impl FnMut(i32, i32, i32, StructureHeightmap) -> Option<bool>,
        biome_at: &mut impl FnMut(BlockPos) -> Option<String>,
        cold_enough_to_snow: &mut impl FnMut(&str, BlockPos) -> Option<bool>,
        template_size: &mut impl FnMut(&str) -> Option<[i32; 3]>,
        require_allowed_biome: bool,
    ) -> Option<RuinedPortalGenerationStub> {
        let definition = self.structures().get(structure_id)?;
        let settings = definition.ruined_portal.as_ref()?;
        let min_x = chunk_x * 16;
        let min_z = chunk_z * 16;

        let mut random = large_feature_random(level_seed, chunk_x, chunk_z);
        let setup = choose_setup(&settings.setups, &mut random)?;
        let air_pocket = sample(&mut random, setup.air_pocket_probability.get());
        let template = choose_template(&mut random);
        let size = template_size(template)?;
        let rotation = ROTATIONS[random.next_int(ROTATIONS.len() as i32) as usize];
        let mirror = if random.next_float() < 0.5 {
            RuinedPortalMirror::None
        } else {
            RuinedPortalMirror::FrontBack
        };
        let pivot = BlockPos {
            x: size[0] / 2,
            y: 0,
            z: size[2] / 2,
        };
        let base_position = BlockPos {
            x: min_x,
            y: 0,
            z: min_z,
        };
        let bounds = template_bounding_box(base_position, rotation, pivot, mirror, size);
        let center = bounding_box_center(bounds);
        let heightmap = heightmap_for(setup.placement);
        let surface_y = first_occupied_height(center.x, center.z, heightmap)? - 1;
        let y_span = bounds[4] - bounds[1] + 1;
        let projected_y = find_suitable_y(
            &mut random,
            setup.placement,
            air_pocket,
            surface_y,
            y_span,
            bounds,
            min_y,
            heightmap,
            column_opaque,
        )?;
        let generation_anchor = BlockPos {
            x: min_x,
            y: projected_y,
            z: min_z,
        };

        let biome = normalize_resource_location(biome_at(generation_anchor)?).ok()?;
        if require_allowed_biome && !definition.biomes.contains(&biome) {
            return None;
        }
        let cold = setup.can_be_cold && cold_enough_to_snow(&biome, generation_anchor)?;
        let piece = RuinedPortalPiece::new(
            template,
            generation_anchor,
            rotation,
            mirror,
            setup.placement,
            RuinedPortalProperties {
                cold,
                mossiness: setup.mossiness,
                air_pocket,
                overgrown: setup.overgrown,
                vines: setup.vines,
                replace_with_blackstone: setup.replace_with_blackstone,
            },
        )
        .with_template_size(size);
        Some(RuinedPortalGenerationStub::new(
            [chunk_x, chunk_z],
            generation_anchor,
            structure_id,
            piece,
        ))
    }
}

/// `RuinedPortalStructure.findGenerationPoint` weighted setup pick
/// (`RuinedPortalStructure.java:67-83`).
fn choose_setup(
    setups: &[RuinedPortalSetup],
    random: &mut WorldgenRandom,
) -> Option<RuinedPortalSetup> {
    if setups.len() == 1 {
        return Some(setups[0]);
    }
    let total: f32 = setups.iter().map(|setup| setup.weight.get()).sum();
    let mut pick = random.next_float();
    setups.iter().copied().find(|setup| {
        pick -= setup.weight.get() / total;
        pick < 0.0
    })
}

/// `RuinedPortalStructure.sample` (`RuinedPortalStructure.java:152-158`).
fn sample(random: &mut WorldgenRandom, limit: f32) -> bool {
    if limit == 0.0 {
        false
    } else if limit == 1.0 {
        true
    } else {
        random.next_float() < limit
    }
}

fn choose_template(random: &mut WorldgenRandom) -> &'static str {
    if random.next_float() < GIANT_PORTAL_PROBABILITY {
        GIANT_PORTAL_TEMPLATES[random.next_int(GIANT_PORTAL_TEMPLATES.len() as i32) as usize]
    } else {
        PORTAL_TEMPLATES[random.next_int(PORTAL_TEMPLATES.len() as i32) as usize]
    }
}

fn heightmap_for(placement: RuinedPortalVerticalPlacement) -> StructureHeightmap {
    if placement == RuinedPortalVerticalPlacement::OnOceanFloor {
        StructureHeightmap::OceanFloorWg
    } else {
        StructureHeightmap::WorldSurfaceWg
    }
}

/// `BoundingBox.getCenter` (`BoundingBox.java:237-239`).
fn bounding_box_center(bounds: [i32; 6]) -> BlockPos {
    BlockPos {
        x: bounds[0] + (bounds[3] - bounds[0] + 1) / 2,
        y: bounds[1] + (bounds[4] - bounds[1] + 1) / 2,
        z: bounds[2] + (bounds[5] - bounds[2] + 1) / 2,
    }
}

/// `RuinedPortalStructure.findSuitableY` (`RuinedPortalStructure.java:164-225`).
#[allow(clippy::too_many_arguments)]
fn find_suitable_y(
    random: &mut WorldgenRandom,
    placement: RuinedPortalVerticalPlacement,
    air_pocket: bool,
    surface_y: i32,
    y_span: i32,
    bounds: [i32; 6],
    min_y: i32,
    heightmap: StructureHeightmap,
    column_opaque: &mut impl FnMut(i32, i32, i32, StructureHeightmap) -> Option<bool>,
) -> Option<i32> {
    let walk_min_y = min_y + MIN_Y_INDEX;
    let new_y = match placement {
        RuinedPortalVerticalPlacement::InNether => {
            if air_pocket {
                random_between_inclusive(random, 32, 100)
            } else if random.next_float() < 0.5 {
                random_between_inclusive(random, 27, 29)
            } else {
                random_between_inclusive(random, 29, 100)
            }
        }
        RuinedPortalVerticalPlacement::InMountain => {
            get_random_within_interval(random, 70, surface_y.wrapping_sub(y_span))
        }
        RuinedPortalVerticalPlacement::Underground => {
            get_random_within_interval(random, walk_min_y, surface_y.wrapping_sub(y_span))
        }
        RuinedPortalVerticalPlacement::PartlyBuried => surface_y
            .wrapping_sub(y_span)
            .wrapping_add(random_between_inclusive(random, 2, 8)),
        RuinedPortalVerticalPlacement::OnLandSurface
        | RuinedPortalVerticalPlacement::OnOceanFloor => surface_y,
    };
    let corners = [
        [bounds[0], bounds[2]],
        [bounds[3], bounds[2]],
        [bounds[0], bounds[5]],
        [bounds[3], bounds[5]],
    ];
    let mut projected_y = new_y;
    while projected_y > walk_min_y {
        let mut corners_on_solid = 0;
        for [x, z] in corners {
            if column_opaque(x, projected_y, z, heightmap)? {
                corners_on_solid += 1;
                if corners_on_solid == 3 {
                    return Some(projected_y);
                }
            }
        }
        projected_y -= 1;
    }
    Some(projected_y)
}

fn random_between_inclusive(random: &mut WorldgenRandom, min: i32, max: i32) -> i32 {
    random.next_int(max.wrapping_sub(min).wrapping_add(1)) + min
}

fn get_random_within_interval(random: &mut WorldgenRandom, min_preferred: i32, max: i32) -> i32 {
    if min_preferred < max {
        random_between_inclusive(random, min_preferred, max)
    } else {
        max
    }
}

#[cfg(test)]
mod tests {
    use crate::noise::biome::NoiseBiomeSource;
    use crate::noise::ruined_portal::{RuinedPortalVerticalPlacement, RUINED_PORTAL_SET_ID};
    use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

    use super::*;

    fn startup() -> DatapackStartup {
        DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        })
        .unwrap()
    }

    fn overworld_plan(startup: &DatapackStartup) -> NoiseStructurePlan {
        let source = NoiseBiomeSource::from_registry(
            &startup.registries,
            "minecraft:overworld",
            "minecraft:overworld",
            0,
        )
        .unwrap();
        NoiseStructurePlan::from_startup(startup, source.overworld_biome_order().unwrap()).unwrap()
    }

    fn template_sizes(startup: &DatapackStartup) -> impl FnMut(&str) -> Option<[i32; 3]> + '_ {
        |template| {
            startup
                .structures
                .template(template)
                .map(|element| element.template.size)
        }
    }

    fn generate(
        plan: &NoiseStructurePlan,
        startup: &DatapackStartup,
        chunk: [i32; 2],
        biome: &str,
        first_occupied: i32,
        column_opaque: bool,
        cold: bool,
    ) -> Vec<RuinedPortalStructureStart> {
        let mut sizes = template_sizes(startup);
        plan.generate_missing_random_spread_ruined_portal_starts(
            0,
            chunk[0],
            chunk[1],
            &[],
            63,
            -64,
            |_, _, _| Some(first_occupied),
            |_, _, _, _| Some(column_opaque),
            |_| Some(biome.to_string()),
            |_, _| Some(cold),
            &mut sizes,
        )
    }

    #[test]
    fn desert_candidate_generates_partly_buried_portal() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let chunk = plan.first_candidate(RUINED_PORTAL_SET_ID, 0);
        let desert = plan.structures()["minecraft:ruined_portal_desert"]
            .ruined_portal
            .as_ref()
            .unwrap();
        assert_eq!(desert.setups.len(), 1);
        assert_eq!(
            desert.setups[0].placement,
            RuinedPortalVerticalPlacement::PartlyBuried
        );

        let starts = generate(&plan, &startup, chunk, "minecraft:desert", 64, true, false);
        assert_eq!(starts.len(), 1);
        let start = &starts[0];
        assert_eq!(start.structure_id(), "minecraft:ruined_portal_desert");
        assert_eq!(start.start_chunk(), chunk);
        assert_eq!(start.references(), 0);
        let piece = start.piece();
        assert_eq!(
            piece.vertical_placement(),
            RuinedPortalVerticalPlacement::PartlyBuried
        );
        assert!(!piece.properties().air_pocket);
        assert!(!piece.properties().cold);
        assert!(!piece.properties().overgrown);
        assert!(!piece.properties().vines);
        assert!(!piece.properties().replace_with_blackstone);
        assert_eq!(piece.properties().mossiness.get(), 0.0);
        assert!(
            PORTAL_TEMPLATES.contains(&piece.template())
                || GIANT_PORTAL_TEMPLATES.contains(&piece.template()),
            "{}",
            piece.template()
        );
        assert_ne!(piece.template_size(), [0, 0, 0]);
    }

    #[test]
    fn standard_portal_can_be_cold_queries_after_biome_hit() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let chunk = plan.first_candidate(RUINED_PORTAL_SET_ID, 0);
        let mut sizes = template_sizes(&startup);
        let mut cold_queries = Vec::new();
        let starts = plan.generate_missing_random_spread_ruined_portal_starts(
            0,
            chunk[0],
            chunk[1],
            &[],
            63,
            -64,
            |_, _, _| Some(64),
            |_, _, _, _| Some(true),
            |_| Some("minecraft:plains".to_string()),
            |biome, pos| {
                cold_queries.push((biome.to_string(), pos));
                Some(true)
            },
            &mut sizes,
        );
        assert_eq!(starts.len(), 1);
        assert_eq!(starts[0].structure_id(), "minecraft:ruined_portal");
        assert_eq!(cold_queries.len(), 1);
        assert_eq!(cold_queries[0].0, "minecraft:plains");
        assert!(starts[0].piece().properties().cold);
    }
}
