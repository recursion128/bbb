mod emitter;
mod fluid;
pub(crate) mod geometry;
mod item_bake;
mod lighting;

pub(in crate::terrain) use item_bake::{bake_block_item_quads, bake_moving_block_quads};

use self::{
    emitter::{
        effective_face_material, emit_box, emit_cross, emit_face, emit_quads, quad_lighting,
    },
    geometry::FACES,
};
use super::{
    block_table::{
        CellRef, TerrainBlockState, TerrainCellRender, MESH_ROUTE_CUTOUT, MESH_ROUTE_FLUID_OVERLAY,
        MESH_ROUTE_OPAQUE, MESH_ROUTE_TRANSLUCENT,
    },
    face_occlusion_culls, TerrainBlockRenderTable, TerrainChunkSnapshot, TerrainFace, TerrainFluid,
    TerrainLight, TerrainMaterialClass, TerrainMesh, TerrainRenderShape, TerrainSkipRendering,
    TerrainTextureAtlas,
};
use crate::scene::{
    biome_tint::SectionBiomeTintCache,
    terrain_textures::{BlockRenderPosition, SectionTintAccess},
};

/// One section's output for the three terrain render layers.
pub(super) struct TerrainLayerMeshes {
    pub(super) opaque: TerrainMesh,
    pub(super) cutout: TerrainMesh,
    pub(super) translucent: TerrainMesh,
    pub(super) local_offset: [f32; 3],
}

impl TerrainLayerMeshes {
    fn layer(&mut self, material: TerrainMaterialClass) -> Option<&mut TerrainMesh> {
        match material {
            TerrainMaterialClass::Empty => None,
            TerrainMaterialClass::Opaque => Some(&mut self.opaque),
            TerrainMaterialClass::Cutout => Some(&mut self.cutout),
            TerrainMaterialClass::Fluid | TerrainMaterialClass::Translucent => {
                Some(&mut self.translucent)
            }
        }
    }
}

/// The meshed cell: its state, model, light and resolved tints.
struct EmitCell<'a> {
    state: &'a TerrainBlockState,
    render: TerrainCellRender<'a>,
    light: TerrainLight,
}

