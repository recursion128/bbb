//! Synchronous, mathematically conservative sub-section occlusion culling
//! against fully compiled, non-fading `solid_render` terrain blocks.

use std::cell::Cell;

use bbb_client_world::{BlockPos, WorldStore};
use bbb_protocol::block_state_types::{
    vanilla_block_state_support_facts, VANILLA_BLOCK_STATE_COUNT,
};

use crate::terrain_upload::TerrainUploadState;

const CAMERA_NEAR_MARGIN: f64 = 0.25;
const MIN_SEPARATION_BLOCKS: f64 = 1.25;
const AABB_TRANSVERSE_EPS: f64 = 1.0e-4;
const MAX_SLAB_LAYERS_TO_PROBE: usize = 24;
const MAX_SLAB_CELLS: i64 = 25;

/// Reads opaque `1x1x1` terrain cubes from the current client world and synced
/// `TerrainUploadState` without async lag or pop-in.
pub(crate) struct OpaqueTerrainOccluder<'a> {
    world: &'a WorldStore,
    terrain_upload: &'a TerrainUploadState,
    eye: [f64; 3],
    enabled: bool,
    cached_sections: Cell<[(i32, i32, i32, bool); 2]>,
    cache_cursor: Cell<usize>,
}

impl<'a> OpaqueTerrainOccluder<'a> {
    pub(crate) fn new(
        world: &'a WorldStore,
        terrain_upload: &'a TerrainUploadState,
        eye: [f64; 3],
    ) -> Self {
        let enabled = eye.into_iter().all(f64::is_finite)
            && !world.local_player_is_spectator()
            && !camera_near_solid_block(world, eye);
        Self {
            world,
            terrain_upload,
            eye,
            enabled,
            cached_sections: Cell::new([(i32::MAX, i32::MAX, i32::MAX, false); 2]),
            cache_cursor: Cell::new(0),
        }
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Returns `true` only when every line segment from the camera eye to
    /// every point of `[min, max]` traverses a compiled, fully opaque (`fade == 1.0`),
    /// non-stale voxel slab of `solid_render` blocks strictly between `eye` and `max`/`min`.
    pub(crate) fn is_aabb_occluded(&self, min: [f64; 3], max: [f64; 3]) -> bool {
        if !self.enabled
            || !min.into_iter().all(f64::is_finite)
            || !max.into_iter().all(f64::is_finite)
            || (0..3).any(|axis| min[axis] > max[axis])
        {
            return false;
        }

        let mut best_axis = None;
        let mut best_gap = MIN_SEPARATION_BLOCKS;
        let mut best_dir = 0_i32;
        for axis in 0..3 {
            if self.eye[axis] < min[axis] {
                let gap = min[axis] - self.eye[axis];
                if gap >= best_gap {
                    best_gap = gap;
                    best_axis = Some(axis);
                    best_dir = 1;
                }
            } else if self.eye[axis] > max[axis] {
                let gap = self.eye[axis] - max[axis];
                if gap >= best_gap {
                    best_gap = gap;
                    best_axis = Some(axis);
                    best_dir = -1;
                }
            }
        }
        let Some(w_axis) = best_axis else {
            return false;
        };
        let u_axis = (w_axis + 1) % 3;
        let v_axis = (w_axis + 2) % 3;

        let (k_start, k_end) = if best_dir > 0 {
            let first = (self.eye[w_axis] + CAMERA_NEAR_MARGIN).ceil() as i32;
            let last = (min[w_axis] + 1.0e-7).floor() as i32 - 1;
            (first, last)
        } else {
            let first = (self.eye[w_axis] - CAMERA_NEAR_MARGIN).floor() as i32 - 1;
            let last = (max[w_axis] - 1.0e-7).ceil() as i32;
            (first, last)
        };

        let layer_count = if best_dir > 0 {
            k_end - k_start + 1
        } else {
            k_start - k_end + 1
        };
        if layer_count <= 0 {
            return false;
        }

        let probe_count = (layer_count as usize).min(MAX_SLAB_LAYERS_TO_PROBE);
        if (layer_count as usize) <= MAX_SLAB_LAYERS_TO_PROBE {
            for offset in 0..probe_count {
                let k = k_start + best_dir * offset as i32;
                if self.slab_occludes(w_axis, u_axis, v_axis, k, best_dir, min, max) {
                    return true;
                }
            }
        } else {
            for offset in 0..16 {
                let k = k_start + best_dir * offset;
                if self.slab_occludes(w_axis, u_axis, v_axis, k, best_dir, min, max) {
                    return true;
                }
            }
            for offset in (0..8).rev() {
                let k = k_end - best_dir * offset;
                if self.slab_occludes(w_axis, u_axis, v_axis, k, best_dir, min, max) {
                    return true;
                }
            }
        }
        false
    }

    #[allow(clippy::too_many_arguments)]
    fn slab_occludes(
        &self,
        w_axis: usize,
        u_axis: usize,
        v_axis: usize,
        k: i32,
        dir: i32,
        min: [f64; 3],
        max: [f64; 3],
    ) -> bool {
        let near_plane = if dir > 0 {
            f64::from(k)
        } else {
            f64::from(k + 1)
        };
        let far_plane = if dir > 0 {
            f64::from(k + 1)
        } else {
            f64::from(k)
        };
        let (denom_near, denom_far) = if dir > 0 {
            (
                min[w_axis] - self.eye[w_axis],
                max[w_axis] - self.eye[w_axis],
            )
        } else {
            (
                self.eye[w_axis] - max[w_axis],
                self.eye[w_axis] - min[w_axis],
            )
        };
        if denom_near <= 1.0e-6 || denom_far <= 1.0e-6 {
            return false;
        }
        let dist_near = (near_plane - self.eye[w_axis]).abs();
        let dist_far = (far_plane - self.eye[w_axis]).abs();
        let t_near_min = (dist_near / denom_far).clamp(0.0, 1.0);
        let t_near_max = (dist_near / denom_near).clamp(0.0, 1.0);
        let t_far_min = (dist_far / denom_far).clamp(0.0, 1.0);
        let t_far_max = (dist_far / denom_near).clamp(0.0, 1.0);

        let t_min = t_near_min.min(t_far_min);
        let t_max = t_near_max.max(t_far_max);

        let (u0, u1) = shaft_cell_range(
            self.eye[u_axis],
            min[u_axis] - AABB_TRANSVERSE_EPS,
            max[u_axis] + AABB_TRANSVERSE_EPS,
            t_min,
            t_max,
        );
        let (v0, v1) = shaft_cell_range(
            self.eye[v_axis],
            min[v_axis] - AABB_TRANSVERSE_EPS,
            max[v_axis] + AABB_TRANSVERSE_EPS,
            t_min,
            t_max,
        );
        let width = i64::from(u1) - i64::from(u0) + 1;
        let height = i64::from(v1) - i64::from(v0) + 1;
        if width <= 0 || height <= 0 || width.saturating_mul(height) > MAX_SLAB_CELLS {
            return false;
        }

        let front_w = k - dir;
        let crosses_section = front_w.div_euclid(16) != k.div_euclid(16);

        for cu in u0..=u1 {
            for cv in v0..=v1 {
                let mut coords = [0; 3];
                coords[w_axis] = k;
                coords[u_axis] = cu;
                coords[v_axis] = cv;
                if !self.is_opaque_cell(coords[0], coords[1], coords[2]) {
                    return false;
                }
                if crosses_section {
                    coords[w_axis] = front_w;
                    if self.is_solid_world_block(coords[0], coords[1], coords[2])
                        && !self.is_section_ready_for_block(coords[0], coords[1], coords[2])
                    {
                        // Guard against a rendered front face having been culled against
                        // a section that is still compiling or fading in.
                        return false;
                    }
                }
            }
        }
        true
    }

    fn is_section_ready_for_block(&self, bx: i32, by: i32, bz: i32) -> bool {
        if !self.world.dimension().contains_y(by) {
            return false;
        }
        let sx = bx.div_euclid(16);
        let sy = by.div_euclid(16);
        let sz = bz.div_euclid(16);
        let cached = self.cached_sections.get();
        if cached[0].0 == sx && cached[0].1 == sy && cached[0].2 == sz {
            return cached[0].3;
        }
        if cached[1].0 == sx && cached[1].1 == sy && cached[1].2 == sz {
            return cached[1].3;
        }
        let ready = self
            .terrain_upload
            .is_section_occluder_ready(self.world, sx, sy, sz);
        let cursor = self.cache_cursor.get() & 1;
        let mut next = cached;
        next[cursor] = (sx, sy, sz, ready);
        self.cached_sections.set(next);
        self.cache_cursor.set(cursor ^ 1);
        ready
    }

    fn is_solid_world_block(&self, bx: i32, by: i32, bz: i32) -> bool {
        self.world.dimension().contains_y(by)
            && self
                .world
                .block_state_id_at(BlockPos {
                    x: bx,
                    y: by,
                    z: bz,
                })
                .is_some_and(is_solid_occluder_state)
    }

    fn is_opaque_cell(&self, bx: i32, by: i32, bz: i32) -> bool {
        self.is_section_ready_for_block(bx, by, bz) && self.is_solid_world_block(bx, by, bz)
    }
}

fn is_solid_occluder_state(state_id: i32) -> bool {
    (0..VANILLA_BLOCK_STATE_COUNT).contains(&state_id)
        && vanilla_block_state_support_facts(state_id).is_some_and(|facts| facts.solid_render())
}

fn camera_near_solid_block(world: &WorldStore, eye: [f64; 3]) -> bool {
    let min_x = (eye[0] - CAMERA_NEAR_MARGIN).floor() as i32;
    let max_x = (eye[0] + CAMERA_NEAR_MARGIN).floor() as i32;
    let min_y = (eye[1] - CAMERA_NEAR_MARGIN).floor() as i32;
    let max_y = (eye[1] + CAMERA_NEAR_MARGIN).floor() as i32;
    let min_z = (eye[2] - CAMERA_NEAR_MARGIN).floor() as i32;
    let max_z = (eye[2] + CAMERA_NEAR_MARGIN).floor() as i32;
    for bx in min_x..=max_x {
        for by in min_y..=max_y {
            if !world.dimension().contains_y(by) {
                continue;
            }
            for bz in min_z..=max_z {
                if world
                    .block_state_id_at(BlockPos {
                        x: bx,
                        y: by,
                        z: bz,
                    })
                    .is_some_and(is_solid_occluder_state)
                {
                    return true;
                }
            }
        }
    }
    false
}

fn shaft_cell_range(
    eye: f64,
    target_min: f64,
    target_max: f64,
    t_min: f64,
    t_max: f64,
) -> (i32, i32) {
    let d_min = target_min - eye;
    let d_max = target_max - eye;
    let lo_a = eye + t_min * d_min;
    let lo_b = eye + t_max * d_min;
    let hi_a = eye + t_min * d_max;
    let hi_b = eye + t_max * d_max;
    let span_min = lo_a.min(lo_b).min(hi_a).min(hi_b);
    let span_max = lo_a.max(lo_b).max(hi_a).max(hi_b);
    (span_min.floor() as i32, span_max.floor() as i32)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use bbb_client_world::{
        vanilla_air_block_state_id, vanilla_block_state_id, ChunkColumn, ChunkPos, ChunkSection,
        ChunkState, LightData, PaletteDomain, PaletteKind, PalettedContainerData,
    };
    use bbb_protocol::packets::BlockUpdate as ProtocolBlockUpdate;
    use std::collections::BTreeMap;

    pub(crate) fn stone_state_id() -> i32 {
        vanilla_block_state_id("minecraft:stone", &BTreeMap::new()).unwrap()
    }

    fn glass_state_id() -> i32 {
        vanilla_block_state_id("minecraft:glass", &BTreeMap::new()).unwrap()
    }

    fn slab_state_id() -> i32 {
        vanilla_block_state_id(
            "minecraft:stone_slab",
            &BTreeMap::from([
                ("type".to_string(), "bottom".to_string()),
                ("waterlogged".to_string(), "false".to_string()),
            ]),
        )
        .unwrap()
    }

    fn air_section() -> ChunkSection {
        ChunkSection {
            non_empty_block_count: 0,
            fluid_count: 0,
            block_states: PalettedContainerData {
                domain: PaletteDomain::BlockStates,
                bits_per_entry: 0,
                palette_kind: PaletteKind::SingleValue,
                palette_global_ids: vec![vanilla_air_block_state_id()],
                packed_data: Vec::new(),
                entry_count: 4096,
            },
            biomes: PalettedContainerData {
                domain: PaletteDomain::Biomes,
                bits_per_entry: 0,
                palette_kind: PaletteKind::SingleValue,
                palette_global_ids: vec![0],
                packed_data: Vec::new(),
                entry_count: 64,
            },
        }
    }

    pub(crate) fn populate_test_world_blocks(
        world: &mut WorldStore,
        blocks: &[(i32, i32, i32, i32)],
    ) {
        let mut seen_chunks = std::collections::BTreeSet::new();
        for &(bx, _, bz, _) in blocks {
            seen_chunks.insert(ChunkPos {
                x: bx.div_euclid(16),
                z: bz.div_euclid(16),
            });
        }
        let section_count = world.dimension().section_count();
        let empty_section = std::sync::Arc::new(air_section());
        for pos in seen_chunks {
            world.insert_decoded_chunk(ChunkColumn {
                pos,
                state: ChunkState::Decoded,
                heightmaps: Vec::new(),
                sections: vec![empty_section.clone(); section_count],
                block_entities: std::sync::Arc::default(),
                light: LightData::default(),
            });
        }
        for &(bx, by, bz, block_state_id) in blocks {
            assert!(world.apply_block_update(ProtocolBlockUpdate {
                pos: BlockPos {
                    x: bx,
                    y: by,
                    z: bz,
                },
                block_state_id,
            }));
        }
    }

    pub(crate) fn test_world_with_blocks(blocks: &[(i32, i32, i32, i32)]) -> WorldStore {
        let mut world = WorldStore::new();
        populate_test_world_blocks(&mut world, blocks);
        world
    }

    #[test]
    fn solid_stone_wall_occludes_target_behind_it_and_open_gap_does_not() {
        let stone = stone_state_id();
        let mut blocks = Vec::new();
        for y in 63..=67 {
            for x in -2..=2 {
                blocks.push((x, y, 5, stone));
            }
        }
        let world = test_world_with_blocks(&blocks);
        let mut upload = TerrainUploadState::default();
        upload.mark_section_occluder_ready_for_test(&world, 0, 3, 0, 1.0);
        upload.mark_section_occluder_ready_for_test(&world, 0, 4, 0, 1.0);
        upload.mark_section_occluder_ready_for_test(&world, -1, 3, 0, 1.0);
        upload.mark_section_occluder_ready_for_test(&world, -1, 4, 0, 1.0);

        let occluder = OpaqueTerrainOccluder::new(&world, &upload, [0.5, 65.5, 1.0]);
        assert!(occluder.is_aabb_occluded([0.1, 64.0, 7.0], [0.9, 65.8, 7.8]));

        // Punch a single air block hole in the middle of the wall at (0, 65, 5):
        // shaft intersection must detect the window and avoid culling.
        let mut windowed = test_world_with_blocks(&blocks);
        assert!(windowed.apply_block_update(ProtocolBlockUpdate {
            pos: BlockPos { x: 0, y: 65, z: 5 },
            block_state_id: vanilla_air_block_state_id(),
        }));
        upload.mark_section_occluder_ready_for_test(&windowed, 0, 4, 0, 1.0);
        upload.mark_section_occluder_ready_for_test(&windowed, -1, 4, 0, 1.0);
        let windowed_occluder = OpaqueTerrainOccluder::new(&windowed, &upload, [0.5, 65.5, 1.0]);
        assert!(!windowed_occluder.is_aabb_occluded([0.1, 64.0, 7.0], [0.9, 65.8, 7.8]));
    }

    #[test]
    fn non_solid_blocks_fading_sections_and_camera_in_block_never_cull() {
        let stone = stone_state_id();
        let glass = glass_state_id();
        let slab = slab_state_id();
        for non_solid in [glass, slab] {
            let mut blocks = Vec::new();
            for y in 63..=67 {
                for x in -2..=2 {
                    blocks.push((x, y, 5, if x == 0 && y == 65 { non_solid } else { stone }));
                }
            }
            let world = test_world_with_blocks(&blocks);
            let mut upload = TerrainUploadState::default();
            upload.mark_section_occluder_ready_for_test(&world, 0, 4, 0, 1.0);
            upload.mark_section_occluder_ready_for_test(&world, -1, 4, 0, 1.0);
            let occluder = OpaqueTerrainOccluder::new(&world, &upload, [0.5, 65.5, 1.0]);
            assert!(!occluder.is_aabb_occluded([0.1, 64.2, 7.0], [0.9, 65.8, 7.8]));
        }

        // Fading chunk section (< 1.0) must never occlude.
        let mut solid_blocks = Vec::new();
        for y in 63..=67 {
            for x in -2..=2 {
                solid_blocks.push((x, y, 5, stone));
            }
        }
        let world = test_world_with_blocks(&solid_blocks);
        let mut fading_upload = TerrainUploadState::default();
        fading_upload.mark_section_occluder_ready_for_test(&world, 0, 4, 0, 0.5);
        fading_upload.mark_section_occluder_ready_for_test(&world, -1, 4, 0, 1.0);
        let fading_occluder = OpaqueTerrainOccluder::new(&world, &fading_upload, [0.5, 65.5, 1.0]);
        assert!(!fading_occluder.is_aabb_occluded([0.1, 64.2, 7.0], [0.9, 65.8, 7.8]));

        // Camera eye overlapping a solid block disables occlusion culling cleanly.
        solid_blocks.push((0, 65, 1, stone));
        let clipped_world = test_world_with_blocks(&solid_blocks);
        let mut upload = TerrainUploadState::default();
        upload.mark_section_occluder_ready_for_test(&clipped_world, 0, 4, 0, 1.0);
        upload.mark_section_occluder_ready_for_test(&clipped_world, -1, 4, 0, 1.0);
        let clipped_occluder =
            OpaqueTerrainOccluder::new(&clipped_world, &upload, [0.5, 65.5, 1.0]);
        assert!(!clipped_occluder.is_enabled());
        assert!(!clipped_occluder.is_aabb_occluded([0.1, 64.2, 7.0], [0.9, 65.8, 7.8]));
    }
}
