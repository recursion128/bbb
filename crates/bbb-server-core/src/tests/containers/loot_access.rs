use super::*;
use crate::tests::entities::console_command_answer;

const TABLE: &str = "minecraft:chests/igloo_chest";

#[test]
fn all_builtin_chest_tables_fill_through_the_live_native_environment() {
    let startup = crate::registry_sync::builtin_datapack_startup_shared();
    let tables: Vec<_> = startup
        .loot
        .registries
        .iter()
        .filter(|registry| registry.registry_id == "minecraft:loot_table")
        .flat_map(|registry| &registry.elements)
        .filter(|element| element.id.starts_with("minecraft:chests/"))
        .map(|element| element.id.clone())
        .collect();
    assert_eq!(tables.len(), 56);
    let root =
        std::env::temp_dir().join(format!("bbb-builtin-chest-loot-{}", uuid::Uuid::new_v4()));
    let storage = created_fixture_save(&root, "bbb-builtin-chest-loot");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    let config = ServerWorldConfig {
        datapacks: Some(startup),
        ..creative_fixture_config()
    };
    let mut world = spawn_pinned_restore(config.clone(), &candidate);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let mut filled = Vec::new();
    for (index, table) in tables.iter().enumerate() {
        let pos = BlockPos {
            x: 1 + (index % 14) as i32,
            y: 64,
            z: 1 + (index / 14) as i32,
        };
        put(&mut world, pos, chest_state("north", "single"));
        assert!(entity(&mut world, pos).set_container_loot_table(table, 26_101));
        let command = format!(
            "item replace block {} {} {} container.0 from block {} {} {} container.0",
            pos.x, pos.y, pos.z, pos.x, pos.y, pos.z
        );
        enqueue(
            &sender,
            WorldCommand::Admin {
                envelope: admin_envelope(index as u64 + 1, table, AdminScope::Command),
                request: AdminRequest::ExecuteCommand { command },
            },
        )
        .unwrap();
        let tick = world.tick(receiver.drain());
        let outcome = admin_outcome(&tick);
        let Ok(AdminResult::Command(report)) = &outcome.result else {
            panic!("{table}: {outcome:?}");
        };
        assert_eq!(report.result, Some(1), "{table}: {report:?}");
        let chest = entity(&mut world, pos);
        assert_eq!(chest.container_loot_table_id(), None, "{table}");
        let slots = chest.container_items().unwrap();
        filled.push((
            pos,
            slots
                .iter()
                .map(|stack| stack.summary().clone())
                .collect::<Vec<_>>(),
        ));
    }
    let request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let candidate = storage.load_candidate().unwrap();
    let mut reloaded = spawn_pinned_restore(config, &candidate);
    for (pos, expected) in filled {
        let chest = entity(&mut reloaded, pos);
        assert_eq!(chest.container_loot_table_id(), None);
        let actual: Vec<_> = chest
            .container_items()
            .unwrap()
            .iter()
            .map(|stack| stack.summary().clone())
            .collect();
        assert_eq!(actual, expected, "{pos:?}");
    }
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

fn loot_config() -> ServerWorldConfig {
    datapack_config_with_loot_table(
        TABLE,
        &serde_json::json!({
            "type": "minecraft:chest",
            "pools": [{"rolls": 1, "entries": [{"type": "minecraft:item", "name": "minecraft:stone"}]}]
        }),
    )
}

fn entity(world: &mut ServerWorld, pos: BlockPos) -> &mut BlockEntity {
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, pos)
        .unwrap()
}

fn count(world: &mut ServerWorld, pos: BlockPos) -> i32 {
    entity(world, pos)
        .container_items()
        .unwrap()
        .iter()
        .map(|item| item.count())
        .sum()
}

fn put(world: &mut ServerWorld, pos: BlockPos, state: i32) {
    world.set_block(
        Java26BuiltinDimension::Overworld,
        pos,
        state,
        SET_BLOCK_UPDATE_ALL,
    );
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, pos),
        Some(state)
    );
}

