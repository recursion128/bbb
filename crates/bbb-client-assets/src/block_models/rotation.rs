use super::{
    shape::{box_to_quads, face_uvs_from_crop_f32, model_face_corners, recalculate_winding},
    BlockFaceTextures, BlockModelBox, BlockModelFace, BlockModelQuad, BlockModelShape,
};

pub(super) fn apply_variant_rotation(
    local: BlockFaceTextures,
    x_degrees: i32,
    y_degrees: i32,
    z_degrees: i32,
) -> BlockFaceTextures {
    let mut rotated: [Option<String>; 6] = std::array::from_fn(|_| None);
    for face in BlockModelFace::ALL {
        let target = face.rotate(x_degrees, y_degrees, z_degrees);
        rotated[target.index()] = Some(local.textures[face.index()].clone());
    }
    BlockFaceTextures {
        textures: std::array::from_fn(|index| {
            rotated[index]
                .clone()
                .unwrap_or_else(|| local.textures[index].clone())
        }),
        tint_indices: rotate_face_values(local.tint_indices, x_degrees, y_degrees, z_degrees),
        force_translucent: rotate_face_values(
            local.force_translucent,
            x_degrees,
            y_degrees,
            z_degrees,
        ),
    }
}

pub(super) fn rotate_model_shape(
    shape: BlockModelShape,
    x_degrees: i32,
    y_degrees: i32,
    z_degrees: i32,
    uvlock: bool,
) -> BlockModelShape {
    if (x_degrees, y_degrees, z_degrees) == (0, 0, 0) {
        return shape;
    }
    let rotate_quads = |quads: Vec<BlockModelQuad>| {
        BlockModelShape::Quads(
            quads
                .into_iter()
                .map(|quad| rotate_model_quad(quad, x_degrees, y_degrees, z_degrees, uvlock))
                .collect(),
        )
    };
    match shape {
        BlockModelShape::Box(model_box) => {
            match rotate_model_box(&model_box, x_degrees, y_degrees, z_degrees, uvlock) {
                Some(rotated) => BlockModelShape::Box(rotated),
                None => rotate_quads(box_to_quads(&model_box)),
            }
        }
        BlockModelShape::Boxes(model_boxes) => {
            match model_boxes
                .iter()
                .map(|model_box| {
                    rotate_model_box(model_box, x_degrees, y_degrees, z_degrees, uvlock)
                })
                .collect::<Option<Vec<_>>>()
            {
                Some(rotated) => BlockModelShape::Boxes(rotated),
                None => rotate_quads(model_boxes.iter().flat_map(box_to_quads).collect()),
            }
        }
        BlockModelShape::Quads(model_quads) => rotate_quads(model_quads),
        BlockModelShape::Cube | BlockModelShape::Cross { .. } | BlockModelShape::Crosses(_) => {
            unreachable!("fast-path shapes are chosen after rotating")
        }
    }
}

impl BlockModelFace {
    fn from_vector(vector: (i32, i32, i32)) -> Option<Self> {
        match vector {
            (0, -1, 0) => Some(Self::Down),
            (0, 1, 0) => Some(Self::Up),
            (0, 0, -1) => Some(Self::North),
            (0, 0, 1) => Some(Self::South),
            (-1, 0, 0) => Some(Self::West),
            (1, 0, 0) => Some(Self::East),
            _ => None,
        }
    }

    fn vector(self) -> (i32, i32, i32) {
        match self {
            Self::Down => (0, -1, 0),
            Self::Up => (0, 1, 0),
            Self::North => (0, 0, -1),
            Self::South => (0, 0, 1),
            Self::West => (-1, 0, 0),
            Self::East => (1, 0, 0),
        }
    }

    fn rotate(self, x_degrees: i32, y_degrees: i32, z_degrees: i32) -> Self {
        let mut vector = self.vector();
        for _ in 0..quarter_turns(x_degrees) {
            vector = rotate_x_quarter(vector);
        }
        for _ in 0..quarter_turns(y_degrees) {
            vector = rotate_y_quarter(vector);
        }
        for _ in 0..quarter_turns(z_degrees) {
            vector = rotate_z_quarter(vector);
        }
        Self::from_vector(vector).unwrap_or(self)
    }
}

