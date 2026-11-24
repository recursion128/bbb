//! Stronghold `StructureStart` materialization: the concentric-ring chunk
//! gate, the biome gate at the generation stub, and only then `generatePieces`
//! (`StrongholdStructure.java:19-44`, `Structure.java:131-140,204-206`).

use bbb_protocol::normalize_resource_location;
use bbb_protocol::packets::BlockPos;

use super::pieces;
use super::StrongholdStructureStart;
use crate::noise::structures::NoiseStructurePlan;

impl NoiseStructurePlan {
    /// Runs the stronghold concentric-ring half of `createStructures`
    /// (`ChunkGenerator.java:476-589`, `Structure.java:204-206`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_missing_concentric_ring_stronghold_starts(
        &self,
        set_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[StrongholdStructureStart],
        ring_positions: &[[i32; 2]],
        sea_level: i32,
        min_y: i32,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
    ) -> Vec<StrongholdStructureStart> {
        let Some(set) = self.set(set_id) else {
            return Vec::new();
        };
        if set.structures.iter().any(|selection| {
            existing_starts
                .iter()
                .any(|start| start.structure_id() == selection.structure)
        }) || !ring_positions.contains(&[chunk_x, chunk_z])
        {
            return Vec::new();
        }
        let Some(attempts) = self.weighted_attempt_order(set_id, level_seed, chunk_x, chunk_z)
        else {
            return Vec::new();
        };
        attempts
            .into_iter()
            .find_map(|structure_id| {
                self.stronghold_generation_start(
                    &structure_id,
                    level_seed,
                    chunk_x,
                    chunk_z,
                    sea_level,
                    min_y,
                    &mut biome_at,
                    true,
                )
            })
            .into_iter()
            .collect()
    }

    /// `PlaceCommand.placeStructure` (`PlaceCommand.java:277-294`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_forced_stronghold_start(
        &self,
        structure_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        sea_level: i32,
        min_y: i32,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
    ) -> Option<StrongholdStructureStart> {
        self.stronghold_generation_start(
            structure_id,
            level_seed,
            chunk_x,
            chunk_z,
            sea_level,
            min_y,
            &mut biome_at,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn stronghold_generation_start(
        &self,
        structure_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        sea_level: i32,
        min_y: i32,
        biome_at: &mut impl FnMut(BlockPos) -> Option<String>,
        require_allowed_biome: bool,
    ) -> Option<StrongholdStructureStart> {
        let definition = self.structures().get(structure_id)?;
        // `findGenerationPoint` always stubs at `chunkPos.getWorldPosition()`
        // (`StrongholdStructure.java:19-20`, `ChunkPos.java:176-177`).
        let stub = BlockPos {
            x: chunk_x.wrapping_mul(16),
            y: 0,
            z: chunk_z.wrapping_mul(16),
        };
        // `Structure.isValidBiome` samples `QuartPos.fromBlock` of that stub
        // (`Structure.java:131-140,204-206`).
        let biome = normalize_resource_location(biome_at(stub)?).ok()?;
        if require_allowed_biome && !definition.biomes.contains(&biome) {
            return None;
        }

        let pieces = pieces::generate_pieces(level_seed, chunk_x, chunk_z, sea_level, min_y);
        Some(
            StrongholdStructureStart::from_generation(structure_id, [chunk_x, chunk_z], 0, pieces)
                .with_terrain_adjustment(definition.terrain_adjustment),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::noise::biome::NoiseBiomeSource;
    use crate::noise::stronghold::{StrongholdPieceKind, STRONGHOLD_ID, STRONGHOLD_SET_ID};
    use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

    use super::*;

    const SEED_ZERO_RING: [i32; 2] = [-13, -106];

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

    #[test]
    fn seed_zero_ring_chunk_with_plains_emits_start_and_portal() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let starts = plan.generate_missing_concentric_ring_stronghold_starts(
            STRONGHOLD_SET_ID,
            0,
            SEED_ZERO_RING[0],
            SEED_ZERO_RING[1],
            &[],
            &[SEED_ZERO_RING],
            63,
            -64,
            |_| Some("minecraft:plains".to_string()),
        );
        assert_eq!(starts.len(), 1);
        let start = &starts[0];
        assert_eq!(start.structure_id(), STRONGHOLD_ID);
        assert_eq!(start.start_chunk(), SEED_ZERO_RING);
        assert!(start
            .pieces()
            .iter()
            .any(|piece| piece.kind() == StrongholdPieceKind::Start
                && piece.piece_id() == "minecraft:shstart"));
        assert!(start
            .pieces()
            .iter()
            .any(|piece| piece.kind() == StrongholdPieceKind::PortalRoom
                && piece.piece_id() == "minecraft:shpr"));
        assert!(start.intersects_chunk(SEED_ZERO_RING[0], SEED_ZERO_RING[1]));
    }

    #[test]
    fn nether_wastes_misses_without_generating_pieces() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let starts = plan.generate_missing_concentric_ring_stronghold_starts(
            STRONGHOLD_SET_ID,
            0,
            SEED_ZERO_RING[0],
            SEED_ZERO_RING[1],
            &[],
            &[SEED_ZERO_RING],
            63,
            -64,
            |_| Some("minecraft:nether_wastes".to_string()),
        );
        assert!(starts.is_empty());
    }

    #[test]
    fn non_ring_chunk_misses() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let starts = plan.generate_missing_concentric_ring_stronghold_starts(
            STRONGHOLD_SET_ID,
            0,
            0,
            0,
            &[],
            &[SEED_ZERO_RING],
            63,
            -64,
            |_| Some("minecraft:plains".to_string()),
        );
        assert!(starts.is_empty());
    }

    #[test]
    fn non_jigsaw_starts_union_pieces_and_inflate_by_twelve_when_terrain_adjusting() {
        use crate::noise::nether_fossil::{NetherFossilPiece, NetherFossilStructureStart};
        use crate::noise::scattered::HorizontalDirection;
        use crate::noise::stronghold::{SmallDoorType, StrongholdPiece, StrongholdPieceExtra};
        use crate::noise::structure_pieces::JigsawRotation;
        use crate::noise::structures::StructureTerrainAdjustment;

        // Stronghold with two separated pieces + Bury terrain adjustment:
        // raw piece X spans [0..=2] and [32..=34]; union is [0..=34]; +12 inflate -> [-12..=46].
        let p0 = StrongholdPiece::new(
            StrongholdPieceKind::Start,
            [0, 10, 0, 2, 15, 2],
            HorizontalDirection::North,
            0,
            SmallDoorType::Opening,
            StrongholdPieceExtra::None,
        );
        let p1 = StrongholdPiece::new(
            StrongholdPieceKind::Straight,
            [32, 10, 0, 34, 15, 2],
            HorizontalDirection::North,
            1,
            SmallDoorType::Opening,
            StrongholdPieceExtra::None,
        );
        let stronghold =
            StrongholdStructureStart::from_generation(STRONGHOLD_ID, [0, 0], 0, vec![p0, p1])
                .with_terrain_adjustment(StructureTerrainAdjustment::Bury);
        // Gap chunk between separated pieces is inside the unioned box.
        assert!(stronghold.intersects_chunk(1, 0));
        // Chunk (-1, 0) covers X=[-16..=-1], overlapping inflated min.x=-12.
        assert!(stronghold.intersects_chunk(-1, 0));
        // Chunk (2, 0) covers X=[32..=47]; chunk (3, 0) covers X=[48..=63], outside max.x=46.
        assert!(stronghold.intersects_chunk(2, 0));
        assert!(!stronghold.intersects_chunk(3, 0));

        // Nether fossil with BeardThin terrain adjustment inflates by 12 and updates on size mutation.
        let mut fossil = NetherFossilStructureStart::from_parts(
            [0, 0],
            0,
            StructureTerrainAdjustment::BeardThin,
            NetherFossilPiece::new(
                "minecraft:nether_fossils/fossil_1",
                BlockPos { x: 2, y: 40, z: 2 },
                JigsawRotation::None,
                [1, 1, 1],
            ),
        );
        assert!(fossil.intersects_chunk(-1, 0));
        assert!(!fossil.intersects_chunk(1, 0));
        // Growing template size via piece_mut invalidates the cached bounding box.
        fossil.piece_mut().set_template_size([10, 5, 10]);
        assert!(fossil.intersects_chunk(1, 0));
    }
}
