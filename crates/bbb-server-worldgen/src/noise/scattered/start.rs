//! Scattered-feature `StructureStart` materialization and biome-gated selection.

use crate::random::large_feature_random;
use std::collections::BTreeSet;

use bbb_protocol::normalize_resource_location;
use bbb_protocol::packets::BlockPos;

use super::{
    make_bounding_box, HorizontalDirection, ScatteredFeatureExtra, ScatteredFeatureKind,
    ScatteredFeaturePiece, ScatteredFeatureStructureStart, INITIAL_FLOOR_Y,
};
use crate::noise::structures::{NoiseStructurePlan, StructureHeightmap, StructureStartSetOwner};

/// `Direction.Plane.HORIZONTAL` (`Direction.java:577-589`).
const HORIZONTAL: [HorizontalDirection; 4] = [
    HorizontalDirection::North,
    HorizontalDirection::East,
    HorizontalDirection::South,
    HorizontalDirection::West,
];

impl NoiseStructurePlan {
    /// Random-spread leftover scattered features, biome-gated at the
    /// WORLD_SURFACE_WG chunk-middle stub (`ChunkGenerator.java:464-589`,
    /// `SinglePieceStructure.java:24-33`, `SwampHutStructure.java:18-24`,
    /// `Structure.java:131-140`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_missing_random_spread_scattered_feature_starts(
        &self,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[ScatteredFeatureStructureStart],
        sea_level: i32,
        mut first_occupied_height: impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
    ) -> Vec<ScatteredFeatureStructureStart> {
        let mut existing_ids: BTreeSet<String> = existing_starts
            .iter()
            .map(|start| start.structure_id().to_string())
            .collect();
        let mut generated = Vec::new();
        for set in self.sets() {
            let kind = match self.structure_start_owner(set) {
                StructureStartSetOwner::DesertPyramid => ScatteredFeatureKind::DesertPyramid,
                StructureStartSetOwner::JungleTemple => ScatteredFeatureKind::JungleTemple,
                StructureStartSetOwner::SwampHut => ScatteredFeatureKind::SwampHut,
                _ => continue,
            };
            if set
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
                let Some(start) = self.scattered_feature_start(
                    Some(kind),
                    &structure_id,
                    level_seed,
                    chunk_x,
                    chunk_z,
                    sea_level,
                    &mut first_occupied_height,
                    &mut biome_at,
                    true,
                ) else {
                    continue;
                };
                existing_ids.insert(structure_id);
                generated.push(start);
                break;
            }
        }
        generated
    }

    /// `PlaceCommand.placeStructure` (`PlaceCommand.java:277-296`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_forced_scattered_feature_start(
        &self,
        structure_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        sea_level: i32,
        mut first_occupied_height: impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
    ) -> Option<ScatteredFeatureStructureStart> {
        self.scattered_feature_start(
            None,
            structure_id,
            level_seed,
            chunk_x,
            chunk_z,
            sea_level,
            &mut first_occupied_height,
            &mut biome_at,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn scattered_feature_start(
        &self,
        set_kind: Option<ScatteredFeatureKind>,
        structure_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        sea_level: i32,
        first_occupied_height: &mut impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        biome_at: &mut impl FnMut(BlockPos) -> Option<String>,
        check_allowed_biome: bool,
    ) -> Option<ScatteredFeatureStructureStart> {
        let definition = self.structures().get(structure_id)?;
        let kind = ScatteredFeatureKind::from_type_id(&definition.type_id)?;
        if set_kind.is_some_and(|set_kind| kind != set_kind) {
            return None;
        }
        let min_x = chunk_x * 16;
        let min_z = chunk_z * 16;

        // `getLowestY(sizeX, sizeZ)` uses chunk min as origin (`Structure.java:171-180`).
        if let Some((size_x, size_z)) = kind.lowest_y_size() {
            if get_lowest_y(first_occupied_height, min_x, min_z, size_x, size_z)? < sea_level {
                return None;
            }
        }

        // Biome gate uses the WORLD_SURFACE_WG stub y, not the piece floor.
        let generation_anchor = BlockPos {
            x: min_x + 8,
            y: first_occupied_height(min_x + 8, min_z + 8, StructureHeightmap::WorldSurfaceWg)?,
            z: min_z + 8,
        };
        let biome = normalize_resource_location(biome_at(generation_anchor)?).ok()?;
        if check_allowed_biome && !definition.biomes.contains(&biome) {
            return None;
        }

        let mut random = large_feature_random(level_seed, chunk_x, chunk_z);
        let orientation = HORIZONTAL[random.next_int(HORIZONTAL.len() as i32) as usize];
        let (width, height, depth) = kind.dimensions();
        let bounding_box = make_bounding_box(
            min_x,
            INITIAL_FLOOR_Y,
            min_z,
            orientation,
            width,
            height,
            depth,
        );
        let piece = ScatteredFeaturePiece::new(
            kind,
            width,
            height,
            depth,
            -1,
            orientation,
            bounding_box,
            ScatteredFeatureExtra::default_for(kind),
        );
        Some(
            ScatteredFeatureStructureStart::from_parts(structure_id, [chunk_x, chunk_z], 0, piece)
                .with_terrain_adjustment(definition.terrain_adjustment),
        )
    }
}

/// `Structure.getCornerHeights` then min (`Structure.java:154-180`).
fn get_lowest_y(
    first_occupied_height: &mut impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
    min_x: i32,
    min_z: i32,
    size_x: i32,
    size_z: i32,
) -> Option<i32> {
    let (far_x, far_z) = (min_x + size_x, min_z + size_z);
    let mut lowest = i32::MAX;
    for (x, z) in [
        (min_x, min_z),
        (min_x, far_z),
        (far_x, min_z),
        (far_x, far_z),
    ] {
        lowest = lowest.min(first_occupied_height(
            x,
            z,
            StructureHeightmap::WorldSurfaceWg,
        )?);
    }
    Some(lowest)
}

#[cfg(test)]
mod tests {
    use crate::noise::biome::NoiseBiomeSource;
    use crate::noise::scattered::{
        DESERT_PYRAMID_ID, DESERT_PYRAMID_SET_ID, JUNGLE_TEMPLE_ID, JUNGLE_TEMPLE_SET_ID,
        SWAMP_HUT_ID, SWAMP_HUT_SET_ID,
    };
    use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

    use super::*;

    const OVERWORLD_SEA_LEVEL: i32 = 63;

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

    fn matching_biome(plan: &NoiseStructurePlan, structure_id: &str) -> String {
        plan.structures()[structure_id]
            .biomes
            .iter()
            .next()
            .expect("scattered feature has at least one biome")
            .clone()
    }

    fn generate(
        plan: &NoiseStructurePlan,
        chunk: [i32; 2],
        existing: &[ScatteredFeatureStructureStart],
        height: i32,
        biome: &str,
        queries: &mut Vec<(i32, i32, StructureHeightmap)>,
    ) -> Vec<ScatteredFeatureStructureStart> {
        plan.generate_missing_random_spread_scattered_feature_starts(
            0,
            chunk[0],
            chunk[1],
            existing,
            OVERWORLD_SEA_LEVEL,
            |x, z, heightmap| {
                queries.push((x, z, heightmap));
                Some(height)
            },
            |_| Some(biome.to_string()),
        )
    }

    fn expected_corners(
        chunk: [i32; 2],
        size_x: i32,
        size_z: i32,
    ) -> Vec<(i32, i32, StructureHeightmap)> {
        let min_x = chunk[0] * 16;
        let min_z = chunk[1] * 16;
        vec![
            (min_x, min_z, StructureHeightmap::WorldSurfaceWg),
            (min_x, min_z + size_z, StructureHeightmap::WorldSurfaceWg),
            (min_x + size_x, min_z, StructureHeightmap::WorldSurfaceWg),
            (
                min_x + size_x,
                min_z + size_z,
                StructureHeightmap::WorldSurfaceWg,
            ),
        ]
    }

    fn expected_middle(chunk: [i32; 2]) -> (i32, i32, StructureHeightmap) {
        (
            chunk[0] * 16 + 8,
            chunk[1] * 16 + 8,
            StructureHeightmap::WorldSurfaceWg,
        )
    }

    #[test]
    fn forced_custom_desert_pyramid_ignores_sets_and_allowed_biomes_but_keeps_terrain() {
        let mut startup = startup();
        let registry = startup
            .registries
            .registries
            .iter_mut()
            .find(|registry| registry.registry_id == "minecraft:worldgen/structure")
            .unwrap();
        let mut custom = registry
            .elements
            .iter()
            .find(|element| element.id == DESERT_PYRAMID_ID)
            .unwrap()
            .clone();
        custom.id = "test:forced_pyramid".to_string();
        registry.elements.push(custom);
        startup
            .registries
            .registries
            .iter_mut()
            .find(|registry| registry.registry_id == "minecraft:worldgen/structure_set")
            .unwrap()
            .elements
            .clear();
        let plan = overworld_plan(&startup);
        assert!(plan.sets().is_empty());
        assert!(!plan.structures()["test:forced_pyramid"]
            .biomes
            .contains("minecraft:plains"));

        let mut queries = Vec::new();
        let mut biome_positions = Vec::new();
        let start = plan
            .generate_forced_scattered_feature_start(
                "test:forced_pyramid",
                0,
                0,
                0,
                OVERWORLD_SEA_LEVEL,
                |x, z, heightmap| {
                    queries.push((x, z, heightmap));
                    Some(72)
                },
                |position| {
                    biome_positions.push(position);
                    Some("minecraft:plains".to_string())
                },
            )
            .unwrap();
        assert_eq!(start.structure_id(), "test:forced_pyramid");
        assert_eq!(start.references(), 0);
        assert_eq!(start.piece().kind(), ScatteredFeatureKind::DesertPyramid);
        let mut expected = expected_corners([0, 0], 21, 21);
        expected.push(expected_middle([0, 0]));
        assert_eq!(queries, expected);
        assert_eq!(biome_positions, vec![BlockPos { x: 8, y: 72, z: 8 }]);

        assert!(plan
            .generate_forced_scattered_feature_start(
                "test:forced_pyramid",
                0,
                0,
                0,
                OVERWORLD_SEA_LEVEL,
                |_, _, _| Some(OVERWORLD_SEA_LEVEL - 1),
                |_| panic!("rejected terrain never reaches the biome gate"),
            )
            .is_none());
    }

    #[test]
    fn matching_biome_generates_desert_pyramid_start() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let chunk = plan.first_candidate(DESERT_PYRAMID_SET_ID, 0);
        let biome = matching_biome(&plan, DESERT_PYRAMID_ID);
        assert_eq!(biome, "minecraft:desert");
        let mut queries = Vec::new();
        let starts = generate(&plan, chunk, &[], 80, &biome, &mut queries);
        assert_eq!(starts.len(), 1);
        let start = &starts[0];
        assert_eq!(start.structure_id(), DESERT_PYRAMID_ID);
        assert_eq!(start.start_chunk(), chunk);
        assert_eq!(start.references(), 0);
        let piece = start.piece();
        assert_eq!(piece.kind(), ScatteredFeatureKind::DesertPyramid);
        assert_eq!((piece.width(), piece.height(), piece.depth()), (21, 15, 21));
        assert_eq!(piece.height_position(), -1);
        assert!(piece.extra().all_false());
        let expected_bb = make_bounding_box(
            chunk[0] * 16,
            INITIAL_FLOOR_Y,
            chunk[1] * 16,
            piece.orientation(),
            21,
            15,
            21,
        );
        assert_eq!(piece.bounding_box(), expected_bb);
        let mut expected = expected_corners(chunk, 21, 21);
        expected.push(expected_middle(chunk));
        assert_eq!(queries, expected);
    }

    #[test]
    fn matching_biome_generates_jungle_temple_start() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let chunk = plan.first_candidate(JUNGLE_TEMPLE_SET_ID, 0);
        let biome = matching_biome(&plan, JUNGLE_TEMPLE_ID);
        assert_eq!(biome, "minecraft:bamboo_jungle");
        let mut queries = Vec::new();
        let starts = generate(&plan, chunk, &[], 80, &biome, &mut queries);
        assert_eq!(starts.len(), 1);
        let start = &starts[0];
        assert_eq!(start.structure_id(), JUNGLE_TEMPLE_ID);
        assert_eq!(start.start_chunk(), chunk);
        let piece = start.piece();
        assert_eq!(piece.kind(), ScatteredFeatureKind::JungleTemple);
        assert_eq!((piece.width(), piece.height(), piece.depth()), (12, 10, 15));
        assert_eq!(piece.height_position(), -1);
        assert!(piece.extra().all_false());
        let expected_bb = make_bounding_box(
            chunk[0] * 16,
            INITIAL_FLOOR_Y,
            chunk[1] * 16,
            piece.orientation(),
            12,
            10,
            15,
        );
        assert_eq!(piece.bounding_box(), expected_bb);
        let mut expected = expected_corners(chunk, 12, 15);
        expected.push(expected_middle(chunk));
        assert_eq!(queries, expected);
    }

    #[test]
    fn matching_biome_generates_swamp_hut_start() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let chunk = plan.first_candidate(SWAMP_HUT_SET_ID, 0);
        let biome = matching_biome(&plan, SWAMP_HUT_ID);
        assert_eq!(biome, "minecraft:swamp");
        let mut queries = Vec::new();
        let starts = generate(&plan, chunk, &[], 80, &biome, &mut queries);
        assert_eq!(starts.len(), 1);
        let start = &starts[0];
        assert_eq!(start.structure_id(), SWAMP_HUT_ID);
        assert_eq!(start.start_chunk(), chunk);
        let piece = start.piece();
        assert_eq!(piece.kind(), ScatteredFeatureKind::SwampHut);
        assert_eq!((piece.width(), piece.height(), piece.depth()), (7, 7, 9));
        assert_eq!(piece.height_position(), -1);
        assert!(piece.extra().all_false());
        let expected_bb = make_bounding_box(
            chunk[0] * 16,
            INITIAL_FLOOR_Y,
            chunk[1] * 16,
            piece.orientation(),
            7,
            7,
            9,
        );
        assert_eq!(piece.bounding_box(), expected_bb);
        assert_eq!(queries, [expected_middle(chunk)]);
    }

    #[test]
    fn desert_and_jungle_skip_when_lowest_corner_is_below_sea_level() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let mut desert_queries = Vec::new();
        let desert_starts = generate(
            &plan,
            plan.first_candidate(DESERT_PYRAMID_SET_ID, 0),
            &[],
            40,
            "minecraft:desert",
            &mut desert_queries,
        );
        assert!(desert_starts.is_empty());
        assert_eq!(
            desert_queries,
            expected_corners(plan.first_candidate(DESERT_PYRAMID_SET_ID, 0), 21, 21)
        );

        let mut jungle_queries = Vec::new();
        let jungle_starts = generate(
            &plan,
            plan.first_candidate(JUNGLE_TEMPLE_SET_ID, 0),
            &[],
            40,
            "minecraft:jungle",
            &mut jungle_queries,
        );
        assert!(jungle_starts.is_empty());
        assert_eq!(
            jungle_queries,
            expected_corners(plan.first_candidate(JUNGLE_TEMPLE_SET_ID, 0), 12, 15)
        );
    }

    #[test]
    fn swamp_hut_still_generates_when_surface_is_below_sea_level() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let mut queries = Vec::new();
        let starts = generate(
            &plan,
            plan.first_candidate(SWAMP_HUT_SET_ID, 0),
            &[],
            40,
            "minecraft:swamp",
            &mut queries,
        );
        assert_eq!(starts.len(), 1);
        assert_eq!(starts[0].structure_id(), SWAMP_HUT_ID);
        assert_eq!(
            queries,
            [expected_middle(plan.first_candidate(SWAMP_HUT_SET_ID, 0))]
        );
    }

    #[test]
    fn biome_miss_skips_the_attempt() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let mut queries = Vec::new();
        let starts = generate(
            &plan,
            plan.first_candidate(DESERT_PYRAMID_SET_ID, 0),
            &[],
            80,
            "minecraft:plains",
            &mut queries,
        );
        assert!(starts.is_empty());
    }

    #[test]
    fn existing_start_skips_that_set() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let mut queries = Vec::new();
        let generated = generate(
            &plan,
            plan.first_candidate(DESERT_PYRAMID_SET_ID, 0),
            &[],
            80,
            "minecraft:desert",
            &mut queries,
        );
        assert_eq!(generated.len(), 1);
        queries.clear();
        let skipped = generate(
            &plan,
            plan.first_candidate(DESERT_PYRAMID_SET_ID, 0),
            &generated,
            80,
            "minecraft:desert",
            &mut queries,
        );
        assert!(skipped.is_empty());
    }
}