/// Meshes one section in y, z, x order. Each quad goes to the layer of its
/// face material, so every layer keeps the emission order of a per-layer walk;
/// a cell's fluid overlay precedes its own translucent geometry.
pub(super) fn build_section_mesh_layers(
    snapshot: &TerrainChunkSnapshot,
    lookup: &TerrainChunkLookup<'_>,
    section_y: i32,
    shared_quad_indices: bool,
) -> Option<TerrainLayerMeshes> {
    let section_min_y = section_y.checked_mul(16)?;
    let start_y = usize::try_from(section_min_y.checked_sub(snapshot.min_y)?).ok()?;
    if start_y >= snapshot.height {
        return None;
    }
    let end_y = (start_y + 16).min(snapshot.height);
    let section_mesh = |shared_quad_indices| TerrainMesh {
        shared_quad_indices,
        source_sections: 1,
        ..TerrainMesh::default()
    };
    let mut out = TerrainLayerMeshes {
        opaque: section_mesh(shared_quad_indices),
        cutout: section_mesh(shared_quad_indices),
        translucent: section_mesh(false),
        local_offset: [0.0; 3],
    };
    let table = lookup.table;
    if snapshot
        .uniform_block_state_id(section_y)
        .is_some_and(|id| table.state(id).fixed_model_has_no_mesh())
    {
        return Some(out);
    }
    let center_view = lookup
        .center_section_view(section_y)
        .or_else(|| snapshot.section_view(section_y))?;
    let atlas = table.atlas();
    let cardinal = snapshot.cardinal_lighting;
    let biome_at = |position: BlockRenderPosition| lookup.biome_id(position);
    let sample_quart_biome = |world_x: i32, qy: usize, world_z: i32| {
        lookup.biome_id(BlockRenderPosition {
            x: world_x,
            y: section_min_y + (qy as i32) * 4,
            z: world_z,
        })
    };
    let uniform_qy = lookup.quart_y_uniform_flags(section_min_y);
    thread_local! {
        static TINT_CACHE: std::cell::RefCell<SectionBiomeTintCache> =
            std::cell::RefCell::new(SectionBiomeTintCache::default());
    }
    TINT_CACHE.with_borrow_mut(|tint_cache| {
        tint_cache.begin_section(
            snapshot.chunk_x,
            snapshot.chunk_z,
            table.biome_blend_radius(),
        );
        let mut tint_access = SectionTintAccess {
            cache: tint_cache,
            uniform_qy,
            sample_quart_biome: &sample_quart_biome,
            fallback_biome_at: &biome_at,
        };
        for y in start_y..end_y {
            let world_y = snapshot.min_y + y as i32;
            let local_y = world_y.rem_euclid(16) as usize;
            for z in 0..16_usize {
                for x in 0..16_usize {
                    let block_index = (local_y << 8) | (z << 4) | x;
                    let block_state_id = lookup
                        .center_block_state_id(block_index)
                        .unwrap_or_else(|| center_view.block_state_id_at(block_index));
                    let state = table.state(block_state_id);
                    if state.fixed_model_has_no_mesh() {
                        continue;
                    }
                    let world_x = snapshot.chunk_x * 16 + x as i32;
                    let world_z = snapshot.chunk_z * 16 + z as i32;
                    let position = BlockRenderPosition {
                        x: world_x,
                        y: world_y,
                        z: world_z,
                    };
                    let model = table.model(state, position);
                    let route = model.route;
                    if route == 0 {
                        continue;
                    }
                    let quart_index = ((local_y >> 2) << 4) | ((z >> 2) << 2) | (x >> 2);
                    let cell = EmitCell {
                        state,
                        render: table.cell_render_in_section(
                            state,
                            model,
                            center_view.biome_id_at(quart_index),
                            position,
                            &mut tint_access,
                        ),
                        light: center_view.light_at(block_index),
                    };
                    if route & MESH_ROUTE_FLUID_OVERLAY != 0 {
                        emit_fluid_overlay(
                            &mut out, world_x, world_y, world_z, &cell, lookup, atlas, cardinal,
                        );
                        if route & (MESH_ROUTE_OPAQUE | MESH_ROUTE_CUTOUT | MESH_ROUTE_TRANSLUCENT)
                            == 0
                        {
                            continue;
                        }
                    }
                    emit_cell(
                        &mut out, world_x, world_y, world_z, &cell, lookup, atlas, cardinal,
                    );
                }
            }
        }
    });
    Some(out)
}

