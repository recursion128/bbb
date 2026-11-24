//! Mineshaft `StructureStart` materialization and biome-gated selection.

use std::collections::BTreeSet;

use bbb_protocol::{normalize_resource_location, packets::BlockPos};

use super::{MineshaftPieceGraph, MineshaftStructureStart};
use crate::noise::structure_pieces::{
    CachedStartBoundingBox, JigsawBoundingBox, JigsawHeightAccess,
};
use crate::noise::structures::{NoiseStructurePlan, StructureStartSetOwner};

impl MineshaftStructureStart {
    /// Materializes an ordered graph only after the selected structure has a
    /// valid generation stub (`Structure.java:83-118`).
    pub fn from_graph(
        plan: &NoiseStructurePlan,
        graph: MineshaftPieceGraph,
        references: i32,
    ) -> Option<Self> {
        Some(Self {
            terrain_adjustment: plan
                .structures()
                .get(&graph.structure_id)?
                .terrain_adjustment,
            structure_id: graph.structure_id,
            start_chunk: graph.start_chunk,
            references,
            pieces: graph.pieces,
            cached_bounding_box: CachedStartBoundingBox::new(),
        })
    }

    pub fn bounding_box(&self) -> JigsawBoundingBox {
        self.cached_bounding_box
            .get_or_init(|| {
                JigsawBoundingBox::adjusted_union(
                    self.pieces.iter().map(|piece| piece.bounding_box),
                    self.terrain_adjustment,
                )
            })
            .expect("mineshaft start has at least one piece")
    }

    pub fn intersects_chunk(&self, chunk_x: i32, chunk_z: i32) -> bool {
        self.bounding_box().intersects_chunk(chunk_x, chunk_z)
    }
}

impl NoiseStructurePlan {
    /// Runs the mineshaft-owned random-spread half of `createStructures` and
    /// biome-gates the completed generation stub (`ChunkGenerator.java:476-589`,
    /// `Structure.java:131-140`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_missing_random_spread_mineshaft_starts(
        &self,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        min_y: i32,
        sea_level: i32,
        existing_starts: &[MineshaftStructureStart],
        height_access: &dyn JigsawHeightAccess,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
    ) -> Vec<MineshaftStructureStart> {
        let mut existing_ids: BTreeSet<String> = existing_starts
            .iter()
            .map(|start| start.structure_id.clone())
            .collect();

        let mut generated = Vec::new();
        for set in self.sets() {
            if self.structure_start_owner(set) != StructureStartSetOwner::Mineshaft
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
                let Some(start) = self.mineshaft_start(
                    &structure_id,
                    level_seed,
                    chunk_x,
                    chunk_z,
                    min_y,
                    sea_level,
                    height_access,
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
    pub fn generate_forced_mineshaft_start(
        &self,
        structure_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        min_y: i32,
        sea_level: i32,
        height_access: &dyn JigsawHeightAccess,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
    ) -> Option<MineshaftStructureStart> {
        self.mineshaft_start(
            structure_id,
            level_seed,
            chunk_x,
            chunk_z,
            min_y,
            sea_level,
            height_access,
            &mut biome_at,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn mineshaft_start(
        &self,
        structure_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        min_y: i32,
        sea_level: i32,
        height_access: &dyn JigsawHeightAccess,
        biome_at: &mut impl FnMut(BlockPos) -> Option<String>,
        check_allowed_biome: bool,
    ) -> Option<MineshaftStructureStart> {
        let graph = self.mineshaft_piece_graph(
            structure_id,
            level_seed,
            chunk_x,
            chunk_z,
            min_y,
            sea_level,
            height_access,
        )?;
        let biome = normalize_resource_location(biome_at(graph.generation_anchor)?).ok()?;
        if check_allowed_biome && !self.structures().get(structure_id)?.biomes.contains(&biome) {
            return None;
        }
        MineshaftStructureStart::from_graph(self, graph, 0)
    }
}
