//! Standalone block geometry for item-model residency: bake a block's [`TerrainRenderShape`] into
//! [`ItemModelQuad`]s, reusing terrain box/quad/cross geometry, atlas UV mapping, and directional
//! shade. Unlike chunk meshing there is no neighbour culling and no lightmap/AO. Held / dropped /
//! framed / GUI block items show every box/quad face but continue to suppress in-world-only
//! `Cross`/`Crosses`; moving blocks preserve those crosses because `FallingBlockRenderer` accepts any
//! block whose render shape is `MODEL`.

use super::super::{
    TerrainCardinalLighting, TerrainQuad, TerrainRenderShape, TerrainTextureAtlas, TerrainTint,
    TerrainTransparency,
};
use super::emitter::face_from_normal;
use super::geometry::{box_face_corners, face_uvs_from_crop, CROSS_FACES, FACES};
use crate::item_models::ItemModelQuad;

/// The default full-face UV crop (vanilla `0..=16` over the sprite) used for a bare cube's faces.
const FULL_FACE_UVS: [u8; 4] = [0, 0, 16, 16];

/// Bakes a block's terrain render shape (plus its per-face atlas `texture_indices` and `tint`, as
/// produced by the native `block_render_data`) into item-model quads in vanilla `0..=16` model space.
pub(in crate::terrain) fn bake_block_item_quads(
    shape: &TerrainRenderShape,
    texture_indices: [u32; 6],
    tint: [TerrainTint; 6],
    atlas: &TerrainTextureAtlas,
) -> Vec<ItemModelQuad> {
    bake_block_quads(
        shape,
        texture_indices,
        tint,
        TerrainCardinalLighting::Default,
        atlas,
        false,
        [TerrainTransparency::OPAQUE; 6],
    )
}

pub(in crate::terrain) fn bake_moving_block_quads(
    shape: &TerrainRenderShape,
    texture_indices: [u32; 6],
    tint: [TerrainTint; 6],
    face_transparency: [TerrainTransparency; 6],
    cardinal_lighting: TerrainCardinalLighting,
    atlas: &TerrainTextureAtlas,
) -> Vec<ItemModelQuad> {
    bake_block_quads(
        shape,
        texture_indices,
        tint,
        cardinal_lighting,
        atlas,
        true,
        face_transparency,
    )
}

fn bake_block_quads(
    shape: &TerrainRenderShape,
    texture_indices: [u32; 6],
    tint: [TerrainTint; 6],
    cardinal_lighting: TerrainCardinalLighting,
    atlas: &TerrainTextureAtlas,
    include_crosses: bool,
    fallback_face_transparency: [TerrainTransparency; 6],
) -> Vec<ItemModelQuad> {
    let mut quads = Vec::new();
    match shape {
        TerrainRenderShape::Cube => push_box(
            &mut quads,
            atlas,
            cardinal_lighting,
            [0, 0, 0],
            [16, 16, 16],
            [true; 6],
            [FULL_FACE_UVS; 6],
            [0; 6],
            [true; 6],
            fallback_face_transparency,
            texture_indices,
            tint,
        ),
        TerrainRenderShape::Box {
            from,
            to,
            face_present,
            face_uvs,
            face_uv_rotations,
            face_shade,
            face_transparency,
            ..
        } => push_box(
            &mut quads,
            atlas,
            cardinal_lighting,
            *from,
            *to,
            *face_present,
            *face_uvs,
            *face_uv_rotations,
            *face_shade,
            *face_transparency,
            texture_indices,
            tint,
        ),
        TerrainRenderShape::Boxes(boxes) => {
            for model_box in boxes {
                push_box(
                    &mut quads,
                    atlas,
                    cardinal_lighting,
                    model_box.from,
                    model_box.to,
                    model_box.face_present,
                    model_box.face_uvs,
                    model_box.face_uv_rotations,
                    model_box.face_shade,
                    model_box.face_transparency,
                    model_box.texture_indices,
                    model_box.tint,
                );
            }
        }
        TerrainRenderShape::Quads(model_quads) => {
            for quad in model_quads {
                quads.push(quad_to_item_quad(quad, cardinal_lighting, atlas));
            }
        }
        TerrainRenderShape::Cross {
            shade,
            light_emission: _,
        } if include_crosses => push_cross(
            &mut quads,
            atlas,
            cardinal_lighting,
            texture_indices,
            tint,
            fallback_face_transparency,
            *shade,
        ),
        TerrainRenderShape::Crosses(crosses) if include_crosses => {
            for cross in crosses {
                push_cross(
                    &mut quads,
                    atlas,
                    cardinal_lighting,
                    cross.texture_indices,
                    cross.tint,
                    cross.face_transparency,
                    cross.shade,
                );
            }
        }
        TerrainRenderShape::Cross { .. } | TerrainRenderShape::Crosses(_) => {}
    }
    quads
}

