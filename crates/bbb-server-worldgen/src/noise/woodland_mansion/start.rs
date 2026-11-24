//! Woodland-mansion `StructureStart` materialization: the eager rotation
//! draw, the height gate that consumes no further randomness on a miss, the
//! biome gate, and only then the full `generateMansion` piece placement
//! (`WoodlandMansionStructure.java:29-43`, `Structure.java:121-140,184-199`).

use crate::random::large_feature_random;
use std::collections::BTreeSet;

use bbb_protocol::normalize_resource_location;
use bbb_protocol::packets::BlockPos;

use super::grid::MansionGrid;
use super::{placer, WoodlandMansionGenerationStub, WoodlandMansionStructureStart};
use crate::noise::structure_pieces::JigsawRotation;
use crate::noise::structures::{NoiseStructurePlan, StructureHeightmap, StructureStartSetOwner};

/// `Rotation.values()` declaration order (`Rotation.java:17-21`).
const ROTATIONS: [JigsawRotation; 4] = [
    JigsawRotation::None,
    JigsawRotation::Clockwise90,
    JigsawRotation::Clockwise180,
    JigsawRotation::CounterClockwise90,
];

impl NoiseStructurePlan {
    /// Runs the woodland-mansion random-spread half of `createStructures`
    /// (`ChunkGenerator.java:476-589`, `Structure.java:121-140`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_missing_random_spread_woodland_mansion_starts(
        &self,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[WoodlandMansionStructureStart],
        mut first_occupied_height: impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
        mut template_size: impl FnMut(&str) -> Option<[i32; 3]>,
    ) -> Vec<WoodlandMansionStructureStart> {
        let mut existing_ids: BTreeSet<String> = existing_starts
            .iter()
            .map(|start| start.structure_id().to_string())
            .collect();

        let mut generated = Vec::new();
        for set in self.sets() {
            if self.structure_start_owner(set) != StructureStartSetOwner::WoodlandMansion
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
                let Some(stub) = self.woodland_mansion_generation_stub(
                    &structure_id,
                    level_seed,
                    chunk_x,
                    chunk_z,
                    &mut first_occupied_height,
                    &mut biome_at,
                    &mut template_size,
                    true,
                ) else {
                    continue;
                };
                let terrain_adjustment = self.structures().get(&structure_id).map_or(
                    crate::noise::structures::StructureTerrainAdjustment::None,
                    |def| def.terrain_adjustment,
                );
                let start = WoodlandMansionStructureStart::from_generation_stub(stub, 0)
                    .with_terrain_adjustment(terrain_adjustment);
                existing_ids.insert(structure_id);
                generated.push(start);
                break;
            }
        }
        generated
    }

    /// `PlaceCommand.placeStructure` (`PlaceCommand.java:277-294`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_forced_woodland_mansion_start(
        &self,
        structure_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        mut first_occupied_height: impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
        mut template_size: impl FnMut(&str) -> Option<[i32; 3]>,
    ) -> Option<WoodlandMansionStructureStart> {
        let terrain_adjustment = self.structures().get(structure_id).map_or(
            crate::noise::structures::StructureTerrainAdjustment::None,
            |def| def.terrain_adjustment,
        );
        self.woodland_mansion_generation_stub(
            structure_id,
            level_seed,
            chunk_x,
            chunk_z,
            &mut first_occupied_height,
            &mut biome_at,
            &mut template_size,
            false,
        )
        .map(|stub| {
            WoodlandMansionStructureStart::from_generation_stub(stub, 0)
                .with_terrain_adjustment(terrain_adjustment)
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn woodland_mansion_generation_stub(
        &self,
        structure_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        first_occupied_height: &mut impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        biome_at: &mut impl FnMut(BlockPos) -> Option<String>,
        template_size: &mut impl FnMut(&str) -> Option<[i32; 3]>,
        require_allowed_biome: bool,
    ) -> Option<WoodlandMansionGenerationStub> {
        let definition = self.structures().get(structure_id)?;

        let mut random = large_feature_random(level_seed, chunk_x, chunk_z);
        // `Rotation.getRandom` runs unconditionally before the height gate
        // (`WoodlandMansionStructure.java:30-31`); a Y<60 or biome miss below
        // still leaves this single draw consumed and no more.
        let rotation = ROTATIONS[random.next_int(4) as usize];

        let stub_pos = lowest_y_in_5by5_box_offset_7_blocks(
            chunk_x,
            chunk_z,
            rotation,
            first_occupied_height,
        )?;
        if stub_pos.y < 60 {
            return None;
        }

        let biome = normalize_resource_location(biome_at(stub_pos)?).ok()?;
        if require_allowed_biome && !definition.biomes.contains(&biome) {
            return None;
        }

        // `WoodlandMansionPieces.generateMansion` (`WoodlandMansionPieces.java:35-45`),
        // deferred until here because Vanilla's `Structure.GenerationStub`
        // consumer only runs after the biome filter accepts the stub
        // (`Structure.java:204-206`).
        let grid = MansionGrid::new(&mut random);
        let mut pieces = placer::create_mansion(stub_pos, rotation, &grid, &mut random);
        for piece in &mut pieces {
            piece.set_template_size(template_size(piece.template())?);
        }

        Some(WoodlandMansionGenerationStub::new(
            [chunk_x, chunk_z],
            stub_pos,
            structure_id,
            pieces,
        ))
    }
}

/// `Structure.getLowestYIn5by5BoxOffset7Blocks` (`Structure.java:184-199`),
/// consuming no randomness: only heightmap queries.
pub(crate) fn lowest_y_in_5by5_box_offset_7_blocks(
    chunk_x: i32,
    chunk_z: i32,
    rotation: JigsawRotation,
    first_occupied_height: &mut impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
) -> Option<BlockPos> {
    let (offset_x, offset_z) = match rotation {
        JigsawRotation::None => (5, 5),
        JigsawRotation::Clockwise90 => (-5, 5),
        JigsawRotation::Clockwise180 => (-5, -5),
        JigsawRotation::CounterClockwise90 => (5, -5),
    };
    let block_x = chunk_x * 16 + 7;
    let block_z = chunk_z * 16 + 7;
    let other_x = block_x + offset_x;
    let other_z = block_z + offset_z;
    let mut y = i32::MAX;
    for (x, z) in [
        (block_x, block_z),
        (block_x, other_z),
        (other_x, block_z),
        (other_x, other_z),
    ] {
        y = y.min(first_occupied_height(
            x,
            z,
            StructureHeightmap::WorldSurfaceWg,
        )?);
    }
    Some(BlockPos {
        x: block_x,
        y,
        z: block_z,
    })
}

#[cfg(test)]
mod tests {
    use crate::noise::biome::NoiseBiomeSource;
    use crate::noise::woodland_mansion::{WOODLAND_MANSION_ID, WOODLAND_MANSION_SET_ID};
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

    pub fn forced_template_sizes() -> impl FnMut(&str) -> Option<[i32; 3]> {
        |_| Some([8, 8, 8])
    }

    fn first_seed_zero_candidate(plan: &NoiseStructurePlan) -> [i32; 2] {
        for z in 0..=64 {
            for x in 0..=64 {
                if let Some(attempts) =
                    plan.random_spread_attempt_order(WOODLAND_MANSION_SET_ID, 0, x, z)
                {
                    if !attempts.is_empty() {
                        return [x, z];
                    }
                }
            }
        }
        panic!("no seed-0 woodland_mansions candidate in the 0..=64 window");
    }

    #[test]
    fn forced_height_and_biome_emit_a_non_empty_piece_list() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let [x, z] = first_seed_zero_candidate(&plan);
        let starts = plan.generate_missing_random_spread_woodland_mansion_starts(
            0,
            x,
            z,
            &[],
            |_, _, heightmap| {
                assert_eq!(heightmap, StructureHeightmap::WorldSurfaceWg);
                Some(70)
            },
            |_| Some("minecraft:dark_forest".to_string()),
            forced_template_sizes(),
        );
        assert_eq!(starts.len(), 1);
        let start = &starts[0];
        assert_eq!(start.structure_id(), WOODLAND_MANSION_ID);
        assert_eq!(start.start_chunk(), [x, z]);
        assert!(!start.pieces().is_empty());
        assert!(start
            .pieces()
            .iter()
            .any(|piece| piece.template() == "minecraft:woodland_mansion/entrance"));
    }

    #[test]
    fn forced_structure_start_bypasses_set_and_allowed_biome_gates() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let [x, z] = [0, 0];
        assert!(plan
            .random_spread_attempt_order(WOODLAND_MANSION_SET_ID, 0, x, z)
            .unwrap()
            .is_empty());
        let mut sampled = false;
        let start = plan
            .generate_forced_woodland_mansion_start(
                WOODLAND_MANSION_ID,
                0,
                x,
                z,
                |_, _, heightmap| {
                    assert_eq!(heightmap, StructureHeightmap::WorldSurfaceWg);
                    Some(70)
                },
                |_| {
                    sampled = true;
                    Some("minecraft:plains".to_string())
                },
                forced_template_sizes(),
            )
            .unwrap();
        assert!(sampled);
        assert_eq!(start.structure_id(), WOODLAND_MANSION_ID);
        assert_eq!(start.start_chunk(), [x, z]);
        assert_eq!(start.references(), 0);
        assert!(start
            .pieces()
            .iter()
            .any(|piece| piece.template() == "minecraft:woodland_mansion/entrance"));
    }

    #[test]
    fn height_below_sixty_misses_without_generating_pieces() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let [x, z] = first_seed_zero_candidate(&plan);
        let starts = plan.generate_missing_random_spread_woodland_mansion_starts(
            0,
            x,
            z,
            &[],
            |_, _, _| Some(59),
            |_| panic!("biome must not run once the height gate misses"),
            |_| panic!("template lookup must not run once the height gate misses"),
        );
        assert!(starts.is_empty());
    }

    #[test]
    fn getlowesty_corners_match_each_rotation() {
        let mut none_corners = Vec::new();
        lowest_y_in_5by5_box_offset_7_blocks(0, 0, JigsawRotation::None, &mut |x, z, _| {
            none_corners.push((x, z));
            Some(64)
        })
        .unwrap();
        assert_eq!(none_corners, vec![(7, 7), (7, 12), (12, 7), (12, 12)]);

        let mut cw90_corners = Vec::new();
        lowest_y_in_5by5_box_offset_7_blocks(0, 0, JigsawRotation::Clockwise90, &mut |x, z, _| {
            cw90_corners.push((x, z));
            Some(64)
        })
        .unwrap();
        assert_eq!(cw90_corners, vec![(7, 7), (7, 12), (2, 7), (2, 12)]);

        let mut cw180_corners = Vec::new();
        lowest_y_in_5by5_box_offset_7_blocks(0, 0, JigsawRotation::Clockwise180, &mut |x, z, _| {
            cw180_corners.push((x, z));
            Some(64)
        })
        .unwrap();
        assert_eq!(cw180_corners, vec![(7, 7), (7, 2), (2, 7), (2, 2)]);

        let mut ccw90_corners = Vec::new();
        lowest_y_in_5by5_box_offset_7_blocks(
            0,
            0,
            JigsawRotation::CounterClockwise90,
            &mut |x, z, _| {
                ccw90_corners.push((x, z));
                Some(64)
            },
        )
        .unwrap();
        assert_eq!(ccw90_corners, vec![(7, 7), (7, 2), (12, 7), (12, 2)]);
    }
}
