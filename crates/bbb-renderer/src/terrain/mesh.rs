mod emitter;
mod fluid;
pub(crate) mod geometry;
mod item_bake;

pub(in crate::terrain) use item_bake::bake_block_item_quads;

use std::collections::HashMap;

use self::{
    emitter::{
        effective_face_material, emit_box, emit_cross, emit_face, emit_quads,
        face_ambient_occlusion, face_vertex_lights,
    },
    geometry::FACES,
};
use super::{
    TerrainCell, TerrainChunkSnapshot, TerrainFace, TerrainFluid, TerrainMaterialClass,
    TerrainMesh, TerrainRenderShape, TerrainSkipRendering, TerrainTextureAtlas,
};

#[derive(Debug, Clone, Copy)]
pub(super) enum TerrainMeshMode {
    OpaqueOnly,
    CutoutOnly,
    TranslucentOnly,
    OpaqueCutout,
}

impl TerrainMeshMode {
    fn is_meshed(self, material: TerrainMaterialClass) -> bool {
        match self {
            Self::OpaqueOnly => material.is_meshed_opaque(),
            Self::CutoutOnly => material.is_meshed_cutout(),
            Self::TranslucentOnly => material.is_meshed_translucent(),
            Self::OpaqueCutout => material.is_meshed_terrain(),
        }
    }

    fn is_occluded_by(self, material: TerrainMaterialClass) -> bool {
        match self {
            Self::OpaqueOnly => material.occludes_opaque(),
            Self::CutoutOnly => material.occludes_terrain(),
            Self::TranslucentOnly => material.occludes_terrain(),
            Self::OpaqueCutout => material.occludes_terrain(),
        }
    }

    fn is_occluded_by_cell(self, cell: &TerrainCell) -> bool {
        render_shape_has_geometry(&cell.render_shape) && self.is_occluded_by(cell.material)
    }
}