#[test]
fn a_hopper_unpacks_the_reached_half_and_preserves_pending_loot_across_restart() {
    let root = std::env::temp_dir().join(format!("bbb-hopper-lazy-loot-{}", uuid::Uuid::new_v4()));
    let storage = created_fixture_save(&root, "bbb-hopper-lazy-loot");
    let paths = storage.paths().clone();
    let mut candidate = storage.load_candidate().unwrap();
    candidate.weather = bbb_server_persistence::Java26WeatherData {
        clear_weather_time: 100_000,
        ..Default::default()
    };
    let config = datapack_config_with_loot_table(
        TABLE,
        &serde_json::json!({
            "type": "minecraft:chest",
            "pools": [
                {"rolls": 1, "conditions": [{
                    "condition": "minecraft:weather_check", "raining": false, "thundering": false
                }], "entries": [{"type": "minecraft:item", "name": "minecraft:stone"}]},
                {"rolls": 1, "conditions": [{
                    "condition": "minecraft:weather_check", "raining": true, "thundering": false
                }], "entries": [{"type": "minecraft:item", "name": "minecraft:diamond"}]}
            ]
        }),
    );
    let mut world = spawn_pinned_restore(config.clone(), &candidate);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let weather = world.level_weather(Java26BuiltinDimension::Overworld);
    assert!(!weather.is_raining());
    assert!(!weather.is_thundering());
    let hopper = BlockPos { x: 0, y: 64, z: 0 };
    let first = BlockPos { y: 65, ..hopper };
    let second = BlockPos { x: 1, ..first };
    put(&mut world, first, chest_state("south", "right"));
    put(&mut world, second, chest_state("south", "left"));
    put(
        &mut world,
        BlockPos { y: 66, ..first },
        block_state_id("minecraft:stone"),
    );
    put(
        &mut world,
        BlockPos { y: 66, ..second },
        block_state_id("minecraft:stone"),
    );
    let enabled = hopper_down_enabled();
    put(&mut world, hopper, enabled);
    let power = BlockPos { z: 1, ..hopper };
    put(
        &mut world,
        power,
        block_state_id("minecraft:redstone_block"),
    );
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, hopper),
        crate::fluid::with_property(enabled, "enabled", "false")
    );
    for (pos, seed) in [(hopper, 31), (first, 17), (second, 19)] {
        assert!(entity(&mut world, pos).set_container_loot_table(TABLE, seed));
    }
    world.tick(receiver.drain());
    for pos in [hopper, first, second] {
        assert_eq!(
            entity(&mut world, pos).container_loot_table_id(),
            Some(TABLE)
        );
    }
    put(&mut world, power, block_state_id("minecraft:air"));
    entity(&mut world, hopper)
        .hopper_mut()
        .unwrap()
        .set_cooldown(2);
    world.tick(receiver.drain());
    for pos in [hopper, first, second] {
        assert_eq!(
            entity(&mut world, pos).container_loot_table_id(),
            Some(TABLE)
        );
    }
    world.tick(receiver.drain());
    assert_eq!(count(&mut world, hopper), 2);
    assert!(entity(&mut world, hopper)
        .container_items()
        .unwrap()
        .iter()
        .filter(|item| !item.is_empty())
        .all(|item| item.item_resource_id() == Some("minecraft:stone")));
    assert_eq!(
        entity(&mut world, hopper).hopper().unwrap().cooldown_time(),
        8
    );
    assert_eq!(entity(&mut world, hopper).container_loot_table_id(), None);
    assert_eq!(entity(&mut world, first).container_loot_table_id(), None);
    assert_eq!(count(&mut world, first), 0);
    assert_eq!(
        entity(&mut world, second).container_loot_table_id(),
        Some(TABLE)
    );
    assert_eq!(count(&mut world, second), 0);
    let request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    let mut world = spawn_pinned_restore(config, &reloaded);
    let weather = world.level_weather(Java26BuiltinDimension::Overworld);
    assert!(!weather.is_raining());
    assert!(!weather.is_thundering());
    assert_eq!(count(&mut world, hopper), 2);
    assert_eq!(
        entity(&mut world, hopper).hopper().unwrap().cooldown_time(),
        8
    );
    assert_eq!(entity(&mut world, first).container_loot_table_id(), None);
    assert_eq!(
        entity(&mut world, second).container_loot_table_id(),
        Some(TABLE)
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_comparator_unpacks_both_halves_after_the_chest_obstruction_is_removed() {
    let mut world = spawn_pinned_world(datapack_config_with_loot_table(
        TABLE,
        &serde_json::json!({
            "type": "minecraft:chest",
            "pools": [{"rolls": 1, "conditions": [{
                "condition": "minecraft:any_of",
                "terms": ([-1, 0].map(|offset| serde_json::json!({
                    "condition": "minecraft:location_check", "offsetX": offset, "offsetZ": 1,
                    "predicate": {"block": {"nbt": "{id:'minecraft:chest',x:1,y:64,z:1,Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}"}}
                })))
            }], "entries": [{"type": "minecraft:item", "name": "minecraft:stone"}]}]
        }),
    ));
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    for command in [
        "setblock 1 64 1 minecraft:chest",
        "item replace block 1 64 1 container.0 with minecraft:diamond 3",
    ] {
        let setup = console_command(&sender, &receiver, &mut world, 1, command);
        assert_eq!(console_command_answer(&setup).0, Some(1));
    }
    let comparator = BlockPos { x: 0, y: 64, z: 0 };
    let first = BlockPos { x: 1, ..comparator };
    let second = BlockPos { x: 2, ..comparator };
    put(
        &mut world,
        BlockPos {
            y: 63,
            ..comparator
        },
        block_state_id("minecraft:stone"),
    );
    put(&mut world, first, chest_state("south", "right"));
    put(&mut world, second, chest_state("south", "left"));
    let cover = BlockPos { y: 65, ..first };
    put(&mut world, cover, block_state_id("minecraft:stone"));
    for pos in [first, second] {
        assert!(entity(&mut world, pos).set_container_loot_table(TABLE, 19));
    }
    let state = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:comparator",
        &BTreeMap::from([
            ("facing".into(), "east".into()),
            ("mode".into(), "compare".into()),
            ("powered".into(), "false".into()),
        ]),
    )
    .unwrap();
    put(&mut world, comparator, state);
    put(
        &mut world,
        BlockPos {
            y: 65,
            ..comparator
        },
        block_state_id("minecraft:stone"),
    );
    for _ in 0..3 {
        world.tick(receiver.drain());
    }
    assert_eq!(
        entity(&mut world, comparator)
            .comparator()
            .unwrap()
            .output_signal(),
        0
    );
    for pos in [first, second] {
        assert_eq!(
            entity(&mut world, pos).container_loot_table_id(),
            Some(TABLE)
        );
    }
    put(&mut world, cover, block_state_id("minecraft:air"));
    put(
        &mut world,
        BlockPos {
            y: 65,
            ..comparator
        },
        block_state_id("minecraft:air"),
    );
    for _ in 0..3 {
        world.tick(receiver.drain());
    }
    for pos in [first, second] {
        assert_eq!(entity(&mut world, pos).container_loot_table_id(), None);
        assert_eq!(count(&mut world, pos), 1);
    }
    assert_eq!(
        entity(&mut world, comparator)
            .comparator()
            .unwrap()
            .output_signal(),
        1
    );
}

