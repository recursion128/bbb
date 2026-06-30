//! Generated (flat) item extrusion: vanilla `builtin/generated` item models — a `layerN` sprite turned
//! into a 1/16-thick slab. Faithful transcription of `ItemModelGenerator`: a full front (`SOUTH`) and
//! back (`NORTH`) face over the `0..=16` sprite, plus per-pixel "side" faces tracing the alpha
//! silhouette (every opaque pixel bordering a transparent one gets a `1px` edge quad linking front to
//! back). Corners use the same `FaceInfo` vertex selection and `CuboidFace`/`FaceBakery` UV assignment
//! the block path uses, so the output matches what vanilla's `FaceBakery.bakeQuad` produces (for the
//! identity model state the item generator always bakes with). The slab is thin; the item-model mesh
//! bake step normalizes triangle index order from each submitted normal so generated faces can run
//! through the vanilla-default back-face-culled item pipeline.

use crate::item_models::ItemModelQuad;

/// Vanilla `MIN_Z` / `MAX_Z`: the slab spans `7.5..=8.5` in model space (a `1/16` depth centered on the
/// flat sprite plane).
const MIN_Z: f32 = 7.5;
const MAX_Z: f32 = 8.5;
/// Vanilla `UV_SHRINK`: the side-face UVs inset by `0.1px` on each edge to avoid sampling neighbours.
const UV_SHRINK: f32 = 0.1;

/// Per-pixel alpha coverage of one sprite frame, row-major (`width * height`), `true` where the pixel is
/// opaque enough to contribute geometry. The native layer derives this from the sprite's atlas pixels
/// (vanilla `SpriteContents.isTransparent`: a pixel is transparent when its alpha is below the cutoff).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteAlphaMask {
    width: u32,
    height: u32,
    opaque: Vec<bool>,
}

impl SpriteAlphaMask {
    /// `opaque` is row-major, `width * height` booleans (`true` = opaque). Panics if the length mismatches.
    pub fn new(width: u32, height: u32, opaque: Vec<bool>) -> Self {
        assert_eq!(
            opaque.len(),
            (width as usize) * (height as usize),
            "sprite alpha mask length must be width * height"
        );
        Self {
            width,
            height,
            opaque,
        }
    }

    /// Vanilla `ItemModelGenerator.isTransparent`: out-of-bounds counts as transparent.
    fn is_transparent(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return true;
        }
        !self.opaque[(y as u32 * self.width + x as u32) as usize]
    }
}

/// The atlas sub-rectangle (absolute UVs) a sprite occupies: its `min`/`max` corners. A sprite-local UV
/// in `0..=1` maps linearly into this rect (vanilla `TextureAtlasSprite.getU`/`getV`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemSpriteRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

impl ItemSpriteRect {
    fn map(&self, u: f32, v: f32) -> [f32; 2] {
        [
            self.min[0] + (self.max[0] - self.min[0]) * u,
            self.min[1] + (self.max[1] - self.min[1]) * v,
        ]
    }
}

/// The six cuboid faces, with the `FaceInfo` vertex selection (which `from`/`to` extent each of the four
/// vertices reads) and the directional shade (vanilla `Direction.getShade`, AO off).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemFace {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl ItemFace {
    /// Vanilla `FaceInfo` corner extents: each entry picks, per axis, the `from` (`false`) or `to`
    /// (`true`) coordinate. Some generated side faces pass inverted extents; mesh baking normalizes
    /// triangle indices from the submitted normal before culling.
    fn corner_extents(self) -> [[bool; 3]; 4] {
        match self {
            ItemFace::Down => [
                [false, false, true],
                [false, false, false],
                [true, false, false],
                [true, false, true],
            ],
            ItemFace::Up => [
                [false, true, false],
                [false, true, true],
                [true, true, true],
                [true, true, false],
            ],
            ItemFace::North => [
                [true, true, false],
                [true, false, false],
                [false, false, false],
                [false, true, false],
            ],
            ItemFace::South => [
                [false, true, true],
                [false, false, true],
                [true, false, true],
                [true, true, true],
            ],
            ItemFace::West => [
                [false, true, false],
                [false, false, false],
                [false, false, true],
                [false, true, true],
            ],
            ItemFace::East => [
                [true, true, true],
                [true, false, true],
                [true, false, false],
                [true, true, false],
            ],
        }
    }

    /// Vanilla `Direction.getShade` with ambient occlusion off.
    fn shade(self) -> f32 {
        match self {
            ItemFace::Down => 0.5,
            ItemFace::Up => 1.0,
            ItemFace::North | ItemFace::South => 0.8,
            ItemFace::West | ItemFace::East => 0.6,
        }
    }

    fn normal(self) -> [f32; 3] {
        match self {
            ItemFace::Down => [0.0, -1.0, 0.0],
            ItemFace::Up => [0.0, 1.0, 0.0],
            ItemFace::North => [0.0, 0.0, -1.0],
            ItemFace::South => [0.0, 0.0, 1.0],
            ItemFace::West => [-1.0, 0.0, 0.0],
            ItemFace::East => [1.0, 0.0, 0.0],
        }
    }
}

