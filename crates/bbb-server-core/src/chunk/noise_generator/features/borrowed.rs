//! Short-lived FEATURES access to the canonical store.

use super::*;
use crate::chunk::poi::{PoiBlockChange, PoiManager};
use crate::chunk::Chunk;

type ChunkKey = (i32, i32);

pub(crate) struct FeatureHeightmapQueries {
    pub(super) ocean_floor_wg: [i32; 256],
    pub(super) world_surface_wg: [i32; 256],
    pub(super) carvers: Option<FeatureHeightmaps>,
}

pub(crate) struct FeatureRegionData<'a> {
    pub(super) chunks: &'a mut rustc_hash::FxHashMap<ChunkKey, Chunk>,
    pub(super) block_entities: &'a mut LevelBlockEntities,
    pub(super) poi: Option<&'a mut PoiManager>,
    pub(crate) poi_changes: Vec<PoiBlockChange>,
    pub(super) pending_light_changes:
        &'a mut rustc_hash::FxHashMap<ChunkKey, rustc_hash::FxHashSet<(i32, i32, i32)>>,
    pub(super) heightmaps: rustc_hash::FxHashMap<ChunkKey, FeatureHeightmapQueries>,
    pub(crate) dirty: BTreeSet<ChunkKey>,
}

impl<'a> FeatureRegionData<'a> {
    pub(crate) fn new(
        chunks: &'a mut rustc_hash::FxHashMap<ChunkKey, Chunk>,
        block_entities: &'a mut LevelBlockEntities,
        poi: &'a mut PoiManager,
        pending_light_changes: &'a mut rustc_hash::FxHashMap<
            ChunkKey,
            rustc_hash::FxHashSet<(i32, i32, i32)>,
        >,
        region: &BTreeSet<ChunkKey>,
    ) -> Self {
        Self::with_optional_poi(
            chunks,
            block_entities,
            Some(poi),
            pending_light_changes,
            region,
        )
    }

    pub(crate) fn new_isolated(
        chunks: &'a mut rustc_hash::FxHashMap<ChunkKey, Chunk>,
        block_entities: &'a mut LevelBlockEntities,
        pending_light_changes: &'a mut rustc_hash::FxHashMap<
            ChunkKey,
            rustc_hash::FxHashSet<(i32, i32, i32)>,
        >,
        region: &BTreeSet<ChunkKey>,
    ) -> Self {
        Self::with_optional_poi(chunks, block_entities, None, pending_light_changes, region)
    }

    fn with_optional_poi(
        chunks: &'a mut rustc_hash::FxHashMap<ChunkKey, Chunk>,
        block_entities: &'a mut LevelBlockEntities,
        poi: Option<&'a mut PoiManager>,
        pending_light_changes: &'a mut rustc_hash::FxHashMap<
            ChunkKey,
            rustc_hash::FxHashSet<(i32, i32, i32)>,
        >,
        region: &BTreeSet<ChunkKey>,
    ) -> Self {
        let heightmaps = region
            .iter()
            .map(|&chunk| {
                let loaded = &chunks[&chunk];
                // The column's own heightmaps are primed once and kept
                // current, so the windows sharing a column do not re-prime it.
                let maps = loaded.heightmaps();
                let carvers = loaded.generation.as_ref().unwrap().features().is_none();
                (
                    chunk,
                    FeatureHeightmapQueries {
                        ocean_floor_wg: *maps.values(VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID).unwrap(),
                        world_surface_wg: *maps.values(VANILLA_HEIGHTMAP_WORLD_SURFACE_ID).unwrap(),
                        carvers: carvers.then(|| maps.clone()),
                    },
                )
            })
            .collect();
        Self {
            chunks,
            block_entities,
            poi,
            poi_changes: Vec::new(),
            pending_light_changes,
            heightmaps,
            dirty: BTreeSet::new(),
        }
    }

    pub(crate) fn complete_target(&mut self, target: ChunkKey) {
        if let Some(maps) = self.heightmaps.get_mut(&target).unwrap().carvers.take() {
            self.chunks
                .get_mut(&target)
                .unwrap()
                .generation
                .as_mut()
                .unwrap()
                .complete_features(maps);
            self.dirty.insert(target);
        }
    }

    pub(crate) fn region<'b>(
        &'b mut self,
        source: &FeaturePlacementSource<'b>,
        target: [i32; 2],
    ) -> SaveResult<FeatureWorldGenRegion<'b>> {
        Ok(FeatureWorldGenRegion::new(
            target,
            source.world_seed,
            source.sea_level,
            source.resources,
            source.tags,
            self.chunks,
            self.block_entities,
            &mut self.poi_changes,
            self.pending_light_changes,
            &mut self.heightmaps,
            &mut self.dirty,
            &source.resources.registries.worldgen.biome_climates,
            source.publication,
        ))
    }
}

impl Drop for FeatureRegionData<'_> {
    fn drop(&mut self) {
        if let Some(poi) = self.poi.as_deref_mut() {
            if !self.poi_changes.is_empty() {
                poi.apply_changes(std::mem::take(&mut self.poi_changes));
            }
        }
    }
}