#[allow(clippy::too_many_arguments)]
fn emit_cell(
    out: &mut TerrainLayerMeshes,
    world_x: i32,
    world_y: i32,
    world_z: i32,
    cell: &EmitCell<'_>,
    lookup: &TerrainChunkLookup<'_>,
    atlas: &TerrainTextureAtlas,
    cardinal: super::TerrainCardinalLighting,
) {
    let state = cell.state;
    let render = &cell.render;
    let model = &*render.model;
    let face_tints: [_; 6] = std::array::from_fn(|face| render.face_tint(face));
    // Keep light and neighbor queries on the original block cell while offsetting emitted vertices.
    out.local_offset = if state.model_offset_limits != [0.0; 2] {
        bbb_protocol::block_state_types::vanilla_block_model_offset(
            world_x,
            world_z,
            state.model_offset_limits,
        )
        .map(|v| v as f32)
    } else {
        [0.0; 3]
    };
    match &model.render_shape {
        TerrainRenderShape::Cross {
            shade,
            light_emission,
        } => {
            emit_cross(
                out,
                world_x,
                world_y,
                world_z,
                state.material,
                cell.light,
                state.state_light_emission,
                state.emissive_rendering,
                face_tints,
                model.texture_indices,
                model.face_transparency,
                *shade,
                *light_emission,
                atlas,
                cardinal,
            );
        }
        TerrainRenderShape::Crosses(model_crosses) => {
            for (element, model_cross) in model_crosses.iter().enumerate() {
                emit_cross(
                    out,
                    world_x,
                    world_y,
                    world_z,
                    state.material,
                    cell.light,
                    state.state_light_emission,
                    state.emissive_rendering,
                    std::array::from_fn(|face| {
                        render.shape_tint(element, face, model_cross.tint[face])
                    }),
                    model_cross.texture_indices,
                    model_cross.face_transparency,
                    model_cross.shade,
                    model_cross.light_emission,
                    atlas,
                    cardinal,
                );
            }
        }
        TerrainRenderShape::Box {
            from,
            to,
            face_present,
            face_uvs,
            face_uv_rotations,
            face_shade,
            face_light_emission,
            face_cull,
            face_transparency,
        } => {
            emit_box(
                out,
                world_x,
                world_y,
                world_z,
                state.material,
                state.skip_rendering,
                state.face_occlusion_shape_indices,
                state.collision_full_block,
                state.fluid,
                cell.light,
                state.state_light_emission,
                state.emissive_rendering,
                face_tints,
                model.texture_indices,
                state.fluid_overlay_texture_index,
                atlas,
                *from,
                *to,
                *face_present,
                *face_uvs,
                *face_uv_rotations,
                *face_shade,
                *face_light_emission,
                *face_cull,
                *face_transparency,
                model.ambient_occlusion,
                lookup,
                cardinal,
            );
        }
        TerrainRenderShape::Boxes(model_boxes) => {
            for (element, model_box) in model_boxes.iter().enumerate() {
                emit_box(
                    out,
                    world_x,
                    world_y,
                    world_z,
                    state.material,
                    state.skip_rendering,
                    state.face_occlusion_shape_indices,
                    state.collision_full_block,
                    state.fluid,
                    cell.light,
                    state.state_light_emission,
                    state.emissive_rendering,
                    std::array::from_fn(|face| {
                        render.shape_tint(element, face, model_box.tint[face])
                    }),
                    model_box.texture_indices,
                    state.fluid_overlay_texture_index,
                    atlas,
                    model_box.from,
                    model_box.to,
                    model_box.face_present,
                    model_box.face_uvs,
                    model_box.face_uv_rotations,
                    model_box.face_shade,
                    model_box.face_light_emission,
                    model_box.face_cull,
                    model_box.face_transparency,
                    model.ambient_occlusion,
                    lookup,
                    cardinal,
                );
            }
        }
        TerrainRenderShape::Quads(model_quads) => {
            emit_quads(
                out,
                world_x,
                world_y,
                world_z,
                state.material,
                state.skip_rendering,
                state.face_occlusion_shape_indices,
                state.collision_full_block,
                cell.light,
                state.state_light_emission,
                state.emissive_rendering,
                model_quads,
                &|element, tint| render.shape_tint(element, 0, tint),
                atlas,
                model.ambient_occlusion,
                lookup,
                cardinal,
            );
        }
        TerrainRenderShape::Cube => {
            for face in FACES {
                let face_index = face.face.index();
                let face_material =
                    effective_face_material(state.material, model.face_transparency[face_index]);
                if matches!(face_material, TerrainMaterialClass::Empty) {
                    continue;
                }
                let neighbor = lookup.cell(world_x + face.dx, world_y + face.dy, world_z + face.dz);
                if neighbor.is_some_and(|neighbor| {
                    culls_face_between_cells(
                        state.face_occlusion_shape_indices,
                        state.skip_rendering,
                        face_material,
                        state.fluid,
                        face.face,
                        &neighbor,
                    )
                }) {
                    if let Some(mesh) = out.layer(face_material) {
                        mesh.culled_faces += 1;
                    }
                    continue;
                }
                let (vertex_lights, ambient_occlusion) = quad_lighting(
                    face.face,
                    &face.corners,
                    false,
                    Some(face.face),
                    0,
                    model.ambient_occlusion,
                    state.collision_full_block,
                    (world_x, world_y, world_z),
                    cell.light,
                    state.state_light_emission,
                    state.emissive_rendering,
                    lookup,
                );
                emit_face(
                    out,
                    world_x,
                    world_y,
                    world_z,
                    face_material,
                    face_tints[face_index],
                    atlas.rect(model.texture_indices[face_index]),
                    face,
                    vertex_lights,
                    ambient_occlusion,
                    cardinal,
                );
            }
        }
    }
    out.local_offset = [0.0; 3];
}

