use super::*;
use crate::world::simulation_level::LevelCtx;
use bbb_server_types::block_queries::DIRECTION_VALUES;
use bbb_server_types::directions::relative;

const ORE_POS: BlockPos = BlockPos { x: 8, y: 64, z: 8 };

fn ore_state(deepslate: bool, lit: bool) -> i32 {
    let block = if deepslate {
        bbb_protocol::block_types::VANILLA_BLOCK_DEEPSLATE_REDSTONE_ORE_ID
    } else {
        bbb_protocol::block_types::VANILLA_BLOCK_REDSTONE_ORE_ID
    };
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(block).unwrap();
    crate::fluid::with_property(state, "lit", if lit { "true" } else { "false" }).unwrap()
}

fn install_ore(
    level: &mut LevelCtx<'_, '_>,
    deepslate: bool,
    lit: bool,
    covered_faces: usize,
) -> i32 {
    let gold = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_GOLD_BLOCK_ID,
    )
    .unwrap();
    let air = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_AIR_ID,
    )
    .unwrap();
    for (index, direction) in DIRECTION_VALUES.into_iter().enumerate() {
        assert!(level
            .write_block(
                relative(ORE_POS, direction),
                if index < covered_faces { gold } else { air }
            )
            .is_some());
    }
    let state = ore_state(deepslate, lit);
    assert!(level.write_block(ORE_POS, state).is_some());
    state
}

#[test]
fn ground_steps_redstone_ore_lights_once_but_lit_interactions_still_draw() {
    for deepslate in [false, true] {
        with_level(|level| {
            let state = install_ore(level, deepslate, false, 2);
            level.random.reseed(92);
            let before_sink = format!("{:?}", level.event_sink);
            assert_eq!(
                LevelCtx::interact_redstone_ore(
                    level,
                    crate::entity::level::EntityBlockContext::new(None, None),
                    state,
                    ORE_POS
                ),
                Some(())
            );
            let lit = ore_state(deepslate, true);
            assert_eq!(level.block_state(ORE_POS), lit);
            let changed = |level: &LevelCtx| {
                level
                    .chunks
                    .changed_blocks()
                    .collect::<std::collections::BTreeMap<_, _>>()
            };
            assert_eq!(changed(level).get(&(8, 64, 8)), Some(&lit));
            assert!(level.changed_block_broadcasts.contains(&(8, 64, 8)));
            let before_changes = changed(level);
            let before_broadcasts = level.changed_block_broadcasts.clone();
            assert_eq!(
                LevelCtx::interact_redstone_ore(
                    level,
                    crate::entity::level::EntityBlockContext::new(None, None),
                    lit,
                    ORE_POS
                ),
                Some(())
            );
            assert_eq!(changed(level), before_changes);
            assert_eq!(*level.changed_block_broadcasts, before_broadcasts);
            assert_eq!(format!("{:?}", level.event_sink), before_sink);
        });
    }
}

#[test]
fn ground_steps_actual_stationary_stored_tick_lights_ore_and_keeps_drawing() {
    for deepslate in [false, true] {
        with_config(crate::tests::datapack_config(), |level| {
            let state = install_ore(level, deepslate, false, 0);
            let at = Vec3d {
                x: 8.5,
                y: 65.0,
                z: 8.5,
            };
            let mut store = batch_store();
            let id = crate::entity::spawn_armor_stand(&mut store, at).unwrap();
            crate::entity::set_spawn_on_ground(store.get_mut(id).unwrap(), &|pos| {
                level.block_collision_boxes(pos)
            });
            store.get_mut(id).unwrap().set_delta_movement(Vec3d {
                x: 0.0,
                y: -0.01,
                z: 0.0,
            });
            level.random.reseed(95);
            for _ in 0..2 {
                assert!(matches!(
                    crate::entity::tick_one_entity(&mut store, level, id),
                    crate::entity::EntityTickOutcome::Complete
                ));
                assert_eq!(store.get(id).unwrap().position(), at);
                assert_eq!(level.block_state(ORE_POS), ore_state(deepslate, true));
            }
            assert_ne!(level.block_state(ORE_POS), state);
        });
    }
}

