use std::collections::BTreeMap;

use super::{
    resolve_texture_alias, BlockModelBox, BlockModelFace, BlockModelShape, RawBlockElement,
};

pub(super) fn classify_model_shape(
    elements: &[RawBlockElement],
    textures: &BTreeMap<String, String>,
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
            return BlockModelShape::Cross {
                shade: cross_shade(elements),
            };
        }
        return BlockModelShape::Custom;
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
        return BlockModelShape::Cross {
            shade: cross_shade(elements),
        };
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
    for shape in std::iter::once(first)
        .chain(std::iter::once(second))
        .chain(shapes)
    {
        match shape {
            BlockModelShape::Box(model_box) => boxes.push(model_box),
            BlockModelShape::Boxes(model_boxes) => boxes.extend(model_boxes),
            BlockModelShape::Cube => return BlockModelShape::Cube,
            BlockModelShape::Cross { .. } | BlockModelShape::Custom => {
                return BlockModelShape::Custom;
            }
        }
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
    textures: &BTreeMap<String, String>,
) -> Option<BlockModelBox> {
    let [element] = elements else {
        return None;
    };
    element_box_shape(element, textures)
}

fn multi_box_shape(
    elements: &[RawBlockElement],
    textures: &BTreeMap<String, String>,
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
    textures: &BTreeMap<String, String>,
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
    let mut face_cull = [false; 6];
    let mut face_tint_indices = [None; 6];
    let mut face_textures: [Option<String>; 6] = std::array::from_fn(|_| None);
    let element_shade = element_shade(element);
    for (face_name, raw_face) in &element.faces {
        let face = BlockModelFace::from_name(face_name)?;
        face_present[face.index()] = true;
        face_uvs[face.index()] = raw_face
            .uv
            .and_then(quantize_uv_0_16)
            .unwrap_or([0, 0, 16, 16]);
        face_uv_rotations[face.index()] = raw_face
            .rotation
            .map(quantize_face_uv_rotation)
            .unwrap_or(Some(0))?;
        face_shade[face.index()] = element_shade;
        face_cull[face.index()] = raw_face
            .cullface
            .as_deref()
            .and_then(BlockModelFace::from_name)
            .is_some();
        face_tint_indices[face.index()] = raw_face.tintindex;
        face_textures[face.index()] = resolve_texture_alias(textures, &raw_face.texture);
    }

    Some(BlockModelBox {
        from,
        to,
        face_present,
        face_uvs,
        face_uv_rotations,
        face_shade,
        face_cull,
        face_tint_indices,
        face_textures,
    })
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

fn quantize_face_uv_rotation(degrees: i32) -> Option<u8> {
    match degrees.rem_euclid(360) {
        0 => Some(0),
        90 => Some(1),
        180 => Some(2),
        270 => Some(3),
        _ => None,
    }
}

fn has_box_metadata_transform(element: &RawBlockElement) -> bool {
    !element_shade(element) || has_face_uv_transform(element)
}

fn cross_shade(elements: &[RawBlockElement]) -> bool {
    elements.iter().all(element_shade)
}

fn element_shade(element: &RawBlockElement) -> bool {
    element.shade.unwrap_or(true)
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
