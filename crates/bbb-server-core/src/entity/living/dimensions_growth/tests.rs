use super::*;
use crate::entity::tests::native_level::{state, NativeLevel};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
use bbb_protocol::packets::BlockPos;

fn fixture(width: f32, height: f32) -> Box<Entity> {
    let mut store = crate::entity::EntityStore::new(37, crate::entity::EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
        )
        .unwrap();
    let mut entity = store.detach(id).unwrap();
    entity.first_tick = false;
    entity.set_dimensions_and_eye(width, height, 0.3);
    entity
}

fn enclosure(ceiling: i32) -> NativeLevel {
    let mut native = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            native.set(BlockPos { x, y: ceiling, z }, state("minecraft:stone"));
        }
    }
    native
}

#[test]
fn dimensions_growth_gates_preserve_position_and_bounds() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        for gate in [1, 2, 4] {
            let mut entity = fixture(2.0, 2.0);
            let mut previous = [1.0, 1.0];
            match gate {
                1 => entity.set_no_physics(true),
                2 => entity.set_dimensions(4.01, 2.0),
                4 => previous = [2.0, 2.0],
                _ => unreachable!(),
            }
            let before = (entity.position(), entity.bounding_box());
            assert!(!requires_collision(&entity, previous));
            assert_eq!(resolve(&mut entity, previous, level), Some(()));
            assert_eq!((entity.position(), entity.bounding_box()), before);
        }
        assert!(requires_collision(&fixture(4.0, 4.0), [1.0, 1.0]));
    });
}

#[test]
fn dimensions_growth_open_space_moves_the_installed_body() {
    let mut entity = fixture(2.0, 2.0);
    entity.set_pos(Vec3d {
        y: 80.0,
        ..entity.position()
    });
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        assert_eq!(resolve(&mut entity, [1.0, 1.0], level), Some(()));
    });
    assert_eq!(
        entity.position(),
        Vec3d {
            x: 0.5,
            y: 79.5,
            z: 0.5
        }
    );
    assert_eq!(entity.eye_height(), 0.3);
    assert_eq!(entity.bb_width(), 2.0);
    assert_eq!(entity.bounding_box().min_y, 79.5);
}

#[test]
fn dimensions_growth_low_ceiling_keeps_the_body_above_the_floor() {
    let mut native = enclosure(65);
    for [width, height] in [[2.0, 2.0], [2.0, 1.0], [1.0, 2.0]] {
        let mut entity = fixture(width, height);
        let before = (entity.position(), entity.bounding_box());
        native.with_level(|level| {
            assert_eq!(resolve(&mut entity, [1.0, 1.0], level), Some(()));
        });
        assert_eq!((entity.position(), entity.bounding_box()), before);
    }
}

#[test]
fn dimensions_growth_slab_ceiling_resolves_without_sinking_into_the_floor() {
    let mut native = NativeLevel::new();
    let slab = crate::fluid::with_property(state("minecraft:oak_slab"), "type", "top").unwrap();
    for x in -3..=3 {
        for z in -3..=3 {
            native.set(BlockPos { x, y: 65, z }, slab);
        }
    }
    let mut entity = fixture(2.0, 2.0);
    native.with_level(|level| {
        assert_eq!(resolve(&mut entity, [1.0, 1.0], level), Some(()));
    });
    assert!((entity.position().y - 64.000001).abs() < 1.0e-10);
}

#[test]
fn dimensions_growth_armor_stand_keeps_its_position() {
    let mut store = crate::entity::EntityStore::new(37, crate::entity::EntityIdCounter::new());
    let origin = Vec3d {
        x: 0.5,
        y: 80.0,
        z: 0.5,
    };
    let id = crate::entity::spawn_armor_stand(&mut store, origin).unwrap();
    let mut entity = store.detach(id).unwrap();
    entity.first_tick = false;
    entity.set_no_physics(false);
    entity.set_dimensions_and_eye(2.0, 2.0, 0.4);
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        assert!(requires_collision(&entity, [1.0, 1.0]));
        assert_eq!(
            resolve_in_store(&mut entity, [1.0, 1.0], level, Some(&mut store)),
            Some(())
        );
    });
    assert_eq!(entity.position(), origin);
}

#[test]
fn dimensions_growth_slime_and_magma_keep_their_position() {
    for ty in [
        VANILLA_ENTITY_TYPE_SLIME_ID,
        VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
    ] {
        let mut store = crate::entity::EntityStore::new(37, crate::entity::EntityIdCounter::new());
        let origin = Vec3d {
            x: 0.5,
            y: 80.0,
            z: 0.5,
        };
        let id = store.spawn_general_living(ty, origin).unwrap();
        let mut entity = store.detach(id).unwrap();
        entity.first_tick = false;
        entity.set_dimensions_and_eye(2.0, 2.0, 0.3);
        let mut native = NativeLevel::new();
        native.with_level(|level| {
            assert_eq!(
                resolve_in_store(&mut entity, [1.0, 1.0], level, Some(&mut store)),
                Some(())
            );
        });
        assert_eq!(entity.position(), origin);
    }
}

#[test]
fn dimensions_growth_moving_border_uses_the_live_world_clock() {
    for (time, expected_y) in [(0, 79.5), (100, 80.0)] {
        let mut native = NativeLevel::new();
        native.set_border(WorldBorder::from_settings(
            0.0, 0.0, 0.2, 5.0, 5, 300, 4.0, 100, 8.0, 0,
        ));
        native.world.set_game_time_for_test(time);
        native.set(BlockPos { x: 2, y: 80, z: 0 }, state("minecraft:stone"));
        let mut entity = fixture(2.0, 2.0);
        entity.set_pos(Vec3d {
            x: 2.5,
            y: 80.0,
            z: 0.5,
        });
        native.with_level(|level| {
            assert_eq!(resolve(&mut entity, [1.0, 1.0], level), Some(()));
        });
        assert_eq!(entity.position().y, expected_y);
    }
}

#[test]
fn dimensions_growth_final_body_can_extend_across_the_border() {
    let mut native = NativeLevel::new();
    native.set_border(WorldBorder::from_settings(
        0.0, 0.0, 0.2, 5.0, 5, 300, 16.0, 0, 16.0, 0,
    ));
    let mut entity = fixture(2.0, 2.0);
    entity.set_pos(Vec3d {
        x: 7.8,
        y: 80.0,
        z: 0.0,
    });
    native.with_level(|level| {
        assert_eq!(resolve(&mut entity, [1.0, 1.0], level), Some(()));
    });
    assert_eq!(entity.position().y, 79.5);
    assert!(entity.bounding_box().max_x > 8.0);
}

#[test]
fn dimensions_growth_border_shape_bounds_keep_the_margin() {
    let border = WorldBorder::from_settings(0.0, 0.0, 0.2, 5.0, 5, 300, 2.0, 0, 2.0, 0);
    let bounds = Aabb {
        min_x: -1.0,
        min_y: 64.0,
        min_z: -1.0,
        max_x: 1.0,
        max_y: 65.0,
        max_z: 1.0,
    };
    assert!(within_border(&border, bounds, 0));
    assert!(!within_border(
        &border,
        Aabb {
            min_x: -1.0 - 1.0e-12,
            ..bounds
        },
        0
    ));
    assert!(!within_border(
        &border,
        Aabb {
            max_x: 1.0 + 1.0e-5 - 1.0e-13,
            ..bounds
        },
        0
    ));
}
