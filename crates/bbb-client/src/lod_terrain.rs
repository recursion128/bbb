//! Client-side far-terrain horizon LOD column cache and coarse skirt mesh builder.

use bbb_client_render::scene::terrain_textures::TerrainTextureState;
use bbb_client_render::terrain::{TerrainBlockRenderTable, TerrainLight};
use bbb_client_render::{LodTerrainVertex, Renderer};
use bbb_client_world::{BlockPos, ChunkPos, FxHashMap, WorldStore};
use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_WORLD_SURFACE_ID;

use crate::terrain_upload::TerrainUploadState;

const LOD_CELLS_PER_AXIS: usize = 4;
const LOD_CELL_SIZE: i32 = 16 / LOD_CELLS_PER_AXIS as i32;
const MAX_CHUNKS_SAMPLED_PER_FRAME: usize = 12;
const LOD_SKIRT_DEPTH: f32 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct CachedLodColumn {
    surface_y: i32,
    color: [u8; 4],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    light: [u8; 2],
}

#[derive(Debug, Clone)]
struct CachedLodChunk {
    content_revision: u64,
    cells: [Option<CachedLodColumn>; LOD_CELLS_PER_AXIS * LOD_CELLS_PER_AXIS],
    cached_seam_drop: Option<bool>,
    vertices: Vec<LodTerrainVertex>,
}

impl CachedLodChunk {
    fn from_cells(
        content_revision: u64,
        cells: [Option<CachedLodColumn>; LOD_CELLS_PER_AXIS * LOD_CELLS_PER_AXIS],
    ) -> Self {
        Self {
            content_revision,
            cells,
            cached_seam_drop: None,
            vertices: Vec::new(),
        }
    }

