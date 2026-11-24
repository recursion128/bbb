use std::collections::BTreeMap;
use std::sync::OnceLock;

use super::{
    raw_model::RawBlockModelFace, resolve_texture_alias, BlockFaceTextures, BlockModelBox,
    BlockModelCross, BlockModelFace, BlockModelQuad, BlockModelShape, RawBlockElement,
    RawBlockElementRotation, ResolvedTextureReference,
};

/// Bakes `elements` to exact geometry: axis-aligned boxes when every element is one, quads
/// otherwise. `simplify_model_shape` picks the renderer's fast paths afterwards.
pub(super) fn classify_model_shape(
    elements: &[RawBlockElement],
    textures: &BTreeMap<String, ResolvedTextureReference>,
) -> BlockModelShape {
    if let Some(mut boxes) = elements
        .iter()
        .map(|element| element_box_shape(element, textures))
        .collect::<Option<Vec<_>>>()
    {
        return match boxes.len() {
            1 => BlockModelShape::Box(boxes.remove(0)),
            _ => BlockModelShape::Boxes(boxes),
        };
    }
    // A model Vanilla would reject bakes no geometry rather than a stand-in cube.
    elements
        .iter()
        .map(|element| element_quads(element, textures))
        .collect::<Option<Vec<_>>>()
        .map(|quads| BlockModelShape::Quads(quads.into_iter().flatten().collect()))
        .unwrap_or_default()
}

/// Replaces exact geometry with the `Cube` or `Cross` fast path when that path draws the same
/// faces with the same textures, UVs and culling.
pub(super) fn simplify_model_shape(
    shape: BlockModelShape,
    face_textures: &BlockFaceTextures,
) -> BlockModelShape {
    match shape {
        BlockModelShape::Box(model_box) if is_plain_cube(&model_box, face_textures) => {
            BlockModelShape::Cube
        }
        BlockModelShape::Quads(quads) => {
            canonical_cross_shape(&quads, face_textures).unwrap_or(BlockModelShape::Quads(quads))
        }
        shape => shape,
    }
}

pub(super) const MAX_COMBINED_BOXES_PER_STATE: usize = 2_048;
pub(super) const MAX_COMBINED_QUADS_PER_STATE: usize = 12_288;

pub(super) fn combine_model_shapes(shapes: Vec<BlockModelShape>) -> BlockModelShape {
    let mut shapes = shapes.into_iter();
    let Some(first) = shapes.next() else {
        return BlockModelShape::default();
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
            BlockModelShape::Box(model_box) => {
                if boxes.len() < MAX_COMBINED_BOXES_PER_STATE {
                    boxes.push(model_box);
                }
            }
            BlockModelShape::Boxes(model_boxes) => {
                let remaining = MAX_COMBINED_BOXES_PER_STATE.saturating_sub(boxes.len());
                boxes.extend(model_boxes.into_iter().take(remaining));
            }
            BlockModelShape::Quads(model_quads) => {
                has_quads = true;
                let remaining = MAX_COMBINED_QUADS_PER_STATE.saturating_sub(quads.len());
                quads.extend(model_quads.into_iter().take(remaining));
            }
            BlockModelShape::Cube | BlockModelShape::Cross { .. } | BlockModelShape::Crosses(_) => {
                unreachable!("fast-path shapes are chosen after combining")
            }
        }
    }

    if has_quads {
        for model_box in boxes {
            let remaining = MAX_COMBINED_QUADS_PER_STATE.saturating_sub(quads.len());
            if remaining == 0 {
                break;
            }
            quads.extend(box_to_quads(&model_box).into_iter().take(remaining));
        }
        return BlockModelShape::Quads(quads);
    }

    match boxes.len() {
        1 => BlockModelShape::Box(boxes.remove(0)),
        _ => BlockModelShape::Boxes(boxes),
    }
}

pub(super) fn shape_has_force_translucent(
    shape: &BlockModelShape,
    face_textures: &BlockFaceTextures,
) -> bool {
    if face_textures.force_translucent.iter().any(|&v| v) {
        return true;
    }
    match shape {
        BlockModelShape::Cube | BlockModelShape::Cross { .. } => false,
        BlockModelShape::Crosses(crosses) => crosses
            .iter()
            .any(|cross| cross.face_force_translucent.iter().any(|&v| v)),
        BlockModelShape::Box(model_box) => model_box.face_force_translucent.iter().any(|&v| v),
        BlockModelShape::Boxes(boxes) => boxes
            .iter()
            .any(|model_box| model_box.face_force_translucent.iter().any(|&v| v)),
        BlockModelShape::Quads(quads) => quads.iter().any(|quad| quad.force_translucent),
    }
}

fn is_plain_cube(model_box: &BlockModelBox, face_textures: &BlockFaceTextures) -> bool {
    model_box.from == [0; 3]
        && model_box.to == [16; 3]
        && BlockModelFace::ALL.into_iter().all(|face| {
            let index = face.index();
            model_box.face_present[index]
                && model_box.face_uvs[index] == [0, 0, 16, 16]
                && model_box.face_uv_rotations[index] == 0
                && model_box.face_shade[index]
                && model_box.face_light_emission[index] == 0
                && model_box.face_cull[index] == Some(face)
                && model_box.face_textures[index].as_deref() == Some(face_textures.get(face))
                && model_box.face_tint_indices[index] == face_textures.tint_index(face)
                && model_box.face_force_translucent[index] == face_textures.force_translucent(face)
        })
}

