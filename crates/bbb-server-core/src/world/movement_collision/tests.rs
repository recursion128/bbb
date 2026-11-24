use super::*;
use crate::block_entity::new_block_entity;
use crate::chunk::ChunkStore;

const POS: BlockPos = BlockPos { x: 8, y: 70, z: 8 };

fn state(name: &str, facing: &str) -> i32 {
    let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name).unwrap();
    let default =
        bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id).unwrap();
    crate::fluid::with_property(default, "facing", facing).unwrap()
}

fn fixture(state: i32) -> ChunkStore {
    let mut chunks = ChunkStore::new(
        crate::world_creation::default_overworld_level_source().unwrap(),
        0,
    );
    chunks.load_column(0, 0);
    chunks.set_block_state(POS.x, POS.y, POS.z, state).unwrap();
    chunks
        .block_entities_mut()
        .add_and_register(new_block_entity(POS, state).unwrap());
    chunks
}

#[test]
fn shulker_closing_residue_uses_native_create_before_collision_publication() {
    let mut chunks = fixture(state("minecraft:shulker_box", "east"));
    let shulker = chunks
        .block_entities_mut()
        .get_mut(POS)
        .unwrap()
        .shulker_box_mut()
        .unwrap();
    assert!(shulker.trigger_event(1, 1));
    for _ in 0..3 {
        shulker.tick_animation();
    }
    assert!(shulker.trigger_event(1, 0));
    for _ in 0..3 {
        shulker.tick_animation();
    }
    let raw = shulker.collision_box(Direction::East);
    assert!(raw.max_x > 1.0 && raw.max_x < 1.0 + 1e-7);
    assert_eq!(
        block_collision_shape_checked(&chunks, POS, None),
        Some(vec![Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 })])
    );
}

#[test]
fn shulker_fully_open_keeps_each_facing_extension() {
    for (name, direction, delta) in [
        (
            "down",
            Direction::Down,
            Vec3d {
                x: 0.0,
                y: -0.5,
                z: 0.0,
            },
        ),
        (
            "up",
            Direction::Up,
            Vec3d {
                x: 0.0,
                y: 0.5,
                z: 0.0,
            },
        ),
        (
            "north",
            Direction::North,
            Vec3d {
                x: 0.0,
                y: 0.0,
                z: -0.5,
            },
        ),
        (
            "south",
            Direction::South,
            Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.5,
            },
        ),
        (
            "west",
            Direction::West,
            Vec3d {
                x: -0.5,
                y: 0.0,
                z: 0.0,
            },
        ),
        (
            "east",
            Direction::East,
            Vec3d {
                x: 0.5,
                y: 0.0,
                z: 0.0,
            },
        ),
    ] {
        let mut chunks = fixture(state("minecraft:shulker_box", name));
        let shulker = chunks
            .block_entities_mut()
            .get_mut(POS)
            .unwrap()
            .shulker_box_mut()
            .unwrap();
        assert!(shulker.trigger_event(1, 1));
        for _ in 0..10 {
            shulker.tick_animation();
        }
        let expected = Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 }).expand_towards(delta);
        assert_eq!(shulker.collision_box(direction), expected);
        assert_eq!(
            block_collision_shape_checked(&chunks, POS, None),
            Some(vec![expected])
        );
    }
}

#[test]
fn retracting_source_piston_merges_base_and_head_before_publication() {
    let mut chunks = fixture(state("minecraft:moving_piston", "east"));
    let piston = chunks.block_entities_mut().get_mut(POS).unwrap();
    assert!(piston.configure_piston(
        state("minecraft:piston", "east"),
        Direction::East,
        false,
        true
    ));
    piston.tick_piston_without_entities(1);
    let cube = Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 });
    // PistonHeadBlock.java:40-42: the long rod overlaps the retracting base.
    assert_eq!(
        block_collision_shape_checked(&chunks, POS, None),
        Some(vec![
            Aabb {
                max_x: 0.75,
                ..cube
            },
            Aabb {
                min_x: 1.25,
                max_x: 1.5,
                ..cube
            },
            Aabb {
                min_x: 0.75,
                max_x: 1.25,
                min_y: 0.375,
                max_y: 0.625,
                min_z: 0.375,
                max_z: 0.625
            },
        ])
    );
    assert_eq!(
        block_collision_shape_checked(&chunks, POS, Some(Direction::West)),
        Some(vec![Aabb {
            max_x: 0.75,
            ..cube
        }])
    );
}

#[test]
fn moved_piston_half_progress_retains_translation_and_noclip_refusal() {
    let mut chunks = fixture(state("minecraft:moving_piston", "east"));
    let stone = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
    )
    .unwrap();
    let piston = chunks.block_entities_mut().get_mut(POS).unwrap();
    assert!(piston.configure_piston(stone, Direction::East, true, false));
    piston.tick_piston_without_entities(1);
    let cube = Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 });
    assert_eq!(
        block_collision_shape_checked(&chunks, POS, None),
        Some(vec![cube.moved(Vec3d {
            x: -0.5,
            y: 0.0,
            z: 0.0
        })])
    );
    assert_eq!(
        block_collision_shape_checked(&chunks, POS, Some(Direction::East)),
        Some(Vec::new())
    );
}
