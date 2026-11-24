use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::VANILLA_BLOCK_HONEY_BLOCK_ID;
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::packets::Vec3d;

/// Entity.java:1015-1018; Blocks.java:5772-5776.
pub fn body_block_jump_factor(here: i32, below: i32) -> Option<f32> {
    body_block_jump_factor_with_registry(here, below, ContentRegistry::vanilla())
}

pub fn body_block_jump_factor_with_registry(
    here: i32,
    below: i32,
    registry: &ContentRegistry,
) -> Option<f32> {
    let here = block_jump_factor(here, registry)?;
    let below = block_jump_factor(below, registry)?;
    Some(if here == 1.0 { below } else { here })
}

fn block_jump_factor(state: i32, registry: &ContentRegistry) -> Option<f32> {
    if let Some(block_type_id) = vanilla_block_state_block_type_id(state) {
        return Some(if block_type_id == VANILLA_BLOCK_HONEY_BLOCK_ID {
            0.5
        } else {
            1.0
        });
    }
    registry.block_by_state_id(state).map(|_| 1.0)
}

/// LivingEntity.java:2330-2355.
pub fn body_ground_jump_velocity(
    movement: Vec3d,
    jump_strength: f64,
    block_factor: f32,
    boost_amplifier: Option<i32>,
    sprinting: bool,
    y_rot: f32,
) -> Option<Vec3d> {
    let boost = boost_amplifier.map_or(0.0, |amplifier| 0.1_f32 * (amplifier as f32 + 1.0));
    let power = jump_strength as f32 * block_factor + boost;
    if power <= 1.0e-5_f32 {
        return None;
    }
    let mut movement = movement;
    movement.y = if power.is_nan() || movement.y.is_nan() {
        f64::NAN
    } else {
        movement.y.max(f64::from(power))
    };
    if sprinting {
        let angle = y_rot * (std::f64::consts::PI / 180.0) as f32;
        movement.x -= f64::from(bbb_server_types::geometry::mth_sin(f64::from(angle))) * 0.2;
        movement.z += f64::from(bbb_server_types::geometry::mth_cos(f64::from(angle))) * 0.2;
    }
    Some(movement)
}
