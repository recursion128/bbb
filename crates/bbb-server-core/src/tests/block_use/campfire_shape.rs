use super::*;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const CAMPFIRE: BlockPos = BlockPos { x: 1, y: 80, z: 1 };
const BELOW: BlockPos = BlockPos { y: 79, ..CAMPFIRE };
const EAST: BlockPos = BlockPos { x: 2, ..CAMPFIRE };

fn state(name: &str, signal: bool, waterlogged: bool) -> i32 {
    placement_state(
        name,
        &[
            ("facing", "north"),
            ("lit", if waterlogged { "false" } else { "true" }),
            ("signal_fire", if signal { "true" } else { "false" }),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

#[test]
fn campfire_signal_tracks_hay_below_and_sends_updated_states() {
    for name in ["minecraft:campfire", "minecraft:soul_campfire"] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(datapack_config());
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        stand_at(
            &sender,
            &receiver,
            &mut world,
            1,
            Vec3d {
                x: 1.5,
                y: 80.0,
                z: 3.5,
            },
        );
        assert!(world.set_block(
            DIMENSION,
            BELOW,
            block_state_id("minecraft:stone"),
            SET_BLOCK_UPDATE_ALL
        ));
        assert!(world.set_block(
            DIMENSION,
            CAMPFIRE,
            state(name, false, false),
            SET_BLOCK_UPDATE_ALL
        ));
        world.tick(receiver.drain());

        assert!(world.set_block(
            DIMENSION,
            EAST,
            placement_state("minecraft:hay_block", &[("axis", "y")]),
            SET_BLOCK_UPDATE_ALL
        ));
        assert_eq!(
            live_block(&mut world, CAMPFIRE),
            Some(state(name, false, false))
        );
        world.tick(receiver.drain());
        for (below_state, signal) in [
            (
                placement_state("minecraft:hay_block", &[("axis", "y")]),
                true,
            ),
            (block_state_id("minecraft:air"), false),
        ] {
            assert!(world.set_block(DIMENSION, BELOW, below_state, SET_BLOCK_UPDATE_ALL));
            let expected = state(name, signal, false);
            assert_eq!(live_block(&mut world, CAMPFIRE), Some(expected));
            let tick = world.tick(receiver.drain());
            assert_eq!(block_update_sent(&tick, CAMPFIRE), Some(expected));
            if signal {
                assert!(world.set_block(
                    DIMENSION,
                    EAST,
                    block_state_id("minecraft:air"),
                    SET_BLOCK_UPDATE_ALL
                ));
                assert_eq!(live_block(&mut world, CAMPFIRE), Some(expected));
                world.tick(receiver.drain());
            }
        }
    }
}

#[test]
fn waterlogged_campfire_neighbour_update_spreads_water() {
    for name in ["minecraft:campfire", "minecraft:soul_campfire"] {
        let mut world = spawn_pinned_world(datapack_config());
        world.set_chunks_forced(
            DIMENSION.into(),
            true,
            &[bbb_protocol::packets::ChunkPos { x: 0, z: 0 }],
        );
        for x in 0..=2 {
            for z in 0..=2 {
                assert!(world.set_block(
                    DIMENSION,
                    BlockPos { x, y: 79, z },
                    block_state_id("minecraft:stone"),
                    SET_BLOCK_UPDATE_ALL
                ));
            }
        }
        assert!(world.set_block(
            DIMENSION,
            CAMPFIRE,
            state(name, false, true),
            SET_BLOCK_UPDATE_ALL
        ));
        assert!(world.set_block(
            DIMENSION,
            BlockPos { y: 81, ..CAMPFIRE },
            block_state_id("minecraft:stone"),
            SET_BLOCK_UPDATE_ALL
        ));
        for _ in 0..10 {
            world.tick(Vec::new());
        }
        let east = live_block(&mut world, EAST).unwrap();
        assert!(
            block_state_fluid_state(east).is_same_family(FluidFamily::Water),
            "{name}"
        );
        assert_eq!(
            live_block(&mut world, CAMPFIRE),
            Some(state(name, false, true))
        );
    }
}
