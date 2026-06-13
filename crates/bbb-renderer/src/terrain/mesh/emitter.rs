use super::super::{
    TerrainFace, TerrainLight, TerrainMaterialClass, TerrainMesh, TerrainQuad, TerrainTextureAtlas,
    TerrainTint, TerrainTransparency, TerrainUvRect, TerrainVertex,
};
use super::{
    geometry::{box_face_corners, face_uvs_from_crop, FaceDef, CROSS_FACES, FACES},
    TerrainChunkLookup, TerrainMeshMode,
};

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
    if matches!(material, TerrainMaterialClass::Fluid)
        && lookup
            .cell(x, y + 1, z)
            .is_some_and(|neighbor| matches!(neighbor.material, TerrainMaterialClass::Fluid))
    {
        max[1] = 1.0;
    }

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
                .map(|neighbor| mode.culls_face_between(face_material, neighbor.material))
                .unwrap_or(false)
            {
                mesh.culled_faces += 1;
                continue;
            }
        }

        let smooth_light = ambient_occlusion && face_light_emission[face_index] == 0;
        let face_cubic = box_face_is_cubic(face.face, min, max);
        let ambient_occlusion =
            face_ambient_occlusion(smooth_light, face.face, x, y, z, face_cubic, lookup, mode);
        let vertex_lights = face_vertex_lights(
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
        );
        emit_custom_quad_with_uvs(
            mesh,
            x,
            y,
            z,
            block_state_id,
            face_material,
            tint[face_index],
            atlas.rect(texture_indices[face_index]),
            face.normal,
            box_face_corners(face.face, min, max),
            face_uvs_from_crop(face_uvs[face_index], face_uv_rotations[face_index]),
            cardinal_shade(face_shade[face_index], face.face),
            vertex_lights,
            ambient_occlusion,
        );
    }
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