fn push_cross(
    quads: &mut Vec<ItemModelQuad>,
    atlas: &TerrainTextureAtlas,
    cardinal_lighting: TerrainCardinalLighting,
    texture_indices: [u32; 6],
    tint: [TerrainTint; 6],
    face_transparency: [TerrainTransparency; 6],
    shade: bool,
) {
    let local_uvs = [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];
    for (face, normal, corners) in CROSS_FACES {
        let index = face.index();
        let rect = atlas.rect(texture_indices[index]);
        quads.push(ItemModelQuad {
            corners: corners.map(|corner| corner.map(|coordinate| coordinate * 16.0)),
            uvs: local_uvs.map(|uv| rect.map(uv)),
            tint: tint_rgba(tint[index]),
            normal,
            shade: cardinal_lighting.shade(shade, face),
            translucent: face_transparency[index].has_translucent,
        });
    }
}

/// Emits every present face of a `0..=16` box as an item-model quad (mirrors the terrain `emit_box`
/// geometry without culling / lighting): box-face corners, the cropped + rotated face UVs mapped into
/// the face texture's atlas rect, the per-face tint, and the directional cardinal shade.
#[allow(clippy::too_many_arguments)]
fn push_box(
    quads: &mut Vec<ItemModelQuad>,
    atlas: &TerrainTextureAtlas,
    cardinal_lighting: TerrainCardinalLighting,
    from: [u8; 3],
    to: [u8; 3],
    face_present: [bool; 6],
    face_uvs: [[u8; 4]; 6],
    face_uv_rotations: [u8; 6],
    face_shade: [bool; 6],
    face_transparency: [TerrainTransparency; 6],
    texture_indices: [u32; 6],
    tint: [TerrainTint; 6],
) {
    let min = [from[0] as f32, from[1] as f32, from[2] as f32];
    let max = [to[0] as f32, to[1] as f32, to[2] as f32];
    for face in FACES {
        let index = face.face.index();
        if !face_present[index] {
            continue;
        }
        let corners = box_face_corners(face.face, min, max);
        let local_uvs = face_uvs_from_crop(face.face, face_uvs[index], face_uv_rotations[index]);
        let rect = atlas.rect(texture_indices[index]);
        quads.push(ItemModelQuad {
            corners,
            uvs: local_uvs.map(|uv| rect.map(uv)),
            tint: tint_rgba(tint[index]),
            normal: face.normal,
            shade: cardinal_lighting.shade(face_shade[index], face.face),
            translucent: face_transparency[index].has_translucent,
        });
    }
}

/// Converts a free-form model quad (already in `0..=16` model space with local UVs) into an item-model
/// quad: the quad's local UVs mapped into its texture's atlas rect, tint, and the directional shade for
/// the quad's cull face (or, lacking one, the face nearest its normal — vanilla `emit_quads`).
fn quad_to_item_quad(
    quad: &TerrainQuad,
    cardinal_lighting: TerrainCardinalLighting,
    atlas: &TerrainTextureAtlas,
) -> ItemModelQuad {
    let rect = atlas.rect(quad.texture_index);
    let face = quad.cull.unwrap_or_else(|| face_from_normal(quad.normal));
    ItemModelQuad {
        corners: quad.corners,
        uvs: quad.uvs.map(|uv| rect.map(uv)),
        tint: tint_rgba(quad.tint),
        normal: quad.normal,
        shade: cardinal_lighting.shade(quad.shade, face),
        translucent: quad.transparency.has_translucent,
    }
}

