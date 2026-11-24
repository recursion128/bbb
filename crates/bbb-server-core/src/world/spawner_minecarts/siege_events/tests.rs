use super::*;
use crate::entity::EntityLifecycleEvent;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHICKEN_ID as CHICKEN;
use std::sync::OnceLock;

fn world() -> ServerWorld {
    static CONFIG: OnceLock<crate::world::ServerWorldConfig> = OnceLock::new();
    let mut world =
        ServerWorld::new(CONFIG.get_or_init(crate::tests::datapack_config).clone()).unwrap();
    for chunk in [(-1, -1), (-1, 0), (0, -1), (0, 0)] {
        world
            .load_entity_chunk_for_test(Java26BuiltinDimension::Overworld, chunk)
            .unwrap();
    }
    world
}
fn key() -> Java26LevelKey {
    Java26BuiltinDimension::Overworld.into()
}
#[test]
fn siege_event_existing_chicken_query_excludes_dead_and_occupied() {
    let mut world = world();
    let dimension = world.require_level_id(&key());
    let entities = &mut world.levels.get_mut(&dimension).unwrap().entities;
    let dead = entities
        .spawn_general_living(CHICKEN, Vec3d::default())
        .unwrap();
    entities
        .get_mut(dead)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_health(0.0);
    let occupied = entities
        .spawn_general_living(CHICKEN, Vec3d::default())
        .unwrap();
    let mut rider =
        create_natural_spawn_root(entities, VANILLA_ENTITY_TYPE_ZOMBIE_ID, Vec3d::default())
            .unwrap();
    assert!(entities.fresh_ride(&mut rider, occupied, false));
    entities.add_fresh(rider);
    let valid = entities
        .spawn_general_living(CHICKEN, Vec3d::default())
        .unwrap();
    assert_eq!(
        first_unridden_alive(
            entities,
            "minecraft:chicken",
            Aabb::of_size(Vec3d::default(), 10.0, 6.0, 10.0)
        ),
        Some(valid)
    );
}

/// Whether any of 64 rule checks at `pos` passes: the sky-light roll alone
/// fails a dark cell 15 times in 32, so 64 all failing has odds near 1e-21.
fn can_siege_spawn(world: &mut ServerWorld, pos: BlockPos) -> bool {
    let dimension = world.require_level_id(&key());
    (0..64).any(|_| {
        world
            .siege_spawn_rules(&dimension, pos)
            .expect("the rules answer")
    })
}

#[test]
fn siege_event_predicate_uses_committed_light_sky_darkening_and_event_floor() {
    let mut world = world();
    let pos = BlockPos { x: 0, y: 65, z: 0 };
    let dimension = world.require_level_id(&key());
    let level = world.levels.get_mut(&dimension).unwrap();
    level.chunks.load_column(0, 0);
    let stone = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
    )
    .unwrap();
    level.chunks.set_block_state(0, 64, 0, stone);
    level.chunks.flush_live_relights().unwrap();
    assert_eq!(level.chunks.sky_light_level(0, 65, 0), 15);
    level.sky_darkening = Some(0);
    assert!(!can_siege_spawn(&mut world, pos));
    let dimension = world.require_level_id(&key());
    let level = world.levels.get_mut(&dimension).unwrap();
    level.sky_darkening = Some(15);
    assert!(can_siege_spawn(&mut world, pos));
    let dimension = world.require_level_id(&key());
    let level = world.levels.get_mut(&dimension).unwrap();
    level.sky_darkening = Some(0);
    level
        .weather
        .prepare(bbb_server_persistence::Java26WeatherData {
            clear_weather_time: 0,
            rain_time: 100,
            thunder_time: 100,
            raining: true,
            thundering: true,
        });
    assert!(can_siege_spawn(&mut world, pos));
    let dimension = world.require_level_id(&key());
    let level = world.levels.get_mut(&dimension).unwrap();
    level.chunks.set_block_state(
        0,
        64,
        0,
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_AIR_ID,
        )
        .unwrap(),
    );
    level.chunks.flush_live_relights().unwrap();
    assert!(!can_siege_spawn(&mut world, pos));
    world.admin.set_difficulty(Difficulty::Peaceful);
    assert!(!can_siege_spawn(&mut world, pos));
}

#[test]
fn siege_event_public_spawn_uses_level_rng_and_no_ambient() {
    let mut world = world();
    let position = BlockPos {
        x: 80,
        y: 70,
        z: 96,
    };
    let id = world
        .spawn_siege_zombie(&world.require_level_id(&key()), position)
        .unwrap();
    let entity = world
        .levels
        .get(&world.require_level_id(&key()))
        .unwrap()
        .entities
        .get(id)
        .unwrap();
    assert_eq!(
        entity.position(),
        Vec3d {
            x: 80.5,
            y: 70.0,
            z: 96.5
        }
    );
    assert_eq!(entity.portal_cooldown(), 0);
    assert!(!world
        .levels
        .get_mut(&world.require_level_id(&key()))
        .unwrap()
        .entities
        .drain_lifecycle_events()
        .iter()
        .any(|event| matches!(
            event,
            EntityLifecycleEvent::Sound { .. } | EntityLifecycleEvent::SoundFromEntity { .. }
        )));
}
