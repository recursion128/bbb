//! `FishingRodItem.use`: casting into water, waiting for a bite and reeling
//! the catch in.

use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID, VANILLA_ENTITY_TYPE_ITEM_ID,
    VANILLA_ENTITY_TYPE_PAINTING_ID,
};
use bbb_protocol::packets::{BlockPos, ItemStackSummary, Vec3d};

use super::*;
use crate::entity::{Entity, EntityData};
use crate::tests::entities::{console_command, console_command_answer};

mod hook_player;

fn entities_of(world: &ServerWorld, entity_type_id: i32) -> Vec<Entity> {
    world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .filter_map(|id| world.stored_entity_for_test(Java26BuiltinDimension::Overworld, id))
        .filter(|entity| entity.entity_type_id() == entity_type_id)
        .cloned()
        .collect()
}

#[test]
fn a_bite_reeled_in_lands_fishing_loot_and_fires_fishing_rod_hooked() {
    for raining in [false, true] {
        reel_bite(false, raining);
    }
}

#[test]
fn empty_fishing_loot_still_matches_context_and_awards_rewards_once() {
    reel_bite(true, false);
}

fn reel_bite(empty: bool, raining: bool) {
    let mounted = !empty && !raining;
    let mut pools = vec![serde_json::json!({
        "rolls":1, "conditions":[{
            "condition":"minecraft:location_check", "offsetY":-2,
            "predicate":{"block":{"nbt":"{id:'minecraft:chest',x:0,y:61,z:4,Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}"}}
        }, {
            "condition":"minecraft:weather_check", "raining":false
        }], "entries":[{
            "type":"minecraft:item", "name":"minecraft:cod",
            "functions":[{"function":"minecraft:set_count", "count":2}]
        }]
    })];
    if !empty {
        pools.insert(
            0,
            serde_json::json!({
                "rolls":1,
                "entries":[{"type":"minecraft:item", "name":"minecraft:apple"}]
            }),
        );
    }
    let root = std::env::temp_dir().join(format!("bbb-fishing-weather-{}", Uuid::new_v4()));
    let pack = root.join("test");
    std::fs::create_dir_all(pack.join("data/minecraft/loot_table/gameplay")).unwrap();
    std::fs::create_dir_all(pack.join("data/test/advancement")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"fishing weather","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    std::fs::write(
        pack.join("data/minecraft/loot_table/gameplay/fishing.json"),
        serde_json::to_vec(&serde_json::json!({"type":"minecraft:fishing", "pools":pools}))
            .unwrap(),
    )
    .unwrap();
    for (name, conditions) in [
        ("reward", serde_json::json!({})),
        (
            "fish",
            serde_json::json!({"rod":{"items":"minecraft:fishing_rod"}, "item":{"count":{"min":1}}}),
        ),
        (
            "player",
            serde_json::json!({"player":{"type":"minecraft:player","distance":{"absolute":0},"nbt":"{SelectedItem:{id:'minecraft:fishing_rod',count:1}}"}}),
        ),
        (
            "list",
            serde_json::json!({"player":[{"condition":"entity_properties","entity":"this","predicate":{"type":"minecraft:player","nbt":"{SelectedItem:{id:'minecraft:fishing_rod',count:1}}"}}], "entity":[{"condition":"entity_properties","entity":"this","predicate":{"type":"minecraft:fishing_bobber"}}]}),
        ),
        (
            "wrong_nbt",
            serde_json::json!({"player":{"nbt":"{SelectedItem:{id:'minecraft:stick'}}"}}),
        ),
        (
            "wrong_nbt_list",
            serde_json::json!({"player":[{"condition":"entity_properties","entity":"this","predicate":{"nbt":"{SelectedItem:{id:'minecraft:stick'}}"}}]}),
        ),
        (
            "bobber",
            serde_json::json!({"entity":{"type":"minecraft:fishing_bobber","distance":{"absolute":{"min":1,"max":32}}}}),
        ),
        (
            "wrong_player",
            serde_json::json!({"player":{"type":"minecraft:pig"}}),
        ),
        (
            "wrong_entity",
            serde_json::json!({"entity":{"type":"minecraft:item"}}),
        ),
        (
            "wrong_rod",
            serde_json::json!({"rod":{"items":"minecraft:stick"}}),
        ),
        (
            "wrong_item",
            serde_json::json!({"item":{"items":"minecraft:bedrock"}}),
        ),
    ] {
        std::fs::write(
            pack.join(format!("data/test/advancement/{name}.json")),
            serde_json::to_vec(&serde_json::json!({
                "criteria":{"hooked":{"trigger":"minecraft:fishing_rod_hooked","conditions":conditions}},
                "rewards":{"experience":if name == "reward" { 5 } else { 0 }}
            })).unwrap(),
        ).unwrap();
    }
    if mounted {
        std::fs::create_dir_all(pack.join("data/test/loot_table")).unwrap();
        std::fs::write(
            pack.join("data/test/loot_table/fishing_rider_cargo.json"),
            serde_json::to_vec(&serde_json::json!({
                "type":"minecraft:chest", "pools":[{
                    "rolls":1,
                    "entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]
                }]
            }))
            .unwrap(),
        )
        .unwrap();
        for (name, count) in [("rider_cargo", 1), ("wrong_rider_cargo", 2)] {
            std::fs::write(
                pack.join(format!("data/test/advancement/{name}.json")),
                serde_json::to_vec(&serde_json::json!({
                    "criteria":{"hooked":{
                        "trigger":"minecraft:fishing_rod_hooked",
                        "conditions":{"player":{"vehicle":{
                            "type":"minecraft:oak_chest_boat",
                            "slots":{"container.*":{"items":"minecraft:diamond","count":count}}
                        }}}
                    }},
                    "rewards":{"experience":0}
                }))
                .unwrap(),
            )
            .unwrap();
        }
    }
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(root.clone()),
            saved_data_packs: None,
            safe_mode: false,
            limits: Default::default(),
        },
    )
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
    let config = ServerWorldConfig {
        datapacks: Some(std::sync::Arc::new(startup)),
        ..ServerWorldConfig::default()
    };
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
    );
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let water = vanilla_block_state_id_by_name_and_properties(
        "minecraft:water",
        &std::collections::BTreeMap::from([("level".to_owned(), "0".to_owned())]),
    )
    .unwrap();
    for x in -3..=3 {
        for z in 2..=9 {
            world.set_block(
                Java26BuiltinDimension::Overworld,
                BlockPos { x, y: 63, z },
                water,
                SET_BLOCK_UPDATE_ALL,
            );
        }
    }
    let count = if empty { 4 } else { 3 };
    for (operation, command) in [
        "setblock 0 61 4 chest".to_owned(),
        format!("item replace block 0 61 4 container.0 with diamond {count}"),
    ]
    .iter()
    .enumerate()
    {
        let result = console_command(
            &sender,
            &receiver,
            &mut world,
            operation as u64 + 1,
            command,
        );
        let answer = console_command_answer(&result);
        assert_eq!(answer.0, Some(1), "{command}: {answer:?}");
    }
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(item_id("minecraft:fishing_rod")),
            count: 1,
            ..ItemStackSummary::empty()
        },
    );

    let idle = |world: &mut ServerWorld| {
        let tick = world.tick(receiver.drain());
        for event in tick.events {
            if let WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::KeepAlive { id },
            } = event
            {
                enqueue(
                    &sender,
                    WorldCommand::Play {
                        connection_id,
                        packet: PlayServerbound::KeepAlive { id },
                    },
                )
                .unwrap();
            }
        }
    };
    let weather_command = if raining {
        "weather rain 100000t"
    } else {
        "weather clear 100000t"
    };
    let weather = console_command(&sender, &receiver, &mut world, 3, weather_command);
    assert_eq!(console_command_answer(&weather).0, Some(100000));
    for _ in 0..200 {
        if world
            .level_weather(Java26BuiltinDimension::Overworld)
            .is_raining()
            == raining
        {
            break;
        }
        idle(&mut world);
    }
    assert_eq!(
        world
            .level_weather(Java26BuiltinDimension::Overworld)
            .is_raining(),
        raining
    );

    let mounted_boat = if mounted {
        let summoned = console_command(
            &sender,
            &receiver,
            &mut world,
            1001,
            "summon minecraft:oak_chest_boat 0.5 64 0.5 {NoGravity:1b,LootTable:'test:fishing_rider_cargo',LootTableSeed:37L}",
        );
        assert_eq!(console_command_answer(&summoned).0, Some(1));
        let boats = entities_of(
            &world,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
        );
        assert_eq!(boats.len(), 1);
        let boat = &boats[0];
        let mounted = console_command(
            &sender,
            &receiver,
            &mut world,
            1002,
            &format!("ride alice mount {}", boat.uuid()),
        );
        assert_eq!(console_command_answer(&mounted).0, Some(1));
        assert_eq!(
            world
                .external_player_vehicle_state_for_test(ConnectionId(1))
                .2,
            Some(boat.id())
        );
        Some((boat.id(), boat.uuid()))
    } else {
        None
    };

    for attempt in 0..if empty { 2 } else { 1 } {
        throw(
            &sender,
            &receiver,
            &mut world,
            1,
            0.0,
            30.0,
            attempt * 2 + 1,
        );
        assert_eq!(
            entities_of(&world, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID).len(),
            1
        );
        // `catchingFish`: up to 600 ticks to lure, 80 to hook, then the nibble.
        let biting = (0..1_000).any(|_| {
            idle(&mut world);
            entities_of(&world, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID)
            .first()
            .is_some_and(
                |hook| matches!(hook.data(), EntityData::FishingBobber(data) if data.nibble() > 0),
            )
        });
        assert!(
            biting,
            "attempt {attempt}: no bite within the lure and hook windows"
        );
        let hook = entities_of(&world, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID)[0].id();
        world.move_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            hook,
            Vec3d {
                x: 0.5,
                y: 63.5,
                z: 4.5,
            },
        );
        world
            .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, hook)
            .unwrap()
            .set_delta_movement(Vec3d::default());
        let items_before = entities_of(&world, VANILLA_ENTITY_TYPE_ITEM_ID).len();

        if let Some((boat, uuid)) = mounted_boat {
            assert_eq!(
                world
                    .external_player_vehicle_state_for_test(ConnectionId(1))
                    .2,
                Some(boat)
            );
            assert_eq!(
                world
                    .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
                    .unwrap()
                    .uuid(),
                uuid
            );
            let NbtSummaryValue::Compound(saved) = world
                .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, boat)
                .unwrap()
            else {
                panic!("boat record")
            };
            assert!(saved.iter().any(|entry| entry.name == "LootTable"
                && entry.value == NbtSummaryValue::String("test:fishing_rider_cargo".into())));
            assert!(saved
                .iter()
                .any(|entry| entry.name == "LootTableSeed"
                    && entry.value == NbtSummaryValue::Long(37)));
            assert!(!world.player_advancement_is_done(ConnectionId(1), "test:rider_cargo"));
        }

        let reeled = throw(
            &sender,
            &receiver,
            &mut world,
            1,
            0.0,
            30.0,
            attempt * 2 + 2,
        );
        assert_eq!(
            world
                .level_weather(Java26BuiltinDimension::Overworld)
                .is_raining(),
            raining
        );
        if empty {
            let totals: Vec<_> = reeled
                .events
                .iter()
                .filter_map(|event| match event {
                    WorldEvent::Send {
                        packet: ClientboundPlayPacket::SetExperience(experience),
                        ..
                    } => Some(experience.total),
                    _ => None,
                })
                .collect();
            if attempt == 0 {
                assert_eq!(totals, [5]);
            } else {
                assert!(
                    totals.is_empty(),
                    "completed reward fired twice: {totals:?}"
                );
            }
        }

        assert!(entities_of(&world, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID).is_empty());
        let caught = entities_of(&world, VANILLA_ENTITY_TYPE_ITEM_ID);
        let expected_count = if empty {
            0
        } else if raining {
            1
        } else {
            2
        };
        assert_eq!(caught.len(), items_before + expected_count);
        if !empty {
            let mut actual = caught
                .iter()
                .map(|entity| {
                    let EntityData::Item(item) = entity.data() else {
                        unreachable!();
                    };
                    (item.item().item_resource_id().unwrap(), item.item().count())
                })
                .collect::<Vec<_>>();
            actual.sort_unstable();
            let expected = if raining {
                vec![("minecraft:apple", 1)]
            } else {
                vec![("minecraft:apple", 1), ("minecraft:cod", 2)]
            };
            assert_eq!(actual, expected);
        }
        let rod = &world.publication().players[0].inventory.slots[0];
        assert_eq!(rod.component_patch.damage, Some(attempt + 1));
        assert_eq!(
            world.player_advancement_is_done(ConnectionId(1), "test:fish"),
            !empty
        );
        for name in ["reward", "player", "list", "bobber"] {
            assert!(
                world.player_advancement_is_done(ConnectionId(1), &format!("test:{name}")),
                "{name}"
            );
        }
        for name in [
            "wrong_player",
            "wrong_entity",
            "wrong_rod",
            "wrong_item",
            "wrong_nbt",
            "wrong_nbt_list",
        ] {
            assert!(
                !world.player_advancement_is_done(ConnectionId(1), &format!("test:{name}")),
                "{name}"
            );
        }
        if let Some((boat, uuid)) = mounted_boat {
            assert!(world.player_advancement_is_done(ConnectionId(1), "test:rider_cargo"));
            assert!(!world.player_advancement_is_done(ConnectionId(1), "test:wrong_rider_cargo"));
            assert!(reeled.events.iter().any(|event| matches!(event,
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::UpdateAdvancements(update),
                } if update.progress.iter().any(|progress| progress.id == "test:rider_cargo"
                    && progress.criteria.iter().any(|criterion| criterion.name == "hooked"
                        && criterion.obtained_epoch_millis.is_some()))
            )));
            let written = world.player_advancements_to_data(ConnectionId(1));
            assert!(written
                .progress
                .get("test:rider_cargo")
                .unwrap()
                .criteria()
                .get(&bbb_protocol::JavaString::from("hooked"))
                .unwrap()
                .obtained_epoch_millis()
                .is_some());
            assert!(written
                .progress
                .get("test:wrong_rider_cargo")
                .is_none_or(|progress| progress
                    .criteria()
                    .get(&bbb_protocol::JavaString::from("hooked"))
                    .is_none_or(|criterion| criterion.obtained_epoch_millis().is_none())));
            assert_eq!(
                world
                    .external_player_vehicle_state_for_test(ConnectionId(1))
                    .2,
                Some(boat)
            );
            let actual = world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
                .unwrap();
            assert_eq!(actual.uuid(), uuid);
            assert!(!crate::entity::item_slots::container_has_loot(actual));
            let cargo: Vec<_> = (0..crate::entity::item_slots::container_size(actual).unwrap())
                .filter_map(|slot| {
                    crate::entity::item_slots::read(actual, slot.try_into().unwrap()).unwrap()
                })
                .filter(|stack| !stack.is_empty())
                .map(|stack| (stack.item_resource_id().unwrap().to_owned(), stack.count()))
                .collect();
            assert_eq!(cargo, [("minecraft:diamond".to_owned(), 1)]);
            let NbtSummaryValue::Compound(saved) = world
                .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, boat)
                .unwrap()
            else {
                panic!("boat record")
            };
            assert!(!saved.iter().any(|entry| entry.name == "LootTable"));
            let NbtSummaryValue::List(items) = &saved
                .iter()
                .find(|entry| entry.name == "Items")
                .unwrap()
                .value
            else {
                panic!("saved cargo")
            };
            assert_eq!(items.len(), 1);
            let NbtSummaryValue::Compound(item) = &items[0] else {
                panic!("saved stack")
            };
            assert!(item.iter().any(|entry| entry.name == "id"
                && entry.value == NbtSummaryValue::String("minecraft:diamond".into())));
            assert!(item
                .iter()
                .any(|entry| entry.name == "count" && entry.value == NbtSummaryValue::Int(1)));
        }
    }
}

