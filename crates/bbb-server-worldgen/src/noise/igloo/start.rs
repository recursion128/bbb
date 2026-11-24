//! Igloo `StructureStart` materialization and biome-gated selection.

use crate::random::large_feature_random;
use std::collections::BTreeSet;

use crate::random::WorldgenRandom;
use bbb_protocol::normalize_resource_location;
use bbb_protocol::packets::BlockPos;

use super::{
    template_offset, IglooGenerationStub, IglooPiece, IglooStructureStart, INITIAL_TEMPLATE_Y,
    TEMPLATE_BOTTOM, TEMPLATE_MIDDLE, TEMPLATE_TOP,
};
#[cfg(test)]
use super::{IGLOO_ID, IGLOO_SET_ID};
use crate::noise::structure_pieces::JigsawRotation;
use crate::noise::structures::{NoiseStructurePlan, StructureHeightmap, StructureStartSetOwner};

const ROTATIONS: [JigsawRotation; 4] = [
    JigsawRotation::None,
    JigsawRotation::Clockwise90,
    JigsawRotation::Clockwise180,
    JigsawRotation::CounterClockwise90,
];

impl NoiseStructurePlan {
    /// Runs the igloo random-spread half of `createStructures` and biome-gates
    /// the completed generation stub (`ChunkGenerator.java:476-589`,
    /// `Structure.java:121-140`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_missing_random_spread_igloo_starts(
        &self,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[IglooStructureStart],
        mut first_occupied_height: impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
        mut template_size: impl FnMut(&str) -> Option<[i32; 3]>,
    ) -> Vec<IglooStructureStart> {
        let mut existing_ids: BTreeSet<String> = existing_starts
            .iter()
            .map(|start| start.structure_id().to_string())
            .collect();
        let mut generated = Vec::new();
        for set in self.sets() {
            if self.structure_start_owner(set) != StructureStartSetOwner::Igloo
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
                let Some(stub) = self.igloo_generation_stub(
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
                existing_ids.insert(structure_id);
                generated.push(
                    IglooStructureStart::from_generation_stub(stub, 0)
                        .with_terrain_adjustment(terrain_adjustment),
                );
                break;
            }
        }
        generated
    }

    /// `PlaceCommand.placeStructure` (`PlaceCommand.java:277-296`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_forced_igloo_start(
        &self,
        structure_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        mut first_occupied_height: impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
        mut template_size: impl FnMut(&str) -> Option<[i32; 3]>,
    ) -> Option<IglooStructureStart> {
        let terrain_adjustment = self.structures().get(structure_id).map_or(
            crate::noise::structures::StructureTerrainAdjustment::None,
            |def| def.terrain_adjustment,
        );
        self.igloo_generation_stub(
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
            IglooStructureStart::from_generation_stub(stub, 0)
                .with_terrain_adjustment(terrain_adjustment)
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn igloo_generation_stub(
        &self,
        structure_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        first_occupied_height: &mut impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        biome_at: &mut impl FnMut(BlockPos) -> Option<String>,
        template_size: &mut impl FnMut(&str) -> Option<[i32; 3]>,
        check_allowed_biome: bool,
    ) -> Option<IglooGenerationStub> {
        let definition = self.structures().get(structure_id)?;
        let min_x = chunk_x * 16;
        let min_z = chunk_z * 16;
        // `IglooStructure.findGenerationPoint` (`IglooStructure.java:22-32`).
        let anchor_y =
            first_occupied_height(min_x + 8, min_z + 8, StructureHeightmap::WorldSurfaceWg)?;
        let generation_anchor = BlockPos {
            x: min_x + 8,
            y: anchor_y,
            z: min_z + 8,
        };
        let biome = normalize_resource_location(biome_at(generation_anchor)?).ok()?;
        if check_allowed_biome && !definition.biomes.contains(&biome) {
            return None;
        }

        let mut random = large_feature_random(level_seed, chunk_x, chunk_z);
        let rotation = ROTATIONS[random.next_int(ROTATIONS.len() as i32) as usize];
        let origin = BlockPos {
            x: min_x,
            y: INITIAL_TEMPLATE_Y,
            z: min_z,
        };
        let pieces = add_pieces(origin, rotation, &mut random, template_size)?;
        Some(IglooGenerationStub::new(
            [chunk_x, chunk_z],
            generation_anchor,
            structure_id,
            pieces,
        ))
    }
}

/// `IglooPieces.addPieces` (`IglooPieces.java:45-61`).
fn add_pieces(
    origin: BlockPos,
    rotation: JigsawRotation,
    random: &mut WorldgenRandom,
    template_size: &mut impl FnMut(&str) -> Option<[i32; 3]>,
) -> Option<Vec<IglooPiece>> {
    let mut pieces = Vec::new();
    if random.next_double() < 0.5 {
        let depth = random.next_int(8) + 4;
        pieces.push(make_piece(
            TEMPLATE_BOTTOM,
            origin,
            rotation,
            depth * 3,
            template_size,
        )?);
        for i in 0..depth - 1 {
            pieces.push(make_piece(
                TEMPLATE_MIDDLE,
                origin,
                rotation,
                i * 3,
                template_size,
            )?);
        }
    }
    pieces.push(make_piece(
        TEMPLATE_TOP,
        origin,
        rotation,
        0,
        template_size,
    )?);
    Some(pieces)
}

/// `IglooPieces.makePosition` (`IglooPieces.java:98-100`).
fn make_piece(
    template: &str,
    origin: BlockPos,
    rotation: JigsawRotation,
    depth: i32,
    template_size: &mut impl FnMut(&str) -> Option<[i32; 3]>,
) -> Option<IglooPiece> {
    let offset = template_offset(template)?;
    let position = BlockPos {
        x: origin.x + offset.x,
        y: origin.y + offset.y - depth,
        z: origin.z + offset.z,
    };
    Some(IglooPiece::new(template, position, rotation).with_template_size(template_size(template)?))
}

#[cfg(test)]
mod tests {
    use crate::noise::biome::NoiseBiomeSource;
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

    fn template_sizes(template: &str) -> Option<[i32; 3]> {
        match template {
            TEMPLATE_TOP => Some([7, 6, 8]),
            TEMPLATE_MIDDLE => Some([3, 4, 3]),
            TEMPLATE_BOTTOM => Some([7, 8, 9]),
            _ => None,
        }
    }

    #[test]
    fn candidate_chunk_generates_igloo_stub() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let definition = plan.structures().get(IGLOO_ID).unwrap();
        assert!(definition.biomes.contains("minecraft:snowy_taiga"));
        assert!(!definition.biomes.contains("minecraft:plains"));
        assert!(plan.possible_biomes().contains("minecraft:plains"));
        let chunk = plan.first_candidate(IGLOO_SET_ID, 0);

        let mut height_queries = Vec::new();
        let starts = plan.generate_missing_random_spread_igloo_starts(
            0,
            chunk[0],
            chunk[1],
            &[],
            |x, z, heightmap| {
                height_queries.push((x, z, heightmap));
                Some(64)
            },
            |_| Some("minecraft:snowy_taiga".to_string()),
            template_sizes,
        );

        assert_eq!(starts.len(), 1);
        let start = &starts[0];
        assert_eq!(start.structure_id(), IGLOO_ID);
        assert_eq!(start.start_chunk(), chunk);
        assert_eq!(start.references(), 0);
        let pieces = start.pieces();
        let rotation = pieces[0].rotation();
        assert!(pieces.iter().all(|piece| piece.rotation() == rotation));
        assert_eq!(pieces.last().unwrap().template(), TEMPLATE_TOP);
        if pieces.len() > 1 {
            assert_eq!(pieces[0].template(), TEMPLATE_BOTTOM);
            assert!(pieces[1..pieces.len() - 1]
                .iter()
                .all(|piece| piece.template() == TEMPLATE_MIDDLE));
        }
        assert_eq!(
            height_queries,
            [(
                chunk[0] * 16 + 8,
                chunk[1] * 16 + 8,
                StructureHeightmap::WorldSurfaceWg
            )]
        );
    }
}
