use bbb_client_world::{BlockPos, WorldStore};

const REDSTONE_PARTICLE_AXIS_OFFSET: f64 = 0.5625;
pub(crate) const REDSTONE_PARTICLE_COLOR: u32 = 0x00ff_0000;
pub(crate) const SCULK_PARTICLE_COLOR: u32 = 0x0039_d6e0;

/// Vanilla `Direction.values()` order used by redstone-family block effects.
pub(crate) const REDSTONE_EFFECT_DIRECTION_STEPS: [[i32; 3]; 6] = [
    [0, -1, 0],
    [0, 1, 0],
    [0, 0, -1],
    [0, 0, 1],
    [-1, 0, 0],
    [1, 0, 0],
];

pub(crate) fn redstone_ore_particle_positions(
    pos: BlockPos,
    solid_render_faces: [bool; 6],
    mut next_float: impl FnMut() -> f32,
) -> [Option<[f64; 3]>; 6] {
    let mut positions = [None; 6];
    for (index, (step, solid_render)) in REDSTONE_EFFECT_DIRECTION_STEPS
        .into_iter()
        .zip(solid_render_faces)
        .enumerate()
    {
        if solid_render {
            continue;
        }
        positions[index] = Some([
            f64::from(pos.x) + redstone_ore_coordinate(step[0], &mut next_float),
            f64::from(pos.y) + redstone_ore_coordinate(step[1], &mut next_float),
            f64::from(pos.z) + redstone_ore_coordinate(step[2], &mut next_float),
        ]);
    }
    positions
}

pub(crate) fn redstone_ore_solid_render_faces(world: &WorldStore, pos: BlockPos) -> [bool; 6] {
    REDSTONE_EFFECT_DIRECTION_STEPS.map(|step| {
        world
            .environment_animate_block_sample(BlockPos {
                x: pos.x + step[0],
                y: pos.y + step[1],
                z: pos.z + step[2],
            })
            .is_some_and(|sample| sample.solid_render)
    })
}

fn redstone_ore_coordinate(step: i32, next_float: &mut impl FnMut() -> f32) -> f64 {
    if step == 0 {
        f64::from(next_float())
    } else {
        0.5 + REDSTONE_PARTICLE_AXIS_OFFSET * f64::from(step)
    }
}
