use super::{BlockFaceTextures, BlockModelBox, BlockModelCross, BlockModelFace, BlockModelShape};

pub(super) fn apply_variant_rotation(
    local: BlockFaceTextures,
    x_degrees: i32,
    y_degrees: i32,
) -> BlockFaceTextures {
    let mut rotated: [Option<String>; 6] = std::array::from_fn(|_| None);
    for face in BlockModelFace::ALL {
        let target = face.rotate(x_degrees, y_degrees);
        rotated[target.index()] = Some(local.textures[face.index()].clone());
    }
    BlockFaceTextures {
        textures: std::array::from_fn(|index| {
            rotated[index]
                .clone()
                .unwrap_or_else(|| local.textures[index].clone())
        }),
        tint_indices: rotate_face_values(local.tint_indices, x_degrees, y_degrees),
        force_translucent: rotate_face_values(local.force_translucent, x_degrees, y_degrees),
    }
}

pub(super) fn rotate_model_shape(
    shape: BlockModelShape,
    x_degrees: i32,
    y_degrees: i32,
    uvlock: bool,
) -> BlockModelShape {
    match shape {
        BlockModelShape::Box(model_box) => {
            BlockModelShape::Box(rotate_model_box(model_box, x_degrees, y_degrees, uvlock))
        }
        BlockModelShape::Boxes(model_boxes) => BlockModelShape::Boxes(
            model_boxes
                .into_iter()
                .map(|model_box| rotate_model_box(model_box, x_degrees, y_degrees, uvlock))
                .collect(),
        ),
        BlockModelShape::Crosses(model_crosses) => BlockModelShape::Crosses(
            model_crosses
                .into_iter()
                .map(|model_cross| rotate_model_cross(model_cross, x_degrees, y_degrees))
                .collect(),
        ),
        BlockModelShape::Cube | BlockModelShape::Cross { .. } | BlockModelShape::Custom => shape,
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

    fn rotate(self, x_degrees: i32, y_degrees: i32) -> Self {
        let mut vector = self.vector();
        for _ in 0..quarter_turns(x_degrees) {
            vector = rotate_x_quarter(vector);
        }
        for _ in 0..quarter_turns(y_degrees) {
            vector = rotate_y_quarter(vector);
        }
        Self::from_vector(vector).unwrap_or(self)
    }
}

fn rotate_model_box(
    model_box: BlockModelBox,
    x_degrees: i32,
    y_degrees: i32,
    uvlock: bool,
) -> BlockModelBox {
    let mut min = [u8::MAX; 3];
    let mut max = [u8::MIN; 3];
    for x in [model_box.from[0], model_box.to[0]] {
        for y in [model_box.from[1], model_box.to[1]] {
            for z in [model_box.from[2], model_box.to[2]] {
                let rotated = rotate_model_point([x, y, z], x_degrees, y_degrees);
                for axis in 0..3 {
                    min[axis] = min[axis].min(rotated[axis]);
                    max[axis] = max[axis].max(rotated[axis]);
                }
            }
        }
    }

    let mut face_present = [false; 6];
    let mut face_uvs = [[0, 0, 16, 16]; 6];
    let mut face_uv_rotations = [0; 6];
    let mut face_shade = [true; 6];
    let mut face_light_emission = [0; 6];
    let mut face_cull = [None; 6];
    let mut face_tint_indices = [None; 6];
    let mut face_textures: [Option<String>; 6] = std::array::from_fn(|_| None);
    let mut face_force_translucent = [false; 6];
    for face in BlockModelFace::ALL {
        let target = face.rotate(x_degrees, y_degrees);
        face_present[target.index()] = model_box.face_present[face.index()];
        let (uv, uv_rotation) = if uvlock {
            uvlock_face_uvs(
                face,
                target,
                model_box.face_uvs[face.index()],
                model_box.face_uv_rotations[face.index()],
                x_degrees,
                y_degrees,
            )
        } else {
            (
                model_box.face_uvs[face.index()],
                model_box.face_uv_rotations[face.index()],
            )
        };
        face_uvs[target.index()] = uv;
        face_uv_rotations[target.index()] = uv_rotation;
        face_shade[target.index()] = model_box.face_shade[face.index()];
        face_light_emission[target.index()] = model_box.face_light_emission[face.index()];
        face_cull[target.index()] = model_box.face_cull[face.index()]
            .map(|cull_face| cull_face.rotate(x_degrees, y_degrees));
        face_tint_indices[target.index()] = model_box.face_tint_indices[face.index()];
        face_textures[target.index()] = model_box.face_textures[face.index()].clone();
        face_force_translucent[target.index()] = model_box.face_force_translucent[face.index()];
    }

    BlockModelBox {
        from: min,
        to: max,
        face_present,
        face_uvs,
        face_uv_rotations,
        face_shade,
        face_light_emission,
        face_cull,
        face_tint_indices,
        face_textures,
        face_force_translucent,
    }
}

fn rotate_model_cross(
    model_cross: BlockModelCross,
    x_degrees: i32,
    y_degrees: i32,
) -> BlockModelCross {
    let mut face_textures: [Option<String>; 6] = std::array::from_fn(|_| None);
    let mut face_tint_indices = [None; 6];
    let mut face_force_translucent = [false; 6];
    for face in BlockModelFace::ALL {
        let target = face.rotate(x_degrees, y_degrees);
        face_textures[target.index()] = model_cross.face_textures[face.index()].clone();
        face_tint_indices[target.index()] = model_cross.face_tint_indices[face.index()];
        face_force_translucent[target.index()] = model_cross.face_force_translucent[face.index()];
    }

    BlockModelCross {
        face_textures,
        face_tint_indices,
        face_force_translucent,
        shade: model_cross.shade,
        light_emission: model_cross.light_emission,
    }
}

fn rotate_face_values<T: Copy>(values: [T; 6], x_degrees: i32, y_degrees: i32) -> [T; 6] {
    let mut rotated = values;
    for face in BlockModelFace::ALL {
        let target = face.rotate(x_degrees, y_degrees);
        rotated[target.index()] = values[face.index()];
    }
    rotated
}

// Mirrors vanilla UV lock in the current box-face crop representation.
fn uvlock_face_uvs(
    source_face: BlockModelFace,
    target_face: BlockModelFace,
    uv: [u8; 4],
    uv_rotation: u8,
    x_degrees: i32,
    y_degrees: i32,
) -> ([u8; 4], u8) {
    let transformed = uv_corners(uv, uv_rotation).map(|[u, v]| {
        let vector = (i32::from(u) - 8, i32::from(v) - 8, 0);
        let vector = local_to_global(target_face, vector);
        let vector = inverse_model_rotation(vector, x_degrees, y_degrees);
        let vector = global_to_local(source_face, vector);
        [
            (vector.0 + 8).clamp(0, 16) as u8,
            (vector.1 + 8).clamp(0, 16) as u8,
        ]
    });
    let min_u = transformed
        .iter()
        .map(|corner| corner[0])
        .min()
        .unwrap_or(0);
    let min_v = transformed
        .iter()
        .map(|corner| corner[1])
        .min()
        .unwrap_or(0);
    let max_u = transformed
        .iter()
        .map(|corner| corner[0])
        .max()
        .unwrap_or(16);
    let max_v = transformed
        .iter()
        .map(|corner| corner[1])
        .max()
        .unwrap_or(16);
    let transformed_uv = [min_u, min_v, max_u, max_v];
    for rotation in 0..4 {
        if uv_corners(transformed_uv, rotation) == transformed {
            return (transformed_uv, rotation);
        }
    }
    (transformed_uv, uv_rotation)
}

fn uv_corners(uv: [u8; 4], rotation: u8) -> [[u8; 2]; 4] {
    let min_u = uv[0].min(uv[2]);
    let min_v = uv[1].min(uv[3]);
    let max_u = uv[0].max(uv[2]);
    let max_v = uv[1].max(uv[3]);
    let corners = [
        [min_u, min_v],
        [max_u, min_v],
        [max_u, max_v],
        [min_u, max_v],
    ];
    std::array::from_fn(|index| corners[(index + rotation as usize) % corners.len()])
}

fn local_to_global(face: BlockModelFace, vector: (i32, i32, i32)) -> (i32, i32, i32) {
    match face {
        BlockModelFace::South => vector,
        BlockModelFace::East => rotate_y_quarter(vector),
        BlockModelFace::West => rotate_y_quarter(rotate_y_quarter(rotate_y_quarter(vector))),
        BlockModelFace::North => rotate_y_quarter(rotate_y_quarter(vector)),
        BlockModelFace::Up => rotate_x_counter_quarter(vector),
        BlockModelFace::Down => rotate_x_quarter(vector),
    }
}

fn global_to_local(face: BlockModelFace, vector: (i32, i32, i32)) -> (i32, i32, i32) {
    match face {
        BlockModelFace::South => vector,
        BlockModelFace::East => rotate_y_counter_quarter(vector),
        BlockModelFace::West => rotate_y_quarter(vector),
        BlockModelFace::North => rotate_y_quarter(rotate_y_quarter(vector)),
        BlockModelFace::Up => rotate_x_quarter(vector),
        BlockModelFace::Down => rotate_x_counter_quarter(vector),
    }
}

fn inverse_model_rotation(
    mut vector: (i32, i32, i32),
    x_degrees: i32,
    y_degrees: i32,
) -> (i32, i32, i32) {
    for _ in 0..quarter_turns(y_degrees) {
        vector = rotate_y_counter_quarter(vector);
    }
    for _ in 0..quarter_turns(x_degrees) {
        vector = rotate_x_counter_quarter(vector);
    }
    vector
}

fn rotate_model_point(point: [u8; 3], x_degrees: i32, y_degrees: i32) -> [u8; 3] {
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
    [
        (vector.0 + 8).clamp(0, 16) as u8,
        (vector.1 + 8).clamp(0, 16) as u8,
        (vector.2 + 8).clamp(0, 16) as u8,
    ]
}

fn quarter_turns(degrees: i32) -> usize {
    degrees.rem_euclid(360) as usize / 90
}

fn rotate_x_quarter((x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
    (x, -z, y)
}

fn rotate_x_counter_quarter((x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
    (x, z, -y)
}

fn rotate_y_quarter((x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
    (z, y, -x)
}

fn rotate_y_counter_quarter((x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
    (-z, y, x)
}
