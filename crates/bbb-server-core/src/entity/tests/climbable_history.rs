use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use bbb_protocol::entity_types::*;

const POS: BlockPos = BlockPos { x: 2, y: 70, z: 3 };

fn fixture(armor_stand: bool) -> (EntityStore, i32, NativeLevel) {
    let mut store = EntityStore::new(57, EntityIdCounter::new());
    let position = Vec3d {
        x: 2.5,
        y: 70.25,
        z: 3.5,
    };
    let id = if armor_stand {
        crate::entity::spawn_armor_stand(&mut store, position).unwrap()
    } else {
        store
            .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position)
            .unwrap()
    };
    store.get_mut(id).unwrap().set_no_gravity(true);
    let mut level = NativeLevel::new();

    level.world.set_block(
        DIMENSION,
        BlockPos {
            x: POS.x,
            y: POS.y,
            z: POS.z,
        },
        state_id("minecraft:ladder"),
        18,
    );
    (store, id, level)
}

fn prime(store: &mut EntityStore, id: i32, level: &mut NativeLevel) {
    let mut body = store.detach(id).unwrap();
    level
        .with_level(|ctx| {
            crate::entity::living::travel_in_air(&mut body, store, ctx, Vec3d::default())
        })
        .unwrap();
    assert_eq!(body.living().unwrap().last_climbable_pos(), Some(POS));
    store.attach(body);
}

#[test]
fn climbable_history_actual_air_travel_records_position_and_negative_query_keeps_it() {
    for armor_stand in [false, true] {
        let (mut store, id, mut level) = fixture(armor_stand);
        assert_eq!(
            store
                .get(id)
                .unwrap()
                .living()
                .unwrap()
                .last_climbable_pos(),
            None
        );
        prime(&mut store, id, &mut level);
        level
            .world
            .set_block(DIMENSION, POS, state_id("minecraft:air"), 18);
        let mut body = store.detach(id).unwrap();
        body.set_pos(Vec3d {
            x: 5.5,
            y: 70.25,
            z: 3.5,
        });
        level
            .with_level(|ctx| {
                crate::entity::living::travel_in_air(&mut body, &mut store, ctx, Vec3d::default())
            })
            .unwrap();
        assert_eq!(body.living().unwrap().last_climbable_pos(), Some(POS));
        level.world.set_block(
            DIMENSION,
            BlockPos { x: 5, y: 70, z: 3 },
            state_id("minecraft:ladder"),
            18,
        );
        level
            .with_level(|ctx| {
                crate::entity::living::travel_in_air(&mut body, &mut store, ctx, Vec3d::default())
            })
            .unwrap();
        assert_eq!(
            body.living().unwrap().last_climbable_pos(),
            Some(BlockPos { x: 5, ..POS })
        );
    }
}

#[test]
fn climbable_history_command_reload_keeps_transient_but_entity_chunk_reload_starts_empty() {
    for armor_stand in [false, true] {
        let (mut store, id, mut level) = fixture(armor_stand);
        prime(&mut store, id, &mut level);
        let NbtSummaryValue::Compound(entries) = store.command_data(id).unwrap() else {
            unreachable!()
        };
        store.load_command_data(id, &entries).unwrap();
        assert_eq!(
            store
                .get(id)
                .unwrap()
                .living()
                .unwrap()
                .last_climbable_pos(),
            Some(POS)
        );
        let chunk = write_entity_chunk(&store, (0, 0)).unwrap();
        let mut reloaded = EntityStore::new(57, EntityIdCounter::new());
        let ids = read_entity_chunk(&mut reloaded, &chunk).unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(
            reloaded
                .get(ids[0])
                .unwrap()
                .living()
                .unwrap()
                .last_climbable_pos(),
            None
        );
    }
}

fn landing_fixture(stand: bool, distance: f64) -> (EntityStore, i32, NativeLevel) {
    let (mut store, id, mut level) = fixture(stand);
    prime(&mut store, id, &mut level);
    let body = store.get_mut(id).unwrap();
    body.set_no_gravity(false);
    body.set_pos(Vec3d {
        x: 0.5,
        y: if distance == 0.0 { 64.0 } else { 64.25 },
        z: 0.5,
    });
    body.fall_distance = distance;
    body.set_delta_movement(Vec3d {
        x: 0.0,
        y: -0.5,
        z: 0.0,
    });
    store.drain_lifecycle_events();
    (store, id, {
        let mut native = NativeLevel::new();
        for x in -3..=3 {
            for z in -3..=3 {
                native.world.set_block(
                    DIMENSION,
                    BlockPos { x, y: 63, z },
                    state_id("minecraft:stone"),
                    18,
                );
            }
        }
        native
    })
}

