mod emitter;
mod fluid;
mod geometry;

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
    TerrainMesh, TerrainRenderShape, TerrainTextureAtlas,
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

    fn culls_face_between(
        self,
        current: TerrainMaterialClass,
        neighbor: TerrainMaterialClass,
    ) -> bool {
        self.is_occluded_by(neighbor)
            || (matches!(current, TerrainMaterialClass::Fluid)
                && matches!(neighbor, TerrainMaterialClass::Fluid))
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
                    emit_fluid_overlay(&mut mesh, world_x, world_y, world_z, cell, lookup, atlas);
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
                            cell.fluid,
                            cell.light,
                            cell.tint,
                            cell.texture_indices,
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
                                cell.fluid,
                                cell.light,
                                model_box.tint,
                                model_box.texture_indices,
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
                            cell.light,
                            model_quads,
                            atlas,
                            cell.ambient_occlusion,
                            lookup,
                            mode,
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
                            culls_face_between_cells(mode, face_material, cell.fluid, neighbor)
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
                    );
                }
            }
        }
    }
    mesh
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
) {
    let face_cull = TerrainFace::ALL.map(Some);
    emit_box(
        mesh,
        world_x,
        world_y,
        world_z,
        cell.block_state_id,
        TerrainMaterialClass::Fluid,
        cell.fluid,
        cell.light,
        cell.fluid_tint,
        cell.fluid_texture_indices,
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
    );
}

pub(super) fn culls_face_between_cells(
    mode: TerrainMeshMode,
    current: TerrainMaterialClass,
    current_fluid: Option<TerrainFluid>,
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
    mode.culls_face_between(current, neighbor.material)
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
