use super::super::{
    TerrainCell, TerrainFace, TerrainFluid, TerrainFluidKind, TerrainLight, TerrainMaterialClass,
    TerrainMesh, TerrainQuad, TerrainRenderShape, TerrainTextureAtlas, TerrainTint,
    TerrainTransparency, TerrainUvRect, TerrainVertex,
};
use super::{
    culls_face_between_cells,
    geometry::{box_face_corners, face_uvs_from_crop, FaceDef, CROSS_FACES, FACES},
    TerrainChunkLookup, TerrainMeshMode,
};

const FLUID_FACE_OFFSET: f32 = 0.001;

pub(super) fn emit_face(
    mesh: &mut TerrainMesh,
    x: i32,
    y: i32,
    z: i32,
    block_state_id: i32,
    material: TerrainMaterialClass,
    tint: TerrainTint,
    uv_rect: TerrainUvRect,
    face: FaceDef,
    vertex_lights: [[f32; 2]; 4],
    ambient_occlusion: [f32; 4],
) {
    let base = mesh.vertices.len() as u32;
    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    for (((corner, uv), light), ambient_occlusion) in face
        .corners
        .into_iter()
        .zip(uvs)
        .zip(vertex_lights)
        .zip(ambient_occlusion)
    {
        mesh.vertices.push(TerrainVertex {
            position: [
                x as f32 + corner[0],
                y as f32 + corner[1],
                z as f32 + corner[2],
            ],
            normal: face.normal,
            uv: uv_rect.map(uv),
            light,
            tint: tint.as_shader_tint(),
            shade: cardinal_shade(true, face.face),
            ambient_occlusion,
            block_state_id,
        });
    }
    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    match material {
        TerrainMaterialClass::Opaque => mesh.opaque_faces += 1,
        TerrainMaterialClass::Cutout => mesh.cutout_faces += 1,
        TerrainMaterialClass::Fluid | TerrainMaterialClass::Translucent => {
            mesh.translucent_faces += 1
        }
        _ => {}
    }
}

pub(super) fn emit_cross(
    mesh: &mut TerrainMesh,
    x: i32,
    y: i32,
    z: i32,
    block_state_id: i32,
    material: TerrainMaterialClass,
    light: TerrainLight,
    tint: [TerrainTint; 6],
    texture_indices: [u32; 6],
    face_transparency: [TerrainTransparency; 6],
    shade: bool,
    light_emission: u8,
    atlas: &TerrainTextureAtlas,
    mode: TerrainMeshMode,
) {
    for (face, normal, corners) in CROSS_FACES {
        let face_material = effective_face_material(material, face_transparency[face.index()]);
        if !mode.is_meshed(face_material) {
            continue;
        }
        emit_custom_quad(
            mesh,
            x,
            y,
            z,
            block_state_id,
            face_material,
            tint[face.index()],
            atlas.rect(texture_indices[face.index()]),
            normal,
            corners,
            cardinal_shade(shade, face),
            [shader_light_with_emission(light, light_emission); 4],
        );
    }
}

