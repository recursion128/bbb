use super::*;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::{VANILLA_BLOCK_STONE_ID, VANILLA_BLOCK_TYPES};

fn state(block_id: i32) -> i32 {
    vanilla_block_default_state_id(block_id).unwrap()
}

fn movement(y: f64) -> Vec3d {
    Vec3d {
        x: -0.125,
        y,
        z: 3.25,
    }
}

fn assert_components(actual: Vec3d, expected: Vec3d) {
    assert_eq!(actual.x.to_bits(), expected.x.to_bits());
    assert_eq!(actual.y.to_bits(), expected.y.to_bits());
    assert_eq!(actual.z.to_bits(), expected.z.to_bits());
}

#[test]
fn ground_contact_velocity_slime_bounces_only_unsuppressed_downward_motion() {
    for living in [false, true] {
        for suppress in [false, true] {
            for y in [-3.7, -0.0, 0.0, 2.0] {
                let expected_y = if suppress {
                    y * 0.0
                } else if y < 0.0 {
                    -y * if living { 1.0 } else { 0.8 }
                } else {
                    y
                };
                assert_components(
                    movement_after_fall_on(
                        state(VANILLA_BLOCK_SLIME_BLOCK_ID),
                        living,
                        suppress,
                        movement(y),
                    )
                    .unwrap(),
                    movement(expected_y),
                );
            }
        }
    }
}

#[test]
fn ground_contact_velocity_bed_bounces_downward_motion() {
    let bed = VANILLA_BLOCK_TYPES
        .iter()
        .find(|block| {
            block.use_without_item_override == Some(VanillaBlockUseWithoutItemOverride::BedBlock)
        })
        .unwrap();
    for living in [false, true] {
        for suppress in [false, true] {
            for y in [-3.7, 0.0, 2.0] {
                let expected_y = if suppress {
                    y * 0.0
                } else if y < 0.0 {
                    -y * f64::from(0.66_f32) * if living { 1.0 } else { 0.8 }
                } else {
                    y
                };
                assert_components(
                    movement_after_fall_on(state(bed.id), living, suppress, movement(y)).unwrap(),
                    movement(expected_y),
                );
            }
        }
    }
}

#[test]
fn ground_contact_velocity_default_and_honey_zero_y_without_bouncing() {
    for block in [VANILLA_BLOCK_STONE_ID, VANILLA_BLOCK_HONEY_BLOCK_ID] {
        for living in [false, true] {
            for y in [-5.0, 0.0, 4.0] {
                assert_components(
                    movement_after_fall_on(state(block), living, false, movement(y)).unwrap(),
                    movement(y * 0.0),
                );
            }
        }
    }
}
