use super::super::{
    block_table::{CellRef, TerrainBlockState},
    fluid_face_occlusion_culls, TerrainCardinalLighting, TerrainFace, TerrainFluid,
    TerrainFluidKind, TerrainLight, TerrainMaterialClass, TerrainQuad, TerrainSkipRendering,
    TerrainTextureAtlas, TerrainTint, TerrainTransparency, TerrainUvRect,
};
use super::lighting::{
    cull_offset, flat_quad_light, pack, render_light_for_cell, smooth_quad_lighting,
    state_render_light, with_emission, LightCoords,
};
use super::{
    culls_face_between_cells,
    fluid::{
        fluid_corner_heights, fluid_face_corners, fluid_face_texture_index, fluid_face_uvs,
        fluid_self_height, fluid_should_render_backward_up_face, fluid_top_flow, FLUID_FACE_OFFSET,
    },
    geometry::{box_face_corners, face_uvs_from_crop, FaceDef, CROSS_FACES, FACES},
    TerrainChunkLookup, TerrainLayerMeshes,
};
use crate::render_core::gpu::TerrainGpuVertex;

#[allow(clippy::too_many_arguments)]
pub(super) fn emit_face(
    out: &mut TerrainLayerMeshes,
    x: i32,
    y: i32,
    z: i32,
    material: TerrainMaterialClass,
    tint: TerrainTint,
    uv_rect: TerrainUvRect,
    face: FaceDef,
    vertex_lights: [LightCoords; 4],
    ambient_occlusion: [f32; 4],
    cardinal: TerrainCardinalLighting,
) {
    // `BlockModelShape::Cube` intentionally omits redundant per-face UV data,
    // but it still follows FaceBakery's FaceInfo vertex order. Keep that
    // ordering here instead of treating the renderer's winding order as UV
    // order; otherwise every full-cube vertical texture is rotated 90 degrees.
    let uvs = face_uvs_from_crop(face.face, [0, 0, 16, 16], 0);
    emit_custom_quad_with_uvs(
        out,
        x,
        y,
        z,
        material,
        tint,
        uv_rect,
        face.normal,
        face.corners,
        uvs,
        cardinal.shade(true, face.face),
        vertex_lights,
        ambient_occlusion,
        false,
        face_info_first(face.face),
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn emit_cross(
    out: &mut TerrainLayerMeshes,
    x: i32,
    y: i32,
    z: i32,
    material: TerrainMaterialClass,
    light: TerrainLight,
    state_light_emission: u8,
    emissive_rendering: bool,
    tint: [TerrainTint; 6],
    texture_indices: [u32; 6],
    face_transparency: [TerrainTransparency; 6],
    shade: bool,
    light_emission: u8,
    atlas: &TerrainTextureAtlas,
    cardinal: TerrainCardinalLighting,
) {
    let light = state_render_light(light, state_light_emission, emissive_rendering);
    for (face, normal, corners) in CROSS_FACES {
        let face_material = effective_face_material(material, face_transparency[face.index()]);
        if matches!(face_material, TerrainMaterialClass::Empty) {
            continue;
        }
        emit_custom_quad(
            out,
            x,
            y,
            z,
            face_material,
            tint[face.index()],
            atlas.rect(texture_indices[face.index()]),
            normal,
            corners,
            cardinal.shade(shade, face),
            [with_emission(pack(light), light_emission); 4],
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn emit_box(
    out: &mut TerrainLayerMeshes,
    x: i32,
    y: i32,
    z: i32,
    material: TerrainMaterialClass,
    skip_rendering: TerrainSkipRendering,
    face_occlusion_shape_indices: [u8; 6],
    collision_full_block: bool,
    fluid: Option<TerrainFluid>,
    light: TerrainLight,
    state_light_emission: u8,
    emissive_rendering: bool,
    tint: [TerrainTint; 6],
    texture_indices: [u32; 6],
    fluid_overlay_texture_index: Option<u32>,
    atlas: &TerrainTextureAtlas,
    from: [u8; 3],
    to: [u8; 3],
    face_present: [bool; 6],
    face_uvs: [[u8; 4]; 6],
    face_uv_rotations: [u8; 6],
    face_shade: [bool; 6],
    face_light_emission: [u8; 6],
    face_cull: [Option<TerrainFace>; 6],
    face_transparency: [TerrainTransparency; 6],
    ambient_occlusion: bool,
    lookup: &TerrainChunkLookup<'_>,
    cardinal: TerrainCardinalLighting,
) {
    let min = [
        from[0] as f32 / 16.0,
        from[1] as f32 / 16.0,
        from[2] as f32 / 16.0,
    ];
    let mut max = [
        to[0] as f32 / 16.0,
        to[1] as f32 / 16.0,
        to[2] as f32 / 16.0,
    ];
    let is_fluid = matches!(material, TerrainMaterialClass::Fluid);
    if is_fluid {
        max[1] = fluid_self_height(fluid, max[1], x, y, z, lookup);
    }
    let fluid_flow = if is_fluid {
        fluid.and_then(|fluid| fluid_top_flow(x, y, z, fluid, lookup))
    } else {
        None
    };
    let base_fluid_heights = is_fluid
        .then(|| fluid_corner_heights(x, y, z, max[1], fluid.map(|fluid| fluid.kind), lookup));
    let fluid_top_offset = if is_fluid
        && box_face_will_render(
            TerrainFace::Up,
            x,
            y,
            z,
            face_present,
            face_transparency,
            face_cull,
            skip_rendering,
            face_occlusion_shape_indices,
            material,
            fluid,
            base_fluid_heights,
            lookup,
        ) {
        FLUID_FACE_OFFSET
    } else {
        0.0
    };
    let fluid_bottom_offset = if is_fluid
        && box_face_will_render(
            TerrainFace::Down,
            x,
            y,
            z,
            face_present,
            face_transparency,
            face_cull,
            skip_rendering,
            face_occlusion_shape_indices,
            material,
            fluid,
            base_fluid_heights,
            lookup,
        ) {
        FLUID_FACE_OFFSET
    } else {
        0.0
    };

    for face in FACES {
        let face_index = face.face.index();
        if !face_present[face_index] {
            continue;
        }
        let face_material = effective_face_material(material, face_transparency[face_index]);
        if matches!(face_material, TerrainMaterialClass::Empty) {
            continue;
        }
        if let Some(cull_face) = face_cull[face_index] {
            let (dx, dy, dz) = cull_offset(cull_face);
            let neighbor = lookup.cell(x + dx, y + dy, z + dz);
            let culled = if is_fluid {
                fluid_face_culled(
                    cull_face,
                    face_occlusion_shape_indices,
                    fluid,
                    base_fluid_heights,
                    &neighbor.unwrap_or_else(CellRef::empty),
                )
            } else {
                neighbor
                    .map(|neighbor| {
                        culls_face_between_cells(
                            face_occlusion_shape_indices,
                            skip_rendering,
                            face_material,
                            fluid,
                            cull_face,
                            &neighbor,
                        )
                    })
                    .unwrap_or(false)
            };
            if culled {
                if let Some(mesh) = out.layer(face_material) {
                    mesh.culled_faces += 1;
                }
                continue;
            }
        }

        let fluid_heights =
            base_fluid_heights.map(|heights| heights.with_top_offset(fluid_top_offset));
        let fluid_side_overlay_texture_index = is_fluid
            .then(|| {
                fluid_side_overlay_texture_index(
                    face.face,
                    fluid,
                    fluid_overlay_texture_index,
                    x,
                    y,
                    z,
                    lookup,
                )
            })
            .flatten();
        let texture_index = fluid_side_overlay_texture_index.unwrap_or_else(|| {
            fluid_face_texture_index(face.face, texture_indices, fluid_flow.is_some())
        });
        let corners = fluid_heights
            .map(|heights| fluid_face_corners(face.face, min, max, heights, fluid_bottom_offset))
            .unwrap_or_else(|| box_face_corners(face.face, min, max));
        let (vertex_lights, ambient_occlusion) = if is_fluid {
            (
                fluid_face_vertex_lights(
                    face.face,
                    x,
                    y,
                    z,
                    light,
                    state_light_emission,
                    emissive_rendering,
                    face_light_emission[face_index],
                    lookup,
                ),
                [1.0; 4],
            )
        } else {
            quad_lighting(
                face.face,
                &corners,
                false,
                face_cull[face_index],
                face_light_emission[face_index],
                ambient_occlusion,
                collision_full_block,
                (x, y, z),
                light,
                state_light_emission,
                emissive_rendering,
                lookup,
            )
        };
        let uvs = fluid_heights
            .map(|heights| fluid_face_uvs(face.face, heights, fluid_flow))
            .unwrap_or_else(|| {
                face_uvs_from_crop(
                    face.face,
                    face_uvs[face_index],
                    face_uv_rotations[face_index],
                )
            });
        let uv_rect = atlas.rect(texture_index);
        let shade = cardinal.shade(face_shade[face_index], face.face);
        emit_custom_quad_with_uvs(
            out,
            x,
            y,
            z,
            face_material,
            tint[face_index],
            uv_rect,
            face.normal,
            corners,
            uvs,
            shade,
            vertex_lights,
            ambient_occlusion,
            false,
            if is_fluid {
                0
            } else {
                face_info_first(face.face)
            },
        );
        // Vanilla `FluidRenderer.tesselate` renders fluid faces double-sided via
        // `addFace(..., addBackFace)`, which re-emits the four vertices in reverse
        // (`v3, v2, v1, v0`) to form a back face for the inside-fluid view:
        //   - top:    `addBackFace = shouldRenderBackwardUpFace(level, pos.above())`
        //   - sides:  `addBackFace = !isOverlay`, where water uses `water_overlay`
        //             against HalfTransparentBlock/LeavesBlock neighbors
        //   - bottom: `addBackFace = false`
        // The terrain pipeline uses back-face culling, so the reversed quad is what
        // keeps the fluid surface visible when the camera is submerged.
        let emit_backward_face = is_fluid
            && match face.face {
                TerrainFace::Up => fluid.is_some_and(|fluid| {
                    fluid_should_render_backward_up_face(x, y, z, fluid, lookup)
                }),
                TerrainFace::North | TerrainFace::South | TerrainFace::West | TerrainFace::East => {
                    fluid.is_some() && fluid_side_overlay_texture_index.is_none()
                }
                TerrainFace::Down => false,
            };
        if emit_backward_face {
            emit_custom_quad_with_uvs(
                out,
                x,
                y,
                z,
                face_material,
                tint[face_index],
                uv_rect,
                face.normal,
                [corners[3], corners[2], corners[1], corners[0]],
                [uvs[3], uvs[2], uvs[1], uvs[0]],
                shade,
                [
                    vertex_lights[3],
                    vertex_lights[2],
                    vertex_lights[1],
                    vertex_lights[0],
                ],
                [
                    ambient_occlusion[3],
                    ambient_occlusion[2],
                    ambient_occlusion[1],
                    ambient_occlusion[0],
                ],
                true,
                0,
            );
        }
    }
}

fn fluid_side_overlay_texture_index(
    face: TerrainFace,
    fluid: Option<TerrainFluid>,
    overlay_texture_index: Option<u32>,
    x: i32,
    y: i32,
    z: i32,
    lookup: &TerrainChunkLookup<'_>,
) -> Option<u32> {
    if !matches!(
        face,
        TerrainFace::North | TerrainFace::South | TerrainFace::West | TerrainFace::East
    ) || !matches!(fluid.map(|fluid| fluid.kind), Some(TerrainFluidKind::Water))
    {
        return None;
    }

    let overlay_texture_index = overlay_texture_index?;
    let (dx, dy, dz) = cull_offset(face);
    lookup
        .cell(x + dx, y + dy, z + dz)
        .is_some_and(|neighbor| neighbor.fluid_overlay_neighbor)
        .then_some(overlay_texture_index)
}

#[allow(clippy::too_many_arguments)]
fn box_face_will_render(
    face: TerrainFace,
    x: i32,
    y: i32,
    z: i32,
    face_present: [bool; 6],
    face_transparency: [TerrainTransparency; 6],
    face_cull: [Option<TerrainFace>; 6],
    skip_rendering: TerrainSkipRendering,
    face_occlusion_shape_indices: [u8; 6],
    material: TerrainMaterialClass,
    fluid: Option<TerrainFluid>,
    fluid_heights: Option<super::fluid::FluidCornerHeights>,
    lookup: &TerrainChunkLookup<'_>,
) -> bool {
    let face_index = face.index();
    if !face_present[face_index] {
        return false;
    }
    let face_material = effective_face_material(material, face_transparency[face_index]);
    if matches!(face_material, TerrainMaterialClass::Empty) {
        return false;
    }
    if let Some(cull_face) = face_cull[face_index] {
        let (dx, dy, dz) = cull_offset(cull_face);
        let neighbor = lookup.cell(x + dx, y + dy, z + dz);
        let culled = if matches!(material, TerrainMaterialClass::Fluid) {
            fluid_face_culled(
                cull_face,
                face_occlusion_shape_indices,
                fluid,
                fluid_heights,
                &neighbor.unwrap_or_else(CellRef::empty),
            )
        } else {
            neighbor.is_some_and(|neighbor| {
                culls_face_between_cells(
                    face_occlusion_shape_indices,
                    skip_rendering,
                    face_material,
                    fluid,
                    cull_face,
                    &neighbor,
                )
            })
        };
        if culled {
            return false;
        }
    }
    true
}

fn fluid_face_culled(
    face: TerrainFace,
    current_face_indices: [u8; 6],
    fluid: Option<TerrainFluid>,
    heights: Option<super::fluid::FluidCornerHeights>,
    neighbor: &TerrainBlockState,
) -> bool {
    if let (Some(current), Some(neighbor_fluid)) = (fluid, neighbor.fluid) {
        if current.kind == neighbor_fluid.kind {
            return true;
        }
    }
    fluid_face_occlusion_culls(
        current_face_indices,
        face,
        neighbor,
        heights.map_or(1.0, |heights| heights.occlusion_height_for_face(face)),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn emit_quads(
    out: &mut TerrainLayerMeshes,
    x: i32,
    y: i32,
    z: i32,
    material: TerrainMaterialClass,
    skip_rendering: TerrainSkipRendering,
    face_occlusion_shape_indices: [u8; 6],
    collision_full_block: bool,
    light: TerrainLight,
    state_light_emission: u8,
    emissive_rendering: bool,
    quads: &[TerrainQuad],
    quad_tint: &dyn Fn(usize, TerrainTint) -> TerrainTint,
    atlas: &TerrainTextureAtlas,
    ambient_occlusion: bool,
    lookup: &TerrainChunkLookup<'_>,
    cardinal: TerrainCardinalLighting,
) {
    for (element, quad) in quads.iter().enumerate() {
        let face_material = effective_face_material(material, quad.transparency);
        if matches!(face_material, TerrainMaterialClass::Empty) {
            continue;
        }
        if let Some(cull_face) = quad.cull {
            let (dx, dy, dz) = cull_offset(cull_face);
            let neighbor = lookup.cell(x + dx, y + dy, z + dz);
            if neighbor
                .map(|neighbor| {
                    culls_face_between_cells(
                        face_occlusion_shape_indices,
                        skip_rendering,
                        face_material,
                        None,
                        cull_face,
                        &neighbor,
                    )
                })
                .unwrap_or(false)
            {
                if let Some(mesh) = out.layer(face_material) {
                    mesh.culled_faces += 1;
                }
                continue;
            }
        }

        // `BlockModelLighter` lights a quad by its baked facing; the cull face only decides
        // culling, and in the flat path which neighbour lights a culled quad.
        let corners = quad
            .corners
            .map(|corner| [corner[0] / 16.0, corner[1] / 16.0, corner[2] / 16.0]);
        let face = face_from_normal(quad.normal);
        let (vertex_lights, ambient_occlusion) = quad_lighting(
            face,
            &corners,
            true,
            quad.cull,
            quad.light_emission,
            ambient_occlusion,
            collision_full_block,
            (x, y, z),
            light,
            state_light_emission,
            emissive_rendering,
            lookup,
        );
        emit_custom_quad_with_uvs(
            out,
            x,
            y,
            z,
            face_material,
            quad_tint(element, quad.tint),
            atlas.rect(quad.texture_index),
            quad.normal,
            corners,
            quad.uvs,
            // Vanilla shades by `BakedQuad.direction`, the baked facing, not the cull face.
            cardinal.shade(quad.shade, face),
            vertex_lights,
            ambient_occlusion,
            false,
            0,
        );
    }
}

/// Per-corner light and ambient shade of one baked quad. Smooth lighting is decided per block
/// state; an element's `light_emission` only raises the result.
#[allow(clippy::too_many_arguments)]
pub(super) fn quad_lighting(
    face: TerrainFace,
    corners: &[[f32; 3]; 4],
    baked_order: bool,
    cull: Option<TerrainFace>,
    light_emission: u8,
    smooth: bool,
    collision_full_block: bool,
    position: (i32, i32, i32),
    light: TerrainLight,
    state_light_emission: u8,
    emissive_rendering: bool,
    lookup: &TerrainChunkLookup<'_>,
) -> ([LightCoords; 4], [f32; 4]) {
    if smooth {
        let render_light = state_render_light(light, state_light_emission, emissive_rendering);
        let (lights, shades) = smooth_quad_lighting(
            face,
            corners,
            baked_order,
            collision_full_block,
            position,
            render_light,
            lookup,
        );
        (
            lights.map(|coords| with_emission(coords, light_emission)),
            shades,
        )
    } else {
        let coords = flat_quad_light(
            face,
            corners,
            cull,
            collision_full_block,
            position,
            light,
            state_light_emission,
            emissive_rendering,
            lookup,
        );
        ([with_emission(coords, light_emission); 4], [1.0; 4])
    }
}

pub(super) fn effective_face_material(
    material: TerrainMaterialClass,
    transparency: TerrainTransparency,
) -> TerrainMaterialClass {
    if matches!(material, TerrainMaterialClass::Fluid) {
        material
    } else if transparency.has_translucent || matches!(material, TerrainMaterialClass::Translucent)
    {
        TerrainMaterialClass::Translucent
    } else if transparency.has_transparent {
        TerrainMaterialClass::Cutout
    } else {
        material
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_custom_quad(
    out: &mut TerrainLayerMeshes,
    x: i32,
    y: i32,
    z: i32,
    material: TerrainMaterialClass,
    tint: TerrainTint,
    uv_rect: TerrainUvRect,
    normal: [f32; 3],
    corners: [[f32; 3]; 4],
    shade: f32,
    vertex_lights: [LightCoords; 4],
) {
    emit_custom_quad_with_uvs(
        out,
        x,
        y,
        z,
        material,
        tint,
        uv_rect,
        normal,
        corners,
        [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
        shade,
        vertex_lights,
        [1.0; 4],
        false,
        0,
    );
}

#[allow(clippy::too_many_arguments)]
fn emit_custom_quad_with_uvs(
    out: &mut TerrainLayerMeshes,
    x: i32,
    y: i32,
    z: i32,
    material: TerrainMaterialClass,
    tint: TerrainTint,
    uv_rect: TerrainUvRect,
    normal: [f32; 3],
    corners: [[f32; 3]; 4],
    uvs: [[f32; 2]; 4],
    shade: f32,
    vertex_lights: [LightCoords; 4],
    ambient_occlusion: [f32; 4],
    reverse_winding: bool,
    diagonal_from: u32,
) {
    let local_x = x.rem_euclid(16) as f32 + out.local_offset[0];
    let local_y = y.rem_euclid(16) as f32 + out.local_offset[1];
    let local_z = z.rem_euclid(16) as f32 + out.local_offset[2];
    let Some(mesh) = out.layer(material) else {
        return;
    };
    let base = mesh.vertices.len() as u32;
    let indices = quad_indices(corners, normal, reverse_winding).map(|i| (i + diagonal_from) % 4);
    let corner_order = if mesh.shared_quad_indices {
        // Keep both oriented triangles and their diagonal when sharing a quad index buffer.
        if indices[2] == indices[4] {
            [indices[0], indices[1], indices[2], indices[5]]
        } else {
            [indices[1], indices[2], indices[0], indices[4]]
        }
    } else {
        mesh.indices.extend(indices.map(|i| base + i));
        [0, 1, 2, 3]
    };
    for index in corner_order {
        let index = index as usize;
        let corner = corners[index];
        mesh.vertices.push(TerrainGpuVertex::pack(
            [
                local_x + corner[0],
                local_y + corner[1],
                local_z + corner[2],
            ],
            uv_rect.map(uvs[index]),
            vertex_lights[index],
            tint,
            shade,
            ambient_occlusion[index],
        ));
    }
    match material {
        TerrainMaterialClass::Opaque => mesh.opaque_faces += 1,
        TerrainMaterialClass::Cutout => mesh.cutout_faces += 1,
        TerrainMaterialClass::Fluid | TerrainMaterialClass::Translucent => {
            mesh.translucent_faces += 1
        }
        TerrainMaterialClass::Empty => {}
    }
}

/// The box corner holding `FaceInfo` vertex 0; Vanilla splits every quad along vertices 0-2.
fn face_info_first(face: TerrainFace) -> u32 {
    match face {
        TerrainFace::Down | TerrainFace::Up => 0,
        TerrainFace::North | TerrainFace::South | TerrainFace::West | TerrainFace::East => 1,
    }
}

fn quad_indices(corners: [[f32; 3]; 4], normal: [f32; 3], reverse_winding: bool) -> [u32; 6] {
    let ab = [
        corners[1][0] - corners[0][0],
        corners[1][1] - corners[0][1],
        corners[1][2] - corners[0][2],
    ];
    let ac = [
        corners[2][0] - corners[0][0],
        corners[2][1] - corners[0][1],
        corners[2][2] - corners[0][2],
    ];
    let geometric_normal = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    let dot = geometric_normal[0] * normal[0]
        + geometric_normal[1] * normal[1]
        + geometric_normal[2] * normal[2];
    if (dot < 0.0) == reverse_winding {
        [0, 1, 2, 0, 2, 3]
    } else {
        [0, 2, 1, 0, 3, 2]
    }
}

#[allow(clippy::too_many_arguments)]
fn fluid_face_vertex_lights(
    face: TerrainFace,
    x: i32,
    y: i32,
    z: i32,
    light: TerrainLight,
    state_light_emission: u8,
    emissive_rendering: bool,
    light_emission: u8,
    lookup: &TerrainChunkLookup<'_>,
) -> [LightCoords; 4] {
    let light = state_render_light(light, state_light_emission, emissive_rendering);
    let neighbor_y = match face {
        TerrainFace::Down => y - 1,
        TerrainFace::Up
        | TerrainFace::North
        | TerrainFace::South
        | TerrainFace::West
        | TerrainFace::East => y + 1,
    };
    let neighbor = lookup
        .cell(x, neighbor_y, z)
        .map_or(TerrainLight { sky: 0, block: 0 }, render_light_for_cell);
    let light = TerrainLight {
        block: light.block.max(neighbor.block),
        sky: light.sky.max(neighbor.sky),
    };
    [with_emission(pack(light), light_emission); 4]
}

pub(super) fn face_from_normal(normal: [f32; 3]) -> TerrainFace {
    let [x, y, z] = normal;
    let abs_x = x.abs();
    let abs_y = y.abs();
    let abs_z = z.abs();
    if abs_y >= abs_x && abs_y >= abs_z {
        if y < 0.0 {
            TerrainFace::Down
        } else {
            TerrainFace::Up
        }
    } else if abs_z >= abs_x {
        if z < 0.0 {
            TerrainFace::North
        } else {
            TerrainFace::South
        }
    } else if x < 0.0 {
        TerrainFace::West
    } else {
        TerrainFace::East
    }
}