pub(super) fn emit_box(
    mesh: &mut TerrainMesh,
    x: i32,
    y: i32,
    z: i32,
    block_state_id: i32,
    material: TerrainMaterialClass,
    fluid: Option<TerrainFluid>,
    light: TerrainLight,
    tint: [TerrainTint; 6],
    texture_indices: [u32; 6],
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
    mode: TerrainMeshMode,
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
    let fluid_top_offset = if is_fluid
        && box_face_will_render(
            TerrainFace::Up,
            x,
            y,
            z,
            face_present,
            face_transparency,
            face_cull,
            material,
            fluid,
            mode,
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
            material,
            fluid,
            mode,
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
        if !mode.is_meshed(face_material) {
            continue;
        }
        if let Some(cull_face) = face_cull[face_index] {
            let (dx, dy, dz) = cull_offset(cull_face);
            let neighbor = lookup.cell(x + dx, y + dy, z + dz);
            if neighbor
                .map(|neighbor| culls_face_between_cells(mode, face_material, fluid, neighbor))
                .unwrap_or(false)
            {
                mesh.culled_faces += 1;
                continue;
            }
        }

        let smooth_light = !is_fluid && ambient_occlusion && face_light_emission[face_index] == 0;
        let face_cubic = box_face_is_cubic(face.face, min, max);
        let fluid_heights = is_fluid.then(|| {
            fluid_corner_heights(x, y, z, max[1], fluid.map(|fluid| fluid.kind), lookup)
                .with_top_offset(fluid_top_offset)
        });
        let texture_index =
            fluid_face_texture_index(face.face, texture_indices, fluid_flow.is_some());
        let ambient_occlusion = if is_fluid {
            [1.0; 4]
        } else {
            face_ambient_occlusion(smooth_light, face.face, x, y, z, face_cubic, lookup, mode)
        };
        let vertex_lights = if is_fluid {
            fluid_face_vertex_lights(
                face.face,
                x,
                y,
                z,
                light,
                face_light_emission[face_index],
                lookup,
            )
        } else {
            face_vertex_lights(
                smooth_light,
                face.face,
                x,
                y,
                z,
                face_cubic,
                light,
                face_light_emission[face_index],
                lookup,
                mode,
            )
        };
        emit_custom_quad_with_uvs(
            mesh,
            x,
            y,
            z,
            block_state_id,
            face_material,
            tint[face_index],
            atlas.rect(texture_index),
            face.normal,
            fluid_heights
                .map(|heights| {
                    fluid_face_corners(face.face, min, max, heights, fluid_bottom_offset)
                })
                .unwrap_or_else(|| box_face_corners(face.face, min, max)),
            fluid_heights
                .map(|heights| fluid_face_uvs(face.face, heights, fluid_flow))
                .unwrap_or_else(|| {
                    face_uvs_from_crop(face_uvs[face_index], face_uv_rotations[face_index])
                }),
            cardinal_shade(face_shade[face_index], face.face),
            vertex_lights,
            ambient_occlusion,
        );
    }
}

fn box_face_will_render(
    face: TerrainFace,
    x: i32,
    y: i32,
    z: i32,
    face_present: [bool; 6],
    face_transparency: [TerrainTransparency; 6],
    face_cull: [Option<TerrainFace>; 6],
    material: TerrainMaterialClass,
    fluid: Option<TerrainFluid>,
    mode: TerrainMeshMode,
    lookup: &TerrainChunkLookup<'_>,
) -> bool {
    let face_index = face.index();
    if !face_present[face_index] {
        return false;
    }
    let face_material = effective_face_material(material, face_transparency[face_index]);
    if !mode.is_meshed(face_material) {
        return false;
    }
    if let Some(cull_face) = face_cull[face_index] {
        let (dx, dy, dz) = cull_offset(cull_face);
        if lookup
            .cell(x + dx, y + dy, z + dz)
            .is_some_and(|neighbor| culls_face_between_cells(mode, face_material, fluid, neighbor))
        {
            return false;
        }
    }
    true
}

