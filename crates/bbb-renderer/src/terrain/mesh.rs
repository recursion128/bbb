mod emitter;
mod geometry;

use std::collections::HashMap;

use self::{
    emitter::{emit_box, emit_cross, emit_face},
    geometry::FACES,
};
use super::{
    TerrainCell, TerrainChunkSnapshot, TerrainMaterialClass, TerrainMesh, TerrainRenderShape,
    TerrainTextureAtlas,
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
                if !mode.is_meshed(cell.material) {
                    continue;
                }

                let world_x = snapshot.chunk_x * 16 + x;
                let world_y = snapshot.min_y + y;
                let world_z = snapshot.chunk_z * 16 + z;
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
                            *shade,
                            *light_emission,
                            atlas,
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
                                model_cross.shade,
                                model_cross.light_emission,
                                atlas,
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
                    } => {
                        emit_box(
                            &mut mesh,
                            world_x,
                            world_y,
                            world_z,
                            cell.block_state_id,
                            cell.material,
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
                                lookup,
                                mode,
                            );
                        }
                        continue;
                    }
                    TerrainRenderShape::Cube => {}
                }

                for face in FACES {
                    let neighbor =
                        lookup.cell(world_x + face.dx, world_y + face.dy, world_z + face.dz);
                    if neighbor
                        .map(|neighbor| mode.culls_face_between(cell.material, neighbor.material))
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
                        cell.material,
                        cell.light,
                        cell.tint[face.face.index()],
                        atlas.rect(cell.texture_indices[face.face.index()]),
                        face,
                    );
                }
            }
        }
    }
    mesh
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
