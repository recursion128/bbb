use super::*;
use crate::connection::ConnectionId;
use crate::tests::{created_fixture_save, datapack_config, join_loaded, spawn_pinned_restore};
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PILLAGER_ID;
use bbb_server_persistence::RegionCompression;
use bbb_server_types::save::Java26LevelKey;

fn key() -> Java26LevelKey {
    Java26BuiltinDimension::Overworld.into()
}

fn rule(world: &mut ServerWorld, name: &'static str, enabled: bool) {
    let value = GameRuleValue::Boolean(enabled);
    assert!(world.admin.set_game_rule(name, value));
    world.game_rules.set(name, value);
}

fn clock(world: &mut ServerWorld, ticks: i64) {
    let index = world
        .registries
        .registry_element_index("minecraft:world_clock", "minecraft:overworld")
        .unwrap() as usize;
    let mut state = world.world_clock_states_for_test()[index];
    state.total_ticks = ticks;
    state.paused = true;
    world.set_world_clock_state_for_test(index, state);
    world.refresh_level_sky_darkening(&world.require_level_id(&key()));
}

#[test]
fn patrol_spawner_requires_all_margin_columns_without_loading_them() {
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    let pos = BlockPos {
        x: 104,
        y: 65,
        z: -104,
    };
    assert!(!world.custom_spawn_has_chunks_at(&world.require_level_id(&key()), pos));
    for x in (pos.x - 10) >> 4..=(pos.x + 10) >> 4 {
        for z in (pos.z - 10) >> 4..=(pos.z + 10) >> 4 {
            if (x, z) != ((pos.x + 10) >> 4, (pos.z + 10) >> 4) {
                world
                    .level_mut_by_id(&world.require_level_id(&key()))
                    .chunks
                    .try_ensure_served_full_column(x, z)
                    .unwrap();
            }
        }
    }
    assert!(!world.custom_spawn_has_chunks_at(&world.require_level_id(&key()), pos));
    world
        .level_mut_by_id(&world.require_level_id(&key()))
        .chunks
        .try_ensure_served_full_column((pos.x + 10) >> 4, (pos.z + 10) >> 4)
        .unwrap();
    assert!(world.custom_spawn_has_chunks_at(&world.require_level_id(&key()), pos));
}

#[test]
fn patrol_spawner_normal_tick_and_real_save_keep_patrol_members_but_reset_cooldown() {
    let root = std::env::temp_dir().join(format!("bbb-patrol-spawner-{}", std::process::id()));
    let storage = created_fixture_save(&root, "patrol-spawner");
    let saved = storage.load_candidate().unwrap();
    let mut world = spawn_pinned_restore(datapack_config(), &saved);
    world.admin.set_difficulty(Difficulty::Normal);
    rule(&mut world, "minecraft:spawn_phantoms", false);
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "PatrolSave", 92);
    let player = world.levels[ConnectionId(1)].position();
    let center = BlockPos {
        x: player.x.floor() as i32,
        y: player.y.floor() as i32,
        z: player.z.floor() as i32,
    };
    for x in (center.x - 64) >> 4..=(center.x + 64) >> 4 {
        for z in (center.z - 64) >> 4..=(center.z + 64) >> 4 {
            world
                .level_mut_by_id(&world.require_level_id(&key()))
                .chunks
                .try_ensure_served_full_column(x, z)
                .unwrap();
        }
    }
    clock(&mut world, 126_000);
    world.cold.mob_spawns = None;
    let mut events = Vec::new();
    for seed in 0..64 {
        world.cold.patrol_spawner.next_tick = 0;
        world
            .level_mut_by_id(&world.require_level_id(&key()))
            .random
            .reseed(seed);
        events.extend(world.tick(receiver.drain()).events);
        let entities = &world.levels[&world.require_level_id(&key())].entities;
        if entities
            .ids()
            .into_iter()
            .any(|id| entities.get(id).unwrap().entity_type_id() == VANILLA_ENTITY_TYPE_PILLAGER_ID)
        {
            break;
        }
    }
    assert!(world.cold.patrol_spawner.next_tick > 0, "{events:?}");
    let entities = &world.levels[&world.require_level_id(&key())].entities;
    let members: Vec<_> = entities
        .ids()
        .into_iter()
        .filter_map(|id| entities.get(id))
        .filter(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_PILLAGER_ID)
        .map(|entity| {
            (
                entity.uuid(),
                crate::entity::patrol_spawn_state(entity).unwrap(),
            )
        })
        .collect();
    assert_eq!(members.len(), 3, "{events:?}");
    assert_eq!(
        members.iter().filter(|(_, (_, leader, _))| *leader).count(),
        1
    );
    assert!(members.iter().all(|(_, (_, _, patrolling))| *patrolling));
    let leader = members.iter().find(|(_, (_, leader, _))| *leader).unwrap();
    let target = leader.1 .0.unwrap();
    assert_eq!(target.y, 0);
    assert!((-500..500).contains(&target.x) && (-500..500).contains(&target.z));
    let request = world
        .save_request(1_786_000_000_000, 100, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&saved, &request).unwrap();
    world.acknowledge_save_success(&request);
    let restored = spawn_pinned_restore(datapack_config(), &storage.load_candidate().unwrap());
    assert_eq!(restored.cold.patrol_spawner, PatrolSpawner::default());
    let entities = &restored.levels[&restored.require_level_id(&key())].entities;
    for (uuid, state) in members {
        let entity = entities
            .ids()
            .into_iter()
            .filter_map(|id| entities.get(id))
            .find(|entity| entity.uuid() == uuid)
            .unwrap();
        assert_eq!(crate::entity::patrol_spawn_state(entity), Some(state));
    }
    drop(storage);
    std::fs::remove_dir_all(root).unwrap();
}
