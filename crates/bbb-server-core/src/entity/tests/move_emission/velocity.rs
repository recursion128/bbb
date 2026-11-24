use super::*;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN;
use bbb_server_types::geometry::static_block_collision_boxes;

fn fixture(block: &str, item: bool) -> (EntityStore, Box<Entity>, NativeLevel) {
    let state = state_id(block);
    let top = static_block_collision_boxes(state)
        .unwrap()
        .iter()
        .map(|shape| shape.max_y)
        .reduce(f64::max)
        .unwrap();
    let position = Vec3d {
        y: f64::from(GOLD_PLATE_Y) + top,
        ..FEET
    };
    let mut store = EntityStore::new(51, EntityIdCounter::new());
    let id = if item {
        spawn_item_entity(
            &mut store,
            position,
            plain_stack("minecraft:diamond", 1).unwrap(),
        )
        .unwrap()
    } else {
        store
            .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position)
            .unwrap()
    };
    let mut body = store.detach(id).unwrap();
    body.set_delta_movement(Vec3d {
        x: 0.25,
        y: -0.75,
        z: 0.125,
    });
    store.drain_lifecycle_events();
    (store, body, plate(state))
}

fn move_body(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &mut NativeLevel,
    item: bool,
    delta: Vec3d,
) {
    if item {
        level.with_level(|ctx| {
            crate::entity::move_entity(body, store, ctx, crate::entity::MoverType::Self_, delta)
        });
    } else {
        assert_eq!(
            level.with_level(|ctx| crate::entity::movement::move_living_entity(
                body,
                store,
                ctx,
                crate::entity::MoverType::Self_,
                delta
            )),
            Ok(())
        );
    }
}

const DOWN: Vec3d = Vec3d {
    x: 0.0,
    y: -0.1,
    z: 0.0,
};

#[test]
fn slime_and_bed_real_collision_use_living_and_nonliving_bounce_factors() {
    for (block, factor) in [
        ("minecraft:slime_block", 1.0),
        ("minecraft:red_bed", f64::from(0.66_f32)),
    ] {
        for item in [false, true] {
            let (mut store, mut body, mut level) = fixture(block, item);
            move_body(&mut body, &mut store, &mut level, item, DOWN);
            assert!(body.on_ground());
            let expected = 0.75 * factor * if item { 0.8 } else { 1.0 };
            assert_eq!(
                body.delta_movement(),
                Vec3d {
                    x: 0.25,
                    y: expected,
                    z: 0.125
                },
                "{block}, item={item}"
            );
        }
    }
}

#[test]
fn bounce_suppression_uses_shift_flag_independently_of_pose() {
    for block in ["minecraft:slime_block", "minecraft:red_bed"] {
        for (shift, crouching) in [(true, false), (false, true)] {
            let (mut store, mut body, mut level) = fixture(block, false);
            if crouching {
                body.set_pose_id(bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID);
            }
            body.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, shift);
            move_body(&mut body, &mut store, &mut level, false, DOWN);
            let expected = if shift {
                0.0
            } else if block == "minecraft:red_bed" {
                0.75 * f64::from(0.66_f32)
            } else {
                0.75
            };
            assert_eq!(body.delta_movement().y, expected);
        }
    }
}

#[test]
fn unsuppressed_slime_and_bed_keep_nonnegative_velocity_after_downward_collision() {
    for block in ["minecraft:slime_block", "minecraft:red_bed"] {
        for y in [0.0, 0.75] {
            let (mut store, mut body, mut level) = fixture(block, false);
            body.set_delta_movement(Vec3d {
                y,
                ..body.delta_movement()
            });
            move_body(&mut body, &mut store, &mut level, false, DOWN);
            assert!(body.on_ground());
            assert_eq!(body.delta_movement().y, y);
        }
    }
}

#[test]
fn default_and_honey_zero_vertical_velocity_on_collision() {
    // Honey's `speedFactor` 0.4 scales the horizontal step (`Entity.java:1021-1029`).
    for (block, factor) in [
        ("minecraft:gold_block", 1.0),
        ("minecraft:honey_block", 0.4),
    ] {
        let (mut store, mut body, mut level) = fixture(block, false);
        move_body(&mut body, &mut store, &mut level, false, DOWN);
        assert!(body.on_ground());
        let factor = f64::from(factor as f32);
        assert_eq!(
            body.delta_movement(),
            Vec3d {
                x: 0.25 * factor,
                y: 0.0,
                z: 0.125 * factor
            }
        );
    }
}

#[test]
fn no_vertical_collision_does_not_run_ground_velocity_response() {
    for block in [
        "minecraft:slime_block",
        "minecraft:red_bed",
        "minecraft:gold_block",
    ] {
        let (mut store, mut body, mut level) = fixture(block, false);
        let before = body.delta_movement();
        move_body(
            &mut body,
            &mut store,
            &mut level,
            false,
            Vec3d {
                x: 0.0,
                y: 0.25,
                z: 0.0,
            },
        );
        assert!(!body.on_ground());
        assert_eq!(body.delta_movement(), before);
    }
}

#[test]
fn horizontal_collision_axes_are_cleared_before_bed_bounce() {
    let (mut store, mut body, mut level) = fixture("minecraft:red_bed", false);
    level.set(BlockPos { x: 1, y: 64, z: 0 }, state_id("minecraft:stone"));
    level.set(BlockPos { x: 0, y: 64, z: 1 }, state_id("minecraft:stone"));
    move_body(
        &mut body,
        &mut store,
        &mut level,
        false,
        Vec3d {
            x: 0.5,
            y: -0.1,
            z: 0.5,
        },
    );
    assert!(body.on_ground());
    assert!(body.position().x < 1.0 && body.position().z < 1.0);
    assert_eq!(
        body.delta_movement(),
        Vec3d {
            x: 0.0,
            y: 0.75 * f64::from(0.66_f32),
            z: 0.0
        }
    );
}

#[test]
fn a_lethal_fall_on_a_bed_removes_the_armor_stand_without_bouncing() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let bed = state_id("minecraft:red_bed");
    let top = static_block_collision_boxes(bed)
        .unwrap()
        .iter()
        .map(|shape| shape.max_y)
        .reduce(f64::max)
        .unwrap();
    let id = crate::entity::spawn_armor_stand(
        &mut store,
        Vec3d {
            y: f64::from(GOLD_PLATE_Y) + top,
            ..FEET
        },
    )
    .unwrap();
    let mut body = store.detach(id).unwrap();
    body.fall_distance = 12.0;
    body.set_delta_movement(Vec3d {
        x: 0.25,
        y: -0.75,
        z: 0.125,
    });
    let before = body.delta_movement();
    let mut level = plate(bed);
    level.install_tags(registry_tag_snapshot(&[(
        "minecraft:damage_type",
        "minecraft:always_kills_armor_stands",
        &["minecraft:fall"],
    )]));
    assert_eq!(
        level.with_level(|ctx| crate::entity::movement::move_living_entity(
            &mut body,
            &mut store,
            ctx,
            crate::entity::MoverType::Self_,
            DOWN
        )),
        Ok(())
    );
    assert_eq!(body.removal_reason(), Some(RemovalReason::Killed));
    assert_eq!(body.delta_movement(), before);
    assert!(!game_events(&level)
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::STEP));
}