/// The quads of Vanilla's `block/cross` template, which the `Cross` fast path draws.
fn canonical_cross_quads() -> &'static [BlockModelQuad] {
    static QUADS: OnceLock<Vec<BlockModelQuad>> = OnceLock::new();
    QUADS.get_or_init(|| {
        let plane = |from: [f32; 3], to: [f32; 3], faces: [&str; 2]| RawBlockElement {
            from: Some(from),
            to: Some(to),
            rotation: Some(RawBlockElementRotation {
                origin: [8.0, 8.0, 8.0],
                axis: Some("y".to_string()),
                angle: Some(45.0),
                x: None,
                y: None,
                z: None,
                rescale: true,
            }),
            shade: Some(false),
            light_emission: None,
            faces: faces
                .into_iter()
                .map(|face| {
                    (
                        face.to_string(),
                        RawBlockModelFace {
                            texture: "#cross".to_string(),
                            uv: Some([0.0, 0.0, 16.0, 16.0]),
                            rotation: None,
                            cullface: None,
                            tintindex: None,
                        },
                    )
                })
                .collect(),
        };
        [
            plane([0.8, 0.0, 8.0], [15.2, 16.0, 8.0], ["north", "south"]),
            plane([8.0, 0.0, 0.8], [8.0, 16.0, 15.2], ["west", "east"]),
        ]
        .iter()
        .flat_map(|element| element_quads(element, &BTreeMap::new()).expect("cross template bakes"))
        .collect()
    })
}

/// Each run of four quads must be one canonical cross (in any quarter turn) with a single shade
/// and light emission and no cull faces.
fn canonical_cross_shape(
    quads: &[BlockModelQuad],
    face_textures: &BlockFaceTextures,
) -> Option<BlockModelShape> {
    if quads.is_empty() || !quads.len().is_multiple_of(4) {
        return None;
    }
    let canonical = canonical_cross_quads();
    let mut crosses = Vec::with_capacity(quads.len() / 4);
    for chunk in quads.chunks(4) {
        let mut cross = BlockModelCross {
            face_textures: std::array::from_fn(|_| None),
            face_tint_indices: [None; 6],
            face_force_translucent: [false; 6],
            shade: chunk[0].shade,
            light_emission: chunk[0].light_emission,
        };
        let mut matched = [false; 4];
        for quad in chunk {
            if quad.cull.is_some()
                || quad.shade != cross.shade
                || quad.light_emission != cross.light_emission
            {
                return None;
            }
            let slot = (0..4)
                .find(|&slot| !matched[slot] && same_quad_geometry(quad, &canonical[slot]))?;
            matched[slot] = true;
            let index = canonical[slot].face.index();
            cross.face_textures[index] = quad.texture.clone();
            cross.face_tint_indices[index] = quad.tint_index;
            cross.face_force_translucent[index] = quad.force_translucent;
        }
        crosses.push(cross);
    }
    let [cross] = crosses.as_slice() else {
        return Some(BlockModelShape::Crosses(crosses));
    };
    let uses_face_textures = canonical.iter().all(|quad| {
        let index = quad.face.index();
        cross.face_textures[index].as_deref() == Some(face_textures.get(quad.face))
            && cross.face_tint_indices[index] == face_textures.tint_index(quad.face)
            && cross.face_force_translucent[index] == face_textures.force_translucent(quad.face)
    });
    Some(if uses_face_textures {
        BlockModelShape::Cross {
            shade: cross.shade,
            light_emission: cross.light_emission,
        }
    } else {
        BlockModelShape::Crosses(crosses)
    })
}

fn same_quad_geometry(quad: &BlockModelQuad, other: &BlockModelQuad) -> bool {
    let close = |a: &[f32], b: &[f32]| a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-4);
    close(&quad.normal, &other.normal)
        && quad.corners.iter().zip(&quad.uvs).all(|(corner, uv)| {
            other
                .corners
                .iter()
                .zip(&other.uvs)
                .any(|(other_corner, other_uv)| close(corner, other_corner) && close(uv, other_uv))
        })
}

