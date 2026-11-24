use super::*;
use crate::entity::{EntityData, EntityTickOutcome, FallingBlockData};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;

const START: BlockPos = BlockPos { x: 8, y: 67, z: 8 };
const WATER: BlockPos = BlockPos { y: 65, ..START };

fn state(name: &str) -> i32 {
    vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
    .unwrap()
}

fn source(store: &mut EntityStore) -> i32 {
    let id = store.spawn(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d {
            x: 8.5,
            y: 67.0,
            z: 8.5,
        },
        EntityData::FallingBlock(FallingBlockData::new(
            state("minecraft:white_concrete_powder"),
            START,
        )),
    );
    store.get_mut(id).unwrap().set_delta_movement(Vec3d {
        x: 0.0,
        y: -3.0,
        z: 0.0,
    });
    id
}

#[test]
fn falling_source_clip_actual_tick_crosses_source_but_does_not_clip_flowing_water() {
    for flowing in [false, true] {
        with_config(crate::tests::datapack_config(), |level| {
            for y in 62..=68 {
                level.write_block(BlockPos { y, ..START }, 0).unwrap();
            }
            let water = if flowing {
                crate::fluid::with_property(state("minecraft:water"), "level", "5").unwrap()
            } else {
                state("minecraft:water")
            };
            level.write_block(WATER, water).unwrap();
            let mut store = batch_store();
            let id = source(&mut store);
            assert!(matches!(
                crate::entity::tick_one_entity(&mut store, level, id),
                EntityTickOutcome::Complete
            ));
            if flowing {
                let body = store.get(id).unwrap();
                assert!((body.position().y - 63.96).abs() < 1.0e-12);
                assert_eq!(body.old_position().y, 67.0);
                assert_eq!(level.block_state(WATER), water);
            } else {
                assert!(store.get(id).is_none());
                assert_eq!(level.block_state(WATER), state("minecraft:white_concrete"));
                assert_eq!(level.block_state(BlockPos { y: 63, ..START }), 0);
            }
        });
    }
}

#[test]
fn falling_source_clip_first_block_masks_later_water_and_source_exit_is_a_miss() {
    with_config(crate::tests::datapack_config(), |level| {
        let mut store = batch_store();
        let id = source(&mut store);
        let body = store.get(id).unwrap();
        let from = Vec3d {
            x: 6.5,
            y: 67.5,
            z: 8.5,
        };
        let to = Vec3d { x: 10.5, ..from };
        let stone = BlockPos { x: 7, ..START };
        level.write_block(stone, state("minecraft:stone")).unwrap();
        level.write_block(START, state("minecraft:water")).unwrap();
        assert_eq!(
            LevelCtx::clip_collider_source_only(level, None, body, from, to),
            Some(Some(stone))
        );
        level.write_block(stone, 0).unwrap();
        assert_eq!(
            LevelCtx::clip_collider_source_only(level, None, body, from, to),
            Some(Some(START))
        );
        let from = Vec3d { x: 8.9999, ..from };
        assert_eq!(
            LevelCtx::clip_collider_source_only(level, None, body, from, to),
            Some(None)
        );
    });
}
