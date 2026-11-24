//! `PointedDripstoneBlock.maybeTransferFluid` turning mud into clay.

use super::*;

use crate::entity::{ArmorStandData, EntityData};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

/// PointedDripstoneBlock: mud dried to clay by a stalactite
/// lifts what stands on it by the two pixels clay adds.
#[test]
fn mud_dried_into_clay_pushes_the_entity_on_it_up() {
    let (sender, receiver, mut world) = helpers::joined_world(helpers::operator_datapack_config());
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule random_tick_speed 4096",
    );
    let mud = BlockPos { x: 2, y: 152, z: 2 };
    let tip = BlockPos { y: 150, ..mud };
    let dripstone = helpers::state_with(
        "minecraft:pointed_dripstone",
        &[
            ("thickness", "tip"),
            ("vertical_direction", "down"),
            ("waterlogged", "false"),
        ],
    );
    for (pos, block) in [
        (mud, block_state_id("minecraft:mud")),
        (
            BlockPos { y: 151, ..mud },
            block_state_id("minecraft:stone"),
        ),
        (tip, dripstone),
    ] {
        assert!(world.set_block(DIM, pos, block, SET_BLOCK_UPDATE_ALL));
    }
    let (stand, _) = world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: 2.5,
            y: 152.875,
            z: 2.5,
        },
        EntityData::ArmorStand(ArmorStandData::new().expect("the armor stand has attributes")),
    );
    for (dimension, chunk) in world.pending_runtime_entity_chunks() {
        world
            .complete_runtime_entity_chunk(&dimension, chunk, None)
            .unwrap();
    }
    for _ in 0..400 {
        world.tick(receiver.drain());
        if world.block_state(DIM, mud) == Some(block_state_id("minecraft:clay")) {
            break;
        }
    }
    assert_eq!(
        world.block_state(DIM, mud),
        Some(block_state_id("minecraft:clay"))
    );
    world.tick(receiver.drain());
    let y = world
        .stored_entity_for_test(DIM, stand)
        .unwrap()
        .position()
        .y;
    assert!(y >= 153.0, "{y}");
}
