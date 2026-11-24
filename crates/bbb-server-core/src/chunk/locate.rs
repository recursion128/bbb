mod structures;

use std::collections::BTreeSet;

use bbb_protocol::packets::BlockPos;

use crate::chunk::{ChunkSource, ChunkStore};

impl ChunkStore {
    pub fn find_closest_biome_3d(
        &self,
        origin: BlockPos,
        allowed_registry_ids: &BTreeSet<i32>,
        search_radius: i32,
        horizontal_step: i32,
        vertical_step: i32,
    ) -> Option<(BlockPos, i32)> {
        match &self.source {
            ChunkSource::Flat(generator) => {
                allowed_registry_ids.contains(&self.biome_id).then(|| {
                    // `FixedBiomeSource.java:66-75`.
                    let min_search_y = generator.min_y() + 1;
                    let max_search_y = generator.min_y() + generator.height();
                    (
                        BlockPos {
                            y: origin.y.clamp(min_search_y, max_search_y),
                            ..origin
                        },
                        self.biome_id,
                    )
                })
            }
            ChunkSource::Debug(generator) => {
                allowed_registry_ids.contains(&self.biome_id).then(|| {
                    let min_search_y = generator.min_y() + 1;
                    let max_search_y = generator.min_y() + generator.height();
                    (
                        BlockPos {
                            y: origin.y.clamp(min_search_y, max_search_y),
                            ..origin
                        },
                        self.biome_id,
                    )
                })
            }
            ChunkSource::NoiseCarvers(program) => program.find_closest_biome_3d(
                origin,
                allowed_registry_ids,
                search_radius,
                horizontal_step,
                vertical_step,
            ),
        }
    }
}
