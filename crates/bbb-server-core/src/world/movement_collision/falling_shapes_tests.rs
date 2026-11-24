use super::*;
use crate::block_entity::new_block_entity;
use crate::chunk::ChunkStore;
use crate::entity::{Entity, EntityData, EntityIdCounter, EntityStore, FallingBlockData};
use crate::fluid::with_property;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID;
use bbb_protocol::{packets::BlockPos, NbtSummaryValue};

const POS: BlockPos = BlockPos { x: 8, y: 70, z: 8 };
const LOCAL: BlockPos = BlockPos { x: 0, y: 0, z: 0 };

fn state(name: &str) -> i32 {
    vanilla_block_default_state_id(vanilla_block_type_for_resource_id(name).unwrap().id).unwrap()
}

fn chunks() -> ChunkStore {
    let mut chunks = ChunkStore::new(
        crate::world_creation::default_overworld_level_source().unwrap(),
        0,
    );
    chunks.load_column(0, 0);
    chunks
}

fn install(chunks: &mut ChunkStore, state: i32) {
    assert!(chunks.set_block_state(POS.x, POS.y, POS.z, state).is_some());
    chunks.block_entities_mut().remove(POS);
    if let Some(entity) = new_block_entity(POS, state) {
        chunks.block_entities_mut().add_and_register(entity);
    }
}

fn falling(distance: f64) -> Entity {
    let mut store = EntityStore::new(71, EntityIdCounter::new());
    let id = store.spawn(
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d {
            x: 8.5,
            y: 75.0,
            z: 8.5,
        },
        EntityData::FallingBlock(FallingBlockData::new(state("minecraft:sand"), POS)),
    );
    let NbtSummaryValue::Compound(mut record) = store.command_data(id).unwrap() else {
        panic!("the body saves a compound");
    };
    record
        .iter_mut()
        .find(|entry| entry.name == "fall_distance")
        .unwrap()
        .value = NbtSummaryValue::Double(distance.to_bits());
    store.load_command_data(id, &record).unwrap();
    let entity = store.get(id).unwrap().clone();
    assert_eq!(entity.fall_distance(), distance);
    entity
}

#[test]
fn falling_shapes_static_are_local_and_share_the_movement_owner() {
    let mut chunks = chunks();
    let entity = falling(0.0);
    for (name, height) in [
        ("minecraft:stone", Some(1.0)),
        ("minecraft:stone_slab", Some(0.5)),
        ("minecraft:air", None),
    ] {
        install(&mut chunks, state(name));
        let expected = height.map_or_else(Vec::new, |height| {
            vec![Aabb {
                max_y: height,
                ..Aabb::of_block(LOCAL)
            }]
        });
        assert_eq!(
            falling_block_collision_shape(&chunks, &entity, POS, None),
            Some(expected.clone())
        );
        assert_eq!(block_collision_shape(&chunks, POS, None), expected);
    }
}

#[test]
fn falling_shapes_powder_snow_uses_distance_before_falling_context_without_tags() {
    let mut chunks = chunks();
    install(&mut chunks, state("minecraft:powder_snow"));
    assert!(chunks.registry_tags().is_none());
    assert_eq!(
        block_collision_shape_checked(&chunks, POS, None),
        Some(vec![])
    );
    for (distance, height) in [(0.0, 1.0), (2.5, 1.0), (2.500_000_001, f64::from(0.9_f32))] {
        for descending in [false, true] {
            let mut entity = falling(distance);
            entity.set_old_pos_and_rot();
            entity.set_pos(Vec3d {
                x: 8.5,
                y: 10.0,
                z: 8.5,
            });
            entity.set_shared_flag(
                bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
                descending,
            );
            assert_eq!(
                falling_block_collision_shape(&chunks, &entity, POS, None),
                Some(vec![Aabb {
                    max_y: height,
                    ..Aabb::of_block(LOCAL)
                }])
            );
        }
    }
}

#[test]
fn falling_shapes_moving_piston_reads_live_progress_and_noclip() {
    let mut chunks = chunks();
    install(&mut chunks, state("minecraft:moving_piston"));
    let entity = falling(0.0);
    assert!(chunks
        .block_entities_mut()
        .get_mut(POS)
        .unwrap()
        .configure_piston(state("minecraft:stone"), Direction::East, true, false));
    for (time, min_x) in [(None, -1.0), (Some(1), -0.5)] {
        if let Some(time) = time {
            chunks
                .block_entities_mut()
                .get_mut(POS)
                .unwrap()
                .tick_piston_without_entities(time);
        }
        let expected = vec![Aabb {
            min_x,
            max_x: min_x + 1.0,
            ..Aabb::of_block(LOCAL)
        }];
        assert_eq!(
            falling_block_collision_shape(&chunks, &entity, POS, None),
            Some(expected.clone())
        );
        assert_eq!(
            falling_block_collision_shape(&chunks, &entity, POS, Some(Direction::West)),
            Some(expected.clone())
        );
        assert_eq!(block_collision_shape(&chunks, POS, None), expected);
        assert_eq!(
            falling_block_collision_shape(&chunks, &entity, POS, Some(Direction::East)),
            Some(vec![])
        );
    }
    chunks.block_entities_mut().remove(POS);
    assert_eq!(
        falling_block_collision_shape(&chunks, &entity, POS, None),
        Some(vec![])
    );
}

#[test]
fn falling_shapes_shulker_reads_live_lid_and_absent_payload_is_full() {
    let mut chunks = chunks();
    let entity = falling(0.0);
    for (facing, direction) in [
        ("down", Direction::Down),
        ("up", Direction::Up),
        ("north", Direction::North),
        ("south", Direction::South),
        ("west", Direction::West),
        ("east", Direction::East),
    ] {
        install(
            &mut chunks,
            with_property(state("minecraft:shulker_box"), "facing", facing).unwrap(),
        );
        assert_eq!(
            falling_block_collision_shape(&chunks, &entity, POS, None),
            Some(vec![Aabb::of_block(LOCAL)])
        );
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
        let expected = shulker.collision_box(direction);
        assert_ne!(expected, Aabb::of_block(LOCAL));
        assert_eq!(
            falling_block_collision_shape(&chunks, &entity, POS, None),
            Some(vec![expected])
        );
        assert_eq!(block_collision_shape(&chunks, POS, None), vec![expected]);
        assert!(chunks.block_entities_mut().remove(POS));
        assert_eq!(
            falling_block_collision_shape(&chunks, &entity, POS, None),
            Some(vec![Aabb::of_block(LOCAL)])
        );
    }
}
