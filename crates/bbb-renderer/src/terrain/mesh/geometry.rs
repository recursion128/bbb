use super::super::TerrainFace;

#[derive(Debug, Clone, Copy)]
pub(super) struct FaceDef {
    pub(super) face: TerrainFace,
    pub(super) normal: [f32; 3],
    pub(super) dx: i32,
    pub(super) dy: i32,
    pub(super) dz: i32,
    pub(super) corners: [[f32; 3]; 4],
}

pub(super) const FACES: [FaceDef; 6] = [
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

pub(super) const CROSS_FACES: [(TerrainFace, [f32; 3], [[f32; 3]; 4]); 4] = [
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

pub(super) fn box_face_corners(face: TerrainFace, min: [f32; 3], max: [f32; 3]) -> [[f32; 3]; 4] {
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

pub(super) fn face_uvs_from_crop(uv: [u8; 4]) -> [[f32; 2]; 4] {
    let min_u = uv[0] as f32 / 16.0;
    let min_v = uv[1] as f32 / 16.0;
    let max_u = uv[2] as f32 / 16.0;
    let max_v = uv[3] as f32 / 16.0;
    [
        [min_u, min_v],
        [max_u, min_v],
        [max_u, max_v],
        [min_u, max_v],
    ]
}
