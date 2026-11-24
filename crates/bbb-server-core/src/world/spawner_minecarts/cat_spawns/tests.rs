use super::*;
use crate::entity::EntityLifecycleEvent;
use std::sync::OnceLock;

fn key() -> Java26LevelKey {
    Java26BuiltinDimension::Overworld.into()
}
fn world() -> ServerWorld {
    static CONFIG: OnceLock<crate::world::ServerWorldConfig> = OnceLock::new();
    ServerWorld::new(CONFIG.get_or_init(crate::tests::datapack_config).clone()).unwrap()
}

#[test]
fn cat_spawn_placement_requires_floor_but_flat_hut_is_a_valid_miss() {
    let mut world = world();
    let pos = BlockPos { x: 1, y: 80, z: 1 };
    world
        .levels
        .get_mut(&world.require_level_id(&key()))
        .unwrap()
        .chunks
        .load_column(0, 0);
    assert_eq!(
        world.cat_spawn_position_ok(&world.require_level_id(&key()), pos),
        Ok(false)
    );
    world
        .levels
        .get_mut(&world.require_level_id(&key()))
        .unwrap()
        .chunks
        .set_block_state(
            1,
            79,
            1,
            bbb_protocol::block_state_types::vanilla_block_default_state_id(
                bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
            )
            .unwrap(),
        );
    assert_eq!(
        world.cat_spawn_position_ok(&world.require_level_id(&key()), pos),
        Ok(true)
    );
    assert!(!world.cat_spawn_in_hut(&world.require_level_id(&key()), pos));
}

#[test]
fn cat_spawn_publishes_sound_variant_persistence_and_snap_state() {
    for persistent in [false, true] {
        let mut world = world();
        let pos = BlockPos {
            x: 32,
            y: 90,
            z: 48,
        };
        assert_eq!(
            world.spawn_custom_cat(&world.require_level_id(&key()), pos, persistent),
            Ok(true)
        );
        let level = world
            .levels
            .get_mut(&world.require_level_id(&key()))
            .unwrap();
        assert_eq!(level.chunks.column_count(), 0);
        let ids = level.entities.ids();
        assert_eq!(ids.len(), 1);
        let entity = level.entities.get(ids[0]).unwrap();
        assert_eq!(
            entity.position(),
            Vec3d {
                x: 32.5,
                y: 90.0,
                z: 48.5
            }
        );
        assert_eq!(entity.old_position(), entity.position());
        assert_eq!(entity.y_rot(), 0.0);
        let NbtSummaryValue::Compound(fields) = level.entities.command_data(ids[0]).unwrap() else {
            panic!("compound");
        };
        assert!(fields
            .iter()
            .any(|entry| entry.name == "PersistenceRequired"
                && entry.value == NbtSummaryValue::Byte(i8::from(persistent))));
        assert!(fields.iter().any(|entry| entry.name == "sound_variant"));
        let saved = crate::entity::write_entity_chunk(&level.entities, (2, 3)).unwrap();
        assert_eq!(saved.entities().len(), 1);
        assert_eq!(saved.entities().next().unwrap().uuid(), entity.uuid());
        let events = level.entities.drain_lifecycle_events();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, EntityLifecycleEvent::Added { .. }))
                .count(),
            1
        );
        assert!(!events.iter().any(|event| matches!(
            event,
            EntityLifecycleEvent::Sound { .. } | EntityLifecycleEvent::SoundFromEntity { .. }
        )));
    }
}

mod finalize;
mod structure;
