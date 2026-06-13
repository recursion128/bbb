use super::super::{
    TerrainFace, TerrainLight, TerrainMaterialClass, TerrainMesh, TerrainTextureAtlas, TerrainTint,
    TerrainTransparency, TerrainUvRect, TerrainVertex,
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
) {
    let base = mesh.vertices.len() as u32;
    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    for (corner, uv) in face.corners.into_iter().zip(uvs) {
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
) {
    let base = mesh.vertices.len() as u32;
    let shade = if shade { 1.0 } else { 0.0 };
    let light = shader_light_with_emission(light, light_emission);
    for (corner, uv) in corners.into_iter().zip(uvs) {
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