pub(super) fn build_chunk_mesh_with_lookup(
    snapshot: &TerrainChunkSnapshot,
    lookup: &TerrainChunkLookup<'_>,
    atlas: &TerrainTextureAtlas,
    mode: TerrainMeshMode,
) -> TerrainMesh {
    let mut mesh = TerrainMesh {
        source_sections: snapshot.height.div_ceil(16),
        ..TerrainMesh::default()
    };
    let cardinal = snapshot.cardinal_lighting;
    for y in 0..snapshot.height as i32 {
        for z in 0..16 {
            for x in 0..16 {
                let cell = snapshot
                    .cell(x, y, z)
                    .expect("in-bounds terrain cell exists");
                let has_fluid_overlay = has_fluid_overlay(cell);
                if !mode.is_meshed(cell.material)
                    && !has_texture_layer_overrides(cell)
                    && !(matches!(mode, TerrainMeshMode::TranslucentOnly) && has_fluid_overlay)
                {
                    continue;
                }

                let world_x = snapshot.chunk_x * 16 + x;
                let world_y = snapshot.min_y + y;
                let world_z = snapshot.chunk_z * 16 + z;
                if matches!(mode, TerrainMeshMode::TranslucentOnly) && has_fluid_overlay {
                    emit_fluid_overlay(
                        &mut mesh, world_x, world_y, world_z, cell, lookup, atlas, cardinal,
                    );
                    if !mode.is_meshed(cell.material) && !has_texture_layer_overrides(cell) {
                        continue;
                    }
                }
                match &cell.render_shape {
                    TerrainRenderShape::Cross {
                        shade,
                        light_emission,
                    } => {
                        emit_cross(
                            &mut mesh,
                            world_x,
                            world_y,
                            world_z,
                            cell.block_state_id,
                            cell.material,
                            cell.light,
                            cell.tint,
                            cell.texture_indices,
                            cell.face_transparency,
                            *shade,
                            *light_emission,
                            atlas,
                            mode,
                            cardinal,
                        );
                        continue;
                    }
                    TerrainRenderShape::Crosses(model_crosses) => {
                        for model_cross in model_crosses {
                            emit_cross(
                                &mut mesh,
                                world_x,
                                world_y,
                                world_z,
                                cell.block_state_id,
                                cell.material,
                                cell.light,
                                model_cross.tint,
                                model_cross.texture_indices,
                                model_cross.face_transparency,
                                model_cross.shade,
                                model_cross.light_emission,
                                atlas,
                                mode,
                                cardinal,
                            );
                        }
                        continue;
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
                            &mut mesh,
                            world_x,
                            world_y,
                            world_z,
                            cell.block_state_id,
                            cell.material,
                            cell.skip_rendering,
                            cell.fluid,
                            cell.light,
                            cell.tint,
                            cell.texture_indices,
                            cell.fluid_overlay_texture_index,
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
                            cell.ambient_occlusion,
                            lookup,
                            mode,
                            cardinal,
                        );
                        continue;
                    }
                    TerrainRenderShape::Boxes(model_boxes) => {
                        for model_box in model_boxes {
                            emit_box(
                                &mut mesh,
                                world_x,
                                world_y,
                                world_z,
                                cell.block_state_id,
                                cell.material,
                                cell.skip_rendering,
                                cell.fluid,
                                cell.light,
                                model_box.tint,
                                model_box.texture_indices,
                                cell.fluid_overlay_texture_index,
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
                                cell.ambient_occlusion,
                                lookup,
                                mode,
                                cardinal,
                            );
                        }
                        continue;
                    }
                    TerrainRenderShape::Quads(model_quads) => {
                        emit_quads(
                            &mut mesh,
                            world_x,
                            world_y,
                            world_z,
                            cell.block_state_id,
                            cell.material,
                            cell.skip_rendering,
                            cell.light,
                            model_quads,
                            atlas,
                            cell.ambient_occlusion,
                            lookup,
                            mode,
                            cardinal,
                        );
                        continue;
                    }
                    TerrainRenderShape::Cube => {}
                }

                for face in FACES {
                    let face_index = face.face.index();
                    let face_material =
                        effective_face_material(cell.material, cell.face_transparency[face_index]);
                    if !mode.is_meshed(face_material) {
                        continue;
                    }
                    let neighbor =
                        lookup.cell(world_x + face.dx, world_y + face.dy, world_z + face.dz);
                    if neighbor
                        .map(|neighbor| {
                            culls_face_between_cells(
                                mode,
                                cell.skip_rendering,
                                face_material,
                                cell.fluid,
                                face.face,
                                neighbor,
                            )
                        })
                        .unwrap_or(false)
                    {
                        mesh.culled_faces += 1;
                        continue;
                    }
                    emit_face(
                        &mut mesh,
                        world_x,
                        world_y,
                        world_z,
                        cell.block_state_id,
                        face_material,
                        cell.tint[face_index],
                        atlas.rect(cell.texture_indices[face_index]),
                        face,
                        face_vertex_lights(
                            cell.ambient_occlusion,
                            face.face,
                            world_x,
                            world_y,
                            world_z,
                            true,
                            cell.light,
                            0,
                            lookup,
                            mode,
                        ),
                        face_ambient_occlusion(
                            cell.ambient_occlusion,
                            face.face,
                            world_x,
                            world_y,
                            world_z,
                            true,
                            lookup,
                            mode,
                        ),
                        cardinal,
                    );
                }
            }
        }
    }
    mesh
}

pub(super) fn sort_translucent_quads_by_distance(
    mesh: &mut TerrainMesh,
    camera_position: [f32; 3],
) {
    debug_assert_eq!(mesh.vertices.len() % 4, 0);
    if mesh.vertices.len() % 4 != 0 {
        return;
    }

    let mut quad_distances: Vec<_> = mesh
        .vertices
        .chunks_exact(4)
        .enumerate()
        .map(|(quad_index, quad)| {
            let centroid = [
                (quad[0].position[0] + quad[2].position[0]) * 0.5,
                (quad[0].position[1] + quad[2].position[1]) * 0.5,
                (quad[0].position[2] + quad[2].position[2]) * 0.5,
            ];
            let dx = centroid[0] - camera_position[0];
            let dy = centroid[1] - camera_position[1];
            let dz = centroid[2] - camera_position[2];
            (quad_index, dx * dx + dy * dy + dz * dz)
        })
        .collect();
    quad_distances.sort_by(
        |(left_index, left_distance), (right_index, right_distance)| {
            right_distance
                .total_cmp(left_distance)
                .then_with(|| left_index.cmp(right_index))
        },
    );

    mesh.indices.clear();
    mesh.indices.reserve(quad_distances.len() * 6);
    for (quad_index, _) in quad_distances {
        let base = (quad_index * 4) as u32;
        mesh.indices
            .extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
    }
}

