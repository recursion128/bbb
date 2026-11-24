use super::*;
use crate::connection::ConnectionId;
use crate::tests::{created_fixture_save, datapack_config, join_loaded, spawn_pinned_restore};
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::VANILLA_BLOCK_BELL_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::packets::Vec3d;
use bbb_server_persistence::RegionCompression;
use bbb_server_types::save::Java26LevelKey;

fn key() -> Java26LevelKey {
    Java26BuiltinDimension::Overworld.into()
}

fn spawn_rules(world: &mut ServerWorld, mobs: bool, monsters: bool) {
    for (rule, enabled) in [
        ("minecraft:spawn_mobs", mobs),
        ("minecraft:spawn_monsters", monsters),
    ] {
        let value = GameRuleValue::Boolean(enabled);
        assert!(world.admin.set_game_rule(rule, value));
        world.game_rules.set(rule, value);
    }
}

fn clock(world: &mut ServerWorld, time: i64) {
    let index = world
        .registries
        .registry_element_index("minecraft:world_clock", "minecraft:overworld")
        .unwrap() as usize;
    let mut state = world.world_clock_states_for_test()[index];
    state.total_ticks = time;
    state.paused = true;
    world.set_world_clock_state_for_test(index, state);
    world.refresh_level_sky_darkening(&world.require_level_id(&key()));
}

fn occupied_bell(world: &mut ServerWorld, pos: BlockPos) {
    let bell = vanilla_block_default_state_id(VANILLA_BLOCK_BELL_ID).unwrap();
    assert!(world.set_block(world.require_level_id(&key()), pos, bell, 0));
    assert!(world.acquire_poi_ticket(&key(), [pos.x, pos.y, pos.z]));
}

#[test]
fn village_siege_rules_daylight_and_dimensions_preserve_their_distinct_reset_edges() {
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    world.cold.village_siege = VillageSiege {
        active: true,
        setup: true,
        remaining: 7,
        next_spawn: 2,
        ..VillageSiege::default()
    };
    let mut events = Vec::new();
    spawn_rules(&mut world, false, true);
    world.tick_village_siege(&world.require_level_id(&key()), &mut events);
    assert!(world.cold.village_siege.active && world.cold.village_siege.setup);
    spawn_rules(&mut world, true, true);
    world.tick_village_siege(&Java26BuiltinDimension::Nether.into(), &mut events);
    assert!(world.cold.village_siege.active && world.cold.village_siege.setup);
    clock(&mut world, 6_000);
    world.tick_village_siege(&world.require_level_id(&key()), &mut events);
    assert!(!world.cold.village_siege.active && !world.cold.village_siege.setup);
    assert_eq!(
        (
            world.cold.village_siege.remaining,
            world.cold.village_siege.next_spawn
        ),
        (7, 2)
    );
    world.cold.village_siege.active = true;
    world.cold.village_siege.setup = true;
    clock(&mut world, 18_001);
    spawn_rules(&mut world, true, false);
    world.tick_village_siege(&world.require_level_id(&key()), &mut events);
    assert!(!world.cold.village_siege.active && !world.cold.village_siege.setup);
    assert!(events.is_empty());
}

#[test]
fn village_siege_failed_positions_still_consume_twenty_attempts_at_three_tick_intervals() {
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    spawn_rules(&mut world, true, true);
    clock(&mut world, 18_001);
    world.cold.village_siege = VillageSiege {
        active: true,
        setup: true,
        remaining: 20,
        ..VillageSiege::default()
    };
    let mut events = Vec::new();
    for tick in 0..61 {
        world.tick_village_siege(&world.require_level_id(&key()), &mut events);
        assert_eq!(world.cold.village_siege.remaining, (19 - tick / 3).max(0));
        assert_eq!(world.cold.village_siege.active, tick < 60);
    }
    assert!(world.levels[&world.require_level_id(&key())]
        .entities
        .ids()
        .is_empty());
    assert!(events.is_empty());
}

#[test]
fn village_siege_setup_accepts_first_non_spectator_even_when_all_positions_fail() {
    let mut config = datapack_config();
    config.difficulty = bbb_protocol::difficulties::Difficulty::Peaceful;
    let mut world = ServerWorld::new(config).unwrap();
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "SiegeSetup", 84);
    let pos = BlockPos { x: 1, y: 65, z: 1 };
    world.levels[ConnectionId(1)].set_position(Vec3d {
        x: 1.0,
        y: 65.0,
        z: 1.0,
    });
    occupied_bell(&mut world, pos);
    join_loaded(&sender, &receiver, &mut world, 2, "SiegeSecond", 85);
    world.levels[ConnectionId(2)].set_position(Vec3d {
        x: 257.0,
        y: 65.0,
        z: 1.0,
    });
    occupied_bell(&mut world, BlockPos { x: 257, ..pos });
    world.cold.village_siege.remaining = 7;
    world.cold.village_siege.next_spawn = 4;
    world.levels[ConnectionId(1)].game_type = GameType::Spectator;
    let mut events = Vec::new();
    assert!(world
        .setup_village_siege(&world.require_level_id(&key()), &mut events)
        .unwrap());
    assert!((world.cold.village_siege.center.x - 257).abs() <= 32);
    world.levels[ConnectionId(1)].game_type = GameType::Creative;
    world.levels[ConnectionId(1)].avatar_removed = true;
    assert!(world
        .setup_village_siege(&world.require_level_id(&key()), &mut events)
        .unwrap());
    assert_eq!(
        (
            world.cold.village_siege.remaining,
            world.cold.village_siege.next_spawn
        ),
        (7, 4)
    );
    assert!((world.cold.village_siege.center.x - 1).abs() <= 32);
    assert!(events.is_empty());
}

