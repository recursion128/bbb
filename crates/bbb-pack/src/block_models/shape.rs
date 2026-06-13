use std::collections::BTreeMap;

use super::{
    resolve_texture_alias, BlockModelBox, BlockModelCross, BlockModelFace, BlockModelQuad,
    BlockModelShape, RawBlockElement, RawBlockElementRotation, ResolvedTextureReference,
};

pub(super) fn classify_model_shape(
    elements: &[RawBlockElement],
    textures: &BTreeMap<String, ResolvedTextureReference>,
) -> BlockModelShape {
    let has_element_rotation = elements.iter().any(|element| element.rotation.is_some());
    let has_box_metadata_transforms = elements.iter().any(has_box_metadata_transform);
    let mut face_counts = [0usize; 6];
    let mut total_faces = 0usize;
    for element in elements {
        for face_name in element.faces.keys() {
            let Some(face) = BlockModelFace::from_name(face_name) else {
                continue;
            };
            face_counts[face.index()] += 1;
            total_faces += 1;
        }
    }

    let has_cross_faces = total_faces == 4
        && face_counts[BlockModelFace::North.index()] == 1
        && face_counts[BlockModelFace::South.index()] == 1
        && face_counts[BlockModelFace::West.index()] == 1
        && face_counts[BlockModelFace::East.index()] == 1
        && face_counts[BlockModelFace::Down.index()] == 0
        && face_counts[BlockModelFace::Up.index()] == 0;

    if has_element_rotation {
        if has_cross_faces {
            if let Some(shape) = single_cross_shape(elements) {
                return shape;
            }
        }
        if let Some(model_crosses) = multi_cross_shape(elements, textures) {
            return BlockModelShape::Crosses(model_crosses);
        }
        return model_quads_shape(elements, textures)
            .map(BlockModelShape::Quads)
            .unwrap_or(BlockModelShape::Custom);
    }

    if elements.len() > 1 {
        if let Some(model_boxes) = multi_box_shape(elements, textures) {
            return BlockModelShape::Boxes(model_boxes);
        }
    }

    if elements.iter().any(is_full_cube_element) {
        return BlockModelShape::Cube;
    }

    if has_cross_faces {
        if let Some(shape) = single_cross_shape(elements) {
            return shape;
        }
    }

    if let Some(model_box) = single_box_shape(elements, textures) {
        return BlockModelShape::Box(model_box);
    }
    if let Some(model_boxes) = multi_box_shape(elements, textures) {
        return BlockModelShape::Boxes(model_boxes);
    }

    let has_cube_faces = !has_box_metadata_transforms && face_counts.iter().all(|count| *count > 0);
    if has_cube_faces {
        return BlockModelShape::Cube;
    }

    if let Some(model_quads) = model_quads_shape(elements, textures) {
        return BlockModelShape::Quads(model_quads);
    }

    BlockModelShape::Custom
}

pub(super) fn combine_model_shapes(shapes: Vec<BlockModelShape>) -> BlockModelShape {
    let mut shapes = shapes.into_iter();
    let Some(first) = shapes.next() else {
        return BlockModelShape::Custom;
    };
    let Some(second) = shapes.next() else {
        return first;
    };

    let mut boxes = Vec::new();
    let mut quads = Vec::new();
    let mut has_quads = false;
    for shape in std::iter::once(first)
        .chain(std::iter::once(second))
        .chain(shapes)
    {
        match shape {
            BlockModelShape::Box(model_box) => boxes.push(model_box),
            BlockModelShape::Boxes(model_boxes) => boxes.extend(model_boxes),
            BlockModelShape::Quads(model_quads) => {
                has_quads = true;
                quads.extend(model_quads);
            }
            BlockModelShape::Cube => return BlockModelShape::Cube,
            BlockModelShape::Cross { .. }
            | BlockModelShape::Crosses(_)
            | BlockModelShape::Custom => {
                return BlockModelShape::Custom;
            }
        }
    }

    if has_quads {
        for model_box in boxes {
            quads.extend(box_to_quads(&model_box));
        }
        return BlockModelShape::Quads(quads);
    }

    match boxes.len() {
        0 => BlockModelShape::Custom,
        1 => BlockModelShape::Box(boxes[0].clone()),
        _ => BlockModelShape::Boxes(boxes),
    }
}