/// Rotates each face as the quad `FaceBakery` bakes, then reads it back as a crop and quarter
/// turn on the rotated box; `None` when a face's UVs leave the integer crop grid.
fn rotate_model_box(
    model_box: &BlockModelBox,
    x_degrees: i32,
    y_degrees: i32,
    z_degrees: i32,
    uvlock: bool,
) -> Option<BlockModelBox> {
    let mut min = [u8::MAX; 3];
    let mut max = [u8::MIN; 3];
    for x in [model_box.from[0], model_box.to[0]] {
        for y in [model_box.from[1], model_box.to[1]] {
            for z in [model_box.from[2], model_box.to[2]] {
                let rotated = rotate_model_point([x, y, z], x_degrees, y_degrees, z_degrees);
                for axis in 0..3 {
                    min[axis] = min[axis].min(rotated[axis]);
                    max[axis] = max[axis].max(rotated[axis]);
                }
            }
        }
    }

    let mut rotated = BlockModelBox {
        from: min,
        to: max,
        face_present: [false; 6],
        face_uvs: [[0, 0, 16, 16]; 6],
        face_uv_rotations: [0; 6],
        face_shade: [true; 6],
        face_light_emission: [0; 6],
        face_cull: [None; 6],
        face_tint_indices: [None; 6],
        face_textures: std::array::from_fn(|_| None),
        face_force_translucent: [false; 6],
    };
    for quad in box_to_quads(model_box) {
        let quad = rotate_model_quad(quad, x_degrees, y_degrees, z_degrees, uvlock);
        let index = quad.face.index();
        let corners = model_face_corners(quad.face, min.map(f32::from), max.map(f32::from));
        let uvs = corners.map(|corner| {
            quad.corners
                .iter()
                .position(|rotated| *rotated == corner)
                .map(|position| quad.uvs[position])
        });
        let (uv, uv_rotation) = face_crop_from_uvs(uvs)?;
        rotated.face_present[index] = true;
        rotated.face_uvs[index] = uv;
        rotated.face_uv_rotations[index] = uv_rotation;
        rotated.face_shade[index] = quad.shade;
        rotated.face_light_emission[index] = quad.light_emission;
        rotated.face_cull[index] = quad.cull;
        rotated.face_tint_indices[index] = quad.tint_index;
        rotated.face_textures[index] = quad.texture;
        rotated.face_force_translucent[index] = quad.force_translucent;
    }
    Some(rotated)
}