pub(super) fn sort_translucent_quads_by_distance(
    mesh: &mut TerrainMesh,
    camera_position: [f32; 3],
) {
    debug_assert_eq!(mesh.vertices.len() % 4, 0);
    if !mesh.vertices.len().is_multiple_of(4) {
        return;
    }

    let mut quad_distances: Vec<_> = mesh
        .vertices
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .map(|(quad_index, quad)| {
            let p0 = quad[0].local_position();
            let p2 = quad[2].local_position();
            let centroid = [
                (p0[0] + p2[0]) * 0.5,
                (p0[1] + p2[1]) * 0.5,
                (p0[2] + p2[2]) * 0.5,
            ];
            let dx = centroid[0] - camera_position[0];
            let dy = centroid[1] - camera_position[1];
            let dz = centroid[2] - camera_position[2];
            let base = (quad_index * 4) as u32;
            let index_offset = quad_index * 6;
            let local_indices = mesh
                .indices
                .get(index_offset..index_offset + 6)
                .map(|indices| std::array::from_fn(|index| indices[index] - base))
                .unwrap_or([0, 1, 2, 0, 2, 3]);
            (quad_index, dx * dx + dy * dy + dz * dz, local_indices)
        })
        .collect();
    quad_distances.sort_by(
        |(left_index, left_distance, _), (right_index, right_distance, _)| {
            right_distance
                .total_cmp(left_distance)
                .then_with(|| left_index.cmp(right_index))
        },
    );

    mesh.indices.clear();
    mesh.indices.reserve(quad_distances.len() * 6);
    for (quad_index, _, local_indices) in quad_distances {
        let base = (quad_index * 4) as u32;
        mesh.indices
            .extend(local_indices.into_iter().map(|index| base + index));
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_fluid_overlay(
    out: &mut TerrainLayerMeshes,
    world_x: i32,
    world_y: i32,
    world_z: i32,
    cell: &EmitCell<'_>,
    lookup: &TerrainChunkLookup<'_>,
    atlas: &TerrainTextureAtlas,
    cardinal: super::TerrainCardinalLighting,
) {
    let state = cell.state;
    let face_cull = TerrainFace::ALL.map(Some);
    emit_box(
        out,
        world_x,
        world_y,
        world_z,
        TerrainMaterialClass::Fluid,
        TerrainSkipRendering::NONE,
        state.face_occlusion_shape_indices,
        false,
        state.fluid,
        cell.light,
        state.state_light_emission,
        state.emissive_rendering,
        cell.render.fluid_tint,
        state.fluid_texture_indices,
        state.fluid_overlay_texture_index,
        atlas,
        [0, 0, 0],
        [16, 16, 16],
        [true; 6],
        [[0, 0, 16, 16]; 6],
        [0; 6],
        [true; 6],
        [0; 6],
        face_cull,
        [super::TerrainTransparency::OPAQUE; 6],
        false,
        lookup,
        cardinal,
    );
}

pub(super) fn culls_face_between_cells(
    current_face_occlusion_shape_indices: [u8; 6],
    current_skip: TerrainSkipRendering,
    current: TerrainMaterialClass,
    current_fluid: Option<TerrainFluid>,
    direction: TerrainFace,
    neighbor: &TerrainBlockState,
) -> bool {
    if matches!(current, TerrainMaterialClass::Fluid) {
        if let (Some(current), Some(neighbor)) = (current_fluid, neighbor.fluid) {
            return current.kind == neighbor.kind;
        }
    }
    let neighbor_skip = neighbor.skip_rendering;
    if current_skip.culls_with_leaves && neighbor_skip.culls_with_leaves {
        return true;
    }
    if current_skip.same_block_culls_all_faces && current_skip.is_same_block(neighbor_skip) {
        return true;
    }
    if current_skip.same_block_culls_vertical_faces
        && !direction.is_horizontal()
        && current_skip.is_same_block(neighbor_skip)
    {
        return true;
    }
    if current_skip.iron_bars_block {
        let same_block = current_skip.is_same_block(neighbor_skip);
        let bars_pair = current_skip.bars_tag
            && neighbor_skip.bars_tag
            && neighbor_skip.has_connection_property(direction.opposite());
        if same_block || bars_pair {
            if !direction.is_horizontal() {
                return true;
            }
            return current_skip.is_connected(direction)
                && neighbor_skip.is_connected(direction.opposite());
        }
    }
    face_occlusion_culls(
        current_face_occlusion_shape_indices[direction.index()],
        neighbor.face_occlusion_shape_indices[direction.opposite().index()],
    )
}

const CENTER_CHUNK_SLOT: usize = 4;
const CENTER_SECTION_SLOT: usize = 13;
const NEIGHBOR_EDGE: usize = 18;
const NEIGHBOR_LEN: usize = NEIGHBOR_EDGE * NEIGHBOR_EDGE * NEIGHBOR_EDGE;
const NEIGHBOR_PRESENT_BIT: u64 = 1u64 << 48;

thread_local! {
    static NEIGHBOR_CACHE: std::cell::Cell<Option<Box<[u64; NEIGHBOR_LEN]>>> =
        const { std::cell::Cell::new(None) };
}

fn take_neighbor_cache() -> Box<[u64; NEIGHBOR_LEN]> {
    NEIGHBOR_CACHE
        .with(std::cell::Cell::take)
        .unwrap_or_else(|| {
            vec![0u64; NEIGHBOR_LEN]
                .into_boxed_slice()
                .try_into()
                .expect("neighbor cache length")
        })
}

#[inline]
fn pack_cached_cell(block_state_id: u32, light: TerrainLight) -> u64 {
    NEIGHBOR_PRESENT_BIT
        | u64::from(block_state_id)
        | (u64::from(light.sky) << 32)
        | (u64::from(light.block) << 40)
}

#[inline]
fn axis_slice(delta_idx: usize) -> (usize, usize, usize) {
    match delta_idx {
        0 => (0, 15, 1),
        1 => (1, 0, 16),
        _ => (17, 0, 1),
    }
}

/// The target chunk and its loaded 3x3 neighborhood, as the mesher samples them.
pub(super) struct TerrainChunkLookup<'a> {
    table: &'a TerrainBlockRenderTable,
    origin_x: i32,
    origin_z: i32,
    chunks: [Option<&'a TerrainChunkSnapshot>; 9],
    window_section_y: Option<i32>,
    views: [Option<bbb_render_types::TerrainSectionView<'a>>; 27],
    center_unpacked: bool,
    center_blocks: [u32; 4096],
    neighbor_cache: Option<Box<[u64; NEIGHBOR_LEN]>>,
    cache_origin: Option<[i32; 3]>,
}

impl Drop for TerrainChunkLookup<'_> {
    fn drop(&mut self) {
        if let Some(cache) = self.neighbor_cache.take() {
            NEIGHBOR_CACHE.with(|slot| slot.set(Some(cache)));
        }
    }
}

impl<'a> TerrainChunkLookup<'a> {
    pub(super) fn new(
        target: &'a TerrainChunkSnapshot,
        neighborhood: impl IntoIterator<Item = &'a TerrainChunkSnapshot>,
        table: &'a TerrainBlockRenderTable,
    ) -> Self {
        let mut lookup = Self {
            table,
            origin_x: target.chunk_x,
            origin_z: target.chunk_z,
            chunks: [None; 9],
            window_section_y: None,
            views: [None; 27],
            center_unpacked: false,
            center_blocks: [0; 4096],
            neighbor_cache: None,
            cache_origin: None,
        };
        for snapshot in neighborhood {
            if let Some(slot) = lookup.slot(snapshot.chunk_x, snapshot.chunk_z) {
                lookup.chunks[slot] = Some(snapshot);
            }
        }
        let center = lookup
            .slot(target.chunk_x, target.chunk_z)
            .expect("target slot");
        lookup.chunks[center] = Some(target);
        lookup
    }

    pub(in crate::terrain) fn prepare_section(&mut self, section_y: i32) {
        self.window_section_y = Some(section_y);
        for dz in -1..=1_i32 {
            for dx in -1..=1_i32 {
                let chunk_slot = ((dz + 1) * 3 + (dx + 1)) as usize;
                let chunk = self.chunks[chunk_slot];
                for dy in -1..=1_i32 {
                    let view_slot = ((dz + 1) * 9 + (dy + 1) * 3 + (dx + 1)) as usize;
                    self.views[view_slot] = chunk.and_then(|snapshot| {
                        section_y
                            .checked_add(dy)
                            .and_then(|sy| snapshot.section_view(sy))
                    });
                }
            }
        }
        let skip_unpack = self.chunks[CENTER_CHUNK_SLOT]
            .and_then(|target| target.uniform_block_state_id(section_y))
            .is_some_and(|id| self.table.state(id).fixed_model_has_no_mesh());
        if let Some(center_view) = self.views[CENTER_SECTION_SLOT].filter(|_| !skip_unpack) {
            center_view
                .block_states
                .unpack_u32_into(&mut self.center_blocks);
            self.center_unpacked = true;
            let cache_origin = (|| {
                Some([
                    self.origin_x.checked_mul(16)?.checked_sub(1)?,
                    section_y.checked_mul(16)?.checked_sub(1)?,
                    self.origin_z.checked_mul(16)?.checked_sub(1)?,
                ])
            })();
            if let Some(origin) = cache_origin {
                let cache = self.neighbor_cache.get_or_insert_with(take_neighbor_cache);
                populate_neighbor_cache(
                    cache,
                    section_y,
                    &self.chunks,
                    &self.views,
                    &self.center_blocks,
                );
                self.cache_origin = Some(origin);
            } else {
                self.cache_origin = None;
            }
        } else {
            self.center_unpacked = false;
            self.cache_origin = None;
        }
    }

    #[inline]
    pub(in crate::terrain) fn center_block_state_id(&self, block_index: usize) -> Option<u32> {
        if self.center_unpacked {
            Some(self.center_blocks[block_index])
        } else {
            self.views[CENTER_SECTION_SLOT].map(|view| view.block_state_id_at(block_index))
        }
    }

    #[inline]
    fn center_section_view(
        &self,
        section_y: i32,
    ) -> Option<bbb_render_types::TerrainSectionView<'a>> {
        (self.window_section_y == Some(section_y))
            .then_some(self.views[CENTER_SECTION_SLOT])
            .flatten()
    }

    fn slot(&self, chunk_x: i32, chunk_z: i32) -> Option<usize> {
        let dx = i64::from(chunk_x) - i64::from(self.origin_x);
        let dz = i64::from(chunk_z) - i64::from(self.origin_z);
        ((-1..=1).contains(&dx) && (-1..=1).contains(&dz)).then(|| ((dz + 1) * 3 + dx + 1) as usize)
    }

    #[inline]
    fn section_view_at(
        &self,
        chunk_slot: usize,
        snapshot: &'a TerrainChunkSnapshot,
        section_y: i32,
    ) -> Option<bbb_render_types::TerrainSectionView<'a>> {
        if let Some(window_y) = self.window_section_y {
            let dy = i64::from(section_y) - i64::from(window_y);
            if (-1..=1).contains(&dy) {
                let dz = chunk_slot / 3;
                let dx = chunk_slot % 3;
                let view_slot = dz * 9 + (dy + 1) as usize * 3 + dx;
                return self.views[view_slot];
            }
        }
        snapshot.section_view(section_y)
    }

    #[inline]
    fn cell(&self, world_x: i32, world_y: i32, world_z: i32) -> Option<CellRef<'a>> {
        if let (Some(cache), Some([ox, oy, oz])) =
            (self.neighbor_cache.as_deref(), self.cache_origin)
        {
            let cx = (i64::from(world_x) - i64::from(ox)) as u64;
            let cy = (i64::from(world_y) - i64::from(oy)) as u64;
            let cz = (i64::from(world_z) - i64::from(oz)) as u64;
            if cx < NEIGHBOR_EDGE as u64 && cy < NEIGHBOR_EDGE as u64 && cz < NEIGHBOR_EDGE as u64 {
                let index =
                    (cy as usize * NEIGHBOR_EDGE + cz as usize) * NEIGHBOR_EDGE + cx as usize;
                let packed = cache[index];
                if packed == 0 {
                    return None;
                }
                return Some(self.table.cell_ref(
                    packed as u32,
                    TerrainLight {
                        sky: (packed >> 32) as u8,
                        block: (packed >> 40) as u8,
                    },
                ));
            }
        }
        self.cell_slow(world_x, world_y, world_z)
    }

    #[inline(never)]
    fn cell_slow(&self, world_x: i32, world_y: i32, world_z: i32) -> Option<CellRef<'a>> {
        let chunk_slot = self.slot(world_x.div_euclid(16), world_z.div_euclid(16))?;
        let snapshot = self.chunks[chunk_slot]?;
        if !snapshot.contains_world_y(world_y) {
            return None;
        }
        let section_y = world_y.div_euclid(16);
        let local_x = world_x.rem_euclid(16) as usize;
        let local_y = world_y.rem_euclid(16) as usize;
        let local_z = world_z.rem_euclid(16) as usize;
        let block_index = (local_y << 8) | (local_z << 4) | local_x;
        if chunk_slot == CENTER_CHUNK_SLOT
            && self.window_section_y == Some(section_y)
            && self.center_unpacked
        {
            let view = self.views[CENTER_SECTION_SLOT]?;
            return Some(
                self.table
                    .cell_ref(self.center_blocks[block_index], view.light_at(block_index)),
            );
        }
        let view = self.section_view_at(chunk_slot, snapshot, section_y)?;
        Some(self.table.cell_ref(
            view.block_state_id_at(block_index),
            view.light_at(block_index),
        ))
    }

    fn biome_id(&self, position: BlockRenderPosition) -> Option<i32> {
        let chunk_slot = self.slot(position.x.div_euclid(16), position.z.div_euclid(16))?;
        let snapshot = self.chunks[chunk_slot]?;
        if !snapshot.contains_world_y(position.y) {
            return None;
        }
        let section_y = position.y.div_euclid(16);
        let view = self.section_view_at(chunk_slot, snapshot, section_y)?;
        let local_x = position.x.rem_euclid(16) as usize;
        let local_y = position.y.rem_euclid(16) as usize;
        let local_z = position.z.rem_euclid(16) as usize;
        let quart_index = ((local_y >> 2) << 4) | ((local_z >> 2) << 2) | (local_x >> 2);
        view.biome_id_at(quart_index)
    }

    fn quart_y_uniform_flags(&self, section_min_y: i32) -> [bool; 4] {
        std::array::from_fn(|qy| {
            let y0 = section_min_y + (qy as i32) * 4;
            let y1 = y0 + 4;
            self.chunks.iter().flatten().all(|snapshot| {
                let top = snapshot.min_y + snapshot.height as i32;
                !(snapshot.min_y > y0 && snapshot.min_y < y1) && !(top > y0 && top < y1)
            })
        })
    }
}

