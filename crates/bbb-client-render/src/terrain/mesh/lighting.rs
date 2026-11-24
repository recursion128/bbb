//! Vanilla `BlockModelLighter` and `LightCoordsUtil` for terrain quads.

use super::super::{block_table::CellRef, TerrainFace, TerrainLight};
use super::TerrainChunkLookup;
use bbb_render_types::VanillaLightCoords;

/// Vanilla packed light coordinates: block in the low 16 bits, sky in the high 16 bits, 16 per
/// level, with the fractional smooth-light values in between.
pub(super) type LightCoords = u32;

pub(super) fn pack(light: TerrainLight) -> LightCoords {
    u32::from(light.block.min(15)) << 4 | u32::from(light.sky.min(15)) << 20
}

fn block(coords: LightCoords) -> u32 {
    coords >> 4 & 15
}

fn sky(coords: LightCoords) -> u32 {
    coords >> 20 & 15
}

/// `LightCoordsUtil.lightCoordsWithEmission`: the element's emission raises both channels and
/// drops the smooth fraction.
pub(super) fn with_emission(coords: LightCoords, emission: u8) -> LightCoords {
    if emission == 0 {
        return coords;
    }
    let emission = u32::from(emission);
    block(coords).max(emission) << 4 | sky(coords).max(emission) << 20
}

/// `LightCoordsUtil.smoothBlend`.
fn smooth_blend(
    mut n1: LightCoords,
    mut n2: LightCoords,
    mut n3: LightCoords,
    center: LightCoords,
) -> LightCoords {
    if sky(center) > 2 || block(center) > 2 {
        for neighbor in [&mut n1, &mut n2, &mut n3] {
            if sky(*neighbor) == 0 {
                *neighbor |= center & 0xFF_0000;
            }
            if block(*neighbor) == 0 {
                *neighbor |= center & 0xFF;
            }
        }
    }
    (n1 + n2 + n3 + center) >> 2 & 0xFF_00FF
}

/// `LightCoordsUtil.smoothWeightedBlend`.
fn smooth_weighted_blend(coords: [LightCoords; 4], weights: [f32; 4]) -> LightCoords {
    let channel = |shift: u32| {
        let value = |i: usize| (coords[i] >> shift & 0xFF) as f32 * weights[i];
        (value(0) + value(1) + value(2) + value(3)) as i32 as u32 & 0xFF
    };
    channel(0) | channel(16) << 16
}

pub(super) fn state_render_light(
    light: TerrainLight,
    state_light_emission: u8,
    emissive_rendering: bool,
) -> TerrainLight {
    let coords = VanillaLightCoords::from_stored(
        light.block,
        light.sky,
        state_light_emission,
        emissive_rendering,
    );
    TerrainLight {
        block: coords.block(),
        sky: coords.sky(),
    }
}

pub(super) fn render_light_for_cell(cell: CellRef<'_>) -> TerrainLight {
    state_render_light(
        cell.light,
        cell.state_light_emission,
        cell.emissive_rendering,
    )
}

pub(super) fn cull_offset(face: TerrainFace) -> (i32, i32, i32) {
    match face {
        TerrainFace::Down => (0, -1, 0),
        TerrainFace::Up => (0, 1, 0),
        TerrainFace::North => (0, 0, -1),
        TerrainFace::South => (0, 0, 1),
        TerrainFace::West => (-1, 0, 0),
        TerrainFace::East => (1, 0, 0),
    }
}

/// `BlockModelLighter.prepareQuadShape` for quad corners in block units.
struct QuadShape {
    min: [f32; 3],
    max: [f32; 3],
    cubic: bool,
    partial: bool,
    /// `faceShape`, indexed by `SizeInfo`.
    size: [f32; 12],
}

fn quad_shape(face: TerrainFace, corners: &[[f32; 3]; 4], collision_full_block: bool) -> QuadShape {
    let mut min = [32.0_f32; 3];
    let mut max = [-32.0_f32; 3];
    for corner in corners {
        for axis in 0..3 {
            min[axis] = min[axis].min(corner[axis]);
            max[axis] = max[axis].max(corner[axis]);
        }
    }
    let size = [
        min[1],
        max[1],
        min[2],
        max[2],
        min[0],
        max[0],
        1.0 - min[1],
        1.0 - max[1],
        1.0 - min[2],
        1.0 - max[2],
        1.0 - min[0],
        1.0 - max[0],
    ];
    let (normal, a, b) = face_axes(face);
    let partial = min[a] >= 1.0E-4 || min[b] >= 1.0E-4 || max[a] <= 0.9999 || max[b] <= 0.9999;
    let boundary = match face {
        TerrainFace::Down | TerrainFace::North | TerrainFace::West => min[normal] < 1.0E-4,
        TerrainFace::Up | TerrainFace::South | TerrainFace::East => max[normal] > 0.9999,
    };
    QuadShape {
        min,
        max,
        cubic: min[normal] == max[normal] && (boundary || collision_full_block),
        partial,
        size,
    }
}