#[test]
fn climbable_history_real_root_landing_clears_zero_and_positive_distance_for_living_and_stand() {
    for stand in [false, true] {
        for distance in [0.0, 5.0] {
            let (mut store, id, mut level) = landing_fixture(stand, distance);
            let health = store.get(id).unwrap().living().unwrap().health();
            assert!(matches!(
                level.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, id)),
                crate::entity::EntityTickOutcome::Complete
            ));
            let body = store.get(id).unwrap();
            assert_eq!(body.position().y, 64.0);
            assert!(body.on_ground());
            assert_eq!(body.fall_distance(), 0.0);
            assert_eq!(body.living().unwrap().last_climbable_pos(), None);
            // ArmorStand's known hurt refusal still completes the ordinary fall callback.
            assert_eq!(
                body.living().unwrap().health(),
                health - if !stand && distance > 0.0 { 2.0 } else { 0.0 }
            );
        }
    }
}

#[test]
fn climbable_history_checked_landing_retains_flying_and_old_lava_strider_overrides() {
    for kind in [
        VANILLA_ENTITY_TYPE_PARROT_ID,
        VANILLA_ENTITY_TYPE_STRIDER_ID,
    ] {
        let mut store = EntityStore::new(57, EntityIdCounter::new());
        let id = store
            .spawn_general_living(
                kind,
                Vec3d {
                    x: 0.5,
                    y: 64.25,
                    z: 0.5,
                },
            )
            .unwrap();
        let mut body = store.detach(id).unwrap();
        body.living_mut().unwrap().set_last_climbable_pos(Some(POS));
        body.fall_distance = 5.0;
        if kind == VANILLA_ENTITY_TYPE_STRIDER_ID {
            let mut lava = NativeLevel::new();

            for y in 64..=66 {
                lava.world.set_block(
                    DIMENSION,
                    BlockPos { x: 0, y, z: 0 },
                    state_id("minecraft:lava"),
                    18,
                );
            }
            body.first_tick = false;
            lava.with_level(|ctx| {
                crate::entity::environment::update_fluid_interaction(&mut body, ctx)
            });
            assert!(body.is_in_lava());
        }
        let mut level = NativeLevel::new();
        for x in -3..=3 {
            for z in -3..=3 {
                level.world.set_block(
                    DIMENSION,
                    BlockPos { x, y: 63, z },
                    state_id("minecraft:stone"),
                    18,
                );
            }
        }
        let health = body.living().unwrap().health();
        assert_eq!(
            level.with_level(|ctx| crate::entity::movement::move_living_entity(
                &mut body,
                &mut store,
                ctx,
                crate::entity::MoverType::Self_,
                Vec3d {
                    x: 0.0,
                    y: -0.5,
                    z: 0.0
                }
            )),
            Ok(())
        );
        assert_eq!(body.position().y, 64.0);
        assert!(body.on_ground());
        assert_eq!(
            body.living().unwrap().last_climbable_pos(),
            Some(POS),
            "kind={kind}"
        );
        assert_eq!(
            body.fall_distance(),
            if kind == VANILLA_ENTITY_TYPE_STRIDER_ID {
                0.0
            } else {
                5.0
            }
        );
        assert_eq!(body.living().unwrap().health(), health);
    }
}

#[test]
fn climbable_history_second_air_travel_query_records_moved_feet() {
    let (mut store, id, mut level) = fixture(false);
    prime(&mut store, id, &mut level);
    level
        .world
        .set_block(DIMENSION, POS, state_id("minecraft:air"), 18);
    let ladder = vanilla_block_state_id_by_name_and_properties(
        "minecraft:ladder",
        &BTreeMap::from([
            ("facing".into(), "west".into()),
            ("waterlogged".into(), "false".into()),
        ]),
    )
    .unwrap();
    level
        .world
        .set_block(DIMENSION, BlockPos { x: 5, y: 70, z: 3 }, ladder, 18);
    let mut body = store.detach(id).unwrap();
    body.set_pos(Vec3d {
        x: 4.5,
        y: 70.25,
        z: 3.5,
    });
    body.set_delta_movement(Vec3d {
        x: 0.75,
        y: 0.0,
        z: 0.0,
    });
    body.living_mut().unwrap().jumping = true;
    level
        .with_level(|ctx| {
            crate::entity::living::travel_in_air(&mut body, &mut store, ctx, Vec3d::default())
        })
        .unwrap();
    assert_eq!(
        body.position(),
        Vec3d {
            x: 5.25,
            y: 70.25,
            z: 3.5
        }
    );
    assert!(!body.on_ground());
    assert_eq!(
        body.living().unwrap().last_climbable_pos(),
        Some(BlockPos { x: 5, ..POS })
    );
    assert_eq!(body.delta_movement().y, 0.2 * f64::from(0.98_f32));
}
