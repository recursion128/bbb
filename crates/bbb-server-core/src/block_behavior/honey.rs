use bbb_protocol::packets::{BlockPos, Vec3d};

fn old_delta_y(delta_y: f64) -> f64 {
    delta_y / f64::from(0.98_f32) + 0.08
}

// HoneyBlock.java:84-101.
pub fn is_sliding_down(
    pos: BlockPos,
    position: Vec3d,
    width: f32,
    on_ground: bool,
    delta: Vec3d,
) -> bool {
    if on_ground || position.y > f64::from(pos.y) + 0.9375 - 1.0e-7 {
        return false;
    }
    if old_delta_y(delta.y) >= -0.08 {
        return false;
    }
    let dx = (f64::from(pos.x) + 0.5 - position.x).abs();
    let dz = (f64::from(pos.z) + 0.5 - position.z).abs();
    let overlap = 0.4375 + f64::from(width / 2.0_f32);
    dx + 1.0e-7 > overlap || dz + 1.0e-7 > overlap
}

// HoneyBlock.java:109-119.
pub fn slide_movement(delta: Vec3d) -> Vec3d {
    let old_y = old_delta_y(delta.y);
    let mut movement = delta;
    if old_y < -0.13 {
        let factor = -0.05 / old_y;
        movement.x *= factor;
        movement.z *= factor;
    }
    movement.y = (-0.05 - 0.08) * f64::from(0.98_f32);
    movement
}

// HoneyBlock.java:121-131: whether the slide sound and the slide event play,
// each from its own level draw.
pub fn slide_effects(mut next_int_five: impl FnMut() -> i32) -> (bool, bool) {
    let sound = next_int_five() == 0;
    (sound, next_int_five() == 0)
}

#[cfg(test)]
mod tests;
