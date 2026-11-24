use super::*;
use crate::level_event::{PositionedGameEvent, VanillaGameEvent};

fn gold_state() -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_GOLD_BLOCK_ID,
    )
    .unwrap()
}

fn positioned_events(level: &LevelCtx<'_, '_>) -> Vec<PositionedGameEvent> {
    level
        .event_sink
        .recent_positioned_game_events()
        .copied()
        .collect()
}

#[test]
fn real_level_tick_landing_retains_fractional_hit_ground_position_and_resets_distance() {
    with_config(crate::tests::datapack_config(), |level| {
        let gold = gold_state();
        for x in 7..=9 {
            for z in 7..=9 {
                assert!(level.write_block(BlockPos { x, y: 63, z }, gold).is_some());
            }
        }
        let mut store = batch_store();
        let start = Vec3d {
            x: 8.3125,
            y: 69.0,
            z: 8.6875,
        };
        let id = crate::entity::spawn_armor_stand(&mut store, start).unwrap();
        store.get_mut(id).unwrap().set_delta_movement(Vec3d {
            x: 0.0,
            y: -0.5,
            z: 0.0,
        });
        let mut accumulated = false;
        let mut landed = false;
        for _ in 0..30 {
            assert!(matches!(
                crate::entity::tick_one_entity(&mut store, level, id),
                crate::entity::EntityTickOutcome::Complete
            ));
            let body = store
                .get(id)
                .expect("ordinary fall damage does not destroy the stand");
            accumulated |= body.fall_distance() > 0.0;
            if body.on_ground() {
                landed = true;
                assert_eq!(body.position(), Vec3d { y: 64.0, ..start });
                assert_eq!(body.fall_distance(), 0.0);
                break;
            }
        }
        assert!(
            accumulated,
            "the actual airborne ticks must accumulate distance"
        );
        assert!(landed, "the actual tick must reach the gold floor");
        let hits: Vec<_> = positioned_events(level)
            .into_iter()
            .filter(|event| event.recorded.game_event == VanillaGameEvent::HIT_GROUND)
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].position, Vec3d { y: 64.0, ..start });
        assert_eq!(hits[0].recorded.pos, BlockPos { x: 8, y: 64, z: 8 });
        assert_eq!(hits[0].recorded.source_entity_id, Some(id));
        assert_eq!(hits[0].recorded.affected_state, Some(gold));
    });
}
