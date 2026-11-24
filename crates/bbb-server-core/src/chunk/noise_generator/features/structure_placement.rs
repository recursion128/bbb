//! Shared region setup for borrowed structure FEATURES placement.

use bbb_protocol::packets::BlockPos;
use bbb_server_worldgen::noise::structure_pieces::JigsawBoundingBox;
use bbb_server_worldgen::noise::structure_starts::PlaceableStart;
use bbb_server_worldgen::noise::structures::NoiseStructureDecorationInvocation;
use bbb_server_worldgen::random::WorldgenRandom;
use bbb_server_worldgen::WorldGenLevel;

use super::FeatureWorldGenRegion;
use crate::chunk::noise_generator::SaveResult;
use crate::chunk::structure_status::NoiseStructureStatusCache;

impl crate::chunk::structure_generation::StructureGenerationContext<'_> {
    pub(crate) fn place_referenced_structure<S: PlaceableStart>(
        &self,
        cache: &mut NoiseStructureStatusCache,
        invocation: &NoiseStructureDecorationInvocation,
        region: &mut FeatureWorldGenRegion<'_>,
    ) -> SaveResult<()> {
        let target = region.target_chunk();
        region.prepare_structure_placement();
        let min_x = target[0].wrapping_mul(16);
        let min_z = target[1].wrapping_mul(16);
        let bounds = JigsawBoundingBox {
            min: BlockPos {
                x: min_x,
                y: region.get_min_y() + 1,
                z: min_z,
            },
            max: BlockPos {
                x: min_x.wrapping_add(15),
                y: region.get_max_y(),
                z: min_z.wrapping_add(15),
            },
        };
        let mut random = WorldgenRandom::with_seed(invocation.feature_seed);
        cache.place_referenced_starts::<S>(target, &invocation.structure, |start| {
            start.place(self.startup, region, target, bounds, &mut random);
        })?;
        Ok(())
    }
}
