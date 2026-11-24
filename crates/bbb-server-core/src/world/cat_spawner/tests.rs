use super::*;
use crate::connection::ConnectionId;
use crate::tests::{created_fixture_save, datapack_config, join_loaded, spawn_pinned_restore};
use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::packets::Vec3d;
use bbb_protocol::NbtSummaryValue;
use bbb_server_persistence::RegionCompression;
use bbb_server_types::save::Java26LevelKey;

fn key() -> Java26LevelKey {
    Java26BuiltinDimension::Overworld.into()
}

fn home(world: &mut ServerWorld, position: BlockPos) {
    let properties = [("facing", "north"), ("occupied", "false"), ("part", "head")]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    let state =
        vanilla_block_state_id_by_name_and_properties("minecraft:red_bed", &properties).unwrap();
    assert!(world.set_block(world.require_level_id(&key()), position, state, 0));
    assert!(world.acquire_poi_ticket(&key(), [position.x, position.y, position.z]));
}

#[test]
fn cat_spawner_uses_fixed_delay_and_mob_rule_but_not_hostile_difficulty() {
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    world.admin.set_difficulty(Difficulty::Peaceful);
    world.cold.cat_spawner.next_tick = -9;
    let mut events = Vec::new();
    world
        .game_rules
        .set("minecraft:spawn_mobs", GameRuleValue::Boolean(false));
    world.tick_cat_spawner(&world.require_level_id(&key()), &mut events);
    assert_eq!(world.cold.cat_spawner.next_tick, -9);
    world
        .game_rules
        .set("minecraft:spawn_mobs", GameRuleValue::Boolean(true));
    world.tick_cat_spawner(&Java26BuiltinDimension::Nether.into(), &mut events);
    assert_eq!(world.cold.cat_spawner.next_tick, -9);
    world.tick_cat_spawner(&world.require_level_id(&key()), &mut events);
    assert_eq!(world.cold.cat_spawner.next_tick, 1200);
    world.tick_cat_spawner(&world.require_level_id(&key()), &mut events);
    assert_eq!(world.cold.cat_spawner.next_tick, 1199);
    assert!(events.is_empty());
}

#[test]
fn cat_spawner_village_requires_five_occupied_homes_and_stops_at_five_cats() {
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    load_chunks(&mut world, -1, 5);
    let position = BlockPos {
        x: 32,
        y: 65,
        z: 32,
    };
    for x in 0..4 {
        home(
            &mut world,
            BlockPos {
                x: position.x + x + 2,
                ..position
            },
        );
    }
    assert_eq!(
        world.is_close_to_village(&key(), [32, 65, 32], 2),
        Some(true)
    );
    world
        .try_spawn_local_cat(&world.require_level_id(&key()), position)
        .unwrap();
    assert_eq!(
        world.local_cat_count(&world.require_level_id(&key()), position, 48.0),
        0
    );
    home(
        &mut world,
        BlockPos {
            x: position.x + 6,
            ..position
        },
    );
    for count in 1..=5 {
        world
            .try_spawn_local_cat(&world.require_level_id(&key()), position)
            .unwrap();
        assert_eq!(
            world.local_cat_count(&world.require_level_id(&key()), position, 48.0),
            count
        );
    }
    world
        .try_spawn_local_cat(&world.require_level_id(&key()), position)
        .unwrap();
    assert_eq!(
        world.local_cat_count(&world.require_level_id(&key()), position, 48.0),
        5
    );
    assert_eq!(world.release_poi_ticket(&key(), [38, 65, 32]), Some(true));
    world
        .try_spawn_local_cat(&world.require_level_id(&key()), position)
        .unwrap();
}

#[test]
fn cat_spawner_count_uses_intersecting_boxes_and_keeps_dying_cats() {
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    load_chunks(&mut world, -3, 3);
    let position = BlockPos { x: 0, y: 65, z: 0 };
    let entities = &mut world
        .level_mut_by_id(&world.require_level_id(&key()))
        .entities;
    let dead = entities
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_CAT_ID,
            Vec3d {
                x: 0.5,
                y: 65.0,
                z: 0.5,
            },
        )
        .unwrap();
    entities
        .get_mut(dead)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_health(0.0);
    entities
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_CAT_ID,
            Vec3d {
                x: 49.1,
                y: 65.0,
                z: 0.5,
            },
        )
        .unwrap();
    entities
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_CAT_ID,
            Vec3d {
                x: 0.5,
                y: 74.0,
                z: 0.5,
            },
        )
        .unwrap();
    entities
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 0.5,
                y: 65.0,
                z: 0.5,
            },
        )
        .unwrap();
    assert_eq!(
        world.local_cat_count(&world.require_level_id(&key()), position, 48.0),
        2
    );
    assert_eq!(
        world.local_cat_count(&world.require_level_id(&key()), position, 16.0),
        1
    );
}

