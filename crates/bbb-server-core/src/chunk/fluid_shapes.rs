use crate::fluid::SharedFluidShapeCache;

use super::ChunkStore;

impl ChunkStore {
    pub fn fluid_shape_cache(&self) -> &SharedFluidShapeCache {
        &self.fluid_shape_cache
    }

    pub fn install_fluid_shape_cache(&mut self, cache: SharedFluidShapeCache) {
        self.fluid_shape_cache = cache;
    }
}
