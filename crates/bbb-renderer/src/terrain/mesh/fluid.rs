use super::super::{
    TerrainCell, TerrainFace, TerrainFluid, TerrainFluidKind, TerrainMaterialClass,
    TerrainRenderShape,
};
use super::TerrainChunkLookup;

pub(super) const FLUID_FACE_OFFSET: f32 = 0.001;

#[derive(Debug, Clone, Copy)]
pub(super) struct FluidCornerHeights {
    north_west: f32,
    north_east: f32,
    south_east: f32,
    south_west: f32,
}

impl FluidCornerHeights {
    pub(super) fn with_top_offset(self, offset: f32) -> Self {
        if offset == 0.0 {
            return self;
        }
        Self {
            north_west: (self.north_west - offset).max(0.0),
            north_east: (self.north_east - offset).max(0.0),
            south_east: (self.south_east - offset).max(0.0),
            south_west: (self.south_west - offset).max(0.0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct FluidFlow {
    x: f32,
    z: f32,
}

pub(super) fn fluid_corner_heights(
    x: i32,
    y: i32,
    z: i32,
    self_height: f32,
    kind: Option<TerrainFluidKind>,
    lookup: &TerrainChunkLookup<'_>,
) -> FluidCornerHeights {
    if self_height >= 1.0 {
        return FluidCornerHeights {
            north_west: 1.0,
            north_east: 1.0,
            south_east: 1.0,
            south_west: 1.0,
        };
    }

    let north = fluid_height_at(x, y, z - 1, kind, lookup);
    let south = fluid_height_at(x, y, z + 1, kind, lookup);
    let west = fluid_height_at(x - 1, y, z, kind, lookup);
    let east = fluid_height_at(x + 1, y, z, kind, lookup);

    FluidCornerHeights {
        north_west: average_fluid_corner_height(
            self_height,
            north,
            west,
            fluid_height_at(x - 1, y, z - 1, kind, lookup),
        ),
        north_east: average_fluid_corner_height(
            self_height,
            north,
            east,
            fluid_height_at(x + 1, y, z - 1, kind, lookup),
        ),
        south_east: average_fluid_corner_height(
            self_height,
            south,
            east,
            fluid_height_at(x + 1, y, z + 1, kind, lookup),
        ),
        south_west: average_fluid_corner_height(
            self_height,
            south,
            west,
            fluid_height_at(x - 1, y, z + 1, kind, lookup),
        ),
    }
}

pub(super) fn fluid_face_corners(
    face: TerrainFace,
    min: [f32; 3],
    max: [f32; 3],
    heights: FluidCornerHeights,
    bottom_offset: f32,
) -> [[f32; 3]; 4] {
    let bottom = min[1] + bottom_offset;
    match face {
        TerrainFace::Down => [
            [min[0], bottom, max[2]],
            [max[0], bottom, max[2]],
            [max[0], bottom, min[2]],
            [min[0], bottom, min[2]],
        ],
        TerrainFace::Up => [
            [min[0], heights.north_west, min[2]],
            [max[0], heights.north_east, min[2]],
            [max[0], heights.south_east, max[2]],
            [min[0], heights.south_west, max[2]],
        ],
        TerrainFace::North => [
            [max[0], bottom, min[2] + FLUID_FACE_OFFSET],
            [max[0], heights.north_east, min[2] + FLUID_FACE_OFFSET],
            [min[0], heights.north_west, min[2] + FLUID_FACE_OFFSET],
            [min[0], bottom, min[2] + FLUID_FACE_OFFSET],
        ],
        TerrainFace::South => [
            [min[0], bottom, max[2] - FLUID_FACE_OFFSET],
            [min[0], heights.south_west, max[2] - FLUID_FACE_OFFSET],
            [max[0], heights.south_east, max[2] - FLUID_FACE_OFFSET],
            [max[0], bottom, max[2] - FLUID_FACE_OFFSET],
        ],
        TerrainFace::West => [
            [min[0] + FLUID_FACE_OFFSET, bottom, min[2]],
            [min[0] + FLUID_FACE_OFFSET, heights.north_west, min[2]],
            [min[0] + FLUID_FACE_OFFSET, heights.south_west, max[2]],
            [min[0] + FLUID_FACE_OFFSET, bottom, max[2]],
        ],
        TerrainFace::East => [
            [max[0] - FLUID_FACE_OFFSET, bottom, max[2]],
            [max[0] - FLUID_FACE_OFFSET, heights.south_east, max[2]],
            [max[0] - FLUID_FACE_OFFSET, heights.north_east, min[2]],
            [max[0] - FLUID_FACE_OFFSET, bottom, min[2]],
        ],
    }
}

pub(super) fn fluid_face_texture_index(
    face: TerrainFace,
    texture_indices: [u32; 6],
    has_top_flow: bool,
) -> u32 {
    if matches!(face, TerrainFace::Up) && has_top_flow {
        texture_indices[TerrainFace::North.index()]
    } else {
        texture_indices[face.index()]
    }
}

pub(super) fn fluid_top_flow(
    x: i32,
    y: i32,
    z: i32,
    fluid: TerrainFluid,
    lookup: &TerrainChunkLookup<'_>,
) -> Option<FluidFlow> {
    let own_height = fluid.own_height();
    let mut flow_x = 0.0;
    let mut flow_z = 0.0;
    for (dx, dz) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
        let Some(distance) = fluid_flow_distance(own_height, fluid.kind, x + dx, y, z + dz, lookup)
        else {
            continue;
        };
        if distance != 0.0 {
            flow_x += dx as f32 * distance;
            flow_z += dz as f32 * distance;
        }
    }
    normalized_fluid_flow(flow_x, flow_z)
}

pub(super) fn same_fluid_own_height(cell: &TerrainCell, kind: TerrainFluidKind) -> Option<f32> {
    if let Some(fluid) = cell.fluid {
        return (fluid.kind == kind).then(|| fluid.own_height());
    }
    matches!(cell.material, TerrainMaterialClass::Fluid)
        .then(|| fluid_cell_height(cell))
        .flatten()
}

pub(super) fn fluid_should_render_backward_up_face(
    x: i32,
    y: i32,
    z: i32,
    fluid: TerrainFluid,
    lookup: &TerrainChunkLookup<'_>,
) -> bool {
    for dz in -1..=1 {
        for dx in -1..=1 {
            let cell = lookup.cell(x + dx, y + 1, z + dz);
            let same_fluid = cell
                .and_then(|cell| same_fluid_own_height(cell, fluid.kind))
                .is_some();
            let solid_render = cell
                .map(|cell| material_solid_render(cell.material))
                .unwrap_or(false);
            if !same_fluid && !solid_render {
                return true;
            }
        }
    }
    false
}

pub(super) fn fluid_face_uvs(
    face: TerrainFace,
    heights: FluidCornerHeights,
    top_flow: Option<FluidFlow>,
) -> [[f32; 2]; 4] {
    match face {
        TerrainFace::Up => top_flow.map(fluid_top_flow_uvs).unwrap_or([
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
        ]),
        TerrainFace::Down => [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        TerrainFace::North => [
            [0.5, 0.5],
            [0.5, fluid_side_v(heights.north_east)],
            [0.0, fluid_side_v(heights.north_west)],
            [0.0, 0.5],
        ],
        TerrainFace::South => [
            [0.5, 0.5],
            [0.5, fluid_side_v(heights.south_west)],
            [0.0, fluid_side_v(heights.south_east)],
            [0.0, 0.5],
        ],
        TerrainFace::West => [
            [0.5, 0.5],
            [0.5, fluid_side_v(heights.north_west)],
            [0.0, fluid_side_v(heights.south_west)],
            [0.0, 0.5],
        ],
        TerrainFace::East => [
            [0.5, 0.5],
            [0.5, fluid_side_v(heights.south_east)],
            [0.0, fluid_side_v(heights.north_east)],
            [0.0, 0.5],
        ],
    }
}

pub(super) fn fluid_self_height(
    fluid: Option<TerrainFluid>,
    fallback_height: f32,
    x: i32,
    y: i32,
    z: i32,
    lookup: &TerrainChunkLookup<'_>,
) -> f32 {
    let Some(fluid) = fluid else {
        return if lookup
            .cell(x, y + 1, z)
            .is_some_and(|above| matches!(above.material, TerrainMaterialClass::Fluid))
        {
            1.0
        } else {
            fallback_height
        };
    };
    if lookup
        .cell(x, y + 1, z)
        .is_some_and(|above| same_fluid_own_height(above, fluid.kind).is_some())
    {
        1.0
    } else {
        fluid.own_height()
    }
}

fn fluid_flow_distance(
    own_height: f32,
    kind: TerrainFluidKind,
    x: i32,
    y: i32,
    z: i32,
    lookup: &TerrainChunkLookup<'_>,
) -> Option<f32> {
    let neighbor = lookup.cell(x, y, z);
    let neighbor_height = match neighbor {
        Some(cell) if same_fluid_own_height(cell, kind).is_some() => {
            same_fluid_own_height(cell, kind)?
        }
        Some(cell) if material_blocks_motion(cell.material) => return None,
        Some(_) | None => 0.0,
    };

    if neighbor_height == 0.0 {
        let blocks_motion = neighbor
            .map(|cell| material_blocks_motion(cell.material))
            .unwrap_or(false);
        if blocks_motion {
            return Some(0.0);
        }
        return same_fluid_own_height_at(x, y - 1, z, kind, lookup)
            .filter(|height| *height > 0.0)
            .map(|height| own_height - (height - 0.888_888_9))
            .or(Some(0.0));
    }

    Some(own_height - neighbor_height)
}

fn same_fluid_own_height_at(
    x: i32,
    y: i32,
    z: i32,
    kind: TerrainFluidKind,
    lookup: &TerrainChunkLookup<'_>,
) -> Option<f32> {
    lookup
        .cell(x, y, z)
        .and_then(|cell| same_fluid_own_height(cell, kind))
}

fn material_solid_render(material: TerrainMaterialClass) -> bool {
    matches!(material, TerrainMaterialClass::Opaque)
}

fn material_blocks_motion(material: TerrainMaterialClass) -> bool {
    matches!(material, TerrainMaterialClass::Opaque)
}

fn normalized_fluid_flow(x: f32, z: f32) -> Option<FluidFlow> {
    let length = (x * x + z * z).sqrt();
    (length > 0.0001).then(|| FluidFlow {
        x: x / length,
        z: z / length,
    })
}

fn fluid_top_flow_uvs(flow: FluidFlow) -> [[f32; 2]; 4] {
    let angle = flow.z.atan2(flow.x) - std::f32::consts::FRAC_PI_2;
    let s = angle.sin() * 0.25;
    let c = angle.cos() * 0.25;
    let u00 = 0.5 + (-c - s);
    let v00 = 0.5 + (-c + s);
    let u01 = 0.5 + (-c + s);
    let v01 = 0.5 + (c + s);
    let u10 = 0.5 + (c + s);
    let v10 = 0.5 + (c - s);
    let u11 = 0.5 + (c - s);
    let v11 = 0.5 + (-c - s);

    [[u00, v00], [u11, v11], [u10, v10], [u01, v01]]
}

fn fluid_side_v(height: f32) -> f32 {
    (1.0 - height) * 0.5
}

fn fluid_height_at(
    x: i32,
    y: i32,
    z: i32,
    kind: Option<TerrainFluidKind>,
    lookup: &TerrainChunkLookup<'_>,
) -> f32 {
    let Some(cell) = lookup.cell(x, y, z) else {
        return 0.0;
    };
    if let Some(kind) = kind {
        if let Some(height) = same_fluid_own_height(cell, kind) {
            if lookup
                .cell(x, y + 1, z)
                .is_some_and(|above| same_fluid_own_height(above, kind).is_some())
            {
                return 1.0;
            }
            return height;
        }
    }
    match cell.material {
        TerrainMaterialClass::Fluid => {
            if let Some(kind) = kind {
                if lookup
                    .cell(x, y + 1, z)
                    .is_some_and(|above| same_fluid_own_height(above, kind).is_some())
                {
                    return 1.0;
                }
                return same_fluid_own_height(cell, kind).unwrap_or(0.0);
            }
            if lookup
                .cell(x, y + 1, z)
                .is_some_and(|above| matches!(above.material, TerrainMaterialClass::Fluid))
            {
                return 1.0;
            }
            fluid_cell_height(cell).unwrap_or(1.0)
        }
        TerrainMaterialClass::Opaque => -1.0,
        TerrainMaterialClass::Empty
        | TerrainMaterialClass::Cutout
        | TerrainMaterialClass::Translucent => 0.0,
    }
}

fn fluid_cell_height(cell: &TerrainCell) -> Option<f32> {
    match &cell.render_shape {
        TerrainRenderShape::Box { to, .. } => Some((to[1] as f32 / 16.0).clamp(0.0, 1.0)),
        _ => None,
    }
}

fn average_fluid_corner_height(self_height: f32, side_a: f32, side_b: f32, corner: f32) -> f32 {
    if side_a >= 1.0 || side_b >= 1.0 {
        return 1.0;
    }

    let mut weighted_height = [0.0, 0.0];
    if side_a > 0.0 || side_b > 0.0 {
        if corner >= 1.0 {
            return 1.0;
        }
        add_weighted_fluid_height(&mut weighted_height, corner);
    }

    add_weighted_fluid_height(&mut weighted_height, self_height);
    add_weighted_fluid_height(&mut weighted_height, side_a);
    add_weighted_fluid_height(&mut weighted_height, side_b);
    if weighted_height[1] == 0.0 {
        0.0
    } else {
        weighted_height[0] / weighted_height[1]
    }
}

fn add_weighted_fluid_height(weighted_height: &mut [f32; 2], height: f32) {
    if height >= 0.8 {
        weighted_height[0] += height * 10.0;
        weighted_height[1] += 10.0;
    } else if height >= 0.0 {
        weighted_height[0] += height;
        weighted_height[1] += 1.0;
    }
}
