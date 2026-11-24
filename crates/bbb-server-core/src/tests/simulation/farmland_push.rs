//! `FarmlandBlock.turnToDirt` from a block tick lifting what stands on it.

use super::*;

use crate::entity::{ArmorStandData, EntityData};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

/// FarmlandBlock and DirtPathBlock: covered
/// farmland and dirt paths revert on their tick and lift the stand by the
/// pixel dirt adds.
#[test]
fn covered_farmland_and_dirt_path_push_the_entity_on_them_up() {
    let (_sender, receiver, mut world) = helpers::joined_world(helpers::operator_datapack_config());
    let mut stands = Vec::new();
    for (x, block) in [(2, "minecraft:farmland"), (5, "minecraft:dirt_path")] {
        let pos = BlockPos { x, y: 152, z: 2 };
        assert!(world.set_block(
            DIM,
            pos,
            helpers::default_block_state(block),
            SET_BLOCK_UPDATE_ALL
        ));
        let (stand, _) = world.spawn_stored_entity_for_test(
            DIM,
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            Vec3d {
                x: f64::from(x) + 0.5,
                y: 152.9375,
                z: 2.5,
            },
            EntityData::ArmorStand(ArmorStandData::new().expect("the armor stand has attributes")),
        );
        stands.push((pos, stand));
    }
    for (dimension, chunk) in world.pending_runtime_entity_chunks() {
        world
            .complete_runtime_entity_chunk(&dimension, chunk, None)
            .unwrap();
    }
    for (pos, _) in &stands {
        let cover = BlockPos { y: 153, ..*pos };
        assert!(world.set_block(
            DIM,
            cover,
            block_state_id("minecraft:stone"),
            SET_BLOCK_UPDATE_ALL
        ));
    }
    for _ in 0..3 {
        world.tick(receiver.drain());
    }
    for (pos, stand) in stands {
        assert_eq!(
            world.block_state(DIM, pos),
            Some(block_state_id("minecraft:dirt"))
        );
        let y = world
            .stored_entity_for_test(DIM, stand)
            .unwrap()
            .position()
            .y;
        assert!(y >= 153.0, "{y}");
    }
}
