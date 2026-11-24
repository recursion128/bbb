use super::*;
use crate::entity::{EntityData, FallingBlockData};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::{vanilla_block_default_state_id, VANILLA_BLOCK_STATE_COUNT};

const POS: BlockPos = BlockPos { x: 8, y: 65, z: 8 };

fn state(name: &str) -> i32 {
    vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
    .unwrap()
}

fn body(store: &mut EntityStore, block: &str, y: f64) -> i32 {
    store.spawn(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d { x: 8.5, y, z: 8.5 },
        EntityData::FallingBlock(FallingBlockData::new(
            state(block),
            BlockPos {
                y: y.floor() as i32,
                ..POS
            },
        )),
    )
}

#[test]
fn falling_dynamic_every_generated_dynamic_collision_state_has_a_live_body_query() {
    with_config(crate::tests::datapack_config(), |level| {
        let mut store = batch_store();
        let id = body(&mut store, "minecraft:sand", 70.0);
        let mut count = 0;
        for state in 0..VANILLA_BLOCK_STATE_COUNT {
            if bbb_server_types::geometry::static_block_collision_boxes(state).is_some() {
                continue;
            }
            level.write_block(POS, state).unwrap();
            level.chunks.block_entities_mut().remove(POS);
            if let Some(payload) = crate::block_entity::new_block_entity(POS, state) {
                level.chunks.block_entities_mut().add_and_register(payload);
            }
            assert!(
                LevelCtx::block_collision_boxes_for_entity(level, store.get(id).unwrap(), POS)
                    .is_some(),
                "state {state}"
            );
            count += 1;
        }
        assert_eq!(count, 179);
    });
}

#[test]
fn falling_dynamic_actual_tick_scaffolding_uses_start_body_context_and_current_ray_context() {
    for descending in [false, true] {
        with_config(crate::tests::datapack_config(), |level| {
            for y in 62..=68 {
                level.write_block(BlockPos { y, ..POS }, 0).unwrap();
            }
            level
                .write_block(POS, state("minecraft:scaffolding"))
                .unwrap();
            level
                .write_block(BlockPos { y: 64, ..POS }, state("minecraft:water"))
                .unwrap();
            let mut store = batch_store();
            let id = body(&mut store, "minecraft:white_concrete_powder", 67.0);
            let entity = store.get_mut(id).unwrap();
            entity.set_delta_movement(Vec3d {
                x: 0.0,
                y: -3.0,
                z: 0.0,
            });
            entity.set_shared_flag(
                bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
                descending,
            );
            crate::entity::tick_one_entity(&mut store, level, id);
            assert!(store.get(id).is_none());
            assert_eq!(
                level.block_state(BlockPos { y: 66, ..POS }),
                if descending {
                    0
                } else {
                    state("minecraft:white_concrete_powder")
                }
            );
            assert_eq!(
                level.block_state(BlockPos { y: 64, ..POS }),
                state(if descending {
                    "minecraft:white_concrete"
                } else {
                    "minecraft:water"
                })
            );
        });
    }
}

#[test]
fn falling_dynamic_actual_tick_powder_snow_collision_uses_prior_fall_distance() {
    for (distance, height) in [(2.5_f64, 66.0), (2.5000001, 65.0 + f64::from(0.9_f32))] {
        with_config(crate::tests::datapack_config(), |level| {
            for y in 62..=68 {
                level.write_block(BlockPos { y, ..POS }, 0).unwrap();
            }
            level
                .write_block(POS, state("minecraft:powder_snow"))
                .unwrap();
            let mut store = batch_store();
            let id = body(&mut store, "minecraft:sand", 67.0);
            let bbb_protocol::NbtSummaryValue::Compound(mut record) =
                store.command_data(id).unwrap()
            else {
                unreachable!()
            };
            record
                .iter_mut()
                .find(|entry| entry.name == "fall_distance")
                .unwrap()
                .value = bbb_protocol::NbtSummaryValue::Double(distance.to_bits());
            store.load_command_data(id, &record).unwrap();
            store.get_mut(id).unwrap().set_delta_movement(Vec3d {
                x: 0.0,
                y: -3.0,
                z: 0.0,
            });
            crate::entity::tick_one_entity(&mut store, level, id);
            assert!(store.get(id).is_none());
            if height == 66.0 {
                assert!(level.item_spawns.is_empty());
                assert_eq!(
                    level.block_state(BlockPos { y: 66, ..POS }),
                    state("minecraft:sand")
                );
                assert_eq!(level.block_state(POS), state("minecraft:powder_snow"));
            } else {
                assert_eq!(level.item_spawns.len(), 1);
                assert!((level.item_spawns[0].position.y - height).abs() < 1.0e-12);
                assert_eq!(level.block_state(POS), state("minecraft:powder_snow"));
            }
        });
    }
}

#[test]
fn falling_dynamic_moving_piston_resolves_positioned_and_empty_context_moved_shapes() {
    with_config(crate::tests::datapack_config(), |level| {
        let mut store = batch_store();
        let id = body(&mut store, "minecraft:sand", 62.0);
        let moving = state("minecraft:moving_piston");
        level.write_block(POS, moving).unwrap();
        level
            .chunks
            .block_entities_mut()
            .add_and_register(crate::block_entity::new_block_entity(POS, moving).unwrap());
        for name in [
            "minecraft:bamboo",
            "minecraft:pointed_dripstone",
            "minecraft:scaffolding",
            "minecraft:powder_snow",
        ] {
            let state = state(name);
            level
                .chunks
                .block_entities_mut()
                .get_mut(POS)
                .unwrap()
                .configure_piston(state, Direction::East, true, false);
            let expected = if name == "minecraft:scaffolding" {
                bbb_server_types::geometry::scaffolding::collision_boxes(
                    state,
                    POS,
                    bbb_server_types::geometry::scaffolding::Context::Empty,
                )
                .unwrap()
            } else if name == "minecraft:powder_snow" {
                vec![]
            } else {
                bbb_server_types::geometry::positioned_shapes::collision_boxes(state, POS).unwrap()
            };
            let expected: Vec<_> = expected
                .into_iter()
                .map(|shape| {
                    shape.moved(Vec3d {
                        x: -1.0,
                        y: 0.0,
                        z: 0.0,
                    })
                })
                .collect();
            assert_eq!(
                LevelCtx::block_collision_boxes_for_entity(level, store.get(id).unwrap(), POS),
                Some(expected)
            );
            assert!(LevelCtx::clip_collider_source_only(
                level,
                None,
                store.get(id).unwrap(),
                Vec3d {
                    x: 6.0,
                    y: 65.95,
                    z: 8.5
                },
                Vec3d {
                    x: 10.0,
                    y: 65.95,
                    z: 8.5
                }
            )
            .is_some());
        }
    });
}
