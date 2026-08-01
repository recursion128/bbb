use super::super::TerrainFace;

#[derive(Debug, Clone, Copy)]
pub(crate) struct FaceDef {
    pub(crate) face: TerrainFace,
    pub(crate) normal: [f32; 3],
    pub(super) dx: i32,
    pub(super) dy: i32,
    pub(super) dz: i32,
    pub(crate) corners: [[f32; 3]; 4],
}

pub(crate) const FACES: [FaceDef; 6] = [
    FaceDef {
        face: TerrainFace::Down,
        normal: [0.0, -1.0, 0.0],
        dx: 0,
        dy: -1,
        dz: 0,
        corners: [
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
        ],
    },
    FaceDef {
        face: TerrainFace::Up,
        normal: [0.0, 1.0, 0.0],
        dx: 0,
        dy: 1,
        dz: 0,
        corners: [
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ],
    },
    FaceDef {
        face: TerrainFace::North,
        normal: [0.0, 0.0, -1.0],
        dx: 0,
        dy: 0,
        dz: -1,
        corners: [
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0],
        ],
    },
    FaceDef {
        face: TerrainFace::South,
        normal: [0.0, 0.0, 1.0],
        dx: 0,
        dy: 0,
        dz: 1,
        corners: [
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, 0.0, 1.0],
        ],
    },
    FaceDef {
        face: TerrainFace::West,
        normal: [-1.0, 0.0, 0.0],
        dx: -1,
        dy: 0,
        dz: 0,
        corners: [
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
    },
    FaceDef {
        face: TerrainFace::East,
        normal: [1.0, 0.0, 0.0],
        dx: 1,
        dy: 0,
        dz: 0,
        corners: [
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, 1.0, 0.0],
            [1.0, 0.0, 0.0],
        ],
    },
];

pub(crate) const CROSS_FACES: [(TerrainFace, [f32; 3], [[f32; 3]; 4]); 4] = [
    (
        TerrainFace::North,
        [-0.70710677, 0.0, 0.70710677],
        [
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 1.0],
            [1.0, 0.0, 1.0],
        ],
    ),
    (
        TerrainFace::South,
        [0.70710677, 0.0, -0.70710677],
        [
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0],
        ],
    ),
    (
        TerrainFace::West,
        [-0.70710677, 0.0, -0.70710677],
        [
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
    ),
    (
        TerrainFace::East,
        [0.70710677, 0.0, 0.70710677],
        [
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
            [1.0, 1.0, 0.0],
            [1.0, 0.0, 0.0],
        ],
    ),
];

pub(crate) fn box_face_corners(face: TerrainFace, min: [f32; 3], max: [f32; 3]) -> [[f32; 3]; 4] {
    match face {
        TerrainFace::Down => [
            [min[0], min[1], max[2]],
            [max[0], min[1], max[2]],
            [max[0], min[1], min[2]],
            [min[0], min[1], min[2]],
        ],
        TerrainFace::Up => [
            [min[0], max[1], min[2]],
            [max[0], max[1], min[2]],
            [max[0], max[1], max[2]],
            [min[0], max[1], max[2]],
        ],
        TerrainFace::North => [
            [max[0], min[1], min[2]],
            [max[0], max[1], min[2]],
            [min[0], max[1], min[2]],
            [min[0], min[1], min[2]],
        ],
        TerrainFace::South => [
            [min[0], min[1], max[2]],
            [min[0], max[1], max[2]],
            [max[0], max[1], max[2]],
            [max[0], min[1], max[2]],
        ],
        TerrainFace::West => [
            [min[0], min[1], min[2]],
            [min[0], max[1], min[2]],
            [min[0], max[1], max[2]],
            [min[0], min[1], max[2]],
        ],
        TerrainFace::East => [
            [max[0], min[1], max[2]],
            [max[0], max[1], max[2]],
            [max[0], max[1], min[2]],
            [max[0], min[1], min[2]],
        ],
    }
}

pub(super) fn face_uvs_from_crop(face: TerrainFace, uv: [u8; 4], rotation: u8) -> [[f32; 2]; 4] {
    let u0 = uv[0] as f32 / 16.0;
    let v0 = uv[1] as f32 / 16.0;
    let u1 = uv[2] as f32 / 16.0;
    let v1 = uv[3] as f32 / 16.0;
    // `CuboidFace.UVs.getVertexU/V` in Vanilla FaceBakery order. Preserve
    // reversed explicit UV ranges instead of normalizing them away.
    let vanilla_vertex_uvs = [[u0, v0], [u0, v1], [u1, v1], [u1, v0]];
    // BBB keeps a winding chosen for wgpu's viewport convention. Map each BBB
    // corner back to its Vanilla `FaceInfo` vertex before applying Quadrant's
    // `(vertex + shift) % 4` UV rotation.
    let vanilla_vertex_for_corner = match face {
        TerrainFace::Down | TerrainFace::Up => [0, 3, 2, 1],
        TerrainFace::North | TerrainFace::South | TerrainFace::West | TerrainFace::East => {
            [1, 0, 3, 2]
        }
    };
    std::array::from_fn(|corner| {
        vanilla_vertex_uvs
            [(vanilla_vertex_for_corner[corner] + rotation as usize) % vanilla_vertex_uvs.len()]
    })
}
