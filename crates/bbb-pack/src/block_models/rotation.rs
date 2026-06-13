use super::{BlockFaceTextures, BlockModelBox, BlockModelFace, BlockModelShape};

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
    }
}

pub(super) fn rotate_model_shape(
    shape: BlockModelShape,
    x_degrees: i32,
    y_degrees: i32,
) -> BlockModelShape {
    match shape {
        BlockModelShape::Box(model_box) => {
            BlockModelShape::Box(rotate_model_box(model_box, x_degrees, y_degrees))
        }
        BlockModelShape::Boxes(model_boxes) => BlockModelShape::Boxes(
            model_boxes
                .into_iter()
                .map(|model_box| rotate_model_box(model_box, x_degrees, y_degrees))
                .collect(),
        ),
        BlockModelShape::Cube | BlockModelShape::Cross | BlockModelShape::Custom => shape,
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

fn rotate_model_box(model_box: BlockModelBox, x_degrees: i32, y_degrees: i32) -> BlockModelBox {
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
    let mut face_cull = [false; 6];
    let mut face_tint_indices = [None; 6];
    let mut face_textures: [Option<String>; 6] = std::array::from_fn(|_| None);
    for face in BlockModelFace::ALL {
        let target = face.rotate(x_degrees, y_degrees);
        face_present[target.index()] = model_box.face_present[face.index()];
        face_uvs[target.index()] = model_box.face_uvs[face.index()];
        face_cull[target.index()] = model_box.face_cull[face.index()];
        face_tint_indices[target.index()] = model_box.face_tint_indices[face.index()];
        face_textures[target.index()] = model_box.face_textures[face.index()].clone();
    }

    BlockModelBox {
        from: min,
        to: max,
        face_present,
        face_uvs,
        face_cull,
        face_tint_indices,
        face_textures,
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

fn rotate_y_quarter((x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
    (z, y, -x)
}
