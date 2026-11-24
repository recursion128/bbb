//! ChunkGenerator terrain queries for live placement commands.

use super::*;

impl crate::chunk::ChunkStore {
    pub fn command_structure_terrain_adjustment(
        &self,
        id: &str,
    ) -> SaveResult<bbb_server_worldgen::noise::structures::StructureTerrainAdjustment> {
        let context = match &self.source {
            crate::chunk::ChunkSource::NoiseCarvers(program) => {
                program.structure_generation_context()?
            }
            crate::chunk::ChunkSource::Flat(_) | crate::chunk::ChunkSource::Debug(_) => self
                .ordinary_structures
                .as_ref()
                .ok_or_else(|| invalid_schema("structure.command", "frozen resources are absent"))?
                .context(&self.source),
        };
        context
            .plan
            .structures()
            .get(id)
            .map(|definition| definition.terrain_adjustment)
            .ok_or_else(|| invalid_schema("structure.command", "structure definition is absent"))
    }

    pub fn command_generation_bounds(&self) -> (i32, i32) {
        let (min_y, height) = match &self.source {
            crate::chunk::ChunkSource::NoiseCarvers(program) => (program.min_y(), program.height()),
            crate::chunk::ChunkSource::Flat(_) | crate::chunk::ChunkSource::Debug(_) => (0, 384),
        };
        (
            min_y.max(self.min_y()),
            height.min(self.section_count() as i32 * 16),
        )
    }
}

/// The live level as `ChunkGenerator.getFirstFreeHeight` for `/place jigsaw`.
impl JigsawHeightAccess for crate::chunk::ChunkStore {
    fn first_free_height(&self, x: i32, z: i32, heightmap: StructureHeightmap) -> Option<i32> {
        let kind = match heightmap {
            StructureHeightmap::WorldSurfaceWg => VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID,
            StructureHeightmap::OceanFloorWg => VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID,
            _ => return None,
        };
        match &self.source {
            crate::chunk::ChunkSource::NoiseCarvers(program) => {
                program.fresh_base_height(x, z, kind)
            }
            crate::chunk::ChunkSource::Flat(flat) => flat.base_height(kind),
            crate::chunk::ChunkSource::Debug(_) => Some(0),
        }
    }

    fn generation_bounds(&self) -> Option<(i32, i32)> {
        Some(self.command_generation_bounds())
    }
}
