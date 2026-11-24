use super::*;
use crate::level_event::PendingLevelBroadcast;

const EGG_POS: BlockPos = BlockPos { x: 8, y: 64, z: 8 };

fn egg_state(count: i32) -> i32 {
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_TURTLE_EGG_ID,
    )
    .unwrap();
    crate::fluid::with_property(state, "eggs", &count.to_string()).unwrap()
}

pub(super) fn install_eggs(level: &mut LevelCtx<'_, '_>, count: i32) -> i32 {
    let gold = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_GOLD_BLOCK_ID,
    )
    .unwrap();
    assert!(level
        .write_block(BlockPos { y: 63, ..EGG_POS }, gold)
        .is_some());
    let state = egg_state(count);
    assert!(level.write_block(EGG_POS, state).is_some());
    state
}

pub(super) fn expected_state(count: i32) -> i32 {
    if count == 1 {
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_AIR_ID,
        )
        .unwrap()
    } else {
        egg_state(count - 1)
    }
}

/// A seed whose first draw passes `nextInt(3) == 0`, so a landing breaks an egg.
fn breaking_seed() -> i64 {
    (0..100)
        .find(|seed| GameRandom::with_seed(*seed).next_int(3) == 0)
        .unwrap()
}

pub(super) fn assert_egg_outputs(level: &mut LevelCtx<'_, '_>, old_state: i32) {
    let sounds: Vec<_> = level.event_sink.drain_block_sounds().collect();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].pos, EGG_POS);
    assert_eq!(sounds[0].resource_id, "minecraft:entity.turtle.egg_break");
    assert_eq!(sounds[0].volume, 0.7);
    // `0.9 + nextFloat() * 0.2`.
    assert!((0.9..1.1).contains(&sounds[0].pitch));
    let broadcasts: Vec<_> = level.event_sink.drain_level_broadcasts().collect();
    assert_eq!(
        broadcasts,
        vec![PendingLevelBroadcast::LevelEvent(
            crate::level_event::PendingLevelEvent {
                event_type: 2001,
                pos: EGG_POS,
                data: old_state,
            },
        )]
    );
}

#[test]
fn actual_landing_decreases_one_or_two_eggs_before_precise_hit_ground_event() {
    for count in [1, 2] {
        with_config(crate::tests::datapack_config(), |level| {
            let old_state = install_eggs(level, count);
            let top = bbb_server_types::geometry::static_block_collision_boxes(old_state)
                .unwrap()
                .iter()
                .map(|shape| shape.max_y)
                .reduce(f64::max)
                .unwrap();
            let start = Vec3d {
                x: 8.375,
                y: 69.0,
                z: 8.625,
            };
            let landing = Vec3d {
                y: f64::from(EGG_POS.y) + top,
                ..start
            };
            let mut store = batch_store();
            let id = crate::entity::spawn_armor_stand(&mut store, start).unwrap();
            store.get_mut(id).unwrap().set_delta_movement(Vec3d {
                x: 0.0,
                y: -0.5,
                z: 0.0,
            });
            level.random.reseed(breaking_seed());
            let mut airborne_distance = false;
            let mut landed = false;
            for _ in 0..30 {
                assert!(matches!(
                    crate::entity::tick_one_entity(&mut store, level, id),
                    crate::entity::EntityTickOutcome::Complete
                ));
                let body = store.get(id).unwrap();
                airborne_distance |= body.fall_distance() > 0.0;
                if body.on_ground() {
                    assert_eq!(body.position(), landing);
                    assert_eq!(body.fall_distance(), 0.0);
                    landed = true;
                    break;
                }
            }
            assert!(airborne_distance && landed);
            assert_eq!(level.block_state(EGG_POS), expected_state(count));
            assert_egg_outputs(level, old_state);
        });
    }
}