/// The face's normal axis and its two in-plane axes.
fn face_axes(face: TerrainFace) -> (usize, usize, usize) {
    match face {
        TerrainFace::Down | TerrainFace::Up => (1, 0, 2),
        TerrainFace::North | TerrainFace::South => (2, 0, 1),
        TerrainFace::West | TerrainFace::East => (0, 1, 2),
    }
}

/// `prepareQuadFlat`: a culled quad takes light from its cull neighbour, an unculled one from the
/// facing neighbour when cubic and from its own position otherwise; the state stays the block's.
#[allow(clippy::too_many_arguments)]
pub(super) fn flat_quad_light(
    face: TerrainFace,
    corners: &[[f32; 3]; 4],
    cull: Option<TerrainFace>,
    collision_full_block: bool,
    (x, y, z): (i32, i32, i32),
    light: TerrainLight,
    state_light_emission: u8,
    emissive_rendering: bool,
    lookup: &TerrainChunkLookup<'_>,
) -> LightCoords {
    let offset = match cull {
        Some(cull) => Some(cull_offset(cull)),
        None => quad_shape(face, corners, collision_full_block)
            .cubic
            .then(|| cull_offset(face)),
    };
    let light = offset
        .and_then(|(dx, dy, dz)| lookup.cell(x + dx, y + dy, z + dz))
        .map_or(light, |cell| cell.light);
    pack(state_render_light(
        light,
        state_light_emission,
        emissive_rendering,
    ))
}

/// `prepareQuadAmbientOcclusion`: per-corner light coordinates and ambient shade, in the order of
/// `corners`. Baked quads already hold `FaceBakery`'s vertex order; box faces are matched to
/// their `FaceInfo` vertex by position.
pub(super) fn smooth_quad_lighting(
    face: TerrainFace,
    corners: &[[f32; 3]; 4],
    baked_order: bool,
    collision_full_block: bool,
    (x, y, z): (i32, i32, i32),
    render_light: TerrainLight,
    lookup: &TerrainChunkLookup<'_>,
) -> ([LightCoords; 4], [f32; 4]) {
    let shape = quad_shape(face, corners, collision_full_block);
    let direction = cull_offset(face);
    let base = if shape.cubic {
        (x + direction.0, y + direction.1, z + direction.2)
    } else {
        (x, y, z)
    };

    let next = lookup.cell(x + direction.0, y + direction.1, z + direction.2);
    let mut light_center = pack(render_light);
    if let Some(next) = next.filter(|next| shape.cubic || !next.solid_render) {
        light_center = pack(render_light_for_cell(next));
    }
    let cell_at = |(px, py, pz): (i32, i32, i32)| lookup.cell(px, py, pz);
    let sample = |position: (i32, i32, i32)| {
        let cell = cell_at(position);
        (
            cell.map_or(light_center, |cell| pack(render_light_for_cell(cell))),
            cell.map_or(1.0, |cell| f32::from(cell.shade_brightness) / 255.0),
        )
    };
    let offset =
        |(px, py, pz): (i32, i32, i32), (dx, dy, dz): (i32, i32, i32)| (px + dx, py + dy, pz + dz);

    let sides = adjacency_corners(face).map(cull_offset);
    let side = sides.map(|side| sample(offset(base, side)));
    let translucent = sides.map(|side| {
        !cell_at(offset(offset(base, side), direction))
            .is_some_and(|cell| cell.view_blocking && cell.light_dampening != 0)
    });
    // Both sides opaque reuses side 0 for every corner, as Vanilla does.
    let corner = |i: usize, j: usize| {
        if !translucent[i] && !translucent[j] {
            side[0]
        } else {
            sample(offset(offset(base, sides[i]), sides[j]))
        }
    };
    let (c02, c03, c12, c13) = (corner(0, 2), corner(0, 3), corner(1, 2), corner(1, 3));
    let shade_center = sample(base).1;

    let shades = [
        (side[3].1 + side[0].1 + c03.1 + shade_center) * 0.25,
        (side[2].1 + side[0].1 + c02.1 + shade_center) * 0.25,
        (side[2].1 + side[1].1 + c12.1 + shade_center) * 0.25,
        (side[3].1 + side[1].1 + c13.1 + shade_center) * 0.25,
    ];
    let lights = [
        smooth_blend(side[3].0, side[0].0, c03.0, light_center),
        smooth_blend(side[2].0, side[0].0, c02.0, light_center),
        smooth_blend(side[2].0, side[1].0, c12.0, light_center),
        smooth_blend(side[3].0, side[1].0, c13.0, light_center),
    ];

    let remap = ambient_vertex_remap(face);
    let mut vertex_light = [0; 4];
    let mut vertex_shade = [0.0; 4];
    for vertex in 0..4 {
        let (light, shade) = if shape.partial {
            let table = vertex_weights(face)[vertex];
            let weights: [f32; 4] = std::array::from_fn(|pair| {
                shape.size[table[2 * pair]] * shape.size[table[2 * pair + 1]]
            });
            (
                smooth_weighted_blend(lights, weights),
                (shades[0] * weights[0]
                    + shades[1] * weights[1]
                    + shades[2] * weights[2]
                    + shades[3] * weights[3])
                    .clamp(0.0, 1.0),
            )
        } else {
            (lights[vertex], shades[vertex])
        };
        vertex_light[remap[vertex]] = light;
        vertex_shade[remap[vertex]] = shade;
    }

    if baked_order {
        return (vertex_light, vertex_shade);
    }
    let order = corners.map(|corner| face_info_vertex(face, corner, &shape));
    (
        order.map(|vertex| vertex_light[vertex]),
        order.map(|vertex| vertex_shade[vertex]),
    )
}