#[test]
fn a_hooked_item_uses_the_target_entity_and_its_complete_stack() {
    use crate::tests::parse_advancement;
    use bbb_protocol::painting_variant_types::{
        vanilla_painting_variant_for_resource_id, vanilla_painting_variant_resource_id,
    };
    use bbb_server_datapack::advancement::AdvancementElement;
    use bbb_server_types::item_stack::RuntimeItemStack;

    let painting_variant = vanilla_painting_variant_for_resource_id("minecraft:kebab").unwrap();
    let wrong_painting_variant =
        vanilla_painting_variant_for_resource_id("minecraft:aztec").unwrap();
    let mut config = datapack_config();
    let mut startup = (*config.datapacks.take().unwrap()).clone();
    for (name, conditions) in [
        (
            "target",
            serde_json::json!({"entity":{"type":"minecraft:item"},"item":{"items":"minecraft:diamond","count":3,"predicates":{"minecraft:stored_enchantments":[{"levels":3}]}}}),
        ),
        (
            "bobber",
            serde_json::json!({"entity":{"type":"minecraft:fishing_bobber"}}),
        ),
        (
            "wrong_stack",
            serde_json::json!({"item":{"items":"minecraft:diamond","count":2}}),
        ),
        (
            "painting",
            serde_json::json!({"entity":{"type":"minecraft:painting","components":{"minecraft:painting/variant":vanilla_painting_variant_resource_id(painting_variant.id).unwrap()}}}),
        ),
        (
            "wrong_painting_variant",
            serde_json::json!({"entity":{"type":"minecraft:painting","components":{"minecraft:painting/variant":vanilla_painting_variant_resource_id(wrong_painting_variant.id).unwrap()}}}),
        ),
    ] {
        startup.advancements.advancements.push(AdvancementElement {
            id: format!("test:{name}"), source_pack: "test".into(),
            advancement: parse_advancement(&serde_json::json!({
                "criteria":{"hooked":{"trigger":"minecraft:fishing_rod_hooked","conditions":conditions}}
            })).unwrap(),
        });
    }
    config.datapacks = Some(std::sync::Arc::new(startup));
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
    );
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(item_id("minecraft:fishing_rod")),
            count: 1,
            ..ItemStackSummary::empty()
        },
    );
    let (target_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        VANILLA_ENTITY_TYPE_ITEM_ID,
        Vec3d {
            x: 0.5,
            y: 65.5,
            z: 2.5,
        },
        EntityData::Item(crate::entity::ItemEntityData::new(
            RuntimeItemStack::from_summary(ItemStackSummary {
                item_id: Some(item_id("minecraft:diamond")),
                count: 3,
                ..ItemStackSummary::empty()
            })
            .with_component_nbt(
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
                Some(&bbb_protocol::NbtSummaryValue::Compound(vec![bbb_protocol::NbtSummaryEntry {
                    name: "minecraft:sharpness".into(),
                    value: bbb_protocol::NbtSummaryValue::Int(3),
                }])),
                &world.item_stack_resolver_for_test(),
            )
            .unwrap(),
            0.0,
        )),
    );
    throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 1);
    let hooked = (0..10).any(|_| {
        world.tick(receiver.drain());
        entities_of(&world, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID).first().is_some_and(|hook| {
            matches!(hook.data(), EntityData::FishingBobber(data) if data.hooked_entity_id() == Some(target_id))
        })
    });
    assert!(hooked, "the cast did not collide with the item");
    throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 2);
    assert!(world.player_advancement_is_done(ConnectionId(1), "test:target"));
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:bobber"));
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:wrong_stack"));

    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 4.5,
            y: 64.0,
            z: 0.0,
        },
    );
    let wall = BlockPos { x: 4, y: 65, z: 3 };
    world.set_block(
        Java26BuiltinDimension::Overworld,
        wall,
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    );
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(item_id("minecraft:painting")),
            count: 1,
            component_patch: DataComponentPatchSummary {
                painting_variant_id: world.synchronized_registries().registry_element_index(
                    "minecraft:painting_variant",
                    painting_variant.resource_id,
                ),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: wall,
                direction: Direction::North,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence: 3,
        }),
    );
    let paintings = entities_of(&world, VANILLA_ENTITY_TYPE_PAINTING_ID);
    assert_eq!(paintings.len(), 1);
    let painting = &paintings[0];
    let EntityData::Painting(data) = painting.data() else {
        unreachable!();
    };
    assert_eq!(data.variant().resource_id(), painting_variant.resource_id);
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:painting"));
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(item_id("minecraft:fishing_rod")),
            count: 1,
            ..ItemStackSummary::empty()
        },
    );
    throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 4);
    let hooked = (0..10).any(|_| {
        world.tick(receiver.drain());
        entities_of(&world, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID).first().is_some_and(|hook| {
            matches!(hook.data(), EntityData::FishingBobber(data) if data.hooked_entity_id() == Some(painting.id()))
        })
    });
    assert!(hooked, "the cast did not collide with the painting");
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:painting"));
    throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 5);
    assert!(entities_of(&world, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID).is_empty());
    assert!(world.player_advancement_is_done(ConnectionId(1), "test:painting"));
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:wrong_painting_variant"));
}