fn is_full_cube_element(element: &RawBlockElement) -> bool {
    if has_box_metadata_transform(element) {
        return false;
    }
    let Some(from) = element.from.and_then(quantize_vec3_0_16) else {
        return false;
    };
    let Some(to) = element.to.and_then(quantize_vec3_0_16) else {
        return false;
    };
    from == [0, 0, 0]
        && to == [16, 16, 16]
        && BlockModelFace::ALL
            .iter()
            .all(|face| element.faces.contains_key(face.name()))
}

fn single_box_shape(
    elements: &[RawBlockElement],
    textures: &BTreeMap<String, ResolvedTextureReference>,
) -> Option<BlockModelBox> {
    let [element] = elements else {
        return None;
    };
    element_box_shape(element, textures)
}

fn multi_box_shape(
    elements: &[RawBlockElement],
    textures: &BTreeMap<String, ResolvedTextureReference>,
) -> Option<Vec<BlockModelBox>> {
    if elements.len() <= 1 {
        return None;
    }
    elements
        .iter()
        .map(|element| element_box_shape(element, textures))
        .collect()
}

fn element_box_shape(
    element: &RawBlockElement,
    textures: &BTreeMap<String, ResolvedTextureReference>,
) -> Option<BlockModelBox> {
    let from = element.from.and_then(quantize_vec3_0_16)?;
    let to = element.to.and_then(quantize_vec3_0_16)?;
    if from[0] >= to[0] || from[1] >= to[1] || from[2] >= to[2] {
        return None;
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
    let element_shade = element_shade(element);
    let element_light_emission = element_light_emission(element)?;
    for (face_name, raw_face) in &element.faces {
        let face = BlockModelFace::from_name(face_name)?;
        let index = face.index();
        face_present[face.index()] = true;
        face_uvs[index] = raw_face
            .uv
            .and_then(quantize_uv_0_16)
            .unwrap_or_else(|| default_face_uv(from, to, face));
        face_uv_rotations[index] = raw_face
            .rotation
            .map(quantize_face_uv_rotation)
            .unwrap_or(Some(0))?;
        face_shade[index] = element_shade;
        face_light_emission[index] = element_light_emission;
        face_cull[index] = raw_face
            .cullface
            .as_deref()
            .and_then(BlockModelFace::from_name);
        face_tint_indices[index] = raw_face.tintindex;
        let resolved_texture = resolve_texture_alias(textures, &raw_face.texture);
        face_force_translucent[index] = resolved_texture
            .as_ref()
            .map(|texture| texture.force_translucent)
            .unwrap_or(false);
        face_textures[index] = resolved_texture.map(|texture| texture.id);
    }

    Some(BlockModelBox {
        from,
        to,
        face_present,
        face_uvs,
        face_uv_rotations,
        face_shade,
        face_light_emission,
        face_cull,
        face_tint_indices,
        face_textures,
        face_force_translucent,
    })
}

fn model_quads_shape(
    elements: &[RawBlockElement],
    textures: &BTreeMap<String, ResolvedTextureReference>,
) -> Option<Vec<BlockModelQuad>> {
    if elements.is_empty() {
        return None;
    }
    let quads = elements
        .iter()
        .map(|element| element_quads(element, textures))
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    (!quads.is_empty()).then_some(quads)
}

fn element_quads(
    element: &RawBlockElement,
    textures: &BTreeMap<String, ResolvedTextureReference>,
) -> Option<Vec<BlockModelQuad>> {
    let from = element.from?;
    let to = element.to?;
    if from[0] >= to[0] || from[1] >= to[1] || from[2] >= to[2] {
        return None;
    }

    let element_shade = element_shade(element);
    let element_light_emission = element_light_emission(element)?;
    let mut quads = Vec::with_capacity(element.faces.len());
    for (face_name, raw_face) in &element.faces {
        let face = BlockModelFace::from_name(face_name)?;
        let uv = raw_face
            .uv
            .unwrap_or_else(|| default_face_uv_f32(from, to, face));
        let uv_rotation = raw_face
            .rotation
            .map(quantize_face_uv_rotation)
            .unwrap_or(Some(0))?;
        let resolved_texture = resolve_texture_alias(textures, &raw_face.texture);
        let mut corners = model_face_corners(face, from, to);
        if let Some(rotation) = &element.rotation {
            for corner in &mut corners {
                *corner = rotate_element_point(*corner, rotation)?;
            }
        }
        let normal = if let Some(rotation) = &element.rotation {
            rotate_element_normal(face_normal(face), rotation)?
        } else {
            face_normal(face)
        };
        let force_translucent = resolved_texture
            .as_ref()
            .map(|texture| texture.force_translucent)
            .unwrap_or(false);
        quads.push(BlockModelQuad {
            face,
            corners,
            normal,
            uvs: face_uvs_from_crop_f32(uv, uv_rotation),
            cull: raw_face
                .cullface
                .as_deref()
                .and_then(BlockModelFace::from_name),
            tint_index: raw_face.tintindex,
            texture: resolved_texture.map(|texture| texture.id),
            force_translucent,
            shade: element_shade,
            light_emission: element_light_emission,
        });
    }
    Some(quads)
}

fn box_to_quads(model_box: &BlockModelBox) -> Vec<BlockModelQuad> {
    let from = [
        model_box.from[0] as f32,
        model_box.from[1] as f32,
        model_box.from[2] as f32,
    ];
    let to = [
        model_box.to[0] as f32,
        model_box.to[1] as f32,
        model_box.to[2] as f32,
    ];
    let mut quads = Vec::new();
    for face in BlockModelFace::ALL {
        let index = face.index();
        if !model_box.face_present[index] {
            continue;
        }
        let corners = model_face_corners(face, from, to);
        quads.push(BlockModelQuad {
            face,
            corners,
            normal: face_normal(face),
            uvs: face_uvs_from_crop(
                model_box.face_uvs[index],
                model_box.face_uv_rotations[index],
            ),
            cull: model_box.face_cull[index],
            tint_index: model_box.face_tint_indices[index],
            texture: model_box.face_textures[index].clone(),
            force_translucent: model_box.face_force_translucent[index],
            shade: model_box.face_shade[index],
            light_emission: model_box.face_light_emission[index],
        });
    }
    quads
}

fn quantize_vec3_0_16(values: [f32; 3]) -> Option<[u8; 3]> {
    Some([
        quantize_0_16(values[0])?,
        quantize_0_16(values[1])?,
        quantize_0_16(values[2])?,
    ])
}

fn quantize_uv_0_16(values: [f32; 4]) -> Option<[u8; 4]> {
    Some([
        quantize_0_16(values[0])?,
        quantize_0_16(values[1])?,
        quantize_0_16(values[2])?,
        quantize_0_16(values[3])?,
    ])
}

fn face_uvs_from_crop(uv: [u8; 4], rotation: u8) -> [[f32; 2]; 4] {
    face_uvs_from_crop_f32(
        [uv[0] as f32, uv[1] as f32, uv[2] as f32, uv[3] as f32],
        rotation,
    )
}

fn face_uvs_from_crop_f32(uv: [f32; 4], rotation: u8) -> [[f32; 2]; 4] {
    let min_u = uv[0].min(uv[2]) / 16.0;
    let min_v = uv[1].min(uv[3]) / 16.0;
    let max_u = uv[0].max(uv[2]) / 16.0;
    let max_v = uv[1].max(uv[3]) / 16.0;
    let uvs = [
        [min_u, min_v],
        [max_u, min_v],
        [max_u, max_v],
        [min_u, max_v],
    ];
    std::array::from_fn(|index| uvs[(index + rotation as usize) % uvs.len()])
}

fn default_face_uv(from: [u8; 3], to: [u8; 3], face: BlockModelFace) -> [u8; 4] {
    match face {
        BlockModelFace::Down => [from[0], 16 - to[2], to[0], 16 - from[2]],
        BlockModelFace::Up => [from[0], from[2], to[0], to[2]],
        BlockModelFace::North => [16 - to[0], 16 - to[1], 16 - from[0], 16 - from[1]],
        BlockModelFace::South => [from[0], 16 - to[1], to[0], 16 - from[1]],
        BlockModelFace::West => [from[2], 16 - to[1], to[2], 16 - from[1]],
        BlockModelFace::East => [16 - to[2], 16 - to[1], 16 - from[2], 16 - from[1]],
    }
}

fn default_face_uv_f32(from: [f32; 3], to: [f32; 3], face: BlockModelFace) -> [f32; 4] {
    match face {
        BlockModelFace::Down => [from[0], 16.0 - to[2], to[0], 16.0 - from[2]],
        BlockModelFace::Up => [from[0], from[2], to[0], to[2]],
        BlockModelFace::North => [16.0 - to[0], 16.0 - to[1], 16.0 - from[0], 16.0 - from[1]],
        BlockModelFace::South => [from[0], 16.0 - to[1], to[0], 16.0 - from[1]],
        BlockModelFace::West => [from[2], 16.0 - to[1], to[2], 16.0 - from[1]],
        BlockModelFace::East => [16.0 - to[2], 16.0 - to[1], 16.0 - from[2], 16.0 - from[1]],
    }
}

fn model_face_corners(face: BlockModelFace, from: [f32; 3], to: [f32; 3]) -> [[f32; 3]; 4] {
    match face {
        BlockModelFace::Down => [
            [from[0], from[1], to[2]],
            [to[0], from[1], to[2]],
            [to[0], from[1], from[2]],
            [from[0], from[1], from[2]],
        ],
        BlockModelFace::Up => [
            [from[0], to[1], from[2]],
            [to[0], to[1], from[2]],
            [to[0], to[1], to[2]],
            [from[0], to[1], to[2]],
        ],
        BlockModelFace::North => [
            [to[0], from[1], from[2]],
            [to[0], to[1], from[2]],
            [from[0], to[1], from[2]],
            [from[0], from[1], from[2]],
        ],
        BlockModelFace::South => [
            [from[0], from[1], to[2]],
            [from[0], to[1], to[2]],
            [to[0], to[1], to[2]],
            [to[0], from[1], to[2]],
        ],
        BlockModelFace::West => [
            [from[0], from[1], from[2]],
            [from[0], to[1], from[2]],
            [from[0], to[1], to[2]],
            [from[0], from[1], to[2]],
        ],
        BlockModelFace::East => [
            [to[0], from[1], to[2]],
            [to[0], to[1], to[2]],
            [to[0], to[1], from[2]],
            [to[0], from[1], from[2]],
        ],
    }
}

fn face_normal(face: BlockModelFace) -> [f32; 3] {
    match face {
        BlockModelFace::Down => [0.0, -1.0, 0.0],
        BlockModelFace::Up => [0.0, 1.0, 0.0],
        BlockModelFace::North => [0.0, 0.0, -1.0],
        BlockModelFace::South => [0.0, 0.0, 1.0],
        BlockModelFace::West => [-1.0, 0.0, 0.0],
        BlockModelFace::East => [1.0, 0.0, 0.0],
    }
}

fn rotate_element_point(point: [f32; 3], rotation: &RawBlockElementRotation) -> Option<[f32; 3]> {
    let transform = element_rotation_transform(rotation)?;
    let mut vector = sub3(point, rotation.origin);
    if rotation.rescale {
        let scale = rotation_rescale(transform);
        vector = [
            vector[0] * scale[0],
            vector[1] * scale[1],
            vector[2] * scale[2],
        ];
    }
    Some(add3(transform_vector(transform, vector), rotation.origin))
}

fn rotate_element_normal(normal: [f32; 3], rotation: &RawBlockElementRotation) -> Option<[f32; 3]> {
    Some(normalize3(transform_vector(
        element_rotation_transform(rotation)?,
        normal,
    )))
}

fn element_rotation_transform(rotation: &RawBlockElementRotation) -> Option<[[f32; 3]; 3]> {
    if rotation.axis.is_some() || rotation.angle.is_some() {
        return single_axis_rotation(rotation.axis.as_deref()?, rotation.angle?);
    }
    if rotation.x.is_none() && rotation.y.is_none() && rotation.z.is_none() {
        return None;
    }
    let x = rotation.x.unwrap_or(0.0);
    let y = rotation.y.unwrap_or(0.0);
    let z = rotation.z.unwrap_or(0.0);
    Some(euler_xyz_rotation(x, y, z))
}

fn single_axis_rotation(axis: &str, angle: f32) -> Option<[[f32; 3]; 3]> {
    let radians = angle.to_radians();
    let (sin, cos) = radians.sin_cos();
    match axis.to_ascii_lowercase().as_str() {
        "x" => Some([[1.0, 0.0, 0.0], [0.0, cos, -sin], [0.0, sin, cos]]),
        "y" => Some([[cos, 0.0, sin], [0.0, 1.0, 0.0], [-sin, 0.0, cos]]),
        "z" => Some([[cos, -sin, 0.0], [sin, cos, 0.0], [0.0, 0.0, 1.0]]),
        _ => None,
    }
}

fn euler_xyz_rotation(x: f32, y: f32, z: f32) -> [[f32; 3]; 3] {
    multiply_rotation(
        multiply_rotation(
            single_axis_rotation("z", z).expect("z axis is supported"),
            single_axis_rotation("y", y).expect("y axis is supported"),
        ),
        single_axis_rotation("x", x).expect("x axis is supported"),
    )
}

fn multiply_rotation(left: [[f32; 3]; 3], right: [[f32; 3]; 3]) -> [[f32; 3]; 3] {
    std::array::from_fn(|row| {
        std::array::from_fn(|col| {
            left[row][0] * right[0][col]
                + left[row][1] * right[1][col]
                + left[row][2] * right[2][col]
        })
    })
}

fn rotation_rescale(rotation: [[f32; 3]; 3]) -> [f32; 3] {
    std::array::from_fn(|axis| {
        let transformed = transform_vector(
            rotation,
            [
                (axis == 0) as u8 as f32,
                (axis == 1) as u8 as f32,
                (axis == 2) as u8 as f32,
            ],
        );
        let max_component = transformed
            .into_iter()
            .map(f32::abs)
            .fold(0.0_f32, f32::max);
        if max_component <= f32::EPSILON {
            1.0
        } else {
            1.0 / max_component
        }
    })
}

fn transform_vector(matrix: [[f32; 3]; 3], vector: [f32; 3]) -> [f32; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn normalize3(vector: [f32; 3]) -> [f32; 3] {
    let length = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if length <= f32::EPSILON {
        [0.0, 1.0, 0.0]
    } else {
        [vector[0] / length, vector[1] / length, vector[2] / length]
    }
}

fn quantize_face_uv_rotation(degrees: i32) -> Option<u8> {
    match degrees.rem_euclid(360) {
        0 => Some(0),
        90 => Some(1),
        180 => Some(2),
        270 => Some(3),
        _ => None,
    }
}

fn quantize_light_emission(value: i32) -> Option<u8> {
    if (0..=15).contains(&value) {
        Some(value as u8)
    } else {
        None
    }
}

fn has_box_metadata_transform(element: &RawBlockElement) -> bool {
    !element_shade(element)
        || element.light_emission.unwrap_or(0) != 0
        || has_face_uv_transform(element)
}

fn single_cross_shape(elements: &[RawBlockElement]) -> Option<BlockModelShape> {
    Some(BlockModelShape::Cross {
        shade: cross_shade(elements),
        light_emission: cross_light_emission(elements)?,
    })
}

fn multi_cross_shape(
    elements: &[RawBlockElement],
    textures: &BTreeMap<String, ResolvedTextureReference>,
) -> Option<Vec<BlockModelCross>> {
    if elements.len() <= 2 {
        return None;
    }

    let mut crosses = Vec::new();
    let mut current = empty_cross();
    for element in elements {
        let element_light_emission = element_light_emission(element)?;
        let mut element_faces = [false; 6];
        let mut element_face_count = 0;
        for (face_name, raw_face) in &element.faces {
            let face = BlockModelFace::from_name(face_name)?;
            if matches!(face, BlockModelFace::Down | BlockModelFace::Up) {
                return None;
            }
            let index = face.index();
            if current.face_textures[index].is_some() {
                return None;
            }
            element_faces[index] = true;
            element_face_count += 1;
            let resolved_texture = resolve_texture_alias(textures, &raw_face.texture);
            current.face_force_translucent[index] = resolved_texture
                .as_ref()
                .map(|texture| texture.force_translucent)
                .unwrap_or(false);
            current.face_textures[index] = resolved_texture.map(|texture| texture.id);
            current.face_tint_indices[index] = raw_face.tintindex;
        }

        let has_north_south = element_faces[BlockModelFace::North.index()]
            && element_faces[BlockModelFace::South.index()];
        let has_west_east = element_faces[BlockModelFace::West.index()]
            && element_faces[BlockModelFace::East.index()];
        if element_face_count != 2 || !(has_north_south || has_west_east) {
            return None;
        }

        current.shade &= element_shade(element);
        current.light_emission = current.light_emission.max(element_light_emission);
        if is_complete_cross(&current) {
            crosses.push(current);
            current = empty_cross();
        }
    }

    if has_any_cross_face(&current) || crosses.len() <= 1 {
        return None;
    }
    Some(crosses)
}

fn empty_cross() -> BlockModelCross {
    BlockModelCross {
        face_textures: std::array::from_fn(|_| None),
        face_tint_indices: [None; 6],
        face_force_translucent: [false; 6],
        shade: true,
        light_emission: 0,
    }
}

fn is_complete_cross(cross: &BlockModelCross) -> bool {
    cross.face_textures[BlockModelFace::North.index()].is_some()
        && cross.face_textures[BlockModelFace::South.index()].is_some()
        && cross.face_textures[BlockModelFace::West.index()].is_some()
        && cross.face_textures[BlockModelFace::East.index()].is_some()
}

fn has_any_cross_face(cross: &BlockModelCross) -> bool {
    cross.face_textures.iter().any(Option::is_some)
}

fn cross_shade(elements: &[RawBlockElement]) -> bool {
    elements.iter().all(element_shade)
}

fn cross_light_emission(elements: &[RawBlockElement]) -> Option<u8> {
    let mut light_emission = 0;
    for element in elements {
        light_emission = light_emission.max(element_light_emission(element)?);
    }
    Some(light_emission)
}

fn element_shade(element: &RawBlockElement) -> bool {
    element.shade.unwrap_or(true)
}

fn element_light_emission(element: &RawBlockElement) -> Option<u8> {
    quantize_light_emission(element.light_emission.unwrap_or(0))
}

fn has_face_uv_transform(element: &RawBlockElement) -> bool {
    element.faces.values().any(|face| {
        face.uv.is_some()
            || face
                .rotation
                .is_some_and(|degrees| quantize_face_uv_rotation(degrees) != Some(0))
    })
}

fn quantize_0_16(value: f32) -> Option<u8> {
    if !(0.0..=16.0).contains(&value) {
        return None;
    }
    let rounded = value.round();
    if (value - rounded).abs() > f32::EPSILON {
        return None;
    }
    Some(rounded as u8)
}
