use super::*;
use crate::entity::EntityLifecycleEvent;
use std::sync::OnceLock;

fn key() -> Java26LevelKey {
    Java26BuiltinDimension::Overworld.into()
}
fn player() -> BlockPos {
    BlockPos { x: 0, y: 80, z: 0 }
}
fn world() -> ServerWorld {
    static CONFIG: OnceLock<crate::world::ServerWorldConfig> = OnceLock::new();
    let mut world =
        ServerWorld::new(CONFIG.get_or_init(crate::tests::datapack_config).clone()).unwrap();
    world.admin.set_difficulty(Difficulty::Hard);
    world
        .levels
        .get_mut(&world.require_level_id(&key()))
        .unwrap()
        .random
        .reseed(0);
    world
}

#[test]
fn phantom_group_shares_snap_position_and_anchor() {
    let mut world = world();
    let pos = BlockPos { x: 1, y: 100, z: 1 };
    // Hard spawns `1 + nextInt(4)` phantoms.
    let count = world
        .spawn_phantom_group(&world.require_level_id(&key()), pos, player(), 4.0)
        .unwrap();
    assert!((1..=4).contains(&count));
    let level = world
        .levels
        .get_mut(&world.require_level_id(&key()))
        .unwrap();
    assert!(level.chunks.is_ticking_full(0, 0));
    assert_eq!(level.entities.ids().len(), count);
    for id in level.entities.ids() {
        let entity = level.entities.get(id).unwrap();
        assert_eq!(entity.entity_type_id(), VANILLA_ENTITY_TYPE_PHANTOM_ID);
        assert_eq!(
            entity.position(),
            Vec3d {
                x: 1.5,
                y: 100.0,
                z: 1.5
            }
        );
        assert_eq!(entity.old_position(), entity.position());
        assert_eq!(entity.y_rot(), 0.0);
        assert_eq!(entity.x_rot(), 0.0);
        let NbtSummaryValue::Compound(fields) = level.entities.command_data(id).unwrap() else {
            panic!("compound");
        };
        assert!(fields.iter().any(|entry| entry.name == "anchor_pos"
            && entry.value == NbtSummaryValue::IntArray(vec![1, 105, 1])));
        assert!(fields
            .iter()
            .any(|entry| entry.name == "size" && entry.value == NbtSummaryValue::Int(0)));
    }
    let events = level.entities.drain_lifecycle_events();
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, EntityLifecycleEvent::Added { .. }))
            .count(),
        count
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        EntityLifecycleEvent::Sound { .. } | EntityLifecycleEvent::SoundFromEntity { .. }
    )));
}

#[test]
fn phantom_group_outside_build_height_is_air_without_column_generation() {
    for pos in [
        BlockPos {
            x: 1600,
            y: 1000,
            z: 1600,
        },
        BlockPos {
            x: 1600,
            y: -1000,
            z: 1600,
        },
        BlockPos {
            x: HORIZONTAL_LIMIT,
            y: 80,
            z: 0,
        },
    ] {
        let mut world = world();
        assert!(
            world
                .spawn_phantom_group(&world.require_level_id(&key()), pos, player(), 0.0)
                .unwrap()
                > 0
        );
        let level = &world.levels[&world.require_level_id(&key())];
        assert!(!level.chunks.is_loaded(pos.x >> 4, pos.z >> 4));
        for id in level.entities.ids() {
            assert_eq!(
                level.entities.get(id).unwrap().position().y,
                f64::from(pos.y)
            );
        }
    }
}

#[test]
fn phantom_group_outside_height_still_checks_loaded_void_air_tags() {
    for (member, rejected) in [("minecraft:air", false), ("minecraft:void_air", true)] {
        let mut world = world();
        let tags = crate::registry_tags::RegistryTagSnapshot::new([(
            "minecraft:block".to_string(),
            "minecraft:prevent_mob_spawning_inside".to_string(),
            vec![member.to_string()],
        )]);
        let level = world
            .levels
            .get_mut(&world.require_level_id(&key()))
            .unwrap();
        level
            .chunks
            .install_registry_tags(std::sync::Arc::new(tags));
        let position = BlockPos {
            x: 1600,
            y: 1000,
            z: 1600,
        };
        let count = world
            .spawn_phantom_group(&world.require_level_id(&key()), position, player(), 4.0)
            .unwrap();
        assert_eq!(count == 0, rejected);
        let level = &world.levels[&world.require_level_id(&key())];
        assert!(!level.chunks.is_loaded(100, 100));
        if rejected {
            assert!(level.entities.ids().is_empty());
        }
    }
}