#[test]
fn cat_spawner_normal_tick_and_real_save_keep_cat_sound_and_reset_scheduler() {
    let root = std::env::temp_dir().join(format!("bbb-cat-spawner-{}", std::process::id()));
    let storage = created_fixture_save(&root, "cat-spawner");
    let candidate = storage.load_candidate().unwrap();
    let mut world = spawn_pinned_restore(datapack_config(), &candidate);
    for name in ["minecraft:spawn_patrols", "minecraft:spawn_phantoms"] {
        assert!(world
            .admin
            .set_game_rule(name, GameRuleValue::Boolean(false)));
    }
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "CatSave", 103);
    let player = world.levels[ConnectionId(1)].position();
    let center = BlockPos {
        x: player.x.floor() as i32,
        y: player.y.floor() as i32,
        z: player.z.floor() as i32,
    };
    for x in (center.x - 48) >> 4..=(center.x + 48) >> 4 {
        for z in (center.z - 48) >> 4..=(center.z + 48) >> 4 {
            world
                .level_mut_by_id(&world.require_level_id(&key()))
                .chunks
                .try_ensure_served_full_column(x, z)
                .unwrap();
        }
    }
    for x in 0..5 {
        home(
            &mut world,
            BlockPos {
                x: center.x + x,
                ..center
            },
        );
    }
    world.cold.mob_spawns = None;
    world.cold.cat_spawner.next_tick = 0;
    world
        .level_mut_by_id(&world.require_level_id(&key()))
        .random
        .reseed(3);
    let tick = world.tick(receiver.drain());
    assert_eq!(world.cold.cat_spawner.next_tick, 1200);
    let entities = &world.levels[&world.require_level_id(&key())].entities;
    let cats: Vec<_> = entities
        .ids()
        .into_iter()
        .filter_map(|id| entities.get(id))
        .filter(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_CAT_ID)
        .collect();
    assert_eq!(cats.len(), 1, "{:?}", tick.events);
    let cat = cats[0];
    let uuid = cat.uuid();
    let NbtSummaryValue::Compound(fields) = entities.command_data(cat.id()).unwrap() else {
        panic!("cat data");
    };
    let sound = fields
        .iter()
        .find(|entry| entry.name == "sound_variant")
        .unwrap()
        .value
        .clone();
    assert!(
        matches!(&sound, NbtSummaryValue::String(value) if value == "minecraft:classic" || value == "minecraft:royal")
    );
    let request = world
        .save_request(1_786_000_000_000, 100, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    world.acknowledge_save_success(&request);
    let restored = spawn_pinned_restore(datapack_config(), &storage.load_candidate().unwrap());
    assert_eq!(restored.cold.cat_spawner, CatSpawner::default());
    let entities = &restored.levels[&restored.require_level_id(&key())].entities;
    let cat = entities
        .ids()
        .into_iter()
        .filter_map(|id| entities.get(id))
        .find(|entity| entity.uuid() == uuid)
        .unwrap();
    let NbtSummaryValue::Compound(fields) = entities.command_data(cat.id()).unwrap() else {
        panic!("cat data");
    };
    assert_eq!(
        fields
            .iter()
            .find(|entry| entry.name == "sound_variant")
            .unwrap()
            .value,
        sound
    );
    drop(storage);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cat_spawner_village_shortfall_does_not_spawn_a_cat() {
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    let mut chunks = crate::chunk::ChunkStore::new_overworld_carvers_with_structure_generation(
        crate::chunk::noise_generator::overworld_program(true, 0),
        false,
    )
    .unwrap();
    chunks.install_registry_tags(world.registries.registry_tags_shared());
    let properties = [("facing", "north"), ("occupied", "false"), ("part", "head")]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    let bed =
        vanilla_block_state_id_by_name_and_properties("minecraft:red_bed", &properties).unwrap();
    let air = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_AIR_ID,
    )
    .unwrap();
    chunks.update_poi_on_block_change([0, 65, 0], air, bed);
    assert!(chunks.poi_mut().acquire_ticket([0, 65, 0]));
    world
        .level_mut_by_id(&world.require_level_id(&key()))
        .chunks = chunks;
    let position = BlockPos { x: 1, y: 65, z: 1 };
    assert_eq!(world.is_close_to_village(&key(), [1, 65, 1], 2), Some(true));
    world
        .try_spawn_local_cat(&world.require_level_id(&key()), position)
        .unwrap();
    assert!(world.levels[&world.require_level_id(&key())]
        .entities
        .ids()
        .is_empty());
}

/// Entities only answer queries in loaded chunks.
fn load_chunks(world: &mut ServerWorld, min: i32, max: i32) {
    for x in min..=max {
        for z in min..=max {
            world
                .load_entity_chunk_for_test(Java26BuiltinDimension::Overworld, (x, z))
                .unwrap();
        }
    }
}