fn populate_neighbor_cache(
    cache: &mut [u64; NEIGHBOR_LEN],
    section_y: i32,
    chunks: &[Option<&TerrainChunkSnapshot>; 9],
    views: &[Option<bbb_render_types::TerrainSectionView<'_>>; 27],
    center_blocks: &[u32; 4096],
) {
    for dy_idx in 0..3_usize {
        let (y_cache_start, y_local_start, y_len) = axis_slice(dy_idx);
        let sy_base = section_y
            .checked_add(dy_idx as i32 - 1)
            .and_then(|sy| sy.checked_mul(16));
        for dz_idx in 0..3_usize {
            let (z_cache_start, z_local_start, z_len) = axis_slice(dz_idx);
            for dx_idx in 0..3_usize {
                let (x_cache_start, x_local_start, x_len) = axis_slice(dx_idx);
                let chunk_slot = dz_idx * 3 + dx_idx;
                let view_slot = dz_idx * 9 + dy_idx * 3 + dx_idx;
                let Some(((sy_base, snapshot), view)) =
                    sy_base.zip(chunks[chunk_slot]).zip(views[view_slot])
                else {
                    for oy in 0..y_len {
                        let cy = y_cache_start + oy;
                        for oz in 0..z_len {
                            let row = (cy * NEIGHBOR_EDGE + (z_cache_start + oz)) * NEIGHBOR_EDGE
                                + x_cache_start;
                            cache[row..row + x_len].fill(0);
                        }
                    }
                    continue;
                };
                let is_center = chunk_slot == CENTER_CHUNK_SLOT && dy_idx == 1;
                let uniform_id = view.uniform_block_state_id();
                let uniform_packed = if let (
                    Some(id),
                    bbb_render_types::TerrainSectionLightRef::Uniform(sky),
                    bbb_render_types::TerrainSectionLightRef::Uniform(block),
                ) = (uniform_id, view.sky_light, view.block_light)
                {
                    Some(pack_cached_cell(id, TerrainLight { sky, block }))
                } else {
                    None
                };

                for oy in 0..y_len {
                    let local_y = y_local_start + oy;
                    let cy = y_cache_start + oy;
                    if !snapshot.contains_world_y(sy_base + local_y as i32) {
                        for oz in 0..z_len {
                            let row = (cy * NEIGHBOR_EDGE + (z_cache_start + oz)) * NEIGHBOR_EDGE
                                + x_cache_start;
                            cache[row..row + x_len].fill(0);
                        }
                        continue;
                    }
                    for oz in 0..z_len {
                        let local_z = z_local_start + oz;
                        let cache_row = (cy * NEIGHBOR_EDGE + (z_cache_start + oz)) * NEIGHBOR_EDGE
                            + x_cache_start;
                        if let Some(packed) = uniform_packed {
                            cache[cache_row..cache_row + x_len].fill(packed);
                            continue;
                        }
                        let block_row = (local_y << 8) | (local_z << 4) | x_local_start;
                        match (view.sky_light, view.block_light) {
                            (
                                bbb_render_types::TerrainSectionLightRef::Uniform(sky),
                                bbb_render_types::TerrainSectionLightRef::Uniform(block),
                            ) => {
                                let light_bits = NEIGHBOR_PRESENT_BIT
                                    | (u64::from(sky) << 32)
                                    | (u64::from(block) << 40);
                                for ox in 0..x_len {
                                    let bi = block_row + ox;
                                    let id = if is_center {
                                        center_blocks[bi]
                                    } else {
                                        uniform_id.unwrap_or_else(|| view.block_state_id_at(bi))
                                    };
                                    cache[cache_row + ox] = light_bits | u64::from(id);
                                }
                            }
                            (
                                bbb_render_types::TerrainSectionLightRef::Packed(sky_bytes),
                                bbb_render_types::TerrainSectionLightRef::Uniform(block),
                            ) => {
                                let block_bits = NEIGHBOR_PRESENT_BIT | (u64::from(block) << 40);
                                for ox in 0..x_len {
                                    let bi = block_row + ox;
                                    let id = if is_center {
                                        center_blocks[bi]
                                    } else {
                                        uniform_id.unwrap_or_else(|| view.block_state_id_at(bi))
                                    };
                                    let sky = (sky_bytes[bi >> 1] >> ((bi & 1) << 2)) & 0x0f;
                                    cache[cache_row + ox] =
                                        block_bits | (u64::from(sky) << 32) | u64::from(id);
                                }
                            }
                            _ => {
                                for ox in 0..x_len {
                                    let bi = block_row + ox;
                                    let id = if is_center {
                                        center_blocks[bi]
                                    } else {
                                        uniform_id.unwrap_or_else(|| view.block_state_id_at(bi))
                                    };
                                    cache[cache_row + ox] = pack_cached_cell(id, view.light_at(bi));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub(super) fn cell_index(x: usize, y: usize, z: usize, height: usize) -> usize {
    debug_assert!(x < 16);
    debug_assert!(y < height);
    debug_assert!(z < 16);
    ((y * 16) + z) * 16 + x
}

#[cfg(test)]
mod tests;