/// The four silhouette-tracing side directions (vanilla `ItemModelGenerator.SideDirection`). Each maps to
/// the cuboid `Direction` its extruded face actually carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SideDirection {
    Up,
    Down,
    Left,
    Right,
}

impl SideDirection {
    /// The neighbour pixel offset whose transparency exposes this side (vanilla `checkTransition` reads
    /// `(x - dir.stepX, y - dir.stepY)`; `UP`→`Direction.UP`, `DOWN`→`DOWN`, `LEFT`→`EAST`, `RIGHT`→`WEST`).
    fn neighbor_offset(self) -> (i32, i32) {
        match self {
            SideDirection::Up => (0, -1),
            SideDirection::Down => (0, 1),
            SideDirection::Left => (-1, 0),
            SideDirection::Right => (1, 0),
        }
    }

    /// The cuboid face the extruded side quad carries (vanilla `SideDirection.getDirection`).
    fn face(self) -> ItemFace {
        match self {
            SideDirection::Up => ItemFace::Up,
            SideDirection::Down => ItemFace::Down,
            SideDirection::Left => ItemFace::East,
            SideDirection::Right => ItemFace::West,
        }
    }

    fn is_horizontal(self) -> bool {
        matches!(self, SideDirection::Up | SideDirection::Down)
    }
}

const SIDE_DIRECTIONS: [SideDirection; 4] = [
    SideDirection::Up,
    SideDirection::Down,
    SideDirection::Left,
    SideDirection::Right,
];

/// Bakes a generated (flat) item layer — its alpha `mask` and atlas `rect` — into the slab's item-model
/// quads, tinted by `tint`. Produces the full front + back faces plus one edge quad per exposed pixel
/// border, exactly as vanilla's `ItemModelGenerator.bakeExtrudedSprite`.
pub fn bake_generated_item_quads(
    mask: &SpriteAlphaMask,
    rect: ItemSpriteRect,
    tint: [f32; 4],
) -> Vec<ItemModelQuad> {
    let mut quads = Vec::new();
    // Front (SOUTH) over the whole sprite, then back (NORTH) with its U mirrored.
    quads.push(bake_face(
        [0.0, 0.0, MIN_Z],
        [16.0, 16.0, MAX_Z],
        [0.0, 0.0, 16.0, 16.0],
        ItemFace::South,
        rect,
        tint,
    ));
    quads.push(bake_face(
        [0.0, 0.0, MIN_Z],
        [16.0, 16.0, MAX_Z],
        [16.0, 0.0, 0.0, 16.0],
        ItemFace::North,
        rect,
        tint,
    ));
    bake_side_faces(&mut quads, mask, rect, tint);
    quads
}