#[test]
fn redstone_unpacks_a_dispenser_before_dispensing_and_a_dropper_before_live_insertion() {
    for dropper in [false, true] {
        let mut world = spawn_pinned_world(loot_config());
        let (sender, receiver) = mailbox();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        stand_at(
            &sender,
            &receiver,
            &mut world,
            1,
            Vec3d {
                x: 8.5,
                y: 64.0,
                z: 0.5,
            },
        );
        let source = BlockPos { x: 0, y: 64, z: 0 };
        let first = BlockPos { x: 1, ..source };
        let second = BlockPos { x: 2, ..source };
        let state = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            if dropper {
                "minecraft:dropper"
            } else {
                "minecraft:dispenser"
            },
            &BTreeMap::from([
                ("facing".into(), "east".into()),
                ("triggered".into(), "false".into()),
            ]),
        )
        .unwrap();
        put(&mut world, source, state);
        put(&mut world, first, chest_state("south", "right"));
        put(&mut world, second, chest_state("south", "left"));
        put(
            &mut world,
            BlockPos { y: 65, ..first },
            block_state_id("minecraft:stone"),
        );
        for pos in [source, first, second] {
            assert!(entity(&mut world, pos).set_container_loot_table(TABLE, 31));
        }
        world.tick(receiver.drain());
        for pos in [source, first, second] {
            assert_eq!(
                entity(&mut world, pos).container_loot_table_id(),
                Some(TABLE)
            );
        }
        put(
            &mut world,
            BlockPos { y: 65, ..source },
            block_state_id("minecraft:redstone_block"),
        );
        for _ in 0..5 {
            world.tick(receiver.drain());
        }
        assert_eq!(entity(&mut world, source).container_loot_table_id(), None);
        assert_eq!(count(&mut world, source), 0);
        assert_eq!(
            entity(&mut world, first).container_loot_table_id(),
            if dropper { None } else { Some(TABLE) }
        );
        assert_eq!(count(&mut world, first), if dropper { 2 } else { 0 });
        assert_eq!(
            entity(&mut world, second).container_loot_table_id(),
            Some(TABLE)
        );
        assert_eq!(count(&mut world, second), 0);
        let dropped = dropped_ground_items(&world, Java26BuiltinDimension::Overworld);
        if dropper {
            assert!(dropped.is_empty());
        } else {
            assert_eq!(dropped.len(), 1);
            assert_eq!((&*dropped[0].0, dropped[0].1), ("minecraft:stone", 1));
        }
    }
}