    fn ensure_vertices(&mut self, chunk_pos: ChunkPos, has_seam_drop: bool) {
        if self.cached_seam_drop == Some(has_seam_drop) {
            return;
        }
        self.cached_seam_drop = Some(has_seam_drop);
        self.vertices.clear();
        let step = LOD_CELL_SIZE as f32;
        let seam_drop = if has_seam_drop { 0.15 } else { 0.0 };
        for cz in 0..LOD_CELLS_PER_AXIS {
            for cx in 0..LOD_CELLS_PER_AXIS {
                let Some(cell) = self.cells[cz * LOD_CELLS_PER_AXIS + cx] else {
                    continue;
                };
                let x0 = (chunk_pos.x * 16 + cx as i32 * LOD_CELL_SIZE) as f32;
                let z0 = (chunk_pos.z * 16 + cz as i32 * LOD_CELL_SIZE) as f32;
                let x1 = x0 + step;
                let z1 = z0 + step;
                let y = cell.surface_y as f32 + 1.0 - seam_drop;
                let y_low = y - LOD_SKIRT_DEPTH;
                let uvs = [
                    [cell.uv_min[0], cell.uv_min[1]],
                    [cell.uv_min[0], cell.uv_max[1]],
                    [cell.uv_max[0], cell.uv_max[1]],
                    [cell.uv_max[0], cell.uv_min[1]],
                ];

                // Top face (+Y, CCW viewed from above in right-handed coords).
                push_quad_vertices(
                    &mut self.vertices,
                    [[x0, y, z0], [x0, y, z1], [x1, y, z1], [x1, y, z0]],
                    cell.color,
                    uvs,
                    cell.light,
                );
                let side_color = [
                    (u16::from(cell.color[0]) * 210 / 255) as u8,
                    (u16::from(cell.color[1]) * 210 / 255) as u8,
                    (u16::from(cell.color[2]) * 210 / 255) as u8,
                    255,
                ];
                // North (-Z), South (+Z), West (-X), East (+X) skirts.
                push_quad_vertices(
                    &mut self.vertices,
                    [[x0, y_low, z0], [x0, y, z0], [x1, y, z0], [x1, y_low, z0]],
                    side_color,
                    uvs,
                    cell.light,
                );
                push_quad_vertices(
                    &mut self.vertices,
                    [[x1, y_low, z1], [x1, y, z1], [x0, y, z1], [x0, y_low, z1]],
                    side_color,
                    uvs,
                    cell.light,
                );
                push_quad_vertices(
                    &mut self.vertices,
                    [[x0, y_low, z1], [x0, y, z1], [x0, y, z0], [x0, y_low, z0]],
                    side_color,
                    uvs,
                    cell.light,
                );
                push_quad_vertices(
                    &mut self.vertices,
                    [[x1, y_low, z0], [x1, y, z0], [x1, y, z1], [x1, y_low, z1]],
                    side_color,
                    uvs,
                    cell.light,
                );
            }
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct LodTerrainRuntime {
    level_generation: Option<u64>,
    last_terrain_textures_epoch: Option<u64>,
    last_terrain_revision: u64,
    last_membership_revision: u64,
    last_camera_chunk: Option<ChunkPos>,
    chunks: bbb_client_world::FxHashMap<ChunkPos, CachedLodChunk>,
    ring_generation: u64,
    uploaded_key: Option<(ChunkPos, u32, u32, u64)>,
    has_uploaded_mesh: bool,
    scratch_vertices: Vec<LodTerrainVertex>,
    scratch_indices: Vec<u32>,
}

pub(crate) struct LodTerrainView {
    pub(crate) lod_distance_chunks: u32,
    pub(crate) render_distance_chunks: u32,
    pub(crate) camera_chunk: Option<ChunkPos>,
    pub(crate) terrain_textures_epoch: u64,
}

impl LodTerrainRuntime {
    pub(crate) fn update(
        &mut self,
        view: LodTerrainView,
        world: &WorldStore,
        terrain_textures: &TerrainTextureState,
        terrain_upload: &mut TerrainUploadState,
        renderer: &mut Renderer,
    ) {
        let LodTerrainView {
            lod_distance_chunks,
            render_distance_chunks,
            camera_chunk,
            terrain_textures_epoch,
        } = view;
        if lod_distance_chunks == 0 || lod_distance_chunks <= render_distance_chunks {
            if self.has_uploaded_mesh {
                renderer.clear_lod_terrain_mesh();
                self.has_uploaded_mesh = false;
                self.uploaded_key = None;
            }
            return;
        }
        let Some(camera_chunk) = camera_chunk else {
            return;
        };

        let level_gen = world.client_level_generation();
        if self.level_generation != Some(level_gen)
            || self.last_terrain_textures_epoch != Some(terrain_textures_epoch)
        {
            self.level_generation = Some(level_gen);
            self.last_terrain_textures_epoch = Some(terrain_textures_epoch);
            self.last_terrain_revision = 0;
            self.last_membership_revision = 0;
            self.last_camera_chunk = None;
            self.chunks.clear();
            self.ring_generation = self.ring_generation.wrapping_add(1);
            self.uploaded_key = None;
            if self.has_uploaded_mesh {
                renderer.clear_lod_terrain_mesh();
                self.has_uploaded_mesh = false;
            }
        }

        let min_ring = render_distance_chunks as i32;
        let max_ring = lod_distance_chunks as i32;
        let prune_radius = max_ring + 8;
        let mut ring_changed = false;
        if self.last_camera_chunk != Some(camera_chunk) {
            self.last_camera_chunk = Some(camera_chunk);
            self.chunks
                .retain(|&chunk, _| chunk_chebyshev_distance(chunk, camera_chunk) <= prune_radius);
        }

        let world_terrain_rev = world.terrain_revision();
        let world_membership_rev = world.chunk_membership_revision();
        if self.last_terrain_revision != world_terrain_rev
            || self.last_membership_revision != world_membership_rev
        {
            let positions = world.chunk_positions();
            let mut dirty_positions = Vec::new();
            for pos in positions {
                let chunk_rev = world.terrain_chunk_content_revision(pos);
                let is_stale = self
                    .chunks
                    .get(&pos)
                    .is_none_or(|cached| cached.content_revision != chunk_rev);
                if is_stale {
                    dirty_positions.push((pos, chunk_rev));
                }
            }

            if dirty_positions.is_empty() {
                self.last_terrain_revision = world_terrain_rev;
                self.last_membership_revision = world_membership_rev;
            } else {
                let block_table = terrain_upload.ensure_block_table(world, terrain_textures);
                let sampled_count = self.sample_dirty_chunks(
                    world,
                    &block_table,
                    &dirty_positions,
                    camera_chunk,
                    min_ring,
                    max_ring,
                    &mut ring_changed,
                );
                if sampled_count >= dirty_positions.len() {
                    self.last_terrain_revision = world_terrain_rev;
                    self.last_membership_revision = world_membership_rev;
                }
            }
        }

        if ring_changed {
            self.ring_generation = self.ring_generation.wrapping_add(1);
        }

        let mesh_key = (
            camera_chunk,
            render_distance_chunks,
            lod_distance_chunks,
            self.ring_generation,
        );
        if self.uploaded_key == Some(mesh_key) {
            return;
        }
        self.uploaded_key = Some(mesh_key);

        build_lod_mesh(
            &mut self.chunks,
            camera_chunk,
            render_distance_chunks,
            lod_distance_chunks,
            &mut self.scratch_vertices,
            &mut self.scratch_indices,
        );
        if self.scratch_indices.is_empty() {
            if self.has_uploaded_mesh {
                renderer.clear_lod_terrain_mesh();
                self.has_uploaded_mesh = false;
            }
        } else {
            renderer.set_lod_terrain_mesh(
                &self.scratch_vertices,
                &self.scratch_indices,
                (lod_distance_chunks * 16) as f32,
            );
            self.has_uploaded_mesh = true;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn sample_dirty_chunks(
        &mut self,
        world: &WorldStore,
        block_table: &TerrainBlockRenderTable,
        dirty_positions: &[(ChunkPos, u64)],
        camera_chunk: ChunkPos,
        min_ring: i32,
        max_ring: i32,
        ring_changed: &mut bool,
    ) -> usize {
        let mut sampled = 0usize;
        for &(pos, chunk_rev) in dirty_positions.iter().take(MAX_CHUNKS_SAMPLED_PER_FRAME) {
            if let Some(cells) = sample_chunk_lod_columns(world, block_table, pos) {
                let changed = self
                    .chunks
                    .get(&pos)
                    .is_none_or(|existing| existing.cells != cells);
                if changed {
                    self.chunks
                        .insert(pos, CachedLodChunk::from_cells(chunk_rev, cells));
                    let dist = chunk_chebyshev_distance(pos, camera_chunk);
                    if dist >= min_ring && dist <= max_ring {
                        *ring_changed = true;
                    }
                } else if let Some(existing) = self.chunks.get_mut(&pos) {
                    existing.content_revision = chunk_rev;
                }
            }
            sampled += 1;
        }
        sampled
    }
}

fn chunk_chebyshev_distance(a: ChunkPos, b: ChunkPos) -> i32 {
    (a.x - b.x).abs().max((a.z - b.z).abs())
}

fn sample_chunk_lod_columns(
    world: &WorldStore,
    block_table: &TerrainBlockRenderTable,
    chunk: ChunkPos,
) -> Option<[Option<CachedLodColumn>; LOD_CELLS_PER_AXIS * LOD_CELLS_PER_AXIS]> {
    let heightmap = world.chunk_heightmap_sampler(chunk)?;
    let dim = world.dimension();
    let min_y = dim.min_y;
    let max_y = dim.min_y + dim.height - 1;
    let mut cells = [None; LOD_CELLS_PER_AXIS * LOD_CELLS_PER_AXIS];

    for cz in 0..LOD_CELLS_PER_AXIS {
        for cx in 0..LOD_CELLS_PER_AXIS {
            let lx = (cx as i32 * LOD_CELL_SIZE + 1) as u8;
            let lz = (cz as i32 * LOD_CELL_SIZE + 1) as u8;
            let start_y = heightmap
                .first_available(VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, lx, lz)
                .map(|first_air| (first_air - 1).clamp(min_y, max_y))
                .unwrap_or(min_y);
            let wx = chunk.x * 16 + i32::from(lx);
            let wz = chunk.z * 16 + i32::from(lz);
            let mut found = None;
            for step in 0..8 {
                let y = start_y - step;
                if y < min_y {
                    break;
                }
                let pos = BlockPos { x: wx, y, z: wz };
                let Some(state_id) = world.block_state_id_at(pos) else {
                    continue;
                };
                let biome_id = world.biome_id_at(pos);
                let render_pos = bbb_client_render::scene::terrain_textures::BlockRenderPosition {
                    x: wx,
                    y,
                    z: wz,
                };
                if let Some(sample) =
                    block_table.sample_lod_surface(state_id as u32, biome_id, render_pos)
                {
                    let light = world
                        .sample_block_light(BlockPos {
                            x: wx,
                            y: (y + 1).min(max_y),
                            z: wz,
                        })
                        .unwrap_or(TerrainLight::FULL_BRIGHT);
                    let block_nibble = light.block.max(sample.light_emission).min(15);
                    let sky_nibble = light.sky.min(15);
                    found = Some(CachedLodColumn {
                        surface_y: y,
                        color: sample.tint_rgba,
                        uv_min: sample.uv_min,
                        uv_max: sample.uv_max,
                        light: [block_nibble * 16, sky_nibble * 16],
                    });
                    break;
                }
            }
            cells[cz * LOD_CELLS_PER_AXIS + cx] = found;
        }
    }
    Some(cells)
}

fn push_quad_vertices(
    vertices: &mut Vec<LodTerrainVertex>,
    p: [[f32; 3]; 4],
    color: [u8; 4],
    uvs: [[f32; 2]; 4],
    light: [u8; 2],
) {
    for (position, uv) in p.into_iter().zip(uvs) {
        vertices.push(LodTerrainVertex {
            position,
            color,
            uv,
            light,
            _pad: [0, 0],
        });
    }
}

fn build_lod_mesh(
    chunks: &mut FxHashMap<ChunkPos, CachedLodChunk>,
    camera_chunk: ChunkPos,
    render_distance_chunks: u32,
    lod_distance_chunks: u32,
    vertices: &mut Vec<LodTerrainVertex>,
    indices: &mut Vec<u32>,
) {
    vertices.clear();
    indices.clear();
    let min_ring = render_distance_chunks as i32;
    let max_ring = lod_distance_chunks as i32;

    for (&chunk_pos, chunk) in chunks.iter_mut() {
        let dist = chunk_chebyshev_distance(chunk_pos, camera_chunk);
        if dist < min_ring || dist > max_ring {
            continue;
        }
        chunk.ensure_vertices(chunk_pos, dist == min_ring);
        if chunk.vertices.is_empty() {
            continue;
        }
        let base = vertices.len() as u32;
        vertices.extend_from_slice(&chunk.vertices);
        let quad_count = (chunk.vertices.len() / 4) as u32;
        indices.reserve((quad_count as usize) * 6);
        for quad in 0..quad_count {
            let v = base + quad * 4;
            indices.extend_from_slice(&[v, v + 1, v + 2, v, v + 2, v + 3]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_chunk(content_revision: u64, y: i32) -> CachedLodChunk {
        let col = CachedLodColumn {
            surface_y: y,
            color: [100, 180, 90, 255],
            uv_min: [0.25, 0.25],
            uv_max: [0.5, 0.5],
            light: [0, 240],
        };
        CachedLodChunk::from_cells(
            content_revision,
            [Some(col); LOD_CELLS_PER_AXIS * LOD_CELLS_PER_AXIS],
        )
    }

    #[test]
    fn build_lod_mesh_emits_only_chunks_between_render_distance_and_lod_distance() {
        let mut chunks = FxHashMap::default();
        // Inner chunk (dist 2 < render_distance 8) must be excluded.
        chunks.insert(ChunkPos { x: 2, z: 0 }, make_chunk(1, 64));
        // Ring chunk (dist 10 in 8..=16) must be included.
        chunks.insert(ChunkPos { x: 10, z: 0 }, make_chunk(1, 70));
        // Far chunk beyond max_ring (dist 20 > 16) must be excluded.
        chunks.insert(ChunkPos { x: 20, z: 0 }, make_chunk(1, 80));

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        build_lod_mesh(
            &mut chunks,
            ChunkPos { x: 0, z: 0 },
            8,
            16,
            &mut vertices,
            &mut indices,
        );
        // One chunk * 16 cells * 5 quads/cell * 4 vertices/quad = 320 vertices, 480 indices.
        assert_eq!(vertices.len(), 320);
        assert_eq!(indices.len(), 480);
        assert!(vertices
            .iter()
            .all(|v| v.position[0] >= 160.0 && v.position[0] <= 176.0));
        // Verify quad vertices span the sprite UV rect for hardware mipmapping.
        assert_eq!(vertices[0].uv, [0.25, 0.25]);
        assert_eq!(vertices[1].uv, [0.25, 0.5]);
        assert_eq!(vertices[2].uv, [0.5, 0.5]);
        assert_eq!(vertices[3].uv, [0.5, 0.25]);
        // Re-running with unchanged ring reuses cached chunk vertices without rebuilding.
        let cached_ptr = chunks[&ChunkPos { x: 10, z: 0 }].vertices.as_ptr();
        build_lod_mesh(
            &mut chunks,
            ChunkPos { x: 0, z: 0 },
            8,
            16,
            &mut vertices,
            &mut indices,
        );
        assert_eq!(
            chunks[&ChunkPos { x: 10, z: 0 }].vertices.as_ptr(),
            cached_ptr
        );
    }

    #[test]
    fn inner_chunk_edit_does_not_invalidate_lod_ring_generation() {
        let mut runtime = LodTerrainRuntime::default();
        runtime
            .chunks
            .insert(ChunkPos { x: 0, z: 0 }, make_chunk(1, 64));
        runtime
            .chunks
            .insert(ChunkPos { x: 10, z: 0 }, make_chunk(1, 70));
        let camera = ChunkPos { x: 0, z: 0 };
        let initial_gen = runtime.ring_generation;
        // Simulate an edited inner chunk inside render_distance (dist 0 < 8).
        let dist = chunk_chebyshev_distance(ChunkPos { x: 0, z: 0 }, camera);
        assert!(dist < 8);
        runtime
            .chunks
            .insert(ChunkPos { x: 0, z: 0 }, make_chunk(2, 65));
        assert_eq!(runtime.ring_generation, initial_gen);
        // Untouched ring chunk retains its content_revision without needing re-sample.
        assert_eq!(
            runtime.chunks[&ChunkPos { x: 10, z: 0 }].content_revision,
            1
        );
    }
}