/// Vanilla `bakeSideFaces`: walk the silhouette and emit a `1px` edge quad for every opaque pixel that
/// borders a transparent one.
fn bake_side_faces(
    quads: &mut Vec<ItemModelQuad>,
    mask: &SpriteAlphaMask,
    rect: ItemSpriteRect,
    tint: [f32; 4],
) {
    let x_scale = 16.0 / mask.width as f32;
    let y_scale = 16.0 / mask.height as f32;
    for (side, px, py) in side_faces(mask) {
        let x = px as f32;
        let y = py as f32;
        let u0 = x + UV_SHRINK;
        let u1 = x + 1.0 - UV_SHRINK;
        let (v0, v1) = if side.is_horizontal() {
            (y + UV_SHRINK, y + 1.0 - UV_SHRINK)
        } else {
            (y + 1.0 - UV_SHRINK, y + UV_SHRINK)
        };

        let (mut start_x, mut start_y, mut end_x, mut end_y) = (x, y, x, y);
        match side {
            SideDirection::Up => end_x += 1.0,
            SideDirection::Down => {
                end_x += 1.0;
                start_y += 1.0;
                end_y += 1.0;
            }
            SideDirection::Left => end_y += 1.0,
            SideDirection::Right => {
                start_x += 1.0;
                end_x += 1.0;
                end_y += 1.0;
            }
        }
        start_x *= x_scale;
        end_x *= x_scale;
        start_y *= y_scale;
        end_y *= y_scale;
        start_y = 16.0 - start_y;
        end_y = 16.0 - end_y;

        let (from, to) = match side {
            SideDirection::Up => ([start_x, start_y, MIN_Z], [end_x, start_y, MAX_Z]),
            SideDirection::Down => ([start_x, end_y, MIN_Z], [end_x, end_y, MAX_Z]),
            SideDirection::Left => ([start_x, start_y, MIN_Z], [start_x, end_y, MAX_Z]),
            SideDirection::Right => ([end_x, start_y, MIN_Z], [end_x, end_y, MAX_Z]),
        };
        let uvs = [u0 * x_scale, v0 * y_scale, u1 * x_scale, v1 * y_scale];
        quads.push(bake_face(from, to, uvs, side.face(), rect, tint));
    }
}

/// Vanilla `getSideFaces`: every opaque pixel that borders a transparent pixel (or the sprite edge)
/// contributes a side face on that border. Uses frame 0 of the sprite.
fn side_faces(mask: &SpriteAlphaMask) -> Vec<(SideDirection, i32, i32)> {
    let mut faces = Vec::new();
    for y in 0..mask.height as i32 {
        for x in 0..mask.width as i32 {
            if mask.is_transparent(x, y) {
                continue;
            }
            for side in SIDE_DIRECTIONS {
                let (dx, dy) = side.neighbor_offset();
                if mask.is_transparent(x + dx, y + dy) {
                    faces.push((side, x, y));
                }
            }
        }
    }
    faces
}