/// `DecoratedPotBlock.useItemOn` (`DecoratedPotBlock.java:96-139`) inserts one
/// item and wobbles; empty-hand `useWithoutItem` (`:142-153`) fail-wobbles
/// without taking the stack out.
#[test]
fn placing_an_item_in_a_decorated_pot_then_fail_wobbles_on_empty_hand() {
    let (sender, receiver) = mailbox();
    let table = "minecraft:chests/igloo_chest";
    let mut world = spawn_pinned_world(datapack_config_with_loot_table(
        table,
        &serde_json::json!({"type": "minecraft:chest", "pools": []}),
    ));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        decorated_pot_state(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .unwrap()
        .set_container_loot_table(table, 31));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        }),
    );
    let inserted = right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert_eq!(
        world
            .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
            .unwrap()
            .container_loot_table_id(),
        None
    );
    let pot = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|block_entity| block_entity.decorated_pot())
        .expect("pot");
    assert_eq!(
        pot.item().item_id(),
        Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID)
    );
    assert_eq!(pot.item().count(), 1);
    assert!(sounds_sent(&inserted)
        .iter()
        .any(|sound| (sound.pitch - (0.7 + 0.5 / 64.0)).abs() < f32::EPSILON));
    assert!(pot.redstone_signal().unwrap() > 0);
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::StatKey::new(
                bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID,
                bbb_protocol::item_types::VANILLA_ITEM_STONE_ID,
            )
        ),
        1
    );
    assert!(inserted.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::LevelParticles(particles), .. }
        if particles.particle.particle_type_id == bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_DUST_PLUME_ID && particles.count == 7
    )));
    assert!(
        sounds_sent(&inserted).iter().any(|sound| sound.sound
            == SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id(
                    "minecraft:block.decorated_pot.insert"
                )
                .expect("a registered sound"),
            }),
        "{inserted:?}"
    );
    assert!(
        block_events_sent(&inserted)
            .iter()
            .any(|event| event.b0 == 1 && event.b1 == 0),
        "{inserted:?}"
    );

    right_click(&sender, &receiver, &mut world, 1, at, 2);
    assert_eq!(
        world
            .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
            .and_then(|body| body.decorated_pot())
            .unwrap()
            .original_items()[0]
            .count(),
        2
    );

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary::empty(),
        }),
    );
    let failed = right_click(&sender, &receiver, &mut world, 1, at, 3);
    let pot = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|block_entity| block_entity.decorated_pot())
        .expect("pot");
    assert_eq!(pot.item().count(), 2);
    assert!(
        sounds_sent(&failed).iter().any(|sound| sound.sound
            == SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id(
                    "minecraft:block.decorated_pot.insert_fail"
                )
                .expect("a registered sound"),
            }),
        "{failed:?}"
    );

    let resolver = world.item_stack_resolver();
    let stack = |components, count| {
        bbb_server_types::item_stack::RuntimeItemStack::try_read(
            &NbtSummaryValue::Compound(vec![
                nbt("id", NbtSummaryValue::String("minecraft:stone".into())),
                nbt("count", NbtSummaryValue::Int(count)),
                nbt("components", NbtSummaryValue::Compound(components)),
            ]),
            &resolver,
        )
        .unwrap()
    };
    let default = stack(vec![], 4);
    let common = stack(
        vec![nbt(
            "minecraft:rarity",
            NbtSummaryValue::String("common".into()),
        )],
        4,
    );
    let uncommon = stack(
        vec![nbt(
            "minecraft:rarity",
            NbtSummaryValue::String("uncommon".into()),
        )],
        4,
    );
    let removed = stack(
        vec![nbt("!minecraft:rarity", NbtSummaryValue::Compound(vec![]))],
        4,
    );
    let mut abilities = world.player_abilities_for_test(ConnectionId(1));
    abilities.instabuild = false;
    world.set_player_mode_and_abilities_for_test(ConnectionId(1), GameType::Survival, abilities);
    for (index, (stored, incoming, accepts)) in [
        (default.clone(), common.clone(), true),
        (common, default.clone(), true),
        (default.clone(), uncommon, false),
        (default, removed, false),
    ]
    .into_iter()
    .enumerate()
    {
        entity(&mut world, at)
            .decorated_pot_mut()
            .unwrap()
            .set_item(stored.clone());
        assert!(world
            .player_inventory_for_test(ConnectionId(1))
            .set_inventory_slot(0, incoming.clone()));
        let stat = bbb_server_types::stats::StatKey::new(
            bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID,
            bbb_protocol::item_types::VANILLA_ITEM_STONE_ID,
        );
        let used_before = world.player_stat_for_test(ConnectionId(1), stat);
        let clicked = right_click(&sender, &receiver, &mut world, 1, at, 4 + index as i32);
        let pot = entity(&mut world, at)
            .decorated_pot()
            .unwrap()
            .item()
            .clone();
        let held = world
            .player_inventory_for_test(ConnectionId(1))
            .inventory_slots()[0]
            .clone();
        assert_eq!(pot.count(), stored.count() + i32::from(accepts));
        assert_eq!(held.count(), incoming.count() - i32::from(accepts));
        assert_eq!(
            pot.count() + held.count(),
            stored.count() + incoming.count()
        );
        assert!(pot
            .same_item_same_effective_components(&stored, &resolver)
            .unwrap());
        assert!(held
            .same_item_same_effective_components(&incoming, &resolver)
            .unwrap());
        assert_eq!(
            world.player_stat_for_test(ConnectionId(1), stat),
            used_before + i32::from(accepts)
        );
        let sound = if accepts {
            "minecraft:block.decorated_pot.insert"
        } else {
            "minecraft:block.decorated_pot.insert_fail"
        };
        assert!(sounds_sent(&clicked)
            .iter()
            .any(|sound_event| sound_event.sound
                == SoundEventHolder::Reference {
                    registry_id: vanilla_sound_event_registry_id(sound).unwrap(),
                }));
        assert!(block_events_sent(&clicked)
            .iter()
            .any(|event| event.b0 == 1 && event.b1 == u8::from(!accepts)));
        assert_eq!(clicked.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::LevelParticles(particles), .. }
            if particles.particle.particle_type_id == bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_DUST_PLUME_ID && particles.count == 7
        )), accepts);
    }
}
