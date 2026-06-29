//! Block-item geometry for the item-model renderer: bake a block's [`TerrainRenderShape`] into
//! standalone [`ItemModelQuad`]s, reusing the terrain box/quad geometry, atlas UV mapping, and
//! directional shade. Unlike chunk meshing there is no neighbour culling and no lightmap/AO: a held /
//! dropped / framed / GUI block-item shows every present face, lit by the item context. `Cube`, `Box`,
//! `Boxes`, and `Quads` cover every full and partial block model; `Cross`/`Crosses` are in-world-only
//! foliage geometry and never the shape of an item, so they bake nothing.

use super::super::{
    TerrainQuad, TerrainRenderShape, TerrainTextureAtlas, TerrainTint, TerrainTransparency,
};
use super::emitter::{cardinal_shade, face_from_normal};
use super::geometry::{box_face_corners, face_uvs_from_crop, FACES};
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
    let mut quads = Vec::new();
    match shape {
        TerrainRenderShape::Cube => push_box(
            &mut quads,
            atlas,
            [0, 0, 0],
            [16, 16, 16],
            [true; 6],
            [FULL_FACE_UVS; 6],
            [0; 6],
            [true; 6],
            [TerrainTransparency::OPAQUE; 6],
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
                quads.push(quad_to_item_quad(quad, atlas));
            }
        }
        TerrainRenderShape::Cross { .. } | TerrainRenderShape::Crosses(_) => {}
    }
    quads
}

/// Emits every present face of a `0..=16` box as an item-model quad (mirrors the terrain `emit_box`
/// geometry without culling / lighting): box-face corners, the cropped + rotated face UVs mapped into
/// the face texture's atlas rect, the per-face tint, and the directional cardinal shade.
#[allow(clippy::too_many_arguments)]
fn push_box(
    quads: &mut Vec<ItemModelQuad>,
    atlas: &TerrainTextureAtlas,
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
        let local_uvs = face_uvs_from_crop(face_uvs[index], face_uv_rotations[index]);
        let rect = atlas.rect(texture_indices[index]);
        quads.push(ItemModelQuad {
            corners,
            uvs: local_uvs.map(|uv| rect.map(uv)),
            tint: tint_rgba(tint[index]),
            normal: face.normal,
            shade: cardinal_shade(face_shade[index], face.face),
            translucent: face_transparency[index].has_translucent,
        });
    }
}

/// Converts a free-form model quad (already in `0..=16` model space with local UVs) into an item-model
/// quad: the quad's local UVs mapped into its texture's atlas rect, tint, and the directional shade for
/// the quad's cull face (or, lacking one, the face nearest its normal — vanilla `emit_quads`).
fn quad_to_item_quad(quad: &TerrainQuad, atlas: &TerrainTextureAtlas) -> ItemModelQuad {
    let rect = atlas.rect(quad.texture_index);
    let face = quad.cull.unwrap_or_else(|| face_from_normal(quad.normal));
    ItemModelQuad {
        corners: quad.corners,
        uvs: quad.uvs.map(|uv| rect.map(uv)),
        tint: tint_rgba(quad.tint),
        normal: quad.normal,
        shade: cardinal_shade(quad.shade, face),
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
    use crate::terrain::{TerrainFace, TerrainTransparency};

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
        // A free-form quad renders unculled; its `0..=16` corners pass through and the shade follows the
        // face nearest its normal (here +Z = south = 0.8).
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
}