/// Vanilla `FaceBakery.bakeQuad` for the identity model state: select the four `FaceInfo` corners from
/// `from`/`to` (model space `0..=16`) and assign the `CuboidFace` UVs (`R0` rotation), mapped from the
/// `0..=16` sprite UVs into the atlas `rect`.
fn bake_face(
    from: [f32; 3],
    to: [f32; 3],
    uvs: [f32; 4],
    facing: ItemFace,
    rect: ItemSpriteRect,
    tint: [f32; 4],
) -> ItemModelQuad {
    let select = |pick: [bool; 3]| {
        [
            if pick[0] { to[0] } else { from[0] },
            if pick[1] { to[1] } else { from[1] },
            if pick[2] { to[2] } else { from[2] },
        ]
    };
    let corners = facing.corner_extents().map(select);

    // CuboidFace.UVs(minU, minV, maxU, maxV), R0 vertex assignment: U = min,min,max,max; V = min,max,max,min.
    let [min_u, min_v, max_u, max_v] = uvs;
    let raw_uvs = [
        [min_u, min_v],
        [min_u, max_v],
        [max_u, max_v],
        [max_u, min_v],
    ];
    let mapped = raw_uvs.map(|[u, v]| rect.map(u / 16.0, v / 16.0));

    ItemModelQuad {
        corners,
        uvs: mapped,
        tint,
        normal: facing.normal(),
        shade: facing.shade(),
        translucent: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item_models::{bake_item_model_mesh, ItemModelMesh};
    use glam::{Mat4, Vec3};

    /// An atlas rect that maps `0..=1` sprite UVs straight through (identity), so UV asserts read as the
    /// raw `0..=1` sprite coordinates.
    const UNIT_RECT: ItemSpriteRect = ItemSpriteRect {
        min: [0.0, 0.0],
        max: [1.0, 1.0],
    };
    const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

    fn full_16x16() -> SpriteAlphaMask {
        SpriteAlphaMask::new(16, 16, vec![true; 256])
    }

    #[test]
    fn front_and_back_span_the_whole_slab() {
        let quads = bake_generated_item_quads(&full_16x16(), UNIT_RECT, WHITE);
        // Front (SOUTH) at z = 8.5, back (NORTH) at z = 7.5.
        let front = quads[0];
        let back = quads[1];
        assert!(front.corners.iter().all(|c| c[2] == MAX_Z));
        assert!(back.corners.iter().all(|c| c[2] == MIN_Z));
        assert_eq!(front.normal, [0.0, 0.0, 1.0]);
        assert_eq!(back.normal, [0.0, 0.0, -1.0]);
        assert_eq!(front.shade, 0.8);
        assert_eq!(back.shade, 0.8);
        // The front face covers the full 0..=16 sprite plane.
        assert_eq!(
            front.corners,
            [
                [0.0, 16.0, MAX_Z],
                [0.0, 0.0, MAX_Z],
                [16.0, 0.0, MAX_Z],
                [16.0, 16.0, MAX_Z]
            ]
        );
        // Front UVs: top-left sprite (0,0) at the top corner, reading down the sprite.
        assert_eq!(front.uvs[0], [0.0, 0.0]);
        assert_eq!(front.uvs[2], [1.0, 1.0]);
        // Back face mirrors U so the texture reads correctly from behind.
        assert_eq!(back.uvs[0], [1.0, 0.0]);
        assert_eq!(back.uvs[2], [0.0, 1.0]);
    }

    #[test]
    fn a_full_sprite_traces_only_its_outer_border() {
        let quads = bake_generated_item_quads(&full_16x16(), UNIT_RECT, WHITE);
        // 2 flat faces + the outline: every edge pixel of a 16×16 solid borders the sprite edge, so each
        // of the four sides contributes 16 edge quads (corners contribute to two sides).
        let edge_quads = quads.len() - 2;
        assert_eq!(edge_quads, 16 * 4);
    }

    #[test]
    fn a_single_opaque_pixel_extrudes_four_side_faces() {
        // One opaque pixel in the center: all four neighbours transparent → four edge quads + 2 flats.
        let mut opaque = vec![false; 256];
        opaque[8 * 16 + 8] = true;
        let mask = SpriteAlphaMask::new(16, 16, opaque);
        let quads = bake_generated_item_quads(&mask, UNIT_RECT, WHITE);
        assert_eq!(quads.len(), 2 + 4);

        // The side quads span the slab depth and carry their direction's shade.
        let sides = &quads[2..];
        assert!(sides.iter().all(|q| {
            let zs: Vec<f32> = q.corners.iter().map(|c| c[2]).collect();
            zs.contains(&MIN_Z) && zs.contains(&MAX_Z)
        }));
        // Up→getShade(UP)=1.0, Down→0.5, Left→East=0.6, Right→West=0.6 appear among the four sides.
        let mut shades: Vec<f32> = sides.iter().map(|q| q.shade).collect();
        shades.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(shades, vec![0.5, 0.6, 0.6, 1.0]);
        let normals: Vec<[f32; 3]> = sides.iter().map(|q| q.normal).collect();
        assert!(normals.contains(&[0.0, 1.0, 0.0]));
        assert!(normals.contains(&[0.0, -1.0, 0.0]));
        assert!(normals.contains(&[1.0, 0.0, 0.0]));
        assert!(normals.contains(&[-1.0, 0.0, 0.0]));
    }

    #[test]
    fn generated_item_mesh_indices_face_submitted_normals_for_default_cull() {
        let mut opaque = vec![false; 256];
        opaque[8 * 16 + 8] = true;
        let mask = SpriteAlphaMask::new(16, 16, opaque);
        let quads = bake_generated_item_quads(&mask, UNIT_RECT, WHITE);
        let mesh = bake_item_model_mesh(&quads, Mat4::IDENTITY);

        assert_mesh_triangles_face_submitted_normals(&mesh);
    }

    #[test]
    fn empty_sprite_bakes_only_the_two_flat_faces() {
        let mask = SpriteAlphaMask::new(16, 16, vec![false; 256]);
        let quads = bake_generated_item_quads(&mask, UNIT_RECT, WHITE);
        assert_eq!(quads.len(), 2);
    }

    #[test]
    fn rect_maps_sprite_uvs_into_the_atlas() {
        let rect = ItemSpriteRect {
            min: [0.25, 0.5],
            max: [0.5, 1.0],
        };
        let quads = bake_generated_item_quads(&full_16x16(), rect, WHITE);
        // Front top-left UV (0,0 sprite) → rect min; bottom-right (1,1 sprite) → rect max.
        assert_eq!(quads[0].uvs[0], [0.25, 0.5]);
        assert_eq!(quads[0].uvs[2], [0.5, 1.0]);
    }

    #[test]
    fn tint_passes_through_to_every_quad() {
        let tint = [0.2, 0.4, 0.6, 1.0];
        let quads = bake_generated_item_quads(&full_16x16(), UNIT_RECT, tint);
        assert!(quads.iter().all(|q| q.tint == tint));
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
