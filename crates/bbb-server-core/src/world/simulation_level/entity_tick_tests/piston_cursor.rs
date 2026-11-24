use super::*;
use crate::block_entity::new_block_entity;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

const PISTON: BlockPos = BlockPos { x: 9, y: 63, z: 8 };

fn state(name: &str) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
    .unwrap()
}

fn fixture(level: &mut LevelCtx<'_, '_>, ticks: i64, no_clip: bool, present: bool) {
    for x in 6..=12 {
        for z in 6..=10 {
            for y in 62..=72 {
                level
                    .write_block(BlockPos { x, y, z }, state("minecraft:air"))
                    .unwrap();
            }
            level
                .write_block(BlockPos { x, y: 61, z }, state("minecraft:stone"))
                .unwrap();
        }
    }
    let moving = state("minecraft:moving_piston");
    level.write_block(PISTON, moving).unwrap();
    level.chunks.block_entities_mut().remove(PISTON);
    if present {
        let mut be = new_block_entity(PISTON, moving).unwrap();
        assert!(be.configure_piston(state("minecraft:oak_fence"), Direction::East, true, false));
        for tick in 0..ticks {
            be.tick_piston_without_entities(tick);
        }
        assert_eq!(
            be.piston_snapshot().unwrap().progress,
            (ticks as f32 * 0.5).min(1.0)
        );
        level.chunks.block_entities_mut().add_and_register(be);
    }
    LevelCtx::set_piston_no_clip(level, no_clip.then_some(Direction::East));
}

fn spawn(store: &mut EntityStore, stand: bool) -> i32 {
    let start = Vec3d {
        x: if stand { 8.65 } else { 8.5 },
        y: 64.6,
        z: 8.5,
    };
    let id = if stand {
        crate::entity::spawn_armor_stand(store, start).unwrap()
    } else {
        store
            .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, start)
            .unwrap()
    };
    if !stand {
        let NbtSummaryValue::Compound(mut data) = store.command_data(id).unwrap() else {
            unreachable!()
        };
        data.retain(|entry| entry.name != "NoAI");
        data.push(NbtSummaryEntry {
            name: "NoAI".into(),
            value: NbtSummaryValue::Byte(0),
        });
        store.load_command_data(id, &data).unwrap();
    }
    store.get_mut(id).unwrap().set_delta_movement(Vec3d {
        x: 0.0,
        y: -0.5,
        z: 0.0,
    });
    id
}

fn tick(store: &mut EntityStore, level: &mut LevelCtx<'_, '_>, id: i32) {
    assert!(matches!(
        crate::entity::tick_one_entity(store, level, id),
        crate::entity::EntityTickOutcome::Complete
    ));
}

#[test]
fn piston_cursor_actual_tick_horizontal_fence_fragment_clips_and_supports_at_two_edges() {
    // BlockCollisions.java:85-102; PistonMovingBlockEntity.java:386-391.
    for stand in [false, true] {
        for ticks in [0, 1, 2] {
            with_config(crate::tests::datapack_config(), |level| {
                fixture(level, ticks, false, true);
                let mut store = batch_store();
                let id = spawn(&mut store, stand);
                let health = store.get(id).unwrap().living().unwrap().health();
                tick(&mut store, level, id);
                let body = store.get(id).unwrap();
                let hit = ticks < 2;
                assert_eq!(body.tick_count(), 1);
                assert_eq!(body.position().y, if hit { 64.5 } else { 64.1 });
                assert_eq!(body.on_ground(), hit);
                assert_eq!(body.is_supported_by(PISTON), hit);
                assert_eq!(body.living().unwrap().health(), health);
                assert_eq!(
                    level
                        .chunks
                        .block_entities()
                        .get(PISTON)
                        .unwrap()
                        .piston_snapshot()
                        .unwrap()
                        .progress,
                    (ticks as f32 * 0.5).min(1.0)
                );
            });
        }
    }
}

#[test]
fn piston_cursor_actual_tick_noclip_and_missing_be_keep_native_empty_shape() {
    for (no_clip, present) in [(true, true), (false, false)] {
        with_config(crate::tests::datapack_config(), |level| {
            fixture(level, 1, no_clip, present);
            let mut store = batch_store();
            let id = spawn(&mut store, false);
            tick(&mut store, level, id);
            let body = store.get(id).unwrap();
            assert_eq!(body.tick_count(), 1);
            assert_eq!(body.position().y, 64.1);
            assert!(!body.on_ground());
            assert!(!body.is_supported_by(PISTON));
        });
    }
}