/// A terrain tint (RGB) as an opaque item-model tint `[r, g, b, 1.0]`.
fn tint_rgba(tint: TerrainTint) -> [f32; 4] {
    let [r, g, b] = tint.as_shader_tint();
    [r, g, b, 1.0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item_models::{bake_item_model_mesh, ItemModelMesh};
    use crate::terrain::{TerrainFace, TerrainTransparency};
    use glam::{Mat4, Vec3};

    #[test]
    fn cube_bakes_six_faces_with_vanilla_directional_shade() {
        // A full cube bakes all six faces (FACES order Down/Up/North/South/West/East) over the `0..=16`
        // box, each with `Direction.getShade`: Up 1.0, Down 0.5, N/S 0.8, W/E 0.6.
        let atlas = TerrainTextureAtlas::unit();
        let quads = bake_block_item_quads(
            &TerrainRenderShape::Cube,
            [0; 6],
            [TerrainTint::WHITE; 6],
            &atlas,
        );
        assert_eq!(quads.len(), 6);
        assert_eq!(quads[0].shade, 0.5, "down face");
        assert_eq!(quads[1].shade, 1.0, "up face");
        assert_eq!(quads[2].shade, 0.8, "north face");
        assert_eq!(quads[4].shade, 0.6, "west face");
        assert_eq!(quads[0].normal, [0.0, -1.0, 0.0], "down normal");
        assert_eq!(quads[1].normal, [0.0, 1.0, 0.0], "up normal");
        assert_eq!(quads[2].normal, [0.0, 0.0, -1.0], "north normal");
        assert_eq!(quads[4].normal, [-1.0, 0.0, 0.0], "west normal");
        assert_mesh_triangles_face_submitted_normals(&bake_item_model_mesh(&quads, Mat4::IDENTITY));
        // The up face spans the top of the `0..=16` cube.
        assert_eq!(
            quads[1].corners,
            [
                [0.0, 16.0, 0.0],
                [16.0, 16.0, 0.0],
                [16.0, 16.0, 16.0],
                [0.0, 16.0, 16.0]
            ]
        );
        // White tint, opaque.
        assert!(quads.iter().all(|quad| quad.tint == [1.0, 1.0, 1.0, 1.0]));
    }

    #[test]
    fn moving_block_quads_use_the_dimension_cardinal_lighting() {
        let atlas = TerrainTextureAtlas::unit();
        let quads = bake_moving_block_quads(
            &TerrainRenderShape::Cube,
            [0; 6],
            [TerrainTint::WHITE; 6],
            [TerrainTransparency::OPAQUE; 6],
            TerrainCardinalLighting::Nether,
            &atlas,
        );
        assert_eq!(quads.len(), 6);
        assert_eq!(quads[0].shade, 0.9, "nether down face");
        assert_eq!(quads[1].shade, 0.9, "nether up face");
        assert_eq!(quads[2].shade, 0.8, "nether north face");
        assert_eq!(quads[4].shade, 0.6, "nether west face");
    }

    #[test]
    fn moving_block_quads_keep_world_cross_geometry() {
        let atlas = TerrainTextureAtlas::unit();
        let moving = bake_moving_block_quads(
            &TerrainRenderShape::Cross {
                shade: true,
                light_emission: 0,
            },
            [0; 6],
            [TerrainTint::WHITE; 6],
            [TerrainTransparency::OPAQUE; 6],
            TerrainCardinalLighting::Default,
            &atlas,
        );
        assert_eq!(moving.len(), 4);
        assert_eq!(moving[0].corners[2], [16.0, 16.0, 16.0]);
        assert!(bake_block_item_quads(
            &TerrainRenderShape::Cross {
                shade: true,
                light_emission: 0,
            },
            [0; 6],
            [TerrainTint::WHITE; 6],
            &atlas,
        )
        .is_empty());
    }

    #[test]
    fn box_bakes_only_present_faces() {
        // A box with only the up + south faces present bakes exactly those two quads.
        let atlas = TerrainTextureAtlas::unit();
        let quads = bake_block_item_quads(
            &TerrainRenderShape::Box {
                from: [2, 0, 2],
                to: [14, 16, 14],
                // Down, Up, North, South, West, East.
                face_present: [false, true, false, true, false, false],
                face_uvs: [[0, 0, 16, 16]; 6],
                face_uv_rotations: [0; 6],
                face_shade: [true; 6],
                face_light_emission: [0; 6],
                face_cull: [None; 6],
                face_transparency: [TerrainTransparency::OPAQUE; 6],
            },
            [0; 6],
            [TerrainTint::WHITE; 6],
            &atlas,
        );
        assert_eq!(quads.len(), 2);
        // The up face sits at the box top y=16, spanning x/z 2..14.
        assert_eq!(quads[0].shade, 1.0);
        assert_eq!(quads[0].corners[0], [2.0, 16.0, 2.0]);
    }

    #[test]
    fn box_marks_translucent_faces_for_item_submit_split() {
        let atlas = TerrainTextureAtlas::unit();
        let mut face_transparency = [TerrainTransparency::OPAQUE; 6];
        face_transparency[TerrainFace::Up.index()] = TerrainTransparency::TRANSLUCENT;
        let quads = bake_block_item_quads(
            &TerrainRenderShape::Box {
                from: [0, 0, 0],
                to: [16, 16, 16],
                face_present: [false, true, false, false, false, false],
                face_uvs: [[0, 0, 16, 16]; 6],
                face_uv_rotations: [0; 6],
                face_shade: [true; 6],
                face_light_emission: [0; 6],
                face_cull: [None; 6],
                face_transparency,
            },
            [0; 6],
            [TerrainTint::WHITE; 6],
            &atlas,
        );

        assert_eq!(quads.len(), 1);
        assert!(quads[0].translucent);
    }

    #[test]
    fn quads_shape_passes_corners_and_shades_by_normal() {
        // A free-form quad preserves caller winding; its `0..=16` corners pass through and the shade
        // follows the face nearest its normal (here +Z = south = 0.8).
        let atlas = TerrainTextureAtlas::unit();
        let quad = TerrainQuad {
            corners: [
                [0.0, 0.0, 16.0],
                [16.0, 0.0, 16.0],
                [16.0, 16.0, 16.0],
                [0.0, 16.0, 16.0],
            ],
            normal: [0.0, 0.0, 1.0],
            uvs: [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            cull: Some(TerrainFace::South),
            texture_index: 0,
            tint: TerrainTint::WHITE,
            transparency: TerrainTransparency::OPAQUE,
            shade: true,
            light_emission: 0,
        };
        let quads = bake_block_item_quads(
            &TerrainRenderShape::Quads(vec![quad]),
            [0; 6],
            [TerrainTint::WHITE; 6],
            &atlas,
        );
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0].corners, quad.corners);
        assert_eq!(quads[0].normal, [0.0, 0.0, 1.0]);
        assert_eq!(quads[0].shade, 0.8);
        assert!(!quads[0].translucent);
        assert_mesh_triangles_face_submitted_normals(&bake_item_model_mesh(&quads, Mat4::IDENTITY));
    }

    #[test]
    fn cross_shapes_are_never_items() {
        // `Cross`/`Crosses` are in-world foliage geometry, never the shape of an item; they bake nothing.
        let atlas = TerrainTextureAtlas::unit();
        assert!(bake_block_item_quads(
            &TerrainRenderShape::Cross {
                shade: true,
                light_emission: 0,
            },
            [0; 6],
            [TerrainTint::WHITE; 6],
            &atlas,
        )
        .is_empty());
        assert!(bake_block_item_quads(
            &TerrainRenderShape::Crosses(Vec::new()),
            [0; 6],
            [TerrainTint::WHITE; 6],
            &atlas,
        )
        .is_empty());
    }

    fn assert_mesh_triangles_face_submitted_normals(mesh: &ItemModelMesh) {
        for indices in mesh.indices.chunks_exact(3) {
            let a = mesh.vertices[indices[0] as usize].position;
            let b = mesh.vertices[indices[1] as usize].position;
            let c = mesh.vertices[indices[2] as usize].position;
            let [nx, ny, nz, _] = mesh.vertices[indices[0] as usize].normal_diffuse;
            let submitted_normal = Vec3::new(nx, ny, nz);
            assert!(
                triangle_normal(a, b, c).dot(submitted_normal) > 0.999,
                "mesh triangle winding must face its submitted normal: {indices:?}"
            );
        }
    }

    fn triangle_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> Vec3 {
        let a = Vec3::from_array(a);
        let b = Vec3::from_array(b);
        let c = Vec3::from_array(c);
        (b - a).cross(c - a).normalize()
    }
}