fn has_texture_layer_overrides(cell: &TerrainCell) -> bool {
    cell.face_transparency
        .iter()
        .any(|transparency| !is_opaque_transparency(*transparency))
        || match &cell.render_shape {
            TerrainRenderShape::Box {
                face_transparency, ..
            } => face_transparency
                .iter()
                .any(|transparency| !is_opaque_transparency(*transparency)),
            TerrainRenderShape::Boxes(model_boxes) => model_boxes.iter().any(|model_box| {
                model_box
                    .face_transparency
                    .iter()
                    .any(|transparency| !is_opaque_transparency(*transparency))
            }),
            TerrainRenderShape::Crosses(model_crosses) => model_crosses.iter().any(|model_cross| {
                model_cross
                    .face_transparency
                    .iter()
                    .any(|transparency| !is_opaque_transparency(*transparency))
            }),
            TerrainRenderShape::Quads(model_quads) => model_quads
                .iter()
                .any(|model_quad| !is_opaque_transparency(model_quad.transparency)),
            TerrainRenderShape::Cube | TerrainRenderShape::Cross { .. } => false,
        }
}

fn has_fluid_overlay(cell: &TerrainCell) -> bool {
    cell.fluid.is_some() && !matches!(cell.material, TerrainMaterialClass::Fluid)
}

fn emit_fluid_overlay(
    mesh: &mut TerrainMesh,
    world_x: i32,
    world_y: i32,
    world_z: i32,
    cell: &TerrainCell,
    lookup: &TerrainChunkLookup<'_>,
    atlas: &TerrainTextureAtlas,
    cardinal: super::TerrainCardinalLighting,
) {
    let face_cull = TerrainFace::ALL.map(Some);
    emit_box(
        mesh,
        world_x,
        world_y,
        world_z,
        cell.block_state_id,
        TerrainMaterialClass::Fluid,
        TerrainSkipRendering::NONE,
        cell.fluid,
        cell.light,
        cell.fluid_tint,
        cell.fluid_texture_indices,
        cell.fluid_overlay_texture_index,
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
        TerrainMeshMode::TranslucentOnly,
        cardinal,
    );
}

pub(super) fn culls_face_between_cells(
    mode: TerrainMeshMode,
    current_skip: TerrainSkipRendering,
    current: TerrainMaterialClass,
    current_fluid: Option<TerrainFluid>,
    direction: TerrainFace,
    neighbor: &TerrainCell,
) -> bool {
    if matches!(current, TerrainMaterialClass::Fluid) {
        if let (Some(current), Some(neighbor)) = (current_fluid, neighbor.fluid) {
            return current.kind == neighbor.kind;
        }
        if matches!(neighbor.material, TerrainMaterialClass::Fluid) {
            return true;
        }
    }
    let neighbor_skip = neighbor.skip_rendering;
    if current_skip.same_block_culls_all_faces && current_skip.is_same_block(neighbor_skip) {
        return true;
    }
    if current_skip.iron_bars_block {
        let same_block = current_skip.is_same_block(neighbor_skip);
        let bars_pair =
            current_skip.bars_tag && neighbor_skip.bars_tag && neighbor_skip.iron_bars_block;
        if same_block || bars_pair {
            if !direction.is_horizontal() {
                return true;
            }
            return current_skip.is_connected(direction)
                && neighbor_skip.is_connected(direction.opposite());
        }
    }
    // Vanilla `Block.shouldRenderFace` (Block.java:304): the current face at the
    // shared boundary is culled when the neighbour's occlusion shape on the
    // opposite side fully covers it. bbb models occlusion conservatively as a
    // per-direction "is this face a full 1×1" test derived from the render
    // cuboids: when the neighbour presents a full opaque occlusion face,
    // `Shapes.joinIsNotEmpty(own, occluder, ONLY_FIRST)` is empty for any own
    // face, so the cull decision collapses to a one-way neighbour-full check.
    // Partial neighbour faces are never treated as occluders (safe over-render,
    // matching vanilla's own-face-empty / partial-join render paths).
    mode.is_occluded_by(neighbor.material)
        && face_occludes(&neighbor.render_shape, direction.opposite())
}

/// Analogue of vanilla `Block.isFaceFull(state.getOcclusionShape(), direction)`
/// (`state.getFaceOcclusionShape(direction)` being a full block face — see
/// `BlockBehaviour` occlusion-shape-per-face cache and `Shapes.blockOccludes`):
/// reports whether `shape` presents a face that covers the entire 1×1 cell
/// boundary in `direction`, derived purely from the render cuboids. This is the
/// exact per-face union test for box-based shapes (a stair's full back face is
/// full only via the union of its two boxes, not any single box), so it is a
/// strict subset of vanilla occlusion and never culls a face vanilla keeps.
/// `Cross`/`Crosses`/`Quads` contribute no occlusion (vanilla foliage / custom
/// models likewise have an empty occlusion shape).
fn face_occludes(shape: &TerrainRenderShape, direction: TerrainFace) -> bool {
    match shape {
        TerrainRenderShape::Cube => true,
        TerrainRenderShape::Cross { .. }
        | TerrainRenderShape::Crosses(_)
        | TerrainRenderShape::Quads(_) => false,
        TerrainRenderShape::Box { from, to, .. } => cuboid_face_full(*from, *to, direction),
        TerrainRenderShape::Boxes(boxes) => {
            // Fast path: any single cuboid already spans the full face.
            if boxes
                .iter()
                .any(|model_box| cuboid_face_full(model_box.from, model_box.to, direction))
            {
                return true;
            }
            // Otherwise rasterise the union of every boundary-touching cuboid's
            // cross-section onto the 16×16 boundary grid and require full cover
            // (stacked stair boxes, split walls, …).
            let mut covered = [[false; 16]; 16];
            for model_box in boxes {
                mark_boundary_cross_section(model_box.from, model_box.to, direction, &mut covered);
            }
            covered.iter().flatten().all(|cell| *cell)
        }
    }
}