/// The integer crop and quarter turn for which `face_uvs_from_crop_f32` yields `uvs`.
fn face_crop_from_uvs(uvs: [Option<[f32; 2]>; 4]) -> Option<([u8; 4], u8)> {
    let uvs = [uvs[0]?, uvs[1]?, uvs[2]?, uvs[3]?];
    let texel = |value: f32| {
        let texel = (value * 16.0).round();
        ((value * 16.0 - texel).abs() < 1e-3 && (0.0..=16.0).contains(&texel))
            .then_some(texel as u8)
    };
    let matches = |crop: [u8; 4], rotation: u8| {
        face_uvs_from_crop_f32(crop.map(f32::from), rotation)
            .iter()
            .zip(&uvs)
            .all(|(a, b)| (a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4)
    };
    // The crop's (u0, v0) and (u1, v1) sit on two of the corners.
    (0..4).find_map(|rotation| {
        (0..16).find_map(|pair| {
            let (first, second) = (uvs[pair / 4], uvs[pair % 4]);
            let crop = [
                texel(first[0])?,
                texel(first[1])?,
                texel(second[0])?,
                texel(second[1])?,
            ];
            matches(crop, rotation).then_some((crop, rotation))
        })
    })
}

fn rotate_model_quad(
    model_quad: BlockModelQuad,
    x_degrees: i32,
    y_degrees: i32,
    z_degrees: i32,
    uvlock: bool,
) -> BlockModelQuad {
    let source_face = model_quad.face;
    let target_face = source_face.rotate(x_degrees, y_degrees, z_degrees);
    let mut quad = BlockModelQuad {
        face: target_face,
        corners: model_quad
            .corners
            .map(|corner| rotate_model_point_f32(corner, x_degrees, y_degrees, z_degrees)),
        normal: normalize3(rotate_model_vector_f32(
            model_quad.normal,
            x_degrees,
            y_degrees,
            z_degrees,
        )),
        uvs: if uvlock {
            uvlock_quad_uvs(
                source_face,
                target_face,
                model_quad.uvs,
                x_degrees,
                y_degrees,
                z_degrees,
            )
        } else {
            model_quad.uvs
        },
        cull: model_quad
            .cull
            .map(|cull_face| cull_face.rotate(x_degrees, y_degrees, z_degrees)),
        tint_index: model_quad.tint_index,
        texture: model_quad.texture,
        force_translucent: model_quad.force_translucent,
        shade: model_quad.shade,
        light_emission: model_quad.light_emission,
        element_rotated: model_quad.element_rotated,
    };
    if !quad.element_rotated {
        recalculate_winding(&mut quad);
    }
    quad
}

fn rotate_face_values<T: Copy>(
    values: [T; 6],
    x_degrees: i32,
    y_degrees: i32,
    z_degrees: i32,
) -> [T; 6] {
    let mut rotated = values;
    for face in BlockModelFace::ALL {
        let target = face.rotate(x_degrees, y_degrees, z_degrees);
        rotated[target.index()] = values[face.index()];
    }
    rotated
}

fn uvlock_quad_uvs(
    source_face: BlockModelFace,
    target_face: BlockModelFace,
    uvs: [[f32; 2]; 4],
    x_degrees: i32,
    y_degrees: i32,
    z_degrees: i32,
) -> [[f32; 2]; 4] {
    uvs.map(|[u, v]| {
        let [u, v] = uvlock_uv_point_16(
            source_face,
            target_face,
            [u * 16.0, v * 16.0],
            x_degrees,
            y_degrees,
            z_degrees,
        );
        [u / 16.0, v / 16.0]
    })
}

fn uvlock_uv_point_16(
    source_face: BlockModelFace,
    target_face: BlockModelFace,
    uv: [f32; 2],
    x_degrees: i32,
    y_degrees: i32,
    z_degrees: i32,
) -> [f32; 2] {
    let vector = (uv[0] - 8.0, uv[1] - 8.0, 0.0);
    let vector = local_to_global_f32(target_face, vector);
    let vector = inverse_model_rotation_f32(vector, x_degrees, y_degrees, z_degrees);
    let vector = global_to_local_f32(source_face, vector);
    [
        (vector.0 + 8.0).clamp(0.0, 16.0),
        (vector.1 + 8.0).clamp(0.0, 16.0),
    ]
}

fn local_to_global_f32(face: BlockModelFace, vector: (f32, f32, f32)) -> (f32, f32, f32) {
    match face {
        BlockModelFace::South => vector,
        BlockModelFace::East => rotate_y_quarter_f32(vector),
        BlockModelFace::West => {
            rotate_y_quarter_f32(rotate_y_quarter_f32(rotate_y_quarter_f32(vector)))
        }
        BlockModelFace::North => rotate_y_quarter_f32(rotate_y_quarter_f32(vector)),
        BlockModelFace::Up => rotate_x_counter_quarter_f32(vector),
        BlockModelFace::Down => rotate_x_quarter_f32(vector),
    }
}

fn global_to_local_f32(face: BlockModelFace, vector: (f32, f32, f32)) -> (f32, f32, f32) {
    match face {
        BlockModelFace::South => vector,
        BlockModelFace::East => rotate_y_counter_quarter_f32(vector),
        BlockModelFace::West => rotate_y_quarter_f32(vector),
        BlockModelFace::North => rotate_y_quarter_f32(rotate_y_quarter_f32(vector)),
        BlockModelFace::Up => rotate_x_quarter_f32(vector),
        BlockModelFace::Down => rotate_x_counter_quarter_f32(vector),
    }
}

fn inverse_model_rotation_f32(
    mut vector: (f32, f32, f32),
    x_degrees: i32,
    y_degrees: i32,
    z_degrees: i32,
) -> (f32, f32, f32) {
    for _ in 0..quarter_turns(z_degrees) {
        vector = rotate_z_counter_quarter_f32(vector);
    }
    for _ in 0..quarter_turns(y_degrees) {
        vector = rotate_y_counter_quarter_f32(vector);
    }
    for _ in 0..quarter_turns(x_degrees) {
        vector = rotate_x_counter_quarter_f32(vector);
    }
    vector
}

fn rotate_model_point(point: [u8; 3], x_degrees: i32, y_degrees: i32, z_degrees: i32) -> [u8; 3] {
    let mut vector = (
        point[0] as i32 - 8,
        point[1] as i32 - 8,
        point[2] as i32 - 8,
    );
    for _ in 0..quarter_turns(x_degrees) {
        vector = rotate_x_quarter(vector);
    }
    for _ in 0..quarter_turns(y_degrees) {
        vector = rotate_y_quarter(vector);
    }
    for _ in 0..quarter_turns(z_degrees) {
        vector = rotate_z_quarter(vector);
    }
    [
        (vector.0 + 8).clamp(0, 16) as u8,
        (vector.1 + 8).clamp(0, 16) as u8,
        (vector.2 + 8).clamp(0, 16) as u8,
    ]
}

fn rotate_model_point_f32(
    point: [f32; 3],
    x_degrees: i32,
    y_degrees: i32,
    z_degrees: i32,
) -> [f32; 3] {
    let vector = rotate_model_vector_f32(
        [point[0] - 8.0, point[1] - 8.0, point[2] - 8.0],
        x_degrees,
        y_degrees,
        z_degrees,
    );
    [vector[0] + 8.0, vector[1] + 8.0, vector[2] + 8.0]
}

fn rotate_model_vector_f32(
    vector: [f32; 3],
    x_degrees: i32,
    y_degrees: i32,
    z_degrees: i32,
) -> [f32; 3] {
    let mut vector = (vector[0], vector[1], vector[2]);
    for _ in 0..quarter_turns(x_degrees) {
        vector = rotate_x_quarter_f32(vector);
    }
    for _ in 0..quarter_turns(y_degrees) {
        vector = rotate_y_quarter_f32(vector);
    }
    for _ in 0..quarter_turns(z_degrees) {
        vector = rotate_z_quarter_f32(vector);
    }
    [vector.0, vector.1, vector.2]
}

fn normalize3(vector: [f32; 3]) -> [f32; 3] {
    let length = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if length <= f32::EPSILON {
        [0.0, 1.0, 0.0]
    } else {
        [vector[0] / length, vector[1] / length, vector[2] / length]
    }
}

fn quarter_turns(degrees: i32) -> usize {
    degrees.rem_euclid(360) as usize / 90
}

fn rotate_x_quarter((x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
    (x, -z, y)
}

fn rotate_y_quarter((x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
    (z, y, -x)
}

fn rotate_z_quarter((x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
    (-y, x, z)
}

fn rotate_x_quarter_f32((x, y, z): (f32, f32, f32)) -> (f32, f32, f32) {
    (x, -z, y)
}

fn rotate_x_counter_quarter_f32((x, y, z): (f32, f32, f32)) -> (f32, f32, f32) {
    (x, z, -y)
}

fn rotate_y_quarter_f32((x, y, z): (f32, f32, f32)) -> (f32, f32, f32) {
    (z, y, -x)
}

fn rotate_y_counter_quarter_f32((x, y, z): (f32, f32, f32)) -> (f32, f32, f32) {
    (-z, y, x)
}

fn rotate_z_quarter_f32((x, y, z): (f32, f32, f32)) -> (f32, f32, f32) {
    (-y, x, z)
}

fn rotate_z_counter_quarter_f32((x, y, z): (f32, f32, f32)) -> (f32, f32, f32) {
    (y, -x, z)
}
