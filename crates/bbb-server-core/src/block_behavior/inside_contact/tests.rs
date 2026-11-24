use super::*;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_BEE_ID, VANILLA_ENTITY_TYPE_FOX_ID, VANILLA_ENTITY_TYPE_PLAYER_ID,
};

fn state(block: i32) -> i32 {
    vanilla_block_default_state_id(block).unwrap()
}
fn berry(age: &str) -> i32 {
    crate::fluid::with_property(state(VANILLA_BLOCK_SWEET_BERRY_BUSH_ID), "age", age).unwrap()
}
fn body(living: bool, entity_type_id: i32) -> ContactBody {
    ContactBody {
        entity_type_id,
        living,
        weaving: false,
    }
}
fn player() -> ContactBody {
    body(true, VANILLA_ENTITY_TYPE_PLAYER_ID)
}

#[test]
fn inside_contact_berry_slows_and_hurts_only_moving_bodies() {
    let threshold = f64::from(0.003_f32);
    for (movement, hurts) in [
        (
            Vec3d {
                x: threshold,
                y: 0.0,
                z: 0.0,
            },
            true,
        ),
        (
            Vec3d {
                x: 0.0,
                y: 0.0,
                z: -threshold,
            },
            true,
        ),
        (Vec3d::default(), false),
        (
            Vec3d {
                x: threshold - 1.0e-12,
                y: 100.0,
                z: 0.0,
            },
            false,
        ),
    ] {
        assert_eq!(sweet_berry_bush::movement_hurts(movement), hurts);
    }
    assert_eq!(
        contact(berry("1"), player()),
        Some(ImmediateContact::SweetBerry {
            hurts_when_moving: true
        })
    );
}

#[test]
fn inside_contact_berry_age_zero_never_hurts_and_exempt_bodies_are_unaffected() {
    assert_eq!(
        contact(berry("0"), player()),
        Some(ImmediateContact::SweetBerry {
            hurts_when_moving: false
        })
    );
    for (living, entity_type) in [
        (false, VANILLA_ENTITY_TYPE_PLAYER_ID),
        (true, VANILLA_ENTITY_TYPE_FOX_ID),
        (true, VANILLA_ENTITY_TYPE_BEE_ID),
    ] {
        assert_eq!(
            contact(berry("3"), body(living, entity_type)),
            Some(ImmediateContact::Resolved)
        );
    }
}

#[test]
fn inside_contact_cactus_hurts_and_wither_rose_withers_only_living() {
    assert_eq!(
        contact(state(VANILLA_BLOCK_CACTUS_ID), player()),
        Some(ImmediateContact::Hurt("minecraft:cactus", 1.0))
    );
    assert_eq!(
        contact(state(VANILLA_BLOCK_WITHER_ROSE_ID), player()),
        Some(ImmediateContact::WitherRose(40))
    );
    assert_eq!(
        contact(
            state(VANILLA_BLOCK_WITHER_ROSE_ID),
            body(false, VANILLA_ENTITY_TYPE_PLAYER_ID)
        ),
        Some(ImmediateContact::Resolved)
    );
}

#[test]
fn inside_contact_cobweb_slowdown_is_halved_by_weaving_for_living() {
    for living in [false, true] {
        for weaving in [false, true] {
            let expected = if living && weaving {
                Vec3d {
                    x: 0.5,
                    y: 0.25,
                    z: 0.5,
                }
            } else {
                Vec3d {
                    x: 0.25,
                    y: f64::from(0.05_f32),
                    z: 0.25,
                }
            };
            assert_eq!(
                contact(
                    state(VANILLA_BLOCK_COBWEB_ID),
                    ContactBody {
                        weaving,
                        ..body(living, VANILLA_ENTITY_TYPE_PLAYER_ID)
                    }
                ),
                Some(ImmediateContact::Stuck(expected))
            );
        }
    }
}

#[test]
fn inside_contact_eyeblossom_affects_only_bees() {
    for block in [
        VANILLA_BLOCK_OPEN_EYEBLOSSOM_ID,
        VANILLA_BLOCK_CLOSED_EYEBLOSSOM_ID,
    ] {
        assert_eq!(
            contact(state(block), body(true, VANILLA_ENTITY_TYPE_BEE_ID)),
            Some(ImmediateContact::BeeEyeblossom(25))
        );
        assert_eq!(
            contact(state(block), player()),
            Some(ImmediateContact::Resolved)
        );
    }
}
