use super::*;
use crate::block_entity::new_block_entity;
use crate::chunk::ChunkStore;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_server_types::geometry::collect_block_collisions;

const POS: BlockPos = BlockPos { x: 8, y: 70, z: 8 };
const QUERY: Aabb = Aabb {
    min_x: 7.9,
    max_x: 7.95,
    min_y: 71.1,
    max_y: 71.2,
    min_z: 8.45,
    max_z: 8.55,
};

fn state(name: &str) -> i32 {
    vanilla_block_default_state_id(vanilla_block_type_for_resource_id(name).unwrap().id).unwrap()
}

fn fixture() -> ChunkStore {
    let mut chunks = ChunkStore::new(
        crate::world_creation::default_overworld_level_source().unwrap(),
        0,
    );
    chunks.load_column(0, 0);
    for x in 5..=10 {
        for y in 68..=74 {
            for z in 6..=10 {
                chunks
                    .set_block_state(x, y, z, state("minecraft:air"))
                    .unwrap();
            }
        }
    }
    chunks
        .set_block_state(POS.x, POS.y, POS.z, state("minecraft:moving_piston"))
        .unwrap();
    chunks
        .block_entities_mut()
        .add_and_register(new_block_entity(POS, state("minecraft:moving_piston")).unwrap());
    assert!(chunks
        .block_entities_mut()
        .get_mut(POS)
        .unwrap()
        .configure_piston(state("minecraft:oak_fence"), Direction::East, true, false));
    chunks
}

fn advance(chunks: &mut ChunkStore, time: i64) {
    chunks
        .block_entities_mut()
        .get_mut(POS)
        .unwrap()
        .tick_piston_without_entities(time);
}

#[test]
fn piston_cursor_horizontal_fence_at_half_progress_hits_real_two_boundary_query() {
    // BlockCollisions.java:85-102; MovingPistonBlock.java:106-113.
    let mut chunks = fixture();
    assert!(block_collision_boxes(&chunks, QUERY).is_empty());
    advance(&mut chunks, 1);
    assert_eq!(moving_piston_at(&chunks, POS), Some(true));
    let expected = Aabb {
        min_x: 7.875,
        max_x: 8.125,
        min_y: 70.0,
        max_y: 71.5,
        min_z: 8.375,
        max_z: 8.625,
    };
    assert_eq!(block_collision_boxes(&chunks, QUERY), vec![expected]);
    assert!(
        collect_block_collisions(QUERY, |pos| block_collision_shape(&chunks, pos, None)).is_empty()
    );
    advance(&mut chunks, 2);
    assert!(block_collision_boxes(&chunks, QUERY).is_empty());
}

#[test]
fn piston_cursor_spawn_support_reads_live_fence_through_static_callback() {
    let mut chunks = fixture();
    advance(&mut chunks, 1);
    let mut store = crate::entity::EntityStore::new(9, crate::entity::EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 7.5,
                y: 71.5,
                z: 8.5,
            },
        )
        .unwrap();
    let mut legacy = store.get(id).unwrap().clone();
    let shapes = |pos| {
        static_collision_shape_with_live_piston(&chunks, pos, stored_block_state(&chunks, pos))
    };
    crate::entity::set_spawn_on_ground(&mut legacy, &shapes);
    assert!(!legacy.is_supported_by(POS));
    let body = store.get_mut(id).unwrap();
    crate::entity::set_spawn_on_ground_with_moving_pistons(body, &shapes, &|pos| {
        moving_piston_at(&chunks, pos).unwrap_or(true)
    });
    assert!(body.on_ground());
    assert!(body.is_supported_by(POS));
}
