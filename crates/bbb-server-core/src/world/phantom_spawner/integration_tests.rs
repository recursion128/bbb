use super::*;
use crate::connection::ConnectionId;
use crate::tests::{created_fixture_save, datapack_config, join_loaded, spawn_pinned_restore};
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PHANTOM_ID;
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::NbtSummaryValue;
use bbb_server_persistence::RegionCompression;
use bbb_server_types::save::Java26LevelKey;
use bbb_server_types::stats::{custom_stat, CUSTOM_STAT_TIME_SINCE_REST};

#[test]
fn phantom_spawner_normal_tick_preserves_phantoms_and_insomnia_stats_across_real_save() {
    let root = std::env::temp_dir().join(format!("bbb-phantom-spawner-{}", std::process::id()));
    let storage = created_fixture_save(&root, "phantom-spawner");
    let candidate = storage.load_candidate().unwrap();
    let mut world = spawn_pinned_restore(datapack_config(), &candidate);
    world.admin.set_difficulty(Difficulty::Hard);
    assert!(world
        .admin
        .set_game_rule("minecraft:spawn_patrols", GameRuleValue::Boolean(false)));
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "PhantomSave", 94);
    let key: Java26LevelKey = Java26BuiltinDimension::Overworld.into();
    let clock = world
        .registries
        .registry_element_index("minecraft:world_clock", "minecraft:overworld")
        .unwrap() as usize;
    let mut state = world.world_clock_states_for_test()[clock];
    state.total_ticks = 19_000;
    state.paused = true;
    world.set_world_clock_state_for_test(clock, state);
    world.refresh_level_sky_darkening(&world.require_level_id(&key));
    let rest = custom_stat(CUSTOM_STAT_TIME_SINCE_REST);
    let player = world.levels.player_mut(ConnectionId(1)).unwrap();
    player.cold.stats.set_value(rest, i32::MAX);
    let player_uuid = player.profile.uuid;
    let player_position = player.position();
    world.cold.mob_spawns = None;
    let mut events = Vec::new();
    for seed in 0..32 {
        world.cold.phantom_spawner.next_tick = 0;
        world
            .level_mut_by_id(&world.require_level_id(&key))
            .random
            .reseed(seed);
        events.extend(world.tick(receiver.drain()).events);
        let entities = &world.levels[&world.require_level_id(&key)].entities;
        if entities
            .ids()
            .into_iter()
            .any(|id| entities.get(id).unwrap().entity_type_id() == VANILLA_ENTITY_TYPE_PHANTOM_ID)
        {
            break;
        }
    }
    let entities = &world.levels[&world.require_level_id(&key)].entities;
    let members: Vec<_> = entities
        .ids()
        .into_iter()
        .filter_map(|id| entities.get(id))
        .filter(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_PHANTOM_ID)
        .map(|entity| {
            (
                entity.uuid(),
                entity.position(),
                entity.old_position(),
                entities.command_data(entity.id()).unwrap(),
            )
        })
        .collect();
    assert!((1..=4).contains(&members.len()), "{events:?}");
    let first_position = members[0].2;
    assert!(members.iter().all(|member| member.2 == first_position));
    assert!((20.0..=34.0).contains(&(first_position.y - player_position.y.floor())));
    assert!((-9.5..=10.5).contains(&(first_position.x - player_position.x.floor())));
    assert!((-9.5..=10.5).contains(&(first_position.z - player_position.z.floor())));
    for (_, _, position, nbt) in &members {
        let NbtSummaryValue::Compound(fields) = nbt else {
            panic!("phantom data");
        };
        assert!(fields
            .iter()
            .any(|entry| entry.name == "size" && entry.value == NbtSummaryValue::Int(0)));
        assert!(fields.iter().any(|entry| entry.name == "anchor_pos"
            && entry.value
                == NbtSummaryValue::IntArray(vec![
                    position.x.floor() as i32,
                    position.y as i32 + 5,
                    position.z.floor() as i32
                ])));
    }
    let request = world
        .save_request(1_786_000_000_000, 100, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    world.acknowledge_save_success(&request);
    let restored = spawn_pinned_restore(datapack_config(), &storage.load_candidate().unwrap());
    assert_eq!(restored.cold.phantom_spawner.next_tick, 0);
    assert_eq!(
        restored.cold.restored_player_stats[&player_uuid].value(rest),
        i32::MAX
    );
    let entities = &restored.levels[&restored.require_level_id(&key)].entities;
    for (uuid, position, _, old_data) in members {
        let entity = entities
            .ids()
            .into_iter()
            .filter_map(|id| entities.get(id))
            .find(|entity| entity.uuid() == uuid)
            .unwrap();
        assert_eq!(entity.position(), position);
        let NbtSummaryValue::Compound(old_fields) = old_data else {
            unreachable!();
        };
        let NbtSummaryValue::Compound(fields) = entities.command_data(entity.id()).unwrap() else {
            unreachable!();
        };
        for name in ["size", "anchor_pos"] {
            assert_eq!(
                fields.iter().find(|entry| entry.name == name),
                old_fields.iter().find(|entry| entry.name == name)
            );
        }
    }
    drop(storage);
    std::fs::remove_dir_all(root).unwrap();
}
