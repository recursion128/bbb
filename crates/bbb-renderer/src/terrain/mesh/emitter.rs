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
    light: TerrainLight,
    tint: TerrainTint,
    uv_rect: TerrainUvRect,
    face: FaceDef,
    ambient_occlusion: [f32; 4],
) {
    let base = mesh.vertices.len() as u32;
    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    for ((corner, uv), ambient_occlusion) in
        face.corners.into_iter().zip(uvs).zip(ambient_occlusion)
    {
        mesh.vertices.push(TerrainVertex {
            position: [
                x as f32 + corner[0],
                y as f32 + corner[1],
                z as f32 + corner[2],
            ],
            normal: face.normal,
            uv: uv_rect.map(uv),
            light: light.as_shader_light(),
            tint: tint.as_shader_tint(),
            shade: 1.0,
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
            light,
            tint[face.index()],
            atlas.rect(texture_indices[face.index()]),
            normal,
            corners,
            shade,
            light_emission,
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

        let ambient_occlusion = face_ambient_occlusion(
            ambient_occlusion && face_light_emission[face_index] == 0,
            face.face,
            x,
            y,
            z,
            box_face_is_cubic(face.face, min, max),
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
            light,
            tint[face_index],
            atlas.rect(texture_indices[face_index]),
            face.normal,
            box_face_corners(face.face, min, max),
            face_uvs_from_crop(face_uvs[face_index], face_uv_rotations[face_index]),
            face_shade[face_index],
            face_light_emission[face_index],
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

        let ambient_occlusion = quad
            .cull
            .filter(|_| ambient_occlusion && quad.light_emission == 0)
            .map(|face| face_ambient_occlusion(true, face, x, y, z, true, lookup, mode))
            .unwrap_or([1.0; 4]);
        emit_custom_quad_with_uvs(
            mesh,
            x,
            y,
            z,
            block_state_id,
            face_material,
            light,
            quad.tint,
            atlas.rect(quad.texture_index),
            quad.normal,
            quad.corners
                .map(|corner| [corner[0] / 16.0, corner[1] / 16.0, corner[2] / 16.0]),
            quad.uvs,
            quad.shade,
            quad.light_emission,
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
    light: TerrainLight,
    tint: TerrainTint,
    uv_rect: TerrainUvRect,
    normal: [f32; 3],
    corners: [[f32; 3]; 4],
    shade: bool,
    light_emission: u8,
) {
    emit_custom_quad_with_uvs(
        mesh,
        x,
        y,
        z,
        block_state_id,
        material,
        light,
        tint,
        uv_rect,
        normal,
        corners,
        [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
        shade,
        light_emission,
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
    light: TerrainLight,
    tint: TerrainTint,
    uv_rect: TerrainUvRect,
    normal: [f32; 3],
    corners: [[f32; 3]; 4],
    uvs: [[f32; 2]; 4],
    shade: bool,
    light_emission: u8,
    ambient_occlusion: [f32; 4],
) {
    let base = mesh.vertices.len() as u32;
    let shade = if shade { 1.0 } else { 0.0 };
    let light = shader_light_with_emission(light, light_emission);
    for ((corner, uv), ambient_occlusion) in corners.into_iter().zip(uvs).zip(ambient_occlusion) {
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