/// The two axes perpendicular to `direction` (the face plane) and whether a
/// boundary-touching cuboid must reach `0` (`false`) or `16` (`true`) on the
/// face axis.
fn face_axes(direction: TerrainFace) -> (usize, usize, usize, bool) {
    match direction {
        TerrainFace::Down => (1, 0, 2, false),
        TerrainFace::Up => (1, 0, 2, true),
        TerrainFace::North => (2, 0, 1, false),
        TerrainFace::South => (2, 0, 1, true),
        TerrainFace::West => (0, 1, 2, false),
        TerrainFace::East => (0, 1, 2, true),
    }
}

/// Whether the `[from, to]` cuboid (in 0..=16 block-sixteenths) touches the cell
/// boundary in `direction` and spans the full `16×16` perpendicular
/// cross-section (a single-cuboid full face: slabs, box bottoms/backs).
fn cuboid_face_full(from: [u8; 3], to: [u8; 3], direction: TerrainFace) -> bool {
    let (face_axis, axis_a, axis_b, positive) = face_axes(direction);
    let touches = if positive {
        to[face_axis] == 16
    } else {
        from[face_axis] == 0
    };
    touches && from[axis_a] == 0 && to[axis_a] == 16 && from[axis_b] == 0 && to[axis_b] == 16
}

/// Marks the `16×16` cells covered by the cuboid's cross-section when it touches
/// the cell boundary in `direction`; a no-op when the cuboid does not reach the
/// boundary (mirroring vanilla `getFaceShape` slicing at the boundary plane).
fn mark_boundary_cross_section(
    from: [u8; 3],
    to: [u8; 3],
    direction: TerrainFace,
    covered: &mut [[bool; 16]; 16],
) {
    let (face_axis, axis_a, axis_b, positive) = face_axes(direction);
    let touches = if positive {
        to[face_axis] == 16
    } else {
        from[face_axis] == 0
    };
    if !touches {
        return;
    }
    for a in from[axis_a]..to[axis_a] {
        for b in from[axis_b]..to[axis_b] {
            covered[a as usize][b as usize] = true;
        }
    }
}

fn render_shape_has_geometry(shape: &TerrainRenderShape) -> bool {
    match shape {
        TerrainRenderShape::Boxes(boxes) => !boxes.is_empty(),
        TerrainRenderShape::Crosses(crosses) => !crosses.is_empty(),
        TerrainRenderShape::Quads(quads) => !quads.is_empty(),
        TerrainRenderShape::Cube
        | TerrainRenderShape::Cross { .. }
        | TerrainRenderShape::Box { .. } => true,
    }
}

fn is_opaque_transparency(transparency: super::TerrainTransparency) -> bool {
    !transparency.has_transparent && !transparency.has_translucent
}

pub(super) struct TerrainChunkLookup<'a> {
    chunks: HashMap<(i32, i32), &'a TerrainChunkSnapshot>,
}

impl<'a> TerrainChunkLookup<'a> {
    pub(super) fn new(snapshots: &'a [TerrainChunkSnapshot]) -> Self {
        Self {
            chunks: snapshots
                .iter()
                .map(|snapshot| ((snapshot.chunk_x, snapshot.chunk_z), snapshot))
                .collect(),
        }
    }

    fn cell(&self, world_x: i32, world_y: i32, world_z: i32) -> Option<&TerrainCell> {
        let chunk_x = world_x.div_euclid(16);
        let chunk_z = world_z.div_euclid(16);
        let snapshot = self.chunks.get(&(chunk_x, chunk_z))?;
        snapshot.cell(
            world_x.rem_euclid(16),
            world_y - snapshot.min_y,
            world_z.rem_euclid(16),
        )
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
