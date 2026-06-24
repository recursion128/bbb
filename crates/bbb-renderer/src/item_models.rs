//! 3D block-model / item-model rendering: baking parsed cuboid models (and extruded flat items) into a
//! mesh of textured quads, drawn standalone with a model transform.
//!
//! Mirrors the entity-model split: the renderer owns the mesh assembly + GPU pipeline, while the native
//! layer (which holds the parsed `bbb-pack` models + the block/item texture atlas) produces the
//! atlas-resolved [`ItemModelQuad`]s. A quad's `corners` are in vanilla model space (the `0..=16` box
//! coordinates, the same units `from`/`to` use), normalized to the `0..1` unit cube at bake time so the
//! caller's `transform` places the model in world / GUI / hand space exactly like vanilla's display
//! transforms. `uvs` are atlas-absolute into the shared block/item atlas. `tint` is the per-face color
//! (biome/dye tint, or white) and `shade` is the directional face-shade multiplier (vanilla
//! `Direction.getShade` with ambient occlusion off); the baked vertex color is `tint × shade`.

use glam::{Mat4, Vec3};

/// Vanilla model space is `0..=16`; the unit cube is that divided by 16.
const MODEL_SPACE_SCALE: f32 = 1.0 / 16.0;

/// One textured quad of a baked block/item model: four corners wound counter-clockwise (front face),
/// in vanilla `0..=16` model space, with atlas-absolute UVs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemModelQuad {
    pub corners: [[f32; 3]; 4],
    pub uvs: [[f32; 2]; 4],
    /// Per-face tint (biome/dye/potion color, or white when untinted). Multiplied into the vertex color.
    pub tint: [f32; 4],
    /// Directional face-shade multiplier (vanilla `Direction.getShade`, AO off). `1.0` = unshaded.
    pub shade: f32,
}

/// A baked block/item model vertex: the model-space position normalized to the unit cube and pushed
/// through the caller's `transform`, the atlas-absolute UV, and the `tint × shade` color.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ItemModelVertex {
    pub(crate) position: [f32; 3],
    pub(crate) uv: [f32; 2],
    pub(crate) color: [f32; 4],
}

/// A baked block/item model mesh: an indexed triangle list ready for the item-model pipeline.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemModelMesh {
    pub(crate) vertices: Vec<ItemModelVertex>,
    pub(crate) indices: Vec<u32>,
}

impl ItemModelMesh {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Appends `quads` to the mesh, normalizing each corner from vanilla `0..=16` model space to the unit
    /// cube and applying `transform` (the model→target-space matrix: world placement, GUI projection, or
    /// the hand attach transform). Each quad becomes two triangles wound from its four corners; the
    /// vertex color is the quad's `tint` scaled by its directional `shade` (alpha preserved).
    pub fn append_quads(&mut self, quads: &[ItemModelQuad], transform: Mat4) {
        for quad in quads {
            let base =
                u32::try_from(self.vertices.len()).expect("item-model vertex count fits in u32");
            let [tr, tg, tb, ta] = quad.tint;
            let color = [tr * quad.shade, tg * quad.shade, tb * quad.shade, ta];
            for (corner, uv) in quad.corners.iter().zip(quad.uvs.iter()) {
                let local = Vec3::from_array(*corner) * MODEL_SPACE_SCALE;
                let position = transform.transform_point3(local).to_array();
                self.vertices.push(ItemModelVertex {
                    position,
                    uv: *uv,
                    color,
                });
            }
            // Two triangles (0,1,2)+(0,2,3) over the CCW quad corners.
            self.indices
                .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
}

/// Bakes a single model's `quads` into a fresh mesh under `transform`. Convenience over
/// [`ItemModelMesh::append_quads`] for the common one-model case.
pub fn bake_item_model_mesh(quads: &[ItemModelQuad], transform: Mat4) -> ItemModelMesh {
    let mut mesh = ItemModelMesh::new();
    mesh.append_quads(quads, transform);
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_quad(shade: f32, tint: [f32; 4]) -> ItemModelQuad {
        // A full-face quad on the south side of a unit (0..=16) cube, atlas UVs 0..1.
        ItemModelQuad {
            corners: [
                [0.0, 0.0, 16.0],
                [16.0, 0.0, 16.0],
                [16.0, 16.0, 16.0],
                [0.0, 16.0, 16.0],
            ],
            uvs: [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            tint,
            shade,
        }
    }

    #[test]
    fn baking_a_quad_emits_two_triangles_normalized_to_the_unit_cube() {
        let mesh = bake_item_model_mesh(&[unit_quad(1.0, [1.0, 1.0, 1.0, 1.0])], Mat4::IDENTITY);
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
        // The `0..=16` corners are normalized to the `0..1` unit cube.
        assert_eq!(mesh.vertices[0].position, [0.0, 0.0, 1.0]);
        assert_eq!(mesh.vertices[2].position, [1.0, 1.0, 1.0]);
        assert_eq!(mesh.vertices[1].uv, [1.0, 1.0]);
    }

    #[test]
    fn shade_scales_rgb_but_not_alpha() {
        let mesh = bake_item_model_mesh(&[unit_quad(0.6, [1.0, 0.5, 0.25, 1.0])], Mat4::IDENTITY);
        // Vanilla applies `Direction.getShade` to the RGB only; alpha stays put.
        assert_eq!(mesh.vertices[0].color, [0.6, 0.3, 0.15, 1.0]);
    }

    #[test]
    fn transform_places_the_model_in_target_space() {
        // A translation places the unit cube; the corner at unit (1,1,1) lands at the offset + 1.
        let transform = Mat4::from_translation(Vec3::new(10.0, 64.0, -5.0));
        let mesh = bake_item_model_mesh(&[unit_quad(1.0, [1.0, 1.0, 1.0, 1.0])], transform);
        assert_eq!(mesh.vertices[2].position, [11.0, 65.0, -4.0]);
        assert_eq!(mesh.vertices[0].position, [10.0, 64.0, -4.0]);
    }

    #[test]
    fn append_quads_rebases_indices_across_models() {
        let mut mesh = ItemModelMesh::new();
        mesh.append_quads(&[unit_quad(1.0, [1.0, 1.0, 1.0, 1.0])], Mat4::IDENTITY);
        mesh.append_quads(&[unit_quad(1.0, [1.0, 1.0, 1.0, 1.0])], Mat4::IDENTITY);
        assert_eq!(mesh.vertices.len(), 8);
        // The second quad's triangles are rebased onto its own vertices.
        assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
    }
}