/// The `FaceInfo` vertex a corner occupies within the quad's bounds.
fn face_info_vertex(face: TerrainFace, corner: [f32; 3], shape: &QuadShape) -> usize {
    let (_, a, b) = face_axes(face);
    let distance = |vertex: usize| {
        let extent = face_info(face)[vertex];
        [a, b]
            .into_iter()
            .map(|axis| {
                let target = if extent[axis] {
                    shape.max[axis]
                } else {
                    shape.min[axis]
                };
                (corner[axis] - target).abs()
            })
            .sum::<f32>()
    };
    (0..4)
        .min_by(|&left, &right| distance(left).total_cmp(&distance(right)))
        .unwrap_or(0)
}

/// `FaceInfo` vertex extents: `true` selects the max bound on that axis.
fn face_info(face: TerrainFace) -> [[bool; 3]; 4] {
    const O: bool = false;
    const I: bool = true;
    match face {
        TerrainFace::Down => [[O, O, I], [O, O, O], [I, O, O], [I, O, I]],
        TerrainFace::Up => [[O, I, O], [O, I, I], [I, I, I], [I, I, O]],
        TerrainFace::North => [[I, I, O], [I, O, O], [O, O, O], [O, I, O]],
        TerrainFace::South => [[O, I, I], [O, O, I], [I, O, I], [I, I, I]],
        TerrainFace::West => [[O, I, O], [O, O, O], [O, O, I], [O, I, I]],
        TerrainFace::East => [[I, I, I], [I, O, I], [I, O, O], [I, I, O]],
    }
}

/// `AdjacencyInfo.corners`.
fn adjacency_corners(face: TerrainFace) -> [TerrainFace; 4] {
    use TerrainFace::{Down, East, North, South, Up, West};
    match face {
        Down => [West, East, North, South],
        Up => [East, West, North, South],
        North => [Up, Down, East, West],
        South => [West, East, Down, Up],
        West => [Up, Down, North, South],
        East => [Down, Up, North, South],
    }
}

/// `AmbientVertexRemap`.
fn ambient_vertex_remap(face: TerrainFace) -> [usize; 4] {
    match face {
        TerrainFace::Down | TerrainFace::South => [0, 1, 2, 3],
        TerrainFace::Up => [2, 3, 0, 1],
        TerrainFace::North | TerrainFace::West => [3, 0, 1, 2],
        TerrainFace::East => [1, 2, 3, 0],
    }
}

/// `AdjacencyInfo.vert0Weights..vert3Weights` as `SizeInfo` indices.
fn vertex_weights(face: TerrainFace) -> [[usize; 8]; 4] {
    const D: usize = 0;
    const U: usize = 1;
    const N: usize = 2;
    const S: usize = 3;
    const W: usize = 4;
    const E: usize = 5;
    const FD: usize = 6;
    const FU: usize = 7;
    const FN: usize = 8;
    const FS: usize = 9;
    const FW: usize = 10;
    const FE: usize = 11;
    match face {
        TerrainFace::Down => [
            [FW, S, FW, FS, W, FS, W, S],
            [FW, N, FW, FN, W, FN, W, N],
            [FE, N, FE, FN, E, FN, E, N],
            [FE, S, FE, FS, E, FS, E, S],
        ],
        TerrainFace::Up => [
            [E, S, E, FS, FE, FS, FE, S],
            [E, N, E, FN, FE, FN, FE, N],
            [W, N, W, FN, FW, FN, FW, N],
            [W, S, W, FS, FW, FS, FW, S],
        ],
        TerrainFace::North => [
            [U, FW, U, W, FU, W, FU, FW],
            [U, FE, U, E, FU, E, FU, FE],
            [D, FE, D, E, FD, E, FD, FE],
            [D, FW, D, W, FD, W, FD, FW],
        ],
        TerrainFace::South => [
            [U, FW, FU, FW, FU, W, U, W],
            [D, FW, FD, FW, FD, W, D, W],
            [D, FE, FD, FE, FD, E, D, E],
            [U, FE, FU, FE, FU, E, U, E],
        ],
        TerrainFace::West => [
            [U, S, U, FS, FU, FS, FU, S],
            [U, N, U, FN, FU, FN, FU, N],
            [D, N, D, FN, FD, FN, FD, N],
            [D, S, D, FS, FD, FS, FD, S],
        ],
        TerrainFace::East => [
            [FD, S, FD, FS, D, FS, D, S],
            [FD, N, FD, FN, D, FN, D, N],
            [FU, N, FU, FN, U, FN, U, N],
            [FU, S, FU, FS, U, FS, U, S],
        ],
    }
}