fn element_box_shape(
    element: &RawBlockElement,
    textures: &BTreeMap<String, ResolvedTextureReference>,
) -> Option<BlockModelBox> {
    if element.rotation.is_some() {
        return None;
    }
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

fn element_quads(
    element: &RawBlockElement,
    textures: &BTreeMap<String, ResolvedTextureReference>,
) -> Option<Vec<BlockModelQuad>> {
    // Flat (ladders, crops, bars) and inverted (`cube_all_inner_faces`) elements are valid:
    // `CuboidModelElement` only bounds each coordinate to [-16, 32].
    let from = element.from?;
    let to = element.to?;

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
        // `FaceBakery.calculateFacing` takes the direction from the baked vertices, so an
        // inverted element's faces point inwards.
        let normal = match (quad_normal(corners), &element.rotation) {
            (Some(normal), _) => normal,
            (None, Some(rotation)) => rotate_element_normal(face_normal(face), rotation)?,
            (None, None) => face_normal(face),
        };
        let force_translucent = resolved_texture
            .as_ref()
            .map(|texture| texture.force_translucent)
            .unwrap_or(false);
        let mut quad = BlockModelQuad {
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
            element_rotated: element.rotation.is_some(),
        };
        if !quad.element_rotated {
            recalculate_winding(&mut quad);
        }
        quads.push(quad);
    }
    Some(quads)
}

pub(super) fn box_to_quads(model_box: &BlockModelBox) -> Vec<BlockModelQuad> {
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
            element_rotated: false,
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

pub(super) fn face_uvs_from_crop_f32(uv: [f32; 4], rotation: u8) -> [[f32; 2]; 4] {
    let u0 = uv[0] / 16.0;
    let v0 = uv[1] / 16.0;
    let u1 = uv[2] / 16.0;
    let v1 = uv[3] / 16.0;
    // `CuboidFace.getU/getV` with Quadrant's `(vertex + shift) % 4`.
    let vanilla_vertex_uvs = [[u0, v0], [u0, v1], [u1, v1], [u1, v0]];
    std::array::from_fn(|vertex| vanilla_vertex_uvs[(vertex + rotation as usize) % 4])
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

/// `FaceInfo` vertex order, selecting `from`/`to` as `FaceInfo.VertexInfo.select` does.
pub(super) fn model_face_corners(
    face: BlockModelFace,
    from: [f32; 3],
    to: [f32; 3],
) -> [[f32; 3]; 4] {
    face_info(face)
        .map(|extent| std::array::from_fn(|axis| if extent[axis] { to[axis] } else { from[axis] }))
}

/// `FaceInfo` vertex extents: `true` selects the max (`to`) bound on that axis.
fn face_info(face: BlockModelFace) -> [[bool; 3]; 4] {
    const O: bool = false;
    const I: bool = true;
    match face {
        BlockModelFace::Down => [[O, O, I], [O, O, O], [I, O, O], [I, O, I]],
        BlockModelFace::Up => [[O, I, O], [O, I, I], [I, I, I], [I, I, O]],
        BlockModelFace::North => [[I, I, O], [I, O, O], [O, O, O], [O, I, O]],
        BlockModelFace::South => [[O, I, I], [O, O, I], [I, O, I], [I, I, I]],
        BlockModelFace::West => [[O, I, O], [O, O, O], [O, O, I], [O, I, I]],
        BlockModelFace::East => [[I, I, I], [I, O, I], [I, O, O], [I, I, O]],
    }
}

/// `FaceBakery.recalculateWinding`: reorders an unrotated element's vertices into the
/// `FaceInfo` order of its baked facing.
pub(super) fn recalculate_winding(quad: &mut BlockModelQuad) {
    let Some(direction) = closest_face(quad.normal) else {
        return;
    };
    let mut min = [999.0_f32; 3];
    let mut max = [-999.0_f32; 3];
    for corner in &quad.corners {
        for axis in 0..3 {
            min[axis] = min[axis].min(corner[axis]);
            max[axis] = max[axis].max(corner[axis]);
        }
    }
    for (vertex, extent) in face_info(direction).into_iter().enumerate() {
        let target: [f32; 3] =
            std::array::from_fn(|axis| if extent[axis] { max[axis] } else { min[axis] });
        let Some(found) = (vertex..4).find(|&index| quad.corners[index] == target) else {
            return;
        };
        quad.corners.swap(found, vertex);
        quad.uvs.swap(found, vertex);
    }
}

/// `FaceBakery.findClosestDirection`.
fn closest_face(normal: [f32; 3]) -> Option<BlockModelFace> {
    let mut best = None;
    let mut closest = 0.0_f32;
    for face in BlockModelFace::ALL {
        let unit = face_normal(face);
        let product = normal[0] * unit[0] + normal[1] * unit[1] + normal[2] * unit[2];
        if product >= 0.0 && product > closest {
            closest = product;
            best = Some(face);
        }
    }
    best
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

/// The outward normal of corners in `FaceInfo` order, `None` for a degenerate quad.
fn quad_normal(corners: [[f32; 3]; 4]) -> Option<[f32; 3]> {
    let a = sub3(corners[3], corners[1]);
    let b = sub3(corners[2], corners[0]);
    let normal = [
        a[2] * b[1] - a[1] * b[2],
        a[0] * b[2] - a[2] * b[0],
        a[1] * b[0] - a[0] * b[1],
    ];
    let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    (length > 1e-6).then(|| normal.map(|component| component / length))
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

fn element_shade(element: &RawBlockElement) -> bool {
    element.shade.unwrap_or(true)
}

fn element_light_emission(element: &RawBlockElement) -> Option<u8> {
    quantize_light_emission(element.light_emission.unwrap_or(0))
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