#[test]
fn village_siege_normal_tick_and_real_save_restore_keep_entities_but_reset_scheduler() {
    let root = std::env::temp_dir().join(format!("bbb-village-siege-{}", std::process::id()));
    let storage = created_fixture_save(&root, "village-siege");
    let candidate = storage.load_candidate().unwrap();
    let mut world = spawn_pinned_restore(datapack_config(), &candidate);
    assert!(world
        .admin
        .set_game_rule("minecraft:spawn_patrols", GameRuleValue::Boolean(false)));
    assert!(world
        .admin
        .set_game_rule("minecraft:spawn_phantoms", GameRuleValue::Boolean(false)));
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "SiegeSave", 81);
    let player = world.levels[ConnectionId(1)].position();
    let center = BlockPos {
        x: player.x.floor() as i32,
        y: player.y.floor() as i32,
        z: player.z.floor() as i32,
    };
    for dx in -3..=3 {
        for dz in -3..=3 {
            occupied_bell(
                &mut world,
                BlockPos {
                    x: center.x + dx * 16,
                    z: center.z + dz * 16,
                    ..center
                },
            );
        }
    }
    spawn_rules(&mut world, true, true);
    clock(&mut world, 18_001);
    world.cold.village_siege.active = true;
    world.cold.mob_spawns = None;
    world
        .level_mut_by_id(&world.require_level_id(&key()))
        .random
        .reseed(43);
    let tick = world.tick(receiver.drain());
    assert!(world.cold.village_siege.setup, "{:?}", tick.events);
    assert_eq!(world.cold.village_siege.remaining, 19, "{:?}", tick.events);
    let center = world.cold.village_siege.center;
    let entities = &world.levels[&world.require_level_id(&key())].entities;
    let zombies: Vec<_> = entities
        .ids()
        .into_iter()
        .filter_map(|id| entities.get(id))
        .filter(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_ZOMBIE_ID)
        .collect();
    assert_eq!(zombies.len(), 1, "{:?}", tick.events);
    let zombie = zombies[0];
    assert!((zombie.position().x - f64::from(center.x)).abs() <= 8.5);
    assert!((zombie.position().z - f64::from(center.z)).abs() <= 8.5);
    let uuid = zombie.uuid();
    let request = world
        .save_request(1_786_000_000_000, 100, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    world.acknowledge_save_success(&request);
    let loaded = storage.load_candidate().unwrap();
    let restored = spawn_pinned_restore(datapack_config(), &loaded);
    assert_eq!(restored.cold.village_siege, VillageSiege::default());
    let entities = &restored.levels[&restored.require_level_id(&key())].entities;
    assert!(entities
        .ids()
        .into_iter()
        .filter_map(|id| entities.get(id))
        .any(|entity| entity.uuid() == uuid));
    drop(storage);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn village_siege_unloaded_height_uses_minimum_without_generating_surface_candidates() {
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    spawn_rules(&mut world, true, true);
    clock(&mut world, 18_001);
    let min_y = world.levels[&world.require_level_id(&key())].chunks.min_y();
    let sea_level = world.levels[&world.require_level_id(&key())]
        .chunks
        .sea_level();
    assert_eq!(
        world
            .custom_spawn_height(
                &world.require_level_id(&key()),
                30_000_000,
                0,
                VANILLA_HEIGHTMAP_WORLD_SURFACE_ID
            )
            .unwrap(),
        sea_level + 1
    );
    let bell = vanilla_block_default_state_id(VANILLA_BLOCK_BELL_ID).unwrap();
    let air = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_AIR_ID,
    )
    .unwrap();
    let mut events = Vec::new();
    world.cold.village_siege.center = BlockPos { x: 8, y: 65, z: 8 };
    for y in [65, min_y] {
        world
            .level_mut_by_id(&world.require_level_id(&key()))
            .chunks
            .update_poi_on_block_change([8, y, 8], air, bell);
        assert!(world.acquire_poi_ticket(&key(), [8, y, 8]));
        let columns = world.levels[&world.require_level_id(&key())]
            .chunks
            .column_count();
        assert_eq!(
            world
                .find_siege_spawn_position(&world.require_level_id(&key()), &mut events)
                .unwrap(),
            None
        );
        assert_eq!(
            world.levels[&world.require_level_id(&key())]
                .chunks
                .column_count(),
            columns
        );
    }
    assert!(events.is_empty());
}