pub(super) fn emit_quads(
    mesh: &mut TerrainMesh,
    x: i32,
    y: i32,
    z: i32,
    block_state_id: i32,
    material: TerrainMaterialClass,
    light: TerrainLight,
    quads: &[TerrainQuad],
    atlas: &TerrainTextureAtlas,
    ambient_occlusion: bool,
    lookup: &TerrainChunkLookup<'_>,
    mode: TerrainMeshMode,
) {
    for quad in quads {
        let face_material = effective_face_material(material, quad.transparency);
        if !mode.is_meshed(face_material) {
            continue;
        }
        if let Some(cull_face) = quad.cull {
            let (dx, dy, dz) = cull_offset(cull_face);
            let neighbor = lookup.cell(x + dx, y + dy, z + dz);
            if neighbor
                .map(|neighbor| mode.culls_face_between(face_material, neighbor.material))
                .unwrap_or(false)
            {
                mesh.culled_faces += 1;
                continue;
            }
        }

        let smooth_light = ambient_occlusion && quad.light_emission == 0;
        let ambient_occlusion = quad
            .cull
            .filter(|_| smooth_light)
            .map(|face| face_ambient_occlusion(true, face, x, y, z, true, lookup, mode))
            .unwrap_or([1.0; 4]);
        let vertex_lights = quad
            .cull
            .filter(|_| smooth_light)
            .map(|face| {
                face_vertex_lights(
                    true,
                    face,
                    x,
                    y,
                    z,
                    true,
                    light,
                    quad.light_emission,
                    lookup,
                    mode,
                )
            })
            .unwrap_or([shader_light_with_emission(light, quad.light_emission); 4]);
        emit_custom_quad_with_uvs(
            mesh,
            x,
            y,
            z,
            block_state_id,
            face_material,
            quad.tint,
            atlas.rect(quad.texture_index),
            quad.normal,
            quad.corners
                .map(|corner| [corner[0] / 16.0, corner[1] / 16.0, corner[2] / 16.0]),
            quad.uvs,
            cardinal_shade(
                quad.shade,
                quad.cull.unwrap_or_else(|| face_from_normal(quad.normal)),
            ),
            vertex_lights,
            ambient_occlusion,
        );
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

fn emit_custom_quad(
    mesh: &mut TerrainMesh,
    x: i32,
    y: i32,
    z: i32,
    block_state_id: i32,
    material: TerrainMaterialClass,
    tint: TerrainTint,
    uv_rect: TerrainUvRect,
    normal: [f32; 3],
    corners: [[f32; 3]; 4],
    shade: f32,
    vertex_lights: [[f32; 2]; 4],
) {
    emit_custom_quad_with_uvs(
        mesh,
        x,
        y,
        z,
        block_state_id,
        material,
        tint,
        uv_rect,
        normal,
        corners,
        [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
        shade,
        vertex_lights,
        [1.0; 4],
    );
}

fn emit_custom_quad_with_uvs(
    mesh: &mut TerrainMesh,
    x: i32,
    y: i32,
    z: i32,
    block_state_id: i32,
    material: TerrainMaterialClass,
    tint: TerrainTint,
    uv_rect: TerrainUvRect,
    normal: [f32; 3],
    corners: [[f32; 3]; 4],
    uvs: [[f32; 2]; 4],
    shade: f32,
    vertex_lights: [[f32; 2]; 4],
    ambient_occlusion: [f32; 4],
) {
    let base = mesh.vertices.len() as u32;
    for (((corner, uv), light), ambient_occlusion) in corners
        .into_iter()
        .zip(uvs)
        .zip(vertex_lights)
        .zip(ambient_occlusion)
    {
        mesh.vertices.push(TerrainVertex {
            position: [
                x as f32 + corner[0],
                y as f32 + corner[1],
                z as f32 + corner[2],
            ],
            normal,
            uv: uv_rect.map(uv),
            light,
            tint: tint.as_shader_tint(),
            shade,
            ambient_occlusion,
            block_state_id,
        });
    }
    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    match material {
        TerrainMaterialClass::Opaque => mesh.opaque_faces += 1,
        TerrainMaterialClass::Cutout => mesh.cutout_faces += 1,
        TerrainMaterialClass::Fluid | TerrainMaterialClass::Translucent => {
            mesh.translucent_faces += 1
        }
        _ => {}
    }
}

fn shader_light_with_emission(light: TerrainLight, light_emission: u8) -> [f32; 2] {
    let mut shader_light = light.as_shader_light();
    shader_light[0] = shader_light[0].max(light_emission.min(15) as f32 / 15.0);
    shader_light
}

fn fluid_face_vertex_lights(
    face: TerrainFace,
    x: i32,
    y: i32,
    z: i32,
    light: TerrainLight,
    light_emission: u8,
    lookup: &TerrainChunkLookup<'_>,
) -> [[f32; 2]; 4] {
    let neighbor = match face {
        TerrainFace::Down => sample_light(x, y - 1, z, lookup),
        TerrainFace::Up
        | TerrainFace::North
        | TerrainFace::South
        | TerrainFace::West
        | TerrainFace::East => sample_light(x, y + 1, z, lookup),
    }
    .unwrap_or(TerrainLight { sky: 0, block: 0 });
    [shader_light_with_emission(max_light(light, neighbor), light_emission); 4]
}

#[derive(Debug, Clone, Copy)]
struct FluidCornerHeights {
    north_west: f32,
    north_east: f32,
    south_east: f32,
    south_west: f32,
}

impl FluidCornerHeights {
    fn with_top_offset(self, offset: f32) -> Self {
        if offset == 0.0 {
            return self;
        }
        Self {
            north_west: (self.north_west - offset).max(0.0),
            north_east: (self.north_east - offset).max(0.0),
            south_east: (self.south_east - offset).max(0.0),
            south_west: (self.south_west - offset).max(0.0),
        }
    }
}

fn fluid_corner_heights(
    x: i32,
    y: i32,
    z: i32,
    self_height: f32,
    kind: Option<TerrainFluidKind>,
    lookup: &TerrainChunkLookup<'_>,
) -> FluidCornerHeights {
    if self_height >= 1.0 {
        return FluidCornerHeights {
            north_west: 1.0,
            north_east: 1.0,
            south_east: 1.0,
            south_west: 1.0,
        };
    }

    let north = fluid_height_at(x, y, z - 1, kind, lookup);
    let south = fluid_height_at(x, y, z + 1, kind, lookup);
    let west = fluid_height_at(x - 1, y, z, kind, lookup);
    let east = fluid_height_at(x + 1, y, z, kind, lookup);

    FluidCornerHeights {
        north_west: average_fluid_corner_height(
            self_height,
            north,
            west,
            fluid_height_at(x - 1, y, z - 1, kind, lookup),
        ),
        north_east: average_fluid_corner_height(
            self_height,
            north,
            east,
            fluid_height_at(x + 1, y, z - 1, kind, lookup),
        ),
        south_east: average_fluid_corner_height(
            self_height,
            south,
            east,
            fluid_height_at(x + 1, y, z + 1, kind, lookup),
        ),
        south_west: average_fluid_corner_height(
            self_height,
            south,
            west,
            fluid_height_at(x - 1, y, z + 1, kind, lookup),
        ),
    }
}

fn fluid_face_corners(
    face: TerrainFace,
    min: [f32; 3],
    max: [f32; 3],
    heights: FluidCornerHeights,
    bottom_offset: f32,
) -> [[f32; 3]; 4] {
    let bottom = min[1] + bottom_offset;
    match face {
        TerrainFace::Down => [
            [min[0], bottom, max[2]],
            [max[0], bottom, max[2]],
            [max[0], bottom, min[2]],
            [min[0], bottom, min[2]],
        ],
        TerrainFace::Up => [
            [min[0], heights.north_west, min[2]],
            [max[0], heights.north_east, min[2]],
            [max[0], heights.south_east, max[2]],
            [min[0], heights.south_west, max[2]],
        ],
        TerrainFace::North => [
            [max[0], bottom, min[2] + FLUID_FACE_OFFSET],
            [max[0], heights.north_east, min[2] + FLUID_FACE_OFFSET],
            [min[0], heights.north_west, min[2] + FLUID_FACE_OFFSET],
            [min[0], bottom, min[2] + FLUID_FACE_OFFSET],
        ],
        TerrainFace::South => [
            [min[0], bottom, max[2] - FLUID_FACE_OFFSET],
            [min[0], heights.south_west, max[2] - FLUID_FACE_OFFSET],
            [max[0], heights.south_east, max[2] - FLUID_FACE_OFFSET],
            [max[0], bottom, max[2] - FLUID_FACE_OFFSET],
        ],
        TerrainFace::West => [
            [min[0] + FLUID_FACE_OFFSET, bottom, min[2]],
            [min[0] + FLUID_FACE_OFFSET, heights.north_west, min[2]],
            [min[0] + FLUID_FACE_OFFSET, heights.south_west, max[2]],
            [min[0] + FLUID_FACE_OFFSET, bottom, max[2]],
        ],
        TerrainFace::East => [
            [max[0] - FLUID_FACE_OFFSET, bottom, max[2]],
            [max[0] - FLUID_FACE_OFFSET, heights.south_east, max[2]],
            [max[0] - FLUID_FACE_OFFSET, heights.north_east, min[2]],
            [max[0] - FLUID_FACE_OFFSET, bottom, min[2]],
        ],
    }
}

#[derive(Debug, Clone, Copy)]
struct FluidFlow {
    x: f32,
    z: f32,
}

fn fluid_face_texture_index(
    face: TerrainFace,
    texture_indices: [u32; 6],
    has_top_flow: bool,
) -> u32 {
    if matches!(face, TerrainFace::Up) && has_top_flow {
        texture_indices[TerrainFace::North.index()]
    } else {
        texture_indices[face.index()]
    }
}

fn fluid_top_flow(
    x: i32,
    y: i32,
    z: i32,
    fluid: TerrainFluid,
    lookup: &TerrainChunkLookup<'_>,
) -> Option<FluidFlow> {
    let own_height = fluid.own_height();
    let mut flow_x = 0.0;
    let mut flow_z = 0.0;
    for (dx, dz) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
        let Some(distance) = fluid_flow_distance(own_height, fluid.kind, x + dx, y, z + dz, lookup)
        else {
            continue;
        };
        if distance != 0.0 {
            flow_x += dx as f32 * distance;
            flow_z += dz as f32 * distance;
        }
    }
    normalized_fluid_flow(flow_x, flow_z)
}

fn fluid_flow_distance(
    own_height: f32,
    kind: TerrainFluidKind,
    x: i32,
    y: i32,
    z: i32,
    lookup: &TerrainChunkLookup<'_>,
) -> Option<f32> {
    let neighbor = lookup.cell(x, y, z);
    let neighbor_height = match neighbor {
        Some(cell) if matches!(cell.material, TerrainMaterialClass::Fluid) => {
            same_fluid_own_height(cell, kind)?
        }
        Some(cell) if material_blocks_motion(cell.material) => return None,
        Some(_) | None => 0.0,
    };

    if neighbor_height == 0.0 {
        let blocks_motion = neighbor
            .map(|cell| material_blocks_motion(cell.material))
            .unwrap_or(false);
        if blocks_motion {
            return Some(0.0);
        }
        return same_fluid_own_height_at(x, y - 1, z, kind, lookup)
            .filter(|height| *height > 0.0)
            .map(|height| own_height - (height - 0.888_888_9))
            .or(Some(0.0));
    }

    Some(own_height - neighbor_height)
}

fn same_fluid_own_height_at(
    x: i32,
    y: i32,
    z: i32,
    kind: TerrainFluidKind,
    lookup: &TerrainChunkLookup<'_>,
) -> Option<f32> {
    lookup
        .cell(x, y, z)
        .and_then(|cell| same_fluid_own_height(cell, kind))
}

fn same_fluid_own_height(cell: &TerrainCell, kind: TerrainFluidKind) -> Option<f32> {
    if let Some(fluid) = cell.fluid {
        return (fluid.kind == kind).then(|| fluid.own_height());
    }
    matches!(cell.material, TerrainMaterialClass::Fluid)
        .then(|| fluid_cell_height(cell))
        .flatten()
}

fn material_blocks_motion(material: TerrainMaterialClass) -> bool {
    matches!(material, TerrainMaterialClass::Opaque)
}

fn normalized_fluid_flow(x: f32, z: f32) -> Option<FluidFlow> {
    let length = (x * x + z * z).sqrt();
    (length > 0.0001).then(|| FluidFlow {
        x: x / length,
        z: z / length,
    })
}

fn fluid_face_uvs(
    face: TerrainFace,
    heights: FluidCornerHeights,
    top_flow: Option<FluidFlow>,
) -> [[f32; 2]; 4] {
    match face {
        TerrainFace::Up => top_flow.map(fluid_top_flow_uvs).unwrap_or([
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
        ]),
        TerrainFace::Down => [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        TerrainFace::North => [
            [0.5, 0.5],
            [0.5, fluid_side_v(heights.north_east)],
            [0.0, fluid_side_v(heights.north_west)],
            [0.0, 0.5],
        ],
        TerrainFace::South => [
            [0.5, 0.5],
            [0.5, fluid_side_v(heights.south_west)],
            [0.0, fluid_side_v(heights.south_east)],
            [0.0, 0.5],
        ],
        TerrainFace::West => [
            [0.5, 0.5],
            [0.5, fluid_side_v(heights.north_west)],
            [0.0, fluid_side_v(heights.south_west)],
            [0.0, 0.5],
        ],
        TerrainFace::East => [
            [0.5, 0.5],
            [0.5, fluid_side_v(heights.south_east)],
            [0.0, fluid_side_v(heights.north_east)],
            [0.0, 0.5],
        ],
    }
}

fn fluid_top_flow_uvs(flow: FluidFlow) -> [[f32; 2]; 4] {
    let angle = flow.z.atan2(flow.x) - std::f32::consts::FRAC_PI_2;
    let s = angle.sin() * 0.25;
    let c = angle.cos() * 0.25;
    let u00 = 0.5 + (-c - s);
    let v00 = 0.5 + (-c + s);
    let u01 = 0.5 + (-c + s);
    let v01 = 0.5 + (c + s);
    let u10 = 0.5 + (c + s);
    let v10 = 0.5 + (c - s);
    let u11 = 0.5 + (c - s);
    let v11 = 0.5 + (-c - s);

    [[u00, v00], [u11, v11], [u10, v10], [u01, v01]]
}

fn fluid_side_v(height: f32) -> f32 {
    (1.0 - height) * 0.5
}

fn fluid_self_height(
    fluid: Option<TerrainFluid>,
    fallback_height: f32,
    x: i32,
    y: i32,
    z: i32,
    lookup: &TerrainChunkLookup<'_>,
) -> f32 {
    let Some(fluid) = fluid else {
        return if lookup
            .cell(x, y + 1, z)
            .is_some_and(|above| matches!(above.material, TerrainMaterialClass::Fluid))
        {
            1.0
        } else {
            fallback_height
        };
    };
    if lookup
        .cell(x, y + 1, z)
        .is_some_and(|above| same_fluid_own_height(above, fluid.kind).is_some())
    {
        1.0
    } else {
        fluid.own_height()
    }
}

fn fluid_height_at(
    x: i32,
    y: i32,
    z: i32,
    kind: Option<TerrainFluidKind>,
    lookup: &TerrainChunkLookup<'_>,
) -> f32 {
    let Some(cell) = lookup.cell(x, y, z) else {
        return 0.0;
    };
    match cell.material {
        TerrainMaterialClass::Fluid => {
            if let Some(kind) = kind {
                if lookup
                    .cell(x, y + 1, z)
                    .is_some_and(|above| same_fluid_own_height(above, kind).is_some())
                {
                    return 1.0;
                }
                return same_fluid_own_height(cell, kind).unwrap_or(0.0);
            }
            if lookup
                .cell(x, y + 1, z)
                .is_some_and(|above| matches!(above.material, TerrainMaterialClass::Fluid))
            {
                return 1.0;
            }
            fluid_cell_height(cell).unwrap_or(1.0)
        }
        TerrainMaterialClass::Opaque => -1.0,
        TerrainMaterialClass::Empty
        | TerrainMaterialClass::Cutout
        | TerrainMaterialClass::Translucent => 0.0,
    }
}

fn fluid_cell_height(cell: &TerrainCell) -> Option<f32> {
    match &cell.render_shape {
        TerrainRenderShape::Box { to, .. } => Some((to[1] as f32 / 16.0).clamp(0.0, 1.0)),
        _ => None,
    }
}

fn average_fluid_corner_height(self_height: f32, side_a: f32, side_b: f32, corner: f32) -> f32 {
    if side_a >= 1.0 || side_b >= 1.0 {
        return 1.0;
    }

    let mut weighted_height = [0.0, 0.0];
    if side_a > 0.0 || side_b > 0.0 {
        if corner >= 1.0 {
            return 1.0;
        }
        add_weighted_fluid_height(&mut weighted_height, corner);
    }

    add_weighted_fluid_height(&mut weighted_height, self_height);
    add_weighted_fluid_height(&mut weighted_height, side_a);
    add_weighted_fluid_height(&mut weighted_height, side_b);
    if weighted_height[1] == 0.0 {
        0.0
    } else {
        weighted_height[0] / weighted_height[1]
    }
}

fn add_weighted_fluid_height(weighted_height: &mut [f32; 2], height: f32) {
    if height >= 0.8 {
        weighted_height[0] += height * 10.0;
        weighted_height[1] += 10.0;
    } else if height >= 0.0 {
        weighted_height[0] += height;
        weighted_height[1] += 1.0;
    }
}

pub(super) fn face_vertex_lights(
    enabled: bool,
    face: TerrainFace,
    x: i32,
    y: i32,
    z: i32,
    face_cubic: bool,
    light: TerrainLight,
    light_emission: u8,
    lookup: &TerrainChunkLookup<'_>,
    mode: TerrainMeshMode,
) -> [[f32; 2]; 4] {
    if !enabled {
        return [shader_light_with_emission(light, light_emission); 4];
    }

    let normal = if face_cubic {
        cull_offset(face)
    } else {
        (0, 0, 0)
    };
    let base = (x + normal.0, y + normal.1, z + normal.2);
    let center_light = if face_cubic {
        sample_light(base.0, base.1, base.2, lookup).unwrap_or(light)
    } else {
        light
    };

    std::array::from_fn(|corner| {
        let (side_a, side_b) = face_corner_sides(face, corner);
        let (side_a_occludes, side_a_light) = light_sample(
            base.0 + side_a.0,
            base.1 + side_a.1,
            base.2 + side_a.2,
            center_light,
            lookup,
            mode,
        );
        let (side_b_occludes, side_b_light) = light_sample(
            base.0 + side_b.0,
            base.1 + side_b.1,
            base.2 + side_b.2,
            center_light,
            lookup,
            mode,
        );
        let corner_light = if side_a_occludes && side_b_occludes {
            side_a_light
        } else {
            light_sample(
                base.0 + side_a.0 + side_b.0,
                base.1 + side_a.1 + side_b.1,
                base.2 + side_a.2 + side_b.2,
                center_light,
                lookup,
                mode,
            )
            .1
        };
        shader_light_with_emission(
            smooth_light_blend(side_a_light, side_b_light, corner_light, center_light),
            light_emission,
        )
    })
}

fn light_sample(
    x: i32,
    y: i32,
    z: i32,
    fallback: TerrainLight,
    lookup: &TerrainChunkLookup<'_>,
    mode: TerrainMeshMode,
) -> (bool, TerrainLight) {
    lookup
        .cell(x, y, z)
        .map(|cell| (mode.is_occluded_by(cell.material), cell.light))
        .unwrap_or((false, fallback))
}

fn sample_light(x: i32, y: i32, z: i32, lookup: &TerrainChunkLookup<'_>) -> Option<TerrainLight> {
    lookup.cell(x, y, z).map(|cell| cell.light)
}

fn smooth_light_blend(
    mut neighbor_a: TerrainLight,
    mut neighbor_b: TerrainLight,
    mut corner: TerrainLight,
    center: TerrainLight,
) -> TerrainLight {
    if center.sky > 2 || center.block > 2 {
        fill_zero_light_channels(&mut neighbor_a, center);
        fill_zero_light_channels(&mut neighbor_b, center);
        fill_zero_light_channels(&mut corner, center);
    }

    TerrainLight {
        sky: ((u16::from(neighbor_a.sky)
            + u16::from(neighbor_b.sky)
            + u16::from(corner.sky)
            + u16::from(center.sky))
            / 4) as u8,
        block: ((u16::from(neighbor_a.block)
            + u16::from(neighbor_b.block)
            + u16::from(corner.block)
            + u16::from(center.block))
            / 4) as u8,
    }
}

fn fill_zero_light_channels(light: &mut TerrainLight, center: TerrainLight) {
    if light.sky == 0 {
        light.sky = center.sky;
    }
    if light.block == 0 {
        light.block = center.block;
    }
}

fn max_light(left: TerrainLight, right: TerrainLight) -> TerrainLight {
    TerrainLight {
        sky: left.sky.max(right.sky),
        block: left.block.max(right.block),
    }
}

fn cardinal_shade(enabled: bool, face: TerrainFace) -> f32 {
    if !enabled {
        return 1.0;
    }
    match face {
        TerrainFace::Down => 0.5,
        TerrainFace::Up => 1.0,
        TerrainFace::North | TerrainFace::South => 0.8,
        TerrainFace::West | TerrainFace::East => 0.6,
    }
}

fn face_from_normal(normal: [f32; 3]) -> TerrainFace {
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

fn cull_offset(face: TerrainFace) -> (i32, i32, i32) {
    match face {
        TerrainFace::Down => (0, -1, 0),
        TerrainFace::Up => (0, 1, 0),
        TerrainFace::North => (0, 0, -1),
        TerrainFace::South => (0, 0, 1),
        TerrainFace::West => (-1, 0, 0),
        TerrainFace::East => (1, 0, 0),
    }
}

pub(super) fn face_ambient_occlusion(
    enabled: bool,
    face: TerrainFace,
    x: i32,
    y: i32,
    z: i32,
    face_cubic: bool,
    lookup: &TerrainChunkLookup<'_>,
    mode: TerrainMeshMode,
) -> [f32; 4] {
    if !enabled {
        return [1.0; 4];
    }

    let normal = if face_cubic {
        cull_offset(face)
    } else {
        (0, 0, 0)
    };
    let base = (x + normal.0, y + normal.1, z + normal.2);
    std::array::from_fn(|corner| {
        let (side_a, side_b) = face_corner_sides(face, corner);
        vertex_ambient_occlusion(base, side_a, side_b, lookup, mode)
    })
}

fn vertex_ambient_occlusion(
    base: (i32, i32, i32),
    side_a: (i32, i32, i32),
    side_b: (i32, i32, i32),
    lookup: &TerrainChunkLookup<'_>,
    mode: TerrainMeshMode,
) -> f32 {
    let (side_a_occludes, side_a_brightness) = ambient_sample(
        base.0 + side_a.0,
        base.1 + side_a.1,
        base.2 + side_a.2,
        lookup,
        mode,
    );
    let (side_b_occludes, side_b_brightness) = ambient_sample(
        base.0 + side_b.0,
        base.1 + side_b.1,
        base.2 + side_b.2,
        lookup,
        mode,
    );
    let corner_brightness = if side_a_occludes && side_b_occludes {
        side_a_brightness
    } else {
        ambient_sample(
            base.0 + side_a.0 + side_b.0,
            base.1 + side_a.1 + side_b.1,
            base.2 + side_a.2 + side_b.2,
            lookup,
            mode,
        )
        .1
    };

    (side_a_brightness + side_b_brightness + corner_brightness + 1.0) * 0.25
}

fn ambient_sample(
    x: i32,
    y: i32,
    z: i32,
    lookup: &TerrainChunkLookup<'_>,
    mode: TerrainMeshMode,
) -> (bool, f32) {
    let occludes = lookup
        .cell(x, y, z)
        .map(|cell| mode.is_occluded_by(cell.material))
        .unwrap_or(false);
    (occludes, if occludes { 0.2 } else { 1.0 })
}

fn box_face_is_cubic(face: TerrainFace, min: [f32; 3], max: [f32; 3]) -> bool {
    const MIN: f32 = 0.0001;
    const MAX: f32 = 0.9999;
    match face {
        TerrainFace::Down => min[1] <= MIN,
        TerrainFace::Up => max[1] >= MAX,
        TerrainFace::North => min[2] <= MIN,
        TerrainFace::South => max[2] >= MAX,
        TerrainFace::West => min[0] <= MIN,
        TerrainFace::East => max[0] >= MAX,
    }
}

fn face_corner_sides(face: TerrainFace, corner: usize) -> ((i32, i32, i32), (i32, i32, i32)) {
    match (face, corner) {
        (TerrainFace::Down, 0) => ((-1, 0, 0), (0, 0, 1)),
        (TerrainFace::Down, 1) => ((1, 0, 0), (0, 0, 1)),
        (TerrainFace::Down, 2) => ((1, 0, 0), (0, 0, -1)),
        (TerrainFace::Down, 3) => ((-1, 0, 0), (0, 0, -1)),
        (TerrainFace::Up, 0) => ((-1, 0, 0), (0, 0, -1)),
        (TerrainFace::Up, 1) => ((1, 0, 0), (0, 0, -1)),
        (TerrainFace::Up, 2) => ((1, 0, 0), (0, 0, 1)),
        (TerrainFace::Up, 3) => ((-1, 0, 0), (0, 0, 1)),
        (TerrainFace::North, 0) => ((1, 0, 0), (0, -1, 0)),
        (TerrainFace::North, 1) => ((1, 0, 0), (0, 1, 0)),
        (TerrainFace::North, 2) => ((-1, 0, 0), (0, 1, 0)),
        (TerrainFace::North, 3) => ((-1, 0, 0), (0, -1, 0)),
        (TerrainFace::South, 0) => ((-1, 0, 0), (0, -1, 0)),
        (TerrainFace::South, 1) => ((-1, 0, 0), (0, 1, 0)),
        (TerrainFace::South, 2) => ((1, 0, 0), (0, 1, 0)),
        (TerrainFace::South, 3) => ((1, 0, 0), (0, -1, 0)),
        (TerrainFace::West, 0) => ((0, 0, -1), (0, -1, 0)),
        (TerrainFace::West, 1) => ((0, 0, -1), (0, 1, 0)),
        (TerrainFace::West, 2) => ((0, 0, 1), (0, 1, 0)),
        (TerrainFace::West, 3) => ((0, 0, 1), (0, -1, 0)),
        (TerrainFace::East, 0) => ((0, 0, 1), (0, -1, 0)),
        (TerrainFace::East, 1) => ((0, 0, 1), (0, 1, 0)),
        (TerrainFace::East, 2) => ((0, 0, -1), (0, 1, 0)),
        (TerrainFace::East, 3) => ((0, 0, -1), (0, -1, 0)),
        _ => unreachable!("terrain quads have four corners"),
    }
}