#[test]
fn ground_steps_actual_slime_tick_scales_live_travel_velocity_only_without_shift() {
    let run = |careful| {
        let mut result = None;
        with_config(crate::tests::datapack_config(), |level| {
            let slime = bbb_protocol::block_state_types::vanilla_block_default_state_id(
                bbb_protocol::block_types::VANILLA_BLOCK_SLIME_BLOCK_ID,
            )
            .unwrap();
            level.write_block(ORE_POS, slime).unwrap();
            let at = Vec3d {
                x: 8.5,
                y: 65.0,
                z: 8.5,
            };
            let mut store = batch_store();
            let id = crate::entity::spawn_armor_stand(&mut store, at).unwrap();
            let entity = store.get_mut(id).unwrap();
            crate::entity::set_spawn_on_ground(entity, &|pos| level.block_collision_boxes(pos));
            entity.set_shared_flag(
                bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
                careful,
            );
            entity.set_delta_movement(Vec3d {
                x: 0.1,
                y: -0.05,
                z: 0.2,
            });
            assert!(matches!(
                crate::entity::tick_one_entity(&mut store, level, id),
                crate::entity::EntityTickOutcome::Complete
            ));
            let entity = store.get(id).unwrap();
            result = Some((entity.position(), entity.delta_movement()));
        });
        result.unwrap()
    };
    let (careful_pos, baseline) = run(true);
    let (pos, slowed) = run(false);
    assert_eq!(pos, careful_pos);
    assert!(baseline.x > 0.0 && baseline.z > 0.0);
    assert!(baseline.y.abs() < 0.1);
    assert!(slowed.y.abs() < 0.1);
    let scale = 0.4 + slowed.y.abs() * 0.2;
    assert_eq!(slowed.x, baseline.x * scale);
    assert_eq!(slowed.z, baseline.z * scale);
}

#[test]
fn ground_steps_actual_stored_tick_decreases_eggs_without_fall_damage() {
    for count in [1, 2] {
        with_config(crate::tests::datapack_config(), |level| {
            let old = super::ground_eggs::install_eggs(level, count);
            let top = bbb_server_types::geometry::static_block_collision_boxes(old)
                .unwrap()
                .iter()
                .map(|shape| shape.max_y)
                .reduce(f64::max)
                .unwrap();
            let at = Vec3d {
                x: 8.5,
                y: 64.0 + top,
                z: 8.5,
            };
            let mut store = batch_store();
            let id = crate::entity::spawn_armor_stand(&mut store, at).unwrap();
            let entity = store.get_mut(id).unwrap();
            crate::entity::set_spawn_on_ground(entity, &|pos| level.block_collision_boxes(pos));
            entity.set_delta_movement(Vec3d {
                x: 0.0,
                y: -0.01,
                z: 0.0,
            });
            let health = entity.living().unwrap().health();
            let seed = (0..1000)
                .find(|seed| GameRandom::with_seed(*seed).next_int(100) == 0)
                .unwrap();
            level.random.reseed(seed);
            assert!(matches!(
                crate::entity::tick_one_entity(&mut store, level, id),
                crate::entity::EntityTickOutcome::Complete
            ));
            assert_eq!(store.get(id).unwrap().position(), at);
            assert_eq!(store.get(id).unwrap().living().unwrap().health(), health);
            assert_eq!(
                level.block_state(ORE_POS),
                super::ground_eggs::expected_state(count)
            );
            super::ground_eggs::assert_egg_outputs(level, old);
        });
    }
}

#[test]
fn ground_steps_actual_magma_tick_hurts_living_body_only_without_shift() {
    for careful in [false, true] {
        with_config(crate::tests::datapack_config(), |level| {
            let magma = bbb_protocol::block_state_types::vanilla_block_default_state_id(
                bbb_protocol::block_types::VANILLA_BLOCK_MAGMA_BLOCK_ID,
            )
            .unwrap();
            level.write_block(ORE_POS, magma).unwrap();
            let at = Vec3d {
                x: 8.5,
                y: 65.0,
                z: 8.5,
            };
            let mut store = batch_store();
            let data = crate::entity::GeneralLivingData::new(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID,
            )
            .unwrap();
            let id = store
                .restore(
                    uuid::Uuid::from_u128(97),
                    bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID,
                    at,
                    crate::entity::EntityData::GeneralLiving(data),
                )
                .unwrap();
            let entity = store.get_mut(id).unwrap();
            crate::entity::set_spawn_on_ground(entity, &|pos| level.block_collision_boxes(pos));
            entity.set_shared_flag(
                bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
                careful,
            );
            entity.set_delta_movement(Vec3d {
                x: 0.0,
                y: -0.01,
                z: 0.0,
            });
            let health = entity.living().unwrap().health();
            assert!(matches!(
                crate::entity::tick_one_entity(&mut store, level, id),
                crate::entity::EntityTickOutcome::Complete
            ));
            let entity = store.get(id).unwrap();
            assert_eq!(entity.position(), at);
            assert_eq!(
                entity.living().unwrap().health(),
                health - if careful { 0.0 } else { 1.0 }
            );
        });
    }
}
