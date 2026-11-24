use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_RAVAGER_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};

use crate::entity::tests::native_level::{state, NativeLevel, DIMENSION};
use crate::entity::{tick_one_entity, EntityIdCounter, EntityStore};

#[test]
fn ravager_walking_into_leaves_breaks_them() {
    let mut native = NativeLevel::new();
    native
        .world
        .load_entity_chunk_for_test(DIMENSION, (0, 0))
        .unwrap();
    let stone = state("minecraft:stone");
    let leaves = state("minecraft:oak_leaves");
    let mut wall = Vec::new();
    for x in 4..=12 {
        for z in 4..=12 {
            native.set(BlockPos { x, y: 99, z }, stone);
        }
    }
    for y in 100..=102 {
        for z in 6..=11 {
            let pos = BlockPos { x: 10, y, z };
            native.set(pos, leaves);
            wall.push(pos);
        }
    }
    let mut store = EntityStore::new(0x170, EntityIdCounter::new());
    let ravager = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_RAVAGER_ID,
            Vec3d {
                x: 9.0,
                y: 100.0,
                z: 8.5,
            },
        )
        .unwrap();
    for _ in 0..10 {
        store.get_mut(ravager).unwrap().set_delta_movement(Vec3d {
            x: 0.3,
            y: 0.0,
            z: 0.0,
        });
        native.with_level(|level| {
            tick_one_entity(&mut store, level, ravager);
        });
    }
    native.with_level(|level| {
        assert!(wall.iter().any(|&pos| level.block_state(pos) != leaves));
    });
}
